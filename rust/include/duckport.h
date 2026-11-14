#pragma once
#include <cstddef>

struct ArrowSchema;
struct ArrowArrayStream;
struct DuckportSource;

// JSON fields are maps of string-valued settings, borrowed only for the open call.
struct StorageOptions {
	const char *json;
};
struct ScanOptions {
	size_t batch_size;
	const char *json;
};
extern "C" {
// Common functions
char *duckport_source_get_schema(const DuckportSource *, ArrowSchema *);
char *duckport_source_get_stream(const DuckportSource *, ArrowArrayStream *);
void duckport_source_destroy(DuckportSource *);
void duckport_source_error_free(char *);

// Format-related functions
char *duckport_csv_open(const char *locator, const StorageOptions *storage_options, const ScanOptions *scan_options,
						DuckportSource **output);
char *duckport_parquet_open(const char *locator, const StorageOptions *storage_options, const ScanOptions *scan_options,
							DuckportSource **output);

// Storage service-related functions (future)
}
