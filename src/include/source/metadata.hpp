#pragma once

#include "duckdb/common/arrow/arrow.hpp"
#include "duckdb/common/types.hpp"

namespace duckdb {
void ImportSourceMetadata(vector<LogicalType> &types, const ArrowSchema &schema);
void ExportSourceMetadata(const vector<LogicalType> &types, ArrowSchema &schema);
vector<string> SourceLocations(const ArrowSchema &schema);
} // namespace duckdb
