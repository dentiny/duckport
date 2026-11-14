# duckport_scan_tsv

```sql
SELECT * FROM duckport_scan_tsv('people.tsv', header := true);
```

Uses the [CSV reader](csv.md) with a tab delimiter. Header inference, type inference, and error handling are shared.

See [shared settings](../../README.md#read) and [test instructions](../../test/README.md).
