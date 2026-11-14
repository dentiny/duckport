#pragma once

#include "duckport.h"

extern "C" {
char *duckport_commoncrawl_open(const char *locator, const DuckportStorageOptions *storage_options,
								const DuckportScanOptions *scan_options, DuckportSource **output);
char *duckport_commoncrawl_segment_open(const char *locator, const DuckportStorageOptions *storage_options,
										const DuckportScanOptions *scan_options, DuckportSource **output);
}
