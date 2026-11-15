#pragma once
#include "duckport.h"

struct BiorxivOptions {
	const char *file_name;
	const char *format;
	const char *record_path;
	const char *endpoint;
	const char *until;
	const char *server;
	const char *cursor;
};

extern "C" {
char *duckport_biorxiv_open(const char *locator, const DuckportStorageOptions &storage_options,
                            const DuckportScanOptions &scan_options, const BiorxivOptions &reader_options,
                            DuckportSource **output);
}
