# Fixture inventory

Payloads are synthetic or adapted from MIT-licensed upstream tests. Real provider responses are reduced to the fields needed by SQL assertions; remote origins become loopback placeholders. No live account is needed for these committed HTTP fixtures.

[Test runner and route format](../test/README.md) · [Reader catalog](../docs/readers/README.md)

| Directory | Coverage |
| --- | --- |
| [arxiv](arxiv/) | Synthetic source and PDF TAR shards, bulk manifests and a cached bibliographic Parquet snapshot. Local and signed/requester-pays loopback scans share the same payloads. |
| [arxiv_metadata](arxiv_metadata/) | OAI-PMH arXivRaw pages, including empty and terminal responses, for automatic metadata harvesting. |
| [atif](atif/) | Single and multiline trajectories, JSONL framing, invalid input, Unicode/tool messages and image/audio media references. |
| [bhl](bhl/) | An item metadata response and selected text attachment, including native API-key query authentication. |
| [biorxiv](biorxiv/) | Detail-page metadata and a named JATS attachment selected by date range/server/cursor. |
| [blob_hf](blob_hf/) | Hub discovery and file responses for scoped Hugging Face blob-source authentication. |
| [blob_http](blob_http/) | HTTP media payloads and authenticated/redirected routes for independent source-secret selection. |
| [blob_s3](blob_s3/) | S3-compatible media payloads for source keys, endpoint selection and session-token behavior. |
| [commoncrawl](commoncrawl/) | A local CC-MAIN-2022-40 mirror with a compressed path manifest and three compressed WARC objects. |
| [courtlistener](courtlistener/) | A dated compressed courts CSV export under bulk-data/, retaining every field as text. |
| [csv](csv/) | Header/type inference, custom delimiters, string-only rows, malformed column counts and invalid UTF-8. |
| [dataverse](dataverse/) | Dataset/version metadata and a logically named attachment, preserving special characters in persistent IDs and file paths. |
| [dataverse_api_key](dataverse_api_key/) | Dataverse restricted-file and cross-origin cases authenticated with X-Dataverse-key. |
| [doab](doab/) | OAI-PMH DOAB pages and continuations using the shared metadata reader. |
| [federal_register](federal_register/) | Document metadata and named publication attachments, with remote origins replaced by loopback placeholders. |
| [figshare](figshare/) | Synthetic article metadata and CSV attachments; the download origin is separate to check API-token isolation. |
| [govinfo](govinfo/) | Bulk-directory metadata and an issue attachment for one selected collection/year. |
| [gutenberg](gutenberg/) | RDF book metadata, metadata-only records and malformed dates/ebook IDs. |
| [hathifiles](hathifiles/) | Catalog metadata and a selected compressed catalog file. |
| [iceberg](iceberg/) | The seed people-table metadata used by the native loopback REST catalog fixture. |
| [internet_archive](internet_archive/) | Item metadata and an attachment path containing spaces/reserved characters, with anonymous and LOW-authenticated requests. |
| [json](json/) | Object/JSONL inputs and malformed/truncated/top-level-array cases. |
| [library_of_congress](library_of_congress/) | Resource metadata and a selected file path, including an auxiliary download origin. |
| [meca](meca/) | Valid and invalid manifest-driven MECA ZIP packages, including unsafe paths and missing content. |
| [oai_pagination](oai_pagination/) | Multiple pages, empty pages with continuation, repeated tokens and provider errors for demand-driven OAI scans. |
| [oapen](oapen/) | OAI-PMH OAPEN pages and terminal responses using the shared metadata reader. |
| [openstax](openstax/) | Repository commit metadata and a selected source ZIP with a separate codeload-style origin. |
| [parquet](parquet/) | Small row data and Arrow array/field metadata fixtures for scan and COPY schema tests. |
| [pmc](pmc/) | Article-version JSON, metadata-only entries and S3 inventory manifest/gzip CSV shards. |
| [regulations](regulations/) | Record metadata and a selected attachment under the provider file namespace. |
| [semantic_scholar](semantic_scholar/) | Release discovery, dataset metadata and one selected dataset shard. |
| [uspto](uspto/) | Product metadata and a selected archive attachment with offset/page-size selection. |
| [warc](warc/) | Two resource records, a metadata record and a truncated body with an inconsistent declared length. |
| [xml](xml/) | Nested inventory/warehouse records plus unknown-entity, truncated-document and DTD inputs. |
| [zenodo](zenodo/) | Record metadata and binary/tabular/XML attachments exercising selected-file decoding. |

## Provenance and regeneration

Provider directories with existing provenance notices retain their original
attribution to open-arrow-access v0.1.0. Later tests and local edits are described
in their directory README. The common source is
[open-arrow-access service tests](https://github.com/dentiny/open-arrow-access/tree/v0.1.0/service/tests).
ATIF media also comes from MIT-licensed duckdb_lance_conversion fixtures.

Common Crawl gzip timestamps are fixed to zero. The arXiv PDF fixture generator
uses Python's standard library and an already installed DuckDB CLI to create
small TAR/Parquet payloads; it does not build DuckPort. Parquet array metadata
has its own generator under parquet/. Keep binary fixtures small and document
changes to expected record counts, payload bytes and schemas.
