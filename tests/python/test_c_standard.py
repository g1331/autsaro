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

    def test_det_module_and_service_arguments_require_names(self):
        for report in ("Det_ReportError", "Det_ReportRuntimeError", "Det_ReportTransientFault"):
            for arguments in ("60u, 0u, API, 1u", "MODULE, 0u, (0x49U), 1u",
                              "(uint16)60, 0u, API, 1u", "MODULE, 0u, API + 1u, 1u"):
                with self.subTest(report=report, arguments=arguments):
                    self.assertIn("det-numeric-id", self.rules(
                        f"void Report(void) {{ {report}(\n{arguments}); }}", "runtime/bsw/src/Test.c"))
        source = '''/* Det_ReportError(60u, 0u, 1u, 0u); */
void Report(uint8 ApiId) {
    Det_ReportRuntimeError(MODULE, 0u, ApiId, 7u);
    Det_ReportError(MODULE, 1u, Select(API, OTHER_API), 2u);
    Log("Det_ReportError(60u, 0u, 1u, 0u)");
}
Std_ReturnType Det_ReportRuntimeError(uint16 ModuleId, uint8 InstanceId,
                                    uint8 ApiId, uint8 ErrorId) { return E_OK; }
'''
        self.assertNotIn("det-numeric-id", self.rules(source, "runtime/bsw/src/Test.c"))
        self.assertNotIn("det-numeric-id", self.rules(
            "Det_ReportError(60u, 0u, 1u, 0u);", "runtime/os/src/Test.c"))

    def test_aggregate_objects_and_forward_tags(self):
        for prefix in ("", 'extern "C" {\n'):
            suffix = "\n}" if prefix else ""
            for kind in ("struct", "union", "enum"):
                body = "VALUE /**< Value. */" if kind == "enum" else "uint8 value; /**< Value. */"
                for declaration in (f"{kind} Opaque;", f"{kind} Tag {{ {body} }};"):
                    self.assertNotIn("state-comment", self.rules(prefix + declaration + suffix))
                for declaration in (f"static {kind} {{ {body} }} objects[2];", f"{kind} Tag *object;"):
                    self.assertIn("state-comment", self.rules(prefix + declaration + suffix))
                    self.assertNotIn("state-comment", self.rules(prefix + declaration + " /**< Owned state. */" + suffix))
                self.assertEqual(self.rules(prefix + f"typedef {kind} {{ {body} }} ValueType;" + suffix).count("typedef-comment"), 1)

    def test_definition_and_consistency_assertions_are_not_feature_switches(self):
        guards = ("#ifndef EXPECTED_STATUS", "#if STATUS_MODE != EXPECTED_STATUS")
        for guard in guards:
            source = guard + "\n#error Invalid independent configuration\n#endif\n"
            self.assertNotIn("config-switch", self.rules(source))
            active = source.replace("#error Invalid independent configuration", "uint8 enabled;")
            findings = c_standard.findings("runtime/os/src/Test.c", active, known_switches=frozenset({"STATUS_MODE"}))
            self.assertIn("config-switch", {key[0] for key in findings})
        self.assertIn("config-switch", self.rules("#if STATUS_MODE\n#error Unavailable\n#endif"))

    def test_release_presence_and_value_assertions_are_not_feature_switches(self):
        source = """#if !defined(CAN_AR_RELEASE_MAJOR_VERSION) || !defined(CAN_AR_RELEASE_MINOR_VERSION)
#error Missing release
#elif CAN_AR_RELEASE_MAJOR_VERSION != 4U || CAN_AR_RELEASE_MINOR_VERSION != 10U
#error Incompatible release
#endif
"""
        self.assertNotIn("config-switch", self.rules(source))
        self.assertNotIn("config-switch", self.rules(source.replace(
            "CAN_AR_RELEASE_MAJOR_VERSION != 4U", "CAN_AR_RELEASE_MAJOR_VERSION != CAN_C_AR_RELEASE_MAJOR_VERSION")))
        for active in (source.replace("#error Missing release", "uint8 selected;"),
                       source.replace("#error Incompatible release", "#include \"active.h\""),
                       source.replace("#endif", "#else\nuint8 selected;\n#endif"),
                       source.replace("CAN_AR_RELEASE_MAJOR_VERSION", "CAN_FEATURE_API")):
            with self.subTest(active=active):
                self.assertIn("config-switch", self.rules(active))

    def test_software_presence_and_value_assertions_keep_feature_selection_rejected(self):
        source = """#define COMM_C_SW_MAJOR_VERSION 0U
#define COMM_C_SW_MINOR_VERSION 1U
#define COMM_C_SW_PATCH_VERSION 0U
#if !defined(COMM_SW_MAJOR_VERSION) || !defined(COMM_SW_MINOR_VERSION) || !defined(COMM_SW_PATCH_VERSION)
#error Missing software version
#elif COMM_SW_MAJOR_VERSION != COMM_C_SW_MAJOR_VERSION || COMM_SW_MINOR_VERSION != COMM_C_SW_MINOR_VERSION || COMM_SW_PATCH_VERSION != COMM_C_SW_PATCH_VERSION
#error Inconsistent software version
#endif
"""
        self.assertNotIn("config-switch", self.rules(source))
        for active in (
            source.replace("#error Missing software version", "uint8 selected;"),
            source.replace("#error Inconsistent software version", '#include "active.h"'),
            source.replace("#endif", "#else\nuint8 selected;\n#endif"),
            source.replace("COMM_SW_PATCH_VERSION", "COMM_FEATURE_API"),
        ):
            with self.subTest(active=active):
                self.assertIn("config-switch", self.rules(active))

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

    def test_void_and_required_unnamed_parameter_comment_forms(self):
        comment = CONTRACT.replace(" * @param[in] id Object index, borrowed for this call.\n * @param[out] value Caller-owned result; written on success.\n", " * @par Parameters\n * 1. [in] Object index.\n * 2. [out] Caller-owned writable result.\n").replace("@return E_OK on success; E_NOT_OK preserves output.", "@par Return\n * None.")
        declaration = "void Run(uint8, const uint32 *);"
        self.assertEqual(self.rules(comment + declaration), [])
        for original, replacement in (("2. [out]", "3. [out]"), ("2. [out]", "2. [inout]"), ("1. [in]", "1. []"), (" * 2. [out] Caller-owned writable result.\n", "")):
            self.assertIn("api-parameters", self.rules(comment.replace(original, replacement) + declaration))
        self.assertIn("api-return", self.rules(comment + "uint8 Run(uint8, const uint32 *);"))
        self.assertIn("api-purpose", self.rules(comment.replace("Read the selected object.", "") + declaration))
        self.assertEqual(c_standard.parameter_name(["const", "uint8", "*"]), "<unnamed>")
        self.assertEqual(c_standard.parameter_name(["unsigned", "int", "id"]), "id")

    def test_wrapped_contract_fields_and_inline_record_result(self):
        wrapped = CONTRACT.replace("Ownership: neither pointer is retained.", "Ownership:\n * neither pointer is retained.")
        self.assertEqual(self.rules(wrapped + DECLARATION), [])
        self.assertIn("api-ownership", self.rules(CONTRACT.replace("neither pointer is retained.", "") + DECLARATION))
        self.assertIn("api-contract", self.rules("static inline struct Value Read(void) { return value; }"))

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

    def test_aggregate_closing_declarations_inside_cpp_linkage(self):
        source = 'extern "C" {\n/** Record. */\ntypedef struct { uint8 id; /**< Handle. */ } RecordType;\nstruct Value { uint8 id; /**< Handle. */ };\n}'
        self.assertEqual(self.rules(source), [])

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
        self.assertNotIn("config-switch", self.rules("#if defined(TEST_API) && (TEST_API == STD_ON)\n#endif"))
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

    def test_include_deviations_match_file_rule_and_literal_header(self):
        with tempfile.TemporaryDirectory(prefix="include-scope-") as temporary:
            root = Path(temporary)
            source = root / "runtime/os/src/Host.c"
            source.parent.mkdir(parents=True)
            document = root / "docs/standards/os.md"
            document.parent.mkdir(parents=True)
            marker = (
                "/* MR12 RULE 21.5 VIOLATION: reviewed fixed platform interface. */\n"
            )
            record = (
                "| MISRA C:2012 Rule 21.5 | `runtime/os/src/Host.c` 中 "
                "`#include <signal.h>` | fixed platform | Approved by maintainer |\n"
            )
            document.write_text(record, encoding="utf-8")
            for directive in (
                "#include <signal.h>",
                "# /* provider */ include /* header */ <signal.h> // spelling",
                "#include \\\n <signal.h>",
            ):
                with self.subTest(directive=directive):
                    source.write_text(marker + directive + "\n", encoding="utf-8")
                    self.assertEqual(c_standard.deviation_findings(root), {})
            source.write_text("#include <signal.h> " + marker, encoding="utf-8")
            self.assertEqual(c_standard.deviation_findings(root), {})
            for header in ('"private/signal.h"', "<time.h>"):
                with self.subTest(header=header):
                    document.write_text(
                        record.replace("<signal.h>", header), encoding="utf-8"
                    )
                    source.write_text(
                        marker + "# include " + header + "\n", encoding="utf-8"
                    )
                    self.assertEqual(c_standard.deviation_findings(root), {})
            cases = (
                (
                    "unapproved",
                    record.replace("Approved by maintainer", "Pending review"),
                    marker + "#include <signal.h>\n",
                    {"deviation-unrecorded"},
                ),
                (
                    "duplicate-record",
                    record + record,
                    marker + "#include <signal.h>\n",
                    {"deviation-duplicate", "deviation-uncommented"},
                ),
                (
                    "wrong-file",
                    record.replace("Host.c", "Other.c"),
                    marker + "#include <signal.h>\n",
                    {"deviation-unrecorded", "deviation-uncommented"},
                ),
                (
                    "wrong-rule",
                    record.replace("Rule 21.5", "Rule 21.10"),
                    marker + "#include <signal.h>\n",
                    {"deviation-unrecorded", "deviation-uncommented"},
                ),
                (
                    "wrong-header",
                    record,
                    marker + "#include <time.h>\n",
                    {"deviation-unrecorded", "deviation-uncommented"},
                ),
                (
                    "wrong-delimiter",
                    record,
                    marker + '#include "signal.h"\n',
                    {"deviation-unrecorded", "deviation-uncommented"},
                ),
                (
                    "header-spelling",
                    record,
                    marker + "#include <sig/**/nal.h>\n",
                    {"deviation-unrecorded", "deviation-uncommented"},
                ),
                ("stale", record, "#include <signal.h>\n", {"deviation-uncommented"}),
                (
                    "macro-include",
                    record,
                    marker + "#include SIGNAL_HEADER\n",
                    {"deviation-unrecorded", "deviation-uncommented"},
                ),
                (
                    "duplicate-site",
                    record,
                    (marker + "#include <signal.h>\n") * 2,
                    {"deviation-duplicate", "deviation-unrecorded", "deviation-ambiguous"},
                ),
            )
            for case, text, code, expected in cases:
                with self.subTest(case=case):
                    document.write_text(text, encoding="utf-8")
                    source.write_text(code, encoding="utf-8")
                    self.assertEqual(
                        {key[0] for key in c_standard.deviation_findings(root)},
                        expected,
                    )

    def test_repeated_directives_require_exact_occurrences(self):
        with tempfile.TemporaryDirectory(prefix="directive-scope-") as temporary:
            root = Path(temporary)
            source = root / "runtime/os/src/Host.c"
            source.parent.mkdir(parents=True)
            document = root / "docs/standards/os.md"
            document.parent.mkdir(parents=True)
            included = '#include "Os_MemMap.h"\n'
            removed = "#undef OS_START_SEC_CODE\n"
            include_marker = "/* MR12 RULE 20.1 VIOLATION: reviewed mapping boundary. */\n"
            undef_marker = "/* MR12 RULE 20.5 VIOLATION: reviewed keyword withdrawal. */\n"
            code = (included + "#if 0\n" + included + "#endif\n" + include_marker + included
                    + removed + undef_marker + removed)
            include_record = ('| MISRA C:2012 Rule 20.1 | `runtime/os/src/Host.c` 中 '
                              '`#include "Os_MemMap.h" [3]` | mapping | Approved by maintainer |\n')
            undef_record = ('| MISRA C:2012 Rule 20.5 | `runtime/os/src/Host.c` 中 '
                            '`#undef OS_START_SEC_CODE [2]` | withdrawal | Approved by maintainer |\n')
            records = include_record + undef_record
            document.write_text(records, encoding="utf-8")
            source.write_text(code, encoding="utf-8")
            self.assertEqual(c_standard.deviation_findings(root), {})
            document.write_text(records.replace("Rule 20.1", "Directive 4.10"), encoding="utf-8")
            source.write_text(code.replace("RULE 20.1", "DIR 4.10"), encoding="utf-8")
            self.assertEqual(c_standard.deviation_findings(root), {})
            document.write_text(records + include_record.replace(" [3]", " [1]"), encoding="utf-8")
            source.write_text(code.replace(included, include_marker + included, 1), encoding="utf-8")
            self.assertEqual(c_standard.deviation_findings(root), {})
            document.write_text(records, encoding="utf-8")
            source.write_text(code.replace(removed, "# /* marker */ undef \\\n OS_START_SEC_CODE // done\n"), encoding="utf-8")
            self.assertEqual(c_standard.deviation_findings(root), {})
            cases = (
                ("unqualified", records.replace(" [3]", ""), code, "deviation-ambiguous"),
                ("wrong-position", records.replace(" [3]", " [2]"), code, "deviation-uncommented"),
                ("out-of-range", records.replace(" [3]", " [9]"), code, "deviation-uncommented"),
                ("zero", records.replace(" [3]", " [0]"), code, "deviation-uncommented"),
                ("leading-zero", records.replace(" [3]", " [03]"), code, "deviation-uncommented"),
                ("negative", records.replace(" [3]", " [-1]"), code, "deviation-uncommented"),
                ("duplicate-record", records + include_record, code, "deviation-duplicate"),
                ("duplicate-marker", records, code.replace(include_marker, include_marker * 2),
                 "deviation-duplicate"),
                ("wrong-header", records.replace("Os_MemMap.h", "Other_MemMap.h"), code,
                 "deviation-unrecorded"),
                ("wrong-file", records.replace("Host.c", "Other.c"), code, "deviation-unrecorded"),
                ("wrong-rule", records.replace("Rule 20.5", "Rule 20.1"), code,
                 "deviation-unrecorded"),
                ("wrong-keyword", records.replace("OS_START_SEC_CODE", "OS_STOP_SEC_CODE"), code,
                 "deviation-unrecorded"),
                ("unapproved", records.replace("Approved by maintainer", "Pending review"), code,
                 "deviation-unrecorded"),
                ("missing-comment", records, code.replace(undef_marker, ""),
                 "deviation-uncommented"),
                ("removed-occurrence", records, code.replace(included, "", 1),
                 "deviation-uncommented"),
                ("inserted-occurrence", records, included + code, "deviation-uncommented"),
                ("mixed-scopes", records + include_record.replace(" [3]", ""), code,
                 "deviation-uncommented"),
                ("invalid-approved-only", include_record.replace(" [3]", " [0]"), "",
                 "deviation-uncommented"),
            )
            for name, recorded, text, required in cases:
                with self.subTest(case=name):
                    document.write_text(recorded, encoding="utf-8")
                    source.write_text(text, encoding="utf-8")
                    findings = c_standard.deviation_findings(root)
                    self.assertIn(required, {key[0] for key in findings})

    def test_single_undef_and_qualified_include_keep_literal_identity(self):
        with tempfile.TemporaryDirectory(prefix="directive-single-") as temporary:
            root = Path(temporary)
            source = root / "runtime/os/src/Host.c"
            source.parent.mkdir(parents=True)
            document = root / "docs/standards/os.md"
            document.parent.mkdir(parents=True)
            document.write_text(
                '| MISRA C:2012 Rule 20.5 | `runtime/os/src/Host.c` 中 '
                '`#undef OS_START_SEC_CODE` | withdrawal | Approved by maintainer |\n'
                '| MISRA C:2012 Rule 20.1 | `runtime/os/src/Host.c` 中 '
                '`#include "Os_MemMap.h" [1]` | mapping | Approved by maintainer |\n', encoding="utf-8")
            source.write_text(
                '/* MR12 RULE 20.5 VIOLATION: reviewed withdrawal. */\n'
                '#undef OS_START_SEC_CODE\n'
                '# include "Os_MemMap.h" /* MR12 RULE 20.1 VIOLATION: reviewed mapping. */\n', encoding="utf-8")
            self.assertEqual(c_standard.deviation_findings(root), {})

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
