# Changelog

## Unreleased

- Upgrade open-arrow-access to 0.21.7 and all open-arrow-sink crates to 0.6.5,
  including source blob annotations and Lance Binary/BinaryView input support.

- Use a Lance 14.0.3 test reader built against DuckDB v2.0.1 for managed-blob
  and nested media round trips.

- Adapt secret scopes and byte profiling to DuckDB v2.0.1's `FileOpenerInfo`
  and I/O timing APIs; use its official Lance reader revision for round-trip tests.

- Upgrade open-arrow-access to 0.20.2, including native multi-directory BHL scans
  and HTTP 406 transport retries.

- Upgrade open-arrow-access to 0.16.0; arXiv content scans now include a tenth,
  nullable `metadata` struct field. Current readers emit null metadata.

- Upgrade open-arrow-access to 0.14.1 to fix the TAR indexer’s Send future
  requirement for arXiv single-shard discovery.

- Add arXiv `format` selection for bulk PDF/LaTeX scans and explicit article PDF, HTML and source downloads through `article_id`.
- Upgrade open-arrow-access to 0.13.0 and open-arrow-sink to 0.6.3; reuse
  `CrawlAccessor` for shared planned reads and partition-local PMC shard URL selection.
- Retain native planners for Common Crawl, Hugging Face, CourtListener, PMC
  inventory and MECA so existing read-concurrency settings can use their partitions.
- Replace the shared OAI manual selection/row-limit wrapper with upstream
  `SelectionAccessor` and `LimitLayer`.

## 0.2.1 - 2026-11-23

### Changed

- Upgrade DuckDB to `v2.0.1`.

## 0.2.0 - 2026-11-19

### Added

- Add `conversion_storage_io_request_size` to control source-object and Lance
  Blob read request sizes, with a default of 2 MiB.
- Add Lance COPY `BLOB_DOWNLOAD_FAILURE_POLICY 'skip_not_found'` to skip entire
  rows only when selected source Blobs return HTTP 404 or typed NotFound.
  Report failed source URLs to stderr and committed/skipped row counts through
  DuckDB logging. Other source and destination errors still fail COPY.

### Improved

- Allow Lance `OVERWRITE` retries when failed writes leave uncommitted objects
  without a dataset manifest. Corrupt manifests and other storage errors still fail.
- Build WARC dates directly as millisecond timestamps while preserving the
  existing date and binary-body schema.
- Upgrade to open-arrow-access 0.10.3 and open-arrow-sink 0.6.2, retaining Arrow 58.

### Fixed

- Apply configured byte-read options to prepared scans.
- Make CSV dialect handling compatible with community CI.
- Declare the native dependencies required by community builds.

## 0.1.0

Initial release prepared for submission to DuckDB Community Extensions.
DuckPort reads files and dataset services into DuckDB tables and streams SQL
query results to Lance, Parquet dataset directories, or Iceberg REST tables.

### Sources

- File readers for CSV, TSV, JSON, JSONL/NDJSON, Parquet, XML and WARC.
  Standard file readers support local paths, `file://`, HTTP(S) and S3 objects,
  same-schema file lists, and local/S3 directory and glob discovery.
- Dataset and archive readers for Hugging Face Parquet exports, Common Crawl
  releases and segments, ATIF trajectories, arXiv bulk TAR shards and MECA packages.
  arXiv PDF reads can join a persisted metadata snapshot.
- Metadata readers for arXiv, DOAB and OAPEN OAI-PMH; Gutenberg RDF; PMC article
  metadata and inventory; and dated CourtListener bulk exports.
  PMC supports selecting PDF/XML/TXT content URLs and reading article metadata
  from one manifest-validated inventory shard.
- Selected attachment readers for Zenodo, Dataverse, Figshare, Internet Archive,
  Federal Register, GovInfo, HathiFiles, Library of Congress, Semantic Scholar,
  OpenStax, BHL, Regulations.gov, USPTO and bioRxiv/medRxiv. Attachments can be
  returned as binary or decoded using the provider reader's supported formats.
- Configurable row-batch size, partition-aware read concurrency and top-level
  column projection. Filters run in DuckDB. Applicable text readers support
  gzip, bzip2, LZ4 and Zstd outer compression.

