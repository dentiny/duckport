# Lance datasets

```sql
COPY events TO 'events.lance' (FORMAT duckport_lance);
COPY events TO 'events.lance' (FORMAT duckport_lance, APPEND);
COPY events TO 'events.lance' (FORMAT duckport_lance, OVERWRITE);
```

Destinations are local paths, file URIs or S3 URIs. Create fails if the dataset
exists; APPEND requires it; OVERWRITE replaces rows, creating the destination if
needed. Empty input commits the selected schema. A successful finalization
publishes the version. Failed writes/process termination can leave uncommitted
objects; a DuckDB transaction rollback does not undo an external commit.

Retry with `OVERWRITE` if a failed write left objects without a committed Lance
manifest. This creates a new dataset while retaining the old uncommitted objects.
Corrupt manifests, permission errors and other storage errors still fail COPY.

Lance is one backend of [open-arrow-sink](https://github.com/dentiny/open-arrow-sink).
Its schema/blob materializer controls native binary handling and physical Blob
layout. Ordinary text URLs remain strings unless explicitly selected or carrying
a recognized ATIF policy. A native BLOB already contains bytes and does not need
URL downloading.

## URL blob columns

```sql
COPY (SELECT id, image_url AS image FROM events)
TO 'images.lance' (FORMAT duckport_lance, BLOB_COLUMNS ('image'));
COPY events TO 'nested.lance'
(FORMAT duckport_lance, BLOB_COLUMNS ('messages.attachments.url'));
```

`BLOB_COLUMNS` selects text URL fields. Dotted paths traverse structs and
implicitly traverse list/array elements; selected string lists are adapted
element by element. Exact top-level names take precedence over nested paths,
including names containing dots. Empty lists, duplicates and invalid selections
fail. NULLs stay NULL; failed downloads fail COPY.

The sink downloads and stores managed bytes, so successful output no longer
requires the original URL to be available. Supported sources include HTTP(S),
S3 and `file://`; use a file URI for local files rather than a bare path. Queries,
embedded credentials and other invalid URL forms are rejected by the applicable
source adapter. Tokens/keys belong in [source-scoped secrets](../secrets.md),
independent of destination credentials.

Individual Hugging Face files use
`hf://datasets/<owner>/<repo>[@revision]/<file>`. Revision defaults to `main`;
percent-encoded revision/file components are decoded. The longest matching
Hugging Face secret supplies TOKEN. Explicit empty matched tokens fail instead
of using implicit credentials; unmatched sources retain upstream environment/cache
behavior. Hub tokens do not follow cross-origin CDN redirects.

## ATIF media

[ATIF SELECT](../readers/atif.md) resolves media URIs without fetching them.
Lance COPY automatically recognizes tagged media paths and downloads them; no
manual `BLOB_COLUMNS` is needed. Tagged binary text defaults to LargeBinary/LZ4,
while explicit column compression takes precedence. Credentials are looked up
at COPY time and are not persisted in schema metadata.

## Size and format options

| COPY option | Values | Effect |
| --- | --- | --- |
| `BLOB_INLINE_SIZE_THRESHOLD` | Non-negative byte count | Maximum inline blob size; zero disables inlining of nonempty payloads |
| `BLOB_DEDICATED_SIZE_THRESHOLD` | Positive byte count | Larger blobs use dedicated objects rather than packed storage |
| `TARGET_FILE_SIZE` | Positive byte count | Soft data-file target; a batch/page/overhead can exceed it |
| `STORAGE_VERSION` | `2.2` or `2.3` | Explicit Lance data-file version; APPEND must match the existing version |

```sql
COPY events TO 'images.lance' (
    FORMAT duckport_lance, BLOB_COLUMNS ('image'),
    BLOB_INLINE_SIZE_THRESHOLD 1048576,
    BLOB_DEDICATED_SIZE_THRESHOLD 8388608,
    TARGET_FILE_SIZE 268435456, STORAGE_VERSION '2.3'
);
```

Omitting options keeps backend defaults rather than inheriting the reference
extension's defaults. Thresholds apply to logical Blob fields, not every binary
field. TARGET_FILE_SIZE is also available on the other sinks; the two blob
thresholds and storage version are Lance-specific.

[Ordering](README.md#copy-ordering), [sampling](README.md#copy-sampling),
[generated columns](README.md#generated-columns),
[column encoding](README.md#column-compression-and-encoding) and
[indexes](README.md#copy-indexes) use the shared COPY interface.

### Missing source Blobs

```sql
SET logging_storage = 'memory';
SET logging_level = 'info';
SET enable_logging = true;
COPY (SELECT id, image_url FROM images)
TO 'images.lance' (
    FORMAT duckport_lance,
    BLOB_COLUMNS ('image_url'),
    BLOB_DOWNLOAD_FAILURE_POLICY 'skip_not_found'
);
SELECT message FROM duckdb_logs()
WHERE message LIKE 'DuckPort COPY: rows_committed=%';
```

`BLOB_DOWNLOAD_FAILURE_POLICY` accepts `fail` (the default) and
`skip_not_found`. The latter drops the entire row only for a selected source
Blob's HTTP 404 or typed NotFound, including missing filesystem/S3 objects.
403, timeout, malformed URL, reporter, and destination errors still fail COPY.
The policy also applies to automatic ATIF media and nested selected Blob fields.
Null Blob values are retained. This option does not skip missing scan inputs.

Each failed source URL/error is recorded to process stderr. With DuckDB logging
enabled, a successful COPY logs session-local `rows_committed`, `rows_skipped`,
and `rows_failed_blobs`; multiple failed Blob fields in one row count once.
DuckDB's COPY `Count` still reports input rows consumed, which can exceed committed
rows. Use the logged summary or query the destination for the committed count.
The option is rejected for Parquet and Iceberg COPY.
