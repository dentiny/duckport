#pragma once
#include "source/reader.hpp"

namespace duckdb {
char *OpenRegulations(ClientContext &context, const string &locator, const DuckportScanOptions &scan,
                      const TableFunctionBindInput &input, DuckportSource **output);
}
