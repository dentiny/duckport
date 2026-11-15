#pragma once
#include "duckport.h"

struct SemanticScholarOptions {
	const char *file_name;
	const char *format;
	const char *record_path;
	const char *endpoint;
	const char *release;
};

extern "C" {
char *duckport_semantic_scholar_open(const char *locator, const DuckportStorageOptions &storage_options,
                                     const DuckportScanOptions &scan_options,
                                     const SemanticScholarOptions &reader_options, DuckportSource **output);
}
