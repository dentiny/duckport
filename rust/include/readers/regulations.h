#pragma once
#include "duckport.h"

struct RegulationsOptions {
	const char *file_name;
	const char *format;
	const char *record_path;
	const char *endpoint;
	const char *resource;
};

extern "C" {
char *duckport_regulations_open(const char *locator, const DuckportStorageOptions &storage_options,
                                const DuckportScanOptions &scan_options, const RegulationsOptions &reader_options,
                                DuckportSource **output);
}
