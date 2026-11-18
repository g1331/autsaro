"""Source gate proof and mutations for the private names deferred from PR #552."""

import copy
import gzip
import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import test_c_comparison as comparison_tests
from autosar_tooling import c_comparison, c_gate, c_rename, c_standard
from ecu_tools.build import digest

UNIT = "bsw/src/Com.c"
RULES = ("misra-c2012-10.3", "misra-c2012-10.4")
OLD = """typedef unsigned char boolean;
#define FALSE 0
static boolean reloaded_before_main[2u];
static struct { boolean reloaded_before_main; boolean remaining; } sample;
void Com_MainFunction(void)
{
    reloaded_before_main[0u] = FALSE;
    if (reloaded_before_main[0u] == FALSE) {
        sample.reloaded_before_main = 0u;
    }
}
"""
NEW = OLD.replace("boolean reloaded_before_main[", "boolean com_reloaded_before_main[").replace(
    "reloaded_before_main[0u]", "com_reloaded_before_main[0u]",
)


def positions(source, spelling):
    return [(source.count("\n", 0, token.start) + 1, token.start - source.rfind("\n", 0, token.start))
            for token in c_standard.lex(source) if token.text == spelling]


class IdentifierRenameTests(unittest.TestCase):
    def check(self, directory, old, new, mutation="", *, crlf=False):
        fixture = comparison_tests.CComparisonTests()
        current, _, output, baseline = fixture.fixture(directory)
        current["scope"].update(
            translationUnits=[UNIT], projectTranslationUnits={UNIT: [UNIT]},
            cppcheckInputs={UNIT: {"project": [UNIT], "sdk": [], "unbound": []}},
        )
        current["programs"][0]["translationUnits"] = [UNIT]
        current["semanticIdentity"]["defines"] = {UNIT: []}
        current["diagnostics"] = [
            {"id": rule, "locations": [{"file": UNIT, "line": line}], "programs": ["target"]}
            for rule, line in zip(RULES, (7, 8))
        ]
        current["projectAcceptance"] = {"status": "blocked", "occurrences": []}
        previous = copy.deepcopy(current)
        if mutation == "multiplicity":
            current["diagnostics"].append(copy.deepcopy(current["diagnostics"][0]))
        elif mutation == "fewer occurrences":
            previous["diagnostics"].append(copy.deepcopy(previous["diagnostics"][0]))
        elif mutation == "program":
            current["diagnostics"][0]["programs"] = ["probe"]
        elif mutation == "spelling-dependent rule":
            for summary in (previous, current):
                summary["diagnostics"][0]["id"] = "misra-c2012-5.9"

        evidence = {}
        for side, output_dir, source, spelling, summary in (
                ("baseline", baseline, old, "reloaded_before_main", previous),
                ("current", output, new, "com_reloaded_before_main", current)):
            root = output_dir / "project"
            for diagnostic in summary["diagnostics"]:
                location = diagnostic["locations"][0]
                line = source.splitlines()[location["line"] - 1]
                column = line.find("=") + 1
                if mutation == "column moved" and side == "current" and location["line"] == 8:
                    column = line.index("FALSE") + 1
                if mutation != "column missing":
                    location["column"] = str(column)
            path = root / UNIT
            path.parent.mkdir(parents=True)
            path.write_bytes((source.replace("\n", "\r\n") if crlf else source).encode())
            fixture.seal(root, {"plan": {"configuration": "same"}})
            summary["sourceSealSha256"] = digest(root / "meta/files.sha256")
            fixture.write(output_dir, summary)
            declaration, *uses = [p for p in positions(source, spelling) if p[0] in (3, 7, 8)]
            if mutation == "shadowed use" and side == "current":
                uses.pop()
            addon = output_dir / "addon-receipts"
            addon.mkdir()
            (addon / "one.json").write_text(json.dumps({
                "source": str(path), "privateBindings": [{
                    "uses": [[*position, spelling, *declaration] for position in [declaration, *uses]],
                    "externs": ["reloaded_before_main"] if mutation == "external linkage" else [],
                }],
            }), encoding="utf-8")
            expansion = source.replace("= FALSE;", "= 0;").replace("== FALSE", "== 0")
            if mutation == "macro expansion" and side == "current":
                expansion = expansion.replace("== 0", "== 1")
            if mutation == "stringification":
                expansion = expansion.replace("= 0;", f'= "{spelling}";', 1)
            if mutation == "token paste":
                expansion = expansion.replace(spelling + "[0u]", spelling + "_suffix[0u]")
            (output_dir / "compiler-origins-0.stdout.log").write_text(
                f'# 1 "{path.as_posix()}"\n' + expansion, encoding="utf-8")
            evidence[side] = c_rename.record(output_dir, summary)
        proof = c_rename._proof_path(output, UNIT)
        if mutation == "evidence missing":
            proof.unlink()
        elif mutation == "evidence mismatched":
            proof.write_bytes(b"unbound")
        elif mutation in ("evidence invalid gzip", "evidence invalid JSON", "evidence invalid structure"):
            broken = {
                "evidence invalid gzip": b"not gzip",
                "evidence invalid JSON": gzip.compress(b"{"),
                "evidence invalid structure": gzip.compress(b'{"gcc":"", "bindings":{"target":[0]}}'),
            }
            proof.write_bytes(broken[mutation])
            evidence["current"][UNIT] = digest(proof)
        if mutation == "consumer":
            for summary in (current, previous):
                summary["scope"]["projectTranslationUnits"]["other.c"] = ["other.c", UNIT]
            fixture.write(baseline, previous)
        raw = copy.deepcopy(current)
        strict = c_comparison.compare(current, output / "project", output, baseline, source_tree=True)
        with patch.object(c_gate.c_acceptance, "evaluate", return_value=current["projectAcceptance"]):
            result = c_gate.verdict(current, output, baseline, {}, identifier_evidence=evidence)
        self.assertEqual(strict, result["comparison"])
        self.assertEqual(raw, current)
        self.assertEqual(result["projectAcceptance"], "blocked")
        return result

    def test_r10_com_private_rename_preserves_strict_deltas_with_lf_and_crlf(self):
        for crlf in (False, True):
            with self.subTest(crlf=crlf), tempfile.TemporaryDirectory() as directory:
                new = NEW.replace("static boolean", "static  boolean")
                gate = self.check(Path(directory), OLD, new, crlf=crlf)
                self.assertEqual(gate["status"], "passed")
                self.assertEqual(len(gate["comparison"]["new"]), 2)
                self.assertEqual(len(gate["comparison"]["removed"]), 2)
                self.assertEqual(gate["outOfScopeNew"], [])

    def test_changed_sources_do_not_get_rename_credit(self):
        cases = {
            "one old name to two new names": NEW.replace("if (com_reloaded", "if (other_reloaded"),
            "two old names to one new name": NEW.replace("boolean remaining;", "boolean com_reloaded_before_main;"),
            "operator": NEW.replace("== FALSE", "!= FALSE"),
            "constant": NEW.replace("[0u] = FALSE", "[1u] = FALSE"),
            "type": NEW.replace("unsigned char", "unsigned short"),
            "entity added": NEW + "static int extra;\n",
            "entity removed": NEW.replace("        sample.reloaded_before_main = 0u;\n", ""),
            "member changed": NEW.replace("sample.reloaded_before_main", "sample.com_reloaded_before_main"),
        }
        for mutation, source in cases.items():
            with self.subTest(mutation=mutation), tempfile.TemporaryDirectory() as directory:
                gate = self.check(Path(directory), OLD, source)
                self.assertEqual(gate["status"], "blocked")
                self.assertEqual(len(gate["unacceptedNew"]), 2)

    def test_shadowed_name_is_not_a_consumer_of_the_file_static_definition(self):
        old = OLD.replace("sample.reloaded_before_main = ", "boolean reloaded_before_main = ")
        new = NEW.replace("sample.reloaded_before_main = ", "boolean com_reloaded_before_main = ")
        with tempfile.TemporaryDirectory() as directory:
            gate = self.check(Path(directory), old, new)
        self.assertEqual(gate["status"], "blocked")
        self.assertEqual(len(gate["unacceptedNew"]), 2)

    def test_block_scope_extern_changes_linkage_when_visible_static_is_renamed(self):
        consumer = (
            "boolean Com_Pending(void) {\n"
            "    extern boolean reloaded_before_main[2u];\n"
            "    return reloaded_before_main[0u];\n"
            "}\n"
        )
        with tempfile.TemporaryDirectory() as directory:
            gate = self.check(Path(directory), OLD + consumer, NEW + consumer, "external linkage")
        self.assertEqual(gate["status"], "blocked")
        self.assertEqual(len(gate["unacceptedNew"]), 2)

    def test_surviving_consumer_and_spelling_dependent_rule_remain_blocked(self):
        with tempfile.TemporaryDirectory() as directory:
            new = NEW.replace("if (com_reloaded", "if (reloaded")
            gate = self.check(Path(directory), OLD, new)
        self.assertEqual(gate["status"], "blocked")
        self.assertEqual(gate["unacceptedNew"][0]["rule"], RULES[0])
        with tempfile.TemporaryDirectory() as directory:
            gate = self.check(Path(directory), OLD, NEW, "spelling-dependent rule")
        self.assertEqual(gate["status"], "blocked")
        self.assertEqual([row["rule"] for row in gate["unacceptedNew"]], ["misra-c2012-5.9"])

    def test_binding_expansion_program_and_occurrence_mutations(self):
        for mutation in ("consumer", "shadowed use", "macro expansion", "stringification",
                         "token paste", "multiplicity", "fewer occurrences", "program",
                         "column moved", "column missing"):
            with self.subTest(mutation=mutation), tempfile.TemporaryDirectory() as directory:
                gate = self.check(Path(directory), OLD, NEW, mutation)
                self.assertEqual(gate["status"], "blocked")
                self.assertEqual(len(gate["unacceptedNew"]), 2)

    def test_every_changed_line_requires_gcc_emission(self):
        old = '# 1 "@project@/fixture.c"\nstatic int storage;\n'
        new = old.replace("storage", "counters")
        changes = {1: [((1, 12, "storage"), (1, 12, "counters"))],
                   2: [((2, 1, "ViaSigned"), (2, 1, "ViaUnsigned"))]}
        self.assertEqual(c_rename._expansions(old, new, "fixture.c", {1: changes[1]}), {1})
        self.assertIsNone(c_rename._expansions(old, new, "fixture.c", changes))

    def test_bad_optional_evidence_keeps_strict_gate_output(self):
        for mutation, reason in (
                ("evidence missing", "file missing"),
                ("evidence mismatched", "bytes differ"),
                ("evidence invalid gzip", "payload unreadable"),
                ("evidence invalid JSON", "payload unreadable"),
                ("evidence invalid structure", "invalid structure")):
            with self.subTest(mutation=mutation), tempfile.TemporaryDirectory() as directory:
                gate = self.check(Path(directory), OLD, NEW, mutation)
                self.assertEqual(gate["status"], "blocked")
                self.assertEqual(gate["unacceptedNew"], gate["comparison"]["new"])
                self.assertIn(reason, gate["identifierEvidenceError"])

    def test_only_the_known_initial_gcc_cwd_marker_is_normalized(self):
        output = Path("/analysis/current")
        flag = '# 1 "/analysis/current//"'
        other = '# 1 "/analysis/another//"'
        first = '# 0 "/analysis/current/project/main.c"'
        raw = f"{first}\n{flag}\n{other}\n{flag}"
        expected = f'# 0 "@project@/main.c"\n# 1 "@cwd@//"\n{other}\n{flag}'
        self.assertEqual(c_rename._normalize_expansion(raw, output), expected)

    def test_evidence_must_match_the_transferred_summary_and_proof_bytes(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            artifact = root / c_rename.EVIDENCE / (UNIT + ".json.gz")
            artifact.parent.mkdir(parents=True)
            artifact.write_bytes(b"proof")
            identity = {UNIT: digest(artifact)}
            (root / "analysis-identity.json").write_text(json.dumps({
                "summarySha256": "raw-summary", "identifierEvidence": identity,
            }), encoding="utf-8")
            self.assertEqual(c_rename.evidence(root, "raw-summary"), identity)
            with self.assertRaisesRegex(ValueError, "different analysis summary"):
                c_rename.evidence(root, "changed-summary")
            c_rename.verify(root, identity)
            artifact.write_bytes(b"changed proof")
            with self.assertRaisesRegex(ValueError, "evidence bytes differ"):
                c_rename.verify(root, identity)
