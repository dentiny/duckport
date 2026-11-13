from reader_test_support import ReaderTestCase, literal, COMMIT


class ReaderTests(ReaderTestCase):
    def test_read(self):
        c = self.context
        scan = c.scan("huggingface_file", "owner/example", path="default/train/0000.parquet",
                      format="parquet", revision=COMMIT, endpoint=c.endpoint, token="fixture-only")
        self.assertEqual(c.query(f"SELECT * FROM {scan}"), c.rows)
        c.query("SELECT * FROM duckport_read_huggingface_file('owner/example')", error="requires path")
