# Tests

Run from the repository root after building:

```sh
cargo test --manifest-path rust/Cargo.toml --locked
make test
```

Rust tests cover the Arrow bridge. Each reader owns a SQL smoke test in `test/sql/readers`.
