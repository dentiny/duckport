#include "source_metadata.hpp"
#include "source/reader.hpp"
#include "source/options.hpp"
#include "sink/copy.hpp"
#include "sink/copy_options.hpp"
#include "sink/copy_query.hpp"
#include "sink/writer_context.hpp"
#include "duckdb/catalog/catalog.hpp"
#include "duckdb/catalog/catalog_entry/copy_function_catalog_entry.hpp"
#include "duckdb/common/arrow/arrow_converter.hpp"
#include "duckdb/common/arrow/arrow_wrapper.hpp"
#include "duckdb/common/arrow/schema_metadata.hpp"
#include "duckdb/function/copy_function.hpp"
#include "duckdb/main/client_context.hpp"
#include "duckdb/main/query_result.hpp"
#include "duckdb/main/settings.hpp"
#include "duckdb/main/query_profiler.hpp"
#include "duckdb/parallel/task_scheduler.hpp"
#include "duckdb/planner/binder.hpp"
#include "duckdb/planner/bound_statement.hpp"
#include "duckdb/planner/operator/logical_copy_to_file.hpp"
#include "storage_options.hpp"

namespace duckdb {
namespace {
constexpr const char *GENERATED_EXPRESSION_METADATA_KEY = "lance_conversion:generated_expression";

void CheckWriteAccess(ClientContext &context) {
	if (!Settings::Get<EnableExternalAccessSetting>(context)) {
		throw PermissionException("DuckPort COPY requires enable_external_access");
	}
}
struct WriterInfo : CopyFunctionInfo {
	explicit WriterInfo(WriterOpen open) : open(std::move(open)) {
	}
	WriterOpen open;
};
struct CopyBindData : FunctionData {
	vector<LogicalType> types;
	vector<string> names;
	vector<string> generated_expressions;
	string path;
	SinkWriteOptions options;
	shared_ptr<CopyFunctionInfo> info;

	unique_ptr<FunctionData> Copy() const override {
		auto result = make_uniq<CopyBindData>();
		result->types = types;
		result->names = names;
		result->generated_expressions = generated_expressions;
		result->path = path;
		result->options = options;
		result->info = info;
		return std::move(result);
	}
	bool Equals(const FunctionData &other) const override {
		auto &bind = other.Cast<CopyBindData>();
		return types == bind.types && names == bind.names && generated_expressions == bind.generated_expressions &&
		       path == bind.path && options.mode == bind.options.mode &&
		       options.blob_columns == bind.options.blob_columns &&
		       options.read_concurrency == bind.options.read_concurrency &&
		       options.read_concurrency_set == bind.options.read_concurrency_set &&
		       options.preserve_order == bind.options.preserve_order &&
		       options.write_concurrency == bind.options.write_concurrency &&
		       options.write_options.blob_inline_size_threshold ==
		           bind.options.write_options.blob_inline_size_threshold &&
		       options.write_options.blob_dedicated_size_threshold ==
		           bind.options.write_options.blob_dedicated_size_threshold &&
		       options.write_options.target_file_size == bind.options.write_options.target_file_size &&
		       options.catalog_uri == bind.options.catalog_uri && options.warehouse == bind.options.warehouse &&
		       options.storage_root == bind.options.storage_root &&
		       options.storage_version == bind.options.storage_version &&
		       options.column_encodings == bind.options.column_encodings && options.indexes == bind.options.indexes &&
		       info == bind.info;
	}
};
struct CopyGlobalState : GlobalFunctionData {
	unique_ptr<SinkWriter> writer;
	ClientProperties properties;
	bool terminal = false;

