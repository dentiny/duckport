#include "duckdb/function/table/arrow.hpp"
#include "duckdb/main/settings.hpp"
#include "duckdb/main/query_profiler.hpp"
#include "error.hpp"
#include "source/options.hpp"
#include "source/reader.hpp"
#include "source/source_factory.hpp"
#include "source_metadata.hpp"
#include "storage_options.hpp"

namespace duckdb {
namespace {
struct ReaderInfo : TableFunctionInfo {
	explicit ReaderInfo(ReaderBind open) : open(std::move(open)) {
	}
	ReaderBind open;
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
	auto scan = ReadScanOptions(input);
	const auto &info = input.info->Cast<ReaderInfo>();
	DuckportSource *raw = nullptr;
	CheckError(info.open(context, path, scan, input, &raw));
	auto factory = make_shared_ptr<SourceFactory>(SourcePtr(raw, duckport_source_destroy));
	auto result = make_uniq<ArrowScanFunctionData>(std::move(factory));
	result->factory->GetSchema(result->schema_root.arrow_schema);
	ArrowTableFunction::PopulateArrowTableSchema(context, result->arrow_table, result->schema_root.arrow_schema);
	names = StringsToIdentifiers(result->arrow_table.GetNames());
	types = result->arrow_table.GetTypes();
	ImportSourceMetadata(types, result->schema_root.arrow_schema);
	if (types.empty()) {
		throw BinderException("source must have at least one column");
	}
	result->all_types = types;
	result->projection_pushdown_enabled = true;
	return std::move(result);
}
unique_ptr<GlobalTableFunctionState> InitReader(ClientContext &context, TableFunctionInitInput &input) {
	CheckAccess(context);
	return ArrowTableFunction::ArrowScanInitGlobal(context, input);
}
} // namespace
SourceFactory &GetFactory(const FunctionData *data) {
	return data->Cast<ArrowScanFunctionData>().factory->Cast<SourceFactory>();
}
void ScanReader(ClientContext &context, TableFunctionInput &input, DataChunk &output) {
	ArrowTableFunction::ArrowScanFunction(context, input, output);
	GetFactory(input.bind_data.get()).ReportBytes(context);
}
void GetReaderMetrics(TableFunctionGetMetricsInput &input) {
	auto &factory = GetFactory(input.bind_data.get());
	factory.ReportBytes(input.context);
	auto rows = input.bind_data->Cast<ArrowScanFunctionData>().lines_read.load();
	input.operator_metrics.rows_scanned = factory.ReportRows(rows);
}
double ReaderProgress(ClientContext &, const FunctionData *data, const GlobalTableFunctionState *) {
	const auto metrics = GetFactory(data).Snapshot();
	if (metrics.finished) {
		return 100.0;
	}
	if (!metrics.total_bytes_known || metrics.total_bytes == 0) {
		return -1.0;
	}
	// Range reads, schema inference and prefetch can exceed the selected input.
	// Only the Arrow reader reaching EOF signals complete consumption.
	return MinValue(99.0, 100.0 * double(metrics.progress_bytes) / double(metrics.total_bytes));
}
void RegisterReader(ExtensionLoader &loader, const string &name, ReaderBind open,
                    const std::function<void(TypedKwargs &)> &add_reader_options) {
	TableFunction function(Identifier(name), {LogicalType::VARCHAR}, ScanReader, BindReader, InitReader,
	                       ArrowTableFunction::ArrowScanInitLocal);
	function.function_info = make_shared_ptr<ReaderInfo>(std::move(open));
	function.GetSignature().WithTypedKwargs("options", [&](TypedKwargs &options) {
		options.Add("batch_size", LogicalType::BIGINT);
		if (add_reader_options) {
			add_reader_options(options);
		}
	});
	function.get_metrics = GetReaderMetrics;
	function.table_scan_progress = ReaderProgress;
	function.projection_pushdown = true;
	loader.RegisterFunction(function);
}
void RegisterReader(ExtensionLoader &loader, const string &name, ReaderOpen open,
                    const std::function<void(TypedKwargs &)> &add_reader_options) {
	ReaderBind bind = [open](ClientContext &context, const string &locator, const DuckportScanOptions &scan,
	                         const TableFunctionBindInput &, DuckportSource **output) {
		StorageParameters storage(context, locator, /*secret_type=*/ {});
		return open(locator.c_str(), storage.config, scan, output);
	};
	RegisterReader(loader, name, std::move(bind), add_reader_options);
}
} // namespace duckdb
