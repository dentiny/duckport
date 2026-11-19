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

## All COPY options

Use these options inside `COPY ... TO ... (FORMAT duckport_lance, ...)`,
substituting `duckport_parquet` or `duckport_iceberg` for the destination.
Option names are case-insensitive. Sizes are integer bytes, not strings such as
`'512MB'`. An omitted backend-specific setting keeps the backend default.

| Option | Accepted value / default | Supported sinks |
| --- | --- | --- |
| `APPEND` | Boolean flag; default create; existing destination required | All |
| `OVERWRITE` | Boolean flag; default create | Lance |
| `TARGET_FILE_SIZE` | Positive integer bytes; default `536870912` (512 MiB), soft target | All |
| `PRESERVE_ORDER` | Boolean; default `false` | All |
| `READ_CONCURRENCY` | `'auto'` or positive integer; inherits reader setting when omitted | All; affects DuckPort sources |
| `WRITE_CONCURRENCY` | `'auto'` or positive integer; default `1` | All |
| `SAMPLE_PERCENT` | Finite number `0`–`100`; unset by default | All |
| `SAMPLE_ROWS` | Integer `0`–`1000000000`; unset by default | All |
| `GENERATED_COLUMNS` | STRUCT of new column names to SQL expression strings; unset | Lance, Parquet; Iceberg requires compatible catalog metadata |
| `EXTRA_OPTIONS` | STRUCT or MAP of reader argument names to values; unset | All; requires explicit DuckPort source calls for nonempty options |
| `BLOB_COLUMNS` | Nonempty list of URL column paths; unset | Lance |
| `BLOB_INLINE_SIZE_THRESHOLD` | Non-negative integer bytes; backend default | Lance |
| `BLOB_DEDICATED_SIZE_THRESHOLD` | Positive integer bytes; backend default | Lance |
| `STORAGE_VERSION` | `'2.2'` or `'2.3'`; backend default | Lance |
| `COLUMN_COMPRESSION` | STRUCT of column paths to codec strings; backend defaults | Lance (nested paths too), Parquet (top-level only) |
| `COLUMN_COMPRESSION_LEVEL` | STRUCT of column paths to integer Zstd levels; requires explicit Zstd for each column | Lance, Parquet |
| `COLUMN_STRUCTURAL_ENCODING` | STRUCT of top-level roots to `'miniblock'` or `'fullzip'`; backend default | Lance |
| `SCALAR_INDEX_COLUMNS` | Nonempty list of top-level columns; no indexes requested by default | Lance; BTree |
| `VECTOR_INDEX_COLUMNS` | Nonempty list of top-level fixed-size floating-point arrays; unset | Lance; IVF_FLAT/L2 |
| `TEXT_INDEX_COLUMNS` | Nonempty list of top-level columns; unset | Lance; Inverted |
| `BLOOM_FILTER_INDEX_COLUMNS` | Nonempty list of top-level columns; unset | Lance; Bloom filter |
| `CATALOG_URI` | Nonempty HTTP(S) URI; required | Iceberg |
| `STORAGE_ROOT` | Nonempty local/file/S3 root covering table storage; required | Iceberg |
| `WAREHOUSE` | Nonempty catalog warehouse string; optional | Iceberg |

`FORMAT` selects the sink; it is not an input decoding option. The sections below
and backend guides explain constraints and examples. Unsupported options fail;
DuckPort does not automatically accept native DuckDB COPY options such as
`PARTITION_BY`, `PER_THREAD_OUTPUT`, `ROW_GROUP_SIZE` or `COMPRESSION`.
Use `COLUMN_COMPRESSION` for supported per-column codecs.

## Forward reader options from COPY

Prefer named arguments on the reader when configuring individual sources:

```sql
COPY (SELECT * FROM duckport_scan_csv('people.csv', header := true))
TO 'people_dataset' (FORMAT duckport_parquet);
```

