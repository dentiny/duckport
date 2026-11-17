# ATIF fixtures

Copied from open-arrow-access's MIT-licensed ATIF fixtures. `trajectory.json` contains one trajectory; `trajectories.jsonl` contains three, with blank lines and CRLF. They include Unicode messages and tool calls. The bad inputs contain a truncated second record or unsupported schema version.

`trajectory_media.json` and the generated PNG/WAV are copied from
`dentiny/duckdb_lance_conversion`'s MIT-licensed media fixtures. They exercise
message and observation image/audio content, Unicode and storage metadata.
`trajectory_media.jsonl.gz` contains two copies compressed with gzip.

`http.json` serves authenticated trajectories/media through the native loopback
fixture. `bad_*_media.json` cover forbidden local-file, cross-origin and
cross-scheme references in remote trajectories.

`parent_media.json` checks URL-relative parent-directory resolution without fetching media.
