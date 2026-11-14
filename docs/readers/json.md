# duckport_scan_json

```sql
SELECT * FROM duckport_scan_json('events.json');
```

Reads JSON objects with a bounded schema sample. Top-level arrays are unsupported; later incompatible records fail the query.

See [shared settings](../../README.md#read) and [test instructions](../../test/README.md).
