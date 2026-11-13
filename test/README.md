# Tests

Run from the repository root after building:

```sh
cargo test --manifest-path rust/Cargo.toml --locked
make test
python3 test/readers.py build/release/extension/duckport/duckport.duckdb_extension
```

Select one reader with `--reader parquet`. Each reader owns a test file in `test/readers` and a SQL smoke test in `test/sql/readers`.

The Python suite requires a matching DuckDB CLI on `PATH`; set `DUCKDB_CLI` to choose a binary. HTTP, S3, and HF tests use local fixture servers and need no account.
