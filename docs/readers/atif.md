# ATIF JSON and JSONL

```sql
SELECT * FROM duckport_scan_atif('testdata/atif/trajectory.json');
SELECT * FROM duckport_scan_atif_jsonl('testdata/atif/trajectories.jsonl', batch_size := 2);
```

Both functions use the same ATIF reader and fixed trajectory schema. `duckport_scan_atif` reads one JSON document; `duckport_scan_atif_jsonl` reads one trajectory per line.

See [shared settings](../../README.md#read) and [test instructions](../../test/README.md).
