# Parquet

```sql
SELECT id, name FROM duckport_scan_parquet('people.parquet');
SELECT * FROM duckport_scan_parquet('https://example.com/people.parquet');
SELECT count(*) FROM duckport_scan_parquet('s3://bucket/people.parquet');
```

Reads one Parquet object through open-arrow-access, using its schema and
projected Arrow reader. Enabled page codecs include Snappy, Zstd, LZ4, gzip and
Brotli. `batch_size` controls the row target. Nested-field and predicate pushdown
are not exposed by the DuckDB adapter.

Local paths, file URIs, HTTP(S) and S3 are supported. Archive-member paths,
outer-compressed `.parquet.gz`, query/signed URLs, directories, globs and file
lists are not accepted by this standalone reader. Use DuckDB's `read_parquet`
for native glob/directory/list queries; those queries can still feed any
DuckPort COPY sink. See [secrets](../secrets.md).
