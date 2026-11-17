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
struct DuckportSourceStorage {
	const char *locator;
	DuckportStorageOptions storage;
};
struct DuckportScanOptions {
	// Target rows per Arrow record batch; SQL default: 8192.
	size_t batch_size;
};
struct DuckportColumnEncoding {
	const char *column;
	const char *compression;
	int32_t has_compression_level;
	int32_t compression_level;
	const char *structural_encoding;
};
struct DuckportIndexOptions {
	const char *column;
	const char *option;
};
// Byte sizes: -1 preserves the backend default; inline may be zero.
struct DuckportWriteOptions {
	int64_t blob_inline_size_threshold;
	int64_t blob_dedicated_size_threshold;
	int64_t target_file_size;
	const char *storage_version; // null when unset; borrowed during open
	const DuckportColumnEncoding *column_encodings;
	size_t column_encoding_count;
	const DuckportIndexOptions *indexes;
	size_t index_count;
	size_t writer_count;
};
extern "C" {
// Common FFI support
void duckport_source_error_free(char *);
char *duckport_validate_byte_size(const char *name, int64_t value);
char *duckport_validate_file_count(int64_t value, bool preserve_order);
char *duckport_validate_sample_percent(double value);
char *duckport_validate_sample_rows(int64_t value, uint64_t max_rows);

// Source functions
char *duckport_source_get_schema(const DuckportSource *, ArrowSchema *);
char *duckport_source_get_stream(const DuckportSource *, ArrowArrayStream *);
void duckport_source_destroy(DuckportSource *);

// Format-related functions
char *duckport_parquet_source_open(const char *locator, const DuckportStorageOptions &storage_options,
                                   const DuckportScanOptions &scan_options, DuckportSource **output);

// Sink writer functions
char *duckport_lance_open(const char *path, const ArrowSchema *, const char *mode, const char *const *blob_columns,
                          size_t column_count, const DuckportWriteOptions *, const DuckportStorageOptions *,
                          const DuckportSourceStorage *sources, size_t source_count, DuckportSink **output);
char *duckport_iceberg_open(const char *identifier, const ArrowSchema *, const char *mode, size_t column_count,
                            const char *catalog_uri, const char *warehouse, const char *storage_root,
                            const DuckportWriteOptions *, const DuckportStorageOptions *, DuckportSink **output);
char *duckport_parquet_open(const char *path, const ArrowSchema *, const char *mode, size_t column_count,
                            const DuckportWriteOptions *, const DuckportStorageOptions *, DuckportSink **output);

// Common sink functions
char *duckport_sink_write(DuckportSink *, ArrowArray *);
char *duckport_sink_commit(DuckportSink *);
void duckport_sink_destroy(DuckportSink *);
}
