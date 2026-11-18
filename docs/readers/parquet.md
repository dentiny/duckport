# Parquet

```sql
SELECT id, name FROM duckport_scan_parquet('people.parquet');
SELECT * FROM duckport_scan_parquet('https://example.com/people.parquet');
SELECT count(*) FROM duckport_scan_parquet('s3://bucket/people.parquet');
```

Reads Parquet objects through open-arrow-access, using its schema and
projected Arrow reader. Enabled page codecs include Snappy, Zstd, LZ4, gzip and
Brotli. `batch_size` controls the row target. Nested-field and predicate pushdown
are not exposed by the DuckDB adapter.

Local paths, file URIs, HTTP(S) and S3 are supported. Archive-member paths,
outer-compressed `.parquet.gz` and query/signed URLs are not accepted.
[File lists, directories and globs](files.md) require exactly matching schemas.
Native DuckDB readers remain available as sources for any DuckPort COPY sink. See [secrets](../secrets.md).
