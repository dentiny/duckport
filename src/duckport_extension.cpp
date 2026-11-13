#define DUCKDB_EXTENSION_MAIN
#include "duckdb/function/table/arrow.hpp"
#include "duckdb/main/settings.hpp"
#include "duckport_extension.hpp"
#include "error.hpp"
#include "source/source_factory.hpp"

namespace duckdb {
namespace {
void CheckAccess(ClientContext &context) {
	if (!Settings::Get<EnableExternalAccessSetting>(context)) {
		throw PermissionException("DuckPort requires enable_external_access");
	}
}
string ReadString(const Value &value) {
	if (value.IsNull()) {
		throw BinderException("DuckPort arguments cannot be NULL");
	}
	auto text = StringValue::Get(value);
	if (text.find('\0') != string::npos) {
		throw BinderException("DuckPort arguments cannot contain NUL bytes");
	}
	return text;
}
unique_ptr<FunctionData> BindParquet(ClientContext &context, TableFunctionBindInput &input, vector<LogicalType> &types,
                                     vector<Identifier> &names) {
	CheckAccess(context);
	auto path = ReadString(input.inputs[0]);
	string storage = "{}";
	idx_t batch_size = 8192;
	auto option = input.named_parameters.find("storage_options");
	if (option != input.named_parameters.end()) {
		storage = ReadString(option->second);
	}
	option = input.named_parameters.find("batch_size");
	if (option != input.named_parameters.end()) {
		if (option->second.IsNull()) {
			throw BinderException("batch_size cannot be NULL");
		}
		auto size = BigIntValue::Get(option->second);
		if (size < 1 || size > 1048576) {
			throw BinderException("batch_size must be between 1 and 1048576");
		}
		batch_size = idx_t(size);
	}
	DuckportSource *raw = nullptr;
	CheckError(duckport_parquet_open(path.c_str(), storage.c_str(), batch_size, &raw));
	auto factory = make_shared_ptr<SourceFactory>(SourcePtr(raw, duckport_source_destroy));
	auto result = make_uniq<ArrowScanFunctionData>(std::move(factory));
	result->factory->GetSchema(result->schema_root.arrow_schema);
	ArrowTableFunction::PopulateArrowTableSchema(context, result->arrow_table, result->schema_root.arrow_schema);
	names = StringsToIdentifiers(result->arrow_table.GetNames());
	types = result->arrow_table.GetTypes();
	if (types.empty()) {
		throw BinderException("Parquet source must have at least one column");
	}
	result->all_types = types;
	result->projection_pushdown_enabled = false;
	return std::move(result);
}
unique_ptr<GlobalTableFunctionState> InitSource(ClientContext &context, TableFunctionInitInput &input) {
	CheckAccess(context);
	return ArrowTableFunction::ArrowScanInitGlobal(context, input);
}
struct ArrowScan : ArrowTableFunction {
	using ArrowTableFunction::ArrowGetPartitionData;
};
void LoadInternal(ExtensionLoader &loader) {
	TableFunction function("duckport_read_parquet", {LogicalType::VARCHAR}, ArrowTableFunction::ArrowScanFunction,
	                       BindParquet, InitSource, ArrowTableFunction::ArrowScanInitLocal);
	function.GetSignature().WithTypedKwargs("options", [](TypedKwargs &options) {
		options.Add("storage_options", LogicalType::VARCHAR);
		options.Add("batch_size", LogicalType::BIGINT);
	});
	function.get_partition_data = ArrowScan::ArrowGetPartitionData;
	function.parallelism = TableFunctionParallelism::SEQUENTIAL;
	loader.RegisterFunction(function);
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
