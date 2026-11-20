from __future__ import annotations

import hashlib
import json
import os
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from autosar_tooling import c_check, c_disposition, c_fingerprint, c_gate, ci_reporting

HEADER = "#define VALUE 1\ntypedef unsigned int Count;\n"
SOURCE = (
    '#include "types.h"\n'
    "const char *source_path = __FILE__;\n"
    "Count read_count(void) { return VALUE + 2; }\n"
)
CHECKER_SOURCE = """\
#ifdef __CPPCHECK__
typedef unsigned char x;
#define ARG(x) (x)
int checker_branch(void) { return ARG(266); }
#endif
int read_count(void) { int count; return count; }
"""
CHANGED_CHECKER_SOURCE = CHECKER_SOURCE.replace("ARG(x) (x)", "ARG (x) (x)")


class FingerprintTests(unittest.TestCase):
    def test_header_input_directives_follow_comment_removal_and_line_splicing(self):
        visible = '/* leading */ # /* keyword */ inc\\\nlude "real.h"'
        source = (
            'const char *text = "/* comment */ #include";\n'
            '// continued comment \\\n#include "ignored.h"\n'
            'int value; /* spans\n */ #include "also-ignored.h"\n'
            f'{visible}\n'
            'int trailing;\n'
        )
        self.assertEqual(c_fingerprint.analyzer_header_inputs(source, True), {
            "prefix": source[:source.index("\nint trailing;")],
            "directives": [visible],
        })

    def test_windows_analyzer_contract_refuses_tampering_and_cross_image_substitution(self):
        with tempfile.TemporaryDirectory() as directory:
            executable = Path(directory) / "cppcheck.exe"
            binaries = {"older": b"older PE", "newer": b"newer PE"}
            expected = {image: hashlib.sha256(data).hexdigest() for image, data in binaries.items()}
            with patch.object(c_disposition, "WINDOWS_ANALYZERS", expected):
                for image, binary in binaries.items():
                    executable.write_bytes(binary)
                    self.assertEqual(c_disposition.verify_windows_analyzer(executable, image), expected[image])
                    with self.assertRaisesRegex(ValueError, "needs assessment"):
                        c_disposition.verify_windows_analyzer(executable, "unknown")
                    with self.assertRaisesRegex(ValueError, "needs assessment"):
                        c_disposition.verify_windows_analyzer(executable, next(iter(expected.keys() - {image})))
                    executable.write_bytes(binary + b"\x00")
                    with self.assertRaisesRegex(ValueError, "needs assessment"):
                        c_disposition.verify_windows_analyzer(executable, image)

    @staticmethod
    def inventory(summary, output, *, driver="a" * 64, backend="b" * 64):
        return c_fingerprint.inventory(summary, output, {"driver": driver, "backend": backend})

    def scan(self, output: Path, *, header: str = HEADER, source: str = SOURCE,
             other: str = "int other(void) { return 0; }\n",
             flags: tuple[str, ...] = ("-std=c99",), revision: str = "pinned",
             duplicate: bool = False, rule: str = "misra-c2012-10.3") -> dict:
        compiler = os.environ.get("AUTOSAR_CC") or shutil.which("gcc")
        if compiler is None:
            self.skipTest("target GCC unavailable")
        project = output / "project"
        project.mkdir(parents=True, exist_ok=True)
        units = ["source.c", "other.c"]
        dependencies = {"source.c": ["source.c", "types.h"], "other.c": ["other.c"]}
        for name, text in (("source.c", source), ("types.h", header), ("other.c", other)):
            (project / name).write_text(text, encoding="utf-8")
        for index, unit in enumerate(units):
            output_text = subprocess.check_output(
                [compiler, *flags, "-E", "-dD", (project / unit).as_posix()], encoding="utf-8",
            )
            (output / f"compiler-origins-{index}.stdout.log").write_text(
                output_text, encoding="utf-8",
            )
        line = next(index for index, text in enumerate(source.splitlines(), 1) if "return" in text)
        diagnostic = {
            "id": rule, "severity": "style", "programs": ["host-batch", "probe"],
            "message": "test diagnostic",
            "locations": [{"file": "source.c", "line": str(line)}],
        }
        summary = {
            "passed": False, "error": None, "coverage": {"completeCompliance": False},
            "scope": {
                "translationUnits": units, "target": "test-target",
                "projectTranslationUnits": dependencies,
                # The target compiler reads a type header outside the analyzer's inputs.
                "cppcheckInputs": {unit: {"project": [unit], "sdk": []} for unit in units},
                "sdkTranslationUnits": {unit: [] for unit in units}, "sdkHeaders": [],
                "compilerFlags": list(flags), "includePaths": [],
            },
            "tool": {"version": "test", "expectedSourceRevision": revision,
                     "binarySha256": "c" * 64, "addons": {}},
            "semanticIdentity": {
                "compiler": {
                    "machine": "test", "release": "test", "predefines": [], "builtins": [],
                },
                "models": {}, "libraries": {}, "addonRuntime": {},
                "analysisCode": {"c_addon.py": "test", "c_check.py": "test"},
                "defines": {unit: [] for unit in units},
            },
            "programs": [{"name": program, "translationUnits": units}
                         for program in diagnostic["programs"]],
            "diagnostics": [diagnostic] * (2 if duplicate else 1),
        }
        (output / "summary.json").write_text(json.dumps(summary), encoding="utf-8")
        return summary

    def test_target_tokens_and_configuration_invalidate_only_relevant_fingerprints(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            output = root / "baseline"
            summary = self.scan(output)
            before = self.inventory(summary, output)
            approved = {row["fingerprint"] for row in before}
            self.assertEqual(len(approved), 2)
            self.assertNotIn(None, approved)
            cases = (
                ("nearby", {"source": SOURCE.replace("+ 2", "+ 3")}, "blocked"),
                ("macro", {"header": HEADER.replace("VALUE 1", "VALUE 3")}, "blocked"),
                ("typedef", {"header": HEADER.replace("unsigned int", "unsigned long")}, "blocked"),
                ("options", {"flags": ("-std=c99", "-fno-strict-aliasing")}, "blocked"),
                ("version", {"revision": "changed"}, "blocked"),
                ("unrelated", {"other": "int other(void) { return 55; }\n"}, "passed"),
                ("moved", {"source": "\n\n" + SOURCE}, "passed"),
            )
            for name, changes, status in cases:
                with self.subTest(name=name):
                    candidate = root / name
                    current = self.inventory(self.scan(candidate, **changes), candidate)
                    result = c_fingerprint.verdict(current, before, approved)
                    self.assertEqual(result["status"], status)
                    if status == "blocked":
                        self.assertEqual(set(result["staleDispositions"]), approved)
                        self.assertEqual(len(result["unacceptedNew"]), 2)

    def test_cppcheck_only_macro_spelling_changes_the_fingerprint(self):
        texts = CHECKER_SOURCE, CHANGED_CHECKER_SOURCE, "\n\n" + CHECKER_SOURCE
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            fingerprints = []
            gcc_hashes = []
            for index, text in enumerate(texts):
                output = root / str(index)
                summary = self.scan(output, source=text, rule="uninitvar")
                line = next(i for i, line in enumerate(text.splitlines(), 1) if "return count" in line)
                summary["diagnostics"][0]["locations"][0]["line"] = str(line)
                fingerprints.append(self.inventory(summary, output))
                compiler = (output / "compiler-origins-0.stdout.log").read_text(encoding="utf-8")
                gcc_hashes.append(c_fingerprint.compiler_context(compiler, "source.c", {}, {})[0])
            self.assertEqual(gcc_hashes, [gcc_hashes[0]] * 3)
            approved = {row["fingerprint"] for row in fingerprints[0]}
            result = c_fingerprint.verdict(fingerprints[1], fingerprints[0], approved)
            self.assertEqual(result["status"], "blocked")
            self.assertEqual(set(result["staleDispositions"]), approved)
            self.assertEqual(fingerprints[0], fingerprints[2])

    def test_pinned_cppcheck_preprocesses_macro_spelling_differently(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            try:
                executable, _, _ = c_check.tool_identity(root)
            except (OSError, RuntimeError, ValueError) as error:
                if os.environ.get("CPPCHECK"):
                    self.fail(f"Configured CPPCHECK is unusable: {error}")
                self.skipTest(f"Pinned Cppcheck unavailable: {error}")
            source = root / "source.c"
            for text, expected in ((CHECKER_SOURCE, "return(266);"),
                                   (CHANGED_CHECKER_SOURCE, "return(x)(x)(266);")):
                source.write_text(text, encoding="utf-8")
                checked = subprocess.check_output(
                    [str(executable), "--quiet", "-E", str(source)], encoding="utf-8", timeout=30,
                )
                self.assertIn(expected, "".join(checked.split()))

    def test_cppcheck_only_line_macro_keeps_physical_lines_in_identity(self):
        source = (
            "#ifdef __CPPCHECK__\n"
            "const int checker_line = __LINE__;\n"
            "#endif\n"
            "int read_count(void) { int count; return count; }\n"
        )
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            previous, current = root / "previous", root / "current"
            before = self.inventory(self.scan(previous, source=source), previous)
            after = self.inventory(self.scan(current, source="\n" + source), current)
            approved = {row["fingerprint"] for row in before}
            result = c_fingerprint.verdict(after, before, approved)
            self.assertEqual(set(result["staleDispositions"]), approved)
            self.assertEqual(len(result["unacceptedNew"]), 2)

    def test_actual_binary_digests_bind_same_version_tooling(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            summary = self.scan(root)
            before = self.inventory(summary, root)
            different = {**summary, "tool": {**summary["tool"], "binarySha256": "d" * 64}}
            variants = (self.inventory(different, root), self.inventory(summary, root, driver="d" * 64),
                        self.inventory(summary, root, backend="d" * 64))
            approved = {row["fingerprint"] for row in before}
            for label, rows in zip(("Cppcheck", "GCC", "cc1"), variants):
                with self.subTest(binary=label):
                    result = c_fingerprint.verdict(rows, before, approved)
                    self.assertEqual(set(result["staleDispositions"]), approved)
                    self.assertEqual(len(result["unacceptedNew"]), 2)

    def test_primary_origin_rejects_a_clang_line_one_marker(self):
        for root, windows in (("/private/var/tmp/project", False), ("C:/Checkout", True)):
            with self.subTest(root=root):
                source = f'"{root}/source.c"'
                gcc = f'# 0 {source}\n# 1 {source}\nint value;\n'
                clang = f'# 1 {source}\n# 1 "<built-in>" 1\n# 1 {source} 2\nint value;\n'
                wanted = {"source.c": {1}}
                _, anchors = c_fingerprint.compiler_context(gcc, "source.c", {}, wanted, windows=windows)
                self.assertEqual(anchors, {("source.c", 1): [["int", "value", ";"]]})
                with self.assertRaisesRegex(ValueError, r"^Primary compiler-origin marker differs: source\.c$"):
                    c_fingerprint.compiler_context(clang, "source.c", {}, wanted, windows=windows)

    def test_windows_sdk_and_checkout_markers_ignore_case(self):
        original = (
            '# 0 "C:/Checkout/Source.c"\n# 1 "C:/msys64/mingw64/include/Header.h"\n'
            'typedef int Count;\n# 1 "C:/Checkout/Source.c"\nCount n;\n'
        )
        alias = original.replace("C:/msys64", "c:/MSYS64").replace("Header.h", "header.H")
        sdk_name = "C:/msys64/mingw64/include/Header.h"
        sdk = {c_fingerprint.path_key(sdk_name, True): "sdk:header"}
        wanted = {"sdk:header": {1}, "Source.c": {1}}
        paths = {"source.c": "Source.c"}
        before = c_fingerprint.compiler_context(
            original, "Source.c", sdk, wanted, windows=True, project_paths=paths,
        )
        after = c_fingerprint.compiler_context(
            alias, "Source.c", sdk, wanted, windows=True, project_paths=paths,
        )
        self.assertEqual(before, after)
        self.assertEqual(set(before[1]), {("sdk:header", 1), ("Source.c", 1)})
        checkout_alias = alias.replace("C:/Checkout/Source.c", "c:/CHECKOUT/source.C")
        self.assertEqual(before, c_fingerprint.compiler_context(
            checkout_alias, "Source.c", sdk, wanted, windows=True, project_paths=paths,
        ))
        self.assertNotEqual(c_fingerprint.compiler_context(original, "Source.c", {}, {})[0],
                            c_fingerprint.compiler_context(alias, "Source.c", {}, {})[0])

    def test_windows_sdk_spellings_remain_distinct_and_collisions_are_refused(self):
        paths = ("C:/msys64/include/Straße.h", "C:/msys64/include/Strasse.h")
        sdk = c_fingerprint.path_index(zip(paths, ("sdk:eszett", "sdk:double-s")), True)
        snapshots = []
        for path, identity in zip(paths, ("sdk:eszett", "sdk:double-s")):
            log = f'# 0 "C:/Checkout/Source.c"\n# 1 "{path}"\nint value;\n'
            snapshot = c_fingerprint.compiler_context(
                log, "Source.c", sdk, {identity: {1}}, windows=True,
            )
            self.assertEqual(snapshot[1], {(identity, 1): [["int", "value", ";"]]})
            snapshots.append(snapshot[0])
        self.assertNotEqual(*snapshots)

        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            summary = self.scan(root)
            scope = summary["scope"]
            scope["target"] = "windows-x64-controlled-v1"
            scope["sdkHeaders"] = [
                {"path": "C:/MSYS64/Include/Example.h", "key": "first"},
                {"path": "c:/msys64/include/example.h", "key": "second"},
            ]
            with self.assertRaisesRegex(ValueError, "Ambiguous registered fingerprint path"):
                self.inventory(summary, root)
            scope["sdkHeaders"] = []
            scope["cppcheckInputs"]["source.c"]["project"].append("Source.C")
            with self.assertRaisesRegex(ValueError, "Ambiguous registered fingerprint path"):
                self.inventory(summary, root)

    def test_ctu_disposition_survives_two_independent_module_merges(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            compiler = os.environ.get("AUTOSAR_CC") or shutil.which("gcc")
            if compiler is None:
                self.skipTest("target GCC unavailable")
            try:
                executable, addons, tool = c_check.tool_identity(root)
            except (OSError, RuntimeError, ValueError) as error:
                if os.environ.get("CPPCHECK"):
                    self.fail(f"Configured CPPCHECK is unusable: {error}")
                self.skipTest(f"Pinned Cppcheck unavailable: {error}")

            repository = root / "source"
            repository.mkdir()

            def git(*args):
                return subprocess.check_output(
                    ["git", "-c", "user.name=Test", "-c", "user.email=test@example.com", *args],
                    cwd=repository, stderr=subprocess.STDOUT, encoding="utf-8", timeout=30,
                ).strip()

            def write(name, value):
                (repository / name).write_text(value, encoding="utf-8", newline="\n")

            def scan(label, *, reorder=False):
                output = root / label
                project = output / "project"
                project.mkdir(parents=True)
                units = ["entry.c", "first.c", "second.c"]
                scope_inputs = {name: [name, "shared.h"] for name in units}
                for name in [*units, "shared.h"]:
                    shutil.copy2(repository / name, project / name)
                commands = [{"directory": str(project), "file": str(project / name),
                             "arguments": [compiler, "-std=c99", "-c", str(project / name)]}
                            for name in units]
                program = output / "host-batch"
                program.mkdir()
                (program / "compile_commands.json").write_text(json.dumps(commands), encoding="utf-8")
                (program / "misra.json").write_text(
                    json.dumps({"script": str(addons / "misra.py"), "ctu": True}), encoding="utf-8",
                )
                (program / "cache").mkdir()
                analysis = subprocess.run([
                    str(executable), "--std=c99", "--enable=style", "--xml", "--xml-version=2",
                    "--project=compile_commands.json", "--cppcheck-build-dir=cache", "--addon=misra.json",
                ], cwd=program, encoding="utf-8", capture_output=True, check=True, timeout=60)
                diagnostics = [row for row in c_check.parse_diagnostics(analysis.stderr, project)
                               if row["id"] != "checkersReport"]
                for row in diagnostics:
                    row["programs"] = ["host-batch"]
                for index, name in enumerate(units):
                    preprocessed = subprocess.check_output(
                        [compiler, "-std=c99", "-E", "-dD", str(project / name)],
                        encoding="utf-8", timeout=30,
                    )
                    (output / f"compiler-origins-{index}.stdout.log").write_text(
                        preprocessed, encoding="utf-8",
                    )
                summary = {
                    "scope": {
                        "translationUnits": units, "target": "test-target",
                        "projectTranslationUnits": scope_inputs,
                        "cppcheckInputs": {unit: {"project": names, "sdk": []}
                                           for unit, names in scope_inputs.items()},
                        "sdkTranslationUnits": {name: [] for name in units}, "sdkHeaders": [],
                        "compilerFlags": ["-std=c99"], "includePaths": [],
                    },
                    "tool": tool,
                    "semanticIdentity": {
                        "compiler": {"machine": "test", "release": "test", "predefines": [], "builtins": []},
                        "models": {}, "libraries": {}, "addonRuntime": {},
                        "analysisCode": {"c_addon.py": "test", "c_check.py": "test"},
                        "defines": {name: [] for name in units},
                    },
                    "programs": [{"name": "host-batch", "translationUnits": units}],
                    "diagnostics": diagnostics,
                }
                if reorder:
                    rows = self.inventory(summary, output)
                    files = program / "cache/files.txt"
                    files.write_text("\n".join(reversed(files.read_text().splitlines())) + "\n")
                    for facts in (program / "cache").glob("*.ctu-info"):
                        facts.write_text("\n".join(reversed(facts.read_text().splitlines())) + "\n")
                    self.assertEqual(self.inventory(summary, output), rows)
                return diagnostics, self.inventory(summary, output)

            shared = "#define UNUSED 7\nint first(void);\nint second(void);\n"
            first = '#include "shared.h"\nint first(void) { return 1; }\n'
            second = '#include "shared.h"\nint second(void) { return 2; }\n'
            entry = '#include "shared.h"\nint main(void) { return first() + second(); }\n'
            git("init", "-b", "main")
            for name, source in (("shared.h", shared), ("first.c", first),
                                 ("second.c", second), ("entry.c", entry)):
                write(name, source)
            git("add", ".")
            git("commit", "-m", "base")
            initial = git("rev-parse", "HEAD")
            raw, base = scan("base")
            self.assertEqual([row["id"] for row in raw], ["misra-c2012-2.5"])
            approved = {base[0]["fingerprint"]}
            self.assertNotIn(None, approved)
            branches = {}
            for name, source in (("first", first.replace("return 1", "return 11")),
                                 ("second", second.replace("return 2", "return 22"))):
                git("switch", "-c", name, initial)
                write(name + ".c", source)
                git("commit", "-am", f"change {name}")
                _, branches[name] = scan(name)
                self.assertEqual(c_fingerprint.verdict(branches[name], base, approved)["status"], "passed")
            git("switch", "main")
            git("merge", "--no-ff", "first", "-m", "merge first")
            git("merge", "--no-ff", "second", "-m", "merge second")
            _, merged = scan("merged")
            self.assertEqual(c_fingerprint.verdict(merged, branches["first"], approved)["status"], "passed")
            self.assertEqual(merged, base)

            write("shared.h", shared.replace("UNUSED 7", "UNUSED 8"))
            _, changed_definition = scan("changed-definition")
            self.assertEqual(c_fingerprint.verdict(changed_definition, merged, approved)["staleDispositions"],
                             sorted(approved))
            write("shared.h", shared)
            write("second.c", second.replace("return 2", "return UNUSED"))
            _, used_in_other_unit = scan("used-in-other-unit")
            self.assertEqual(c_fingerprint.verdict(used_in_other_unit, merged, approved)["staleDispositions"],
                             sorted(approved))

            write("entry.c", entry.replace("first() + second()", "first()"))
            write("second.c", second)
            raw, _ = scan("r87-fact-order", reorder=True)
            self.assertIn("misra-c2012-8.7", {row["id"] for row in raw})

            write("entry.c", entry)
            write("shared.h", shared + "#ifndef TYPE\n#define TYPE unsigned int\n#endif\ntypedef TYPE Count;\n")
            prelude = "#ifdef __CPPCHECK__\n/* checker only */ #define TYPE unsigned int\n#endif\n"
            consumer = "/* shared types */ " + second
            write("second.c", prelude + consumer)
            raw_before, rows_before = scan("cppcheck-type-before")
            write("second.c", prelude.replace("unsigned int", "unsigned long") + consumer)
            raw_after, rows_after = scan("cppcheck-type-after")
            self.assertEqual(raw_before, raw_after)
            typedefs = {index for index, row in enumerate(raw_before) if row["id"] == "misra-c2012-2.3"}
            self.assertTrue(typedefs)
            self.assertNotEqual([row for row in rows_before if row["diagnostic"] in typedefs],
                                [row for row in rows_after if row["diagnostic"] in typedefs])
            emitted, checked = [], []
            for label in ("cppcheck-type-before", "cppcheck-type-after"):
                output = root / label
                compiler_trace = (output / "compiler-origins-2.stdout.log").read_text()
                emitted.append(c_fingerprint.compiler_context(compiler_trace, "second.c", {}, {})[0])
                source = output / "project/second.c"
                result = subprocess.check_output([str(executable), "--quiet", "--force", "-E", str(source)],
                                                 encoding="utf-8", timeout=30)
                checked.append("".join(result.split()))
            self.assertEqual(*emitted)
            self.assertIn("typedefunsignedintCount;", checked[0])
            self.assertIn("typedefunsignedlongCount;", checked[1])

    def test_duplicate_occurrences_never_inherit_a_disposition(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            rows = self.inventory(self.scan(root, duplicate=True), root)
            self.assertEqual(len(rows), 4)
            self.assertTrue(all(row["fingerprint"] is None and row["reason"] == "ambiguous_context"
                                for row in rows))
            self.assertEqual(c_fingerprint.verdict(rows, [], set())["status"], "incomplete")

    def test_sdk_anchor_requires_the_analyzed_header_bytes(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            header = root / "sdk.h"
            header.write_bytes(b"#define EXTERNAL 1\n")
            summary = self.scan(root, flags=("-std=c99", "-include", header.as_posix()))
            scope = summary["scope"]
            scope["sdkHeaders"] = [{
                "path": header.as_posix(), "key": "provider-header",
                "sha256": hashlib.sha256(header.read_bytes()).hexdigest(),
            }]
            scope["sdkTranslationUnits"] = {unit: [header.as_posix()]
                                            for unit in scope["translationUnits"]}
            summary["diagnostics"][0]["locations"] = [{"file": header.as_posix(), "line": "1"}]
            bound = self.inventory(summary, root)
            self.assertTrue(all(row["fingerprint"] is not None for row in bound))
            header.write_bytes(b"#define EXTERNAL 2\n")
            refused = self.inventory(summary, root)
            self.assertTrue(all(row["reason"] == "missing_context" for row in refused))

    def test_transferred_receipt_requires_exact_summary_and_occurrence_coverage(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            summary = self.scan(root)
            inventory = self.inventory(summary, root)
            receipt_sha = c_fingerprint.write_receipt(root, inventory)
            path = root / c_fingerprint.RECEIPT
            receipt = json.loads(path.read_text(encoding="utf-8"))
            rows = c_fingerprint.read_receipt(root, summary, receipt_sha)
            self.assertEqual(len(rows), 2)
            path.write_text(json.dumps({**receipt, "occurrences": rows[:-1]}), encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "differs from the raw analysis manifest"):
                c_fingerprint.read_receipt(root, summary, receipt_sha)
            with self.assertRaisesRegex(ValueError, "coverage differs"):
                changed_sha = hashlib.sha256(path.read_bytes()).hexdigest()
                c_fingerprint.read_receipt(root, summary, changed_sha)
            receipt_sha = c_fingerprint.write_receipt(root, inventory)
            with (root / "summary.json").open("a", encoding="utf-8") as stream:
                stream.write("\n")
            with self.assertRaisesRegex(ValueError, "does not bind"):
                c_fingerprint.read_receipt(root, summary, receipt_sha)

    def test_explicit_disposition_and_removal(self):
        row = {"fingerprint": "a" * 64}
        approval = {row["fingerprint"]}
        self.assertEqual(c_fingerprint.verdict([row], [], set())["status"], "blocked")
        self.assertEqual(c_fingerprint.verdict([row], [], approval)["status"], "passed")
        result = c_fingerprint.verdict([], [row], approval)
        self.assertEqual(result["status"], "blocked")
        self.assertEqual(result["staleDispositions"], sorted(approval))
        self.assertEqual(c_fingerprint.verdict([], [row], set())["status"], "passed")

    def test_disposition_fingerprint_requires_current_technical_and_formal_proof(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory).resolve()
            summary = self.scan(output)
            rows = self.inventory(summary, output)
        summary["coverage"]["rules"] = [{"id": "R10.3", "category": "Required"}]
        occurrence = {
            "diagnostic": 0, "locationIndex": 0, "location": summary["diagnostics"][0]["locations"][0],
            "programs": ["host-batch", "probe"], "rule": "misra-c2012-10.3",
            "entity": "count", "record": {"file": "types.md", "text": "conversion"},
            "assessment": "accepted", "status": "owned_record_matched", "reason": "owned_source_approved",
            "contextId": "owned", "formalContextId": "formal",
        }
        authorities = {
            key: {"id": key, "source": "types.md", "evidence": {}, "records": {}, "formalStatus": "approved"}
            for key in ("owned", "formal")
        }
        proof = c_disposition.proof(occurrence, authorities, set())
        self.assertIsNotNone(proof)
        bindings = {row["fingerprint"]: proof for row in rows}

        def compare(current):
            result = {
                "occurrences": [dict(current)], "objects": [],
                "checkerObligations": {"retainedRules": {"R10.3": {"approval": "formal"}},
                                       "statusCounts": {"retained": 1},
                                       "conformanceAssessment": "compliant_with_deviations"},
            }
            c_disposition.apply(summary, result, rows, bindings, authorities)
            return result

        self.assertEqual(compare(occurrence)["occurrences"], [occurrence])
        for change in ("entity", "approval", "category"):
            with self.subTest(change=change):
                current = occurrence
                if change == "entity":
                    current = {**occurrence, "entity": "another_count"}
                if change == "approval":
                    authorities["formal"]["formalStatus"] = "pending"
                if change == "category":
                    authorities["formal"]["formalStatus"] = "approved"
                    summary["coverage"]["rules"][0]["category"] = "Mandatory"
                result = compare(current)
                self.assertEqual(result["occurrences"][0]["assessment"], "blocked")
                self.assertEqual(result["fingerprintDispositions"]["stale"], sorted(bindings))
                self.assertEqual(result["checkerObligations"]["retainedRules"], {})
                self.assertEqual(result["checkerObligations"]["statusCounts"], {"not_assessed": 1})

    def test_preview_uses_scope_and_excludes_mandatory_retention(self):
        summary = {
            "scope": {},
            "coverage": {"rules": [{"id": "R17.3", "category": "Mandatory"}]},
            "diagnostics": [
                {"id": "misra-c2012-17.3", "locations": [{"file": "source.c"}]},
                {"id": "misra-c2012-10.4", "locations": [{"file": "source.c"}]},
                {"id": "misra-c2012-10.4", "locations": [{"file": "integration/host.c"}]},
            ],
            "projectAcceptance": {"occurrences": [
                {"diagnostic": index, "locationIndex": 0, "assessment": "accepted"}
                for index in range(3)
            ]},
        }
        current = [
            {"diagnostic": index, "locationIndex": 0, "fingerprint": c_fingerprint.sha(index)}
            for index in range(3)
        ]
        with patch.object(c_gate.c_comparison, "matrix_headers", return_value=set()):
            preview = c_gate.fingerprint_comparison(
                {"current": summary, "baseline": summary},
                {"current": current, "baseline": []}, {"current": {}, "baseline": {}},
            )
        self.assertEqual(preview["status"], "blocked")
        self.assertEqual(preview["candidateDispositions"], [current[1]["fingerprint"]])
        self.assertEqual(preview["unacceptedNew"], [current[0]["fingerprint"]])

    def test_sarif_uses_receipt_identities_per_program(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            output = root / "results" / "sample"
            summary = self.scan(output)
            rows = self.inventory(summary, output)
            baseline = output / "baseline-summary.json"
            current_bytes = (output / "summary.json").read_bytes()
            baseline.write_bytes(current_bytes)
            sha = hashlib.sha256(current_bytes).hexdigest()
            gate = {
                "summarySha256": {"current": sha, "baseline": sha},
                "diagnosticFingerprints": {
                    "format": c_fingerprint.FORMAT, "current": rows,
                    "baseline": [rows[0], {**rows[1], "fingerprint": "d" * 64}],
                },
            }
            gate_path = output / "gate.json"
            gate_path.write_text(json.dumps(gate), encoding="utf-8")
            result = ci_reporting.sarif(ci_reporting.cppcheck(output.parent, root), "Cppcheck")
            self.assertEqual([run["automationDetails"]["id"] for run in result["runs"]],
                             ["cppcheck-test-target-sample/",
                              "cppcheck-test-target-sample-probe/"])
            reports = [entry for run in result["runs"] for entry in run["results"]]
            self.assertEqual([item["baselineState"] for item in reports], ["unchanged", "new"])
            self.assertEqual([item["partialFingerprints"] for item in reports],
                             [{c_fingerprint.SARIF_KEY: row["fingerprint"]} for row in rows])

            gate["diagnosticFingerprints"]["baseline"][1].update(
                fingerprint=None, reason="missing_context",
            )
            gate_path.write_text(json.dumps(gate), encoding="utf-8")
            mixed = ci_reporting.sarif(ci_reporting.cppcheck(output.parent, root), "Cppcheck")
            self.assertEqual(mixed["runs"][0]["results"][0]["baselineState"], "unchanged")
            probe, = mixed["runs"][1]["results"]
            self.assertNotIn("baselineState", probe)
            self.assertIn("partialFingerprints", probe)

    def test_sarif_refuses_stale_or_incomplete_gate_metadata(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            output = root / "results/sample"
            summary = self.scan(output)
            rows = self.inventory(summary, output)
            current = output / "summary.json"
            baseline = output / "baseline-summary.json"
            current_bytes = current.read_bytes()
            sha = hashlib.sha256(current_bytes).hexdigest()
            gate = output / "gate.json"
            for failure in ("current", "baseline", "coverage"):
                with self.subTest(failure=failure):
                    current.write_bytes(current_bytes)
                    baseline.write_bytes(current_bytes)
                    gate.write_text(json.dumps({
                        "summarySha256": {"current": sha, "baseline": sha},
                        "diagnosticFingerprints": {
                            "format": c_fingerprint.FORMAT, "current": rows,
                            "baseline": rows[:-1] if failure == "coverage" else rows,
                        },
                    }), encoding="utf-8")
                    if failure in {"current", "baseline"}:
                        path = current if failure == "current" else baseline
                        path.write_bytes(current_bytes.replace(b"misra-c2012-10.3", b"uninitvar"))
                    actual = ci_reporting.cppcheck(output.parent, root)
                    self.assertEqual(len(actual), 2)
                    self.assertTrue(all("partialFingerprints" not in row for row in actual))
                    self.assertTrue(all("baselineState" not in row for row in actual))
                    if failure == "current":
                        self.assertEqual({row["rule"] for row in actual}, {"uninitvar"})
