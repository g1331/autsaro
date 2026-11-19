"""Recompile verified sealed commands with explicit native test linker observers."""
from __future__ import annotations

import json
import os
import runpy
import sys
from pathlib import Path
from unittest.mock import patch

CAPTURE_FLAGS = [
    "-DOS_HOST_CAPTURE_TESTS=1",
    "-g",
    "-Wl,--wrap=Os_BackendShutdown",
    "-Wl,--wrap=Os_BackendRequestShutdown",
    "-Wl,--wrap=Ecu_TargetAbortNative",
    "-Wl,--wrap=WaitForMultipleObjects",
    "-Wl,--wrap=SuspendThread",
    "-Wl,--wrap=ResumeThread",
    "-Wl,--wrap=Ecu_TargetTakeOutput",
    "-Wl,--wrap=Os_TargetAdvanceOneTick",
]


def main() -> None:
    project, output = (Path(value).resolve() for value in sys.argv[1:3])
    mode = sys.argv[3]
    control = Path(sys.argv[4]).resolve() if len(sys.argv) >= 5 else None
    link_flags = sys.argv[5:]
    capture = link_flags[-len(CAPTURE_FLAGS):] == CAPTURE_FLAGS
    checked_flags = link_flags[:-len(CAPTURE_FLAGS)] if capture else link_flags
    if checked_flags not in ([], ["-Wl,--wrap=pthread_detach"],
                          ["-Wl,--wrap=Os_TargetTakeInput", "-Wl,--wrap=Os_TargetPostInput",
                           "-Wl,--wrap=vApplicationTickHook"],
                          ["-Wl,--wrap=Os_TargetWaitTick", "-Wl,--wrap=Os_TargetNotifyNative",
                           "-Wl,--wrap=poll", "-Wl,--wrap=__poll_chk"]):
        raise ValueError("Unsupported test linker observer")
    kinds = os.environ.get("AUTOSAR_SANITIZE", "")
    coverage = os.environ.get("AUTOSAR_C_COVERAGE") == "1" and mode == "host-batch"
    if kinds and (sys.platform != "linux" or kinds not in ("thread", "address,undefined")):
        raise ValueError("Sealed ECU sanitizer tests require Linux thread or address,undefined")
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
                    # Only the declared observer and its test/debug definitions extend
                    # the command; sealed source and include closure stays intact.
                    argv = [*argv, *link_flags]
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
    if not kinds and not coverage:
        # Keep the exact diagnostic command even without a sanitizer rebuild.
        (output / "compile-commands.json").write_text(
            json.dumps({"production": original}, indent=2) + "\n", encoding="utf-8",
        )
        return
    instrumented = list(original)
    binary = output / Path(original[original.index("-o") + 1]).name
    instrumented[instrumented.index("-o") + 1] = str(binary)
    if kinds:
        flags = [f"-fsanitize={kinds}", "-fno-sanitize-recover=all", "-fno-omit-frame-pointer", "-g"]
    else:
        # Keep __FILE__ and debug paths reproducible while leaving gcov's
        # source names resolvable against the actual generated project.
        for flag in original:
            if flag.startswith("-ffile-prefix-map="):
                index = instrumented.index(flag)
                instrumented[index:index + 1] = [flag.replace("-ffile-", "-fmacro-", 1),
                                                 flag.replace("-ffile-", "-fdebug-", 1)]
        exit_observer = Path(__file__).resolve().parents[1] / "fixtures/os/gcov_exit.c"
        flags = ["--coverage", "-fprofile-update=atomic", "-fprofile-abs-path",
                 "-Wl,--wrap=_exit", str(exit_observer)]
    instrumented[1:1] = flags
    stage = "sanitized-c99-build" if kinds else "coverage-c99-build"
    build.query(instrumented, output / "production/logs", stage)
    build.sealed_sources(project)
    (output / "compile-commands.json").write_text(
        json.dumps({"production": original, "instrumented": instrumented}, indent=2) + "\n",
        encoding="utf-8",
    )
    print(f"Built instrumented ECU native entry: {binary}")


if __name__ == "__main__":
    main()
