#pragma once

#include "duckport.h"

extern "C" {
// Both functions return nullptr on success and write the source to *output.
// On failure, return an owned, NUL-terminated UTF-8 error string;
// release it with duckport_source_error_free().
char *duckport_commoncrawl_open(const char *locator, const DuckportStorageOptions &storage_options,
								const DuckportScanOptions &scan_options, DuckportSource **output);
char *duckport_commoncrawl_segment_open(const char *locator, const DuckportStorageOptions &storage_options,
										const DuckportScanOptions &scan_options, DuckportSource **output);
}
