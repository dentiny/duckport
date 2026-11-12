#define DUCKDB_EXTENSION_MAIN

#include "duckport_extension.hpp"
#include "duckdb/common/arrow/arrow_wrapper.hpp"
#include "duckdb/common/arrow/schema_metadata.hpp"
#include "duckdb/common/exception.hpp"
#include "duckdb/function/table/arrow.hpp"
#include "duckdb/main/settings.hpp"
#include <cstring>

extern "C" {
char *duckport_source_open(const char *locator, const char *format, const char *options, size_t batch_size,
                           ArrowArrayStream *out);
void duckport_source_error_free(char *error);
}

namespace duckdb {
namespace {

void CheckSourceAccess(ClientContext &context) {
	if (!Settings::Get<EnableExternalAccessSetting>(context)) {
		throw PermissionException("duckport_scan requires enable_external_access");
	}
}

bool SameSchema(const ArrowSchema &left, const ArrowSchema &right) {
	auto same_string = [](const char *a, const char *b) {
		return (!a && !b) || (a && b && std::strcmp(a, b) == 0);
	};
	if (!same_string(left.format, right.format) || !same_string(left.name, right.name) || left.flags != right.flags ||
	    left.n_children != right.n_children || bool(left.dictionary) != bool(right.dictionary)) {
		return false;
	}
	ArrowSchemaMetadata left_metadata(left.metadata), right_metadata(right.metadata);
	if (left_metadata.GetOption(ArrowSchemaMetadata::ARROW_EXTENSION_NAME) !=
	        right_metadata.GetOption(ArrowSchemaMetadata::ARROW_EXTENSION_NAME) ||
	    left_metadata.GetOption(ArrowSchemaMetadata::ARROW_METADATA_KEY) !=
	        right_metadata.GetOption(ArrowSchemaMetadata::ARROW_METADATA_KEY)) {
		return false;
	}
	for (int64_t i = 0; i < left.n_children; i++) {
		if (!SameSchema(*left.children[i], *right.children[i])) {
			return false;
		}
	}
	return !left.dictionary || SameSchema(*left.dictionary, *right.dictionary);
}

struct DuckportSourceFactory : public ArrowScanFactory {
	DuckportSourceFactory(string locator_p, string format_p, string options_p, idx_t batch_size_p)
	    : locator(std::move(locator_p)), format(std::move(format_p)), options(std::move(options_p)),
	      batch_size(batch_size_p) {
		initial_stream = Open();
		initial_stream->GetSchema(expected_schema);
	}

	unique_ptr<ArrowArrayStreamWrapper> Open() const {
		auto stream = make_uniq<ArrowArrayStreamWrapper>();
		auto error = duckport_source_open(locator.c_str(), format.c_str(), options.c_str(), batch_size,
		                                  &stream->arrow_array_stream);
		if (error) {
			string message(error);
			duckport_source_error_free(error);
			throw IOException("duckport_scan: %s", message);
		}
		return stream;
	}

	void GetSchema(ArrowSchema &schema) override {
		lock_guard<mutex> lock(factory_mutex);
		// get_schema exports a separate owned schema; the factory retains its own.
		if (!initial_stream) {
			initial_stream = Open();
		}
		auto &stream = initial_stream->arrow_array_stream;
		if (stream.get_schema(&stream, &schema) != 0) {
			throw IOException("duckport_scan: %s", initial_stream->GetError());
		}
	}

	unique_ptr<ArrowArrayStreamWrapper> ProduceStream(ArrowStreamParameters &) override {
		lock_guard<mutex> lock(factory_mutex);
		if (initial_stream) {
			return std::move(initial_stream);
		}
		// Prepared statements and rescans must receive a fresh reader, not an exhausted stream.
		auto stream = Open();
		ArrowSchemaWrapper schema;
		stream->GetSchema(schema);
		if (!SameSchema(expected_schema.arrow_schema, schema.arrow_schema)) {
			throw IOException("duckport_scan: source schema changed; rebind the query");
		}
		return stream;
	}

