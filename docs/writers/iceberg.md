# Iceberg sink

Uses `open-arrow-sink-iceberg` 0.1.1 and the Iceberg Rust REST catalog.

```sql
COPY (SELECT id, value FROM input_table)
TO 'analytics.events' (
    FORMAT duckport_iceberg,
    APPEND,
    TARGET_FILE_SIZE 268435456,
    CATALOG_URI 'http://localhost:8181',
    WAREHOUSE 'warehouse',
    STORAGE_ROOT 's3://bucket/'
);
```

Create the namespace and an unpartitioned table in the catalog first. The destination
is a dot-separated namespace and table name (nested namespaces are supported).
`CATALOG_URI` and `STORAGE_ROOT` are required. `WAREHOUSE` is optional and is
forwarded to the REST catalog. Set `DUCKPORT_ICEBERG_TOKEN` in the process environment
for a catalog bearer token. Catalog credentials are independent of data storage credentials.

`STORAGE_ROOT` selects data/metadata storage: a local directory, `file://` directory,
or `s3://bucket/`. All table locations must resolve within that local directory or
S3 bucket. Scoped DuckDB secrets are resolved against `STORAGE_ROOT`. The configured
OpenDAL operator handles data files, metadata, manifests and cleanup. Catalog-vended
storage credentials are not used; configure storage credentials through DuckDB secrets.

`APPEND` is required; create, overwrite, `BLOB_COLUMNS`, partitioned tables,
DuckDB file-rotation options, temporary-file publication and ordering options are unsupported. Input
column names, order and types must match the catalog table. DuckPort attaches
Iceberg field IDs from the catalog schema; NULLs in required columns fail COPY.
Nested types must also match field names and nullability.

Rows publish as one Iceberg snapshot only after successful COPY finalization.
An empty input leaves the current snapshot unchanged. Failed or abandoned writes
do not publish rows, but may leave uncommitted files. Commit errors propagate to
SQL; the catalog controls atomicity and conflict detection. Row order is unspecified.
External access must be enabled.

`TARGET_FILE_SIZE` accepts a positive integer byte count and sets a soft target
for data-file size; batches, row groups and file overhead can exceed it. Omit it
to preserve the backend default. This option is supported by all DuckPort sinks.
