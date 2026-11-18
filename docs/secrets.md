# Secrets and authentication

DuckPort reads credentials from DuckDB's secret manager at reader/writer open.
Use scopes matching the URI that performs the operation. A data-source locator,
provider API endpoint, media URL, metadata snapshot and output destination can all
require different secrets.

```sql
INSTALL httpfs;
LOAD httpfs;
```

This registers the S3 and Hugging Face secret types used below. Secrets created
without `PERSISTENT` are session secrets. Do not store credentials in repository
examples, query URLs, scan parameters or COPY options. Use actual account values
in place of the example placeholders.

## S3 and compatible endpoints

```sql
CREATE SECRET input_s3 (
    TYPE s3, KEY_ID 'input-access-key', SECRET 'input-secret-key',
    SESSION_TOKEN 'temporary-session-token', REGION 'us-east-1',
    SCOPE 's3://input-bucket/'
);
CREATE SECRET output_s3 (
    TYPE s3, KEY_ID 'output-access-key', SECRET 'output-secret-key',
    REGION 'us-east-1', ENDPOINT 'localhost:9000', USE_SSL false,
    URL_STYLE 'path', SCOPE 's3://output-bucket/'
);
```

Omit `SESSION_TOKEN` for long-lived keys. Supported fields are `KEY_ID`, `SECRET`,
`SESSION_TOKEN`, `REGION`, `ENDPOINT`, `USE_SSL` and `URL_STYLE` (`path` or `vhost`).
An endpoint without a scheme receives `http://` or `https://` from `USE_SSL`.
Use a different scope for each bucket/prefix/identity. DuckPort reads resolved
secret values; it does not inherit another extension's storage connection merely
because that extension read the source query.

## HTTP files and media URLs

```sql
CREATE SECRET bearer_media (
    TYPE http, BEARER_TOKEN 'access-token',
    SCOPE 'https://media.example.com/private/'
);
CREATE SECRET api_media (
    TYPE http, EXTRA_HTTP_HEADERS MAP {'X-API-Key': 'api-key'},
    SCOPE 'https://api.example.com/files/'
);
```

Use `EXTRA_HTTP_HEADERS` for a custom Authorization scheme or API-key header.
Duplicate/conflicting authentication headers, invalid headers and protected
transport-header overrides are rejected. Embedded URL credentials and query or
fragment components are not accepted by direct DuckPort file locators.

Direct HTTP file reads and HTTP Blob downloads select the longest matching
secret scope on each same-origin redirect. Leaving a scope removes its headers.
A cross-origin hop disables credentials for the rest of that redirect chain,
even if it returns to the original origin. HTTPS-to-HTTP downgrades are rejected.
Provider-owned download connections also isolate API credentials from auxiliary
origins; their own upstream transport controls redirect behavior.

## Hugging Face

```sql
CREATE SECRET private_hub (
    TYPE huggingface, TOKEN 'hf-access-token',
    SCOPE 'hf://datasets/owner/private-repo'
);
SELECT * FROM duckport_scan_huggingface('owner/private-repo', split := 'train');
```

The scope uses the logical `hf://datasets/...` namespace, not the Hub HTTPS API
URL. A matching secret takes precedence over environment/cache credentials.
`HF_HUB_DISABLE_IMPLICIT_TOKEN=1` disables implicit fallback. `HF_ENDPOINT` can
select a custom Hub endpoint. Generic Lance `BLOB_COLUMNS` can also reference an
individual Hub file, with its own matching secret; see the [Lance guide](writers/lance.md).

## Provider APIs

Scope a provider secret to its primary API endpoint or a prefix covering the
specific API request. Custom `endpoint` options change that lookup URI.

