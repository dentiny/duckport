# TSV

```sql
SELECT * FROM duckport_scan_tsv('people.tsv', header := true);
SELECT * FROM duckport_scan_tsv('records.tsv.gz', batch_size := 4096);
```

This is the [CSV reader](csv.md) with a tab delimiter. It accepts optional
`header` and the shared `batch_size`, with the same schema inference,
all-string-header ambiguity and incompatible-record behavior. It has no SQL
`delimiter` option. Credentials and retries use the [shared storage setup](../storage.md).
