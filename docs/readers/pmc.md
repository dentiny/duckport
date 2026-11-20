# PMC articles and inventory shards

DuckPort uses open-arrow-access 0.12.1. An article is one Arrow row with the
16 original typed metadata fields and derived `content_url`. The duplicate
`metadata` struct is removed, and the TXT URL column is named `text_url`.

## Select content

`duckport_scan_pmc` accepts one metadata JSON object or a directory ending in
`/`. `content_format` is `pdf` (default), `xml`, or `txt`. It selects the matching
URL as `content_url`; a missing URL stays null, without substituting another
format. Scanning metadata never downloads linked article content.

```sql
SELECT pmcid, text_url, content_url
FROM duckport_scan_pmc('s3://pmc-oa-opendata/metadata/PMC10009416.1.json',
                       content_format := 'txt');
```

Source storage uses the standard DuckPort secrets and storage settings. Configure
anonymous S3 access and region `us-east-1` for the public PMC bucket.

## COPY source options and download content

`EXTRA_OPTIONS` forwards a map of named options to each explicit
`duckport_scan_*` source in the COPY query before it binds. Both SQL `MAP`
and struct literals are accepted. The reader validates option names and types;
unknown options and options already supplied in the reader call fail binding.
A nonempty map without an explicit DuckPort reader also fails binding. For a
query with several readers, every reader must accept every forwarded option.

Select TXT explicitly for a text dataset, rename the selected URL using SQL,
and mark that renamed column for blob ingestion:

```sql
COPY (
    SELECT * EXCLUDE (content_url), content_url AS content
    FROM duckport_scan_pmc('s3://pmc-oa-opendata/metadata/')
) TO '/data/pmc-text.lance' (
    FORMAT duckport_lance,
    EXTRA_OPTIONS MAP {'content_format': 'txt'},
    BLOB_COLUMNS ('content'),
    READ_CONCURRENCY 8,
    WRITE_CONCURRENCY 'auto'
);
```

The sink downloads only `content`. `pdf_url`, `xml_url`, and `text_url` remain
original URL strings. `content` contains raw TXT bytes, not PDF or XML bytes.
Use distinct destinations for independently submitted conversions.

## Read one inventory shard as articles

`duckport_scan_pmc_shard` accepts bucket-root storage plus a pinned manifest
and one exact bucket-relative `.csv.gz` shard key from that manifest:

```sql
SELECT pmcid, content_url
FROM duckport_scan_pmc_shard(
    's3://pmc-oa-opendata/',
    manifest := 'inventory-reports/pmc-oa-opendata/metadata/2026-11-18T01-00Z/manifest.json',
    shard := 'inventory-reports/pmc-oa-opendata/metadata/data/13cbabe4-91e8-4d22-977f-adc2fc7eb6cb.csv.gz',
    content_format := 'txt',
    read_concurrency := 8
);
```

It reads the selected CSV and its referenced metadata objects, retaining native
article partitions for parallel reads. It does not read other CSV shards or
fetch article content. The manifest is required to validate membership and read
the inventory's declared CSV schema; a shard key alone is insufficient.

The same `EXTRA_OPTIONS {'content_format': 'txt'}` and SQL rename work in COPY
queries over this shard reader. The native shard reader selects PDF; DuckPort
configures `CrawlAccessor` schema evolution to select XML/TXT inside partition
workers, sharing one transformation plan per scan while retaining native
partitions, original URL fields and pushed projections.

`duckport_scan_pmc_inventory` continues to return inventory index rows, rather
than article metadata/content. Daily inventories are retained for 30 days
upstream, so archive a manifest and shard if you need to replay them later.
