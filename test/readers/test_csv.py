from reader_test_support import ReaderTestCase, literal, COMMIT


class ReaderTests(ReaderTestCase):
    def test_read(self):
        c = self.context
        scan = c.scan("csv", c.csv, header=True, batch_size=2)
        self.assertEqual(c.query(f"SELECT * FROM {scan}"), c.rows)
        c.query(f"SELECT * FROM duckport_read_csv({literal(c.csv)}, header := NULL)", error="NULL")
