from reader_test_support import ReaderTestCase, literal, COMMIT


class ReaderTests(ReaderTestCase):
    def test_read(self):
        c = self.context
        settings = {"endpoint": c.endpoint, "region": "us-east-1", "allow_anonymous": "true",
                    "disable_config_load": "true", "disable_ec2_metadata": "true"}
        scan = c.scan("s3", "s3://bucket/people.parquet", format="parquet", storage_options=settings)
        self.assertEqual(c.query(f"SELECT * FROM {scan}"), c.rows)
        c.query(f"SELECT * FROM {c.scan('s3', c.parquet, format='parquet')}", error="different storage backend")
