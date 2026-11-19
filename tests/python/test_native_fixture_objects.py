"""The fixture compiler shares only the invariant inputs across private package copies."""

import json
import os
import runpy
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

HELPER = Path(__file__).resolve().parents[2] / "core/tests/support/cached_compile.py"
compile_units = runpy.run_path(str(HELPER))["compile_units"]


class NativeFixtureObjects(unittest.TestCase):
    def test_changed_headers_and_runtime_rebuild_but_controls_always_compile(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            cache = root / "objects"
            cache.mkdir()
            with patch.dict(os.environ, AUTOSAR_TEST_OBJECTS=str(cache)):
                self.compare_builds(root)

    def compare_builds(self, root):
        compiled = []
        linked = []

        def compiler(argv, *, check, cwd):
            if "-c" in argv:
                source = Path(argv[argv.index("-c") + 1])
                compiled.append(source.name)
                header = source.with_suffix(".h")
                deps = Path(argv[argv.index("-MF") + 1])
                names = [
                    path.as_posix().replace(" ", "\\ ") for path in (source, header)
                ]
                deps.write_text("autsaro-tu: " + " ".join(names) + "\n")
                Path(argv[-1]).write_bytes(b"compiled object")
            else:
                linked.append(argv)

        def build(label, header, runtime, control, *, added=False):
            project = root / label / "project"
            output = root / label / "build"
            src = project / "src"
            src.mkdir(parents=True)
            (project / "meta").mkdir()
            files = [
                "src/runtime.c",
                "src/other.c",
                "src/observer.c",
                "src/runtime.h",
                "src/other.h",
                "src/unused.h",
            ]
            if added:
                files.append("src/override.h")
                (src / "override.h").write_text("new include candidate")
            (project / "meta/files.list").write_text("\n".join(sorted(files)) + "\n")
            (src / "unused.h").write_text(control)
            (output / "kernel").mkdir(parents=True)
            (src / "runtime.h").write_text(header)
            (src / "other.h").write_text("independent header")
            (src / "other.c").write_text("independent source")
            (src / "runtime.c").write_text(runtime)
            (src / "observer.c").write_text(control)
            entry = root / label / "control.c"
            entry.write_text(control)
            target = {
                "entries": {"probe": "src/main.c", "host-batch": "src/host.c"},
                "sources": ["src/runtime.c", "src/other.c", "src/observer.c"],
                "kernelSources": [],
                "kernelSha256": {},
                "toolchain": {"executable_sha256": "pinned"},
                "linkLibraries": [],
            }
            (project / "target.json").write_text(json.dumps(target))
            compile_units(
                [
                    "gcc",
                    f"-ffile-prefix-map={project}=.",
                    f"-ffile-prefix-map={output}=build",
                    "-I",
                    str(src),
                    str(src / "runtime.c"),
                    str(src / "other.c"),
                    str(src / "observer.c"),
                    str(entry),
                    "-o",
                    str(output / "probe"),
                ],
                output,
                ["observer.c"],
            )
            self.assertIn(str(src / "observer.c"), linked[-1])
            self.assertIn(str(entry), linked[-1])
            return len(compiled)

        with patch("subprocess.run", compiler):
            self.assertEqual(build("first", "1", "a", "original"), 2)
            self.assertEqual(build("copy with spaces", "1", "a", "mutated"), 2)
            self.assertEqual(build("header", "2", "a", "mutated"), 3)
            self.assertEqual(build("runtime", "2", "b", "mutated"), 4)
            self.assertEqual(build("added", "2", "b", "mutated", added=True), 6)
        self.assertEqual(
            compiled,
            ["runtime.c", "other.c", "runtime.c", "runtime.c", "runtime.c", "other.c"],
        )
