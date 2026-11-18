#pragma once
#include "duckport.h"

struct DoabOptions {
	const char *endpoint;
	const char *from_date;
	const char *until_date;
	const char *set_spec;
	const char *record_path;
	int64_t max_pages;
	int64_t max_records;
};

extern "C" {
char *duckport_doab_open(const char *locator, const DuckportStorageOptions &storage_options,
                         const DuckportScanOptions &scan_options, const DoabOptions &reader_options,
                         DuckportSource **output);
}
