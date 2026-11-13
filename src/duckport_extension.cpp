#define DUCKDB_EXTENSION_MAIN

#include "duckport_extension.hpp"
#include "duckdb/common/arrow/arrow_wrapper.hpp"
#include "duckdb/common/arrow/schema_metadata.hpp"
#include "duckdb/common/exception.hpp"
#include "duckdb/function/table/arrow.hpp"
#include "duckdb/main/settings.hpp"
#include <cstring>

extern "C" {
char *duckport_source_open(const char *locator, const char *reader, const char *options, size_t batch_size,
                           ArrowArrayStream *out);
void duckport_source_error_free(char *error);
}

namespace duckdb {
namespace {

void CheckSourceAccess(ClientContext &context) {
	if (!Settings::Get<EnableExternalAccessSetting>(context)) {
		throw PermissionException("DuckPort reader requires enable_external_access");
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
	DuckportSourceFactory(string locator_p, string reader_p, string options_p, idx_t batch_size_p)
	    : locator(std::move(locator_p)), reader(std::move(reader_p)), options(std::move(options_p)),
	      batch_size(batch_size_p) {
		initial_stream = Open();
		initial_stream->GetSchema(expected_schema);
	}

	unique_ptr<ArrowArrayStreamWrapper> Open() const {
		auto stream = make_uniq<ArrowArrayStreamWrapper>();
		auto error = duckport_source_open(locator.c_str(), reader.c_str(), options.c_str(), batch_size,
		                                  &stream->arrow_array_stream);
		if (error) {
			string message(error);
			duckport_source_error_free(error);
			throw IOException("DuckPort reader: %s", message);
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
			throw IOException("DuckPort reader: %s", initial_stream->GetError());
		}
	}

	unique_ptr<ArrowArrayStreamWrapper> ProduceStream(ArrowStreamParameters &) override {
		lock_guard<mutex> lock(factory_mutex);
		if (initial_stream) {
			return std::move(initial_stream);
		}
		// Prepared statements and rescans must receive a fresh reader, not an
		// exhausted stream.
		auto stream = Open();
		ArrowSchemaWrapper schema;
		stream->GetSchema(schema);
		if (!SameSchema(expected_schema.arrow_schema, schema.arrow_schema)) {
			throw IOException("DuckPort reader: source schema changed; rebind the query");
		}
		return stream;
	}

	string locator;
	string reader;
	string options;
	idx_t batch_size;
	mutex factory_mutex;
	ArrowSchemaWrapper expected_schema;
	unique_ptr<ArrowArrayStreamWrapper> initial_stream;
};

struct ReaderInfo : TableFunctionInfo {
	explicit ReaderInfo(string name_p) : name(std::move(name_p)) {
	}
	string name;
};

string JsonString(const string &value) {
	string result = "\"";
	const char hex[] = "0123456789abcdef";
	for (unsigned char c : value) {
		if (c == '\\' || c == '"') {
			result += '\\';
			result += char(c);
		} else if (c < 32) {
			result += "\\u00";
			result += hex[c >> 4];
			result += hex[c & 15];
		} else {
			result += char(c);
		}
	}
	return result + "\"";
}

string ReaderOptions(const named_argument_map_t &parameters) {
	string result = "{";
	for (const auto &entry : parameters) {
		auto key = entry.first.GetIdentifierName();
		const auto &value = entry.second;
		if (value.IsNull()) {
			throw BinderException("DuckPort reader: %s cannot be NULL", key);
		}
		if (key == "batch_size") {
			continue;
		}
		if (result.size() > 1) {
			result += ',';
		}
		result += JsonString(key) + ":";
		if (value.type().id() == LogicalTypeId::BOOLEAN) {
			result += BooleanValue::Get(value) ? "true" : "false";
		} else {
			auto text = StringValue::Get(value);
			if (text.find('\0') != string::npos) {
				throw BinderException("DuckPort reader: %s contains NUL bytes", key);
			}
			result += key == "storage_options" ? text : JsonString(text);
		}
	}
	return result + "}";
}

unique_ptr<FunctionData> BindSource(ClientContext &context, TableFunctionBindInput &input,
                                    vector<LogicalType> &return_types, vector<Identifier> &names) {
	CheckSourceAccess(context);
	if (input.inputs[0].IsNull()) {
		throw BinderException("DuckPort reader: source cannot be NULL");
	}
	auto locator = StringValue::Get(input.inputs[0]);
	if (locator.find('\0') != string::npos) {
		throw BinderException("DuckPort reader: source cannot contain NUL bytes");
	}
	idx_t batch_size = 8192;
	auto batch = input.named_parameters.find("batch_size");
	if (batch != input.named_parameters.end()) {
		if (batch->second.IsNull()) {
			throw BinderException("DuckPort reader: batch_size cannot be NULL");
		}
		auto size = BigIntValue::Get(batch->second);
		if (size < 1 || size > 1048576) {
			throw BinderException("DuckPort reader: batch_size must be between 1 and 1048576");
		}
		batch_size = idx_t(size);
	}
	auto reader = input.info->Cast<ReaderInfo>().name;
	auto require = [&](const char *name) {
		if (input.named_parameters.find(name) == input.named_parameters.end()) {
			throw BinderException("duckport_read_%s requires %s", reader, name);
		}
	};
	if (reader == "xml") {
		require("record_path");
	}
	if (reader == "huggingface_file") {
		require("path");
	}
	if (reader == "fs" || reader == "http" || reader == "s3" || reader == "huggingface_file") {
		require("format");
	}

	auto factory =
	    make_shared_ptr<DuckportSourceFactory>(locator, reader, ReaderOptions(input.named_parameters), batch_size);

	auto result = make_uniq<ArrowScanFunctionData>(std::move(factory));
	result->factory->GetSchema(result->schema_root.arrow_schema);
	ArrowTableFunction::PopulateArrowTableSchema(context, result->arrow_table, result->schema_root.arrow_schema);
	names = StringsToIdentifiers(result->arrow_table.GetNames());
	return_types = result->arrow_table.GetTypes();
	if (return_types.empty()) {
		throw BinderException("DuckPort reader: source must have at least one column");
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

void RegisterReader(ExtensionLoader &loader, const string &reader) {
	TableFunction function(Identifier("duckport_read_" + reader), {LogicalType::VARCHAR},
	                       ArrowTableFunction::ArrowScanFunction, BindSource, InitSource,
	                       ArrowTableFunction::ArrowScanInitLocal);
	function.function_info = make_shared_ptr<ReaderInfo>(reader);
	function.GetSignature().WithTypedKwargs("options", [&](TypedKwargs &options) {
		options.Add("batch_size", LogicalType::BIGINT);
		bool hf = reader == "huggingface" || reader == "huggingface_file";
		bool storage = reader == "fs" || reader == "http" || reader == "s3";
		if (!hf) {
			options.Add("storage_options", LogicalType::VARCHAR);
		}
		if (reader == "csv" || reader == "tsv" || storage || reader == "huggingface_file") {
			options.Add("header", LogicalType::BOOLEAN);
		}
		if (reader == "xml" || storage || reader == "huggingface_file") {
			options.Add("record_path", LogicalType::VARCHAR);
		}
		if (storage || reader == "huggingface_file") {
			options.Add("format", LogicalType::VARCHAR);
		}
		if (hf) {
			for (const auto &name : {"revision", "endpoint", "token"}) {
				options.Add(name, LogicalType::VARCHAR);
			}
			if (reader == "huggingface") {
				options.Add("config", LogicalType::VARCHAR);
				options.Add("split", LogicalType::VARCHAR);
				options.Add("allow_partial", LogicalType::BOOLEAN);
			} else {
				options.Add("path", LogicalType::VARCHAR);
			}
		}
	});
	function.projection_pushdown = false;
	function.filter_pushdown = false;
	function.parallelism = TableFunctionParallelism::FORCE_SINGLE_THREADED;
	loader.RegisterFunction(function);
}

void LoadInternal(ExtensionLoader &loader) {
	for (const auto &reader : {"parquet", "csv", "tsv", "json", "jsonl", "ndjson", "xml", "atif", "atif_jsonl", "warc",
	                           "fs", "http", "s3", "huggingface", "huggingface_file"}) {
		RegisterReader(loader, reader);
	}
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
