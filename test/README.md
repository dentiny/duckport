# Tests

Run all SQLLogicTests from the repository root after building:

```sh
make test
```

Reader assertions live in `test/sql/readers`; COPY framework assertions live in `test/sql/writers`. Committed input
fixtures are grouped under `testdata/<provider-or-format>/`.

The HTTP tests use the same SQLLogicTest runner and a separate
`duckport_test_fixture` extension. It is built by default when `BUILD_UNITTESTS`
is enabled. SQL starts in-process loopback servers on ephemeral ports, configures
secrets, reads provider data, and inspects captured requests through
`duckport_fixture_requests`. The fixture server serves committed files and
supports HEAD, byte ranges and redirects. Authentication, credential isolation,
prepared scans and malformed-metadata checks are expressed in SQL.

Run one provider's HTTP coverage with:

```sh
make test T='test/sql/readers/zenodo_http.test'
```

The fixture extension is excluded from production builds with
`DUCKPORT_BUILD_TEST_FIXTURES=OFF`. No external accounts are needed for loopback
tests. The Hugging Face test reads the public `lhoestq/demo1` repository and
requires network access.

The arXiv S3 SQL tests require `httpfs`. The test-fixture build loads the
version pinned by the DuckDB checkout alongside the native fixture extension.

The Lance writer SQL tests build the upstream
[`lance` extension](https://github.com/lance-format/lance-duckdb) at the revision
pinned in `extension_config.cmake`.
They write with DuckPort, restart without automatically loading extensions, and
load the independent reader to verify committed rows, write modes, nested/NULL
values, native BLOBs, and downloaded URL bytes. The reader is included only when
`BUILD_UNITTESTS` is enabled; it is not a production dependency.

```sh
make test T='test/sql/writers/lance.test'
```
