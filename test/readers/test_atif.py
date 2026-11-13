from reader_test_support import ReaderTestCase, literal, COMMIT


class ReaderTests(ReaderTestCase):
    def test_read(self):
        c = self.context
        scan = c.scan("atif", c.atif)
        self.assertEqual(c.query(f"SELECT trajectory_id FROM {scan}"), [{"trajectory_id": "fixture"}])
