#pragma once
#include <cstddef>
#include <cstdint>

struct ArrowSchema;
struct ArrowArrayStream;
struct DuckportSource;
struct DuckportSink;
struct ArrowArray;

// Optional strings are null when unset and borrowed only for the open call.
struct DuckportStorageOptions {
	const char *endpoint;
	const char *region;
	const char *access_key_id;
	const char *secret_access_key;
	const char *session_token;
	const char *token;
	int32_t virtual_host_style; // 0 or 1
	int32_t allow_anonymous;    // 0 or 1
};
struct DuckportScanOptions {
	// Target rows per Arrow record batch; SQL default: 8192.
	size_t batch_size;
};
extern "C" {
// Common FFI support
void duckport_source_error_free(char *);

// Source functions
char *duckport_source_get_schema(const DuckportSource *, ArrowSchema *);
char *duckport_source_get_stream(const DuckportSource *, ArrowArrayStream *);
void duckport_source_destroy(DuckportSource *);

// Format-related functions
char *duckport_parquet_source_open(const char *locator, const DuckportStorageOptions &storage_options,
                                   const DuckportScanOptions &scan_options, DuckportSource **output);

// Sink writer functions
char *duckport_lance_open(const char *path, const ArrowSchema *, const char *mode, const char *const *blob_columns,
                          size_t column_count, const DuckportStorageOptions *, DuckportSink **output);
char *duckport_parquet_open(const char *path, const ArrowSchema *, const char *mode, const DuckportStorageOptions *,
                            DuckportSink **output);
char *duckport_sink_write(DuckportSink *, ArrowArray *);
char *duckport_sink_commit(DuckportSink *);
void duckport_sink_destroy(DuckportSink *);
}
