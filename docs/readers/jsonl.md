# duckport_read_jsonl

```sql
SELECT * FROM duckport_read_jsonl('events.jsonl.gz');
```

Reads newline-delimited JSON objects. Schema inference is bounded; later incompatible records fail the query.

See [shared settings](../../README.md#shared-settings) and [test instructions](../../test/README.md).
