# duckport_scan_huggingface_file

```sql
LOAD httpfs;
CREATE SECRET hf_source (TYPE huggingface, TOKEN 'token', SCOPE 'hf://datasets/owner/repository/');
SELECT * FROM duckport_scan_huggingface_file('owner/repository', path := 'data/train.parquet', format := 'parquet', revision := 'main');
```

Reads one repository file. `path` and `format` are required; supported formats are Parquet, CSV/TSV, JSON/JSONL/NDJSON, XML, ATIF/ATIF JSONL, and WARC. `revision` defaults to `main`. Authentication uses the matching DuckDB Hugging Face secret. `HF_ENDPOINT` selects a custom Hub endpoint.

The matching secret takes precedence over HF environment/cached tokens; `HF_HUB_DISABLE_IMPLICIT_TOKEN=1` disables fallback.

For CSV/TSV, pass `header`; for XML, pass `record_path`.

See [shared settings](../../README.md#read) and [test instructions](../../test/README.md).
