# duckport_read_huggingface

```sql
SELECT * FROM duckport_read_huggingface('lhoestq/demo1', config := 'default', split := 'train');
```

Reads a dataset’s Parquet export. Config/split default to `default`/`train`. Discovery pins the export to a commit and reads shards in path order. `revision` defaults to `refs/convert/parquet`; `allow_partial := true` permits incomplete exports. Optional `endpoint` and `token` override connection settings.

HF environment/cached tokens are supported; `HF_HUB_DISABLE_IMPLICIT_TOKEN=1` disables implicit token loading.

See [shared settings](../../README.md#shared-settings) and [test instructions](../../test/README.md).
