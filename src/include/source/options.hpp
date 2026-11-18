#pragma once

#include "duckdb.hpp"
#include "duckport.h"
#include "duckdb/common/optional.hpp"

namespace duckdb {
string ReadSourceString(const Value &value);
optional<string> ReadStringOption(const TableFunctionBindInput &input, const char *name);
void ReadBooleanOption(const TableFunctionBindInput &input, const char *name, int32_t &output);
void ReadPaginationOptions(const TableFunctionBindInput &input, int64_t &max_pages, int64_t &max_records);
// nullopt is automatic concurrency; numeric strings are deliberately rejected.
optional<int64_t> ReadConcurrency(const Identifier &name, const Value &value);
idx_t AutoReadConcurrency(ClientContext &context);
DuckportScanOptions ReadScanOptions(ClientContext &context, const TableFunctionBindInput &input);
} // namespace duckdb
