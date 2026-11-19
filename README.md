# DuckPort

Read files and dataset services with DuckDB SQL, then stream any query result to
Lance, a Parquet dataset directory, or an Iceberg table.

```sql
COPY (
    SELECT id, image_url AS image
    FROM read_parquet('catalog/*.parquet')
    WHERE image_url IS NOT NULL
) TO 'images.lance' (FORMAT duckport_lance, BLOB_COLUMNS ('image'));
```

DuckDB runs the query: its readers, tables, views, filters, joins, casts and
expressions remain available. DuckPort adapts the result to Arrow and the selected
sink without an intermediate export. A query may still buffer data for its own
operators, sampling or backend work.

## Start here

Once the community submission is merged and binaries are published for your
DuckDB version and platform, install and load DuckPort:

```sql
FORCE INSTALL duckport FROM community;
LOAD duckport;
```

[Reader catalog](docs/readers/README.md) · [COPY reference](docs/writers/README.md) ·
[Secrets](docs/secrets.md) · [Settings and profiling](docs/storage.md) ·
[Documentation index](docs/README.md)

## Supported sources

| Source family | Readers / selection |
| --- | --- |
| Files | CSV/TSV, JSON/JSONL/NDJSON, Parquet, XML, WARC; same-schema files, lists, directories and globs |
| Datasets and archives | Hugging Face Parquet exports, Common Crawl releases/segments, ATIF trajectories, arXiv bulk TAR shards, MECA packages |
| Metadata | arXiv/DOAB/OAPEN OAI-PMH, Gutenberg RDF, PMC article metadata, PMC inventory and article-shard readers, CourtListener dated exports |
| Selected attachments | Zenodo, Dataverse, Figshare, Internet Archive, Federal Register, GovInfo, HathiFiles, Library of Congress, Semantic Scholar, OpenStax, BHL, Regulations.gov, USPTO, bioRxiv/medRxiv |

