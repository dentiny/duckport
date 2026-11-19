# Reader catalog

Every function returns a DuckDB table and accepts `batch_size` (default 8192,
range 1–1,048,576) and `read_concurrency` (`'auto'` or a positive integer).
Automatic reads use DuckDB's `threads` limit where the source supports partitioning;
providers without a partition planner remain serial.
Reader-specific options below are named SQL arguments. Credentials and storage
regions/endpoints come from [scoped secrets](../secrets.md); scan policy comes
from [session settings](../storage.md).

## Files and datasets

| SQL function | Selection | Guide |
| --- | --- | --- |
| `duckport_scan_csv` | CSV files; optional `header`, `delimiter` | [CSV](csv.md) |
| `duckport_scan_tsv` | TSV files; optional `header` | [TSV](tsv.md) |
| `duckport_scan_json`, `duckport_scan_jsonl`, `duckport_scan_ndjson` | JSON object streams | [JSON](json.md) |
| `duckport_scan_parquet` | Parquet files | [Parquet](parquet.md) |
| `duckport_scan_xml` | XML files; required `record_path` | [XML](xml.md) |
| `duckport_scan_warc` | WARC files | [WARC](warc.md) |
| `duckport_scan_commoncrawl` | One crawl release root | [Common Crawl](warc.md) |
| `duckport_scan_commoncrawl_segment` | One segment root | [Common Crawl](warc.md) |
| `duckport_scan_atif` | One JSON/JSONL trajectory object | [ATIF](atif.md) |
| `duckport_scan_huggingface` | One dataset config/split's Parquet export | [Hugging Face](huggingface.md) |
| `duckport_scan_arxiv` | Manifest-selected bulk TAR shards; optional PDF metadata snapshot | [arXiv](arxiv.md) |

Standard file readers accept local/file/HTTP(S)/S3 objects and same-schema
[file lists, directories and globs](files.md). ATIF and provider readers retain
their own selection rules. Text/record readers use the enabled upstream outer-decompression support
(gzip, bzip2, LZ4 and Zstd); standalone Parquet expects the Parquet object itself.
Do not infer support for a particular format/container combination from a suffix.

## Metadata and structured packages

| SQL function | Locator and options | Output |
| --- | --- | --- |
| `duckport_scan_arxiv_metadata` | `records` or continuation token; OAI options | OAI-PMH metadata records |
| `duckport_scan_doab` | `records` or continuation token; OAI options | OAI-PMH metadata records |
| `duckport_scan_oapen` | `records` or continuation token; OAI options | OAI-PMH metadata records |
| `duckport_scan_gutenberg` | One RDF book object | One metadata row; linked books are not fetched |
| [`duckport_scan_pmc`](pmc.md) | One metadata JSON object or directory ending in `/`; optional `content_format` | Article metadata and selected `content_url`; linked content is not fetched |
| [`duckport_scan_pmc_shard`](pmc.md#read-one-inventory-shard-as-articles) | Bucket root; required `manifest`, `shard`; optional `content_format` | Article metadata from the selected inventory shard; linked content is not fetched |
| `duckport_scan_pmc_inventory` | One S3 inventory manifest or local mirror | Rows from its gzip CSV shards under the same bucket/root |
| `duckport_scan_courtlistener` | Root above `bulk-data/`; required `table_name`, `date` | One dated CSV export, retaining all columns as VARCHAR |
| `duckport_scan_meca` | One MECA ZIP object | Validated manifest entries with binary payloads in manifest order |

[OAI-PMH options](oai.md) include `endpoint`, `from_date`, `until_date`,
`set_spec`, `record_path`, `max_pages` and `max_records`.

```sql
SELECT ebook_id, title FROM duckport_scan_gutenberg('testdata/gutenberg/book.rdf');
SELECT pmcid, version, title FROM duckport_scan_pmc('testdata/pmc/PMC123.json');
SELECT * FROM duckport_scan_courtlistener('s3://free.law/',
    table_name := 'courts', date := '2026-09-30');
SELECT item_id, href, data FROM duckport_scan_meca('submission.meca');
```

## Selected provider attachments

`duckport_scan_zenodo`, `duckport_scan_dataverse`, `duckport_scan_figshare`,
`duckport_scan_internet_archive`, `duckport_scan_federal_register`,
`duckport_scan_govinfo`, `duckport_scan_hathifiles`,
`duckport_scan_library_of_congress`, `duckport_scan_semantic_scholar`,
`duckport_scan_openstax`, `duckport_scan_bhl`, `duckport_scan_regulations`,
`duckport_scan_uspto` and `duckport_scan_biorxiv` each select one explicitly named
provider attachment. The [provider guide](providers.md) lists locators, required
selectors, endpoint defaults and decoding formats once, without repeating them
for every provider.
