#pragma once
#include "duckport.h"

struct OpenstaxOptions {
	const char *file_name;
	const char *format;
	const char *record_path;
	const char *endpoint;
	const char *revision;
	const char *archive_endpoint;
};

extern "C" {
char *duckport_openstax_open(const char *locator, const DuckportStorageOptions &storage_options,
                             const DuckportScanOptions &scan_options, const OpenstaxOptions &reader_options,
                             DuckportSource **output);
}
