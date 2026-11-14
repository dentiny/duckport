#pragma once
#include "duckport.h"

struct LibraryOfCongressOptions {
	const char *file_name;
	const char *format;
	const char *record_path;
	const char *endpoint;
};

extern "C" {
char *duckport_library_of_congress_open(const char *locator, const DuckportStorageOptions &storage_options,
                                        const DuckportScanOptions &scan_options,
                                        const LibraryOfCongressOptions &reader_options, DuckportSource **output);
}
