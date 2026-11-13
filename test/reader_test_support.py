"""Shared CLI and local-server fixtures for independently registered readers."""
import gzip
import http.server
import json
import os
from pathlib import Path
import subprocess
import tempfile
import threading
import unittest
from urllib.parse import unquote, urlparse

COMMIT = "a" * 40
context = None


def literal(value):
    return "'" + str(value).replace("'", "''") + "'"


class ReaderTestCase(unittest.TestCase):
    def setUp(self):
        self.context = context


class ReaderContext:
    def __init__(self, extension):
        self.extension = str(Path(extension).resolve())
        self.cli = os.environ.get("DUCKDB_CLI", "duckdb")
        self.env = {**os.environ, "HF_HUB_DISABLE_IMPLICIT_TOKEN": "1"}
        self.temp = tempfile.TemporaryDirectory(prefix="duckport-readers-")
        self.root = Path(self.temp.name)
        self.server = None
        self.requests = []
        self.rows = [{"id": 1, "name": "Alice"}, {"id": 2, "name": None}, {"id": 3, "name": "Charlie"}]
        self.csv = self.root / "people.csv"
        self.csv.write_text("id,name\n1,Alice\n2,\n3,Charlie\n")
        self.tsv = self.root / "people.tsv"
        self.tsv.write_text(self.csv.read_text().replace(",", "\t"))
        self.json = self.root / "events.jsonl"
        self.json.write_text('{"id":1,"nested":{"ok":true},"tags":["a","b"]}\n{"id":2,"nested":{"ok":false},"tags":[]}\n')
        self.gzip = self.root / "events.jsonl.gz"
        self.gzip.write_bytes(gzip.compress(self.json.read_bytes()))
        self.xml = self.root / "records.xml"
        self.xml.write_text('<records><record><id>1</id></record><record><id>2</id></record></records>')
        self.atif = self.root / "trajectory.json"
        self.atif.write_text(json.dumps({"schema_version": "ATIF-v1.8", "trajectory_id": "fixture", "agent": {"name": "test", "version": "1"}, "steps": [{"step_id": 1, "source": "user", "message": "hello"}]}) + "\n")
        self.warc = self.root / "records.warc"
        self.warc.write_bytes(b"WARC/1.1\r\nWARC-Type: metadata\r\nWARC-Record-ID: <urn:test:one>\r\nWARC-Date: 2020-07-08T02:52:55Z\r\nContent-Length: 4\r\n\r\ntest\r\n\r\n")
        self.parquet = self.root / "people.parquet"
        self.query(f"COPY (SELECT 1::BIGINT AS id, 'Alice' AS name UNION ALL SELECT 2, NULL UNION ALL SELECT 3, 'Charlie') TO {literal(self.parquet)} (FORMAT PARQUET)")

    def execute(self, sql):
        return subprocess.run([self.cli, "-no-agent", "-unsigned", "-json", "-c", f"LOAD {literal(self.extension)}; {sql}"],
                              capture_output=True, text=True, env=self.env, timeout=90)

    def query(self, sql, error=None):
        result = self.execute(sql)
        if error is not None:
            assert result.returncode != 0 and error.lower() in result.stderr.lower(), result.stderr
            return None
        assert result.returncode == 0, result.stderr
        return json.loads(result.stdout) if result.stdout.strip() else None

    def scan(self, reader, locator, **options):
        args = [literal(locator)]
        for key, value in options.items():
            sql_value = ("true" if value else "false") if isinstance(value, bool) else literal(json.dumps(value) if isinstance(value, dict) else value)
            args.append(f"{key} := {sql_value}")
        return "duckport_read_" + reader + "(" + ", ".join(args) + ")"

    @property
    def endpoint(self):
        if self.server is None:
            self._start_server()
        return f"http://127.0.0.1:{self.server.server_port}"

    def _start_server(self):
        root, requests = self.root, self.requests
        shards = {"default/train/0000.parquet": self.parquet.read_bytes(), "default/train/0001.parquet": self.parquet.read_bytes()}

        class Handler(http.server.BaseHTTPRequestHandler):
            def log_message(self, *args):
                pass

            def do_HEAD(self):
                self.handle_request(head=True)

            def do_GET(self):
                self.handle_request()

            def do_POST(self):
                self.handle_request()

            def handle_request(self, head=False):
                path = unquote(urlparse(self.path).path)
                requests.append((self.command, path))
                if "/revision/" in path:
                    return self.send_json({"sha": COMMIT})
                if "/tree/" in path or "/paths-info/" in path:
                    assert COMMIT in path
                    if self.command == "POST":
                        self.rfile.read(int(self.headers.get("Content-Length", "0")))
                    return self.send_json([{"type": "file", "path": name, "size": len(data), "oid": "b" * 40} for name, data in reversed(list(shards.items()))])
                if "/resolve/" in path:
                    assert COMMIT in path
                    data = shards.get(path.split("/resolve/" + COMMIT + "/", 1)[1])
                else:
                    file = root / path.removeprefix("/bucket/").lstrip("/")
                    data = file.read_bytes() if file.is_file() else None
                if data is None:
                    return self.send_error(404)
                start, end = 0, len(data) - 1
                byte_range = self.headers.get("Range")
                if byte_range:
                    first, last = byte_range.removeprefix("bytes=").split("-", 1)
                    start = int(first) if first else len(data) - int(last)
                    end = min(int(last), end) if first and last else end
                self.send_response(206 if byte_range else 200)
                self.send_header("Content-Length", str(end - start + 1))
                self.send_header("Accept-Ranges", "bytes")
                self.send_header("ETag", '"fixture"')
                if byte_range:
                    self.send_header("Content-Range", f"bytes {start}-{end}/{len(data)}")
                self.end_headers()
                if not head:
                    self.wfile.write(data[start:end + 1])

            def send_json(self, value):
                data = json.dumps(value).encode()
                self.send_response(200)
                self.send_header("Content-Type", "application/json")
                self.send_header("Content-Length", str(len(data)))
                self.end_headers()
                self.wfile.write(data)

        self.server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Handler)
        self.thread = threading.Thread(target=self.server.serve_forever, daemon=True)
        self.thread.start()

    def close(self):
        if self.server:
            self.server.shutdown()
            self.server.server_close()
            self.thread.join()
        self.temp.cleanup()
