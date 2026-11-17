# duckport_scan_atif

```sql
SELECT * FROM duckport_scan_atif('testdata/atif/trajectory.json');
SELECT * FROM duckport_scan_atif('testdata/atif/trajectories.jsonl', batch_size := 2);
```

Reads ATIF trajectories with the library's fixed schema. JSON objects are framed automatically from their contents, accepting pretty-printed JSON and JSONL without inspecting filenames or selecting a layout.

See [shared settings](../../README.md#read) and [test instructions](../../test/README.md).

Media references remain text when scanned. To download them during Lance COPY,
select their nested paths explicitly with [BLOB_COLUMNS](../writers/lance.md#nested-blob_columns).
The option uses the existing Blob URL rules; relative paths must be qualified
before writing. Fields not selected by the option, including raw JSON and content
text, keep their ordinary storage policy.
