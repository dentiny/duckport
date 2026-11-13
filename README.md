# DuckPort

Read files and datasets into DuckDB through [open-arrow-access 0.1.0](https://github.com/dentiny/open-arrow-access/releases/tag/v0.1.0). All DuckPort readers use that library, including formats DuckDB already supports.

## Build

Requires CMake 3.20+, C++17, and Rust 1.97+.

```sh
git clone --recurse-submodules https://github.com/dentiny/duckport.git
cd duckport
make
./build/release/duckdb
```

The built shell includes DuckPort. A separately installed shell must match the build's DuckDB version; start it with `-unsigned` and load `build/release/extension/duckport/duckport.duckdb_extension`.

## Read and write

```sql
SELECT * FROM duckport_read_parquet('people.parquet', batch_size := 4096);

COPY (SELECT * FROM duckport_read_parquet('people.parquet'))
TO 'output.parquet' (FORMAT PARQUET);
```

See [reader documentation](docs/readers) for the readers included in this checkout. To list their SQL names:

```sql
SELECT DISTINCT function_name FROM duckdb_functions()
WHERE function_name LIKE 'duckport_read_%' ORDER BY function_name;
```

## Shared settings

- `batch_size`: 1–1,048,576 rows; defaults to 8192. Data streams in batches.
- File readers accept local paths, `file://`, HTTP(S), and S3 URIs. `storage_options` is a JSON map of string-valued OpenDAL backend settings. Credentials use OpenDAL configuration, not DuckDB secrets.
- Final `.gz`, `.bz2`, `.lz4`, `.zst`, and `.zstd` suffixes enable streaming decompression (case-sensitive).
- Archive-member selection, directory/glob scans, HTTP query/signed URLs, and external compression around Parquet are unsupported. Filters and projections run in DuckDB without source pushdown.

[Tests](test/README.md) · [Development](docs/UPDATING.md)
