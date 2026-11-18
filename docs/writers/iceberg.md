# Iceberg REST tables

DuckPort uses `open-arrow-sink-iceberg` 0.5.0 with the Iceberg Rust REST catalog.
Create the catalog namespace first, then create/write a table from the query:

```sql
COPY (SELECT 1 AS id, 'hello' AS body) TO 'analytics.events' (
    FORMAT duckport_iceberg,
    CATALOG_URI 'https://catalog.example.com', WAREHOUSE 'warehouse',
    STORAGE_ROOT 's3://warehouse-bucket/'
);
COPY (SELECT 2 AS id, 'world' AS body) TO 'analytics.events' (
    FORMAT duckport_iceberg, APPEND,
    CATALOG_URI 'https://catalog.example.com', WAREHOUSE 'warehouse',
    STORAGE_ROOT 's3://warehouse-bucket/', TARGET_FILE_SIZE 268435456
);
```

## Destination and storage

The destination is `namespace.table`, with dot-separated nested namespaces
supported. `CATALOG_URI` and `STORAGE_ROOT` are required; WAREHOUSE is optional.
CATALOG_URI is HTTP(S). STORAGE_ROOT is a local directory, file URI or S3 root
covering the catalog table location. S3 locations must stay in the configured
bucket; local locations must remain under the configured directory.

Catalog bearer credentials resolve against CATALOG_URI; storage credentials and
region/endpoint settings resolve independently against STORAGE_ROOT. The
configured OpenDAL storage handles data, metadata, manifests and cleanup;
catalog-vended storage credentials are not used. See the
[two-secret example](../secrets.md#iceberg-catalog-and-table-storage).

## Schema and modes

Default create infers an unpartitioned table schema from the query and assigns
Iceberg field IDs, including nested fields. Existing tables cause an error.
APPEND requires an existing unpartitioned table and does not create one.
Column names, order, compatible types and nested names/nullability must match.
DuckPort attaches the catalog's field IDs; NULLs in required fields fail.

An empty create still creates the table schema. An empty APPEND leaves the
current snapshot unchanged. Generated-expression metadata must match catalog
fields; mismatches fail instead of dropping that metadata.

OVERWRITE, BLOB_COLUMNS, blob thresholds, storage version, per-column codecs,
structural encoding, indexes and partitioned tables are unsupported. ATIF media
references are preserved without downloads. The shared
[ordering](README.md#copy-ordering), [sampling](README.md#copy-sampling),
[generated-column](README.md#generated-columns), WRITE_CONCURRENCY and TARGET_FILE_SIZE
options remain available subject to those schema constraints.

## Publication

Successful COPY commits rows in an Iceberg snapshot; the catalog handles
atomicity and conflicts. Table creation occurs at writer open, before data
commit: a failed create/write can leave an empty table. Failed/abandoned writes
can also leave uncommitted objects. A DuckDB transaction is not a transaction
across the external catalog/storage. Later table scans do not promise source
row order, even if writing used PRESERVE_ORDER.
