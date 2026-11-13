# duckport_read_csv

```sql
SELECT * FROM duckport_read_csv('people.csv', header := true);
```

Infers column types and headers. Set `header` explicitly for ambiguous input; later incompatible records fail the query.

See [shared settings](../../README.md#shared-settings) and [test instructions](../../test/README.md).
