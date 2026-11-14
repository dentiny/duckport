# duckport_scan_atif

```sql
SELECT * FROM duckport_scan_atif('testdata/atif/trajectory.json');
SELECT * FROM duckport_scan_atif('testdata/atif/trajectories.jsonl', jsonl := true, batch_size := 2);
```

Reads ATIF trajectories with the library's fixed schema. By default, the input is one JSON document, including pretty-printed JSON. Set `jsonl := true` for one trajectory per nonblank line. The layout is explicit; filenames are not inspected.

See [shared settings](../../README.md#read) and [test instructions](../../test/README.md).
