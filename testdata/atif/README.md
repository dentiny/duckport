# Atif fixtures

Single and multiline trajectories, JSONL framing, invalid input, Unicode/tool messages and image/audio media references.

Adapted from the MIT-licensed open-arrow-access fixtures.

`trajectory.json` has one trajectory; `trajectories.jsonl` has three and covers blank lines/CRLF. Media JSON and generated PNG/WAV payloads originate from MIT-licensed duckdb_lance_conversion fixtures. SELECT preserves URI references; Lance COPY tests materialize them.

[http.json](http.json) defines response routes and synthetic authentication. Route syntax and loopback commands are documented in the [test guide](../../test/README.md#native-http-fixtures).

[Fixture inventory and provenance](../README.md) · [SQL reader tests](../../test/sql/readers/)
