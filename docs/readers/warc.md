# WARC and Common Crawl

```sql
-- One WARC file (local, HTTP, or S3).
SELECT * FROM duckport_scan_warc('testdata/warc/good_resources.warc');

-- All WARC files in one crawl release.
SELECT * FROM duckport_scan_commoncrawl(
    'testdata/commoncrawl/crawl-data/CC-MAIN-2022-40/');

-- All WARC files in one segment of that release.
SELECT * FROM duckport_scan_commoncrawl_segment(
    'testdata/commoncrawl/crawl-data/CC-MAIN-2022-40/segments/1664030338280.51/');
```

All three return physical WARC records and binary bodies; they do not extract HTML text. `batch_size` controls Arrow batch size.

The Common Crawl functions accept local paths, file URIs, HTTP(S) URLs, and S3 URLs with the standard `crawl-data/<crawl>/segments/<segment>/` layout. The upstream accessor reads `warc.paths.gz` and selects files in manifest order. A segment can contain multiple WARC files. HTTP/S3 credentials come from DuckDB secrets scoped to the input URL.

See [shared settings](../../README.md#read) and [test instructions](../../test/README.md).
