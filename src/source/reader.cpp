#include "duckdb/function/table/arrow.hpp"
#include "duckdb/main/settings.hpp"
#include "duckdb/main/profiler/profiling_node.hpp"
#include "error.hpp"
#include "source/options.hpp"
#include "source/reader.hpp"
#include "source/source_factory.hpp"
#include "source_metadata.hpp"
#include "storage_options.hpp"
#include "duckdb/planner/operator/logical_get.hpp"
#include "duckdb/common/string_util.hpp"

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
vector<string> ReadFileSelections(const Value &value) {
	if (value.IsNull()) {
		throw BinderException("DuckPort arguments cannot be NULL");
	}
	if (value.type().id() != LogicalTypeId::LIST) {
		return {ReadSourceString(value)};
	}
	vector<string> selections;
	for (const auto &entry : ListValue::GetChildren(value)) {
		selections.push_back(ReadSourceString(entry));
	}
	if (selections.empty()) {
		throw BinderException("file list cannot be empty");
	}
	return selections;
}
vector<string> ResolveFiles(ClientContext &context, const vector<string> &selections, const string &format) {
	vector<string> files;
	for (const auto &selection : selections) {
		DuckportFileList *root_raw = nullptr;
		CheckError(duckport_file_discovery_root(selection.c_str(), &root_raw));
		FileListPtr root(root_raw, duckport_file_list_destroy);
		StorageParameters discovery(context, duckport_file_list_get(root.get(), 0), {});
		DuckportFileList *raw = nullptr;
		CheckError(duckport_resolve_files(selection.c_str(), format.c_str(), discovery.config, &raw));
		FileListPtr matches(raw, duckport_file_list_destroy);
		for (idx_t i = 0; i < duckport_file_list_count(matches.get()); i++) {
			files.emplace_back(duckport_file_list_get(matches.get(), i));
		}
	}
	return files;
}
SourcePtr OpenReaderSource(ClientContext &context, const TableFunctionBindInput &input, const ReaderInfo &info,
                           DuckportScanOptions scan) {
	auto files = info.format.empty() ? vector<string> {ReadSourceString(input.inputs[0])}
	                                 : ResolveFiles(context, ReadFileSelections(input.inputs[0]), info.format);
	// Keep file strings and credential snapshots alive until the FFI open copies them.
	vector<unique_ptr<StorageParameters>> storage;
	vector<DuckportSelectedFile> selected;
	if (files.size() > 1) {
		for (const auto &file : files) {
			storage.push_back(make_uniq<StorageParameters>(context, file, string()));
			selected.push_back({file.c_str(), storage.back()->config});
		}
		scan.files = selected.data();
		scan.file_count = selected.size();
	}
	DuckportSource *raw = nullptr;
	CheckError(info.open(context, files[0], scan, input, &raw));
	return SourcePtr(raw, duckport_source_destroy);
}
unique_ptr<FunctionData> BindReader(ClientContext &context, TableFunctionBindInput &input, vector<LogicalType> &types,
                                    vector<Identifier> &names) {
	CheckAccess(context);
	const auto &info = input.info->Cast<ReaderInfo>();
	auto scan = ReadScanOptions(context, input);
	auto concurrency = input.named_parameters.find("read_concurrency");
	const auto automatic = concurrency == input.named_parameters.end() ||
	                       !ReadConcurrency(Identifier("READ_CONCURRENCY"), concurrency->second);
	auto source = OpenReaderSource(context, input, info, scan);
	auto factory = make_shared_ptr<SourceFactory>(std::move(source), scan.read_concurrency, automatic);
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
optional<uint64_t> ConfigureCopySources(LogicalOperator &plan, idx_t read_concurrency, bool inspect_size,
                                        bool automatic_reads) {
	if (plan.type == LogicalOperatorType::LOGICAL_GET) {
		auto &get = plan.Cast<LogicalGet>();
		if (StringUtil::StartsWith(get.function.GetName().GetIdentifierName(), "duckport_scan_") &&
		    get.function.function == ArrowTableFunction::ArrowScanFunction) {
			return GetFactory(get.bind_data.get()).ConfigureCopy(read_concurrency, inspect_size, automatic_reads);
		}
		return nullopt;
	}
	// Joins, aggregates, limits and other transformations have unknown output size.
	const auto direct = plan.type == LogicalOperatorType::LOGICAL_PROJECTION && plan.children.size() == 1;
	optional<uint64_t> bytes;
	for (auto &child : plan.children) {
		auto child_bytes = ConfigureCopySources(*child, read_concurrency, inspect_size && direct, automatic_reads);
		if (direct) {
			bytes = child_bytes;
		}
	}
	return bytes;
}

static void RegisterReaderImpl(ExtensionLoader &loader, const string &name, ReaderBind open,
                               const std::function<void(TypedKwargs &)> &add_reader_options, bool files) {
	auto format = files ? name.substr(string("duckport_scan_").size()) : string();
	auto register_signature = [&](LogicalType input_type) {
		TableFunction function(Identifier(name), {input_type}, ArrowTableFunction::ArrowScanFunction, BindReader,
		                       InitReader, ArrowTableFunction::ArrowScanInitLocal);
		function.function_info = make_shared_ptr<ReaderInfo>(open, format);
		function.GetSignature().WithTypedKwargs("options", [&](TypedKwargs &options) {
			options.Add("batch_size", LogicalType::BIGINT);
			options.Add("read_concurrency", LogicalType::ANY);
			if (add_reader_options) {
				add_reader_options(options);
			}
		});
		function.to_string = [](TableFunctionToStringInput &input) {
			InsertionOrderPreservingMap<string> result;
			result["Read concurrency"] = GetFactory(input.bind_data.get()).ReadConcurrencyDescription();
			return result;
		};
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
