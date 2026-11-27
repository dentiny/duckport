"""Reject an embedded DuckDB engine in the Rust bridge archive."""

import argparse
import re
import subprocess
import sys


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--nm", required=True)
    parser.add_argument("archive")
    args = parser.parse_args()
    command = [args.nm, "-g"]
    if sys.platform == "darwin":
        # Apple nm can lag Rust's LLVM bitcode format; the embedded engine
        # contributes native C++ object files, which are what we audit here.
        command.append("--no-llvm-bc")
    result = subprocess.run(
        [*command, args.archive], check=True, text=True, capture_output=True
    )
    engine_symbol = re.compile(r"^_*(?:ZN6duckdb|Z(?:TV|TI|TS)N6duckdb|duckdb_)")
    for line in result.stdout.splitlines():
        fields = line.split()
        if len(fields) >= 2 and fields[-2].upper() != "U" and engine_symbol.match(fields[-1]):
            raise SystemExit(
                "Rust bridge embeds DuckDB engine symbols: " + fields[-1]
                + ". Disable the dependency's duckdb-bundled feature; "
                "the C++ extension must use its matching host DuckDB."
            )
    print("Rust bridge contains no embedded DuckDB engine symbols")


if __name__ == "__main__":
    main()
