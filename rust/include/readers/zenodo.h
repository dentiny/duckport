#pragma once
#include "duckport.h"

struct ZenodoOptions {
	const char *file_name;
	const char *format;
	const char *record_path;
	const char *endpoint;
};

extern "C" {
char *duckport_zenodo_open(const char *locator, const DuckportStorageOptions &storage_options,
                           const DuckportScanOptions &scan_options, const ZenodoOptions &reader_options,
                           DuckportSource **output);
}
