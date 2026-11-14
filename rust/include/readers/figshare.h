#pragma once
#include "duckport.h"

struct FigshareOptions {
	const char *file_name;
	const char *format;
	const char *record_path;
	const char *endpoint;
	const char *version;
};

extern "C" {
char *duckport_figshare_open(const char *locator, const DuckportStorageOptions &storage_options,
                             const DuckportScanOptions &scan_options, const FigshareOptions &reader_options,
                             DuckportSource **output);
}
