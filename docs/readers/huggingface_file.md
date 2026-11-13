# duckport_read_huggingface_file

```sql
SELECT * FROM duckport_read_huggingface_file('owner/repository', path := 'data/train.parquet', format := 'parquet', revision := 'main');
```

Reads one repository file. `path` and `format` are required; the format must be included in this checkout. `revision` defaults to `main`. Optional `endpoint` and `token` override connection settings.

HF environment/cached tokens are supported; `HF_HUB_DISABLE_IMPLICIT_TOKEN=1` disables implicit token loading.

For CSV/TSV, pass `header`; for XML, pass `record_path`. The corresponding format PR must be merged first.

See [shared settings](../../README.md#shared-settings) and [test instructions](../../test/README.md).
