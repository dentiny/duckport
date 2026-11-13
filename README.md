# DuckPort

Read data into DuckDB through [open-arrow-access 0.1.0](https://github.com/dentiny/open-arrow-access/releases/tag/v0.1.0). Every reader uses that library, including formats DuckDB already supports.

## Build

Requires CMake 3.20+, C++17, and Rust 1.97+.

```sh
git clone --recurse-submodules https://github.com/dentiny/duckport.git
cd duckport
make
./build/release/duckdb
```

The built shell includes DuckPort. For another shell, match the build's DuckDB version, start it with `-unsigned`, and load `build/release/extension/duckport/duckport.duckdb_extension`.

## Readers

All names have the `duckport_read_` prefix. Format readers accept local paths, `file://`, HTTP(S), and S3 URIs.

| Suffix | Input / options |
| --- | --- |
| `parquet` | Parquet |
| `csv`, `tsv` | CSV/TSV; optional `header := true` or `false` |
| `json`, `jsonl`, `ndjson` | JSON objects, including newline-delimited objects |
| `xml` | XML; requires `record_path` |
| `atif`, `atif_jsonl` | ATIF JSON document / one trajectory per line |
| `warc` | Physical WARC records with binary bodies |
| `fs`, `http`, `s3` | Files on that backend; requires `format` |
| `huggingface` | Dataset Parquet export; optional `config`, `split` (defaults: `default`, `train`) |
| `huggingface_file` | One repository file; requires `path` and `format` |

```sql
SELECT * FROM duckport_read_csv('people.csv', header := true);
SELECT * FROM duckport_read_parquet('s3://bucket/people.parquet');
SELECT * FROM duckport_read_jsonl('events.jsonl.gz', batch_size := 4096);
SELECT * FROM duckport_read_xml('records.xml', record_path := '/records/record');

SELECT * FROM duckport_read_s3('s3://bucket/people.parquet',
    format := 'parquet', storage_options := '{"region":"us-east-1"}');

SELECT * FROM duckport_read_huggingface('lhoestq/demo1',
    config := 'default', split := 'train');
SELECT * FROM duckport_read_huggingface_file('owner/repository',
    path := 'data/train.jsonl', format := 'jsonl', revision := 'main');
```

`fs`/`http`/`s3` and HF file readers accept the same format names as the format readers. They also accept CSV/TSV `header` and XML `record_path` options.

## Shared options

- `batch_size`: 1–1,048,576 rows; defaults to 8192. Readers stream batches on demand.
- `storage_options`: a JSON object of string-valued OpenDAL backend settings, for file/format readers. Credentials use OpenDAL configuration, not DuckDB secrets.
- HF readers accept `revision`, `endpoint`, and `token`. Dataset revisions default to `refs/convert/parquet`, file revisions to `main`. `allow_partial := true` permits incomplete dataset exports. Dataset discovery pins a commit and reads shards in path order.

HF also uses environment/cached tokens; `HF_HUB_DISABLE_IMPLICIT_TOKEN=1` disables implicit token loading.

## Formats and limits

Final `.gz`, `.bz2`, `.lz4`, `.zst`, and `.zstd` suffixes enable streaming decompression (case-sensitive). Parquet supports Snappy, Zstd, LZ4, gzip, and Brotli page codecs.

CSV/TSV, JSON, and XML infer schemas from a bounded sample; later incompatible records fail the query. JSON expects objects, not a top-level array.

Not yet supported: external compression around Parquet, archive-member or directory/glob scans, HTTP query/signed URLs, and providers other than HF. Filters and projections run in DuckDB without source pushdown.

## Write results

```sql
COPY (SELECT * FROM duckport_read_csv('people.csv'))
TO 'people.parquet' (FORMAT PARQUET);
```

Parquet output uses DuckDB's writer. Dedicated Iceberg/Lance sinks are planned.

[Tests](test/README.md) · [Development](docs/UPDATING.md)
