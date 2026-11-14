# duckport_scan_huggingface_file

```sql
SELECT * FROM duckport_scan_huggingface_file('owner/repository', path := 'data/train.parquet', format := 'parquet', revision := 'main');
```

Reads one repository file. `path` and `format` are required; supported formats are Parquet, CSV/TSV, JSON/JSONL/NDJSON, XML, ATIF/ATIF JSONL, and WARC. `revision` defaults to `main`. Optional `endpoint` and `token` override connection settings.

HF environment/cached tokens are supported; `HF_HUB_DISABLE_IMPLICIT_TOKEN=1` disables implicit token loading.

For CSV/TSV, pass `header`; for XML, pass `record_path`.

See [shared settings](../../README.md#read) and [test instructions](../../test/README.md).
