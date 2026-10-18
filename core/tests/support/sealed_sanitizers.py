"""Recompile a verified sealed build's exact command for the native sanitizer tests."""
from __future__ import annotations

import json
import os
import runpy
import sys
from pathlib import Path
from unittest.mock import patch


def main() -> None:
    project, output = (Path(value).resolve() for value in sys.argv[1:3])
    mode = sys.argv[3]
    control = Path(sys.argv[4]).resolve() if len(sys.argv) == 5 else None
    kinds = os.environ["AUTOSAR_SANITIZE"]
    if sys.platform != "linux" or kinds != "address,undefined":
        raise ValueError("Sealed ECU sanitizer tests require Linux address,undefined")
    # Let the seal guard establish import paths after its standard-library imports.
    sys.dont_write_bytecode = True
    guard = runpy.run_path(str(project / "tools/ecu-tool.py"))
    load = runpy.run_path
    commands: list[list[str]] = []

    def load_tool(path: str) -> dict:
        standalone = load(path)
        original = standalone["main"]

        def invoke() -> int:
            # The guard has verified tool bytes and the complete seal before this call.
            from ecu_tools import build
            query = build.query

            def observe(argv: list[str], logs: Path, stage: str, **kwargs: object) -> str:
                if stage == "native-c99-build":
                    commands.append(list(argv))
                return query(argv, logs, stage, **kwargs)

            with patch.object(build, "query", observe):
                return original()

        standalone["main"] = invoke
        return standalone

    argv = [str(project / "tools/ecu-tool.py"), "build", "--project", str(project),
            "--output", str(output / "production"), "--mode", mode]
    if control is not None:
        argv += ["--control-source", str(control)]
    if output.exists():
        raise ValueError("Sanitizer build output must be new")
    # Run the original seal guard, CLI, build and strict check_object unchanged.
    with patch.object(sys, "argv", argv), patch.object(runpy, "run_path", load_tool):
        result = guard["run"]()
    if result != 0:
        raise RuntimeError(f"Production ECU build failed: exit={result}")
    from ecu_tools import build

    if len(commands) != 1:
        raise ValueError("Expected exactly one sealed native compilation")
    original = commands[0]
    instrumented = list(original)
    binary = output / Path(original[original.index("-o") + 1]).name
    instrumented[instrumented.index("-o") + 1] = str(binary)
    flags = [f"-fsanitize={kinds}", "-fno-sanitize-recover=all", "-fno-omit-frame-pointer", "-g"]
    instrumented[1:1] = flags
    build.query(instrumented, output / "production/logs", "sanitized-c99-build")
    build.sealed_sources(project)
    (output / "compile-commands.json").write_text(
        json.dumps({"production": original, "instrumented": instrumented}, indent=2) + "\n",
        encoding="utf-8",
    )
    print(f"Built instrumented ECU native entry: {binary}")


if __name__ == "__main__":
    main()
