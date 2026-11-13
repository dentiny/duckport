# duckport_read_fs

```sql
SELECT * FROM duckport_read_fs('people.parquet', format := 'parquet');
```

Reads a local path or file URI. `format` is required and must name a format reader included in this checkout.

For CSV/TSV, pass `header`; for XML, pass `record_path`. The corresponding format PR must be merged first.

See [shared settings](../../README.md#shared-settings) and [test instructions](../../test/README.md).
