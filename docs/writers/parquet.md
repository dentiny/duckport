# Parquet sink

```sql
COPY input_table TO 'output_dataset' (FORMAT duckport_parquet, TARGET_FILE_SIZE 268435456);
COPY input_table TO 'output_dataset' (FORMAT duckport_parquet, APPEND);
SELECT * FROM read_parquet('output_dataset/*.parquet');
```

The released open-arrow-sink-parquet crate writes a directory of `part-*.parquet`
files. Create requires a missing destination; APPEND requires an existing directory
with matching schemas. Empty create writes a readable Parquet footer with the schema.
Local paths, file:// and S3 use the existing storage resolver and scoped DuckDB secrets.

OVERWRITE and BLOB_COLUMNS are unsupported by this backend. Text URLs remain text;
native BLOB values, NULLs and nested values retain their Arrow types. Both
[COPY ordering modes](README.md#copy-ordering) are supported; ordered writes do not
define directory scan order. Files can appear before commit; failures may leave incomplete files,
and append does not provide a directory transaction. Use native `FORMAT PARQUET`
when writing one file instead of a dataset directory.

`TARGET_FILE_SIZE` accepts a positive integer byte count and sets a soft target
for data-file size; batches, row groups and file overhead can exceed it. Omit it
to preserve the backend default. This option is supported by all DuckPort sinks.
