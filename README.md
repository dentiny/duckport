# DuckPort

Read source data into DuckDB through [open-arrow-access](https://github.com/dentiny/open-arrow-access), then query it with SQL or write it using DuckDB's available sinks.

DuckPort pins the latest released `open-arrow-access` version, **0.1.0**, from crates.io. Rust readers export owned Arrow C streams to DuckDB; data is consumed in batches rather than collected into a whole table.

## Build

You need CMake 3.20+, a C++17 toolchain, Cargo/Rust 1.97+, and the submodules:

```sh
git clone --recurse-submodules https://github.com/dentiny/duckport.git
cd duckport
make
make test
```

The template's OpenSSL/vcpkg dependency has been removed. Cargo builds the source bridge with its checked-in lockfile. The default Cargo profile is `release`; for faster development builds, use `make EXT_FLAGS='-DDUCKPORT_CARGO_PROFILE=dev'`. Cross builds must set `DUCKPORT_RUST_TARGET` to the matching Rust triple (or use the build system's `Rust_CARGO_TARGET`).

Build outputs include `build/release/duckdb` and `build/release/extension/duckport/duckport.duckdb_extension`. The built shell includes DuckPort. To load the extension into a separately installed **matching DuckDB version**, start that shell with `-unsigned` and run:

```sql
LOAD 'build/release/extension/duckport/duckport.duckdb_extension';
```

## One scan function

```sql
SELECT * FROM duckport_scan('data/people.parquet');
SELECT * FROM duckport_scan('https://example.com/people.csv');
SELECT * FROM duckport_scan('data/events.jsonl.gz', batch_size := 4096);
SELECT * FROM duckport_scan('data/no-extension', format := 'json');
```

`duckport_scan(source, format := 'auto', options := '{}', batch_size := 8192)` returns the source's Arrow schema as DuckDB columns. `batch_size` must be between 1 and 1,048,576. Filters and projections are currently evaluated by DuckDB; they are not pushed down into source readers.

| Source | Selection |
| --- | --- |
| Local file | Relative/absolute path or `file://` URI |
| HTTP / HTTPS | A file URL; ranged reads are used for Parquet |
| S3 | `s3://bucket/key`, with OpenDAL options in `options.storage` |
| Hugging Face dataset export | `hf://datasets/owner/repository`, with config/split in `options.service` |
| Hugging Face repository file | Same URI, with a repository-relative `options.service.path` |

Supported formats: Parquet (Snappy, Zstd, LZ4, gzip, Brotli), CSV, TSV, JSON/JSONL/NDJSON, XML, ATIF, and WARC. File suffixes select the format; use `format` when the suffix is absent or ambiguous. ATIF uses explicit `format := 'atif'` or `format := 'atif-jsonl'` for `.json` / `.jsonl` inputs. WARC returns physical records and binary bodies. JSON expects objects, not a top-level array.

Final `.gz`, `.bz2`, `.lz4`, `.zst` / `.zstd` suffixes enable streaming decompression. These compression suffixes are case-sensitive. Externally compressed Parquet is unsupported because it loses range access. Archive-member selection, directory/glob discovery, HTTP query/signed URLs, and other provider-specific services are not yet exposed by this extension.

### Format options

Options are a JSON object with optional `storage`, `service`, and `format` objects. Values inside these objects are strings. Unknown top-level, format, and service options are rejected.

```sql
SELECT * FROM duckport_scan('people.csv',
    options := '{"format":{"header":"true"}}');

SELECT * FROM duckport_scan('records.xml',
    options := '{"format":{"record_path":"/records/record"}}');
```

CSV/TSV headers and column types are inferred by default. Set `format.header` to `"true"` or `"false"` for ambiguous input. XML requires an explicit `record_path`. CSV, JSON, and XML schema inference is bounded by the library defaults; later incompatible records produce a query error.

### S3

```sql
SELECT * FROM duckport_scan('s3://my-bucket/data/people.parquet',
    options := '{"storage":{"region":"us-east-1"}}');
```

`storage` passes configuration to the OpenDAL S3 backend, including custom `endpoint` and supported credential settings. This bridge does not use DuckDB's secret manager. Prefer the backend's environment/config credential chain over embedding credentials in SQL.

### Hugging Face

```sql
SELECT * FROM duckport_scan('hf://datasets/lhoestq/demo1',
    options := '{"service":{"config":"default","split":"train"}}');

SELECT * FROM duckport_scan('hf://datasets/owner/repository', format := 'json',
    options := '{"service":{"path":"data/train.jsonl","revision":"main"}}');
```

Dataset exports use HF's published Parquet representation. The service pins the export revision to a commit, discovers the selected config/split's shards, and reads them in path order. `service.revision` selects the export revision (default `refs/convert/parquet`); set it to a commit for repeatability. Set `service.allow_partial` to `"true"` to permit partial exports. Config and split must be explicit; dataset mode already returns Arrow records and does not accept a file `format` override.

Repository-file mode uses `service.path` and defaults to revision `main`; it does not perform dataset discovery. Supported shared settings include `service.endpoint` and `service.token`. HF environment/cached-token resolution follows open-arrow-access; `HF_HUB_DISABLE_IMPLICIT_TOKEN=1` disables implicit token loading.

### Write query results

```sql
COPY (
    SELECT * FROM duckport_scan('hf://datasets/lhoestq/demo1',
        options := '{"service":{"config":"default","split":"train"}}')
) TO 'demo.parquet' (FORMAT PARQUET);
```

Parquet output uses DuckDB's existing writer. Dedicated Iceberg/Lance sink integration remains future work.

## Validation

```sh
cargo test --manifest-path rust/Cargo.toml --locked
make test
python3 test/source_integration.py build/release/extension/duckport/duckport.duckdb_extension
```

The integration test needs the matching DuckDB CLI on `PATH` (or set `DUCKDB_CLI`). It creates local HTTP, S3-compatible, and Hugging Face fixture servers; no account or external service is required. It covers nested Arrow values, nulls, batch boundaries, repeatable scans, early LIMIT, stream failures, external-access restrictions, and `COPY` to Parquet.

The source registry is a compatibility adapter over release 0.1.0 while the [upstream unified resolver API](https://github.com/dentiny/open-arrow-access/pull/364) is under review. Registered handlers can return original bytes or native Arrow records, letting formats and source services evolve independently. It does not automatically turn arbitrary metadata, PDFs, or native provider records into tables.

Based on [duckdb/extension-template](https://github.com/duckdb/extension-template). See [docs/UPDATING.md](docs/UPDATING.md) for DuckDB update instructions.
