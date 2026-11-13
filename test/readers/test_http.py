from reader_test_support import ReaderTestCase, literal, COMMIT


class ReaderTests(ReaderTestCase):
    def test_read(self):
        c = self.context
        scan = c.scan("http", c.endpoint + "/people.parquet", format="parquet")
        self.assertEqual(c.query(f"SELECT * FROM {scan}"), c.rows)
        c.query(f"SELECT * FROM {c.scan('http', c.parquet, format='parquet')}", error="different storage backend")
