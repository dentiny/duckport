#pragma once
#include "duckport.h"

struct BhlOptions {
	const char *file_name;
	const char *format;
	const char *record_path;
	const char *endpoint;
};

struct BhlOpenDataOptions {
	const char *const *keys;
	size_t key_count;
	const char *format;
	const char *record_path;
};

struct BhlPagesOptions {
	const char *prefix;
	const char *metadata_db;
};

extern "C" {
char *duckport_bhl_pages_open(const char *locator, const DuckportStorageOptions &storage_options,
                              const DuckportScanOptions &scan_options, const BhlPagesOptions &reader_options,
                              DuckportSource **output);
char *duckport_bhl_open(const char *locator, const DuckportStorageOptions &storage_options,
                        const DuckportScanOptions &scan_options, const BhlOptions &reader_options,
                        DuckportSource **output);
char *duckport_bhl_open_data_open(const char *locator, const DuckportStorageOptions &storage_options,
                                  const DuckportScanOptions &scan_options, const BhlOpenDataOptions &reader_options,
                                  DuckportSource **output);
}
