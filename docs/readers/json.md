# JSON, JSONL, and NDJSON

```sql
SELECT * FROM duckport_scan_json('events.json');
SELECT * FROM duckport_scan_jsonl('events.jsonl');
SELECT * FROM duckport_scan_ndjson('events.ndjson');
```

All three functions use the same reader for whitespace-separated JSON objects, including newline-delimited records. Schema inference uses a bounded sample. Top-level arrays are unsupported; later incompatible records fail the query.

See [shared settings](../../README.md#read) and [test instructions](../../test/README.md).
