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

## Cargo manifest formatting

Use [Taplo](https://taplo.tamasfe.dev/configuration/file) to format `rust/Cargo.toml` and keep dependency names and feature lists
in alphabetical order. `cargo fmt` formats Rust source files only.

```sh
cargo install taplo-cli --version 0.10.0 --locked
make toml-format       # Apply formatting and sorting.
make toml-format-check # Check without modifying files.
```

The repository's `taplo.toml` limits formatting to its own manifest, excluding
submodules. Package metadata retains its existing field order.

## Reader bridge

Openers share `locator`, `DuckportStorageOptions`, and `DuckportScanOptions`. `DuckportScanOptions` contains
only format/service-independent scan settings: currently `batch_size`. Each reader
introduces its own typed options in its PR when needed, as a separate opener argument.
Reader-specific fields do not belong in the common scan options or binder.
Required option structs are borrowed as C++ `const &` and Rust `&T` for the open call.
C++ resolves scoped DuckDB secrets and passes named storage fields to Rust. It owns
input strings for the open call; Rust copies them before asynchronous reads.

C++ binds SQL parameters and uses DuckDB's Arrow scanner. Rust owns the schema and async reader factory; its FFI layer uses a shared Tokio runtime and open-arrow-access's `BlockingRecordBatchReader` to expose synchronous Arrow streams. The FFI exposes `open`, `get_schema`, `get_stream`, and `destroy`; registrations and source lists are explicit.

`rust/src/lib.rs` declares modules only. `source.rs` wraps the library's `BoundSource` for reader binding and schema checks. File readers use `open_file_source` for storage, decompression and binding; selected provider attachments use `open_attachment_source`. `ffi/common.rs` owns the shared runtime. `ffi/source.rs` owns source handles, openers, and Arrow stream exports; `ffi/sink.rs` adapts synchronous C callers to the async writer. `error.rs` owns error/panic conversion and error freeing. `validation.rs` owns input validation, including C strings, pointers, required options, selectors, and URIs. C option structs live in `options.rs` and storage setup in `storage.rs`. C++ source adapters live under `src/source/`, with headers under `src/include/source/`.

Rust tests use `#[tokio::test]`, await async writer operations, and use `tokio::fs`
for file I/O. Synchronous C ABI calls run in `tokio::task::spawn_blocking`, keeping
the shared runtime's block_on outside the test executor.
