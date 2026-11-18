# Hugging Face Parquet datasets

```sql
SELECT * FROM duckport_scan_huggingface('lhoestq/demo1',
    config := 'default', split := 'train');
SELECT * FROM duckport_scan_huggingface(
    'hf://datasets/lhoestq/demo1@~parquet/default/train/');
```

Reads one dataset config/split's Parquet export through the upstream Hub accessor.
Defaults are `config := 'default'`, `split := 'train'` and revision
`refs/convert/parquet`. Discovery pins an export commit and reads shards in path
order. `allow_partial := true` permits an incomplete published export.

A fully qualified locator is
`hf://datasets/<owner>/<repo>@<revision>/<config>/<split>/`; its trailing slash is
optional. `~parquet` means `refs/convert/parquet`; encode slashes in other
revisions as `%2F`. Such a locator cannot be combined with `revision`, `config`
or `split` options. It selects a split directory, not an individual repository
file. `allow_partial` and `batch_size` still apply.

Configure a DuckDB `TYPE huggingface` secret scoped to the logical dataset URI.
An explicit token takes precedence over environment/cache credentials;
`HF_HUB_DISABLE_IMPLICIT_TOKEN=1` disables implicit fallback and `HF_ENDPOINT`
selects a custom Hub endpoint. See [the secret guide](../secrets.md#hugging-face).

DuckPort does not expose the reference extension's SQL read-ahead/concurrency
knobs. Arbitrary individual Hub files can instead be ingested through Lance's
[URL blob support](../writers/lance.md#url-blob-columns).
