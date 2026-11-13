# duckport_read_tsv

```sql
SELECT * FROM duckport_read_tsv('people.tsv', header := true);
```

Reads tab-separated values. Column types and headers are inferred; set `header` explicitly for ambiguous input.

See [shared settings](../../README.md#shared-settings) and [test instructions](../../test/README.md).
