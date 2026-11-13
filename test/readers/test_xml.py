from reader_test_support import ReaderTestCase, literal, COMMIT


class ReaderTests(ReaderTestCase):
    def test_read(self):
        c = self.context
        scan = c.scan("xml", c.xml, record_path="/records/record")
        self.assertEqual(c.query(f"SELECT count(*) AS n FROM {scan}"), [{"n": 2}])
        c.query(f"SELECT * FROM {c.scan('xml', c.xml)}", error="requires record_path")
