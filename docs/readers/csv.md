# duckport_scan_csv

```sql
SELECT * FROM duckport_scan_csv('people.csv', header := true);
```

Infers column types and headers. Set `header` explicitly for ambiguous input; later incompatible records fail the query.

TSV uses the same reader with a tab delimiter; see [duckport_scan_tsv](tsv.md).

See [shared settings](../../README.md#read) and [test instructions](../../test/README.md).
