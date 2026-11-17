# duckport_scan_atif

```sql
SELECT * FROM duckport_scan_atif('testdata/atif/trajectory.json');
SELECT * FROM duckport_scan_atif('testdata/atif/trajectories.jsonl', batch_size := 2);
```

Reads ATIF trajectories with the library's fixed schema. JSON objects are framed automatically from their contents, accepting pretty-printed JSON and JSONL without inspecting filenames or selecting a layout.

Image/audio `source.path` values in messages and observation results are normalized
to absolute URI strings using the document URI as the base. All media references,
including file references, follow URI resolution rules; ATIF does not interpret
references as filesystem paths. Encode reserved filename characters in references
(for example, `image%20%231%25.png` for `image #1%.png`). Relative references resolve
against the document's directory. `data:` URIs
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
storage. The sink library's downloader handles `data:` URIs and all storage
backends, including explicitly configured source stores.
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

URL downloading, Blob v2 conversion and ATIF text compression defaults apply only
to `duckport_lance`. `duckport_parquet` preserves the Arrow media URI/string and
text/binary fields. `duckport_iceberg` also preserves their values, with only the
existing catalog schema/field-ID/nullability alignment; neither sink fetches media.
