#pragma once
#include <cstddef>
#include <cstdint>

struct ArrowSchema;
struct ArrowArrayStream;
struct DuckportSource;
struct DuckportSink;
struct ArrowArray;

// Per-query remote storage policy; copied during open.
struct DuckportStoragePolicy {
	uint64_t timeout_ms;
	uint64_t io_timeout_ms;
	uint64_t retry_max_times;
	uint64_t retry_min_delay_ms;
	uint64_t retry_max_delay_ms;
	double retry_factor;
};

struct DuckportHttpHeader {
    const char *name;
    const char *value;
};

struct DuckportSecurityOptions {
    const char *access_key_id;
    const char *secret_access_key;
    const char *session_token;
    const char *token;
    const DuckportHttpHeader *http_headers;
    size_t http_header_count;
    int32_t allow_anonymous;
};

// Scoped HTTP credentials are borrowed during open and copied into Rust-owned storage.
struct DuckportHttpSecret {
    const char *scope;
    DuckportSecurityOptions security;
};

// Optional strings are null when unset and borrowed only for the open call.
struct DuckportStorageOptions {
    const char *endpoint;
    const char *region;
    int32_t virtual_host_style;
    DuckportStoragePolicy policy;
    DuckportSecurityOptions security;
    const DuckportHttpSecret *http_secrets;
    size_t http_secret_count;
};
// Borrowed during open; Rust owns independent copies for asynchronous downloads.
struct DuckportBlobSecret {
    const char *scope;
    const char *type;
    DuckportStorageOptions config;
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
DuckportStoragePolicy duckport_default_storage_policy();
void duckport_source_error_free(char *);
char *duckport_validate_byte_size(const char *name, int64_t value);
char *duckport_validate_file_count(int64_t value, bool preserve_order);
char *duckport_validate_sample_percent(double value);
char *duckport_validate_sample_rows(int64_t value, uint64_t max_rows);

// Source functions
char *duckport_source_get_schema(const DuckportSource *, ArrowSchema *);
// Projection names are borrowed for this call; zero columns preserves row counts.
char *duckport_source_get_stream(const DuckportSource *, const char *const *columns, size_t column_count,
                                ArrowArrayStream *);
void duckport_source_destroy(DuckportSource *);

// Format-related functions
char *duckport_parquet_source_open(const char *locator, const DuckportStorageOptions &storage_options,
                                   const DuckportScanOptions &scan_options, DuckportSource **output);

// Sink writer functions
char *duckport_lance_open(const char *path, const ArrowSchema *, const char *mode, const char *const *blob_columns,
                          size_t column_count, const DuckportWriteOptions *, const DuckportBlobSecret *, size_t secret_count, const DuckportStorageOptions *,
                          DuckportSink **output);
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
