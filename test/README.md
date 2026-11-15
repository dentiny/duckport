# Tests

Run all SQLLogicTests from the repository root after building:

```sh
make test
```

All test definitions and assertions live in `test/sql/readers`; committed input
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
