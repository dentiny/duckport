#pragma once
#include "source/reader.hpp"

namespace duckdb {
char *OpenBhl(ClientContext &context, const string &locator, const DuckportScanOptions &scan,
              const TableFunctionBindInput &input, DuckportSource **output);
char *OpenBhlPages(ClientContext &context, const string &locator, const DuckportScanOptions &scan,
                   const TableFunctionBindInput &input, DuckportSource **output);
char *OpenBhlOpenData(ClientContext &context, const string &locator, const DuckportScanOptions &scan,
                      const TableFunctionBindInput &input, DuckportSource **output);
} // namespace duckdb
