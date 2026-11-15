# Tests

Run SQL tests from the repository root after building:

```sh
make test
```

Reader tests live in `test/sql/readers`; fixtures are grouped by format in `testdata/<format>/`.

`test/sql/readers/huggingface.test` reads the public `lhoestq/demo1` repository
and requires network access to Hugging Face Hub. It needs no token.

## Provider integration fixtures

Service SQL cases mirror the provider tests in open-arrow-access-service 0.2.0.
Internet Archive fixtures run with `make test`, as described below. The other
providers use Python wrappers with loopback servers and need no external accounts
or credentials:

```sh
export DUCKDB_CLI=/path/to/matching/duckdb
export DUCKPORT_EXTENSION="$PWD/build/release/extension/duckport/duckport.duckdb_extension"
python3 test/integration/test_zenodo.py
# Run the test_<provider>.py script for another integrated provider.
```

Internet Archive uses the native SQLLogicTest runner:

```sh
make test
# Or run only these tests after building:
build/release/test/unittest '*internet_archive*.test'
```

`internet_archive_http.test` requires `duckport` and the separate
`duckport_test_fixture` extension. With `BUILD_UNITTESTS=ON`, the fixture extension
is built by default (`DUCKPORT_BUILD_TEST_FIXTURES=ON`). It starts an in-process
HTTP server bound to loopback on an ephemeral port; SQL variables pass that
endpoint to the scans. The server serves the committed Internet Archive fixtures,
supports HEAD/range requests, and exposes counters to check that malformed
metadata causes no download. The test explicitly stops the server; process exit
also cleans it up. No Python wrapper, external service or account is needed.
Production builds can set `DUCKPORT_BUILD_TEST_FIXTURES=OFF`.

The other providers' HTTP SQL cases live in `test/integration/sql` because their
endpoint ports are assigned at runtime. They exercise actual extension loading, provider
resolution, original file bytes, format decoding, errors and prepared scans.
Fixtures are adapted from the MIT-licensed open-arrow-access service tests;
`http.json` maps provider routes to local response files. `ORIGIN` placeholders
refer to a separate download server so tests can assert credential isolation.
