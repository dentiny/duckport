# DuckPort

Read file formats and dataset service providers into DuckDB through [open-arrow-access 0.3.0](https://crates.io/crates/open-arrow-access/0.3.0).

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
COPY my_table TO 'output.lance' (FORMAT duckport_lance);
COPY my_table TO 'output.lance' (FORMAT duckport_lance, APPEND);
COPY (SELECT id, image_url AS image FROM my_table)
TO 'images.lance' (FORMAT duckport_lance, BLOB_COLUMNS ('image'), OVERWRITE);
```

The Lance sink uses the released open-arrow-sink 0.1.0 crates. URL columns selected
by `BLOB_COLUMNS` are downloaded and ingested as Lance blobs; NULL values remain
NULL and other strings remain strings. Destinations support local paths, `file://`,
and S3 with scoped DuckDB secrets. See [Lance sink](docs/writers/lance.md).
Builds require `protoc`.

The default mode is create. `APPEND` and `OVERWRITE` select the backend mode;
only one may be specified. Generic file splitting, partitioning, temporary-file
publication, and ordering options are rejected. COPY streams Arrow batches through
one writer, commits on success, and aborts on failure. `duckport_parquet` and
`duckport_iceberg` remain stubs that fail on open; DuckDB's native `FORMAT PARQUET`
remains available.

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
Scoped HTTP bearer tokens are used where the upstream service supports connection headers.
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
