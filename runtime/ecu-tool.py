"""Run the engineering tool shipped inside this sealed source project."""
import os
import sys

# Running this command must not create an unlisted __pycache__ in sealed sources.
sys.dont_write_bytecode = True
os.environ["PYTHONDONTWRITEBYTECODE"] = "1"

from ecu_tools.cli import main

if __name__ == "__main__":
    raise SystemExit(main())
