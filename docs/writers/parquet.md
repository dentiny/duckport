# Parquet dataset directories

```sql
COPY events TO 'events_dataset'
(FORMAT duckport_parquet, TARGET_FILE_SIZE 268435456);
COPY events TO 'events_dataset' (FORMAT duckport_parquet, APPEND);
SELECT * FROM read_parquet('events_dataset/*.parquet');
```

`open-arrow-sink-parquet` writes a directory of `part-*.parquet` files. Create
requires a missing destination; APPEND requires an existing directory with a
matching schema. An empty create still writes a readable schema/footer.
Local paths, file URIs and S3 use the [scoped storage secret](../secrets.md).

TARGET_FILE_SIZE is a positive soft byte target. FILE_COUNT controls writer
streams, not exact file count. Both [ordering modes](README.md#copy-ordering),
[sampling](README.md#copy-sampling), [generated columns](README.md#generated-columns)
and supported top-level [column codecs](README.md#column-compression-and-encoding)
are available. Directory scan order is not the write-order guarantee.

OVERWRITE, BLOB_COLUMNS, blob thresholds, storage version, structural encoding,
nested column-codec paths and indexes are unsupported. URLs and ATIF media
references remain values; they are not downloaded. Native binary/nested fields
retain the supported Arrow representation.

Files can become visible before commit. APPEND is not a transactional directory
update and failed writes can leave incomplete files. Use native `FORMAT PARQUET`
when the output must be one file rather than a dataset directory.
