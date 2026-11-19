# Provider attachments

These functions resolve a provider's file namespace, select one `file_name`, and
optionally decode it as records. They do not recursively download a provider's
whole catalog or follow arbitrary discovery pages.

```sql
SELECT * FROM duckport_scan_figshare('21422193',
    file_name := 'records.csv', format := 'csv');
SELECT * FROM duckport_scan_dataverse('doi:10.7910/DVN/example',
    version := '1.0', file_name := 'records.parquet', format := 'parquet');
```

Identifiers and file names above illustrate the API; replace them with names
present in the selected provider object. File names are logical provider paths,
not necessarily the URL basename. Unknown selections fail.

## Shared options

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
| `semantic_scholar` | Dataset name, such as `s2orc` | Required `release` | `https://api.semanticscholar.org` |
| `openstax` | OpenStax repository name | Required `revision`; optional `archive_endpoint` | `https://api.github.com` |
| `bhl` | Numeric item ID | None | `https://www.biodiversitylibrary.org` |
| `regulations` | Record ID | `resource`: `documents` (default), `comments`, `dockets` | `https://api.regulations.gov` |
| `uspto` | Product identifier | `offset` default `0`, `page_size` default `100` | `https://api.uspto.gov` |
| `biorxiv` | Start date | Required `until`; `server` default `biorxiv` or `medrxiv`; `cursor` default `0` | `https://api.biorxiv.org` |

OpenStax resolves a revision to a commit and selects its repository ZIP, not a
rendered textbook table. `archive_endpoint` defaults to
`https://codeload.github.com`; that auxiliary connection does not inherit API
credentials. Other providers may likewise use separate download origins.

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

This integration currently pins the upstream attachment-planning revision from
[open-arrow-access #586](https://github.com/dentiny/open-arrow-access/pull/586);
a released dependency must replace that pin before merging.
