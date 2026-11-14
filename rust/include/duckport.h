#pragma once
#include <cstddef>
#include <cstdint>

struct ArrowSchema;
struct ArrowArrayStream;
struct DuckportSource;

// Optional strings are null when unset and borrowed only for the open call.
struct DuckportStorageOptions {
	const char *endpoint;
	const char *region;
	const char *access_key_id;
	const char *secret_access_key;
	const char *session_token;
	const char *token;
	int32_t virtual_host_style; // 0 or 1
	int32_t allow_anonymous;	// 0 or 1
};
struct DuckportScanOptions {
	// Target rows per Arrow record batch; SQL default: 8192.
	size_t batch_size;
};
extern "C" {
// Common functions
char *duckport_source_get_schema(const DuckportSource *, ArrowSchema *);
char *duckport_source_get_stream(const DuckportSource *, ArrowArrayStream *);
void duckport_source_destroy(DuckportSource *);
void duckport_source_error_free(char *);

// Format-related functions
char *duckport_parquet_open(const char *locator, const DuckportStorageOptions *storage_options,
							const DuckportScanOptions *scan_options, DuckportSource **output);

// Storage service-related functions (future)
}