Alternatively, `EXTRA_OPTIONS` adds named arguments to every explicit
`duckport_scan_*` call in the COPY query before binding:

```sql
COPY (SELECT * FROM duckport_scan_csv('people.csv'))
TO 'people.lance' (FORMAT duckport_lance, EXTRA_OPTIONS {'header': true});
```

Both STRUCT (`{'header': true}`) and MAP (`MAP {'header': true}`) literals work.
The reader validates names and types; unknown or duplicate arguments fail,
including an argument already supplied directly on the reader. Every affected
reader must accept every forwarded argument. A nonempty option map requires an
explicit DuckPort reader call; it does not configure native readers or readers
hidden behind a table/view. Credentials belong in [secrets](../secrets.md).
The [PMC example](../readers/pmc.md#copy-source-options-and-download-content)
uses `content_format` to select article URLs independently of the output format.

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

`WRITE_CONCURRENCY` defaults to 1. Set a positive integer for a fixed writer
count, or explicitly select `'auto'` for metadata-driven writer planning.
`TARGET_FILE_SIZE` defaults to 512 MiB (536870912 bytes). Each writer rolls to a
new file when its current file reaches the soft target; one writer can therefore
produce many files without parallel writers.

`READ_CONCURRENCY` accepts `'auto'` (the scan default) or a positive integer.
An omitted COPY `READ_CONCURRENCY` inherits each scan's setting.
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
-- Explicitly opt into automatic writer planning.
COPY (SELECT * FROM duckport_scan_parquet(['a.parquet', 'b.parquet']))
TO 'parallel.lance' (FORMAT duckport_lance, READ_CONCURRENCY 'auto',
    WRITE_CONCURRENCY 'auto', TARGET_FILE_SIZE 536870912);

-- Set integers to control concurrency explicitly; 1 selects serial execution.
COPY (SELECT * FROM duckport_scan_parquet(['a.parquet', 'b.parquet']))
TO 'controlled.lance' (FORMAT duckport_lance,
    READ_CONCURRENCY 8, WRITE_CONCURRENCY 4);
```

Automatic reads use DuckDB's `threads` as the concurrent partition limit. An explicit
integer sets the maximum number of concurrent read partitions. Actual concurrency
also depends on available independent file/format partitions or arXiv TAR shards;
providers without a partition planner remain serial. Scan functions accept the
same values, for example `read_concurrency := 'auto'` or `read_concurrency := 1`.
An explicit COPY `READ_CONCURRENCY`, including `'auto'`, overrides all DuckPort scans
in the query. Native DuckDB readers use DuckDB's settings.
The upstream reader preserves logical partition order and limits resident
partitions, but streaming-file queues are not byte bounded. The concurrency limit
is not a memory limit.

Only explicit `WRITE_CONCURRENCY 'auto'` uses DuckDB's `threads` as the writer
budget. For a direct DuckPort file scan (optionally projected), source byte
statistics and the file-size target let the upstream ingestion planner choose
writers within that budget. The default 512 MiB target applies when no target
is specified. Without a usable size hint, auto falls back to one writer. Ordered
output, blob downloads, generated columns and queries whose output size is
unknown also use one writer. No row-counting scan is performed.
DuckDB delivers COPY batches serially; the Rust upstream writer pool handles
concurrent writes.

`TARGET_FILE_SIZE` remains a soft output-file size target in both automatic and
explicit concurrency modes. Encoded input bytes are a coarse output-size proxy,
not a promise of equal compression or exact file counts. An explicit
`WRITE_CONCURRENCY N` fixes the number of writer streams and bypasses automatic
writer selection, while file rotation still follows `TARGET_FILE_SIZE`.

Scan nodes in `EXPLAIN` show their configured `Read concurrency` limit, including
COPY overrides. Writer selection remains internal to the upstream Rust ingestion
layer; DuckPort uses DuckDB's standard COPY logical operator.

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
