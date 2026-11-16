#include "sink/copy.hpp"
#include "duckdb/common/arrow/arrow_converter.hpp"
#include "duckdb/catalog/catalog.hpp"
#include "duckdb/catalog/catalog_entry/copy_function_catalog_entry.hpp"
#include "duckdb/main/query_result.hpp"
#include "duckdb/planner/operator/logical_copy_to_file.hpp"
#include "duckdb/common/arrow/arrow_wrapper.hpp"
#include "duckdb/function/copy_function.hpp"
#include "duckdb/main/client_context.hpp"
#include "duckdb/main/settings.hpp"
#include "duckdb/planner/binder.hpp"
#include "duckdb/planner/bound_statement.hpp"
#include "duckdb/planner/logical_operator.hpp"
#include "storage_options.hpp"
#include <algorithm>

namespace duckdb {
namespace {
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
	string path;
	SinkWriteOptions options;
	shared_ptr<CopyFunctionInfo> info;

	unique_ptr<FunctionData> Copy() const override {
		auto result = make_uniq<CopyBindData>();
		result->types = types;
		result->names = names;
		result->path = path;
		result->options = options;
		result->info = info;
		return std::move(result);
	}
	bool Equals(const FunctionData &other) const override {
		auto &bind = other.Cast<CopyBindData>();
		return types == bind.types && names == bind.names && path == bind.path && options.mode == bind.options.mode &&
		       options.blob_columns == bind.options.blob_columns && info == bind.info;
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

vector<string> ParseBlobColumns(const vector<Value> &values, const vector<Identifier> &names,
                                const vector<LogicalType> &types) {
	const auto &columns = values.size() == 1 && values[0].type().id() == LogicalTypeId::LIST
	                          ? ListValue::GetChildren(values[0])
	                          : values;
	if (columns.empty()) {
		throw BinderException("BLOB_COLUMNS requires at least one column");
	}
	vector<string> selected;
	selected.reserve(columns.size());
	for (const auto &column : columns) {
		if (column.IsNull() || column.type().id() != LogicalTypeId::VARCHAR ||
		    StringValue::Get(column).find('\0') != string::npos) {
			throw BinderException("BLOB_COLUMNS requires non-NULL text column names without NUL bytes");
		}
		auto match = std::find(names.begin(), names.end(), Identifier(StringValue::Get(column)));
		if (match == names.end()) {
			throw BinderException("BLOB_COLUMNS column not found: %s", StringValue::Get(column));
		}
		auto index = idx_t(match - names.begin());
		if (types[index].id() != LogicalTypeId::VARCHAR) {
			throw BinderException("BLOB_COLUMNS column must be text: %s", *match);
		}
		const auto &name = match->GetIdentifierName();
		if (std::find(selected.begin(), selected.end(), name) != selected.end()) {
			throw BinderException("Duplicate BLOB_COLUMNS column: %s", name);
		}
		selected.push_back(name);
	}
	return selected;
}

unique_ptr<FunctionData> BindCopy(ClientContext &context, CopyFunctionBindInput &input, const vector<Identifier> &names,
                                  const vector<LogicalType> &types) {
	CheckWriteAccess(context);
	auto result = make_uniq<CopyBindData>();
	result->names = IdentifiersToStrings(names);
	result->types = types;
	result->path = input.info.file_path;
	result->info = input.function_info;
	bool has_mode = false;
	for (auto &entry : input.info.options) {
		if (entry.first == "blob_columns") {
			result->options.blob_columns = ParseBlobColumns(entry.second, names, types);
			continue;
		}
		if (entry.first != "append" && entry.first != "overwrite") {
			throw BinderException("Unsupported DuckPort COPY option: %s", entry.first);
		}
		if (has_mode) {
			throw BinderException("Only one of APPEND or OVERWRITE can be specified");
		}
		has_mode = true;
		if (entry.second.size() > 1 || (!entry.second.empty() && entry.second[0].IsNull())) {
			throw BinderException("%s requires a boolean", entry.first);
		}
		bool enabled = entry.second.empty() || entry.second[0].CastAs(context, LogicalType::BOOLEAN).GetValue<bool>();
		if (enabled) {
			result->options.mode = entry.first == "append" ? SinkWriteMode::APPEND : SinkWriteMode::OVERWRITE;
		}
	}
	return std::move(result);
}
unique_ptr<GlobalFunctionData> InitCopy(ClientContext &context, FunctionData &data, const string &path) {
	CheckWriteAccess(context);
	auto &bind = data.Cast<CopyBindData>();
	if (path != bind.path) {
		throw NotImplementedException("DuckPort COPY does not support temporary, partitioned, or per-thread output");
	}
	auto state = make_uniq<CopyGlobalState>();
	state->properties = context.GetClientProperties();
	ArrowSchemaWrapper schema;
	ArrowConverter::ToArrowSchema(&schema.arrow_schema, bind.types, bind.names, state->properties);
	StorageParameters storage(context, path, {});
	state->writer = bind.info->Cast<WriterInfo>().open(path, storage.config, bind.options, schema.arrow_schema);
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
	// Batches are written immediately; no local buffers to drain yet.
}
void FinalizeCopy(ClientContext &, FunctionData &, GlobalFunctionData &global) {
	auto &state = global.Cast<CopyGlobalState>();
	// Once publication starts, cleanup belongs to Commit, even if it throws.
	state.terminal = true;
	state.writer->Commit();
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
	auto source = binder.Bind(*info->select_statement);
	QueryResult::DeduplicateColumns(source.names);
	CopyFunctionBindInput input(*info, function.function_info);
	auto data = BindCopy(binder.context, input, source.names, source.types);
	auto copy = make_uniq<LogicalCopyToFile>(function, std::move(data), std::move(info), binder.GenerateTableIndex());
	copy->file_path = statement.info->file_path;
	copy->file_extension = function.extension;
	copy->use_tmp_file = false;
	copy->overwrite_mode = CopyOverwriteMode::COPY_ERROR_ON_CONFLICT;
	copy->per_thread_output = false;
	copy->rotate = false;
	copy->partition_output = false;
	copy->write_partition_columns = false;
	copy->preserve_order = PreserveOrderType::DONT_PRESERVE_ORDER;
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
} // namespace

void RegisterCopyFunction(ExtensionLoader &loader, const string &name, WriterOpen open) {
	CopyFunction function {Identifier(name)};
	function.function_info = make_shared_ptr<WriterInfo>(std::move(open));
	function.copy_options = [](ClientContext &, CopyOptionsInput &input) {
		input.options["blob_columns"] = CopyOption(LogicalType::LIST(LogicalType::VARCHAR), CopyOptionMode::WRITE_ONLY);
		input.options["append"] = CopyOption(LogicalType::BOOLEAN, CopyOptionMode::WRITE_ONLY);
		input.options["overwrite"] = CopyOption(LogicalType::BOOLEAN, CopyOptionMode::WRITE_ONLY);
	};
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
	for (const auto &format : {"parquet", "iceberg"}) {
		RegisterCopyFunction(loader, string("duckport_") + format,
		                     [format](const string &, const DuckportStorageOptions &, const SinkWriteOptions &,
		                              ArrowSchema &) -> unique_ptr<SinkWriter> {
			                     throw NotImplementedException("DuckPort %s sink is not implemented yet", format);
		                     });
	}
}
} // namespace duckdb
