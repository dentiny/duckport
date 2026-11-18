# COPY reference

DuckPort accepts ordinary DuckDB tables or query results:

```sql
COPY events TO 'events.lance' (FORMAT duckport_lance);
COPY (SELECT id, body FROM events WHERE id > 10)
TO 'events_dataset' (FORMAT duckport_parquet, APPEND);
```

DuckDB evaluates the query and exports Arrow batches. The selected upstream sink
owns data files and publication. COPY returns the number of output rows. External
access must be enabled. Cast unsupported query types explicitly before writing;
backend schema/type checks are authoritative.

## Backend capabilities

| Option/behavior | Lance | Parquet directory | Iceberg REST |
| --- | --- | --- | --- |
| Format | `duckport_lance` | `duckport_parquet` | `duckport_iceberg` |
| Create / APPEND | Yes / existing dataset | Yes / existing directory | Yes / existing table |
| OVERWRITE | Yes | No | No |
| Local/file/S3 storage | Yes | Yes | Through `STORAGE_ROOT` |
| `TARGET_FILE_SIZE` | Yes | Yes | Yes |
| `PRESERVE_ORDER`, `WRITE_CONCURRENCY`, `READ_CONCURRENCY` | Yes | Yes | Yes |
| `SAMPLE_PERCENT`, `SAMPLE_ROWS` | Yes | Yes | Yes |
| `GENERATED_COLUMNS` | Yes | Yes | Requires compatible catalog field metadata |
| `BLOB_COLUMNS`, automatic ATIF media | Download/materialize | No | No |
| Blob thresholds / `STORAGE_VERSION` | Yes | No | No |
| Column compression | Top-level and nested | Top-level only | No |
| Structural encoding | Top-level roots | No | No |
| Index options | Yes | No | No |

Without a mode flag, COPY selects create. `APPEND` and `OVERWRITE` are mutually
exclusive even if one is false; false selects create. APPEND does not create
missing datasets/tables. Schemas must match backend requirements.

