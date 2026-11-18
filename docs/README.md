# Documentation

Start with the [repository README](../README.md) for installation and a complete
read/query/write example.

| Guide | What it covers |
| --- | --- |
| [Comparison and migration](comparison.md) | Shared features, additional sources/sinks, renamed interfaces and limits versus duckdb_lance_conversion |
| [File selection](readers/files.md) | Same-schema lists, recursive directories, globs and scoped discovery |
| [Reader catalog](readers/README.md) | Every scan function, options and output behavior |
| [Secrets](secrets.md) | Scope selection, S3-compatible stores, HTTP/provider keys, Hugging Face and catalog authentication |
| [Storage and profiling](storage.md) | Timeouts, retries, progress, projection and operational limits |
| [COPY reference](writers/README.md) | Modes, ordering, sampling, generated columns, compression and indexes |
| [Lance](writers/lance.md) | Blob ingestion, ATIF policies and Lance write options |
| [Parquet](writers/parquet.md) | Dataset directory writes and APPEND |
| [Iceberg](writers/iceberg.md) | REST catalogs, table creation, storage roots and commits |
| [Architecture](architecture.md) | DuckDB bindings and the two upstream Rust projects |
| [Development](UPDATING.md) | Build/load, dependency updates and contributor conventions |
| [Tests](../test/README.md) | SQLLogicTest, native HTTP fixtures and round-trip readers |
| [Fixture inventory](../testdata/README.md) | Payload provenance and each committed test dataset |

Specialized reader guides cover [arXiv](readers/arxiv.md),
[OAI-PMH](readers/oai.md), [provider attachments](readers/providers.md),
[Hugging Face](readers/huggingface.md), [ATIF](readers/atif.md),
[WARC/Common Crawl](readers/warc.md), [CSV](readers/csv.md), [TSV](readers/tsv.md),
[JSON](readers/json.md), [XML](readers/xml.md) and [Parquet](readers/parquet.md).
