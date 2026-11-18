# DuckPort

Read file formats and dataset service providers into DuckDB through [open-arrow-access 0.8.0](https://crates.io/crates/open-arrow-access/0.8.0).

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

Parquet page codecs: Snappy, Zstd, LZ4, gzip, Brotli. For standalone Parquet scans, archive members, directory/glob scans, HTTP query/signed URLs, and outer compression such as `.parquet.gz` are unsupported.

Top-level column projection is passed to readers at scan time. For example,
`SELECT record_id FROM duckport_scan_warc(...)` excludes `body` from the Arrow
output; `COUNT(*)` uses a zero-column scan that preserves row counts. Columns
needed by filters are still requested, and filters execute in DuckDB. Nested-field
and predicate pushdown are not enabled. Projection savings depend on the format:
Parquet can skip unselected column data, while WARC must still parse and consume
record bodies even when they are omitted from Arrow output. Schema binding may
still inspect unselected fields.

## Storage timeouts and retries

The current connection's settings are copied when a reader or COPY writer opens.
Remote HTTP(S)/S3 file readers and all three sinks' S3 destination operations
share this policy, including provider-owned directory reads (arXiv, Common Crawl,
PMC inventory). Provider HTTP metadata and attachment requests, including Hugging
Face discovery and Parquet reads, use the same policy through the upstream HTTP
transport. Auxiliary provider connections also inherit it.

| Setting | Default | Meaning |
| --- | --- | --- |
| `conversion_storage_timeout_ms` | `30000` | Control operation timeout per attempt, such as `stat`; provider HTTP request timeout |
| `conversion_storage_io_timeout_ms` | `30000` | Timeout per I/O operation and reader/writer body method |
| `conversion_storage_retry_max_times` | `5` | Retries after the initial attempt; `0` disables retries |
| `conversion_storage_retry_min_delay_ms` | `500` | Initial exponential backoff |
| `conversion_storage_retry_max_delay_ms` | `10000` | Maximum exponential backoff before jitter |
| `conversion_storage_retry_factor` | `2.0` | Backoff multiplier, finite and between `1` and `f32::MAX` |

```sql
SET conversion_storage_timeout_ms = 30000;
SET conversion_storage_io_timeout_ms = 30000;
SET conversion_storage_retry_max_times = 5;
SET conversion_storage_retry_min_delay_ms = 500;
SET conversion_storage_retry_max_delay_ms = 10000;
SET conversion_storage_retry_factor = 2.0;
```

Timeouts and delays must be positive; the maximum delay must be at least the
minimum delay when opening a reader or writer. `RESET` restores each default.
Timeouts apply to each attempt, rather than to the entire query. OpenDAL retries
temporary errors with exponential backoff and jitter, with timeout inside retry.
Provider HTTP transports apply retry safety rules and do not replay a response
body after bytes have been delivered. Local filesystem operations, Iceberg REST
catalog requests, and backend-owned `BLOB_COLUMNS` URL downloads retain their
library defaults. Other DuckDB extensions use their own settings.

## Authentication with DuckDB secrets

Use DuckDB's existing `CREATE SECRET` system. Credentials are selected by the
request/source URL's `SCOPE`; do not put tokens in scan/COPY options or URLs.
Load `httpfs` to register its S3 and Hugging Face secret types.

```sql
LOAD httpfs;
CREATE SECRET object_storage (
    TYPE s3, KEY_ID 'access-key', SECRET 'secret-key',
    SESSION_TOKEN 'optional-session-token', REGION 'us-east-1',
    SCOPE 's3://private-bucket/'
);
CREATE SECRET private_hub (
    TYPE huggingface, TOKEN 'hf-access-token',
    SCOPE 'hf://datasets/owner/private-repo'
);
CREATE SECRET authenticated_http (
    TYPE http, BEARER_TOKEN 'access-token',
    SCOPE 'https://service.example.com/private/'
);
CREATE SECRET custom_http_auth (
    TYPE http, EXTRA_HTTP_HEADERS MAP {'X-API-Key': 'api-key'},
    SCOPE 'https://api.example.com/files/'
);
```

| Backend/service | DuckDB secret type and authentication fields |
| --- | --- |
| S3-compatible storage | `TYPE s3`: `KEY_ID`, `SECRET`, optional `SESSION_TOKEN`; endpoint/region/addressing settings belong to the same secret. |
| Hugging Face | `TYPE huggingface`: `TOKEN`, scoped to the logical `hf://datasets/...` path. |
| HTTP files and HTTP Blob sources | `TYPE http`: `BEARER_TOKEN` or `EXTRA_HTTP_HEADERS` for API keys/custom Authorization. |
| Zenodo, OpenStax, and bearer-enabled provider APIs | `TYPE http`: `BEARER_TOKEN`, scoped to the provider API endpoint. |
| Figshare | `TYPE http`: raw token in `BEARER_TOKEN`; the adapter uses Figshare's `Authorization: token` scheme. |
| Dataverse | `TYPE http`: `EXTRA_HTTP_HEADERS MAP {'X-Dataverse-key': 'api-token'}`; use `BEARER_TOKEN` for OIDC installations instead. |
| Semantic Scholar, Regulations.gov, USPTO | `TYPE http`: API key in `BEARER_TOKEN` or `EXTRA_HTTP_HEADERS` under `X-API-Key`. |
| BHL | `TYPE http`: API key in `BEARER_TOKEN`; the adapter maps it to the `apikey` query field. |
| Internet Archive | `TYPE s3`: `KEY_ID` and `SECRET`, scoped to its HTTP endpoint; the adapter uses LOW authentication. |
| Iceberg REST catalog | `TYPE http`: `BEARER_TOKEN` scoped to `CATALOG_URI`, independently of the secret for `STORAGE_ROOT`. |

Connection-based provider adapters (Figshare, OpenStax, bioRxiv, Federal Register,
GovInfo, HathiFiles, Library of Congress, OAI-PMH, BHL, Semantic Scholar,
Regulations.gov, and USPTO) also apply `EXTRA_HTTP_HEADERS` to their primary
origin. Duplicate headers, transport-header overrides, and a token combined with
its mapped authentication header are rejected before I/O.

Dataverse accepts `X-Dataverse-key` as its only extra header, or `BEARER_TOKEN`
for OIDC; configuring both is rejected. Zenodo, Hugging Face, and Internet Archive
do not support extra headers and reject them rather than silently ignoring them.

For example, Dataverse API-key and Figshare secrets use the standard HTTP type:

```sql
CREATE SECRET dataverse_auth (
    TYPE http, EXTRA_HTTP_HEADERS MAP {'X-Dataverse-key': 'dataverse-api-token'},
    SCOPE 'https://dataverse.harvard.edu/'
);
CREATE SECRET figshare_auth (
    TYPE http, BEARER_TOKEN 'figshare-personal-or-oauth-token',
    SCOPE 'https://api.figshare.com/v2/articles/'
);
CREATE SECRET catalog_auth (
    TYPE http, BEARER_TOKEN 'catalog-access-token',
    SCOPE 'https://catalog.example.com'
);
```

Use narrow scopes when different datasets/services have different identities.
Provider credentials stay on their intended origin. COPY source credentials are
resolved independently of the destination; an S3 destination key or catalog token
is not a credential for an unrelated media URL.

For HTTP file reads and HTTP Blob downloads, same-origin redirects reselect the
longest matching credential scope. Leaving a scope removes its credentials. Any
cross-origin hop disables credentials for the rest of that redirect chain, even
if it returns to the original origin. HTTPS-to-HTTP redirects are rejected.

## Write

```sql
COPY (SELECT id, image_url AS image FROM my_table)
TO 'images.lance' (FORMAT duckport_lance, BLOB_COLUMNS ('image'));
```

The [Lance sink](docs/writers/lance.md) supports create, APPEND and OVERWRITE,
including URL blob ingestion. The [Parquet sink](docs/writers/parquet.md) writes
dataset directories with create and APPEND. Builds require `protoc`.
The [Iceberg sink](docs/writers/iceberg.md) appends to existing tables through a REST catalog.
Native `FORMAT PARQUET` writes individual files. All three sinks support
[COPY sampling](docs/writers/README.md#copy-sampling) with `SAMPLE_PERCENT` and `SAMPLE_ROWS`.

## Progress and profiling

Enable the progress bar with `SET enable_progress_bar = true`. Single-file scans
and provider attachments report estimated encoded-byte progress when their size
is known; other scans remain indeterminate. Progress reaches 100% at reader EOF.

Readers report scanned rows and observed input bytes to DuckDB's profiler.
COPY reports sink read/write bytes after a successful commit, including APPEND
metadata reads and finalization writes. Byte counts exclude provider discovery
and hidden transports such as Hugging Face discovery/shards.

```sql
SET profiling_coverage = 'ALL';
SET tracked_metrics = ['operator.type', 'operator.rows_scanned',
                       'io.total_bytes_read', 'io.total_bytes_written'];
PRAGMA enable_profiling = 'json';
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
Scoped HTTP secrets supply Zenodo bearer tokens for metadata and attachment reads:

```sql
CREATE SECRET zenodo_auth (
    TYPE http, BEARER_TOKEN 'your-token', SCOPE 'https://zenodo.org/api/records/'
);
```

Tokens are omitted after redirects to another origin. Public records also work
without a secret.
All reader options above are VARCHAR; `batch_size` remains BIGINT.

### Dataverse

```sql
SELECT * FROM duckport_scan_dataverse('<identifier>', file_name := '<logical attachment name>', version := '<version>');
```

Reads one explicitly named attachment from the provider-resolved file namespace.
`format` defaults to `binary` (one BLOB `data` column); `csv`, `tsv`, `json`,
`jsonl`, `parquet`, and `xml` decode records. XML requires `record_path`.
CSV/TSV expect headers. Text formats support outer compression. Binary mode buffers
the selected attachment into one BLOB; use a record format for large tabular files.
`endpoint` defaults to `https://dataverse.harvard.edu`. No discovery pagination or whole-dataset download
is implied. Unknown file names fail through the provider accessor.
Scoped HTTP bearer tokens are used where the upstream service supports connection headers.
All reader options above are VARCHAR; `batch_size` remains BIGINT.

### Figshare

```sql
SELECT * FROM duckport_scan_figshare('<identifier>', file_name := '<logical attachment name>');
```

Reads one explicitly named attachment from the provider-resolved file namespace.
`format` defaults to `binary` (one BLOB `data` column); `csv`, `tsv`, `json`,
`jsonl`, `parquet`, and `xml` decode records. XML requires `record_path`.
CSV/TSV expect headers. Text formats support outer compression. Binary mode buffers
the selected attachment into one BLOB; use a record format for large tabular files.
`endpoint` defaults to `https://api.figshare.com`. No discovery pagination or whole-dataset download
is implied. Unknown file names fail through the provider accessor.
A scoped HTTP secret stores the Figshare token in `BEARER_TOKEN`; DuckPort sends
`Authorization: token <value>` as required by the Figshare API. Tokens stay on
the API origin and are not forwarded to cross-origin attachment hosts.
All reader options above are VARCHAR; `batch_size` remains BIGINT.

### InternetArchive

```sql
SELECT * FROM duckport_scan_internet_archive('<identifier>', file_name := '<logical attachment name>');
```

Reads one explicitly named attachment from the provider-resolved file namespace.
`format` defaults to `binary` (one BLOB `data` column); `csv`, `tsv`, `json`,
`jsonl`, `parquet`, and `xml` decode records. XML requires `record_path`.
CSV/TSV expect headers. Text formats support outer compression. Binary mode buffers
the selected attachment into one BLOB; use a record format for large tabular files.
`endpoint` defaults to `https://archive.org`. No discovery pagination or whole-dataset download
is implied. Unknown file names fail through the provider accessor.
Anonymous reads remain the default. For account-authorized reads, load `httpfs`
and create an S3-type secret with Internet Archive keys scoped to the HTTP endpoint:

```sql
CREATE SECRET ia_account (
    TYPE s3, KEY_ID 'access-key', SECRET 'secret-key', SCOPE 'https://archive.org/'
);
```

The upstream service sends `Authorization: LOW <key>:<secret>` only to its origin.
These credentials do not grant access beyond the account's permissions or borrow books.
All reader options above are VARCHAR; `batch_size` remains BIGINT.

### FederalRegister

```sql
SELECT * FROM duckport_scan_federal_register('<identifier>', file_name := '<logical attachment name>');
```

Reads one explicitly named attachment from the provider-resolved file namespace.
`format` defaults to `binary` (one BLOB `data` column); `csv`, `tsv`, `json`,
`jsonl`, `parquet`, and `xml` decode records. XML requires `record_path`.
CSV/TSV expect headers. Text formats support outer compression. Binary mode buffers
the selected attachment into one BLOB; use a record format for large tabular files.
`endpoint` defaults to `https://www.federalregister.gov`. No discovery pagination or whole-dataset download
is implied. Unknown file names fail through the provider accessor.
Scoped HTTP bearer tokens are used where the upstream service supports connection headers.
All reader options above are VARCHAR; `batch_size` remains BIGINT.

### GovInfo

```sql
SELECT * FROM duckport_scan_govinfo('<identifier>', file_name := '<logical attachment name>');
```

Reads one explicitly named attachment from the provider-resolved file namespace.
`format` defaults to `binary` (one BLOB `data` column); `csv`, `tsv`, `json`,
`jsonl`, `parquet`, and `xml` decode records. XML requires `record_path`.
CSV/TSV expect headers. Text formats support outer compression. Binary mode buffers
the selected attachment into one BLOB; use a record format for large tabular files.
`endpoint` defaults to `https://www.govinfo.gov`. No discovery pagination or whole-dataset download
is implied. Unknown file names fail through the provider accessor.
Scoped HTTP bearer tokens are used where the upstream service supports connection headers.
All reader options above are VARCHAR; `batch_size` remains BIGINT.

### Hathifiles

```sql
SELECT * FROM duckport_scan_hathifiles('<catalog>', file_name := '<logical attachment name>');
```

Reads one explicitly named attachment from the provider-resolved file namespace.
`format` defaults to `binary` (one BLOB `data` column); `csv`, `tsv`, `json`,
`jsonl`, `parquet`, and `xml` decode records. XML requires `record_path`.
CSV/TSV expect headers. Text formats support outer compression. Binary mode buffers
the selected attachment into one BLOB; use a record format for large tabular files.
`endpoint` defaults to `https://www.hathitrust.org`. No discovery pagination or whole-dataset download
is implied. Unknown file names fail through the provider accessor.
Scoped HTTP bearer tokens are used where the upstream service supports connection headers.
All reader options above are VARCHAR; `batch_size` remains BIGINT.

### LibraryOfCongress

```sql
SELECT * FROM duckport_scan_library_of_congress('<identifier>', file_name := '<logical attachment name>');
```

Reads one explicitly named attachment from the provider-resolved file namespace.
`format` defaults to `binary` (one BLOB `data` column); `csv`, `tsv`, `json`,
`jsonl`, `parquet`, and `xml` decode records. XML requires `record_path`.
CSV/TSV expect headers. Text formats support outer compression. Binary mode buffers
the selected attachment into one BLOB; use a record format for large tabular files.
`endpoint` defaults to `https://www.loc.gov`. No discovery pagination or whole-dataset download
is implied. Unknown file names fail through the provider accessor.
Scoped HTTP bearer tokens are used where the upstream service supports connection headers.
All reader options above are VARCHAR; `batch_size` remains BIGINT.

### SemanticScholar

```sql
SELECT * FROM duckport_scan_semantic_scholar('<identifier>', file_name := '<logical attachment name>', release := '<release>');
```

Reads one explicitly named attachment from the provider-resolved file namespace.
`format` defaults to `binary` (one BLOB `data` column); `csv`, `tsv`, `json`,
`jsonl`, `parquet`, and `xml` decode records. XML requires `record_path`.
CSV/TSV expect headers. Text formats support outer compression. Binary mode buffers
the selected attachment into one BLOB; use a record format for large tabular files.
`endpoint` defaults to `https://api.semanticscholar.org`. No discovery pagination or whole-dataset download
is implied. Unknown file names fail through the provider accessor.
A scoped HTTP secret stores the API key in `BEARER_TOKEN`; DuckPort maps it to the provider authentication field.
All reader options above are VARCHAR; `batch_size` remains BIGINT.

### OpenStax

```sql
SELECT * FROM duckport_scan_openstax('<identifier>', file_name := '<logical attachment name>', revision := '<revision>');
```

Reads one explicitly named attachment from the provider-resolved file namespace.
`format` defaults to `binary` (one BLOB `data` column); `csv`, `tsv`, `json`,
`jsonl`, `parquet`, and `xml` decode records. XML requires `record_path`.
CSV/TSV expect headers. Text formats support outer compression. Binary mode buffers
the selected attachment into one BLOB; use a record format for large tabular files.
`endpoint` defaults to `https://api.github.com`. No discovery pagination or whole-dataset download
is implied. Unknown file names fail through the provider accessor.
Scoped HTTP bearer tokens are used where the upstream service supports connection headers.
All reader options above are VARCHAR; `batch_size` remains BIGINT.
`archive_endpoint` defaults to `https://codeload.github.com`; archive downloads do not inherit API credentials.

### Bhl

```sql
SELECT * FROM duckport_scan_bhl('<identifier>', file_name := '<logical attachment name>');
```

Reads one explicitly named attachment from the provider-resolved file namespace.
`format` defaults to `binary` (one BLOB `data` column); `csv`, `tsv`, `json`,
`jsonl`, `parquet`, and `xml` decode records. XML requires `record_path`.
CSV/TSV expect headers. Text formats support outer compression. Binary mode buffers
the selected attachment into one BLOB; use a record format for large tabular files.
`endpoint` defaults to `https://www.biodiversitylibrary.org`. No discovery pagination or whole-dataset download
is implied. Unknown file names fail through the provider accessor.
A scoped HTTP secret stores the API key in `BEARER_TOKEN`; DuckPort maps it to the provider authentication field.
All reader options above are VARCHAR; `batch_size` remains BIGINT.

### Regulations

```sql
SELECT * FROM duckport_scan_regulations('<identifier>', file_name := '<logical attachment name>');
```

Reads one explicitly named attachment from the provider-resolved file namespace.
`format` defaults to `binary` (one BLOB `data` column); `csv`, `tsv`, `json`,
`jsonl`, `parquet`, and `xml` decode records. XML requires `record_path`.
CSV/TSV expect headers. Text formats support outer compression. Binary mode buffers
the selected attachment into one BLOB; use a record format for large tabular files.
`endpoint` defaults to `https://api.regulations.gov`. No discovery pagination or whole-dataset download
is implied. Unknown file names fail through the provider accessor.
A scoped HTTP secret stores the API key in `BEARER_TOKEN`; DuckPort maps it to the provider authentication field.
All reader options above are VARCHAR; `batch_size` remains BIGINT.

### Uspto

```sql
SELECT * FROM duckport_scan_uspto('<identifier>', file_name := '<logical attachment name>');
```

Reads one explicitly named attachment from the provider-resolved file namespace.
`format` defaults to `binary` (one BLOB `data` column); `csv`, `tsv`, `json`,
`jsonl`, `parquet`, and `xml` decode records. XML requires `record_path`.
CSV/TSV expect headers. Text formats support outer compression. Binary mode buffers
the selected attachment into one BLOB; use a record format for large tabular files.
`endpoint` defaults to `https://api.uspto.gov`. No discovery pagination or whole-dataset download
is implied. Unknown file names fail through the provider accessor.
A scoped HTTP secret stores the API key in `BEARER_TOKEN`; DuckPort maps it to the provider authentication field.
All reader options above are VARCHAR; `batch_size` remains BIGINT.

### Biorxiv

```sql
SELECT * FROM duckport_scan_biorxiv('<identifier>', file_name := '<logical attachment name>', until := '<until>');
```

Reads one explicitly named attachment from the provider-resolved file namespace.
`format` defaults to `binary` (one BLOB `data` column); `csv`, `tsv`, `json`,
`jsonl`, `parquet`, and `xml` decode records. XML requires `record_path`.
CSV/TSV expect headers. Text formats support outer compression. Binary mode buffers
the selected attachment into one BLOB; use a record format for large tabular files.
`endpoint` defaults to `https://api.biorxiv.org`. No discovery pagination or whole-dataset download
is implied. Unknown file names fail through the provider accessor.
Scoped HTTP bearer tokens are used where the upstream service supports connection headers.
All reader options above are VARCHAR; `batch_size` remains BIGINT.

### Courtlistener

```sql
SELECT * FROM duckport_scan_courtlistener('s3://free.law/', table_name := 'courts', date := '2026-09-30');
```
The locator is a local directory or HTTP(S)/S3 root above `bulk-data/`.
Reads exactly one validated, dated `.csv.bz2` table export with a header; all columns
are retained as VARCHAR, including columns containing only null values. S3 uses the existing scoped DuckDB secret.

### Meca

```sql
SELECT item_id, href, data FROM duckport_scan_meca('submission.meca');
```
Reads validated MECA manifest entries as Arrow rows, preserving binary payloads,
manifest order, media types, and ZIP limits. Local, HTTP(S), and S3 use existing secrets.

### Arxiv

```sql
SELECT shard_uri, entry_index, entry_name, data
FROM duckport_scan_arxiv('/data/arxiv-mirror', kind := 'source', month := '2410');
```
The locator roots a local/HTTP(S)/S3 mirror above `pdf/` and `src/`. Reads only
manifest-selected TAR shards; `month` is YYMM, `kind` is `source` (default) or `pdf`.
Returns unchanged entry names and binary payloads, with upstream TAR limits.
For an official requester-pays S3 bucket, supply a scoped S3 secret and set
`requester_pays := true` to explicitly opt into requester billing. The option defaults
to false and requires S3 credentials. Example: `duckport_scan_arxiv('s3://arxiv/',
kind := 'source', month := '2410', requester_pays := true)`.

### Doab

```sql
SELECT * FROM duckport_scan_doab('records', from_date := '2026-01-01');
-- Pass an upstream resumption token as the locator to fetch the next page.
```
Fetches one OAI-PMH page and decodes its records, retaining deleted headers and
nested provider metadata. `until_date`, `set_spec`, and `record_path` are optional.
The default record path is `/oai:OAI-PMH/oai:ListRecords/oai:record`. The OAI namespace is bound to `oai`; inferred names such as `"oai:header"`
retain that prefix. `endpoint` defaults to `https://directory.doabooks.org`.
To obtain the next resumption token, scan the envelope with
`record_path := '/oai:OAI-PMH/oai:ListRecords'` and select `"oai:resumptionToken"`.
Automatic pagination is not enabled. Scoped HTTP bearer tokens are supported.

### Oapen

```sql
SELECT * FROM duckport_scan_oapen('records', from_date := '2026-01-01');
-- Pass an upstream resumption token as the locator to fetch the next page.
```
Fetches one OAI-PMH page and decodes its records, retaining deleted headers and
nested provider metadata. `until_date`, `set_spec`, and `record_path` are optional.
The default record path is `/oai:OAI-PMH/oai:ListRecords/oai:record`. The OAI namespace is bound to `oai`; inferred names such as `"oai:header"`
retain that prefix. `endpoint` defaults to `https://library.oapen.org`.
To obtain the next resumption token, scan the envelope with
`record_path := '/oai:OAI-PMH/oai:ListRecords'` and select `"oai:resumptionToken"`.
Automatic pagination is not enabled. Scoped HTTP bearer tokens are supported.

### arXiv metadata

```sql
SELECT * FROM duckport_scan_arxiv_metadata('records', from_date := '2026-01-01',
    until_date := '2026-01-31', set_spec := 'cs');
-- Pass the upstream resumption token as the locator to read the next page.
```

Reads one arXiv OAI-PMH metadata page using the upstream `arXivRaw` prefix.
The `from_date`, `until_date`, `set_spec`, `endpoint`, and `record_path` options
match the DOAB/OAPEN metadata readers; the default endpoint is
`https://oaipmh.arxiv.org`. The inferred XML records preserve deleted headers
and nested metadata. Set `record_path := '/oai:OAI-PMH/oai:ListRecords'` to
read the envelope and its `"oai:resumptionToken"` for explicit continuation.
A continuation token cannot be combined with date/set selectors.
Public metadata needs no key; scoped HTTP bearer secrets are supported for
custom endpoints. `duckport_scan_arxiv` continues to read bulk TAR members.
Empty pages, including `noRecordsMatch`, bind with the standard OAI header columns
`identifier`, `datestamp`, `status`, and `set_specs` and return zero rows. An explicit
`record_path` continues to select the requested XML envelope.
