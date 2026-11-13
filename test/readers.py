#!/usr/bin/env python3
"""Run the reader tests present in this checkout, optionally selecting one reader."""
import argparse
from pathlib import Path
import unittest
import reader_test_support as support

parser = argparse.ArgumentParser()
parser.add_argument("extension")
parser.add_argument("--reader")
args = parser.parse_args()
support.context = support.ReaderContext(args.extension)
try:
    suite = unittest.defaultTestLoader.discover(str(Path(__file__).parent / "readers"),
        pattern=f"test_{args.reader}.py" if args.reader else "test_*.py")
    if not suite.countTestCases():
        parser.error("no reader tests found")
    result = unittest.TextTestRunner(verbosity=2).run(suite)
finally:
    support.context.close()
raise SystemExit(not result.wasSuccessful())
