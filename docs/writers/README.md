# Sink implementation layout

Each backend owns its setup and Arrow C ABI opener. Rust backend tests live beside
the implementation:

```text
rust/src/sink/
  mod.rs              shared fixed-schema writer lifecycle
  lance/
    mod.rs            Lance setup
    ffi.rs            Lance opener
    tests.rs
  parquet/
    mod.rs            Parquet directory setup
    ffi.rs            Parquet opener
  iceberg/
    mod.rs            REST catalog and Iceberg setup
    ffi.rs            Iceberg opener
    tests.rs
    test_utils.rs     catalog fixtures and batch helpers
```

C++ adapters live in `src/sink/<backend>/writer.cpp`. Shared COPY binding stays in
`src/sink/copy.cpp`; shared Arrow C ABI write, commit and destruction stay in
`rust/src/ffi/sink.rs`.

Use the same directories for additional backends. SQL round-trip tests live in
`test/sql/writers/` and `test/integration/`.
