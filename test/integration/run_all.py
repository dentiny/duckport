"""Run all provider HTTP SQL fixtures, or only the providers named on the command line."""
from pathlib import Path
import sys
from service_fixture import run_case

if __name__ == "__main__":
    names = sys.argv[1:] or [p.stem.removeprefix("test_") for p in sorted(Path(__file__).parent.glob("test_*.py"))]
    for name in names:
        run_case(name)
