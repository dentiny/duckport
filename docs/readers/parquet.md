# duckport_read_parquet

```sql
SELECT * FROM duckport_read_parquet('people.parquet');
```

Reads Parquet through open-arrow-access. Supports Snappy, Zstd, LZ4, gzip, and Brotli page codecs.

See [shared settings](../../README.md#shared-settings) and [test instructions](../../test/README.md).
