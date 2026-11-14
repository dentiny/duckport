# duckport_scan_huggingface

```sql
LOAD httpfs;
CREATE SECRET hf_source (TYPE huggingface, TOKEN 'token', SCOPE 'hf://datasets/lhoestq/demo1');
SELECT * FROM duckport_scan_huggingface('lhoestq/demo1', config := 'default', split := 'train');
SELECT * FROM duckport_scan_huggingface('hf://datasets/lhoestq/demo1@~parquet/default/train/');
```

Reads a dataset’s Parquet export. Config/split default to `default`/`train`. Discovery pins the export to a commit and reads shards in path order. `revision` defaults to `refs/convert/parquet`; `allow_partial := true` permits incomplete exports. Authentication uses the DuckDB Hugging Face secret matching the dataset scope. `HF_ENDPOINT` selects a custom Hub endpoint.

Fully qualified paths select one split directory with
`hf://datasets/<owner>/<repo>@<revision>/<config>/<split>/`; the trailing slash is optional.
`~parquet` means `refs/convert/parquet`. Encode slashes in other revisions as `%2F`.
A fully qualified path cannot be combined with `revision`, `config`, or `split` options.
`allow_partial` and shared scan options still apply. File paths are not accepted by this dataset reader.

The matching secret takes precedence over HF environment/cached tokens; `HF_HUB_DISABLE_IMPLICIT_TOKEN=1` disables fallback.

See [shared settings](../../README.md#read) and [test instructions](../../test/README.md).
