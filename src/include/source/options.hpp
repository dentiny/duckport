#pragma once

#include "duckdb.hpp"
#include "duckport.h"

namespace duckdb {
string ReadSourceString(const Value &value);
void ReadStringOption(const TableFunctionBindInput &input, const char *name, string &owned, const char *&output);
void ReadBooleanOption(const TableFunctionBindInput &input, const char *name, int32_t &output);
::ScanOptions ReadScanOptions(const TableFunctionBindInput &input);
} // namespace duckdb
