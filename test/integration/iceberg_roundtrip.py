#!/usr/bin/env python3
"""COPY through DuckPort; read committed snapshots with DuckDB's Iceberg extension."""

import argparse
import copy
import json
import os
from pathlib import Path
import subprocess
import tempfile
import threading
import time
import unittest
import uuid
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from urllib.parse import urlsplit


def sql_string(value):
    return "'" + str(value).replace("'", "''") + "'"


class CatalogHandler(BaseHTTPRequestHandler):
    def log_message(self, *_):
        pass

    def reply(self, value, status=200):
        body = json.dumps(value).encode()
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def do_GET(self):
        if urlsplit(self.path).path == "/v1/config":
            self.reply({"defaults": {}, "overrides": {}})
        elif self.path == "/v1/namespaces/tests/tables/people":
            self.reply(self.server.table())
        else:
            self.reply(
                {
                    "error": {
                        "message": "No such table",
                        "type": "NoSuchTableException",
                        "code": 404,
                    }
                },
                404,
            )

    def do_POST(self):
        if self.path != "/v1/namespaces/tests/tables/people":
            self.reply(
                {
                    "error": {
                        "message": "No such table",
                        "type": "NoSuchTableException",
                        "code": 404,
                    }
                },
                404,
            )
            return
        request = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
        try:
            self.server.commit(request)
        except ValueError as error:
            self.reply(
                {
                    "error": {
                        "message": str(error),
                        "type": "CommitFailedException",
                        "code": 409,
                    }
                },
                409,
            )
        else:
            self.reply(self.server.table())


class Catalog(ThreadingHTTPServer):
    """Local catalog fixture: SDK writes real data/manifests; this publishes metadata."""

    daemon_threads = True

    def __init__(self, root):
        super().__init__(("127.0.0.1", 0), CatalogHandler)
        self.reject_commits = False
        self.commit_count = 0
        self.lock = threading.Lock()
        self.metadata_directory = root / "people" / "metadata"
        self.metadata_directory.mkdir(parents=True)
        self.metadata = {
            "format-version": 2,
            "table-uuid": str(uuid.uuid4()),
            "location": str(root / "people"),
            "last-sequence-number": 0,
            "last-updated-ms": int(time.time() * 1000),
            "last-column-id": 2,
            "schemas": [
                {
                    "type": "struct",
                    "schema-id": 0,
                    "fields": [
                        {"id": 1, "name": "id", "required": False, "type": "int"},
                        {"id": 2, "name": "name", "required": False, "type": "string"},
                    ],
                }
            ],
            "current-schema-id": 0,
            "partition-specs": [{"spec-id": 0, "fields": []}],
            "default-spec-id": 0,
            "last-partition-id": 999,
            "properties": {},
            "sort-orders": [{"order-id": 0, "fields": []}],
            "default-sort-order-id": 0,
            "snapshots": [],
            "snapshot-log": [],
            "metadata-log": [],
            "refs": {},
        }
        self.metadata_file = self.metadata_directory / "00000.metadata.json"
        self.metadata_file.write_text(json.dumps(self.metadata))
        self.worker = threading.Thread(target=self.serve_forever, daemon=True)

    def __enter__(self):
        self.worker.start()
        return self

    def __exit__(self, *_):
        self.shutdown()
        self.server_close()
        self.worker.join()

    def table(self):
        with self.lock:
            return {
                "metadata-location": str(self.metadata_file),
                "metadata": copy.deepcopy(self.metadata),
                "config": {},
            }

    def commit(self, request):
        with self.lock:
            if self.reject_commits:
                raise ValueError("injected commit conflict")
            for requirement in request["requirements"]:
                if requirement["type"] == "assert-table-uuid":
                    valid = requirement["uuid"] == self.metadata["table-uuid"]
                elif requirement["type"] == "assert-ref-snapshot-id":
                    current = (
                        self.metadata["refs"]
                        .get(requirement["ref"], {})
                        .get("snapshot-id")
                    )
                    valid = requirement.get("snapshot-id") == current
                else:
                    raise ValueError(
                        f"Unsupported fixture requirement: {requirement['type']}"
                    )
                if not valid:
                    raise ValueError("stale table state")
            metadata = copy.deepcopy(self.metadata)
            for update in request["updates"]:
                if update["action"] == "add-snapshot":
                    snapshot = update["snapshot"]
                    metadata["snapshots"].append(snapshot)
                    metadata["last-sequence-number"] = snapshot["sequence-number"]
                    metadata["last-updated-ms"] = snapshot["timestamp-ms"]
                elif update["action"] == "set-snapshot-ref":
                    name = update["ref-name"]
                    metadata["refs"][name] = {
                        k: v
                        for k, v in update.items()
                        if k not in ("action", "ref-name")
                    }
                    if name == "main":
                        metadata["current-snapshot-id"] = update["snapshot-id"]
                        metadata["snapshot-log"].append(
                            {
                                "timestamp-ms": metadata["last-updated-ms"],
                                "snapshot-id": update["snapshot-id"],
                            }
                        )
                else:
                    raise ValueError(f"Unsupported fixture update: {update['action']}")
            metadata_file = (
                self.metadata_directory / f"{self.commit_count + 1:05}.metadata.json"
            )
            metadata_file.write_text(json.dumps(metadata))
            self.metadata, self.metadata_file = metadata, metadata_file
            self.commit_count += 1