DuckDB partitioning, temporary-file publication, per-thread output and file
rotation options are not supported. `WRITE_CONCURRENCY` is the supported writer-stream
control. See the [backend guides](#backend-guides) for failure/commit behavior.

## COPY ordering

```sql
COPY (SELECT * FROM events ORDER BY id)
TO 'ordered.lance' (FORMAT duckport_lance, PRESERVE_ORDER true);
COPY events TO 'parallel_dataset'
(FORMAT duckport_parquet, WRITE_CONCURRENCY 4);
```

`READ_CONCURRENCY` and `WRITE_CONCURRENCY` each accept `'auto'` (the default)
or a positive integer. Omitting an option is equivalent to `'auto'`; the one
exception is that an omitted COPY `READ_CONCURRENCY` inherits each scan's setting.
The old `FILE_COUNT` option is no longer accepted: concurrency controls writer
streams, not an exact final file count. A writer can roll multiple files or
receive no rows.

`PRESERVE_ORDER` defaults to false. With `PRESERVE_ORDER true`, automatic write
concurrency selects one writer; an explicit `WRITE_CONCURRENCY` must be 1.
This preserves incoming batch/row order, without sorting the input. Use ORDER BY
for a desired input sequence. Reader planning/directory listing can still reorder
later scans. Always use ORDER BY when a query result must be sorted.

## Parallel reads and automatic writer planning

```sql
-- Automatic concurrency is enabled by default, with or without TARGET_FILE_SIZE.
COPY (SELECT * FROM duckport_scan_parquet(['a.parquet', 'b.parquet']))
TO 'parallel.lance' (FORMAT duckport_lance, READ_CONCURRENCY 'auto',
    WRITE_CONCURRENCY 'auto', TARGET_FILE_SIZE 536870912);

-- Set integers to control concurrency explicitly; 1 selects serial execution.
COPY (SELECT * FROM duckport_scan_parquet(['a.parquet', 'b.parquet']))
TO 'controlled.lance' (FORMAT duckport_lance,
    READ_CONCURRENCY 8, WRITE_CONCURRENCY 4);
```

Automatic reads use at most `min(threads, 4)` concurrent partitions. An explicit
integer sets the maximum number of concurrent read partitions. Actual concurrency
also depends on available independent file/format partitions or arXiv TAR shards;
providers without a partition planner remain serial. Scan functions accept the
same values, for example `read_concurrency := 'auto'` or `read_concurrency := 1`.
An explicit COPY `READ_CONCURRENCY`, including `'auto'`, overrides all DuckPort scans
in the query. Native DuckDB readers use DuckDB's settings.
The upstream reader preserves logical partition order and limits resident
partitions, but streaming-file queues are not byte bounded. The automatic cap is
conservative, not a memory limit.

Automatic writes use DuckDB's `threads` as the writer budget, so omitting
`WRITE_CONCURRENCY` does not force serial writing. With `TARGET_FILE_SIZE` and a
direct DuckPort file scan (optionally projected), source byte statistics let the
upstream ingestion planner choose fewer writers within that budget. Without a
usable size hint or target, auto falls back to the thread budget. Ordered output
uses one writer. Blob downloads, generated columns and queries whose output size
is unknown use the thread-budget fallback. No row-counting scan is performed.
DuckDB delivers COPY batches serially; the Rust upstream writer pool handles
concurrent writes.

`TARGET_FILE_SIZE` remains a soft output-file size target in both automatic and
explicit concurrency modes. Encoded input bytes are a coarse output-size proxy,
not a promise of equal compression or exact file counts. An explicit
`WRITE_CONCURRENCY N` fixes the number of writer streams and bypasses automatic
writer selection, while file rotation still follows `TARGET_FILE_SIZE`.

To inspect the initial writer count and selection reason without writing data,
include the logical plan in EXPLAIN:

```sql
SET explain_output = 'all';
EXPLAIN COPY (SELECT * FROM duckport_scan_parquet('input.parquet'))
TO 'output.lance' (FORMAT duckport_lance, TARGET_FILE_SIZE 536870912);
```

The logical COPY node shows `Write concurrency` and `Writer budget`. Scan nodes
show their configured `Read concurrency` limit, including COPY overrides.
DuckDB's physical COPY node has no extension-specific detail callback, so writer
planning details are shown in the logical plan. Planning can read source metadata
but does not consume rows or open the destination. Concurrency is resolved when
the statement is bound; re-prepare after changing `threads`.

## COPY sampling

```sql
COPY events TO 'sample.lance' (FORMAT duckport_lance, SAMPLE_PERCENT 10);
COPY events TO 'sample_dataset' (FORMAT duckport_parquet, SAMPLE_ROWS 1000);
```

The options are mutually exclusive. `SAMPLE_PERCENT` is a finite number from 0
to 100 and applies independent Bernoulli sampling; row count is approximate.
`SAMPLE_ROWS` is an integer from 0 to DuckDB's maximum (currently 1,000,000,000)
and uses reservoir sampling, retaining all rows if the source is smaller.

Sampling occurs after the source query and before generated columns/writing.
Both methods read the source; fixed-row sampling buffers the selected sample.
Neither implies an output order or avoids source I/O.

## Generated columns

```sql
COPY (SELECT id, body FROM events) TO 'derived.lance' (
    FORMAT duckport_lance,
    GENERATED_COLUMNS {'body_hash': 'sha256(body)', 'key': 'id::VARCHAR || body_hash'}
);
```

Each STRUCT entry defines a new column with one scalar SQL expression. It may
reference source columns and earlier generated entries. Names must be unique and
cannot collide with source columns. Volatile functions, aggregates, windows,
unnest, subqueries, stars and parameters are rejected, including through macros.

Lance/Parquet store normalized expression text in field metadata under
`lance_conversion:generated_expression`. APPEND requires the same definitions;
missing, additional or different definitions fail before rows are written.
Formatting is normalized, but semantic equivalence and changed macro bodies are
not inferred. OVERWRITE replaces definitions. Iceberg must match catalog field
metadata; COPY does not silently discard generated definitions.

## Column compression and encoding

```sql
COPY events TO 'encoded.lance' (
    FORMAT duckport_lance,
    COLUMN_COMPRESSION {'body': 'zstd', 'nested.messages.text': 'lz4'},
    COLUMN_COMPRESSION_LEVEL {'body': 3},
    COLUMN_STRUCTURAL_ENCODING {'body': 'fullzip', 'nested': 'miniblock'}
);
```

Names use ASCII case-insensitive DuckDB identifier matching. Exact top-level
names take precedence over dotted traversal, including a top-level name with a
dot. Nested paths traverse STRUCT and LIST/ARRAY children without item names.

Lance accepts `none`/`uncompressed`, `zstd`, `lz4`/`lz4_raw`, nested codecs and
`miniblock`/`fullzip` layout at top-level roots. Parquet accepts supported
top-level codecs, including Snappy/gzip, but no nested codec paths or structural
encoding. Iceberg rejects these per-column options. `COLUMN_COMPRESSION_LEVEL`
requires explicit Zstd compression on the same column; native backends validate
levels. Omitted options retain backend defaults.

## COPY indexes

```sql
COPY events TO 'indexed.lance' (
    FORMAT duckport_lance,
    SCALAR_INDEX_COLUMNS (id, category),
    VECTOR_INDEX_COLUMNS (embedding),
    TEXT_INDEX_COLUMNS (description),
    BLOOM_FILTER_INDEX_COLUMNS (event_id)
);
```

Lance maps these to BTree, IVF_FLAT/L2, Inverted and Bloom filter indexes.
Columns must be top-level and can appear in only one index option per COPY.
A selected blob root cannot also be indexed. Vector fields require fixed-size
floating-point arrays, such as FLOAT[128]; backend validation checks other index
requirements. Parquet and Iceberg reject index requests.

The upstream Lance sink builds requested indexes against candidate data before
publication. An explicit APPEND request for an already existing generated index
fails under the exposed default policy; keep/rebuild controls are not SQL options.

## Backend guides

[Lance and blobs](lance.md) · [Parquet directories](parquet.md) ·
[Iceberg catalogs](iceberg.md) · [Secrets](../secrets.md)
