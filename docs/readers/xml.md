# duckport_read_xml

```sql
SELECT * FROM duckport_read_xml('records.xml', record_path := '/records/record');
```

Requires `record_path`. Infers a schema from a bounded sample; later incompatible records fail the query.

See [shared settings](../../README.md#shared-settings) and [test instructions](../../test/README.md).
