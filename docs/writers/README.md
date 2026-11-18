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
| `PRESERVE_ORDER`, `FILE_COUNT` | Yes | Yes | Yes |
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
rotation options are not supported. `FILE_COUNT` is the supported writer-stream
control. See the [backend guides](#backend-guides) for failure/commit behavior.

## COPY ordering

```sql
COPY (SELECT * FROM events ORDER BY id)
TO 'ordered.lance' (FORMAT duckport_lance, PRESERVE_ORDER true);
COPY events TO 'parallel_dataset'
(FORMAT duckport_parquet, FILE_COUNT 4);
```

`PRESERVE_ORDER` defaults to false. DuckDB delivers COPY batches serially; Rust
uses the upstream bounded writer pool when order need not be preserved. The
writer count defaults to DuckDB's `threads` setting. `FILE_COUNT` is a positive
integer, configuring independent writer streams rather than an exact final file
count: each stream can roll multiple files or receive no rows.

With `PRESERVE_ORDER true`, one writer preserves incoming batch/row order and
only `FILE_COUNT 1` is accepted. This does not sort the source; use ORDER BY for
a desired input sequence. Reader planning/directory listing can still reorder
later scans. Always use ORDER BY when a query result must be sorted.

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
