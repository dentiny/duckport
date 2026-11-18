#pragma once

#include "duckport.h"

struct CsvOptions {
	int32_t header; // -1: infer; 0: no header; 1: header
	uint8_t delimiter;
};

extern "C" {
char *duckport_csv_open(const char *locator, const DuckportStorageOptions &storage_options,
                        const DuckportScanOptions &scan_options, const CsvOptions &reader_options,
                        DuckportSource **output);
}
