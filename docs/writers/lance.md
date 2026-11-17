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

`BLOB_COLUMNS ('image', 'audio')` selects top-level text columns for download through
DownloadBlobLayer. HTTP(S), file:// and enabled OpenDAL sources such as S3 are
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
options. Blob column names
follow DuckDB identifier matching; empty lists, duplicates and invalid columns fail
at bind time. File URL sources require `file://`; bare local blob paths are not supported.
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

## ATIF media

COPY from `duckport_scan_atif` automatically adapts marked media paths, including
nested content, to native Blob v2 inputs before the sink downloads them. No
`BLOB_COLUMNS` list is needed for these fields. Media ingestion runs through the
same native materializer in single-writer and writer-pool modes. See
[ATIF reader and storage policy](../readers/atif.md) for path resolution,
credentials, metadata preservation and text encoding.
