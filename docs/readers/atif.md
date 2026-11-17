# duckport_scan_atif

```sql
SELECT * FROM duckport_scan_atif('testdata/atif/trajectory.json');
SELECT * FROM duckport_scan_atif('testdata/atif/trajectories.jsonl', batch_size := 2);
```

Reads ATIF trajectories with the library's fixed schema. JSON objects are framed automatically from their contents, accepting pretty-printed JSON and JSONL without inspecting filenames or selecting a layout.

See [shared settings](../../README.md#read) and [test instructions](../../test/README.md).

Image and audio references in `steps.message` and `steps.observation.results.content`
are embedded automatically: `source.path` contains the media bytes as `BLOB`, while
`type`, `media_type`, duration, text bytes and `raw_json` retain their original meaning.
Subagent trajectories remain raw JSON and are not traversed for media.

Relative media paths resolve against the trajectory's directory, including JSONL
and compressed input. Local paths, `file://`, HTTP(S), S3 and base64 or
percent-encoded `data:` references are supported. Remote media must stay on the
trajectory's storage origin (same scheme, host and port / S3 bucket); the reader
uses the trajectory's storage credentials. Missing media and malformed data URIs
fail the scan with the media reference in the error.

```sql
COPY (
    SELECT * FROM duckport_scan_atif('runs/trajectory.json')
) TO 'trajectory.lance' (FORMAT duckport_lance);
```

No `BLOB_COLUMNS` is needed for embedded media. ATIF field metadata survives
DuckDB tables, projections and renaming: the Lance writer stores marked media as
managed Blob v2 fields and text as large binary with lz4 compression by default.
Explicit `COLUMN_COMPRESSION` settings override the text compression default.
These defaults apply only to tagged ATIF fields, not arbitrary binary columns.
When `steps` is read, each Arrow batch contains one trajectory to bound media
buffering; `batch_size` is a target rather than a minimum.
