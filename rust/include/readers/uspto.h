#pragma once
#include "duckport.h"

struct UsptoOptions {
	const char *file_name;
	const char *format;
	const char *record_path;
	const char *endpoint;
	const char *offset;
	const char *page_size;
};

extern "C" {
char *duckport_uspto_open(const char *locator, const DuckportStorageOptions &storage_options,
                          const DuckportScanOptions &scan_options, const UsptoOptions &reader_options,
                          DuckportSource **output);
}
