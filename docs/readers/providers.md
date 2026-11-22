# Provider attachments

These functions resolve a provider's file namespace, select one `file_name`, and
optionally decode it as records. They do not recursively download a provider's
whole catalog or follow arbitrary discovery pages.

```sql
SELECT * FROM duckport_scan_figshare('21422193',
    file_name := 'records.csv', format := 'csv');
SELECT * FROM duckport_scan_dataverse('doi:10.7910/DVN/example',
    version := '1.0', file_name := 'records.csv', format := 'csv');
```

Identifiers and file names above illustrate the API; replace them with names
present in the selected provider object. File names are logical provider paths,
not necessarily the URL basename. Unknown selections fail.

## Shared options

Semantic Scholar is the fixed-format exception: it accepts only `s2orc_v2`
and always decodes gzip JSONL into typed records. Its options are `file_name`,
`release`, `endpoint`, and the common scan options; `format` and `record_path`
are not exposed.

| Option | Default | Meaning |
| --- | --- | --- |
| `file_name` | Required | One logical attachment name |
| `format` | `binary` | `binary`, `csv`, `tsv`, `json`, `jsonl`, `parquet`, or `xml` |
| `record_path` | Unset | Required for XML; rejected for other formats |
| `endpoint` | Provider-specific | Primary service origin; also used for secret lookup |
| `batch_size` | `8192` | Shared row-batch target |

All provider-specific selectors below are VARCHAR; `batch_size` is BIGINT.
Binary mode returns a single BLOB column named `data`, preserving the selected
bytes without decompression. It buffers the complete attachment. CSV/TSV expect
headers, JSON/JSONL decode object records, Parquet uses its schema, and XML needs
a record path. Decoded formats use the upstream attachment decoder's applicable
outer-compression support. Prefer decoded records for large tabular attachments.

In open-arrow-access 0.20.1, provider `Connection` downloads are sequential.
Parquet attachment decoding requires a ranged byte source and is unavailable
through those connections. Use an OpenDAL-backed direct reader such as
`duckport_scan_parquet` for a selected Parquet object. CSV/JSON/XML and binary
attachment downloads continue to use the original GET response.

## Providers

Function names below have the prefix `duckport_scan_`.

| Suffix | Locator | Extra selectors | Default endpoint |
| --- | --- | --- | --- |
| `zenodo` | Numeric record ID | None | `https://zenodo.org` |
| `dataverse` | Dataset persistent ID, such as a DOI | Required `version` | `https://dataverse.harvard.edu` |
| `figshare` | Numeric article ID | Optional `version` | `https://api.figshare.com` |
| `internet_archive` | Item identifier | None | `https://archive.org` |
| `federal_register` | Document number | None | `https://www.federalregister.gov` |
| `govinfo` | Bulk directory, such as `FR/2026` | None | `https://www.govinfo.gov` |
| `hathifiles` | Literal `catalog` | None | `https://www.hathitrust.org` |
| `library_of_congress` | Resource identifier | None | `https://www.loc.gov` |
| `semantic_scholar` | Literal `s2orc_v2` | Required `release` | `https://api.semanticscholar.org` |
| `openstax` | OpenStax repository name | Required `revision`; optional `archive_endpoint` | `https://api.github.com` |
| `bhl` | Removed API route; use `bhl_open_data` | None | None |
| `regulations` | Record ID | `resource`: `documents` (default), `comments`, `dockets` | `https://api.regulations.gov` |
| `uspto` | Product identifier | `offset` default `0`, `page_size` default `100` | `https://api.uspto.gov` |
| `biorxiv` | Start date | Required `until`; `server` default `biorxiv` or `medrxiv`; `cursor` default `0` | `https://api.biorxiv.org` |

OpenStax resolves a revision to a commit and selects its repository ZIP, not a
rendered textbook table. `archive_endpoint` defaults to
`https://codeload.github.com`; that auxiliary connection does not inherit API
credentials. Other providers may likewise use separate download origins.

## Typed S2ORC v2

```sql
SELECT corpusid, text, body.annotations, bibliography.text
FROM duckport_scan_semantic_scholar('s2orc_v2', release := 'latest',
    file_name := 'part-00000');
```

Semantic Scholar only accepts `s2orc_v2` and always uses the upstream
`S2orcV2Accessor` with explicit gzip decompression for official release shards.
The logical `part-00000` names have no compression suffix. This mode expects
gzip-compressed JSONL; uncompressed local exports can use the generic
[JSON readers](json.md).

