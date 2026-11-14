#pragma once
#include "source/reader.hpp"

namespace duckdb {
char *OpenBiorxiv(ClientContext &context, const string &locator, const DuckportScanOptions &scan,
                  const TableFunctionBindInput &input, DuckportSource **output);
}
