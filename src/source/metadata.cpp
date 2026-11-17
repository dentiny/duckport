#include "source/metadata.hpp"
#include "duckdb/common/arrow/arrow_converter.hpp"
#include "duckdb/common/arrow/schema_metadata.hpp"
#include "duckdb/common/extension_type_info.hpp"
#include <set>

namespace duckdb {
namespace {
constexpr const char *KEYS[] = {"duckport:atif_storage", "duckport:atif_source"};

bool HasSourceMetadata(const ArrowSchema &schema) {
	auto metadata = schema.metadata ? ArrowSchemaMetadata(schema.metadata) : ArrowSchemaMetadata();
	for (auto key : KEYS) {
		if (!metadata.GetOption(key).empty()) {
			return true;
		}
	}
	for (int64_t i = 0; i < schema.n_children; i++) {
		if (HasSourceMetadata(*schema.children[i])) {
			return true;
		}
	}
	return false;
}

LogicalType ImportField(const LogicalType &type, const ArrowSchema &schema) {
	auto result = type;
	if (type.id() == LogicalTypeId::STRUCT) {
		child_list_t<LogicalType> children;
		for (idx_t i = 0; i < StructType::GetChildCount(type); i++) {
			children.emplace_back(StructType::GetChildName(type, i),
			                      ImportField(StructType::GetChildType(type, i), *schema.children[i]));
		}
		result = LogicalType::STRUCT(std::move(children)).WithAlias(type.GetAlias());
	} else if (type.id() == LogicalTypeId::LIST) {
		result = LogicalType::LIST(ImportField(ListType::GetChildType(type), *schema.children[0]))
		             .WithAlias(type.GetAlias());
	} else if (type.id() == LogicalTypeId::ARRAY) {
		result = LogicalType::ARRAY(ImportField(ArrayType::GetChildType(type), *schema.children[0]),
		                            ArrayType::GetSize(type))
		             .WithAlias(type.GetAlias());
	}
	auto info = type.HasExtensionInfo() ? make_uniq<ExtensionTypeInfo>(*type.GetExtensionInfo())
	                                    : make_uniq<ExtensionTypeInfo>();
	bool tagged = type.HasExtensionInfo();
	auto metadata = schema.metadata ? ArrowSchemaMetadata(schema.metadata) : ArrowSchemaMetadata();
	for (auto key : KEYS) {
		auto value = metadata.GetOption(key);
		if (!value.empty()) {
			info->properties[key] = Value(value);
			tagged = true;
		}
	}
	return tagged ? result.WithExtensionInfo(std::move(info)) : result;
}

void ExportField(const LogicalType &type, ArrowSchema &schema, DuckDBArrowSchemaHolder &holder) {
	if (auto info = type.GetExtensionInfo()) {
		auto metadata = schema.metadata ? ArrowSchemaMetadata(schema.metadata) : ArrowSchemaMetadata();
		bool tagged = false;
		for (auto key : KEYS) {
			auto entry = info->properties.find(key);
			if (entry != info->properties.end()) {
				metadata.AddOption(key, entry->second.GetValue<string>());
				tagged = true;
			}
		}
		if (tagged) {
			holder.metadata_info.emplace_back(metadata.SerializeMetadata());
			schema.metadata = holder.metadata_info.back().get();
		}
	}
	if (type.id() == LogicalTypeId::STRUCT) {
		for (idx_t i = 0; i < StructType::GetChildCount(type); i++) {
			ExportField(StructType::GetChildType(type, i), *schema.children[i], holder);
		}
	} else if (type.id() == LogicalTypeId::LIST) {
		ExportField(ListType::GetChildType(type), *schema.children[0], holder);
	} else if (type.id() == LogicalTypeId::ARRAY) {
		ExportField(ArrayType::GetChildType(type), *schema.children[0], holder);
	}
}

void CollectLocations(const ArrowSchema &schema, std::set<string> &locations) {
	auto metadata = schema.metadata ? ArrowSchemaMetadata(schema.metadata) : ArrowSchemaMetadata();
	auto source = metadata.GetOption("duckport:atif_source");
	if (!source.empty()) {
		locations.insert(std::move(source));
	}
	for (int64_t i = 0; i < schema.n_children; i++) {
		CollectLocations(*schema.children[i], locations);
	}
}
} // namespace

void ImportSourceMetadata(vector<LogicalType> &types, const ArrowSchema &schema) {
	if (!HasSourceMetadata(schema)) {
		return;
	}
	for (idx_t i = 0; i < types.size(); i++) {
		types[i] = ImportField(types[i], *schema.children[i]);
	}
}

void ExportSourceMetadata(const vector<LogicalType> &types, ArrowSchema &schema) {
	auto &holder = *static_cast<DuckDBArrowSchemaHolder *>(schema.private_data);
	for (idx_t i = 0; i < types.size(); i++) {
		ExportField(types[i], *schema.children[i], holder);
	}
}

vector<string> SourceLocations(const ArrowSchema &schema) {
	std::set<string> locations;
	CollectLocations(schema, locations);
	return vector<string>(locations.begin(), locations.end());
}
} // namespace duckdb
