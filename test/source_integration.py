#!/usr/bin/env python3
"""End-to-end source tests against a built extension and local HTTP/S3/HF fixtures.

Usage: python3 test/source_integration.py /absolute/path/duckport.duckdb_extension
Requires a matching DuckDB CLI on PATH. No external service or credentials needed.
"""
import gzip
import http.server
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import threading
from urllib.parse import unquote, urlparse

EXTENSION = str(Path(sys.argv[1]).resolve())
CLI = os.environ.get("DUCKDB_CLI", "duckdb")
ENV = {**os.environ, "HF_HUB_DISABLE_IMPLICIT_TOKEN": "1"}
COMMIT = "a" * 40


def literal(value):
    return "'" + str(value).replace("'", "''") + "'"


def query(sql, error=None):
    result = subprocess.run(
        [CLI, "-no-agent", "-unsigned", "-json", "-c", f"LOAD {literal(EXTENSION)}; {sql}"],
        text=True, capture_output=True, env=ENV, timeout=90,
    )
    if error is not None:
        assert result.returncode != 0 and error.lower() in result.stderr.lower(), result.stderr
        return None
    assert result.returncode == 0, result.stderr
    return json.loads(result.stdout) if result.stdout.strip() else None


def scan(path, **options):
    args = [literal(path)]
    args += [f"{key} := {literal(json.dumps(value) if key == 'options' else value)}" for key, value in options.items()]
    return "duckport_scan(" + ", ".join(args) + ")"


