# CSV

```sql
SELECT * FROM duckport_scan_csv('people.csv', header := true);
SELECT * FROM duckport_scan_csv('export.csv', header := true, delimiter := ';');
SELECT * FROM duckport_scan_csv('data.csv.gz', header := false, batch_size := 4096);
```

The reader infers columns/types from a bounded sample. `header` is optional;
auto detection preserves an ambiguous all-string first row as data. Specify true
when that row contains names, or false when every row is data. Later records
incompatible with the inferred schema fail the scan.

`delimiter` is one ASCII character, default comma. Quoting, decompression and
batch construction come from open-arrow-access. [TSV](tsv.md) uses the same
implementation with a fixed tab delimiter.

Accepts one local/file/HTTP(S)/S3 object. Use [secrets](../secrets.md) for remote
credentials and [storage settings](../storage.md) for row targets and retries.
