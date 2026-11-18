# Lance sink

Builds require `protoc` and use open-arrow-sink 0.3.0.

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
Dotted paths traverse STRUCTs and implicitly traverse LIST/ARRAY elements;
selected text lists are adapted element by element. Unselected fields keep their
original values. All selections use `DownloadBlobLayer` and the sink's native
Blob materializer, including its size thresholds. HTTP(S), file:// and enabled
OpenDAL sources such as S3 are supported. Bytes are ingested, so the source is no longer needed after commit.
NULLs remain NULL; other strings remain strings. Invalid columns, URLs and failed
downloads fail COPY.

Destinations support local paths, file:// and s3://. Scoped DuckDB secrets configure
the destination; HTTP(S) blob URL sources use matching DuckDB HTTP secrets, independently of the destination.
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

For ATIF, select fields explicitly, for example:
`BLOB_COLUMNS ('steps.message.source.path', 'steps.observation.results.content.source.path')`.
Relative references must be qualified before COPY; no ATIF-specific path resolution
or storage policy is inferred.

## HTTP source authentication

COPY snapshots current DuckDB HTTP secrets when opening the writer. Each source
URL uses its longest matching scope; no destination credential is borrowed.
`BEARER_TOKEN` and `EXTRA_HTTP_HEADERS` support bearer, API-key and custom
Authorization schemes. The same header support applies to direct HTTP file scans.

```sql
CREATE SECRET media_token (TYPE http, BEARER_TOKEN 'access-token',
    SCOPE 'https://media.example.com/private/');
CREATE SECRET api_key (TYPE http,
    EXTRA_HTTP_HEADERS MAP {'X-API-Key': 'api-key'},
    SCOPE 'https://api.example.com/files/');
```

Same-origin redirects resolve the matching source secret again. Cross-origin
redirects omit all credentials; HTTPS downgrades are rejected. Secret snapshots
are owned by Rust through completion/cancellation, without calling DuckDB from
background tasks. Credentials are not placed in URL parameters or COPY options.

## S3 source authentication

Each `s3://` Blob source uses its own longest-scoped DuckDB S3 secret, including
`KEY_ID`, `SECRET`, `SESSION_TOKEN`, `REGION`, `ENDPOINT`, `USE_SSL` and `URL_STYLE`.
Sources may span buckets, prefixes and endpoints in the same COPY, including
when the destination is local. Explicit source credentials take precedence over
the sink's destination namespace. Without a source secret, native source
credential resolution remains independent of destination credentials.

## Hugging Face source authentication

`BLOB_COLUMNS` accepts individual dataset files as
`hf://datasets/<owner>/<repo>[@revision]/<file>`. The longest matching DuckDB
`TYPE huggingface` secret supplies `TOKEN`; revisions default to `main`, and
percent-encoded revision/file paths are decoded before repository access.
Different private repositories can use different tokens in one COPY.

```sql
CREATE SECRET private_repo (TYPE huggingface, TOKEN 'hf_access_token',
    SCOPE 'hf://datasets/owner/private-repo');
```

Explicit secrets take precedence over implicit HF credentials. Empty matched
tokens fail instead of falling back to ambient credentials. Unmatched sources
retain the upstream HF environment/cache behavior, including `HF_ENDPOINT` and
`HF_HUB_DISABLE_IMPLICIT_TOKEN`. Hub credentials are not sent to a cross-origin
CDN redirect. This uses the existing Hugging Face repository accessor, without
requiring a Parquet dataset export or sending Hub tokens as URL parameters.
