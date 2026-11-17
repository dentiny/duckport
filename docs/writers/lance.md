# Lance sink

Builds require `protoc` and use open-arrow-sink 0.1.1.

```sql
COPY input_table TO 'output.lance' (FORMAT duckport_lance);
COPY input_table TO 'output.lance' (FORMAT duckport_lance, APPEND);
COPY (SELECT id, image_url AS image FROM input_table)
TO 'images.lance' (FORMAT duckport_lance, BLOB_COLUMNS ('image'), OVERWRITE);
```

Create fails if the dataset exists; APPEND requires an existing dataset; OVERWRITE
replaces its rows. Empty input commits the selected schema. Row order is unspecified.

`BLOB_COLUMNS ('image', 'audio')` selects top-level text columns for download through
DownloadBlobLayer. HTTP(S), file:// and enabled OpenDAL sources such as S3 are
supported. Bytes are ingested, so the source is no longer needed after commit.
NULLs remain NULL; other strings remain strings. Invalid columns, URLs and failed
downloads fail COPY.

Destinations support local paths, file:// and s3://. Scoped DuckDB secrets configure
the destination; blob URL sources use the library's default credential resolution.
External access must be enabled. COPY publishes on successful finalization;
failed writes can leave uncommitted files. Partitioning, rotation, temporary-file
publication and ordering options are unsupported.

Compared with [duckdb_lance_conversion](https://github.com/dentiny/duckdb_lance_conversion),
this sink exposes APPEND, OVERWRITE, BLOB_COLUMNS and the byte-size options below. Blob column names
follow DuckDB identifier matching; empty lists, duplicates and invalid columns fail
at bind time. File URL sources require `file://`; bare local blob paths are not supported.
Storage version, column compression/encoding, indexes, sampling and generated-column
options are not exposed yet.

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

These options are specific to `duckport_lance`. Blob thresholds apply to logical
Blob fields, including columns selected by `BLOB_COLUMNS`; ordinary binary columns
are unaffected. They configure writes in create, append and overwrite modes.
