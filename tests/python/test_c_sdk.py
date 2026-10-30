"""SDK provenance rejects lookalike paths, changed originals and ambiguous macros."""

from __future__ import annotations

import gzip
import hashlib
import tempfile
import unittest
from copy import deepcopy
from pathlib import Path
from unittest.mock import patch

from autosar_tooling import c_sdk

TARGET = "windows-x64-controlled-v1"


class SdkSourceTests(unittest.TestCase):
    def fixture(self, directory):
        root = directory / "project"
        root.mkdir()
        header = directory / "provider/include/winbase.h"
        header.parent.mkdir(parents=True)
        header.write_bytes(b"#define INFINITE 0xffffffff\n")
        package = {
            "provider": "mingw-w64-x86_64-headers",
            "version": "15.0.0-1",
            "header": "mingw64/include/winbase.h",
            "algorithm": "sha256",
            "expected": hashlib.sha256(header.read_bytes()).hexdigest(),
        }
        packages = {str(header): package}
        record = c_sdk.header_identity(
            header, root, TARGET, "x86_64-w64-mingw32", packages
        )
        return root, header, packages, record

    def diagnostic(self):
        return {
            "id": "misra-c2012-7.2",
            "severity": "style",
            "message": "raw",
            "locations": [{"file": "kernel/port.c", "line": "54", "column": "5"}],
            "patched": True,
            "adopted": False,
            "programs": ["host-batch", "probe"],
        }

    def test_original_identity_is_required_not_a_directory_or_basename(self):
        with tempfile.TemporaryDirectory() as temporary:
            root, header, packages, record = self.fixture(Path(temporary).resolve())
            self.assertEqual(record["category"], "sdk")
            self.assertEqual(record["provider"], "mingw-w64-x86_64-headers")
            cases = [
                ({}, TARGET, "x86_64-w64-mingw32", "unknown provider"),
                (packages, TARGET, "x86_64-linux-gnu", "target/provider mismatch"),
                (
                    {str(header): {**packages[str(header)], "provider": "unknown-sdk"}},
                    TARGET,
                    "x86_64-w64-mingw32",
                    "target/provider mismatch",
                ),
                (
                    packages,
                    "linux-x64-controlled-v1",
                    "x86_64-linux-gnu",
                    "target/provider mismatch",
                ),
            ]
            for providers, target, machine, reason in cases:
                with self.subTest(reason=reason):
                    rejected = c_sdk.header_identity(
                        header, root, target, machine, providers
                    )
                    self.assertEqual(rejected["category"], "unclassified")
                    self.assertIn(reason, rejected["reason"])
            project_header = root / header.name
            project_header.write_bytes(header.read_bytes())
            own = c_sdk.header_identity(
                project_header,
                root,
                TARGET,
                "x86_64-w64-mingw32",
                {str(project_header): packages[str(header)]},
            )
            self.assertEqual(own["reason"], "project source")
            header.write_bytes(header.read_bytes() + b"/* self-owned patch */\n")
            changed = c_sdk.header_identity(
                header, root, TARGET, "x86_64-w64-mingw32", packages
            )
            self.assertEqual(changed["category"], "unclassified")
            self.assertIn("differs", changed["reason"])
            self.assertNotEqual(changed["key"], record["key"])

    def test_actual_dependency_closure_does_not_trust_line_markers_or_search_paths(
        self,
    ):
        with tempfile.TemporaryDirectory() as temporary:
            base = Path(temporary).resolve()
            root = base / "project"
            root.mkdir()
            header = base / "provider space/include/winbase.h"

            def escaped(path):
                return path.as_posix().replace(":", "\\:").replace(" ", "\\ ")

            deps = (
                "autsaro-tu: "
                + escaped(root / "main.c")
                + " \\\n "
                + escaped(header)
                + "\n"
            )
            self.assertEqual(c_sdk.dependency_headers(deps, root), {header})
            for invalid in ("", "other: one.h", "autsaro-tu:"):
                with (
                    self.subTest(invalid=invalid),
                    self.assertRaisesRegex(ValueError, "dependency inventory"),
                ):
                    c_sdk.dependency_headers(invalid, root)

    def test_line_markers_bind_active_definition_and_project_redefinition(self):
        with tempfile.TemporaryDirectory() as temporary:
            root, header, _, _ = self.fixture(Path(temporary).resolve())
            source = root / "port.c"
            source.write_text("wait(INFINITE);\nwait(INFINITE);\nwait(INFINITE);\n")
            text = (
                f'# 1 "{header.as_posix()}" 1\n#define INFINITE 0xffffffff\n'
                f'# 1 "{source.as_posix()}" 2\nwait(0xffffffff);\n'
                "#define INFINITE 3U\n"
                f'# 2 "{source.as_posix()}"\nwait(3U);\n#undef INFINITE\n'
                f'# 3 "{source.as_posix()}"\nwait(INFINITE);\n'
            )
            inputs = c_sdk.preprocessing(text, root, {"INFINITE"})
            self.assertEqual(
                inputs["port.c:1"]["INFINITE"],
                {
                    "file": header.as_posix(),
                    "line": 1,
                    "value": "0xffffffff",
                    "function": False,
                    "use": "wait(INFINITE);",
                },
            )
            self.assertEqual(inputs["port.c:2"]["INFINITE"]["file"], "port.c")
            self.assertNotIn("port.c:3", inputs)

    def test_unsigned_operand_connects_sdk_and_preserves_raw_patch_responsibility(self):
        with tempfile.TemporaryDirectory() as temporary:
            _root, header, _, record = self.fixture(Path(temporary).resolve())
            definition = {
                "file": header.as_posix(),
                "line": 1,
                "value": "0xffffffff",
                "function": False,
                "use": "configASSERT(wait(INFINITE) == 0U);",
            }
            scope = {
                "sdkHeaders": [record],
                "sdkMacroInputs": {
                    "kernel/port.c": {"kernel/port.c:54": {"INFINITE": definition}}
                },
            }
            raw = self.diagnostic()
            rows = [deepcopy(raw)]
            token = {
                "file": "kernel/port.c",
                "line": 54,
                "column": 5,
                "literal": "0xffffffff",
            }
            result = c_sdk.classify(rows, scope, [token, token])
            self.assertEqual(
                {
                    k: v
                    for k, v in rows[0].items()
                    if k not in {"sourceOrigins", "sourceCategory"}
                },
                raw,
            )
            self.assertEqual(rows[0]["sourceOrigins"][0]["kind"], "unsigned-literal")
            self.assertEqual(rows[0]["sourceCategory"], "sdk")
            self.assertEqual(result["unclassifiedDiagnosticKeys"], 0)
            self.assertEqual(result["sdkDiagnosticKeys"], 1)
            self.assertTrue(result["complete"])
            # A project #line can spoof an SDK path; retain unresolved attribution.
            scope["sdkLineDirectives"] = True
            result = c_sdk.classify(rows, scope, [token])
            self.assertFalse(result["macroTraceComplete"])
            self.assertFalse(result["complete"])
            self.assertEqual(rows[0]["sourceCategory"], "unclassified")
            scope.pop("sdkLineDirectives")
            # An identical project literal on the report line cannot be blamed on SDK.
            definition["use"] = "configASSERT(INFINITE == 0xffffffff);"
            c_sdk.classify(rows, scope, [token])
            self.assertEqual(rows[0]["sourceCategory"], "unclassified")
            definition["use"] = "configASSERT(wait(INFINITE) == 0U);"
            # No operand receipt: input evidence remains, exact causality is not claimed.
            c_sdk.classify(rows, scope, [])
            self.assertEqual(rows[0]["sourceOrigins"][0]["kind"], "macro-input")
            # A second TU gives the same name a project definition: no SDK-only operand.
            scope["sdkMacroInputs"]["other.c"] = {
                "kernel/port.c:54": {"INFINITE": {**definition, "file": "own.h"}}
            }
            c_sdk.classify(rows, scope, [token])
            self.assertEqual(rows[0]["sourceOrigins"][0]["kind"], "macro-input")
            scope["sdkHeaders"][0]["category"] = "unclassified"
            result = c_sdk.classify(rows, scope, [token])
            self.assertEqual(result["unclassifiedDiagnosticKeys"], 1)
            self.assertFalse(result["complete"])

    def test_missing_macro_trace_cannot_claim_complete_coverage(self):
        with tempfile.TemporaryDirectory() as temporary:
            root, _header, _packages, record = self.fixture(Path(temporary).resolve())
            source = root / "kernel/port.c"
            source.parent.mkdir()
            source.write_text("\n" * 53 + "configASSERT(wait(INFINITE) == 0U);\n")
            scope = {
                "target": TARGET,
                "sdkHeaders": [record],
                "sdkMacroInputs": {},
                "compilerDiscoveredDefines": {
                    "kernel/port.c": ["-DINFINITE=0xffffffff"]
                },
            }
            rows = [self.diagnostic()]
            result = c_sdk.classify(rows, scope, [], root)
            self.assertFalse(result["complete"])
            self.assertEqual(result["unclassifiedDiagnosticKeys"], 1)
            self.assertEqual(rows[0]["sourceCategory"], "unclassified")
            self.assertEqual(rows[0]["sourceOrigins"][0]["kind"], "unresolved-macro")
            # Comment/string spellings do not create SDK macro uses.
            source.write_text(
                "\n" * 53 + 'own(); /* INFINITE */ const char *s = "INFINITE";\n'
            )
            result = c_sdk.classify(rows, scope, [], root)
            self.assertEqual(rows[0]["sourceOrigins"], [])
            self.assertEqual(result["unclassifiedDiagnosticKeys"], 0)

    def test_unknown_header_fallback_is_visible_even_without_diagnostics(self):
        with tempfile.TemporaryDirectory() as temporary:
            root, header, _, _ = self.fixture(Path(temporary).resolve())
            unknown = c_sdk.header_identity(
                header, root, TARGET, "x86_64-w64-mingw32", {}
            )
            result = c_sdk.classify([], {"sdkHeaders": [unknown]}, [])
            self.assertEqual(result["unclassifiedHeaders"], 1)
            self.assertFalse(result["complete"])
            self.assertFalse(c_sdk.classify([], {}, [])["complete"])

    def test_msys_manifest_comes_from_selected_installation_and_verifies_bytes(self):
        with tempfile.TemporaryDirectory() as temporary:
            base = Path(temporary).resolve()
            compiler = base / "mingw64/bin/gcc.exe"
            compiler.parent.mkdir(parents=True)
            package = base / "var/lib/pacman/local/headers-15.0.0-1"
            package.mkdir(parents=True)
            (package / "desc").write_text(
                "%NAME%\nmingw-w64-x86_64-headers\n\n%VERSION%\n15.0.0-1\n\n"
            )
            with gzip.open(package / "mtree", "wt") as stream:
                stream.write(
                    "./mingw64/include/winbase.h type=file sha256digest="
                    + "a" * 64
                    + "\n"
                )
            records = c_sdk.msys_headers(compiler)
            self.assertEqual(
                records[str(base / "mingw64/include/winbase.h")]["expected"], "a" * 64
            )
            self.assertEqual(
                records[str(base / "mingw64/include/winbase.h")]["version"], "15.0.0-1"
            )
            self.assertEqual(c_sdk.msys_headers(base / "other/bin/gcc.exe"), {})

    def test_debian_provider_package_version_and_architecture_are_explicit(self):
        with tempfile.TemporaryDirectory() as temporary:
            base = Path(temporary)
            status, info = base / "status", base / "info"
            info.mkdir()
            status.write_text(
                "Package: libc6-dev\nStatus: install ok installed\nArchitecture: amd64\nVersion: 2.39-1\n\n"
                "Package: unknown-sdk\nStatus: install ok installed\nArchitecture: amd64\nVersion: 1\n\n"
            )
            for name in ("libc6-dev", "unknown-sdk"):
                (info / (name + ":amd64.md5sums")).write_text(
                    "a" * 32 + "  usr/include/pthread.h\n"
                )
            records = c_sdk.debian_headers(status, info)
            self.assertEqual(records["/usr/include/pthread.h"]["provider"], "libc6-dev")
            self.assertEqual(records["/usr/include/pthread.h"]["version"], "2.39-1")
            status.write_text(status.read_text().replace("amd64", "arm64"))
            self.assertEqual(c_sdk.debian_headers(status, info), {})

    def test_identity_changes_and_missing_provider_remain_strict(self):
        with tempfile.TemporaryDirectory() as temporary:
            _root, header, packages, record = self.fixture(Path(temporary).resolve())
            scope = {
                "target": TARGET,
                "diagnosticCompiler": {"path": "/selected/bin/gcc.exe"},
                "sdkHeaders": [record],
                "sdkTranslationUnits": {"port.c": [header.as_posix()]},
                "sdkMacroInputs": {},
            }
            with patch.object(c_sdk, "msys_headers", return_value=packages):
                initial = c_sdk.semantic(scope)
                self.assertEqual(len(initial), 64)
                # Source-line movement and installation paths are provenance only.
                scope["sdkMacroInputs"] = {
                    "port.c": {
                        "port.c:54": {
                            "INFINITE": {
                                "file": header.as_posix(),
                                "line": 1,
                                "value": "0xffffffff",
                                "function": False,
                                "use": "wait(INFINITE);",
                            }
                        }
                    }
                }
                bound = c_sdk.semantic(scope)
                moved = deepcopy(scope)
                moved["sdkMacroInputs"]["port.c"]["port.c:60"] = moved[
                    "sdkMacroInputs"
                ]["port.c"].pop("port.c:54")
                self.assertEqual(c_sdk.semantic(moved), bound)
                moved["sdkMacroInputs"]["port.c"]["port.c:60"]["INFINITE"]["value"] = (
                    "3U"
                )
                self.assertNotEqual(c_sdk.semantic(moved), bound)
                changed = deepcopy(scope)
                changed["sdkHeaders"][0]["key"] = "b" * 64
                self.assertNotEqual(c_sdk.semantic(changed), initial)
                header.write_bytes(b"changed original")
                with self.assertRaisesRegex(
                    ValueError, "SDK source/provider identity changed"
                ):
                    c_sdk.semantic(scope)
            with (
                patch.object(c_sdk, "msys_headers", return_value={}),
                self.assertRaisesRegex(
                    ValueError, "SDK source/provider identity changed"
                ),
            ):
                c_sdk.semantic(scope)
