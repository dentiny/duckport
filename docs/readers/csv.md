# duckport_scan_csv

```sql
SELECT * FROM duckport_scan_csv('people.csv', header := true);
```

Infers column types and headers. Auto detection retains the first row of ambiguous
all-string input as data; set `header := true` when such a file has column names.
Set `header := false` to keep every row as data. Later incompatible records fail
the query.

`delimiter` selects a single ASCII separator (default `','`), using the same
upstream CSV quoting and batching behavior:

```sql
SELECT * FROM duckport_scan_csv('export.csv', delimiter := ';', header := true);
```

TSV uses the same reader with a tab delimiter; see [duckport_scan_tsv](tsv.md).

See [shared settings](../../README.md#read) and [test instructions](../../test/README.md).
