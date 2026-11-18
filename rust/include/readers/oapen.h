#pragma once
#include "duckport.h"

struct OapenOptions {
	const char *endpoint;
	const char *from_date;
	const char *until_date;
	const char *set_spec;
	const char *record_path;
	int64_t max_pages;
	int64_t max_records;
};

extern "C" {
char *duckport_oapen_open(const char *locator, const DuckportStorageOptions &storage_options,
                          const DuckportScanOptions &scan_options, const OapenOptions &reader_options,
                          DuckportSource **output);
}
