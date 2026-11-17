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

# COPY sampling

`SAMPLE_PERCENT` and `SAMPLE_ROWS` apply to all DuckPort sinks. Sampling happens
after the COPY source query and before writing:

```sql
COPY input_table TO 'sample_dataset' (FORMAT duckport_parquet, SAMPLE_PERCENT 10);
COPY input_table TO 'sample.lance' (FORMAT duckport_lance, SAMPLE_ROWS 1000);
```

`SAMPLE_PERCENT` accepts a finite number from 0 to 100 and independently samples
each source row; the output count is approximate. `SAMPLE_ROWS` accepts an integer
from 0 to DuckDB's maximum sample size (currently 1,000,000,000) and selects that
many rows uniformly, or all rows when the source contains fewer. Both methods
read the source; fixed-row sampling buffers the sample. The options are mutually
exclusive and do not promise output order.

# Column compression and encoding

All DuckPort COPY formats accept the same STRUCT option syntax as
`duckdb_lance_conversion`. Support is checked against the native sink's
compression capability.

```sql
COPY input_table TO 'output.lance' (
    FORMAT duckport_lance,
    COLUMN_COMPRESSION {'body': 'zstd', 'nested.messages.text': 'lz4'},
    COLUMN_COMPRESSION_LEVEL {'body': 3},
    COLUMN_STRUCTURAL_ENCODING {'body': 'fullzip', 'nested': 'miniblock'}
);
```

Column names match DuckDB identifiers without ASCII case sensitivity. Compression
and level options support dotted nested paths through STRUCTs and LIST/ARRAY
containers, omitting list item names. Exact top-level column names take precedence,
including names containing dots. Structural encoding accepts `miniblock` and
`fullzip` for top-level columns and applies recursively to their children.

`COLUMN_COMPRESSION_LEVEL` requires an explicit `zstd` codec for the same column;
other codecs and level-only settings fail instead of silently ignoring the level.
Native backends validate supported levels. Omitted settings preserve backend defaults.

Lance supports `none`/`uncompressed`, `zstd`, `lz4`/`lz4_raw`, nested compression
and both structural encodings. Parquet supports top-level codec overrides, including
`snappy` and `gzip` when enabled, but rejects nested paths and structural encodings.
The current Iceberg sink rejects per-column compression and structural encoding.
These restrictions come from sink capabilities, rather than format-name checks.
