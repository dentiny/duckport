#pragma once
#include "duckport.h"

struct ArxivOptions {
	const char *kind;
	const char *month;
};

extern "C" {
char *duckport_arxiv_open(const char *locator, const DuckportStorageOptions &storage_options,
                          const DuckportScanOptions &scan_options, const ArxivOptions &reader_options,
                          DuckportSource **output);
}
