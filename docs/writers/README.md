# Sink implementation layout

Each backend owns its setup, Arrow C ABI opener and tests:

```text
rust/src/sink/
  mod.rs              shared fixed-schema writer lifecycle
  lance/
    mod.rs            Lance setup
    ffi.rs            Lance opener
    tests.rs
  iceberg/
    mod.rs            REST catalog and Iceberg setup
    ffi.rs            Iceberg opener
    tests.rs
```

C++ adapters live in `src/sink/<backend>/writer.cpp`. Shared COPY binding stays in
`src/sink/copy.cpp`; shared Arrow C ABI write, commit and destruction stay in
`rust/src/ffi/sink.rs`.

Use the same directories for additional backends, such as `sink/parquet/`.
