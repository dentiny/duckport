# duckport_scan_atif

```sql
SELECT * FROM duckport_scan_atif('testdata/atif/trajectory.json');
SELECT * FROM duckport_scan_atif('testdata/atif/trajectories.jsonl', batch_size := 2);
```

Reads ATIF trajectories with the library's fixed schema. JSON objects are framed automatically from their contents, accepting pretty-printed JSON and JSONL without inspecting filenames or selecting a layout.

See [shared settings](../../README.md#read) and [test instructions](../../test/README.md).
