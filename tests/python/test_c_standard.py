"""C coding checks inspect declarations, not keyword counts."""

import collections
import tempfile
import unittest
from pathlib import Path

from autosar_tooling import c_standard

CONTRACT = """/** Read the selected object.
 * @param[in] id Object index, borrowed for this call.
 * @param[out] value Caller-owned result; written on success.
 * @return E_OK on success; E_NOT_OK preserves output.
 * Preconditions: initialized; otherwise E_NOT_OK.
 * Context: task, outside exclusive areas.
 * Reentrancy: reentrant across instances.
 * Ownership: neither pointer is retained.
 */
"""
DECLARATION = "Std_ReturnType Read(uint8 id,\n uint32 *value);\n"


class CStandardTests(unittest.TestCase):
    def rules(self, source, name="runtime/bsw/include/Test.h"):
        return [rule for rule, _, _ in c_standard.findings(name, source)]

    def test_review_declarators_and_empty_comments(self):
        self.assertIn("state-comment", self.rules("static void (*hook)(void);", "runtime/bsw/src/Test.c"))
        self.assertIn("state-comment", self.rules("static Std_ReturnType (*hook)(void);", "runtime/bsw/src/Test.c"))
        self.assertIn("api-contract", self.rules("__attribute__((warn_unused_result)) uint8 Read(uint8 id); /**< Value. */"))
        self.assertEqual(c_standard.function(c_standard.lex("void (*GetCallback(uint8 id))(void);")), ("GetCallback", ["id"]))
        self.assertIn("api-contract", self.rules("static inline void Run(uint8 id) { (void)id; }"))
        self.assertIn("typedef-comment", self.rules("/** */ typedef uint8 ValueType;"))
        self.assertIn("macro-comment", self.rules("#define TEST_LIMIT 7U /**< */"))

    def test_private_and_delayed_guards(self):
        self.assertNotIn("config-switch", self.rules("#ifndef INTERNAL_H\n#define INTERNAL_H\n#endif", "runtime/bsw/src/Internal.h"))
        self.assertNotIn("config-switch", self.rules("#ifndef RTE_TEST_H\n#ifdef RTE_APPLICATION_HEADER_FILE\n#error Duplicate\n#endif\n#define RTE_TEST_H\n#ifdef RTE_CORE\n#endif\n#if defined(RTE_RUNNABLEAPI_Run)\n#endif\n#endif", "emitted/generated/include/Rte_Test.h"))

    def test_complete_multiline_contract(self):
        self.assertEqual(self.rules(CONTRACT + DECLARATION), [])

    def test_each_contract_item_is_observed(self):
        for field in c_standard.FIELDS:
            with self.subTest(field=field):
                self.assertIn("api-" + field.lower(), self.rules((CONTRACT + DECLARATION).replace(field + ":", "Other:")))
        self.assertIn("api-return", self.rules((CONTRACT + DECLARATION).replace("@return", "@result")))
        for original, replacement in (("[out]", "[inout]"), ("[in]", ""), ("id Object", "wrong Object")):
            with self.subTest(original=original):
                self.assertIn("api-parameters", self.rules((CONTRACT + DECLARATION).replace(original, replacement)))

    def test_contract_must_be_adjacent(self):
        self.assertIn("api-contract", self.rules(CONTRACT + "#define OTHER 1\n" + DECLARATION))
        self.assertIn("api-contract", self.rules(CONTRACT + "/* unrelated */\n" + DECLARATION))

    def test_cpp_linkage_does_not_hide_public_functions(self):
        self.assertEqual(self.rules('#ifdef __cplusplus\nextern "C" {\n#endif\nvoid Run(void);\n#ifdef __cplusplus\n}\n#endif'), ["api-contract"])

    def test_continued_directive_does_not_become_a_declaration(self):
        source = "#if defined(A) || " + "\\" + "\n defined(B)\n#endif\nvoid Run(void);"
        self.assertIn("api-contract", self.rules(source))
        self.assertNotIn("typedef-comment", self.rules(source))

    def test_void_still_requires_return_and_four_fields(self):
        source = CONTRACT.replace(" * @param[in] id Object index, borrowed for this call.\n", "").replace(" * @param[out] value Caller-owned result; written on success.\n", "")
        self.assertEqual(self.rules(source + "void Stop(void);"), [])

    def test_function_pointer_typedef_is_not_api(self):
        self.assertEqual(self.rules("/** Callback type. */\ntypedef void (*CallbackType)(uint8 id);"), [])
        self.assertEqual(self.rules("typedef void (*CallbackType)(uint8 id);"), ["typedef-comment"])

    def test_aggregate_members_and_enum_last_value(self):
        self.assertEqual(self.rules("/** Choices. */\ntypedef enum {\n A, /**< First. */\n B /**< Last. */\n} ChoiceType;"), [])
        self.assertEqual(self.rules("/** Record. */\ntypedef struct {\n uint8 id; /**< Index. */\n} RecordType;"), [])
        self.assertIn("member-comment", self.rules("/** Record. */\ntypedef struct { uint8 id; } RecordType;"))

    def test_states_not_local_variables(self):
        source = "static uint8 state;\nvoid Run(void) { uint8 local; local = 1; }\nstatic uint8 next;"
        self.assertEqual(self.rules(source, "runtime/bsw/src/Test.c"), ["state-comment", "state-comment"])

    def test_array_initializers_are_states(self):
        self.assertEqual(self.rules("static uint8 values[2] = {0, 1};", "runtime/bsw/src/Test.c"), ["state-comment"])
        self.assertEqual(self.rules("static uint8 values[2] = {0, 1}; /**< Values. */", "runtime/bsw/src/Test.c"), [])

    def test_strings_comments_and_inactive_code(self):
        self.assertNotIn("bsw-native-type", self.rules('/* unsigned int */\n#define TEXT "unsigned int"\n'))
        self.assertIn("bsw-native-type", self.rules("#if 0\ntypedef unsigned HiddenType;\n#endif"))

    def test_macro_categories_and_function_contracts(self):
        self.assertIn("macro-comment", self.rules("#define TEST_LIMIT 7U"))
        self.assertEqual(self.rules("#define TEST_LIMIT 7U /**< Capacity in entries. */"), [])
        self.assertEqual(self.rules("/** Published version from SWS_BSW_00059. */\n#define TEST_MAJOR_VERSION 1U\n#define TEST_MINOR_VERSION 0U"), [])
        self.assertIn("macro-configuration", self.rules("#define TEST_API STD_ON"))
        self.assertEqual(self.rules("/** ECUC_Test_00001; generated by the configuration owner. */\n#define TEST_API STD_ON"), [])
        self.assertIn("macro-api-contract", self.rules("#define Read(id,value) (id)"))
        self.assertEqual(self.rules(CONTRACT + "#define Read(id,value) (id)"), [])

    def test_native_types_in_continued_macro_report_actual_line(self):
        source = "#define CAST(x) " + "\\" + "\n ((unsigned)(x))"
        keys = c_standard.findings("runtime/bsw/include/Test.h", source)
        self.assertIn(("bsw-native-type", "runtime/bsw/include/Test.h", "((unsigned)(x))"), keys)

    def test_guard_and_memmap_protocol_exempt(self):
        self.assertEqual(self.rules("#ifndef TEST_H\n#define TEST_H\n#endif\n"), [])
        self.assertEqual(self.rules("#ifdef TEST_START_SEC_CODE\n#define TEST_CODE\n#endif\n", "runtime/bsw/include/Test_MemMap.h"), [])

    def test_memmap_file_does_not_exempt_functional_switches(self):
        self.assertIn("config-switch", self.rules("#ifdef TEST_API\n#endif", "runtime/bsw/include/Test_MemMap.h"))

    def test_commented_and_included_switch_definitions(self):
        self.assertIn("config-switch", self.rules("#define FEATURE_FLAG STD_ON /**< Enabled. */\n#if (FEATURE_FLAG == 1)\n#endif"))
        self.assertIn("config-switch", self.rules("#define CAN_RX_POLLING 0\n#if CAN_RX_POLLING == 1\n#endif"))
        switches = c_standard.switch_definitions(["#define FEATURE_FLAG STD_ON /**< Enabled. */"])
        result = c_standard.findings("runtime/bsw/src/Test.c", "#if (FEATURE_FLAG == 1)\n#endif", known_switches=switches)
        self.assertIn("config-switch", {key[0] for key in result})

    def test_switch_definition_and_value_validation_are_not_feature_selection(self):
        self.assertNotIn("config-switch", self.rules("#ifndef TEST_API\n#define TEST_API STD_ON\n#endif"))
        self.assertNotIn("config-switch", self.rules("#define TEST_API STD_ON\n#if (TEST_API != 0) && (TEST_API != 1)\n#error Invalid\n#endif"))
        self.assertIn("config-switch", self.rules("#define TEST_API STD_OFF\n#ifdef TEST_API\n#error Unsupported\n#endif"))

    def test_switch_requires_value_comparison(self):
        self.assertIn("config-switch", self.rules("#define TEST_API STD_ON\n#ifdef TEST_API\n#endif"))
        self.assertNotIn("config-switch", self.rules("#define TEST_API STD_ON\n#if (TEST_API == STD_ON)\n#endif"))

    def test_baseline_multiplicity_and_fixed_entries(self):
        with tempfile.TemporaryDirectory(prefix="171-") as temporary:
            path = Path(temporary) / "baseline.txt"
            path.write_text("api-contract runtime/bsw/include/Test.h void Run(void);\n", encoding="utf-8")
            key = ("api-contract", "runtime/bsw/include/Test.h", "void Run(void);")
            self.assertEqual(c_standard.baseline_errors(collections.Counter({key: 1}), path), [])
            self.assertIn("new C standard", c_standard.baseline_errors(collections.Counter({key: 2}), path)[0])
            self.assertIn("fixed finding", c_standard.baseline_errors(collections.Counter(), path)[0])

    def test_documentation_counts_columns_but_not_repeat_inclusions(self):
        import json

        with tempfile.TemporaryDirectory(prefix="171-") as temporary:
            project = Path(temporary)
            (project / "meta").mkdir()
            (project / "meta/integration.json").write_text(json.dumps({"originalBswSources": {}}), encoding="utf-8")
            header = project / "Test.h"
            header.write_text("/** @param a Bad; @param b Bad. */", encoding="utf-8")
            first = f"{header}:1:5: warning: parameter missing [-Wdocumentation]\n"
            second = f"{header}:1:19: warning: parameter missing [-Wdocumentation]\n"
            self.assertEqual(sum(c_standard.documentation_findings(first + first + second, project).values()), 2)
            header.write_text("/** Same. */\n/** Same. */", encoding="utf-8")
            positions = c_standard.documentation_locations(first, project)
            positions.update(c_standard.documentation_locations(first.replace(":1:5:", ":2:5:"), project))
            self.assertEqual(sum(collections.Counter(positions.values()).values()), 2)

    def test_generated_headers_use_runtime_rules_and_skip_copies(self):
        import json

        with tempfile.TemporaryDirectory(prefix="171-") as temporary:
            samples = Path(temporary)
            project = samples / "sample"
            (project / "generated/include").mkdir(parents=True)
            (project / "bsw/include").mkdir(parents=True)
            (project / "meta").mkdir()
            (project / "target.json").write_text("{}", encoding="utf-8")
            (project / "meta/integration.json").write_text(json.dumps({"originalBswSources": {"runtime/bsw/include/Test.h": "bsw/include/Test.h"}}), encoding="utf-8")
            (project / "bsw/include/Test.h").write_text("void Copied(void);", encoding="utf-8")
            (project / "generated/include/Test.h").write_text("void Emitted(void);", encoding="utf-8")
            self.assertEqual(c_standard.generated_findings(samples), {("api-contract", "emitted/generated/include/Test.h", "void Emitted(void);"): 1})
            (project / "generated/include/Test.h").write_text(CONTRACT + DECLARATION, encoding="utf-8")
            self.assertEqual(c_standard.generated_findings(samples), {})
            (project / "generated/include/Com_Cfg.h").write_text("/** Signal identifier. */ typedef unsigned Com_SignalIdType;", encoding="utf-8")
            self.assertIn("bsw-native-type", {key[0] for key in c_standard.generated_findings(samples)})

    def test_deviations_match_both_directions_and_exact_symbol(self):
        with tempfile.TemporaryDirectory(prefix="171-") as temporary:
            root = Path(temporary)
            (root / "runtime/bsw/src").mkdir(parents=True)
            (root / "docs/standards").mkdir(parents=True)
            source = root / "runtime/bsw/src/Test.c"
            source.write_text('#define TEXT "MR12 RULE 8.7 VIOLATION: text"\n', encoding="utf-8")
            self.assertEqual(c_standard.deviation_findings(root), {})
            source.write_text("/* MR12 RULE 8.7 VIOLATION: required API. */\nvoid Test_Run(void) {}", encoding="utf-8")
            document = root / "docs/standards/test.md"
            document.write_text("| MISRA C:2012 Rule 8.7 | `runtime/bsw/src/Test.c` 中 `Test_Run` | required | Approved by maintainer |\n", encoding="utf-8")
            self.assertEqual(c_standard.deviation_findings(root), {})
            source.write_text("void Test_Run(void) {} /* MR12 RULE 8.7 VIOLATION: required API. */", encoding="utf-8")
            self.assertEqual(c_standard.deviation_findings(root), {})
            for marker in ("/* MR12 RULE 8.7 VIOLATION required API. */", "/* MR12 RULE 8.7 VIOLATION: */"):
                source.write_text(marker + "\nvoid Test_Run(void) {}", encoding="utf-8")
                self.assertIn("deviation-format", {key[0] for key in c_standard.deviation_findings(root)})
            source.write_text("/* MR12 RULE 8.7 VIOLATION: required API. */\nvoid Test_Run(void) {}", encoding="utf-8")
            approved = document.read_text()
            document.write_text(approved + approved, encoding="utf-8")
            self.assertIn("deviation-duplicate", {key[0] for key in c_standard.deviation_findings(root)})
            document.write_text(approved, encoding="utf-8")
            document.write_text(approved.replace("Approved by maintainer", "Pending"), encoding="utf-8")
            self.assertEqual({key[0] for key in c_standard.deviation_findings(root)}, {"deviation-unrecorded"})
            document.write_text(approved, encoding="utf-8")
            document.write_text(document.read_text().replace("Test_Run", "Test_Other"), encoding="utf-8")
            self.assertEqual({key[0] for key in c_standard.deviation_findings(root)}, {"deviation-unrecorded", "deviation-uncommented"})


if __name__ == "__main__":
    unittest.main()