	~CopyGlobalState() override {
		if (writer && !terminal) {
			writer->Abort();
		}
	}
};
struct CopyLocalState : LocalFunctionData {};

unique_ptr<FunctionData> BindCopy(ClientContext &context, CopyFunctionBindInput &input, const vector<Identifier> &names,
                                  const vector<LogicalType> &types) {
	CheckWriteAccess(context);
	auto result = make_uniq<CopyBindData>();
	result->names = IdentifiersToStrings(names);
	result->types = types;
	result->path = input.info.file_path;
	if (result->path.find('\0') != string::npos) {
		throw BinderException("DuckPort COPY destination must not contain NUL bytes");
	}
	result->info = input.function_info;
	result->options = ParseCopyOptions(context, input.info);
	return std::move(result);
}
void ExportGeneratedExpressions(ArrowSchema &schema, const vector<string> &expressions) {
	for (idx_t column = 0; column < expressions.size(); column++) {
		if (expressions[column].empty()) {
			continue;
		}
		auto &field = *schema.children[column];
		auto metadata = field.metadata ? ArrowSchemaMetadata(field.metadata) : ArrowSchemaMetadata();
		metadata.AddOption(GENERATED_EXPRESSION_METADATA_KEY, expressions[column]);
		auto &holder = *static_cast<DuckDBArrowSchemaHolder *>(schema.private_data);
		holder.metadata_info.emplace_back(metadata.SerializeMetadata());
		field.metadata = holder.metadata_info.back().get();
	}
}

unique_ptr<GlobalFunctionData> InitCopy(ClientContext &context, FunctionData &data, const string &path) {
	CheckWriteAccess(context);
	auto &bind = data.Cast<CopyBindData>();
	if (path != bind.path) {
		throw NotImplementedException("DuckPort COPY does not support temporary, "
		                              "partitioned, or per-thread output");
	}
	auto state = make_uniq<CopyGlobalState>();
	state->properties = context.GetClientProperties();
	ArrowSchemaWrapper schema;
	ArrowConverter::ToArrowSchema(&schema.arrow_schema, bind.types, bind.names, state->properties);
	ExportSourceMetadata(bind.types, schema.arrow_schema);
	ExportGeneratedExpressions(schema.arrow_schema, bind.generated_expressions);
	StorageParameters storage(context, bind.options.storage_root.empty() ? path : bind.options.storage_root, {});
	auto options = bind.options;
	options.write_options.writer_count =
	    options.write_concurrency.value_or(options.preserve_order || !options.write_options.source_bytes_known
	                                           ? 1
	                                           : TaskScheduler::GetScheduler(context).NumberOfThreads());
	WriterOpenContext open_context(context, storage.config);
	state->writer = bind.info->Cast<WriterInfo>().open(path, open_context, options, schema.arrow_schema);
	if (!state->writer) {
		throw InternalException("DuckPort sink opener returned no writer");
	}
	return std::move(state);
}
void SinkCopy(ExecutionContext &, FunctionData &, GlobalFunctionData &global, LocalFunctionData &, DataChunk &input) {
	if (input.size() == 0) {
		return;
	}
	auto &state = global.Cast<CopyGlobalState>();
	ArrowArrayWrapper array;
	ArrowConverter::ToArrowArray(input, &array.arrow_array, state.properties, {});
	state.writer->Write(array.arrow_array);
}
void CombineCopy(ExecutionContext &, FunctionData &, GlobalFunctionData &, LocalFunctionData &) {
}
void FinalizeCopy(ClientContext &context, FunctionData &, GlobalFunctionData &global) {
	auto &state = global.Cast<CopyGlobalState>();
	// Once publication starts, cleanup belongs to Commit, even if it throws.
	state.terminal = true;
	state.writer->Commit();
	const auto metrics = state.writer->Metrics();
	auto &profiler = QueryProfiler::Get(context);
	profiler.TrackBytesRead(metrics.bytes_read);
	profiler.TrackBytesWritten(metrics.bytes_written);
}
BoundStatement CreateCopyPlan(Binder &binder, const CopyFunction &function, unique_ptr<CopyInfo> info,
                              unique_ptr<FunctionData> data, BoundStatement source, const string &path) {
	auto preserve_order = data->Cast<CopyBindData>().options.preserve_order;
	auto copy = make_uniq<LogicalCopyToFile>(function, std::move(data), std::move(info), binder.GenerateTableIndex());
	copy->file_path = path;
	copy->file_extension = function.extension;
	copy->use_tmp_file = false;
	copy->overwrite_mode = CopyOverwriteMode::COPY_ERROR_ON_CONFLICT;
	copy->per_thread_output = false;
	copy->rotate = false;
	copy->partition_output = false;
	copy->write_partition_columns = false;
	copy->preserve_order = preserve_order ? PreserveOrderType::PRESERVE_ORDER : PreserveOrderType::DONT_PRESERVE_ORDER;
	copy->return_type = CopyFunctionReturnType::CHANGED_ROWS;
	copy->names = source.names;
	copy->expected_types = source.types;
	copy->AddChild(std::move(source.plan));
	BoundStatement result;
	result.names = GetCopyFunctionReturnNames(copy->return_type);
	result.types = GetCopyFunctionReturnLogicalTypes(copy->return_type);
	result.plan = std::move(copy);
	return result;
}

// Like duckdb_lance_conversion, own the logical COPY plan: dataset backends
// control overwrite, naming and publication, rather than DuckDB's file helpers.
BoundStatement PlanCopy(Binder &binder, CopyStatement &statement) {
	CheckWriteAccess(binder.context);
	auto info = statement.info->Copy();
	if (!info->parsed_options.empty()) {
		throw NotImplementedException("DuckPort COPY does not support PARTITION_PATH");
	}
	auto &entry = Catalog::GetEntry<CopyFunctionCatalogEntry>(
	    binder.context,
	    QualifiedName(Identifier(SYSTEM_CATALOG), Identifier::DefaultSchema(), Identifier(info->format)));
	auto function = entry.function;
	function.plan = nullptr;
	auto query = info->select_statement->Copy();
	ApplyCopyExtraOptions(*query, *info);
	auto generated = TakeGeneratedColumns(binder.context, *info);
	if (auto sample = TakeSampleOptions(binder.context, *info)) {
		query = ApplyCopySample(std::move(query), std::move(sample));
	}
	if (!generated.empty()) {
		query = ApplyGeneratedColumns(std::move(query), generated);
	}
	auto source = binder.Bind(*query);
	QueryResult::DeduplicateColumns(source.names);
	if (!generated.empty()) {
		ValidateBoundGeneratedColumns(source, generated);
	}
	CopyFunctionBindInput input(*info, function.function_info);
	auto data = BindCopy(binder.context, input, source.names, source.types);
	auto &options = data->Cast<CopyBindData>().options;
	const auto automatic = !options.preserve_order && !options.write_concurrency &&
	                       options.write_options.target_file_size > 0 && options.blob_columns.empty() &&
	                       generated.empty();
	const auto read_concurrency =
	    options.read_concurrency_set ? options.read_concurrency.value_or(AutoReadConcurrency(binder.context)) : 0;
	auto bytes = ConfigureCopySources(*source.plan, read_concurrency, automatic, !options.read_concurrency);
	if (bytes) {
		options.write_options.source_bytes = *bytes;
		options.write_options.source_bytes_known = true;
	}
	if (!generated.empty()) {
		auto &expressions = data->Cast<CopyBindData>().generated_expressions;
		expressions.resize(source.names.size() - generated.size());
		for (const auto &column : generated) {
			expressions.emplace_back(column->ToString());
		}
	}
	return CreateCopyPlan(binder, function, std::move(info), std::move(data), std::move(source),
	                      statement.info->file_path);
}
} // namespace

void RegisterCopyFunction(ExtensionLoader &loader, const string &name, WriterOpen open) {
	CopyFunction function {Identifier(name)};
	function.function_info = make_shared_ptr<WriterInfo>(std::move(open));
	function.copy_options = RegisterCopyOptions;
	function.copy_to_bind = BindCopy;
	function.copy_to_initialize_global = InitCopy;
	function.copy_to_initialize_local = [](ExecutionContext &, FunctionData &) -> unique_ptr<LocalFunctionData> {
		return make_uniq<CopyLocalState>();
	};
	function.copy_to_sink = SinkCopy;
	function.copy_to_combine = CombineCopy;
	function.copy_to_finalize = FinalizeCopy;
	function.execution_mode = [](bool, bool) {
		return CopyFunctionExecutionMode::REGULAR_COPY_TO_FILE;
	};
	function.plan = PlanCopy;
	loader.RegisterFunction(function);
}

void RegisterCopyFunctions(ExtensionLoader &loader) {
	RegisterCopyFunction(loader, "duckport_lance", OpenLanceWriter);
	RegisterCopyFunction(loader, "duckport_parquet", OpenParquetWriter);
	RegisterCopyFunction(loader, "duckport_iceberg", OpenIcebergWriter);
}
} // namespace duckdb
