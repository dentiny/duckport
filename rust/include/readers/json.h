#pragma once

#include "duckport.h"

extern "C" {
char *duckport_json_open(const char *locator, const StorageOptions *storage_options, const ScanOptions *scan_options,
						 DuckportSource **output);
}
