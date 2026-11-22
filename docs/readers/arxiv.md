# arXiv archives and metadata

Use `duckport_scan_arxiv` for bulk PDF, attachment-free LaTeX, caller-supplied
XML shards, original TAR members or explicit article representations. Use
`duckport_scan_arxiv_metadata` for a separate OAI-PMH harvest. A persisted
Parquet snapshot can join those metadata fields to PDFs during the bulk scan.

## Choose an article format

```sql
-- Original article bytes from the website (or a mirror with pdf/src/html routes).
SELECT article_id, format, source_uri, data
FROM duckport_scan_arxiv('https://arxiv.org/',
    article_id := '2410.00001v2', format := 'pdf');
SELECT * FROM duckport_scan_arxiv('https://arxiv.org/',
    article_id := '2410.00001v2', format := 'html');
SELECT * FROM duckport_scan_arxiv('https://arxiv.org/',
    article_id := '2410.00001v2', format := 'latex');
```

`format` accepts `pdf`, `latex` or `html` (ASCII case-insensitive). With `article_id`, it is required
and selects exactly `pdf/<id>`, `src/<id>` or `html/<id>` beneath the root.
Modern IDs, including explicit version suffixes, are accepted; legacy IDs are
not supported by the upstream article-route validator.
Unavailable representations fail; no implicit fallback or HTML bulk discovery
is performed. Pin a versioned ID for reproducible selection.

Article output columns are `article_id`, `format`, `source_uri`, `entry_index`,
`entry_name` and `data`. `source_uri` is the relative route under the supplied
root. `data` contains original bytes: PDF is not rendered or text-extracted,
HTML is not parsed and its external assets are not fetched. LaTeX returns the
original source payload, which can be a compressed package or a single file;
it is not expanded or compiled. This reader does not expose the upstream
opt-in source-package expansion API.

Article mode cannot combine `article_id` with `kind`, `month`, `shard`, `metadata_uri`,
`metadata_id_column` or `requester_pays`. Those options describe bulk archives
and snapshot joins. Storage credentials, scan byte options and read metrics
still use the shared source path.

Without `article_id`, `format := 'pdf'` and `format := 'latex'` emit the common
bulk content schema below. This changes bulk `format := 'latex'` from raw source
packages to extracted LaTeX bytes. `format := 'xml'` requires `shard` naming a
caller-owned XML TAR; arXiv has no official XML full-text inventory. XML is passed
through as bytes, not generated from LaTeX or OAI metadata. `format := 'html'`
requires `article_id`. Do not combine `format` with the legacy `kind` option.

## Common bulk content output

```sql
SELECT article_id, source_name, data
FROM duckport_scan_arxiv('s3://arxiv/', format := 'latex',
    shard := 'src/arXiv_src_2410_001.tar', requester_pays := true,
    read_concurrency := 8);
```

All three bulk formats emit the same ten Arrow fields (non-null unless noted):

| Column | Arrow type | Content |
| --- | --- | --- |
| `article_id` | `LargeUtf8` | Normalized, unversioned article ID |
| `format` | `LargeUtf8` | `pdf`, `latex` or `xml` |
| `shard_uri` | `LargeUtf8` | Shard locator |
| `entry_index` | `UInt64` | Outer TAR entry index |
| `entry_name` | `LargeBinary` | Original outer member name |
| `source_index` | `UInt64` | Inner member index; zero for PDF/XML |
| `source_name` | `LargeBinary` | Inner member name; empty for PDF/XML |
| `data` | `LargeBinary` | PDF/XML bytes or decoded UTF-8 LaTeX bytes |
| `data_type` | `LargeUtf8` | Payload type: `latex`, `pdf` or `xml` |
| `metadata` | nullable `Struct` | Full typed arXivRaw OAI record, fetched automatically |

Binary fields appear as SQL `BLOB`. LaTeX packages produce one row per body
`.tex` member, preserving duplicate member positions; images and `anc/`
attachments are excluded. Inputs with no usable LaTeX or invalid encoding fail.
No compilation, PDF text extraction, or asset downloading is performed.
The content schema has ten fields, including nullable `metadata` with the full
arXivRaw OAI record schema. Content scans fetch it automatically; the legacy raw PDF snapshot join below remains separate.

### Exact fields of the content `metadata` struct (0.17.0)

`metadata` contains the **whole OAI record**, including its header; it is not a
flat article struct. The literal Arrow field names retain namespace prefixes,
colons, hyphens and attribute markers. The following tree lists every field in
schema order. `?` means nullable; a field without `?` is non-null when its parent
struct or list element exists.

