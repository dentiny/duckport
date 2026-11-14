# duckport_scan_xml

```sql
SELECT "@id", "@owner"
FROM duckport_scan_xml('testdata/xml/inventory.xml', record_path := '/inventory/warehouse');
```

Requires `record_path`. Infers a schema from a bounded sample; later incompatible records fail the query.

See [shared settings](../../README.md#read) and [test instructions](../../test/README.md).
