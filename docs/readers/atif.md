# duckport_scan_atif

```sql
SELECT * FROM duckport_scan_atif('testdata/atif/trajectory.json');
SELECT * FROM duckport_scan_atif('testdata/atif/trajectories.jsonl', batch_size := 2);
```

Reads ATIF trajectories with the library's fixed schema. JSON objects are framed automatically from their contents, accepting pretty-printed JSON and JSONL without inspecting filenames or selecting a layout.

See [shared settings](../../README.md#read) and [test instructions](../../test/README.md).

Media `source.path` values remain references when reading; the reader does not
fetch image/audio objects. ATIF content text and media fields carry storage-policy
metadata through direct projections, aliases and DuckDB tables. Computed
expressions may lose these markers.

When copying to Lance, the sink adapts marked media fields to nested Blob v2
inputs and downloads referenced objects during the write. Relative paths resolve
against the original ATIF document, including JSONL/gzip input. Local documents
can reference local files, HTTP(S), S3 or data URIs; remote references must retain
the document's scheme, host and port. Data URIs are decoded in the sink adapter.
Missing referenced media fails COPY. `raw_json`, MIME types, durations and content
order remain intact.

Source-origin DuckDB secrets are resolved again when COPY opens, and are passed
to the sink downloader independently of destination secrets. Credentials are
never included in schema metadata. Cross-origin media referenced by a local
document uses the downloader's default credential resolution.

ATIF content text stays `LargeBinary` and requests LZ4 by default. Explicit
`COLUMN_COMPRESSION` settings override this default. Blob layout thresholds also
apply to the adapted nested media fields. Parquet and Iceberg retain the original
text references; this Lance-specific adaptation is not applied to their writes.
