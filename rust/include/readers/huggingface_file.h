#pragma once

#include "duckport.h"

struct HuggingFaceFileOptions {
	const char *revision;
	const char *path;
	const char *format;
	int32_t header; // -1: infer; 0: no header; 1: header
	const char *record_path;
};

extern "C" {
char *duckport_huggingface_file_open(const char *locator, const DuckportStorageOptions &storage_options,
									 const DuckportScanOptions &scan_options,
									 const HuggingFaceFileOptions &reader_options, DuckportSource **output);
}
