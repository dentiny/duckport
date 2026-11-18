# Arxiv fixtures

Synthetic source and PDF TAR shards, bulk manifests and a cached bibliographic Parquet snapshot. Local and signed/requester-pays loopback scans share the same payloads.

Adapted from the MIT-licensed [open-arrow-access v0.1.0 service tests](https://github.com/dentiny/open-arrow-access/tree/v0.1.0/service/tests) and its `arxiv` fixture data. Local payloads are reduced/synthetic adaptations.

The existing source TAR contains two entries totaling 25 payload bytes. The PDF
TAR contains two synthetic PDF-like payloads totaling 21 bytes, with a versioned
and an unversioned filename. `metadata.parquet` has two rows with string IDs,
titles, author lists, nested attributes and dates. PDF IDs match one canonical
and one OAI-prefixed identifier. These bytes are fixtures, not real papers.

Regenerate PDF/metadata payloads with `python3 testdata/arxiv/generate.py` from
the repository root. It invokes an already installed DuckDB CLI for Parquet;
it does not compile the extension. HTTP routes include independent snapshot
S3 keys/session tokens/region checks and a separate HTTP bearer-authenticated
snapshot. See [arXiv behavior](../../docs/readers/arxiv.md).

[http.json](http.json) defines response routes and synthetic authentication. Route syntax and loopback commands are documented in the [test guide](../../test/README.md#native-http-fixtures).

[Fixture inventory and provenance](../README.md) · [SQL reader tests](../../test/sql/readers/)
