from reader_test_support import ReaderTestCase, literal, COMMIT


class ReaderTests(ReaderTestCase):
    def test_read(self):
        c = self.context
        for path in (c.json, c.gzip):
            scan = c.scan("jsonl", path, batch_size=1)
            self.assertEqual(c.query(f"SELECT id, nested.ok AS ok, len(tags) AS n FROM {scan}"),
                             [{"id": 1, "ok": True, "n": 2}, {"id": 2, "ok": False, "n": 0}])
        corrupt = c.root / "corrupt-jsonl.jsonl"
        corrupt.write_text('{"id":1}\n' * 1100 + '{not-json}\n')
        c.query(f"SELECT sum(id) FROM {c.scan('jsonl', corrupt, batch_size=32)}", error="get_next")
