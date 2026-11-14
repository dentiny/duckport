#pragma once

#include "duckport.h"

struct TsvOptions {
	int32_t header; // -1: infer; 0: no header; 1: header
};

extern "C" {
char *duckport_tsv_open(const char *locator, const DuckportStorageOptions *storage_options, const DuckportScanOptions *scan_options,
						const TsvOptions *reader_options, DuckportSource **output);
}
