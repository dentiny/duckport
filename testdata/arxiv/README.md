# Arxiv fixtures

Synthetic source TAR shards and a bulk manifest exercise local and signed,
requester-pays loopback reads. The source TAR contains two entries totaling
25 payload bytes. `http.json` serves the same committed bytes through local
HTTP and S3-compatible endpoints.

Adapted from the MIT-licensed
[open-arrow-access v0.1.0 service tests](https://github.com/dentiny/open-arrow-access/tree/v0.1.0/service/tests)
and its arXiv fixture data. The TAR is a deterministic synthetic fixture, not
real publication content.

[arXiv reader guide](../../docs/readers/arxiv.md) ·
[HTTP route format](../../test/README.md#native-http-fixtures) ·
[Fixture inventory and provenance](../README.md) ·
[SQL reader tests](../../test/sql/readers/)
