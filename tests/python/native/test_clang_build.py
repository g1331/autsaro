"""Generated samples compile with clang as a second compiler (#129).

Each runtime and generated translation unit of the sealed build closure is
compiled with the sample's own pinned flags plus clang warnings that MISRA does
not already cover, in production and in test mode. The FreeRTOS kernel and the
user application keep their own style and are compiled by GCC only.
"""

import collections
import json
import os
import shutil
import subprocess
import tempfile
import unittest
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

from autosar_tooling.c_standard import baseline_errors, documentation_findings

FOREIGN = ("kernel/", "app/")
WARNINGS = (
    "-Wshadow",
    "-Wmissing-prototypes",
    "-Wstrict-prototypes",
    "-Wcast-qual",
    "-Wextra-semi",
    "-Wformat=2",
    "-Wdocumentation",
    "-Wno-error=documentation",
)
TEST_MODE = ("-DECU_TARGET_TESTS", "-DOS_HOST_FAILURE_TESTS")


class ClangBuildTests(unittest.TestCase):
    def test_generated_samples_compile_with_clang_warnings(self):
        samples = os.environ.get("AUTOSAR_C_ANALYSIS_SAMPLES")
        if not samples:
            self.fail(
                "Generate c_analysis samples and set AUTOSAR_C_ANALYSIS_SAMPLES first"
            )
        clang = os.environ.get("AUTOSAR_CLANG") or shutil.which("clang")
        git = os.environ.get("AUTOSAR_GIT") or shutil.which("git")
        if clang is None or git is None:
            self.fail(
                "clang and git are required; set AUTOSAR_CLANG and AUTOSAR_GIT or add them to PATH"
            )
        version = subprocess.run(
            [clang, "--version"], capture_output=True, text=True, check=True
        )
        print(version.stdout.splitlines()[0])
        projects = sorted(p for p in Path(samples).iterdir() if p.is_dir())
        self.assertTrue(projects, "No generated samples")
        documentation = collections.Counter()
        for project in projects:
            with (
                self.subTest(project=project.name),
                tempfile.TemporaryDirectory() as temporary,
            ):
                failures, findings = self.compile(project, Path(temporary), clang, git)
                documentation |= findings
                self.assertEqual(failures, [], "\n".join(failures))
        self.assertEqual(baseline_errors(documentation, documentation=True), [])

    @staticmethod
    def compile(project: Path, scratch: Path, clang: str, git: str) -> tuple[list[str], collections.Counter]:
        target = json.loads((project / "target.json").read_text(encoding="utf-8"))
        # The build applies the pinned kernel patches first; its headers declare port hooks.
        kernel = scratch / "kernel"
        shutil.copytree(project / "kernel", kernel)
        environment = {**os.environ, "GIT_CEILING_DIRECTORIES": str(scratch)}
        for patch in target["kernelPatches"]:
            subprocess.run(
                [
                    git,
                    "apply",
                    "--ignore-space-change",
                    *("--include=" + name for name in patch["includes"]),
                    str(project / patch["path"].removeprefix("runtime/")),
                ],
                cwd=kernel,
                env=environment,
                check=True,
                capture_output=True,
            )
        includes = []
        for path in target["includePaths"]:
            root = (
                kernel / path.removeprefix("kernel/")
                if path.startswith("kernel/")
                else project / path
            )
            includes += ["-I", str(root)]
        sources = [*target["sources"], *target["entries"].values()]
        units = [name for name in sources if not name.startswith(FOREIGN)]

        def build(job: tuple[int, str, tuple[str, ...]]) -> tuple[str | None, collections.Counter]:
            index, name, mode = job
            argv = [
                clang,
                "-c",
                "-o",
                str(scratch / f"{index}.o"),
                *target["compilerFlags"],
                *mode,
                *WARNINGS,
                *includes,
                str(project / name),
            ]
            result = subprocess.run(argv, capture_output=True, text=True, check=False)
            return (
                f"{name} {' '.join(mode)}\n{result.stderr}"
                if result.returncode
                else None
            ), documentation_findings(result.stderr, project)

        jobs = [
            (index, name, mode)
            for index, (name, mode) in enumerate(
                (name, mode) for name in units for mode in ((), TEST_MODE)
            )
        ]
        with ThreadPoolExecutor(max_workers=os.cpu_count() or 2) as pool:
            results = list(pool.map(build, jobs))
        documentation = collections.Counter()
        for _, findings in results:
            documentation |= findings
        return [failure for failure, _ in results if failure], documentation


if __name__ == "__main__":
    unittest.main()