class IcebergRoundTrip(unittest.TestCase):
    def query(self, binary, setup, statement, error=None):
        env = os.environ.copy()
        env.pop("CODEX_CI", None)
        env.pop("DUCKPORT_ICEBERG_TOKEN", None)
        result = subprocess.run(
            [str(binary), "-unsigned", "-json", "-c", setup + statement],
            capture_output=True,
            text=True,
            timeout=90,
            env=env,
        )
        if error:
            self.assertNotEqual(result.returncode, 0, result.stdout)
            self.assertIn(error, result.stderr)
        else:
            self.assertEqual(result.returncode, 0, result.stderr)
        return json.loads(result.stdout or "[]") if not error else None

    def test_copy_round_trip(self):
        args = self.options
        writer_setup = f"LOAD {sql_string(args.duckport_extension.resolve())};"
        reader_setup = (
            f"LOAD {sql_string(args.iceberg_extension.resolve())};"
            if args.iceberg_extension
            else "LOAD iceberg;"
        )
        with tempfile.TemporaryDirectory(prefix="duckport-iceberg-") as temporary:
            root = Path(temporary).resolve()
            with Catalog(root) as catalog:
                options = (
                    f"FORMAT duckport_iceberg, APPEND, CATALOG_URI 'http://127.0.0.1:{catalog.server_port}', "
                    f"STORAGE_ROOT {sql_string(root)}"
                )

                def write(query, error=None):
                    return self.query(
                        args.writer,
                        writer_setup,
                        f"COPY ({query}) TO 'tests.people' ({options});",
                        error,
                    )

                def read():
                    # Only the independent Iceberg reader sees committed metadata.
                    return self.query(
                        args.reader,
                        reader_setup,
                        f"SELECT id, name FROM iceberg_scan({sql_string(catalog.metadata_file)}) ORDER BY id NULLS LAST;",
                    )

                expected = [
                    {"id": 1, "name": "Alice"},
                    {"id": 2, "name": None},
                    {"id": 3, "name": "Charlie"},
                    {"id": None, "name": "missing-id"},
                ]
                write(
                    "SELECT * FROM (VALUES (1, 'Alice'), (2, NULL), (3, 'Charlie'), (NULL, 'missing-id')) AS t(id, name)"
                )
                self.assertEqual(read(), expected)

                write("SELECT * FROM (VALUES (4, 'Δuck'), (5, '')) AS t(id, name)")
                expected[-1:-1] = [{"id": 4, "name": "Δuck"}, {"id": 5, "name": ""}]
                self.assertEqual(read(), expected)
                self.assertEqual(catalog.commit_count, 2)

                committed_metadata = catalog.metadata_file
                write("SELECT 6 AS id, 'empty' AS name WHERE false")
                self.assertEqual(catalog.metadata_file, committed_metadata)
                self.assertEqual(read(), expected)

                write(
                    "SELECT 'wrong type' AS id, 'invalid' AS name",
                    error="Iceberg input column names",
                )
                self.assertEqual(read(), expected)

                catalog.reject_commits = True
                write(
                    "SELECT 6 AS id, 'not-published' AS name",
                    error="CatalogCommitConflicts",
                )
                self.assertEqual(catalog.metadata_file, committed_metadata)
                self.assertEqual(catalog.commit_count, 2)
                self.assertEqual(read(), expected)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--writer", type=Path, default=Path("build/release/duckdb"))
    parser.add_argument(
        "--duckport-extension",
        type=Path,
        default=Path("build/release/extension/duckport/duckport.duckdb_extension"),
    )
    parser.add_argument(
        "--reader",
        default="duckdb",
        help="DuckDB CLI with its matching Iceberg extension installed",
    )
    parser.add_argument(
        "--iceberg-extension",
        type=Path,
        help="Load this explicit Iceberg extension instead of LOAD iceberg",
    )
    test = IcebergRoundTrip("test_copy_round_trip")
    test.options = parser.parse_args()
    result = unittest.TextTestRunner(verbosity=2).run(unittest.TestSuite([test]))
    raise SystemExit(not result.wasSuccessful())
