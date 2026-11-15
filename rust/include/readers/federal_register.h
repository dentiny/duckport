#pragma once
#include "duckport.h"

struct FederalRegisterOptions {
	const char *file_name;
	const char *format;
	const char *record_path;
	const char *endpoint;
};

extern "C" {
char *duckport_federal_register_open(const char *locator, const DuckportStorageOptions &storage_options,
                                     const DuckportScanOptions &scan_options,
                                     const FederalRegisterOptions &reader_options, DuckportSource **output);
}
