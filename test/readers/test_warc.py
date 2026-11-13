from reader_test_support import ReaderTestCase, literal, COMMIT


class ReaderTests(ReaderTestCase):
    def test_read(self):
        c = self.context
        scan = c.scan("warc", c.warc)
        self.assertEqual(c.query(f"SELECT record_type, decode(body) AS body FROM {scan}"),
                         [{"record_type": "metadata", "body": "test"}])
