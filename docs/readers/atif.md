# ATIF agent trajectories

```sql
SELECT * FROM duckport_scan_atif('testdata/atif/trajectory.json');
SELECT * FROM duckport_scan_atif('runs/trajectories.jsonl.gz', batch_size := 4096);
COPY (SELECT * FROM duckport_scan_atif('runs/trajectory.json'))
TO 'trajectories.lance' (FORMAT duckport_lance);
```

The upstream ATIF reader uses a fixed nested trajectory schema and frames JSON
objects from their contents. Pretty-printed JSON and JSONL do not require a
filename-based layout option. `raw_json`, text bytes, content order and media
attributes are retained; embedded subagent trajectories remain raw JSON.

## SELECT resolves references

Image/audio `source.path` fields in messages and observation results become
absolute URI strings relative to the trajectory document. Resolution uses URI
rules, including file references: encode reserved characters, for example
`image%20%231%25.png` for `image #1%.png`. `data:` URIs are retained. Remote
references must stay on the document's canonical storage origin. Opaque non-data
URIs and embedded credentials fail normalization; the backend controls transport
availability.

SELECT does not fetch/decode media. Missing media files and malformed data-URI
payloads can therefore remain visible as references without failing the scan.
Projection and COUNT(*) retain the normal reader behavior.

## Lance COPY materializes tagged media

Lance COPY recognizes tagged media fields and downloads them through
open-arrow-sink's `DownloadBlobLayer`; explicit `BLOB_COLUMNS` is unnecessary for
those tags. The native materializer writes managed Blob v2 storage using its
layout thresholds. Missing media and invalid data-URI payloads fail COPY.
Tagged text defaults to LargeBinary/LZ4 unless an explicit compression overrides
it. Untagged fields keep their ordinary policy.

Source/storage metadata survives direct projections, aliases and stored DuckDB
tables. Credentials are never written to field metadata: COPY looks up current
source secrets again. Remote trajectory documents preserve the source store's
configuration; absolute remote URLs from local documents use the sink's normal
source resolution. See [source secrets](../secrets.md).

Parquet and Iceberg sinks preserve media reference values and do not download
ATIF media. Iceberg additionally aligns to its catalog schema. This difference
also means SELECT and Lance COPY can fail at different times.
