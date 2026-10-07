import sys

from autosar_tooling.cli import main

if __name__ == "__main__":
    # Subprocess diagnostics and paths can contain Unicode even on a GBK host.
    sys.stdout.reconfigure(encoding="utf-8")
    sys.stderr.reconfigure(encoding="utf-8")
    try:
        raise SystemExit(main())
    except (OSError, ValueError) as error:
        print(str(error), file=sys.stderr)
        raise SystemExit(2) from error
