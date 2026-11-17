# Lance sink

Builds require `protoc` and use open-arrow-sink 0.1.1.

```sql
COPY input_table TO 'output.lance' (FORMAT duckport_lance);
COPY input_table TO 'output.lance' (FORMAT duckport_lance, APPEND);
COPY (SELECT id, image_url AS image FROM input_table)
TO 'images.lance' (FORMAT duckport_lance, BLOB_COLUMNS ('image'), OVERWRITE);
```

Create fails if the dataset exists; APPEND requires an existing dataset; OVERWRITE
replaces its rows. Empty input commits the selected schema. `PRESERVE_ORDER true`
writes batches in input order; the default allows reordering. See
[COPY ordering](README.md#copy-ordering) for write scheduling and reader semantics.

`BLOB_COLUMNS ('image', 'audio')` explicitly selects text URL fields for download.
Dotted paths traverse STRUCTs and implicitly traverse LIST/ARRAY elements. A
selected list of text URLs is adapted element by element. Unselected fields keep
their original values. Top-level text selections retain the existing
DownloadBlobLayer path; nested selections adapt to native Blob v2 inputs before
the same sink materializer downloads them. HTTP(S), file:// and enabled OpenDAL sources such as S3 are
supported. Bytes are ingested, so the source is no longer needed after commit.
NULLs remain NULL; other strings remain strings. Invalid columns, URLs and failed
downloads fail COPY.

Destinations support local paths, file:// and s3://. Scoped DuckDB secrets configure
the destination; blob URL sources use the library's default credential resolution.
External access must be enabled. COPY publishes on successful finalization;
failed writes can leave uncommitted files. Partitioning, rotation, temporary-file
publication are unsupported.

Compared with [duckdb_lance_conversion](https://github.com/dentiny/duckdb_lance_conversion),
this sink exposes APPEND, OVERWRITE, BLOB_COLUMNS, byte-size and storage-version
options. Blob paths
follow DuckDB identifier matching; exact top-level names (including dots) take
precedence over nested traversal; empty lists, duplicates and invalid columns fail
before the destination is opened. File URL sources require `file://`; bare local
blob paths are not supported.
Column compression/encoding uses the [shared COPY options](README.md#column-compression-and-encoding).
Indexes use the [shared COPY index options](README.md#copy-indexes).
Generated columns use the [shared COPY option](README.md#generated-columns),
including expression metadata and APPEND definition checks.

## Byte-size options

All sizes are integer byte counts. Omitted options preserve the backend defaults.

| COPY option | Accepted values | Effect |
| --- | --- | --- |
| `BLOB_INLINE_SIZE_THRESHOLD` | Non-negative integer | Maximum Blob size stored inline; zero disables inlining for nonempty blobs. |
| `BLOB_DEDICATED_SIZE_THRESHOLD` | Positive integer | Blobs larger than this size use dedicated objects instead of packed storage. |
| `TARGET_FILE_SIZE` | Positive integer | Soft target for data-file size; batches, pages and file overhead can exceed it. |

```sql
COPY input_table TO 'images.lance' (
    FORMAT duckport_lance,
    BLOB_COLUMNS ('image'),
    BLOB_INLINE_SIZE_THRESHOLD 1048576,
    BLOB_DEDICATED_SIZE_THRESHOLD 8388608,
    TARGET_FILE_SIZE 268435456
);
```

`TARGET_FILE_SIZE` is supported by all DuckPort sinks. The two Blob threshold
options are specific to `duckport_lance` and apply to logical
Blob fields, including columns selected by `BLOB_COLUMNS`; ordinary binary columns
are unaffected. They configure writes in create, append and overwrite modes.

## Storage version

`STORAGE_VERSION` selects the Lance data-file format. The current backend accepts
`'2.2'` and `'2.3'`. Omit it to preserve the backend default; when appending, an
explicit version must match the existing dataset.

```sql
COPY input_table TO 'output.lance' (FORMAT duckport_lance, STORAGE_VERSION '2.3');
```

Support is determined by the sink's capability. The current Parquet and Iceberg
sinks reject `STORAGE_VERSION`.

## Nested BLOB_COLUMNS

ATIF media can be selected explicitly with the same option:

```sql
COPY (SELECT * FROM duckport_scan_atif('trajectories.jsonl'))
TO 'trajectories.lance' (
    FORMAT duckport_lance,
    BLOB_COLUMNS ('steps.message.source.path', 'steps.observation.results.content.source.path')
);
```

The selected leaves must contain absolute URLs supported by the existing Blob
reader (`file://`, HTTP(S), S3 or other enabled services). Qualify relative ATIF
references before COPY; this option does not infer an ATIF document base URI,
source credentials or text-compression policy. Null structs, lists and values
remain null, and hidden values under null parents are not downloaded. Blob size
thresholds apply to the selected nested fields as well as top-level fields.
