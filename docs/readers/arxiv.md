# arXiv archives and metadata

Use `duckport_scan_arxiv` for original PDF/source TAR members. Use
`duckport_scan_arxiv_metadata` for a separate OAI-PMH harvest. A persisted
Parquet snapshot can join those metadata fields to PDFs during the bulk scan.

## Bulk archive scans

```sql
SELECT shard_uri, entry_index, entry_name, data
FROM duckport_scan_arxiv('/data/arxiv-mirror', kind := 'source', month := '2410');
SELECT * FROM duckport_scan_arxiv('/data/arxiv-mirror', kind := 'auto', month := '2410');
```

The locator roots storage above `pdf/` and `src/`. `kind` is `source` (the
DuckPort default), `pdf`, or `auto`. `month` is optional YYMM text; omitting it
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
not a content-verification guarantee. Entries remain sequential within each TAR.
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

This follows the conversion service's persisted-snapshot contract: one selected
Parquet file is read completely, indexed with upstream `ArxivMetadataIndex`, and
joined to PDFs before output. The four bulk columns are retained; `metadata` is
a non-null struct containing **every snapshot column with its original Arrow
type**. Use `metadata.title`, for example, if the snapshot includes `title`.
The `data` column already contains bytes; do not put it in `BLOB_COLUMNS`, which
selects URL strings.

| Option | Default | Constraint |
| --- | --- | --- |
| `metadata_uri` | Unset | One local/file/HTTP(S)/S3 `.parquet` object, no globs or directory inputs |
| `metadata_id_column` | `identifier` | A nonempty string column name; explicit use requires `metadata_uri` |

A snapshot requires `kind := 'pdf'`. IDs must be non-null, unique, unversioned
article IDs or `oai:arXiv.org:<id>` identifiers. PDF version suffixes stay in
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
