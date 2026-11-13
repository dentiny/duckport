from reader_test_support import ReaderTestCase, literal, COMMIT


class ReaderTests(ReaderTestCase):
    def test_read(self):
        c = self.context
        scan = c.scan("huggingface", "owner/example", config="default", split="train",
                      endpoint=c.endpoint, token="fixture-only", batch_size=2)
        self.assertEqual(c.query(f"SELECT * FROM {scan}"), c.rows + c.rows)
        self.assertTrue(any("/revision/" in path for _, path in c.requests))
        self.assertTrue(any("/tree/" in path for _, path in c.requests))
        self.assertTrue(any("/resolve/" in path for _, path in c.requests))
