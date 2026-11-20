# Semantic Scholar fixtures

Release discovery, dataset metadata and synthetic S2ORC v2 gzip JSONL shards. `s2orc_v2.jsonl.gz` contains a row with body text, string-encoded
annotation spans and bibliography text, followed by a row with only `corpusid`.
`s2orc_v2_invalid.jsonl.gz` contains a reversed annotation span to exercise
validation even when fields are projected out. Both gzip files use `mtime=0` and
are served through suffixless logical part names, matching release manifests.

Adapted from the MIT-licensed [open-arrow-access v0.1.0 service tests](https://github.com/dentiny/open-arrow-access/tree/v0.1.0/service/tests) and its `semantic-scholar` fixture data. Local payloads are reduced/synthetic adaptations.

[http.json](http.json) defines response routes and synthetic authentication. Route syntax and loopback commands are documented in the [test guide](../../test/README.md#native-http-fixtures).

[Fixture inventory and provenance](../README.md) · [SQL reader tests](../../test/sql/readers/)
