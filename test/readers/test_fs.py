from reader_test_support import ReaderTestCase, literal, COMMIT


class ReaderTests(ReaderTestCase):
    def test_read(self):
        c = self.context
        for path in (c.parquet, c.parquet.as_uri()):
            scan = c.scan("fs", path, format="parquet")
            self.assertEqual(c.query(f"SELECT * FROM {scan}"), c.rows)
        c.query(f"SELECT * FROM {c.scan('fs', c.parquet)}", error="requires format")
