"""Run the engineering tool shipped inside this sealed source project."""
import os
import sys
from pathlib import Path

# Running this command must not create an unlisted __pycache__ in sealed sources.
sys.dont_write_bytecode = True
os.environ["PYTHONDONTWRITEBYTECODE"] = "1"
# Use this package's sealed tools even when Python excludes script/site paths.
sys.path.insert(0, str(Path(__file__).resolve().parent))

from ecu_tools.cli import main

if __name__ == "__main__":
    raise SystemExit(main())
