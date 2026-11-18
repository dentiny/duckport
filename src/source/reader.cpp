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
using FileListPtr = unique_ptr<DuckportFileList, decltype(&duckport_file_list_destroy)>;
struct ReaderInfo : TableFunctionInfo {
	explicit ReaderInfo(ReaderBind open, string format = {}) : open(std::move(open)), format(std::move(format)) {
	}
	ReaderBind open;
	string format;
};
void CheckAccess(ClientContext &context) {
	if (!Settings::Get<EnableExternalAccessSetting>(context)) {
		throw PermissionException("DuckPort requires enable_external_access");
	}
}
unique_ptr<FunctionData> BindReader(ClientContext &context, TableFunctionBindInput &input, vector<LogicalType> &types,
                                    vector<Identifier> &names) {
	CheckAccess(context);
	auto scan = ReadScanOptions(input);
	const auto &info = input.info->Cast<ReaderInfo>();
	string path;
	vector<string> files;
	vector<unique_ptr<StorageParameters>> storage;
	vector<DuckportSelectedFile> selected;
	if (info.format.empty()) {
		path = ReadSourceString(input.inputs[0]);
	} else {
		vector<string> selections;
		if (input.inputs[0].IsNull()) {
			throw BinderException("DuckPort arguments cannot be NULL");
		}
		if (input.inputs[0].type().id() == LogicalTypeId::LIST) {
			for (const auto &entry : ListValue::GetChildren(input.inputs[0])) {
				selections.push_back(ReadSourceString(entry));
			}
		} else {
			selections.push_back(ReadSourceString(input.inputs[0]));
		}
		if (selections.empty()) {
			throw BinderException("file list cannot be empty");
		}
		for (const auto &selection : selections) {
			DuckportFileList *root_raw = nullptr;
			CheckError(duckport_file_discovery_root(selection.c_str(), &root_raw));
			FileListPtr root(root_raw, duckport_file_list_destroy);
			StorageParameters discovery(context, duckport_file_list_get(root.get(), 0), {});
			DuckportFileList *raw = nullptr;
			CheckError(duckport_resolve_files(selection.c_str(), info.format.c_str(), discovery.config, &raw));
			unique_ptr<DuckportFileList, decltype(&duckport_file_list_destroy)> matches(raw,
			                                                                            duckport_file_list_destroy);
			for (idx_t i = 0; i < duckport_file_list_count(matches.get()); i++) {
				files.emplace_back(duckport_file_list_get(matches.get(), i));
			}
		}
		path = files[0];
		for (const auto &file : files) {
			storage.push_back(make_uniq<StorageParameters>(context, file, string()));
			selected.push_back({file.c_str(), storage.back()->config});
		}
		if (files.size() > 1) {
			scan.files = selected.data();
			scan.file_count = selected.size();
		}
	}
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
SourceFactory &GetFactory(const FunctionData *data) {
	return data->Cast<ArrowScanFunctionData>().factory->Cast<SourceFactory>();
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
} // namespace
static void RegisterReaderImpl(ExtensionLoader &loader, const string &name, ReaderBind open,
                               const std::function<void(TypedKwargs &)> &add_reader_options, bool files) {
	auto format = files ? name.substr(string("duckport_scan_").size()) : string();
	auto register_signature = [&](LogicalType input_type) {
		TableFunction function(Identifier(name), {input_type}, ArrowTableFunction::ArrowScanFunction, BindReader,
		                       InitReader, ArrowTableFunction::ArrowScanInitLocal);
		function.function_info = make_shared_ptr<ReaderInfo>(open, format);
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
	};
	register_signature(LogicalType::VARCHAR);
	if (files) {
		register_signature(LogicalType::LIST(LogicalType::VARCHAR));
	}
}
void RegisterReader(ExtensionLoader &loader, const string &name, ReaderBind open,
                    const std::function<void(TypedKwargs &)> &options) {
	RegisterReaderImpl(loader, name, std::move(open), options, false);
}
void RegisterFileReader(ExtensionLoader &loader, const string &name, ReaderBind open,
                        const std::function<void(TypedKwargs &)> &options) {
	RegisterReaderImpl(loader, name, std::move(open), options, true);
}
void RegisterFileReader(ExtensionLoader &loader, const string &name, ReaderOpen open,
                        const std::function<void(TypedKwargs &)> &options) {
	ReaderBind bind = [open](ClientContext &context, const string &locator, const DuckportScanOptions &scan,
	                         const TableFunctionBindInput &, DuckportSource **output) {
		StorageParameters storage(context, locator, {});
		return open(locator.c_str(), storage.config, scan, output);
	};
	RegisterFileReader(loader, name, std::move(bind), options);
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
