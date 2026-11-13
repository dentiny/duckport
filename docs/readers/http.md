# duckport_read_http

```sql
SELECT * FROM duckport_read_http('https://example.com/people.parquet', format := 'parquet');
```

Reads an HTTP(S) file URL. `format` is required and must be included in this checkout. HTTP query/signed URLs are unsupported.

For CSV/TSV, pass `header`; for XML, pass `record_path`. The corresponding format PR must be merged first.

See [shared settings](../../README.md#shared-settings) and [test instructions](../../test/README.md).
