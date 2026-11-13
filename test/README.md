# Tests

Run from the repository root after building:

```sh
cargo test --manifest-path rust/Cargo.toml --locked
make test
python3 test/source_integration.py build/release/extension/duckport/duckport.duckdb_extension
```

- Rust tests cover the source/Arrow bridge and reader cleanup.
- `make test` runs the SQLLogicTests in `test/sql`. For a debug build, use `make test_debug`.
- The integration suite covers file, HTTP, S3, and HF reads, errors, rescans, and Parquet export. It uses local fixture servers; no account is needed.

The integration suite requires a matching DuckDB CLI on `PATH`. Set `DUCKDB_CLI` to use a specific binary.
