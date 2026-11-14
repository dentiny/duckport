#pragma once
#include "duckport.h"

struct CourtlistenerOptions {
	const char *table_name;
	const char *date;
};

extern "C" {
char *duckport_courtlistener_open(const char *locator, const DuckportStorageOptions &storage_options,
                                  const DuckportScanOptions &scan_options, const CourtlistenerOptions &reader_options,
                                  DuckportSource **output);
}
