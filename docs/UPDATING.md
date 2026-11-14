# Development

## Build profiles

`make` uses Cargo's release profile. For faster Rust builds:

```sh
make EXT_FLAGS='-DDUCKPORT_CARGO_PROFILE=dev'
```

Cross builds must set `DUCKPORT_RUST_TARGET` to the matching Rust triple. The build also accepts `Rust_CARGO_TARGET`. Build each macOS architecture separately.

## Update dependencies

1. Check out the target DuckDB release in `duckdb`.
2. Select a compatible revision of `extension-ci-tools`, which supplies the local Makefile.
3. Rebuild and run the [tests](../test/README.md).
4. Commit the new submodule revisions.

DuckPort uses DuckDB's internal C++ API. Review [DuckDB release notes](https://github.com/duckdb/duckdb/releases) and relevant header changes when upgrading.

To update open-arrow-access, change the version requirement in `rust/Cargo.toml`, regenerate `rust/Cargo.lock`, and rerun the SQL tests.

## Reader bridge

Every opener uses `(locator, const StorageOptions *, const ScanOptions *, output)`.
Both structs use named fields; there is no JSON or key/value options payload.
C++ resolves S3, HTTP, and Hugging Face credentials through DuckDB's scoped secret
manager, then passes the resolved storage fields to Rust. Connection settings are
not scan SQL parameters. Scan fields cover batch size, header, record path, revision,
and dataset/file selection. C++ owns input strings for the call; Rust copies them
before asynchronous reads. S3 uses the resolved secret rather than an independent
Rust credential chain.

C++ binds SQL parameters and uses DuckDB's Arrow scanner. Rust owns the schema and async reader factory; its FFI layer uses a shared Tokio runtime and adapts readers to synchronous Arrow streams. The FFI exposes `open`, `get_schema`, `get_stream`, and `destroy`; registrations and source lists are explicit.

`rust/src/lib.rs` declares modules only. Async reader state lives in `source.rs`, runtime and C exports in `ffi.rs`, C option structs in `options.rs`, error conversion in `error.rs`, URI validation in `validation.rs`, and storage setup in `storage.rs`. C++ source adapters live under `src/source/`, with headers under `src/include/source/`.
