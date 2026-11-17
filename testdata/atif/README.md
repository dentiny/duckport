# ATIF fixtures

Copied from open-arrow-access's MIT-licensed ATIF fixtures. `trajectory.json` contains one trajectory; `trajectories.jsonl` contains three, with blank lines and CRLF. They include Unicode messages and tool calls. The bad inputs contain a truncated second record or unsupported schema version.

`trajectory_media.json` and the generated PNG/WAV are copied from
`dentiny/duckdb_lance_conversion`'s MIT-licensed media fixtures. They exercise
message and observation image/audio content, Unicode and storage metadata.


`http.json` serves authenticated trajectories/media through the native loopback
fixture. `bad_origin_media.json` covers cross-origin references in remote trajectories.
