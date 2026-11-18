#pragma once

#include "duckdb.hpp"
#include "duckport.h"
#include <optional>

namespace duckdb {
string ReadSourceString(const Value &value);
std::optional<string> ReadStringOption(const TableFunctionBindInput &input, const char *name);
void ReadBooleanOption(const TableFunctionBindInput &input, const char *name, int32_t &output);
void ReadPaginationOptions(const TableFunctionBindInput &input, int64_t &max_pages, int64_t &max_records);
DuckportScanOptions ReadScanOptions(const TableFunctionBindInput &input);
} // namespace duckdb
