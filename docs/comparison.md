# Comparison with duckdb_lance_conversion

[duckdb_lance_conversion](https://github.com/dentiny/duckdb_lance_conversion)
and DuckPort both use native DuckDB COPY: any supported query result can become
a Lance dataset. Neither requires an intermediate Parquet conversion.

This comparison was checked against reference commit
[`958b4f0`](https://github.com/dentiny/duckdb_lance_conversion/tree/958b4f0cc2c291e1da5a59a10393da66a0aec84d).
Consult that revision when interpreting differences; both projects evolve.

## Shared features

Both support local/S3 Lance destinations, create/append/overwrite, target file
sizing, Blob v2 storage thresholds, generated scalar columns, Bernoulli/reservoir
sampling, column compression/layout, and scalar/vector/text/Bloom filter indexes.
Both can consume DuckDB tables, joins, native file readers and extension readers.
These are shared capabilities, not additions unique to DuckPort.

## Differences

| Area | duckdb_lance_conversion at the linked revision | DuckPort |
| --- | --- | --- |
| COPY destinations | Lance | Lance, Parquet dataset directories, Iceberg REST tables |
| Extension readers | Hugging Face, WARC/Common Crawl, ATIF, XML | Those families plus CSV/TSV, JSON, Parquet, arXiv archives/metadata, OAI-PMH, PMC, Gutenberg, MECA, CourtListener and selected provider attachments |
| Explicit URL blob selection | Top-level VARCHAR columns | Dotted nested paths through structs/lists/arrays; string lists are adapted element by element |
| Blob source credentials | S3 URLs use destination S3 configuration or an unscoped fallback | Each HTTP/S3/Hugging Face source selects its own matching secret, independently of the destination |
| arXiv PDF metadata | No arXiv-specific joined reader | One cached Parquet snapshot joins by article ID; every snapshot field is retained in a `metadata` struct |
| ATIF | Reader embeds referenced media | SELECT resolves media URIs without fetching; Lance COPY downloads tagged fields automatically |
| Index publication | Index creation follows data publication and may leave a committed data-only version | The sink backend's combined data/index publication is used |
| Write concurrency | DuckDB thread-local Lance writers when order permits | Serial DuckDB COPY delivery with a Rust writer pool; `FILE_COUNT` configures writer streams |
| Reader tuning | Explicit Hugging Face/WARC parallelism/read-ahead controls | These SQL controls are not exposed; upstream reader behavior applies |
| Memory accounting | Explicit reader/writer reservations against DuckDB's memory limit | No equivalent DuckDB-integrated Rust memory-budget controls are exposed |

DuckPort's additional sinks have narrower capabilities than Lance. Iceberg does
not accept per-column compression or indexes; Parquet does not materialize URL
blobs. See the [COPY capability table](writers/README.md#backend-capabilities).
The broader catalog does not imply identical behavior or better throughput for
every source.

## SQL migration

| Reference interface | DuckPort interface |
| --- | --- |
| `LOAD lance_conversion` | `FORCE INSTALL duckport FROM community; LOAD duckport;` |
| `FORMAT LANCE` | `FORMAT duckport_lance` |
| `read_huggingface` | `duckport_scan_huggingface` |
| `read_warc` with a file | `duckport_scan_warc` |
| `read_warc` with a crawl/segment directory | `duckport_scan_commoncrawl` / `duckport_scan_commoncrawl_segment` |
| `read_atif` | `duckport_scan_atif` |
| `read_xml_records` | `duckport_scan_xml` with `record_path` |
| `lance_conversion_storage_*` | `conversion_storage_*`; defaults differ |

Most COPY option names are familiar, but do not copy assumptions about defaults
or accepted values. DuckPort's explicit Lance `STORAGE_VERSION` accepts `2.2` or
`2.3`; its generic blob URL selections require `file://` for local files. DuckPort
standard file readers support same-schema [lists, directories and globs](readers/files.md);
native `read_parquet` remains available for its additional dataset options.

Configure [scoped secrets](secrets.md) for every source namespace. A destination
secret is not an authentication source for unrelated media or metadata URLs.
