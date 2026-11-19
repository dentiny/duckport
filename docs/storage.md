# Storage, settings and profiling

## Locators and projection

Standalone file readers accept one local path, `file://` URI, HTTP(S) URI or
`s3://` object. URI queries, fragments and embedded credentials are rejected.
Globs, explicit file lists, archive-member paths and directory datasets are not
standalone file-reader inputs. Use native DuckDB readers for those features.
Provider functions accept their own identifiers/roots; see the [catalog](readers/README.md).

All scan functions accept `batch_size` from 1 to 1,048,576, default 8192.
This is a row target, not a total-memory bound. Some services also bound batch
payloads. Binary attachment mode buffers the selected object as one BLOB; decode
large tabular attachments with `format := 'parquet'`, `'csv'` or another record
format instead. arXiv metadata snapshots are loaded fully into an in-memory index.

Top-level projection is forwarded at execution time; COUNT(*) requests no
columns while retaining row counts. Filter columns remain requested and predicates
execute in DuckDB. Nested-field/predicate pushdown is not exposed. Parquet can
skip unselected column data; WARC still consumes record bodies. Binding can
inspect fields that a subsequent projection omits.

## Storage request size, timeouts and retries

Settings are copied from the current connection when an operation opens.

| Setting | Default | Meaning |
| --- | --- | --- |
| `conversion_storage_io_request_size` | `67108864` (64 MiB) | Target bytes per source-object and Blob read request |
| `conversion_storage_timeout_ms` | `30000` | Control-operation timeout per attempt; provider HTTP request timeout |
| `conversion_storage_io_timeout_ms` | `30000` | I/O-operation/body-method timeout |
| `conversion_storage_retry_max_times` | `5` | Retries after the initial attempt; zero disables retries |
| `conversion_storage_retry_min_delay_ms` | `500` | Initial exponential backoff |
| `conversion_storage_retry_max_delay_ms` | `10000` | Maximum exponential backoff before jitter |
| `conversion_storage_retry_factor` | `2.0` | Finite multiplier between 1 and `f32::MAX` |

```sql
SET conversion_storage_io_request_size = 64 * 1024 * 1024;
SET conversion_storage_timeout_ms = 30000;
SET conversion_storage_io_timeout_ms = 30000;
SET conversion_storage_retry_max_times = 5;
SET conversion_storage_retry_min_delay_ms = 500;
SET conversion_storage_retry_max_delay_ms = 10000;
SET conversion_storage_retry_factor = 2.0;
RESET conversion_storage_timeout_ms;
```

Request size must be positive and fit the platform byte count. The same captured
setting reaches file/service record requests, archive inputs, arXiv metadata
snapshots and COPY Lance Blob downloads, including Hugging Face media. It controls
network read splitting separately from Arrow batch rows, application output chunks,
whole-object memory and upload part sizes. Small objects and final ranges may be
smaller. Provider discovery/listing APIs and Iceberg catalog requests retain their
protocol-specific response sizes. Bounded Blob range reads may add a stat/HEAD.

Durations must be positive and maximum delay must be at least minimum delay.
Timeouts apply per attempt, not to the total query. Temporary errors use
exponential backoff and jitter. HTTP transport retry rules avoid replaying a
response body after bytes have been delivered.

Remote file operators, provider HTTP requests, auxiliary provider connections,
and supported S3 sink destinations use this policy. Local filesystem operations,
Iceberg REST catalog traffic and backend-owned blob transports retain their
applicable library behavior. Source-specific S3/Hugging Face blob adapters use
their captured storage configuration. Native DuckDB readers and other extensions
use their own settings. Consult [secrets](secrets.md) for identity selection.

## Progress and metrics

```sql
SET enable_progress_bar = true;
SET profiling_coverage = 'ALL';
SET tracked_metrics = ['operator.type', 'operator.rows_scanned',
                       'io.total_bytes_read', 'io.total_bytes_written'];
PRAGMA enable_profiling = 'json';
```

Single files and selected attachments can report encoded-byte progress when
size is known. Other scans remain indeterminate. Reader EOF completes scan
progress. These are estimates, not a download/decode/write stage breakdown.

Readers report scanned rows and observed input bytes. COPY records sink bytes
read/written after successful commit, including observed metadata/finalization
I/O. Hidden upstream transports and provider discovery are not fully represented;
profiler byte counters are not a billing or network-usage ledger.

## Failure and transaction scope

Input validation and format/backend errors fail the query. COPY publication is
backend-specific: see [Lance](writers/lance.md), [Parquet](writers/parquet.md) and
[Iceberg](writers/iceberg.md). Partial or uncommitted files may remain after a
failed operation or process termination. A DuckDB transaction rollback does not
provide a transaction spanning external storage and catalogs.
