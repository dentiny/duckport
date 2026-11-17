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

# COPY ordering

`PRESERVE_ORDER` selects one of two modes. It defaults to `false`:

```sql
COPY (SELECT * FROM input_table ORDER BY id)
TO 'ordered.lance' (FORMAT duckport_lance, PRESERVE_ORDER true);
COPY input_table TO 'output_dataset' (FORMAT duckport_parquet, PRESERVE_ORDER false);
```

All three sinks (Lance, Parquet and Iceberg) support both modes.

With `true`, DuckDB flushes batches in input order to a single backend writer.
Sources that support batch indexes can prepare batches in parallel; other sources
use a serial COPY. Rows retain their input sequence as batches are written into
data files. This does not sort the source: use `ORDER BY` in the source query for
a specific sort order.

With `false`, upstream execution may run in parallel and the backend uses a
bounded pool of independent writers, sized by DuckDB's `threads` setting. Writers
share one publication session. The library overlaps I/O; Parquet and Iceberg
encoding runs in the calling task, while Lance also uses background workers.
Rows may be reordered but are neither dropped nor duplicated by this mode.

This option controls write scheduling, not a reader's dataset scan order.
Parquet directory listing, Iceberg scan planning and parallel readers can choose
a different file or row sequence. Use an explicit query `ORDER BY` when results
must be sorted. Write scheduling belongs to the DuckPort adapter; it does not
request the sink library's separate `Ordering::PreserveInput` scan-order guarantee.

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

# Generated columns

`GENERATED_COLUMNS` uses the same STRUCT syntax as `duckdb_lance_conversion`.
Expressions run after sampling and may reference source columns and earlier
entries in the STRUCT:

```sql
COPY (SELECT id, body FROM input_table) TO 'output.lance' (
    FORMAT duckport_lance,
    GENERATED_COLUMNS {'body_hash': 'sha256(body)', 'key': 'id::VARCHAR || body_hash'}
);
```

Names must be unique and must not collide with source columns. Each value is one
SQL scalar expression evaluated from the source row. Volatile functions,
aggregates, windows, unnest, subqueries, parameters and stars are rejected,
including non-scalar operations introduced by macros.

Lance and Parquet preserve the parsed expression text in each generated field's
`lance_conversion:generated_expression` metadata. APPEND requires the same
field definitions: missing, extra or different definitions fail before writing
rows, even if the resulting column names and types match. Parsing normalizes SQL
formatting; semantic equivalence is not inferred. OVERWRITE replaces definitions.

Iceberg writes use the existing catalog schema. Generated definitions must match
its field metadata; an ordinary catalog field cannot acquire a generated
definition through COPY. Mismatches fail instead of discarding the metadata.

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

# COPY indexes

Index requests use the same column-list options as `duckdb_lance_conversion`:

```sql
COPY input_table TO 'output.lance' (
    FORMAT duckport_lance,
    SCALAR_INDEX_COLUMNS (id, category),
    VECTOR_INDEX_COLUMNS (embedding),
    TEXT_INDEX_COLUMNS (description),
    BLOOM_FILTER_INDEX_COLUMNS (event_id)
);
```

The native Lance sink builds BTree, IVF_FLAT with L2 distance, Inverted full-text,
and Bloom filter indexes respectively. It publishes data and requested indexes
together when COPY succeeds. Column names use DuckDB identifier matching and must
identify top-level columns. Each column may have only one index per COPY;
`BLOB_COLUMNS` columns cannot also be indexed. Vector indexes require a fixed-size
floating-point ARRAY, such as `FLOAT[128]`; the backend validates index-specific types.

Index support is checked against the sink capability. The current Parquet and
Iceberg sinks reject these options. An APPEND that explicitly requests an already
existing generated index fails; this interface uses the backend's default policy
and does not expose index keep/rebuild controls.
