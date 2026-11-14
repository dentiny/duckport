#pragma once

#include "source/reader.hpp"

namespace duckdb {
char *OpenHuggingFace(ClientContext &context, const string &locator, const DuckportScanOptions &scan,
					  const TableFunctionBindInput &input, DuckportSource **output);
} // namespace duckdb
