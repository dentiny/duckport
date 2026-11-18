# Tests and fixture maintenance

Assertions live in SQLLogicTest under `test/sql/readers/` and
`test/sql/writers/`. Run from the repository root with the pinned DuckDB build:

```sh
make test
make test T='test/sql/readers/arxiv_metadata_snapshot.test'
make test T='test/sql/readers/arxiv_snapshot_auth.test'
make test T='test/sql/writers/iceberg_roundtrip.test'
```

These commands may compile. If builds are disallowed, do not run them; use
formatting, local-document-link, ABI-layout and fixture-data checks and report
that runtime tests remain unexecuted.

## Build-time test dependencies

BUILD_UNITTESTS enables the independent readers used for COPY round trips:
DuckDB's pinned Iceberg extension, Parquet, and the Lance reader revision in
`extension_config.cmake`. `DUCKPORT_BUILD_TEST_FIXTURES` defaults to the unit-test
setting and adds `duckport_test_fixture` plus pinned httpfs. Production builds
can set `DUCKPORT_BUILD_TEST_FIXTURES=OFF` and omit those diagnostics/servers.

Lance tests restart the shell and load the independent reader to inspect rows,
nested/blob values and field metadata. Iceberg tests read published metadata with
`iceberg_scan`, checking creation, append, empty input, schema errors and catalog
publication. Test setup is native; no Python test runner or external catalog is
required. Hugging Face coverage that reads the public `lhoestq/demo1` export
requires network access; committed loopback cases do not.

## Native HTTP fixtures

SQL starts an in-process server on ephemeral loopback ports, configures scoped
secrets, queries a reader/writer, and inspects requests:

```sql
SET VARIABLE endpoint = duckport_fixture('arxiv', 'start_anonymous');
SELECT * FROM duckport_fixture_requests('arxiv');
SELECT duckport_fixture('arxiv', 'stop');
```

Provider fixture commands include `start`, `start_anonymous`,
`start_authenticated`, `stop`, `download_endpoint`, `scope`, `reset` and
`reject_metadata`. The Internet Archive compatibility function is
`duckport_ia_fixture(command)`; metadata/download request counts remain available.
The request table exposes `origin`, `target`, `authorization`, `api_key`,
`headers`, `request_payer`, `is_download`, `has_authorization` and
`has_request_payer`. Fixture credentials are synthetic test values only.

Routes are declared in `testdata/<provider>/http.json`. Route entries require
`path` and `file`; optional fields include `redirect`, `status`, `auth_header`,
`auth_value`, `auth_match` (`prefix` or exact), and `session_token`. Separate
`routes`/`downloads` collections model primary and auxiliary origins. Response
payloads may contain API_ORIGIN/ORIGIN placeholders expanded to loopback origins.
Successful status is left unset so httplib can implement range responses.

Add provider data/routes instead of new server branches. Assertions belong in
SQL, not in the server. Provider functions and request-table bindings live in
`provider_functions.cpp`; HTTP behavior is in `provider_fixture.cpp`. The shared
FixtureServer and ScopedFixtureEnvironment classes own cleanup. Workers are
destroyed before handler state and guarded traces/registries use DuckDB
annotated mutexes.

## arXiv snapshot regression coverage

`arxiv_metadata_snapshot.test` checks scalar/nested/list/date metadata retention,
PDF bytes, one-table COPY output, custom ID-column names, missing/duplicate/null
IDs and invalid snapshot selection. `arxiv_snapshot_auth.test` uses separate PDF
and snapshot buckets/keys/regions, verifies requester-pays is absent on snapshot
requests, and checks an independently authenticated HTTP snapshot with local PDFs.
`arxiv.test` also covers explicit auto discovery while retaining the source-only
default. These tests exercise the shared upstream index, not a second ID matcher.

## Rust diagnostics and file helpers

The fixture-only `test-fixtures` Cargo feature exports read-only Lance Blob-layout
and field-metadata diagnostics. C++ owns/free returned strings through one shared
adapter. `duckport_fixture_write` and `duckport_fixture_remove` manage temporary
inputs in SQL tests. The Iceberg fixture serves a small unpartitioned people
schema with controlled create/append/authentication behavior.

Committed payload provenance and regeneration instructions are in the
[fixture inventory](../testdata/README.md). Keep test payloads small, deterministic
where possible, and free of real credentials. See [development](../docs/UPDATING.md)
for formatting and ABI conventions.
