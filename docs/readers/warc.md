# duckport_read_warc

```sql
SELECT * FROM duckport_read_warc('records.warc.gz');
```

Returns physical WARC records and binary bodies; it does not extract HTML text.

See [shared settings](../../README.md#shared-settings) and [test instructions](../../test/README.md).