```text
metadata: Struct?
├── oai:header: Struct
│   ├── oai:identifier: Utf8?
│   ├── oai:datestamp: Utf8?
│   ├── @status: Utf8?
│   └── oai:setSpec: List<Utf8?>?
└── oai:metadata: Struct?
    └── arxiv:arXivRaw: Struct?
        ├── arxiv:id: Utf8?
        ├── arxiv:submitter: Utf8?
        ├── arxiv:title: Utf8?
        ├── arxiv:authors: Utf8?
        ├── arxiv:categories: Utf8?
        ├── arxiv:comments: Utf8?
        ├── arxiv:proxy: Utf8?
        ├── arxiv:report-no: Utf8?
        ├── arxiv:acm-class: Utf8?
        ├── arxiv:msc-class: Utf8?
        ├── arxiv:journal-ref: Utf8?
        ├── arxiv:doi: Utf8?
        ├── arxiv:license: Utf8?
        ├── arxiv:abstract: Utf8?
        ├── @xsi:schemaLocation: Utf8?
        └── arxiv:version: List<item: Struct>?
            ├── @version: Utf8?
            ├── arxiv:date: Utf8?
            ├── arxiv:size: Utf8?
            └── arxiv:source_type: Utf8?
```

The `oai:setSpec` list's Arrow item field is named `oai:setSpec` and is
nullable. The `arxiv:version` list's item field is named `item` and is a
non-null struct; all four fields inside each version are nullable.

| Field / parent | Meaning |
| --- | --- |
| `oai:header.oai:identifier` | OAI record identifier, distinct from the article's `arxiv:id` |
| `oai:header.oai:datestamp` | OAI record datestamp used for harvesting updates |
| `oai:header.@status` | OAI header status, such as `deleted`, when supplied |
| `oai:header.oai:setSpec` | Repeated OAI set memberships |
| `arxiv:id` under `oai:metadata.arxiv:arXivRaw` | Article identifier retained as text, including leading zeros or legacy spelling |
| `arxiv:submitter` | Submitter text |
| `arxiv:title` | Article title |
| `arxiv:authors` | Original author text; not a list of normalized author objects |
| `arxiv:categories` | Original category text; not a list of category objects |
| `arxiv:comments` | Author comments |
| `arxiv:proxy` | Proxy submission text |
| `arxiv:report-no` | Report number text |
| `arxiv:acm-class` | ACM classification text |
| `arxiv:msc-class` | Mathematics Subject Classification text |
| `arxiv:journal-ref` | Journal reference text |
| `arxiv:doi` | DOI text; not a normalized DOI list |
| `arxiv:license` | License value supplied by arXiv |
| `arxiv:abstract` | Abstract text |
| `@xsi:schemaLocation` | XML schema-location attribute on `arxiv:arXivRaw` |
| `arxiv:version[].@version` | Version label supplied in the XML attribute |
| `arxiv:version[].arxiv:date` | Original version date text |
| `arxiv:version[].arxiv:size` | Original version size text; not parsed as a numeric byte count |
| `arxiv:version[].arxiv:source_type` | Original version source-type text |

All scalar values above are Arrow **`Utf8`**, not `LargeUtf8`, numeric, date or
list values. Missing optional scalar values are null. Set memberships and
version history retain list types even with one entry; an existing container
with no matching entries yields an empty list. The namespace URI selects XML
fields, while the Arrow names shown above use the fixed `oai`, `arxiv` and `xsi`
prefixes regardless of the input XML's prefix spelling.

In **0.17.0 content output**, each article is enriched automatically via OAI
`GetRecord`. Lookups are cached by normalized article ID within one accessor;
all TeX rows of an article share its typed metadata. A failed or mismatched OAI
response fails the scan. Deleted records retain their header and have null
`oai:metadata`. The top-level field remains nullable in the Arrow schema.
The complete field tree above is unchanged. Storage credentials are not forwarded
to OAI. `metadata_endpoint` selects a separate OAI origin for a mirror or local
fixture; the default is `https://oaipmh.arxiv.org`. Local content scans also need
reachable OAI metadata. Raw binary and individual-article modes do not fetch OAI.

`shard` is a relative `.tar` key beneath the root. It bypasses manifest discovery
and bucket listing and cannot combine with `month` or `article_id`. Explicit
content shards stream the TAR once and yield owned article partitions for bounded
parallel processing under `read_concurrency`; they do not index all headers or
reopen storage per article. LaTeX body members stream incrementally from each
package while attachments are filtered before materialization. Month scans also
parallelize article processing when the manifest selects a single TAR.
Caller-owned shards must remain
unchanged during a scan.

## Bulk archive scans

```sql
SELECT shard_uri, entry_index, entry_name, data
FROM duckport_scan_arxiv('/data/arxiv-mirror', kind := 'source', month := '2410');
SELECT * FROM duckport_scan_arxiv('/data/arxiv-mirror', kind := 'auto', month := '2410');
```

The locator roots storage above `pdf/` and `src/`. `kind` selects original,
unexpanded members and is `source` (the DuckPort default), `pdf`, or `auto`. `month` is optional YYMM text; omitting it
selects all manifest months. The accessor reads known manifest keys, then only
the selected TAR shards. It does not enumerate the bucket.

