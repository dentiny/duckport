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
this sink currently exposes APPEND, OVERWRITE and BLOB_COLUMNS. Blob column names
follow DuckDB identifier matching; empty lists, duplicates and invalid columns fail
at bind time. File URL sources require `file://`; bare local blob paths are not supported.
Blob thresholds, target file size, storage version, column compression/encoding,
indexes, sampling and generated-column options are not exposed yet.
