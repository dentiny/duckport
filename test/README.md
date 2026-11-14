# Tests

Run SQL tests from the repository root after building:

```sh
make test
```

Reader tests live in `test/sql/readers`; fixtures are grouped by format in `testdata/<format>/`.

`test/sql/readers/huggingface.test` reads the public `lhoestq/demo1` repository
and requires network access to Hugging Face Hub. It needs no token.

## Provider integration fixtures

Service SQL cases mirror the provider tests in open-arrow-access-service 0.1.0.
Local fixtures run with `make test`. HTTP cases use loopback fixture servers and
need no external accounts or credentials:

```sh
export DUCKDB_CLI=/path/to/matching/duckdb
export DUCKPORT_EXTENSION="$PWD/build/release/extension/duckport/duckport.duckdb_extension"
python3 test/integration/test_zenodo.py
# Run the test_<provider>.py script for another integrated provider.
```

The HTTP SQL cases live in `test/integration/sql` because their endpoint ports
are assigned at runtime. They exercise actual extension loading, provider
resolution, original file bytes, format decoding, errors and prepared scans.
Fixtures are adapted from the MIT-licensed open-arrow-access service tests;
`http.json` maps provider routes to local response files. `ORIGIN` placeholders
refer to a separate download server so tests can assert credential isolation.
