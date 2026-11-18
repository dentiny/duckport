# File lists, directories and globs

Parquet, CSV/TSV, JSON/JSONL/NDJSON, XML and WARC readers accept a path or
`VARCHAR[]` file list:

```sql
SELECT * FROM duckport_scan_parquet(['part-a.parquet', 'part-b.parquet']);
SELECT * FROM duckport_scan_csv('data/', header := true);
SELECT * FROM duckport_scan_json('s3://bucket/data/**/*.jsonl');
```

Explicit lists keep their selection order and duplicates; use SQL `ORDER BY` when
result order matters. Each element can also be a directory or glob. Directory discovery is recursive and selects the reader's
file extensions, including supported outer compression for text formats. Explicit
globs select the files matching the pattern regardless of extension: `*` matches
one path component and `**` traverses directories. Discovery results are sorted
lexicographically and deduplicated within each selection. Empty lists, unmatched
selections and NULL elements fail.

Local directories may omit the trailing slash. S3 directory prefixes must end in
`/`; S3 dot path segments are rejected to preserve credential scopes.
Explicit lists can mix local, HTTP(S) and S3 files; each file resolves its own
scoped DuckDB secret. Directory/glob listing uses the secret matched to its actual
discovery root, independently of file-prefix secrets. HTTP directory/glob listing
is unsupported. URI query strings remain unsupported for ordinary file readers.

All files must have exactly matching Arrow schemas, including nullability and
metadata. Complete schemas are checked before emitting rows, even when selecting
fewer columns or using `COUNT(*)`. `union_by_name`, source-file columns, ATIF and provider-specific
multi-file inputs are not exposed here. Existing reader options apply to every
selected file, including headers, delimiters, XML `record_path` and batch size.

File discovery happens during binding, so prepared statements retain the selected
file list; it is not a filesystem snapshot. Record decoding remains sequential
and files are opened as needed after schema preflight. Multi-file encoded-byte
metrics are accumulated, but progress is indeterminate until EOF. These readers
can be used as ordinary COPY sources with the existing sinks.
