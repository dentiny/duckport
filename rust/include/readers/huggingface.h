#pragma once

#include "duckport.h"

struct HuggingFaceDatasetOptions {
	const char *revision;
	const char *config;
	const char *split;
	int32_t allow_partial; // 0 or 1
};

extern "C" {
char *duckport_huggingface_open(const char *locator, const StorageOptions *storage_options,
								const ScanOptions *scan_options, const HuggingFaceDatasetOptions *reader_options,
								DuckportSource **output);
}