See the [reader catalog](docs/readers/README.md) for all 36 scan functions,
required selectors and source-specific behavior.

### Sinks

| COPY format | Destination | Write modes |
| --- | --- | --- |
| `duckport_lance` | Local/file/S3 Lance dataset | Create, APPEND, OVERWRITE |
| `duckport_parquet` | Local/file/S3 Parquet dataset directory | Create, APPEND |
| `duckport_iceberg` | Unpartitioned table in an Iceberg REST catalog | Create, APPEND |

Any DuckDB table or query result can be a COPY source, including native DuckDB
readers, joins, filters and expressions. Iceberg creates table schemas from query
results; its catalog namespace must already exist.

### COPY options

- Shared controls: `APPEND`, `TARGET_FILE_SIZE`, `PRESERVE_ORDER`,
  `READ_CONCURRENCY` and `WRITE_CONCURRENCY`. Writes default to one writer and a
  512 MiB soft file-size target; `'auto'` explicitly enables automatic writer
  planning. Reads default to automatic concurrency where supported.
- Query transformations: `SAMPLE_PERCENT`, `SAMPLE_ROWS` and `GENERATED_COLUMNS`.
  Sampling methods are mutually exclusive; Iceberg generated columns require
  compatible catalog field metadata.
- Source arguments: `EXTRA_OPTIONS` forwards a STRUCT or MAP to explicit
  `duckport_scan_*` calls in the COPY query, with normal reader validation.
- Lance blobs and layout: `BLOB_COLUMNS`, `BLOB_INLINE_SIZE_THRESHOLD`,
  `BLOB_DEDICATED_SIZE_THRESHOLD`, `STORAGE_VERSION`, `OVERWRITE` and
  `COLUMN_STRUCTURAL_ENCODING`. URL blob selection supports nested paths;
  recognized ATIF media fields are materialized automatically.
- Column codecs: `COLUMN_COMPRESSION` and `COLUMN_COMPRESSION_LEVEL` for Lance
  and Parquet. Lance supports nested compression paths; Parquet supports
  top-level columns. Iceberg rejects these per-column options.
- Lance indexes: `SCALAR_INDEX_COLUMNS` (BTree), `VECTOR_INDEX_COLUMNS`
  (IVF_FLAT/L2), `TEXT_INDEX_COLUMNS` (Inverted) and
  `BLOOM_FILTER_INDEX_COLUMNS` (Bloom filter).
- Iceberg catalog configuration: required `CATALOG_URI` and `STORAGE_ROOT`,
  plus optional `WAREHOUSE`.

See the [complete COPY reference](docs/writers/README.md#all-copy-options) for
all 24 options, accepted values, defaults, constraints and examples.

### Authentication and operation

- DuckDB URI-scoped secrets for source storage, provider APIs, blob URLs,
  destination storage and Iceberg catalogs, with independent credential lookup.
- Redirect credential isolation for applicable HTTP readers and blob downloads.
- Session settings for storage timeouts and retries, plus progress reporting
  and DuckDB profiler row/byte metrics where available.
- Built on open-arrow-access 0.9.0, open-arrow-sink 0.5.1 and Arrow 58.

### Release scope and limitations

- Provider readers use explicit selectors; they do not imply unrestricted
  discovery or whole-catalog downloads. Metadata scans do not automatically
  download linked article/book content.
- Parquet output is a dataset directory. Use native DuckDB `FORMAT PARQUET`
  for single-file output. Parquet and Iceberg do not support OVERWRITE, Lance
  blob materialization or Lance indexes; partitioned Iceberg tables are unsupported.
- Write concurrency controls writer streams, not an exact file count, and is
  not a memory limit. `PRESERVE_ORDER` preserves incoming order without sorting.
- External writes are not rolled back by a DuckDB transaction. Visibility,
  commit behavior and cleanup depend on the sink; failed writes can leave objects.
- Community installation requires published binaries matching the user's
  DuckDB version and platform. This entry does not indicate publication is complete.

The [README](README.md) includes installation instructions and a complete local
read/query/write example. The [documentation index](docs/README.md) links to
source selection, sink guides, secrets, operational settings and development setup.
