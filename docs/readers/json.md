# duckport_scan_json

```sql
SELECT * FROM duckport_scan_json('events.json');
```

Reads whitespace-separated JSON objects with a bounded schema sample. `duckport_scan_jsonl` and `duckport_scan_ndjson` use this same reader. Top-level arrays are unsupported; later incompatible records fail the query.

See [shared settings](../../README.md#read) and [test instructions](../../test/README.md).