with tempfile.TemporaryDirectory(prefix="duckport-source-") as directory:
    root = Path(directory)
    csv = root / "basic.csv"
    csv.write_text("id,name\n1,Alice\n2,\n3,Charlie\n")
    rows = [{"id": 1, "name": "Alice"}, {"id": 2, "name": None}, {"id": 3, "name": "Charlie"}]
    assert query(f"SELECT * FROM {scan(csv, batch_size=2, options={'format': {'header': 'true'}})}") == rows
    assert query(f"SELECT count(*) AS n FROM {scan(csv)}") == [{"n": 3}]
    assert query(f"SELECT name FROM {scan(csv)} WHERE id=3") == [{"name": "Charlie"}]
    assert query(f"SELECT * FROM {scan(csv.as_uri())} LIMIT 1") == rows[:1]
    # The CLI prints one JSON array per result, so check repeated executions in a single boolean result.
    repeat = subprocess.run([CLI, "-no-agent", "-unsigned", "-json", "-c",
        f"LOAD {literal(EXTENSION)}; PREPARE src AS SELECT sum(id)::BIGINT AS total FROM {scan(csv)}; EXECUTE src; EXECUTE src;"],
        capture_output=True, text=True, env=ENV, timeout=90)
    assert repeat.returncode == 0 and repeat.stdout.count('"total":6') == 2, (repeat.stdout, repeat.stderr)
    parquet = root / "basic.parquet"
    query(f"COPY (SELECT 1::BIGINT AS id, 'Alice' AS name UNION ALL SELECT 2, NULL UNION ALL SELECT 3, 'Charlie') TO {literal(parquet)} (FORMAT PARQUET)")
    assert query(f"SELECT * FROM {scan(parquet, batch_size=2)}") == rows
    assert query(f"SELECT count(*) AS n FROM {scan(parquet)}") == [{"n": 3}]
    nested = root / "nested.jsonl"
    nested.write_text('{"id":1,"nested":{"ok":true},"tags":["a","b"]}\n{"id":2,"nested":{"ok":false},"tags":[]}\n')
    assert query(f"SELECT id, nested.ok AS ok, len(tags) AS n FROM {scan(nested)}") == [
        {"id": 1, "ok": True, "n": 2}, {"id": 2, "ok": False, "n": 0}]
    compressed = root / "nested.jsonl.gz"
    compressed.write_bytes(gzip.compress(nested.read_bytes()))
    assert query(f"SELECT count(*) AS n FROM {scan(compressed)}") == [{"n": 2}]
    tsv = root / "basic.tsv"
    tsv.write_text(csv.read_text().replace(",", "\t"))
    assert query(f"SELECT * FROM {scan(tsv, options={'format': {'header': 'true'}})}") == rows
    no_extension = root / "no-extension"
    no_extension.write_bytes(nested.read_bytes())
    assert query(f"SELECT count(*) AS n FROM {scan(no_extension, format='json')}") == [{"n": 2}]
    xml = root / "basic.xml"
    xml.write_text('<records><record><id>1</id><name>Alice</name></record><record><id>2</id><name>Bob</name></record></records>')
    assert query(f"SELECT count(*) AS n FROM {scan(xml, options={'format': {'record_path': '/records/record'}})}") == [{"n": 2}]
    changing = root / "changing.csv"
    changing.write_text("id\n1\n")
    query(f"PREPARE changing_scan AS SELECT * FROM {scan(changing)}; EXECUTE changing_scan; "
          f"COPY (SELECT 'new-type' AS id) TO {literal(changing)} (FORMAT CSV, HEADER true); EXECUTE changing_scan;",
          error="schema changed")
    output = root / "output.parquet"
    query(f"COPY (SELECT * FROM {scan(csv)}) TO {literal(output)} (FORMAT PARQUET)")
    assert query(f"SELECT * FROM read_parquet({literal(output)})") == rows
    query(f"SELECT * FROM {scan(root / 'missing.csv')}", error="duckport_scan")
    query(f"SELECT * FROM {scan(csv, batch_size=0)}", error="batch_size")
    query(f"SELECT * FROM {scan(csv, options={'unknown': {}})}", error="unknown field")
    query(f"SELECT * FROM {scan(csv, options={'format': {'typo': 'true'}})}", error="unrecognized")
    query(f"SELECT * FROM {scan(no_extension)}", error="cannot infer")
    query(f"SELECT * FROM {scan(csv, format='no-such-format')}", error="not registered")
    query(f"SET enable_external_access=false; SELECT * FROM {scan(csv)}", error="enable_external_access")
    query("SELECT * FROM duckport_scan(NULL)", error="NULL")
    query(f"SELECT * FROM duckport_scan({literal(csv)}, options := NULL)", error="NULL")
    query(f"SELECT * FROM duckport_scan({literal(csv)}, format := 'j' || chr(0) || 'son')", error="NUL")
    # Errors after schema inference must propagate through Arrow get_next rather than becoming EOF.
    corrupt = root / "corrupt.jsonl"
    corrupt.write_text('{"id":1}\n' * 1100 + '{not-json}\n')
    query(f"SELECT sum(id) FROM {scan(corrupt, batch_size=32)}", error="get_next")

    requests = []
    shards = {"default/train/0000.parquet": parquet.read_bytes(), "default/train/0001.parquet": parquet.read_bytes()}
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
                self.send_json({"sha": COMMIT})
                return
            if "/tree/" in path:
                assert COMMIT in path, "membership must use pinned revision"
                # Intentionally return files out of order.
                self.send_json([{"type": "file", "path": name, "size": len(data), "oid": "b" * 40} for name, data in reversed(list(shards.items()))])
                return
            if "/paths-info/" in path:
                self.rfile.read(int(self.headers.get("Content-Length", "0")))
                self.send_json([{"type": "file", "path": name, "size": len(data), "oid": "b" * 40} for name, data in shards.items()])
                return
            if "/resolve/" in path:
                assert COMMIT in path
                name = path.split("/resolve/" + COMMIT + "/", 1)[1]
                data = shards.get(name)
            else:
                file = root / path.removeprefix("/bucket/").lstrip("/")
                data = file.read_bytes() if file.is_file() else None
            if data is None:
                self.send_error(404)
                return
            start, end = 0, len(data) - 1
            range_header = self.headers.get("Range")
            if range_header:
                first, last = range_header.removeprefix("bytes=").split("-", 1)
                start = int(first) if first else len(data) - int(last)
                end = min(int(last), end) if first and last else end
            self.send_response(206 if range_header else 200)
            self.send_header("Content-Length", str(end - start + 1))
            self.send_header("Accept-Ranges", "bytes")
            self.send_header("ETag", '"fixture"')
            self.send_header("Last-Modified", "Thu, 12 Nov 2026 00:00:00 GMT")
            if range_header:
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

    server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    endpoint = f"http://127.0.0.1:{server.server_port}"
    try:
        assert query(f"SELECT * FROM {scan(endpoint + '/basic.parquet')}") == rows
        assert query(f"SELECT * FROM {scan(endpoint + '/basic.csv')}") == rows
        s3options = {"storage": {"endpoint": endpoint, "region": "us-east-1", "allow_anonymous": "true", "disable_config_load": "true", "disable_ec2_metadata": "true"}}
        assert query(f"SELECT * FROM {scan('s3://bucket/basic.parquet', options=s3options)}") == rows
        hfoptions = {"service": {"config": "default", "split": "train", "endpoint": endpoint, "token": "fixture-only"}}
        hf = scan('hf://datasets/owner/example', options=hfoptions, batch_size=2)
        assert query(f"SELECT * FROM {hf}") == rows + rows
        assert any("/revision/" in path for _, path in requests)
        assert any("/tree/" in path for _, path in requests)
        assert any("/resolve/" in path for _, path in requests)
        hf_file = {"service": {"path": "default/train/0000.parquet", "revision": COMMIT, "endpoint": endpoint, "token": "fixture-only"}}
        assert query(f"SELECT * FROM {scan('hf://datasets/owner/example', options=hf_file)}") == rows
    finally:
        server.shutdown()
        server.server_close()
        thread.join()
print("Source integration passed: local, HTTP, S3, Hugging Face, formats, streaming, errors, rescans, COPY.")