The fixed Arrow schema exposes body text as top-level `text: LargeUtf8`
(DuckDB `VARCHAR`). `body.text` is removed; `body.annotations` contains typed
annotation lists, and `bibliography.text` remains nested. Missing body text is
NULL. The accessor validates all typed fields even when a query projects only
`text` or counts rows. Prepared scans retain upstream partition planning.

Other dataset names are rejected before provider access. There is no binary
or generic-format fallback for this function.

## Authentication

Use [provider-scoped DuckDB secrets](../secrets.md#provider-apis). For example,
Figshare maps an HTTP secret token to its native `Authorization: token` scheme;
BHL maps the token to `apikey`; Regulations.gov, USPTO and Semantic Scholar map
API keys to a header. The Dataverse API-key header is `X-Dataverse-key`.
Internet Archive uses S3-type keys scoped to its HTTP endpoint and LOW auth.
Do not add credential-bearing options to the scan call.

The committed [HTTP SQL tests](../../test/sql/readers/) demonstrate fixture
selectors, redirects, authentication failures and cross-origin isolation without
requiring live provider accounts.

## Shared crawl execution

All selected-attachment providers bind their resolved namespace through the same
`SelectedAttachment` adapter. Decoded attachments retain the upstream native scan
planner and enter the common `CrawlAccessor` execution path when
`read_concurrency` is greater than one. SQL output remains ordered; projections,
batch size, scoped credentials and byte-I/O settings use the existing interfaces.
A binary attachment still produces one unchanged row from one document partition.

Native attachment planning from
[open-arrow-access #586](https://github.com/dentiny/open-arrow-access/pull/586)
is included in the crates.io 0.13.0 dependency; no temporary Git pin is required.

## BHL AWS object partitions

`duckport_scan_bhl_open_data(root, keys := [...])` reads an explicit, nonempty
list of bucket-relative object keys using open-arrow-access 0.20.1. Each selected
object is an independently planned input partition. The function does not list
the corpus or invent numbered shards; freeze disjoint key lists in job manifests.

```sql
SELECT decode(data) AS text
FROM duckport_scan_bhl_open_data('s3://bhl-open-data/',
    keys := ['ocr/item-295621/item-295621-61016244-0039.txt'],
    read_concurrency := 4);
```

The canonical public bucket uses anonymous access in `us-east-2`, without a BHL
API key or AWS credentials. A local mirror or another storage root uses ordinary
DuckPort storage configuration and scoped secrets; the keys remain relative to
that root. For example, use `'/datasets/bhl/'` for a local mirror.

`keys` is `VARCHAR[]`. Optional `format` and `record_path` have the attachment
semantics above; common scan options, including `read_concurrency`, still apply.
Default binary mode returns one `data: BLOB` row per selected object, preserving
all bytes, including blank OCR pages. It can retain TXT, JP2, XML or metadata
objects as bytes. Decoding a supported structured format is explicit, for example
`format := 'tsv'` for metadata tables. There is no OCR/image extraction or joined
page/item/title/rights schema in this function. `duckport_scan_bhl` now fails with a migration message because the upstream
BHL API was removed in 0.17.0. Use `duckport_scan_bhl_open_data` with explicit keys.
The new upstream typed page accessor requires caller-supplied metadata; this
existing binary object reader does not synthesize those joins.

## Joined BHL page shards

```sql
SELECT page_id, octet_length(data), metadata
FROM duckport_scan_bhl_pages(
    's3://bhl-open-data/', prefix := 'ocr/item-295621/',
    metadata_db := '/mnt/bhl/metadata.duckdb');
```

`prefix` selects one complete item/part OCR directory and `metadata_db` names a
completed local database created with `BhlMetadata::create` (for example, via the
open-arrow-web-crawl BHL CLI). It is opened read-only; the scanner never creates
or imports metadata. `snapshot_id` optionally overrides the metadata-fingerprint
row label. Listing, aggregate exclusion, identity validation and strict metadata
reconciliation use upstream `BhlShardAccessor`/`BhlShardReadRequest`. Relations
remain lists, original page bytes remain BLOBs, and projections omitting `data`
skip OCR downloads. Local/S3 mirrors use the existing storage/secret policies.
`duckport_scan_bhl_open_data` continues to read explicit raw objects.
