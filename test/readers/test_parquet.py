from reader_test_support import ReaderTestCase, literal, COMMIT


class ReaderTests(ReaderTestCase):
    def test_read(self):
        c = self.context
        scan = c.scan("parquet", c.parquet, batch_size=2)
        self.assertEqual(c.query(f"SELECT * FROM {scan}"), c.rows)
        self.assertEqual(c.query(f"SELECT count(*) AS n FROM {scan}"), [{"n": 3}])
        self.assertEqual(c.query(f"SELECT name FROM {scan} WHERE id=3"), [{"name": "Charlie"}])
        self.assertEqual(c.query(f"SELECT * FROM {scan} LIMIT 1"), c.rows[:1])
        repeat = c.execute(f"PREPARE r AS SELECT count(*) AS n FROM {scan}; EXECUTE r; EXECUTE r;")
        self.assertEqual(repeat.returncode, 0, repeat.stderr)
        self.assertEqual(repeat.stdout.count('"n":3'), 2)
        c.query(f"SELECT * FROM {c.scan('parquet', c.parquet, batch_size=0)}", error="batch_size")
        c.query(f"SET enable_external_access=false; SELECT * FROM {scan}", error="enable_external_access")
        c.query("SELECT * FROM duckport_read_parquet(NULL)", error="NULL")

    def test_schema_change_and_export(self):
        c = self.context
        changing = c.root / "changing.parquet"
        changing.write_bytes(c.parquet.read_bytes())
        scan = c.scan("parquet", changing)
        c.query(f"PREPARE changed AS SELECT * FROM {scan}; EXECUTE changed; "
                f"COPY (SELECT 'new-type' AS id) TO {literal(changing)} (FORMAT PARQUET); EXECUTE changed;",
                error="schema changed")
        output = c.root / "output.parquet"
        c.query(f"COPY (SELECT * FROM {c.scan('parquet', c.parquet)}) TO {literal(output)} (FORMAT PARQUET)")
        self.assertEqual(c.query(f"SELECT * FROM {c.scan('parquet', output)}"), c.rows)
