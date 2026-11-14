#include "duckdb/function/table/arrow.hpp"
#include "duckdb/main/settings.hpp"
#include "error.hpp"
#include "source/options.hpp"
#include "source/reader.hpp"
#include "source/source_factory.hpp"

namespace duckdb {
namespace {
struct ReaderInfo : TableFunctionInfo {
	explicit ReaderInfo(ReaderOpen open) : open(open) {}
	ReaderOpen open;
};
void CheckAccess(ClientContext &context) {
	if (!Settings::Get<EnableExternalAccessSetting>(context)) {
		throw PermissionException("DuckPort requires enable_external_access");
	}
}
unique_ptr<FunctionData> BindReader(ClientContext &context, TableFunctionBindInput &input, vector<LogicalType> &types,
									vector<Identifier> &names) {
	CheckAccess(context);
	auto path = ReadSourceString(input.inputs[0]);
	ReaderOptions options(input);
	DuckportSource *raw = nullptr;
	CheckError(input.info->Cast<ReaderInfo>().open(path.c_str(), &options.storage, &options.scan, &raw));
	auto factory = make_shared_ptr<SourceFactory>(SourcePtr(raw, duckport_source_destroy));
	auto result = make_uniq<ArrowScanFunctionData>(std::move(factory));
	result->factory->GetSchema(result->schema_root.arrow_schema);
	ArrowTableFunction::PopulateArrowTableSchema(context, result->arrow_table, result->schema_root.arrow_schema);
	names = StringsToIdentifiers(result->arrow_table.GetNames());
	types = result->arrow_table.GetTypes();
	if (types.empty()) {
		throw BinderException("source must have at least one column");
	}
	result->all_types = types;
	result->projection_pushdown_enabled = false;
	return std::move(result);
}
unique_ptr<GlobalTableFunctionState> InitReader(ClientContext &context, TableFunctionInitInput &input) {
	CheckAccess(context);
	return ArrowTableFunction::ArrowScanInitGlobal(context, input);
}
} // namespace
void RegisterReader(ExtensionLoader &loader, const string &name, ReaderOpen open,
					const std::function<void(TypedKwargs &)> &configure) {
	TableFunction function(Identifier(name), {LogicalType::VARCHAR}, ArrowTableFunction::ArrowScanFunction, BindReader,
						   InitReader, ArrowTableFunction::ArrowScanInitLocal);
	function.function_info = make_shared_ptr<ReaderInfo>(open);
	function.GetSignature().WithTypedKwargs("options", [&](TypedKwargs &options) {
		options.Add("batch_size", LogicalType::BIGINT);
		RegisterStorageOptions(options);
		if (configure) {
			configure(options);
		}
	});
	loader.RegisterFunction(function);
}
} // namespace duckdb
