#include <algorithm>
#include <functional>
#include "source_metadata.hpp"

#include "duckdb/common/arrow/arrow_converter.hpp"
#include "duckdb/common/arrow/schema_metadata.hpp"
#include "duckdb/common/extension_type_info.hpp"

namespace duckdb {
namespace {

constexpr const char *MEDIA_DOCUMENT = "duckport:media_document";
constexpr const char *STORAGE_POLICY = "duckport:storage_policy";

LogicalType ImportField(const LogicalType &type, const ArrowSchema &schema) {
	auto metadata = schema.metadata ? ArrowSchemaMetadata(schema.metadata) : ArrowSchemaMetadata();
	auto result = type;
	if (type.id() == LogicalTypeId::STRUCT) {
		child_list_t<LogicalType> children;
		for (idx_t i = 0; i < StructType::GetChildCount(type); ++i) {
			children.emplace_back(StructType::GetChildName(type, i),
			                      ImportField(StructType::GetChildType(type, i), *schema.children[i]));
		}
		result = LogicalType::STRUCT(std::move(children)).WithAlias(type.GetAlias());
	} else if (type.id() == LogicalTypeId::LIST) {
		result = LogicalType::LIST(ImportField(ListType::GetChildType(type), *schema.children[0]))
		             .WithAlias(type.GetAlias());
	}
	if (metadata.GetOption(STORAGE_POLICY).empty() && metadata.GetOption(MEDIA_DOCUMENT).empty()) {
		return type.HasExtensionInfo()
		           ? result.WithExtensionInfo(make_uniq<ExtensionTypeInfo>(*type.GetExtensionInfo()))
		           : result;
	}
	auto info = type.HasExtensionInfo() ? make_uniq<ExtensionTypeInfo>(*type.GetExtensionInfo())
	                                    : make_uniq<ExtensionTypeInfo>();
	for (auto key : {STORAGE_POLICY, MEDIA_DOCUMENT}) {
		auto value = metadata.GetOption(key);
		if (!value.empty()) {
			info->properties[key] = Value(value);
		}
	}
	return result.WithExtensionInfo(std::move(info));
}

string Property(const LogicalType &type, const char *key) {
	auto info = type.GetExtensionInfo();
	if (!info) {
		return {};
	}
	auto entry = info->properties.find(key);
	return entry == info->properties.end() ? string() : entry->second.GetValue<string>();
}

void StoreMetadata(ArrowSchema &schema, ArrowSchemaMetadata &metadata, DuckDBArrowSchemaHolder &holder) {
	holder.metadata_info.emplace_back(metadata.SerializeMetadata());
	schema.metadata = holder.metadata_info.back().get();
}

void ExportField(const LogicalType &type, ArrowSchema &schema, DuckDBArrowSchemaHolder &holder) {
	auto metadata = schema.metadata ? ArrowSchemaMetadata(schema.metadata) : ArrowSchemaMetadata();
	bool tagged = false;
	for (auto key : {STORAGE_POLICY, MEDIA_DOCUMENT}) {
		auto value = Property(type, key);
		if (!value.empty()) {
			metadata.AddOption(key, value);
			tagged = true;
		}
	}
	if (tagged) {
		StoreMetadata(schema, metadata, holder);
	}
	if (type.id() == LogicalTypeId::STRUCT) {
		for (idx_t i = 0; i < StructType::GetChildCount(type); ++i) {
			ExportField(StructType::GetChildType(type, i), *schema.children[i], holder);
		}
	} else if (type.id() == LogicalTypeId::LIST || type.id() == LogicalTypeId::ARRAY) {
		const auto &child =
		    type.id() == LogicalTypeId::LIST ? ListType::GetChildType(type) : ArrayType::GetChildType(type);
		ExportField(child, *schema.children[0], holder);
	}
}

} // namespace

void ImportSourceMetadata(vector<LogicalType> &types, const ArrowSchema &schema) {
	for (idx_t i = 0; i < types.size(); ++i) {
		types[i] = ImportField(types[i], *schema.children[i]);
	}
}

void ExportSourceMetadata(const vector<LogicalType> &types, ArrowSchema &schema) {
	auto &holder = *static_cast<DuckDBArrowSchemaHolder *>(schema.private_data);
	for (idx_t i = 0; i < types.size(); ++i) {
		ExportField(types[i], *schema.children[i], holder);
	}
}

vector<string> SourceMediaDocuments(const ArrowSchema &schema) {
	vector<string> result;
	std::function<void(const ArrowSchema &)> visit = [&](const ArrowSchema &field) {
		auto metadata = field.metadata ? ArrowSchemaMetadata(field.metadata) : ArrowSchemaMetadata();
		auto document = metadata.GetOption(MEDIA_DOCUMENT);
		if (!document.empty() && std::find(result.begin(), result.end(), document) == result.end()) {
			result.push_back(document);
		}
		for (int64_t i = 0; i < field.n_children; ++i) {
			visit(*field.children[i]);
		}
	};
	visit(schema);
	return result;
}

} // namespace duckdb
