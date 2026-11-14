#pragma once
#include <cstddef>

struct ArrowSchema;
struct ArrowArrayStream;
struct DuckportSource;
extern "C" {
// Common functions
char *duckport_source_get_schema(const DuckportSource *, ArrowSchema *);
char *duckport_source_get_stream(const DuckportSource *, ArrowArrayStream *);
void duckport_source_destroy(DuckportSource *);
void duckport_source_error_free(char *);

// Format-related functions
char *duckport_jsonl_open(const char *locator, const char *options, size_t batch_size, DuckportSource **output);
char *duckport_parquet_open(const char *path, const char *storage_options, size_t batch_size, DuckportSource **output);

// Storage service-related functions (future)
}
