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

To update open-arrow-access, change the pinned version in `rust/Cargo.toml`, regenerate `rust/Cargo.lock`, and rerun the source tests.

## Reader bridge

C++ binds SQL parameters and uses DuckDB's Arrow scanner. Rust owns the schema, reader factory, and runtime. The FFI exposes `open`, `get_schema`, `get_stream`, and `destroy`; registrations and source lists are explicit.
