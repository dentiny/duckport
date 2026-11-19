#pragma once
#include "duckport.h"

struct PmcOptions {
	const char *content_format;
};
struct PmcShardOptions {
	const char *manifest;
	const char *shard;
	const char *content_format;
};

extern "C" {
char *duckport_pmc_open(const char *locator, const DuckportStorageOptions &storage_options,
                        const DuckportScanOptions &scan_options, const PmcOptions &reader_options,
                        DuckportSource **output);
char *duckport_pmc_shard_open(const char *locator, const DuckportStorageOptions &storage_options,
                              const DuckportScanOptions &scan_options, const PmcShardOptions &reader_options,
                              DuckportSource **output);
char *duckport_pmc_inventory_open(const char *locator, const DuckportStorageOptions &storage_options,
                                  const DuckportScanOptions &scan_options, DuckportSource **output);
}
