# DuckPort

Read Parquet into DuckDB through [open-arrow-access 0.1.0](https://github.com/dentiny/open-arrow-access/releases/tag/v0.1.0).

## Build

Requires CMake 3.20+, C++17, and Rust 1.97+.

```sh
git clone --recurse-submodules https://github.com/dentiny/duckport.git
cd duckport
make
./build/release/duckdb
```

The built shell includes DuckPort. A separate shell must match the build's DuckDB version; start it with `-unsigned` and load `build/release/extension/duckport/duckport.duckdb_extension`.

## Read

```sql
SELECT * FROM duckport_scan_parquet('people.parquet', batch_size := 4096);
SELECT * FROM duckport_scan_parquet('https://example.com/people.parquet');
SELECT * FROM duckport_scan_parquet('s3://bucket/people.parquet',
    storage_options := '{"region":"us-east-1"}');
```

Local paths, `file://`, HTTP(S), and S3 use OpenDAL. `storage_options` is a JSON map of string-valued OpenDAL settings; DuckDB secrets are not used. `batch_size` defaults to 8192 and accepts 1–1,048,576 rows.

Parquet page codecs: Snappy, Zstd, LZ4, gzip, Brotli. Archive members, directory/glob scans, HTTP query/signed URLs, and outer compression such as `.parquet.gz` are unsupported. Source projection/filter pushdown is not enabled.

## Write

```sql
COPY (SELECT * FROM duckport_scan_parquet('people.parquet'))
TO 'output.parquet' (FORMAT PARQUET);
```

[Tests](test/README.md) · [Development](docs/UPDATING.md)
