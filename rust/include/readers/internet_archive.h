#pragma once
#include "duckport.h"

struct InternetArchiveOptions {
	const char *file_name;
	const char *format;
	const char *record_path;
	const char *endpoint;
};

extern "C" {
char *duckport_internet_archive_open(const char *locator, const DuckportStorageOptions &storage_options,
                                     const DuckportScanOptions &scan_options,
                                     const InternetArchiveOptions &reader_options, DuckportSource **output);
}