| Provider | Secret and mapping |
| --- | --- |
| Figshare | HTTP `BEARER_TOKEN` becomes `Authorization: token <value>` |
| Dataverse | HTTP `EXTRA_HTTP_HEADERS MAP {'X-Dataverse-key': 'key'}`; alternatively `BEARER_TOKEN` for OIDC, not both |
| BHL | HTTP `BEARER_TOKEN` becomes the native `apikey` query field |
| Semantic Scholar, Regulations.gov, USPTO | HTTP `BEARER_TOKEN` or `X-API-Key` extra header becomes the native API-key header; do not configure both |
| Zenodo | HTTP `BEARER_TOKEN` through the upstream token API; extra headers are rejected |
| OpenStax, bioRxiv, Federal Register, GovInfo, HathiFiles, Library of Congress, OAI-PMH | HTTP bearer tokens and allowed extra headers on the primary connection |
| Internet Archive | S3-type `KEY_ID`/`SECRET` scoped to the HTTP endpoint; mapped to `Authorization: LOW <key>:<secret>` |

```sql
CREATE SECRET dataverse_api (
    TYPE http, EXTRA_HTTP_HEADERS MAP {'X-Dataverse-key': 'api-token'},
    SCOPE 'https://dataverse.harvard.edu/'
);
CREATE SECRET figshare_api (
    TYPE http, BEARER_TOKEN 'personal-or-oauth-token',
    SCOPE 'https://api.figshare.com/'
);
CREATE SECRET archive_account (
    TYPE s3, KEY_ID 'archive-key', SECRET 'archive-secret',
    SCOPE 'https://archive.org/'
);
```

Dataverse accepts only `X-Dataverse-key` as an extra header. Zenodo, Hugging Face
and Internet Archive reject extra HTTP headers. Internet Archive does not accept
session tokens through this adapter. Public resources can work anonymously when
the provider permits it; BHL and API-key-only adapters enforce their required key.

## arXiv PDFs and metadata in different stores

```sql
CREATE SECRET arxiv_pdf (
    TYPE s3, KEY_ID 'pdf-access-key', SECRET 'pdf-secret-key',
    REGION 'us-east-1', SCOPE 's3://arxiv/'
);
CREATE SECRET arxiv_snapshot (
    TYPE s3, KEY_ID 'snapshot-access-key', SECRET 'snapshot-secret-key',
    REGION 'snapshot-region', ENDPOINT 'snapshot-storage.example.com',
    URL_STYLE 'path', SCOPE 's3://metadata-bucket/'
);
SELECT metadata.title, data
FROM duckport_scan_arxiv('s3://arxiv/', kind := 'pdf', month := '2410',
    requester_pays := true,
    metadata_uri := 's3://metadata-bucket/arxiv/2410.parquet');
```

The PDF root and `metadata_uri` resolve separate secrets, regions and endpoints.
Requester-pays is explicit and applies only to the arXiv archive storage, not the
metadata snapshot. Unlike the service URI's `metadata_region` parameter,
DuckPort configures the snapshot's region through its own scoped S3 secret.
A local snapshot needs no secret; an HTTPS snapshot uses its own HTTP scope.
[Snapshot requirements](readers/arxiv.md#pdf-and-metadata-in-one-table) include
complete matching IDs and an in-memory index.

## Iceberg catalog and table storage

```sql
CREATE SECRET iceberg_catalog (
    TYPE http, BEARER_TOKEN 'catalog-token', SCOPE 'https://catalog.example.com'
);
CREATE SECRET iceberg_data (
    TYPE s3, KEY_ID 'data-key', SECRET 'data-secret', REGION 'us-east-1',
    SCOPE 's3://warehouse-bucket/'
);
COPY (SELECT 1 AS id) TO 'analytics.events' (
    FORMAT duckport_iceberg, CATALOG_URI 'https://catalog.example.com',
    WAREHOUSE 'warehouse', STORAGE_ROOT 's3://warehouse-bucket/'
);
```

The catalog token comes from `CATALOG_URI`; data credentials come from
`STORAGE_ROOT`. Catalog extra headers and explicitly empty tokens are rejected.
Catalog-vended storage credentials are not used. Create the namespace first and
follow the [Iceberg guide](writers/iceberg.md).

## Secret changes

Readers/writers use snapshots taken at open; changing a secret does not mutate
an active operation. ATIF rows retain source policy and document information,
not credentials; a later COPY resolves the current secrets again. Remove a
session secret explicitly when finished, for example `DROP SECRET private_hub;`.
