#pragma once

#include "duckport.h"

extern "C" {
char *duckport_gutenberg_open(const char *locator, const DuckportStorageOptions &storage_options,
                              const DuckportScanOptions &scan_options, DuckportSource **output);
}
