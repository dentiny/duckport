# Tests

Run SQL tests from the repository root after building:

```sh
make test
```

Reader tests live in `test/sql/readers`; fixtures are grouped by format in `testdata/<format>/`.

`test/sql/readers/huggingface.test` reads the public `lhoestq/demo1` repository
and requires network access to Hugging Face Hub. It needs no token.
