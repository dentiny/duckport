#pragma once
#include "source/reader.hpp"
namespace duckdb {
char *OpenPmc(ClientContext &context, const string &locator, const DuckportScanOptions &scan,
              const TableFunctionBindInput &input, DuckportSource **output);
char *OpenPmcShard(ClientContext &context, const string &locator, const DuckportScanOptions &scan,
                   const TableFunctionBindInput &input, DuckportSource **output);
} // namespace duckdb
