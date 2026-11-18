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

Lance writer tests build the upstream [Lance reader](https://github.com/lance-format/lance-duckdb)
at the revision pinned in `extension_config.cmake`, only with `BUILD_UNITTESTS`.
They restart and load that reader to check written rows and downloaded blob bytes.

The Iceberg COPY round-trip SQLLogicTest uses the native loopback REST catalog
fixture. DuckDB's independent Iceberg extension reads the committed metadata
with `iceberg_scan`; the test checks COPY, APPEND, NULLs, Unicode, empty strings,
empty input, invalid options and schema mismatch.
No external catalog or Python test runner is needed.

The Iceberg reader is pinned by the DuckDB checkout and built only with
`BUILD_UNITTESTS`. Its `roaring` dependency is provided by the vcpkg manifest.
After building, run:

```sh
make test T='test/sql/writers/iceberg_roundtrip.test'
```

Extension assertions live in SQLLogicTests rather than Rust unit tests.
The fixture-only Rust `test-fixtures` feature exposes read-only native Lance
Blob descriptor/field-metadata diagnostics to `duckport_test_fixture`, allowing
SQL to check physical layouts. File helpers create/remove temporary inputs.
Production builds with `DUCKPORT_BUILD_TEST_FIXTURES=OFF` exclude these diagnostics.
