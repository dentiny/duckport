#pragma once
#include "source/reader.hpp"

namespace duckdb {
char *OpenGovinfo(ClientContext &context, const string &locator, const DuckportScanOptions &scan,
                  const TableFunctionBindInput &input, DuckportSource **output);
}
