# duckport_scan_atif

```sql
SELECT * FROM duckport_scan_atif('testdata/atif/trajectory.json');
SELECT * FROM duckport_scan_atif('testdata/atif/trajectories.jsonl', batch_size := 2);
```

Reads ATIF trajectories with the library's fixed schema. JSON objects are framed automatically from their contents, accepting pretty-printed JSON and JSONL without inspecting filenames or selecting a layout.

Image/audio `source.path` values in messages and observation results are normalized
to absolute URI strings. Local references become percent-encoded `file://` URIs;
Hierarchical remote references resolve against the trajectory's directory. `data:` URIs
are retained. Remote media references must stay on the document's storage origin
(canonical scheme and authority). URI normalization has no transport allowlist;
the configured storage/backend determines which protocols COPY can read. Opaque
non-data URIs and embedded credentials are rejected. Subagent trajectories remain raw JSON.

Scanning does not download, open or decode media. Missing files and malformed data
URI payloads therefore do not fail a scan. Content types, media attributes, text
bytes and `raw_json` retain their original meaning. Projection pushdown, empty
projections (`COUNT(*)`) and the normal `batch_size` target are preserved.

```sql
COPY (
    SELECT * FROM duckport_scan_atif('runs/trajectory.json')
) TO 'trajectory.lance' (FORMAT duckport_lance);
```

COPY automatically selects tagged media URI fields for the sink's
`DownloadBlobLayer`; no `BLOB_COLUMNS` is required for ATIF media. The Lance backend
owns downloading, bounded payload buffers, concurrency and managed Blob v2
storage. A small downloader adapter handles `data:` URIs and explicitly configured
source stores; other downloads use the sink library's default OpenDAL downloader.
Missing media and invalid data URI payloads fail COPY rather than SELECT.

ATIF storage metadata survives DuckDB tables, projection and renaming. Tagged text
uses large binary/lz4 by default; explicit `COLUMN_COMPRESSION` settings override
that default. These policies apply only to tagged fields. COPY resolves the
trajectory's HTTP/S3 source secrets independently of destination credentials;
credential values are never stored in field metadata. Secrets are looked up again
at COPY time, including after storing URI rows in a DuckDB table. Automatic secret registration follows the remote trajectory's document scope.
Absolute remote references from local documents use the sink library's default
source resolution.

See [shared settings](../../README.md#read) and [test instructions](../../test/README.md).
