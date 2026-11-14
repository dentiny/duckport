# duckport_scan_jsonl

```sql
SELECT * FROM duckport_scan_jsonl('events.jsonl.gz');
```

Reads newline-delimited JSON objects. Schema inference is bounded; later incompatible records fail the query.

See [shared settings](../../README.md#read) and [test instructions](../../test/README.md).
