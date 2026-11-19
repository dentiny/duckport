# WARC and Common Crawl

```sql
SELECT id, body FROM duckport_scan_warc('archive.warc.gz');
SELECT * FROM duckport_scan_commoncrawl(
    'testdata/commoncrawl/crawl-data/CC-MAIN-2022-40/');
SELECT * FROM duckport_scan_commoncrawl_segment(
    'testdata/commoncrawl/crawl-data/CC-MAIN-2022-40/segments/1664030338280.51/');
```

`duckport_scan_warc` accepts physical WARC objects and same-schema
[file lists, directories and globs](files.md). The Common Crawl readers
select one release or segment using `warc.paths.gz` under the normal
`crawl-data/<crawl>/segments/<segment>/` layout. Selection follows upstream
manifest order; a segment can contain multiple WARC files. Local mirrors must
retain the `crawl-data` path structure.

WARC and Common Crawl share the upstream schema: `id` is the WARC record ID,
`type` is the record type, `date` is a timestamp with millisecond precision,
and `body` contains the original binary payload.
No HTML text extraction is performed. Top-level projection omits unselected Arrow values, but the parser
still consumes the WARC body bytes. `batch_size` is the row target.

Local paths, file URIs, HTTP(S) and S3 work through the same configured storage.
Use [scoped secrets](../secrets.md) for remote mirrors. These functions do not
expose an `index_path` or `max_read_parallelism` SQL argument; do not copy those
options from `duckdb_lance_conversion` examples.
