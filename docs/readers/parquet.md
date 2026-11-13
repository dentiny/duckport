# duckport_scan_parquet

```sql
SELECT * FROM duckport_scan_parquet('people.parquet');
SELECT * FROM duckport_scan_parquet('https://example.com/people.parquet');
SELECT * FROM duckport_scan_parquet('s3://bucket/people.parquet',
    storage_options := '{"region":"us-east-1"}');
```

Reads Parquet through open-arrow-access. Supports Snappy, Zstd, LZ4, gzip, and Brotli page codecs.

See [shared settings](../../README.md#read) and [test instructions](../../test/README.md).