`auto` probes PDF then source manifests and includes all available distributions.
Only a missing manifest is skipped; permission, malformed-data and other I/O
errors fail the scan. Both missing manifests fail; an existing manifest with no
matching month returns no rows. Selecting both distributions can produce two
representations of an article, so choose `pdf` explicitly for a PDF-only dataset.

| Column | SQL-facing value |
| --- | --- |
| `shard_uri` | Upstream shard locator |
| `entry_index` | Entry index within the shard |
| `entry_name` | Original member-name bytes |
| `data` | Original member payload bytes |

TAR entry/size limits apply upstream. Manifest checksums/counts are metadata,
not a content-verification guarantee. Raw `kind` entries remain sequential within
each TAR. `shard` also works with `kind` to bypass manifests.
`batch_size` is a row target; the upstream request also has a soft payload target.

For the official requester-pays bucket, configure a scoped S3 secret and opt in:

```sql
SELECT * FROM duckport_scan_arxiv('s3://arxiv/', kind := 'pdf', month := '2410',
    requester_pays := true);
```

`requester_pays` defaults to false and requires an S3 root with explicit static
credentials. It is applied to archive operations only. No implicit billing opt-in
is made for local/HTTP mirrors or metadata snapshots.

## PDF and metadata in one table

```sql
COPY (
    SELECT * FROM duckport_scan_arxiv('s3://arxiv/',
        kind := 'pdf', month := '2410', requester_pays := true,
        metadata_uri := 's3://metadata-bucket/arxiv/2410.parquet',
        metadata_id_column := 'identifier')
) TO 'arxiv_2410.lance' (FORMAT duckport_lance);
```

This legacy raw PDF mode reads one selected Parquet file completely, indexes it with upstream `ArxivMetadataIndex`, and
joins it to PDFs before output. The four bulk columns are retained; `metadata` is
a non-null struct containing **every snapshot column with its original Arrow
type**. Use `metadata.title`, for example, if the snapshot includes `title`.
The `data` column already contains bytes; do not put it in `BLOB_COLUMNS`, which
selects URL strings.

| Option | Default | Constraint |
| --- | --- | --- |
| `metadata_uri` | Unset | One local/file/HTTP(S)/S3 `.parquet` object, no globs or directory inputs |
| `metadata_id_column` | `identifier` | A nonempty string column name; explicit use requires `metadata_uri` |

A snapshot requires `kind := 'pdf'` and cannot combine with `format`. IDs must be
non-null, unique, unversioned article IDs or `oai:arXiv.org:<id>` identifiers. PDF version suffixes stay in
`entry_name` while matching uses the unversioned article ID; legacy IDs are
normalized by the upstream crate. Duplicate/invalid IDs and PDFs missing a
metadata match fail rather than silently dropping fields or rows. Metadata-only
records do not create PDF rows. Projection happens after matching.

Acquire a stable snapshot separately, retain the bibliographic fields needed by
your output, and reuse it. Default OAI header columns alone are not a full
bibliographic snapshot. This scan does not harvest OAI records or fetch metadata
per PDF. The index is in memory: monthly snapshots help bound memory. Snapshot
metadata can describe current/version-history information; it does not promise
historical-version-specific metadata for every PDF.

The archive root and snapshot URI select independent secrets and storage
regions. Configure the snapshot's own S3 endpoint/region in its secret; DuckPort
does not expose a separate `metadata_region` SQL parameter. See
[the complete credential example](../secrets.md#arxiv-pdfs-and-metadata-in-different-stores).

## OAI-PMH harvesting

```sql
SELECT identifier, datestamp, status, set_specs
FROM duckport_scan_arxiv_metadata('records', from_date := '2024-10-01',
    until_date := '2024-10-31', set_spec := 'cs', max_pages := 100);
```

The default endpoint is `https://oaipmh.arxiv.org` with upstream `arXivRaw`
metadata prefix. Public harvesting needs no key; a custom endpoint may use a
scoped HTTP secret. Pagination, limits, deleted records and custom XML mappings
are described in the [OAI-PMH guide](oai.md). Coordinate harvesting and reuse
persisted snapshots rather than harvesting independently for every bulk shard.

The content schema appends `data_type` in open-arrow-access 0.15.0. Explicit-format scans still use the requested format; the generic cell fallback API does not enable automatic arXiv PDF fallback.

## HTTP reads in open-arrow-access 0.20.1

Article PDF, source and HTML downloads use one sequential GET per representation,
without HEAD probes or conditional Range replays. Official arxiv.org article
requests share a three-second request interval across SQL scans and hold admission
until the response is consumed or dropped. Custom mirrors retain their configured
transport policy. Automatic bulk-content OAI metadata lookups retain the upstream
shared request limiter.

Raw explicit shards and manifests are streamed sequentially. Legacy standalone
TeX is accepted; PDF/PostScript/DVI-only source packages report unavailable LaTeX,
and corrupt archives remain errors. No automatic PDF fallback is introduced.
