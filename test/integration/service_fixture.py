"""Run provider SQL against local HTTP fixtures derived from open-arrow-access tests."""
from contextlib import contextmanager
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from threading import Thread
import json
import os
import subprocess
from urllib.parse import urlsplit

REPO = Path(__file__).resolve().parents[2]
CLI = os.environ.get("DUCKDB_CLI", "duckdb")
EXTENSION = Path(os.environ.get("DUCKPORT_EXTENSION", REPO / "build/release/extension/duckport/duckport.duckdb_extension")).resolve()

@contextmanager
def fixture_server(required_bearer=None):
    routes, requests = {}, []
    class Handler(BaseHTTPRequestHandler):
        def log_message(self, *args): pass
        def do_HEAD(self):
            self.do_GET()
        def do_GET(self):
            requests.append((self.path, dict(self.headers)))
            result = routes.get(self.path, routes.get(urlsplit(self.path).path))
            if required_bearer and self.headers.get("Authorization") != f"Bearer {required_bearer}":
                result = (401, b"authentication required")
            status, body, *response_headers = result if result else (404, b"missing fixture")
            headers = dict(response_headers[0]) if response_headers else {}
            span = self.headers.get("Range")
            if span and status == 200:
                start, end = span.removeprefix("bytes=").split("-", 1)
                start = int(start)
                end = min(int(end) if end else len(body) - 1, len(body) - 1)
                headers["Content-Range"] = f"bytes {start}-{end}/{len(body)}"
                body = body[start:end + 1]
                status = 206
            self.send_response(status)
            for key, value in headers.items(): self.send_header(key, value)
            self.send_header("Content-Length", str(len(body)))
            self.send_header("Connection", "close")
            self.end_headers()
            if self.command != "HEAD":
                self.wfile.write(body)
    server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    server.daemon_threads = True
    thread = Thread(target=server.serve_forever, daemon=True)
    thread.start()
    try:
        yield f"http://127.0.0.1:{server.server_port}", routes, requests
    finally:
        server.shutdown()
        server.server_close()
        thread.join()

def literal(value):
    return "'" + str(value).replace("'", "''") + "'"

def sql(statement, setup="", error=None):
    script = f"LOAD {literal(EXTENSION)};\n{setup}\nSELECT '__duckport_test_result__';\n{statement}\n"
    result = subprocess.run([CLI, "-no-agent", "-no-init", "-unsigned", "-batch", "-bail", "-list", "-noheader", "-separator", "\t", "-nullvalue", "NULL"],
        input=script, text=True, capture_output=True, cwd=REPO, timeout=45)
    if error is not None:
        assert result.returncode, (statement, "expected failure", result.stdout)
        assert error in result.stderr, (statement, error, result.stderr)
    else:
        assert result.returncode == 0, (statement, result.stdout, result.stderr)
    return result.stdout.partition("__duckport_test_result__")[2].strip()

def run_sql_tests(path, substitutions, setup=""):
    text = Path(path).read_text()
    for key, value in substitutions.items(): text = text.replace(key, value)
    lines = text.splitlines()
    i, count = 0, 0
    while i < len(lines):
        heading = lines[i]
        if not heading.startswith(("query ", "statement ")):
            i += 1; continue
        i += 1
        command = []
        while i < len(lines) and lines[i] and lines[i] != "----":
            command.append(lines[i]); i += 1
        expected = []
        if i < len(lines) and lines[i] == "----":
            i += 1
            while i < len(lines) and lines[i]: expected.append(lines[i]); i += 1
        command = "\n".join(command)
        if heading == "statement ok":
            # Keep setup and PREPARE in every following CLI invocation.
            setup += "\n" + command
        elif heading == "statement error":
            sql(command, setup, error="\n".join(expected))
        else:
            actual = sql(command, setup).splitlines()
            if "rowsort" in heading: actual, expected = sorted(actual), sorted(expected)
            assert actual == expected, (str(path), command, actual, expected)
        count += 1
    return count

def run_case(name, authenticated=True):
    case_dir = REPO / "testdata" / name
    spec = json.loads((case_dir / "http.json").read_text())
    if not authenticated:
        spec.pop("auth", None)
    required_bearer = "fixture-key" if spec.get("auth") == "authorization" else None
    with fixture_server(required_bearer) as (endpoint, routes, requests), fixture_server() as (download, downloads, download_requests):
        def load(entry):
            raw = (case_dir / entry["file"]).read_bytes()
            return raw.replace(b"ORIGIN", download.encode()).replace(b"API_ORIGIN", endpoint.encode())
        for entry in spec["routes"]:
            headers = {"Location": entry["redirect"].replace("ORIGIN", download)} if "redirect" in entry else {}
            routes[entry["path"]] = (entry.get("status", 302 if headers else 200), load(entry), headers)
        for entry in spec.get("downloads", []): downloads[entry["path"]] = (entry.get("status", 200), load(entry))
        setup = ""
        if spec.get("auth"):
            scope = literal(endpoint + spec.get("auth_scope", "/"))
            setup = f"CREATE SECRET provider_test (TYPE http, BEARER_TOKEN 'fixture-key', SCOPE {scope});"
        count = run_sql_tests(REPO / "test/integration/sql" / f"{name}.test", {"__ENDPOINT__": endpoint, "__DOWNLOAD__": download}, setup)
        if "reject_route" in spec:
            before = len(download_requests)
            routes[spec["reject_route"]] = (200, b"not valid metadata")
            count += run_sql_tests(REPO / "test/integration/sql" / f"{name}_invalid.test", {"__ENDPOINT__": endpoint, "__DOWNLOAD__": download}, setup)
            assert len(download_requests) == before, "malformed metadata triggered a download"
        assert requests, "provider resolution was not exercised"
        paths = [path for path, _ in requests]
        for expected in spec.get("expected_paths", []): assert expected in paths, (expected, paths)
        if spec.get("auth"):
            for path, headers in requests:
                lower = {k.lower(): v for k, v in headers.items()}
                if spec["auth"].startswith("query:"):
                    assert spec["auth"][6:] + "=fixture-key" in path, (path, headers)
                else:
                    expected = "Bearer fixture-key" if spec["auth"].lower() == "authorization" else "fixture-key"
                    assert lower.get(spec["auth"].lower()) == expected, (path, headers)
        else:
            for _, headers in requests:
                assert "authorization" not in {key.lower() for key in headers}, headers
        for _, headers in download_requests:
            assert "authorization" not in {key.lower() for key in headers}, headers
            assert "fixture-key" not in str(headers), headers
        assert download_requests or spec.get("same_origin_download"), "attachment bytes were not read"
        print(f"{name} ({'authenticated' if spec.get('auth') else 'anonymous'}): {count} SQL assertions passed")
