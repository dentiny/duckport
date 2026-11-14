#pragma once

#include "duckport.h"

extern "C" {
char *duckport_pmc_open(const char *locator, const DuckportStorageOptions &storage_options,
                        const DuckportScanOptions &scan_options, DuckportSource **output);
char *duckport_pmc_inventory_open(const char *locator, const DuckportStorageOptions &storage_options,
                                  const DuckportScanOptions &scan_options, DuckportSource **output);
}
