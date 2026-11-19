#pragma once
#include "source/reader.hpp"
namespace duckdb {
char *OpenPmcMetadata(ClientContext &context, const string &locator, const DuckportScanOptions &scan,
                      const TableFunctionBindInput &input, DuckportSource **output);
char *OpenPmcInventoryShard(ClientContext &context, const string &locator, const DuckportScanOptions &scan,
                            const TableFunctionBindInput &input, DuckportSource **output);
} // namespace duckdb
