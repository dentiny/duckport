# DuckPort

Read file formats and dataset service providers into DuckDB through [open-arrow-access 0.2.1](https://crates.io/crates/open-arrow-access/0.2.1).

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
LOAD httpfs;
CREATE SECRET source_s3 (
    TYPE s3, KEY_ID 'key', SECRET 'secret', REGION 'us-east-1',
    SCOPE 's3://bucket/'
);
SELECT * FROM duckport_scan_parquet('s3://bucket/people.parquet');
```

Local paths, `file://`, HTTP(S), and S3 use OpenDAL. S3 connection settings and credentials come from the matching DuckDB `CREATE SECRET`. HTTP bearer tokens use a scoped HTTP secret. Scan functions accept only scan options. `batch_size` defaults to 8192 and accepts 1–1,048,576 rows.

Parquet page codecs: Snappy, Zstd, LZ4, gzip, Brotli. For standalone Parquet scans, archive members, directory/glob scans, HTTP query/signed URLs, and outer compression such as `.parquet.gz` are unsupported. Source projection/filter pushdown is not enabled.

## Write

```sql
COPY (SELECT * FROM duckport_scan_parquet('people.parquet'))
TO 'output.parquet' (FORMAT PARQUET);
```

[Tests](test/README.md) · [Development](docs/UPDATING.md)

### Gutenberg

```sql
SELECT ebook_id, title FROM duckport_scan_gutenberg('testdata/gutenberg/book.rdf', batch_size := 1);
```

Reads one RDF book metadata row; linked book files remain metadata.
Local, HTTP(S), and S3 sources use the existing scoped DuckDB secrets.

### Pmc

```sql
SELECT pmcid, version, title FROM duckport_scan_pmc('testdata/pmc/PMC123.json', batch_size := 1);
```

Reads one PMC article-version metadata row. `duckport_scan_pmc_inventory(manifest_uri)` streams the gzip CSV shards named by an S3 inventory manifest; the manifest and shard keys must share a bucket root. Linked article content is not fetched.
Local, HTTP(S), and S3 sources use the existing scoped DuckDB secrets.

### Zenodo

```sql
SELECT * FROM duckport_scan_zenodo('<identifier>', file_name := '<logical attachment name>');
```

Reads one explicitly named attachment from the provider-resolved file namespace.
`format` defaults to `binary` (one BLOB `data` column); `csv`, `tsv`, `json`,
`jsonl`, `parquet`, and `xml` decode records. XML requires `record_path`.
CSV/TSV expect headers. Text formats support outer compression. Binary mode buffers
the selected attachment into one BLOB; use a record format for large tabular files.
`endpoint` defaults to `https://zenodo.org`. No discovery pagination or whole-dataset download
is implied. Unknown file names fail through the provider accessor.
Scoped HTTP bearer tokens are used where the upstream service supports connection headers.
All reader options above are VARCHAR; `batch_size` remains BIGINT.
