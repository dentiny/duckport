# Architecture

DuckPort is a SQL and Arrow C ABI adapter around two reusable Rust projects.

| Component | Responsibility |
| --- | --- |
| DuckDB | Query planning/execution, SQL types, native readers, COPY integration, secret storage and profiling |
| [open-arrow-access](https://github.com/dentiny/open-arrow-access) | Byte access, format/service readers, attachment decoding, prepared sources and projected Arrow streams |
| [open-arrow-sink](https://github.com/dentiny/open-arrow-sink) | Writer sessions, commits/aborts, capabilities, blob downloads, schema remapping and backend implementations |
| DuckPort C++ | Function registration, typed option binding, per-URI secret lookup, source metadata and Arrow export/import |
| DuckPort Rust | Owned async state, policy/credential adaptation and synchronous C entry points on a shared Tokio runtime |

## Reading

C++ binds `duckport_scan_*` and passes borrowed strings/options to Rust. The
shared `export_reader!` declaration macro only supplies the C signature and
`open_reader` call; provider logic stays in `rust/src/readers/<provider>.rs`.
Rust prepares an upstream `PreparedSource`, exposes its schema at bind time, and
produces projected streams through `BlockingRecordBatchReader` and Arrow C ABI.
A full projection can reuse the prepared reader. Empty projections retain row
counts for COUNT(*). SQL filters remain in DuckDB.

`SelectedAttachment` shares SQL attachment parsing across providers, while the
upstream `AttachmentAccessor` owns decoding and provider file membership.
OAI-PMH selection/pagination is in `oai.rs`; ATIF URI policy is in its reader
module. arXiv's metadata module loads one Parquet snapshot and delegates ID
normalization and PDF matching to `ArxivMetadataIndex`.

## Writing

`src/sink/copy_options.cpp` binds COPY options, `copy_query.cpp` handles sampling
and generated expressions, and `copy.cpp` owns planning/execution. Backend
openers live under `src/sink/<backend>/` and `rust/src/sink/<backend>/`.
C++ exports the query schema/batches; Rust opens the selected sink and delivers
batches to its writer. Successful finalization commits; failed writes abort where
supported. Backend publication and cleanup guarantees differ.

The sink crate family includes `open-arrow-sink-core`, `-layer`, `-storage`,
`-lance`, `-parquet` and `-iceberg`. Blob downloading, schema evolution and the
unordered writer pool use their existing layers. Shared storage/format behavior
belongs upstream rather than in a new extension-only framework.

## Credentials and lifetime

C++ resolves secrets on the query thread; Rust receives owned credential
snapshots for background I/O. Writer source credentials and catalog credentials
are independent of destination storage. An arXiv metadata snapshot similarly has
its own `StorageParameters`, resolved against `metadata_uri`.

C strings and option pointers are borrowed only during open. Returned sources,
writers, snapshots and streams own the state needed after that call. C and Rust
option layouts must change together. Arrow release callbacks transfer ownership
across the ABI. Errors are returned as owned strings and freed by the matching
FFI release function.

Native fixtures are a separate test extension. Their server/environment classes
own cleanup through destruction; servers are destroyed before request-handler
state. Shared maps and request traces use DuckDB annotated mutexes and lock
annotations. See the [test guide](../test/README.md).