The [reader catalog](docs/readers/README.md) lists the exact `duckport_scan_*`
functions, required selectors and links to examples. These are input readers;
Lance, Parquet and Iceberg output capabilities are listed [below](#write-query-results).

## Read with SQL

```sql
SELECT * FROM duckport_scan_csv('people.csv', header := true);
SELECT * FROM duckport_scan_json('events.jsonl');
SELECT * FROM duckport_scan_parquet('people.parquet');
SELECT * FROM duckport_scan_huggingface('lhoestq/demo1', split := 'train');
SELECT * FROM duckport_scan_arxiv_metadata('records',
    from_date := '2024-10-01', until_date := '2024-10-31', max_records := 1000);
```

Readers include CSV/TSV, JSON/JSONL/NDJSON, Parquet, XML, WARC, Common Crawl,
ATIF trajectories, arXiv bulk archives and OAI-PMH, PMC, Gutenberg, MECA,
CourtListener, and selected attachments from research and public-data services.
The [reader catalog](docs/readers/README.md) documents every SQL function and its
selection rules. Provider readers do not imply unrestricted discovery or a full
dataset download.

All readers accept `batch_size` (default 8192, range 1–1,048,576). Local paths,
`file://`, HTTP(S) and S3 are supported by the applicable reader. Standard file readers accept [file lists, directories and globs](docs/readers/files.md)
with matching schemas. DuckPort pushes top-level projections into its readers; filters execute in DuckDB.

## Write query results

```sql
CREATE TABLE events AS SELECT 1 AS id, 'hello' AS body;

COPY events TO 'events.lance' (FORMAT duckport_lance);
COPY events TO 'events.lance' (FORMAT duckport_lance, APPEND);
COPY events TO 'events_dataset' (FORMAT duckport_parquet);
SELECT * FROM read_parquet('events_dataset/*.parquet');
```

| Destination | COPY format | Modes | Additional behavior |
| --- | --- | --- | --- |
| [Lance](docs/writers/lance.md) | `duckport_lance` | Create, APPEND, OVERWRITE | Nested URL blob ingestion, automatic tagged ATIF media, indexes, column encodings |
| [Parquet directory](docs/writers/parquet.md) | `duckport_parquet` | Create, APPEND | Multiple data files and top-level column compression |
| [Iceberg REST catalog](docs/writers/iceberg.md) | `duckport_iceberg` | Create, APPEND | Schema-derived table creation and snapshot publication |

All three accept sampling, generated columns, `TARGET_FILE_SIZE`, `READ_CONCURRENCY`,
`WRITE_CONCURRENCY` and `PRESERVE_ORDER`. Writes default to one writer and a
512 MiB soft file-size target, rolling to a new file as needed. Use
`WRITE_CONCURRENCY 'auto'` to opt into automatic writer planning. Read concurrency
defaults to `'auto'`. Backend-specific options are validated against the sink's
capabilities. Use native `FORMAT PARQUET` to write a single file.
See [all COPY options](docs/writers/README.md#all-copy-options) for accepted
values, defaults and supported sinks, including blob policies, compression,
indexes, Iceberg catalog options and `EXTRA_OPTIONS` for source arguments.

## Try a complete local round trip

After loading DuckPort, this example creates its own input without downloads or
credentials. Run it in a writable directory with unused output paths:

```sql
COPY (SELECT 1 AS id, 'hello' AS body UNION ALL SELECT 2, 'world')
TO 'duckport_demo.csv' (FORMAT CSV, HEADER);

SELECT * FROM duckport_scan_csv('duckport_demo.csv', header := true);

COPY (
    SELECT id, upper(body) AS body
    FROM duckport_scan_csv('duckport_demo.csv', header := true)
    WHERE id > 0
) TO 'duckport_demo_dataset' (FORMAT duckport_parquet);

SELECT * FROM read_parquet('duckport_demo_dataset/*.parquet') ORDER BY id;
-- id | body
--  1 | HELLO
--  2 | WORLD
```

To write the same result to Lance, use a fresh `.lance` destination with
`FORMAT duckport_lance`. Iceberg additionally requires a REST catalog, an existing
namespace and storage configuration; follow the [Iceberg guide](docs/writers/iceberg.md).

## Configure secrets once

DuckPort resolves DuckDB secrets by URI scope. Scan/COPY options contain data
selection, not tokens or access keys. Load `httpfs` for its S3 and Hugging Face
secret types:

```sql
INSTALL httpfs;
LOAD httpfs;
CREATE SECRET source_store (
    TYPE s3, KEY_ID 'access-key', SECRET 'secret-key', REGION 'us-east-1',
    SCOPE 's3://source-bucket/'
);
CREATE SECRET media_store (
    TYPE http, BEARER_TOKEN 'access-token',
    SCOPE 'https://media.example.com/private/'
);
```

Use separate scopes for input storage, media URLs, output storage and an Iceberg
catalog. The [secret guide](docs/secrets.md) includes S3-compatible endpoints,
Hugging Face, provider API keys, redirect rules, and arXiv's independent PDF and
metadata-snapshot credentials.

## How this differs from duckdb_lance_conversion

Both extensions stream DuckDB query results to Lance and support APPEND,
OVERWRITE, sampling, generated columns, Blob v2 policies and indexes. DuckPort
adds Parquet-directory and Iceberg sinks, a broader provider catalog, nested
`BLOB_COLUMNS`, and per-source scoped credentials. Its arXiv reader can join PDFs
to a persisted metadata snapshot in one output table.

The interfaces are different: use `duckport_scan_*`, `FORMAT duckport_lance`
and `conversion_storage_*`. DuckPort does not expose every read-parallelism or
memory-budget control of the reference extension. See the
[feature and migration comparison](docs/comparison.md) for the exact differences.

## Built on two upstream projects

- [open-arrow-access](https://github.com/dentiny/open-arrow-access) provides
  Arrow readers, service accessors, attachment decoding and prepared scans.
- [open-arrow-sink](https://github.com/dentiny/open-arrow-sink) provides writer
  sessions, Lance/Parquet/Iceberg backends and reusable blob, schema and concurrency
  layers. DuckPort uses its `core`, `layer`, `storage` and backend crates.

The current manifest selects open-arrow-access 0.9.0 and open-arrow-sink 0.5.1
with Arrow 58. These projects own format/service and storage mechanics; DuckPort
owns SQL binding, secret lookup and Arrow C ABI adaptation. Read the
[architecture guide](docs/architecture.md) before extending the bridge.

[MIT license](LICENSE)
