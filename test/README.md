# Tests

Run SQL tests from the repository root after building:

```sh
make test
```

Reader tests live in `test/sql/readers`; fixtures are grouped by format in `testdata/<format>/`.

Common Crawl integration tests use the checked-in HTTP mirror. In one terminal:

```sh
python3 -m http.server 8000 --bind 127.0.0.1 --directory testdata/commoncrawl
```

Then run `DUCKPORT_COMMONCRAWL_ORIGIN=http://127.0.0.1:8000 make test`.
These tests are skipped when the variable is unset; other SQL tests still run.
