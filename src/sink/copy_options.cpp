#include "sink/copy_options.hpp"
#include "duckdb/common/string_util.hpp"
#include "duckdb/function/copy_function.hpp"
#include "duckdb/parser/parsed_data/copy_info.hpp"
#include <algorithm>
#include <iterator>

namespace duckdb {
namespace {
constexpr const char *INDEX_OPTIONS[] = {
    "scalar_index_columns",
    "vector_index_columns",
    "text_index_columns",
    "bloom_filter_index_columns",
};

bool IsText(const Value &value) {
	return !value.IsNull() && value.type().id() == LogicalTypeId::VARCHAR && !StringValue::Get(value).empty() &&
	       StringValue::Get(value).find('\0') == string::npos;
}

vector<string> ParseColumnNames(const Identifier &option, const vector<Value> &values, const char *description) {
	const auto &columns = values.size() == 1 && !values[0].IsNull() && values[0].type().id() == LogicalTypeId::LIST
	                          ? ListValue::GetChildren(values[0])
	                          : values;
	if (columns.empty()) {
		throw BinderException("%s requires at least one column", option);
	}
	vector<string> selected;
	for (const auto &column : columns) {
		if (!IsText(column)) {
			throw BinderException("%s requires nonempty text %s without NUL bytes", option, description);
		}
		selected.push_back(StringValue::Get(column));
	}
	return selected;
}

vector<string> ParseBlobColumns(const vector<Value> &values) {
	auto selected = ParseColumnNames(Identifier("BLOB_COLUMNS"), values, "paths");
	identifier_set_t seen;
	for (const auto &name : selected) {
		if (!seen.insert(Identifier(name)).second) {
			throw BinderException("Duplicate BLOB_COLUMNS column: %s", name);
		}
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
	if (values.size() != 1 || !IsText(values[0])) {
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
	CheckCopyOptionError(duckport_validate_byte_size(name.GetIdentifierName().c_str(), value->GetValue<int64_t>()));
	*target = value->GetValue<int64_t>();
	return true;
}

ColumnEncodingOptions &FindColumnEncoding(vector<ColumnEncodingOptions> &encodings, const string &column) {
	auto entry = std::find_if(encodings.begin(), encodings.end(), [&](const ColumnEncodingOptions &value) {
		return StringUtil::CIEquals(value.column, column);
	});
	if (entry == encodings.end()) {
		encodings.emplace_back();
		entry = encodings.end() - 1;
		entry->column = column;
	}
	return *entry;
}

void ParseColumnSetting(const Identifier &name, const Identifier &column, const Value &setting,
                        ColumnEncodingOptions &entry) {
	if (name == "column_compression_level") {
		auto level = setting.DefaultTryCastAs(LogicalType::INTEGER);
		if (!setting.type().IsIntegral() || !level) {
			throw BinderException("COLUMN_COMPRESSION_LEVEL requires integer levels: %s", column);
		}
		entry.has_compression_level = true;
		entry.compression_level = level->GetValue<int32_t>();
	} else {
		if (!IsText(setting)) {
			throw BinderException("%s requires nonempty text settings without NUL bytes: %s", name, column);
		}
		auto &target = name == "column_compression" ? entry.compression : entry.structural_encoding;
		target = StringValue::Get(setting);
	}
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
		auto &encoding = FindColumnEncoding(options.column_encodings, column.GetIdentifierName());
		ParseColumnSetting(name, column, settings[i], encoding);
	}
	return true;
}

bool ParseIndexOption(const Identifier &name, const vector<Value> &values, SinkWriteOptions &options) {
	if (std::none_of(std::begin(INDEX_OPTIONS), std::end(INDEX_OPTIONS),
	                 [&](const char *option) { return name == option; })) {
		return false;
	}
	for (const auto &column : ParseColumnNames(name, values, "column names")) {
		options.indexes.push_back(IndexOptions {
		    .column = column,
		    .option = name.GetIdentifierName(),
		});
	}
	return true;
}

int64_t ParseConcurrency(const Identifier &name, const vector<Value> &values) {
	if (values.size() != 1 || values[0].IsNull() || !values[0].type().IsIntegral()) {
		throw BinderException("%s requires one integer", name);
	}
	auto value = values[0].DefaultTryCastAs(LogicalType::BIGINT);
	if (!value || value->GetValue<int64_t>() <= 0) {
		throw BinderException("%s requires a positive integer that fits in BIGINT", name);
	}
	return value->GetValue<int64_t>();
}

bool ParseBooleanOption(ClientContext &context, const Identifier &name, const vector<Value> &values) {
	if (values.size() > 1 || (!values.empty() && values[0].IsNull())) {
		throw BinderException("%s requires a boolean", name);
	}
	return values.empty() || values[0].CastAs(context, LogicalType::BOOLEAN).GetValue<bool>();
}

void ParseWriteMode(ClientContext &context, const Identifier &name, const vector<Value> &values,
                    SinkWriteOptions &options, bool &has_mode) {
	if (name != "append" && name != "overwrite") {
		throw BinderException("Unsupported DuckPort COPY option: %s", name);
	}
	if (has_mode) {
		throw BinderException("Only one of APPEND or OVERWRITE can be specified");
	}
	has_mode = true;
	if (ParseBooleanOption(context, name, values)) {
		options.mode = name == "append" ? SinkWriteMode::APPEND : SinkWriteMode::OVERWRITE;
	}
}

void ParseCopyOption(ClientContext &context, const Identifier &name, const vector<Value> &values,
                     SinkWriteOptions &options, bool &has_mode) {
	if (ParseSizeOption(name, values, options.write_options) || ParseIndexOption(name, values, options) ||
	    ParseColumnOption(name, values, options) || ParseTextOption(name, values, options)) {
		return;
	}
	if (name == "blob_columns") {
		options.blob_columns = ParseBlobColumns(values);
	} else if (name == "write_concurrency") {
		options.write_concurrency = ParseConcurrency(name, values);
	} else if (name == "read_concurrency") {
		options.read_concurrency = ParseConcurrency(name, values);
	} else if (name == "preserve_order") {
		options.preserve_order = ParseBooleanOption(context, Identifier("PRESERVE_ORDER"), values);
	} else {
		ParseWriteMode(context, name, values, options, has_mode);
	}
}
} // namespace

void CheckCopyOptionError(char *error) {
	unique_ptr<char, decltype(&duckport_source_error_free)> owned(error, duckport_source_error_free);
	if (owned) {
		throw BinderException("%s", owned.get());
	}
}

SinkWriteOptions ParseCopyOptions(ClientContext &context, const CopyInfo &info) {
	SinkWriteOptions options;
	bool has_mode = false;
	for (const auto &entry : info.options) {
		ParseCopyOption(context, entry.first, entry.second, options, has_mode);
	}
	if (options.write_concurrency) {
		CheckCopyOptionError(duckport_validate_write_concurrency(*options.write_concurrency, options.preserve_order));
	}
	return options;
}

void RegisterCopyOptions(ClientContext &, CopyOptionsInput &input) {
	for (const auto &name : {"catalog_uri", "warehouse", "storage_root", "storage_version"}) {
		input.options[name] = CopyOption(LogicalType::VARCHAR, CopyOptionMode::WRITE_ONLY);
	}
	input.options["blob_columns"] = CopyOption(LogicalType::LIST(LogicalType::VARCHAR), CopyOptionMode::WRITE_ONLY);
	for (const auto &option : INDEX_OPTIONS) {
		input.options[option] = CopyOption(LogicalType::LIST(LogicalType::VARCHAR), CopyOptionMode::WRITE_ONLY);
	}
	input.options["append"] = CopyOption(LogicalType::BOOLEAN, CopyOptionMode::WRITE_ONLY);
	input.options["overwrite"] = CopyOption(LogicalType::BOOLEAN, CopyOptionMode::WRITE_ONLY);
	input.options["preserve_order"] = CopyOption(LogicalType::BOOLEAN, CopyOptionMode::WRITE_ONLY);
	// Preserve original values for validation.
	for (const auto &option :
	     {"read_concurrency", "write_concurrency", "target_file_size", "blob_inline_size_threshold",
	      "blob_dedicated_size_threshold", "sample_percent", "sample_rows", "column_compression",
	      "column_compression_level", "column_structural_encoding", "generated_columns"}) {
		input.options[option] = CopyOption(LogicalType::ANY, CopyOptionMode::WRITE_ONLY);
	}
}
} // namespace duckdb
