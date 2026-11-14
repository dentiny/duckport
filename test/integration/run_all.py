"""Run all provider HTTP SQL fixtures, or only the providers named on the command line."""
from pathlib import Path
import sys
import runpy

if __name__ == "__main__":
    names = sys.argv[1:] or [p.stem.removeprefix("test_") for p in sorted(Path(__file__).parent.glob("test_*.py"))]
    for name in names:
        runpy.run_path(str(Path(__file__).parent / f"test_{name}.py"), run_name="__main__")
