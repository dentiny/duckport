# Iceberg sink

Uses `open-arrow-sink-iceberg` 0.3.0 and the Iceberg Rust REST catalog.

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

Create the namespace in the catalog first. A COPY without `APPEND` creates a new
unpartitioned table from the query schema, assigns Iceberg field IDs (including
nested fields), and writes the query results. The catalog chooses the table location,
which must be accessible through `STORAGE_ROOT`. An existing destination causes an
error; it is never replaced. For example:

```sql
COPY (SELECT id, value FROM input_table)
TO 'analytics.new_events' (
    FORMAT duckport_iceberg,
    CATALOG_URI 'http://localhost:8181',
    WAREHOUSE 'warehouse',
    STORAGE_ROOT 's3://bucket/'
);
```

Use `APPEND` to write to an existing unpartitioned table. APPEND does not create
missing tables. The destination is a dot-separated namespace and table name
(nested namespaces are supported).
`CATALOG_URI` and `STORAGE_ROOT` are required. `WAREHOUSE` is optional and is
forwarded to the REST catalog. Configure a scoped DuckDB HTTP secret for catalog
bearer authentication. Catalog credentials are independent of data storage credentials.

`STORAGE_ROOT` selects data/metadata storage: a local directory, `file://` directory,
or `s3://bucket/`. All table locations must resolve within that local directory or
S3 bucket. Scoped DuckDB secrets are resolved against `STORAGE_ROOT`. The configured
OpenDAL operator handles data files, metadata, manifests and cleanup. Catalog-vended
storage credentials are not used; configure storage credentials through DuckDB secrets.

Overwrite, `BLOB_COLUMNS`, partitioned tables,
DuckDB file-rotation options and temporary-file publication are unsupported.
Both [COPY ordering modes](README.md#copy-ordering) are supported. Input
column names, order and types must match the catalog table. DuckPort attaches
Iceberg field IDs from the catalog schema; NULLs in required columns fail COPY.
Nested types must also match field names and nullability.

Rows publish as one Iceberg snapshot only after successful COPY finalization.
An empty APPEND leaves the current snapshot unchanged. An empty create still
creates the table schema. Table creation happens when the writer opens, before
data is committed; a failed COPY can leave an empty table in the catalog.
Failed or abandoned writes do not publish rows, but may leave uncommitted files. Commit errors propagate to
SQL; the catalog controls atomicity and conflict detection. Row order is unspecified.
External access must be enabled.

`TARGET_FILE_SIZE` accepts a positive integer byte count and sets a soft target
for data-file size; batches, row groups and file overhead can exceed it. Omit it
to preserve the backend default. This option is supported by all DuckPort sinks.

## Catalog authentication

Catalog requests use the best matching DuckDB HTTP secret for `CATALOG_URI`:

```sql
CREATE SECRET catalog_auth (
    TYPE http, BEARER_TOKEN 'catalog-access-token',
    SCOPE 'https://catalog.example.com'
);
```

This credential is independent of the storage secret selected by `STORAGE_ROOT`.
It covers catalog configuration, table loading and commits, not data/blob storage.
Without a matching catalog token, requests are anonymous. An explicitly empty
catalog token is rejected. Tokens are resolved at writer open and are not COPY
options.
