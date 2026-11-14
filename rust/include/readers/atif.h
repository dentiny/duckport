#pragma once

#include "duckport.h"

struct AtifOptions {
	int32_t jsonl; // 0: one JSON document; 1: one document per nonblank line
};

extern "C" {
char *duckport_atif_open(const char *locator, const DuckportStorageOptions &storage_options,
						 const DuckportScanOptions &scan_options, const AtifOptions &reader_options,
						 DuckportSource **output);
}
