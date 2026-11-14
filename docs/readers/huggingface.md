# duckport_scan_huggingface

```sql
SELECT * FROM duckport_scan_huggingface('lhoestq/demo1', config := 'default', split := 'train');
```

Reads a dataset’s Parquet export. Config/split default to `default`/`train`. Discovery pins the export to a commit and reads shards in path order. `revision` defaults to `refs/convert/parquet`; `allow_partial := true` permits incomplete exports. Authentication uses the matching DuckDB Hugging Face secret. `HF_ENDPOINT` selects a custom Hub endpoint.

HF environment/cached tokens are supported; `HF_HUB_DISABLE_IMPLICIT_TOKEN=1` disables implicit token loading.

See [shared settings](../../README.md#read) and [test instructions](../../test/README.md).

```sql
LOAD httpfs;
CREATE SECRET hf_source (TYPE huggingface, TOKEN 'token', SCOPE 'hf://datasets/owner/repository');
```
