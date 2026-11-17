#pragma once

#include "duckdb/common/arrow/arrow.hpp"
#include "duckdb/common/types.hpp"

namespace duckdb {

// At scan binding, copy DuckPort storage-policy and source-document field metadata
// into serializable LogicalType properties, including nested struct/list fields.
void ImportSourceMetadata(vector<LogicalType> &types, const ArrowSchema &schema);
// During COPY, restore those properties onto the exported Arrow fields. The schema
// must come from ArrowConverter::ToArrowSchema, whose holder owns the metadata.
void ExportSourceMetadata(const vector<LogicalType> &types, ArrowSchema &schema);

// Collect distinct source-document URIs from field metadata, including nested fields.
// COPY uses them to resolve source credentials independently of the destination.
vector<string> SourceMediaDocuments(const ArrowSchema &schema);

} // namespace duckdb
