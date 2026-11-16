# Lance sink

Builds require `protoc`. The sink uses the released open-arrow-sink 0.1.0 crates.

```sql
COPY input_table TO 'output.lance' (FORMAT duckport_lance);
COPY input_table TO 'output.lance' (FORMAT duckport_lance, APPEND);
COPY (SELECT id, image_url AS image FROM input_table)
TO 'images.lance' (FORMAT duckport_lance, BLOB_COLUMNS ('image'), OVERWRITE);
```

Create is the default and fails if the dataset exists. APPEND requires an existing
dataset; OVERWRITE replaces rows in one new dataset version. Empty input still
commits a dataset with the selected schema. Row order is not guaranteed.

BLOB_COLUMNS names top-level text columns containing URLs. DownloadBlobLayer
selects these columns for Lance's bounded blob downloader. HTTP(S) URLs including
query parameters, file:// URLs, and enabled OpenDAL sources such as S3 are
supported. NULL values remain NULL. Invalid URLs, unknown or non-text columns, and
failed downloads fail COPY. Other strings remain strings. Multiple columns use
`BLOB_COLUMNS ('image', 'audio')`. Payloads are ingested, so the source is no longer
needed after commit.

Destinations support local paths, file:// and s3:// URIs. Destination credentials
use the COPY framework's scoped DuckDB S3 secrets. Those credentials do not configure
independent blob URL sources, which use the sink library's default resolution.
External access must be enabled at bind and execution time.

Only successful SQL finalization commits the Lance manifest. Failed or abandoned
writes may leave uncommitted files and do not delete existing versions. If commit
itself fails, inspect the destination before retrying because publication may be
uncertain. File rotation, partitioning, temporary renames and ordering options are
rejected by the shared COPY framework.
