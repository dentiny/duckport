# JSON, JSONL and NDJSON

```sql
SELECT * FROM duckport_scan_json('events.json');
SELECT * FROM duckport_scan_jsonl('events.jsonl');
SELECT * FROM duckport_scan_ndjson('events.ndjson.gz', batch_size := 4096);
```

The three names use the same upstream reader for a stream of whitespace-separated
JSON objects, including newline-delimited objects. They do not provide different
framing modes based on the SQL function name. Top-level arrays are unsupported.

Schema inference uses a bounded sample; later incompatible values fail the
query. Accepts local/file/HTTP(S)/S3 objects with applicable outer decompression
and same-schema [file lists, directories and globs](files.md). Use native DuckDB
JSON readers for their additional schema-control options. See [storage](../storage.md) and
[secrets](../secrets.md).