	string locator;
	string format;
	string options;
	idx_t batch_size;
	mutex factory_mutex;
	ArrowSchemaWrapper expected_schema;
	unique_ptr<ArrowArrayStreamWrapper> initial_stream;
};

string StringParameter(const named_argument_map_t &parameters, const string &name, const string &fallback) {
	auto entry = parameters.find(Identifier(name));
	if (entry == parameters.end()) {
		return fallback;
	}
	if (entry->second.IsNull()) {
		throw BinderException("duckport_scan: %s cannot be NULL", name);
	}
	auto value = StringValue::Get(entry->second);
	if (value.find('\0') != string::npos) {
		throw BinderException("duckport_scan: %s cannot contain NUL bytes", name);
	}
	return value;
}

unique_ptr<FunctionData> BindSource(ClientContext &context, TableFunctionBindInput &input,
                                    vector<LogicalType> &return_types, vector<Identifier> &names) {
	CheckSourceAccess(context);
	if (input.inputs[0].IsNull()) {
		throw BinderException("duckport_scan: source cannot be NULL");
	}
	auto locator = StringValue::Get(input.inputs[0]);
	if (locator.find('\0') != string::npos) {
		throw BinderException("duckport_scan: source cannot contain NUL bytes");
	}
	idx_t batch_size = 8192;
	auto batch = input.named_parameters.find("batch_size");
	if (batch != input.named_parameters.end()) {
		if (batch->second.IsNull()) {
			throw BinderException("duckport_scan: batch_size cannot be NULL");
		}
		auto size = BigIntValue::Get(batch->second);
		if (size < 1 || size > 1048576) {
			throw BinderException("duckport_scan: batch_size must be between 1 and 1048576");
		}
		batch_size = idx_t(size);
	}
	auto factory =
	    make_shared_ptr<DuckportSourceFactory>(locator, StringParameter(input.named_parameters, "format", "auto"),
	                                           StringParameter(input.named_parameters, "options", "{}"), batch_size);
	auto result = make_uniq<ArrowScanFunctionData>(std::move(factory));
	result->factory->GetSchema(result->schema_root.arrow_schema);
	ArrowTableFunction::PopulateArrowTableSchema(context, result->arrow_table, result->schema_root.arrow_schema);
	names = StringsToIdentifiers(result->arrow_table.GetNames());
	return_types = result->arrow_table.GetTypes();
	if (return_types.empty()) {
		throw BinderException("duckport_scan: source must have at least one column");
	}
	result->all_types = return_types;
	result->projection_pushdown_enabled = false;
	return std::move(result);
}

unique_ptr<GlobalTableFunctionState> InitSource(ClientContext &context, TableFunctionInitInput &input) {
	CheckSourceAccess(context);
	auto state = ArrowTableFunction::ArrowScanInitGlobal(context, input);
	state->Cast<ArrowScanGlobalState>().max_threads = 1;
	return state;
}

void LoadInternal(ExtensionLoader &loader) {
	TableFunction scan("duckport_scan", {LogicalType::VARCHAR}, ArrowTableFunction::ArrowScanFunction, BindSource,
	                   InitSource, ArrowTableFunction::ArrowScanInitLocal);
	scan.GetSignature().AddKeywordOnly("format", LogicalType::VARCHAR, Value("auto"));
	scan.GetSignature().AddKeywordOnly("options", LogicalType::VARCHAR, Value("{}"));
	scan.GetSignature().AddKeywordOnly("batch_size", LogicalType::BIGINT, Value::BIGINT(8192));
	scan.projection_pushdown = false;
	scan.filter_pushdown = false;
	scan.parallelism = TableFunctionParallelism::FORCE_SINGLE_THREADED;
	loader.RegisterFunction(scan);
}

} // namespace

void DuckportExtension::Load(ExtensionLoader &loader) {
	LoadInternal(loader);
}
std::string DuckportExtension::Name() {
	return "duckport";
}
std::string DuckportExtension::Version() const {
#ifdef EXT_VERSION_DUCKPORT
	return EXT_VERSION_DUCKPORT;
#else
	return "";
#endif
}

} // namespace duckdb

extern "C" {
DUCKDB_CPP_EXTENSION_ENTRY(duckport, loader) {
	duckdb::LoadInternal(loader);
}
}
