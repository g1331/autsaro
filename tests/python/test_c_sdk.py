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

    def test_make_dependencies_preserve_real_windows_paths_and_escape_spaces(self):
        text = (
            r"autsaro-tu: C:\Users\runneradmin\AppData\Local\Temp\project\main.c "
            r"D:/a/_temp/msys64/mingw64/include/winbase.h C\:/Program\ Files/sdk.h "
            "\\\n" + r" /opt/provider\ space/header.h /opt/hash\#and$$name.h"
        )
        self.assertEqual(
            c_sdk.dependency_paths(text),
            [
                r"C:\Users\runneradmin\AppData\Local\Temp\project\main.c",
                "D:/a/_temp/msys64/mingw64/include/winbase.h",
                "C:/Program Files/sdk.h",
                "/opt/provider space/header.h",
                "/opt/hash#and$name.h",
            ],
        )

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
                    "unresolved": False,
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

    def test_transitive_project_macro_competitors_fall_back_but_real_wait_stays_precise(
        self,
    ):
        with tempfile.TemporaryDirectory() as temporary:
            root, header, _packages, record = self.fixture(Path(temporary).resolve())
            source = root / "kernel/port.c"
            source.parent.mkdir()
            cases = [
                (
                    "#define OWN_LITERAL 0xffffffff\n#define OWN_ALIAS OWN_LITERAL\n",
                    "consume(INFINITE, OWN_ALIAS);",
                    "OWN_ALIAS",
                    "unclassified",
                ),
                (
                    "#define OWN_ALIAS ((MISSING_LITERAL))\n",
                    "consume(INFINITE, OWN_ALIAS);",
                    "OWN_ALIAS",
                    "unclassified",
                ),
                (
                    "#define OWN_ALIAS MISSING_LITERAL\n",
                    "consume(INFINITE, OWN_ALIAS);",
                    "OWN_ALIAS",
                    "unclassified",
                ),
                (
                    "#define OWN_ALIAS OWN_CYCLE\n#define OWN_CYCLE OWN_ALIAS\n",
                    "consume(INFINITE, OWN_ALIAS);",
                    "OWN_ALIAS",
                    "unclassified",
                ),
                (
                    "#define OWN_LITERAL 3U\n#define OWN_ALIAS OWN_LITERAL\n",
                    "consume(INFINITE, OWN_ALIAS);",
                    "INFINITE",
                    "sdk",
                ),
                (
                    "#define configASSERT(x) if (!(x)) Os_HostAssertFailed(__FILE__, __LINE__)\n",
                    "configASSERT( WaitForSingleObject( xObject, INFINITE ) == 0U );",
                    "configASSERT",
                    "sdk",
                ),
            ]
            for definitions, use, operand, expected in cases:
                with self.subTest(use=use, definitions=definitions):
                    source.write_text("\n" * 53 + use + "\n")
                    text = (
                        f'# 1 "{header.as_posix()}" 1\n#define INFINITE 0xffffffff\n'
                        f'# 1 "{source.as_posix()}" 2\n'
                        + definitions
                        + f'# 54 "{source.as_posix()}"\nexpanded;\n'
                    )
                    inputs = c_sdk.preprocessing(
                        text, root, {"INFINITE", "OWN_ALIAS", "configASSERT"}
                    )
                    scope = {
                        "sdkHeaders": [record],
                        "sdkMacroInputs": {"kernel/port.c": inputs},
                    }
                    raw = self.diagnostic()
                    raw["locations"][0]["column"] = str(use.index(operand) + 1)
                    rows = [deepcopy(raw)]
                    token = {
                        "file": "kernel/port.c",
                        "line": 54,
                        "column": use.index(operand) + 1,
                        "literal": "0xffffffff",
                    }
                    result = c_sdk.classify(rows, scope, [token])
                    self.assertEqual(rows[0]["sourceCategory"], expected, rows)
                    self.assertEqual(result["complete"], expected == "sdk")
                    self.assertEqual(
                        {
                            k: v
                            for k, v in rows[0].items()
                            if k not in {"sourceOrigins", "sourceCategory"}
                        },
                        raw,
                    )
                    if "OWN_LITERAL" in definitions:
                        self.assertIn("OWN_LITERAL", inputs["kernel/port.c:54"])
                    if expected == "unclassified":
                        self.assertFalse(
                            any(
                                origin["kind"] == "unsigned-literal"
                                for origin in rows[0]["sourceOrigins"]
                            )
                        )

    def test_verified_compiler_predefines_are_separate_from_sdk_and_project_macros(
        self,
    ):
        with tempfile.TemporaryDirectory() as temporary:
            root, _header, _packages, record = self.fixture(Path(temporary).resolve())
            source = root / "kernel/port.c"
            source.parent.mkdir()
            source.write_text("\n" * 53 + "store(cell, value, __ATOMIC_SEQ_CST);\n")
            text = (
                '# 0 "<built-in>"\n#define __ATOMIC_SEQ_CST 5\n'
                f'# 54 "{source.as_posix()}"\nstore(cell, value, 5);\n'
            )
            inputs = c_sdk.preprocessing(text, root, {"__ATOMIC_SEQ_CST"})
            scope = {
                "target": TARGET,
                "sdkHeaders": [record],
                "sdkMacroInputs": {"kernel/port.c": inputs},
                "compilerDiscoveredDefines": {
                    "kernel/port.c": ["-D__ATOMIC_SEQ_CST=5"]
                },
                "compilerSemanticInputs": {
                    "machine": "x86_64-w64-mingw32",
                    "release": "16.1.0",
                    "predefines": ["#define __ATOMIC_SEQ_CST 5"],
                },
            }
            rows = [self.diagnostic()]
            rows[0]["id"] = "misra-c2012-17.3"
            raw = deepcopy(rows[0])
            result = c_sdk.classify(rows, scope, [], root)
            self.assertTrue(result["complete"])
            self.assertEqual(result["sdkDiagnosticKeys"], 0)
            self.assertEqual(result["compilerPredefineDiagnosticKeys"], 1)
            self.assertEqual(rows[0]["sourceCategory"], "project-patch")
            self.assertEqual(
                rows[0]["sourceOrigins"][0]["category"], "compiler-predefine"
            )
            self.assertNotIn("headerKey", rows[0]["sourceOrigins"][0])
            self.assertEqual(
                {
                    k: v
                    for k, v in rows[0].items()
                    if k not in {"sourceOrigins", "sourceCategory"}
                },
                raw,
            )
            # Neither a familiar prefix nor an identical project definition is evidence.
            for definitions, origin in (
                ([], "<built-in>"),
                (["#define __ATOMIC_SEQ_CST 7"], "<built-in>"),
                (["#define __ATOMIC_SEQ_CST 5"], "kernel/port.c"),
            ):
                scope["compilerSemanticInputs"]["predefines"] = definitions
                inputs["kernel/port.c:54"]["__ATOMIC_SEQ_CST"]["file"] = origin
                result = c_sdk.classify(rows, scope, [], root)
                self.assertFalse(result["complete"])
                self.assertEqual(result["compilerPredefineDiagnosticKeys"], 0)
                self.assertEqual(result["unclassifiedDiagnosticKeys"], 1)

    def test_macro_replacement_references_are_not_claimed_as_expansions(self):
        with tempfile.TemporaryDirectory() as temporary:
            root, header, _packages, record = self.fixture(Path(temporary).resolve())
            source = root / "kernel/port.c"
            source.parent.mkdir()
            source.write_text("#define LIMIT UINT64_C(1)\n")
            text = (
                f'# 1 "{header.as_posix()}" 1\n#define UINT64_C(c) c ## UL\n'
                f'# 1 "{source.as_posix()}" 2\n#define LIMIT UINT64_C(1)\n'
            )
            inputs = c_sdk.preprocessing(text, root, {"LIMIT", "UINT64_C"})
            self.assertTrue(inputs["kernel/port.c:1"]["UINT64_C"]["reference"])
            self.assertNotIn("LIMIT", inputs["kernel/port.c:1"])
            row = self.diagnostic()
            row.update(id="misra-c2012-2.5", patched=False)
            row["locations"][0].update(line="1", column="0")
            rows = [row]
            result = c_sdk.classify(
                rows,
                {"sdkHeaders": [record], "sdkMacroInputs": {"kernel/port.c": inputs}},
                [],
            )
            self.assertTrue(result["complete"])
            self.assertEqual(rows[0]["sourceCategory"], "project")
            self.assertEqual(rows[0]["sourceOrigins"][0]["kind"], "macro-reference")
            # Reference evidence cannot prove which expanded R7.2 operand caused a report.
            row["id"] = "misra-c2012-7.2"
            token = {
                "file": "kernel/port.c",
                "line": 1,
                "column": 0,
                "literal": "0xffffffff",
            }
            result = c_sdk.classify(
                rows,
                {"sdkHeaders": [record], "sdkMacroInputs": {"kernel/port.c": inputs}},
                [token],
            )
            self.assertFalse(result["complete"])
            self.assertEqual(rows[0]["sourceCategory"], "unclassified")

    def test_integer_operand_proof_excludes_string_char_and_floating_tokens(self):
        self.assertEqual(
            c_sdk.integer_literals("0xffffffff 4294967295U 077L"), {4294967295, 63}
        )
        self.assertEqual(
            c_sdk.integer_literals(
                "\"4294967295\" '1' 4294967295.0 1e+4294967295 0x1p+4294967295"
            ),
            set(),
        )

    def test_multiline_macro_argument_without_operand_origin_is_explicitly_unresolved(
        self,
    ):
        with tempfile.TemporaryDirectory() as temporary:
            root, header, _packages, record = self.fixture(Path(temporary).resolve())
            source = root / "kernel/port.c"
            source.parent.mkdir()
            for use, expected in [
                ("configASSERT(\n    INFINITE == 0U);", "unclassified"),
                ("configASSERT(0xffffffff);", "project-patch"),
            ]:
                source.write_text("\n" * 53 + use + "\n")
                text = (
                    f'# 1 "{header.as_posix()}" 1\n#define INFINITE 0xffffffff\n'
                    f'# 1 "{source.as_posix()}"\n#define configASSERT(x) if (!(x)) stop()\n'
                    f'# 54 "{source.as_posix()}"\nexpanded;\n'
                )
                inputs = c_sdk.preprocessing(text, root, {"configASSERT", "INFINITE"})
                rows = [self.diagnostic()]
                token = {
                    "file": "kernel/port.c",
                    "line": 54,
                    "column": 5,
                    "literal": "0xffffffff",
                }
                result = c_sdk.classify(
                    rows,
                    {
                        "sdkHeaders": [record],
                        "sdkMacroInputs": {"kernel/port.c": inputs},
                    },
                    [token],
                )
                self.assertEqual(rows[0]["sourceCategory"], expected)
                self.assertEqual(result["complete"], expected != "unclassified")
                self.assertEqual(rows[0]["sourceOrigins"], [])

    def test_multiline_replacement_reference_uses_actual_logical_definition(self):
        with tempfile.TemporaryDirectory() as temporary:
            root, header, _packages, record = self.fixture(Path(temporary).resolve())
            source = root / "kernel/port.c"
            source.parent.mkdir()
            source.write_text("#define LIMIT \\\n    UINT64_C(1)\n")
            text = (
                f'# 1 "{header.as_posix()}" 1\n#define UINT64_C(c) c ## UL\n'
                f'# 1 "{source.as_posix()}"\n#define LIMIT UINT64_C(1)\n'
            )
            inputs = c_sdk.preprocessing(text, root, {"LIMIT", "UINT64_C"})
            self.assertIn("UINT64_C", inputs["kernel/port.c:1"])
            row = self.diagnostic()
            row.update(id="misra-c2012-2.5", patched=False)
            row["locations"][0].update(line="1", column="0")
            result = c_sdk.classify(
                [row],
                {"sdkHeaders": [record], "sdkMacroInputs": {"kernel/port.c": inputs}},
                [],
            )
            self.assertTrue(result["complete"])
            self.assertEqual(row["sourceOrigins"][0]["kind"], "macro-reference")

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
            self.assertEqual(
                records[str(Path("/") / "usr/include/pthread.h")]["provider"],
                "libc6-dev",
            )
            self.assertEqual(
                records[str(Path("/") / "usr/include/pthread.h")]["version"], "2.39-1"
            )
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
