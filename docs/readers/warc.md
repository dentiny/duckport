# duckport_scan_warc

```sql
SELECT * FROM duckport_scan_warc('records.warc.gz');
```

Returns physical WARC records and binary bodies; it does not extract HTML text.

See [shared settings](../../README.md#read) and [test instructions](../../test/README.md).
