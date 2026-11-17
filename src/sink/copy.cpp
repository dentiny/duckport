#include "sink/copy.hpp"
#include "duckdb/catalog/catalog.hpp"
#include "duckdb/catalog/catalog_entry/copy_function_catalog_entry.hpp"
#include "duckdb/common/arrow/arrow_converter.hpp"
#include "duckdb/common/arrow/arrow_wrapper.hpp"
#include "duckdb/common/string_util.hpp"
#include "duckdb/function/copy_function.hpp"
#include "duckdb/main/client_context.hpp"
#include "duckdb/main/query_result.hpp"
#include "duckdb/main/settings.hpp"
#include "duckdb/parser/expression/star_expression.hpp"
#include "duckdb/parser/parsed_data/sample_options.hpp"
#include "duckdb/parser/query_node/select_node.hpp"
#include "duckdb/parser/statement/select_statement.hpp"
#include "duckdb/parser/tableref/subqueryref.hpp"
#include "duckdb/planner/binder.hpp"
#include "duckdb/planner/bound_statement.hpp"
#include "duckdb/planner/logical_operator.hpp"
#include "duckdb/planner/operator/logical_copy_to_file.hpp"
#include "storage_options.hpp"
#include <algorithm>
#include <iterator>

namespace duckdb {
namespace {
constexpr const char *INDEX_OPTIONS[] = {
    "scalar_index_columns", "vector_index_columns", "text_index_columns", "bloom_filter_index_columns",
};

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
		       options.blob_columns == bind.options.blob_columns &&
		       options.write_options.blob_inline_size_threshold == bind.options.write_options.blob_inline_size_threshold &&
		       options.write_options.blob_dedicated_size_threshold == bind.options.write_options.blob_dedicated_size_threshold &&
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

vector<string> ParseBlobColumns(const vector<Value> &values, const vector<Identifier> &names,
                                const vector<LogicalType> &types) {
	const auto &columns =
	    values.size() == 1 && values[0].type().id() == LogicalTypeId::LIST ? ListValue::GetChildren(values[0]) : values;
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
		auto index = static_cast<idx_t>(match - names.begin());
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

bool ParseTextOption(const Identifier &name, const vector<Value> &values, SinkWriteOptions &options) {
	string *target = nullptr;
	if (name == "catalog_uri") {
		target = &options.catalog_uri;
	} else if (name == "warehouse") {
		target = &options.warehouse;
	} else if (name == "storage_root") {
		target = &options.storage_root;
	} else if (name == "storage_version") {
		target = &options.storage_version;
	} else {
		return false;
	}
	if (values.size() != 1 || values[0].IsNull() || values[0].type().id() != LogicalTypeId::VARCHAR ||
	    StringValue::Get(values[0]).empty() || StringValue::Get(values[0]).find('\0') != string::npos) {
		throw BinderException("%s requires nonempty text without NUL bytes", name);
	}
	*target = StringValue::Get(values[0]);
	return true;
}

bool ParseSizeOption(const Identifier &name, const vector<Value> &values, DuckportWriteOptions &options) {
	int64_t *target = nullptr;
	if (name == "blob_inline_size_threshold") {
		target = &options.blob_inline_size_threshold;
	} else if (name == "blob_dedicated_size_threshold") {
		target = &options.blob_dedicated_size_threshold;
	} else if (name == "target_file_size") {
		target = &options.target_file_size;
	} else {
		return false;
	}
	if (values.size() != 1 || values[0].IsNull() || !values[0].type().IsIntegral()) {
		throw BinderException("%s requires one integer byte size", name);
	}
	auto value = values[0].DefaultTryCastAs(LogicalType::BIGINT);
	if (!value) {
		throw BinderException("%s requires an integer byte size that fits in BIGINT", name);
	}
	unique_ptr<char, decltype(&duckport_source_error_free)> error(
	    duckport_validate_byte_size(name.GetIdentifierName().c_str(), value->GetValue<int64_t>()),
	    duckport_source_error_free);
	if (error) {
		throw BinderException("%s", error.get());
	}
	*target = value->GetValue<int64_t>();
	return true;
}

bool ParseColumnOption(const Identifier &name, const vector<Value> &values, SinkWriteOptions &options) {
	if (name != "column_compression" && name != "column_compression_level" && name != "column_structural_encoding") {
		return false;
	}
	if (values.size() != 1 || values[0].IsNull() || values[0].type().id() != LogicalTypeId::STRUCT) {
		throw BinderException("%s requires a struct mapping column names to settings", name);
	}
	const auto &fields = StructType::GetChildTypes(values[0].type());
	const auto &settings = StructValue::GetChildren(values[0]);
	identifier_set_t seen;
	for (idx_t i = 0; i < fields.size(); i++) {
		const auto &column = fields[i].first;
		if (column.GetIdentifierName().empty() || column.GetIdentifierName().find('\0') != string::npos ||
		    !seen.insert(column).second || settings[i].IsNull()) {
			throw BinderException("%s requires unique, nonempty column names and non-NULL settings", name);
		}
		auto entry = std::find_if(options.column_encodings.begin(), options.column_encodings.end(),
		                          [&](const ColumnEncodingOptions &value) {
			                          return StringUtil::CIEquals(value.column, column.GetIdentifierName());
		                          });
		if (entry == options.column_encodings.end()) {
			options.column_encodings.emplace_back();
			entry = options.column_encodings.end() - 1;
			entry->column = column.GetIdentifierName();
		}
		if (name == "column_compression_level") {
			auto level = settings[i].DefaultTryCastAs(LogicalType::INTEGER);
			if (!settings[i].type().IsIntegral() || !level) {
				throw BinderException("COLUMN_COMPRESSION_LEVEL requires integer levels: %s", column);
			}
			entry->has_compression_level = true;
			entry->compression_level = level->GetValue<int32_t>();
		} else {
			if (settings[i].type().id() != LogicalTypeId::VARCHAR || StringValue::Get(settings[i]).empty() ||
			    StringValue::Get(settings[i]).find('\0') != string::npos) {
				throw BinderException("%s requires nonempty text settings without NUL bytes: %s", name, column);
			}
			auto &target = name == "column_compression" ? entry->compression : entry->structural_encoding;
			target = StringValue::Get(settings[i]);
		}
	}
	return true;
}

bool ParseIndexOption(const Identifier &name, const vector<Value> &values, SinkWriteOptions &options) {
	if (std::none_of(std::begin(INDEX_OPTIONS), std::end(INDEX_OPTIONS),
	                 [&](const char *option) { return name == option; })) {
		return false;
	}
	const auto &columns = values.size() == 1 && !values[0].IsNull() && values[0].type().id() == LogicalTypeId::LIST
	                          ? ListValue::GetChildren(values[0])
	                          : values;
	if (columns.empty()) {
		throw BinderException("%s requires at least one column", name);
	}
	for (const auto &column : columns) {
		if (column.IsNull() || column.type().id() != LogicalTypeId::VARCHAR || StringValue::Get(column).empty() ||
		    StringValue::Get(column).find('\0') != string::npos) {
			throw BinderException("%s requires nonempty text column names without NUL bytes", name);
		}
		options.indexes.push_back(IndexOptions {
		    .column = StringValue::Get(column),
		    .option = name.GetIdentifierName(),
		});
	}
	return true;
}

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
	bool has_mode = false;
	for (auto &entry : input.info.options) {
		if (ParseSizeOption(entry.first, entry.second, result->options.write_options)) {
			continue;
		}
		if (ParseIndexOption(entry.first, entry.second, result->options)) {
			continue;
		}
		if (ParseColumnOption(entry.first, entry.second, result->options)) {
			continue;
		}
		if (ParseTextOption(entry.first, entry.second, result->options)) {
			continue;
		}
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
		throw NotImplementedException("DuckPort COPY does not support temporary, "
		                              "partitioned, or per-thread output");
	}
	auto state = make_uniq<CopyGlobalState>();
	state->properties = context.GetClientProperties();
	ArrowSchemaWrapper schema;
	ArrowConverter::ToArrowSchema(&schema.arrow_schema, bind.types, bind.names, state->properties);
	StorageParameters storage(context, bind.options.storage_root.empty() ? path : bind.options.storage_root, {});
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
unique_ptr<SampleOptions> TakeSampleOptions(ClientContext &context, CopyInfo &info) {
	auto percent = info.options.find(Identifier("sample_percent"));
	auto rows = info.options.find(Identifier("sample_rows"));
	if (percent != info.options.end() && rows != info.options.end()) {
		throw BinderException("Only one of SAMPLE_PERCENT or SAMPLE_ROWS can be specified");
	}
	auto entry = percent != info.options.end() ? percent : rows;
	if (entry == info.options.end()) {
		return nullptr;
	}
	bool is_percent = percent != info.options.end();
	const auto &values = entry->second;
	if (values.size() != 1 || values[0].IsNull() ||
	    (is_percent ? !values[0].type().IsNumeric() : !values[0].type().IsIntegral())) {
		throw BinderException("%s requires one %s", entry->first, is_percent ? "number" : "integer");
	}
	auto sample = make_uniq<SampleOptions>();
	sample->is_percentage = is_percent;
	sample->method = is_percent ? SampleMethod::BERNOULLI_SAMPLE : SampleMethod::RESERVOIR_SAMPLE;
	sample->sample_size = values[0].CastAs(context, is_percent ? LogicalType::DOUBLE : LogicalType::BIGINT);
	unique_ptr<char, decltype(&duckport_source_error_free)> error(
	    is_percent ? duckport_validate_sample_percent(sample->sample_size.GetValue<double>())
	               : duckport_validate_sample_rows(sample->sample_size.GetValue<int64_t>(), SampleOptions::MAX_SAMPLE_ROWS),
	    duckport_source_error_free);
	if (error) {
		throw BinderException("%s", error.get());
	}
	info.options.erase(entry);
	return sample;
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
	if (auto sample = TakeSampleOptions(binder.context, *info)) {
		auto subquery = make_uniq<SelectStatement>();
		subquery->node = std::move(query);
		auto sampled = make_uniq<SelectNode>();
		sampled->select_list.push_back(make_uniq<StarExpression>());
		sampled->from_table = make_uniq<SubqueryRef>(std::move(subquery));
		sampled->sample = std::move(sample);
		query = std::move(sampled);
	}
	auto source = binder.Bind(*query);
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
void CommonCopyOptions(ClientContext &, CopyOptionsInput &input) {
	for (const auto &name : {"catalog_uri", "warehouse", "storage_root", "storage_version"}) {
		input.options[name] = CopyOption(LogicalType::VARCHAR, CopyOptionMode::WRITE_ONLY);
	}
	input.options["blob_columns"] = CopyOption(LogicalType::LIST(LogicalType::VARCHAR), CopyOptionMode::WRITE_ONLY);
	for (const auto &option : INDEX_OPTIONS) {
		input.options[option] = CopyOption(LogicalType::LIST(LogicalType::VARCHAR), CopyOptionMode::WRITE_ONLY);
	}
	input.options["append"] = CopyOption(LogicalType::BOOLEAN, CopyOptionMode::WRITE_ONLY);
	input.options["overwrite"] = CopyOption(LogicalType::BOOLEAN, CopyOptionMode::WRITE_ONLY);
	// Preserve original values for validation.
	for (const auto &option : {"target_file_size", "blob_inline_size_threshold", "blob_dedicated_size_threshold",
	                          "sample_percent", "sample_rows", "column_compression", "column_compression_level",
	                          "column_structural_encoding"}) {
		input.options[option] = CopyOption(LogicalType::ANY, CopyOptionMode::WRITE_ONLY);
	}
}
} // namespace

void RegisterCopyFunction(ExtensionLoader &loader, const string &name, WriterOpen open) {
	CopyFunction function {Identifier(name)};
	function.function_info = make_shared_ptr<WriterInfo>(std::move(open));
	function.copy_options = CommonCopyOptions;
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
