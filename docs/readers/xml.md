# XML records

```sql
SELECT "@id", "@owner"
FROM duckport_scan_xml('testdata/xml/inventory.xml',
    record_path := '/inventory/warehouse');
```

`record_path` is required and selects the repeated XML element that becomes a
row. The upstream reader infers a nested Arrow schema from a bounded sample;
subsequent incompatible records fail the scan. Attribute field names include
the `@` prefix and need SQL quoting when referenced.

This is a file reader, not an OAI harvester. Use the separate
[OAI-PMH functions](oai.md) for service pagination. XML selected from a
[provider attachment](providers.md) also requires `record_path`.
Local/file/HTTP(S)/S3 storage, outer decompression, `batch_size` and
[secret selection](../secrets.md) follow the shared reader conventions.
