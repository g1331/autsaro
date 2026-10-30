"""C coding checks inspect declarations, not keyword counts."""

import collections
import copy
import json
import shutil
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

    def test_owned_headers_begin_mapping_before_c_declarations(self):
        from autosar_tooling.config import ROOT

        for relative in (
            "runtime/os/include/Arti.h",
            "runtime/os/src/Os_Stack.h",
            "runtime/os/src/host/linux/Os_HostLinux.h",
            "runtime/bsw/include/SchM_Can.h",
            "runtime/bsw/include/SchM_Dcm.h",
            "runtime/bsw/include/SchM_BswM.h",
        ):
            source = (ROOT / relative).read_text(encoding="utf-8")
            tokens = c_standard.lex(source)
            for token in tokens:
                if c_standard.is_comment(token):
                    continue
                self.assertTrue(token.text.lstrip().startswith("#"), (relative, token.text))
                if "_MemMap.h" in token.text:
                    break
            else:
                self.fail(f"{relative}: no memory mapping include")
            c_standard.verify_memory_mapping_pattern(source)

    def test_memory_mapping_protocol_and_mutations(self):
        source = ('#if ENABLED\n#define RTE_Com_START_SEC_CODE\n'
                  '# include "Rte_MemMap.h" /* mapping */\nvoid run(void) {}\n'
                  '#define RTE_Com_STOP_SEC_CODE\n#include "Rte_MemMap.h"\n#endif\n')
        self.assertEqual(c_standard.verify_memory_mapping_pattern(source), 2)
        start = '#define RTE_Com_START_SEC_CODE'
        stop = '#define RTE_Com_STOP_SEC_CODE'
        include = '#include "Rte_MemMap.h"'
        for label, mutant, reason in (
            ("missing-stop", source.replace(stop, '// missing STOP'), "unexpected MemMap include"),
            ("missing-final-include", source.replace(include, ''), "mapping marker must immediately include its header"),
            ("preprocessing", source.replace('void run', '#if X\n#endif\nvoid run'), "preprocessing inside mapped section"),
            ("nested", source.replace('void run', start + '\n' + include + '\nvoid run'), "nested mapping"),
            ("wrong-class", source.replace(stop, stop.replace('CODE', 'CONST_32')), "mismatched mapping"),
            ("wrong-header", source.replace(include, '#include "Other_MemMap.h"'), "mismatched MemMap header"),
            ("unrequested", include, "unexpected MemMap include"),
            ("missing-both", source[:source.index(stop)], "unclosed memory mapping"),
            ("intervening-code", source.replace(start, start + '\nint x;'), "mapping marker must immediately include its header"),
        ):
            with self.subTest(mutation=label):
                self.assertEqual(c_standard.memory_mapping_findings("test.c", mutant),
                                 {("memory-mapping", "test.c", reason): 1})

    def test_memory_mapping_comments_follow_c_preprocessing_order(self):
        source = ('/* request */ #define CAN_START_SEC_CODE\n'
                  '/* request */ #include "Can_MemMap.h" /* request\n */\n'
                  'CAN_CODE void run(void) {}\n')
        self.assertEqual(c_standard.memory_mapping_findings("test.c", source),
                         {("memory-mapping", "test.c", "unclosed memory mapping"): 1})
        source += ('/* request */ #define CAN_STOP_SEC_CODE\n'
                   '/* request */ #include "Can_MemMap.h"\n')
        self.assertEqual(c_standard.verify_memory_mapping_pattern(source), 2)
        spliced = source.replace('/* request */', '/\\\n* request *\\\n/')
        self.assertEqual(c_standard.verify_memory_mapping_pattern(spliced), 2)
        literals = '\nconst char *text = "/* not a comment */ // literal";\n'
        self.assertEqual(c_standard.verify_memory_mapping_pattern(source + literals), 2)

    def test_memory_mapping_checks_runtime_and_generated_c_inputs(self):
        import json

        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            runtime = root / "runtime/host/src"
            runtime.mkdir(parents=True)
            source = '/* request */ #define TEST_START_SEC_CODE\n/* request */ #include "Test_MemMap.h"\n'
            (runtime / "Test.c").write_text(source, encoding="utf-8")
            self.assertIn(("memory-mapping", "runtime/host/src/Test.c", "unclosed memory mapping"),
                          c_standard.runtime_findings(root))
            samples = root / "samples"
            project = samples / "standard-ecu"
            (project / "meta").mkdir(parents=True)
            (project / "target.json").write_text("{}", encoding="utf-8")
            (project / "meta/integration.json").write_text(json.dumps({"originalBswSources": {}}), encoding="utf-8")
            for directory in ("generated/src", "app", "bsw/src"):
                path = project / directory / "Test.c"
                path.parent.mkdir(parents=True)
                path.write_text(source, encoding="utf-8")
                self.assertIn(("memory-mapping", "emitted/" + directory + "/Test.c", "unclosed memory mapping"),
                              c_standard.generated_findings(samples))

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

    def test_retained_macro_shapes_preserve_replacement_tokens_and_legacy_records(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / "runtime/bsw/include/Test.h"
            source.parent.mkdir(parents=True)
            document = root / "docs/standards/test.md"
            document.parent.mkdir(parents=True)
            marker = "/* MR12 RULE 2.5 VIOLATION: retain the approved preprocessing contract. */\n"
            original = marker + "#define API(value) \\\n    ((value) | 7u)\n"
            record = (
                "| MISRA C:2012 Rule 2.5 | `runtime/bsw/include/Test.h` 中 `API` | "
                "shape=`#define API(value) ((value) &#124; 7u)` | 维护者 2026-10-30精确124键决定 |\n"
            )
            legacy = "/* MR12 RULE 2.5 VIOLATION: original name-bound approval. */\n#define LEGACY 9u\n"
            legacy_record = "| MISRA C:2012 Rule 2.5 | `runtime/bsw/include/Test.h` 中 `LEGACY` | old exact name | 维护者 原ADR0023 |\n"
            for label, changed, table, expected in (
                ("continued-and-bar", original, record, None),
                ("comments-and-spacing", original.replace("| 7u", "| /* note */ 7u"), record, None),
                ("changed-body", original.replace("7u", "8u"), record, "deviation-shape"),
                ("changed-macro-kind", original.replace("API(value)", "API (value)"), record, "deviation-shape"),
                ("missing-shape", original, record.replace("shape=", "former="), "deviation-shape-record"),
                ("missing-comment", original.replace(marker, ""), record, "deviation-uncommented"),
                ("wrong-rule", original.replace("RULE 2.5", "RULE 20.10"), record, "deviation-unrecorded"),
                ("wrong-owner", original.replace("API(value)", "OTHER(value)"), record, "deviation-unrecorded"),
                ("outside-approval", original + marker + "#define EXTRA 7u\n", record, "deviation-unrecorded"),
            ):
                with self.subTest(case=label):
                    source.write_text(changed + legacy, encoding="utf-8")
                    document.write_text(table + legacy_record, encoding="utf-8")
                    findings = c_standard.deviation_findings(root)
                    if expected is None:
                        self.assertEqual(findings, {})
                    else:
                        self.assertTrue(any(rule == expected for rule, _, _ in findings), findings)
            source.write_text(original + legacy.replace("9u", "12u"), encoding="utf-8")
            document.write_text(record + legacy_record, encoding="utf-8")
            self.assertEqual(c_standard.deviation_findings(root), {})
            paste = marker + "/* MR12 RULE 20.10 VIOLATION: retain the approved entry binding. */\n#define ENTRY(id) Task_##id\n"
            table = "".join(
                f"| MISRA C:2012 Rule {rule} | `runtime/bsw/include/Test.h` 中 `ENTRY` | "
                "shape=`#define ENTRY(id) Task_##id` | 维护者 2026-10-30精确124键决定 |\n"
                for rule in ("2.5", "20.10")
            )
            source.write_text(paste, encoding="utf-8")
            document.write_text(table, encoding="utf-8")
            self.assertEqual(c_standard.deviation_findings(root), {})
            source.write_text(paste.replace("##", "# #"), encoding="utf-8")
            self.assertTrue(any(rule == "deviation-shape" for rule, _, _ in c_standard.deviation_findings(root)))

    def test_retained_multiline_object_checks_its_complete_declaration(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / "runtime/os/src/Os_Backend.c"
            source.parent.mkdir(parents=True)
            document = root / "docs/standards/test.md"
            document.parent.mkdir(parents=True)
            marker = "/* MR12 RULE 8.7 VIOLATION: retain the observed context array. */\n"
            original = marker + "OS_MEMMAP void *\n    *Os_ArtiTaskContexts[32u];\n"
            document.write_text(
                "| MISRA C:2012 Rule 8.7 | `runtime/os/src/Os_Backend.c` 中 `Os_ArtiTaskContexts` | "
                "shape=`OS_MEMMAP void * *Os_ArtiTaskContexts[32u];` | 维护者 2026-10-30精确124键决定 |\n",
                encoding="utf-8",
            )
            source.write_text(original, encoding="utf-8")
            self.assertEqual(c_standard.deviation_findings(root), {})
            source.write_text(original.replace("32u", "16u"), encoding="utf-8")
            self.assertTrue(any(rule == "deviation-shape" for rule, _, _ in c_standard.deviation_findings(root)))

    def test_retained_application_is_exact_and_both_sides_are_checked(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / "core/tests/fixtures/component-pipeline/applications/Ingress.c"
            source.parent.mkdir(parents=True)
            document = root / "docs/standards/test.md"
            document.parent.mkdir(parents=True)
            marker = "/* MR12 RULE 8.7 VIOLATION: retain the approved provided runnable. */\n"
            original = marker + "/** Existing API contract. */\nAPP_CODE int Ingress_ReadData(int value) { return value; }\n"
            record = (
                "| MISRA C:2012 Rule 8.7 | `core/tests/fixtures/component-pipeline/applications/Ingress.c` 中 `Ingress_ReadData` | "
                "shape=`APP_CODE int Ingress_ReadData(int value) {` | 维护者 2026-10-30精确124键决定 |\n"
            )
            # An unrelated fixture remains outside this narrow source checker.
            source.with_name("Other.c").write_text("/* MR12 unreviewed test text */\n", encoding="utf-8")
            for label, code, table, expected in (
                ("accepted", original, record, None),
                ("missing-comment", original.replace(marker, ""), record, "deviation-uncommented"),
                ("missing-record", original, "", "deviation-unrecorded"),
                ("wrong-rule", original.replace("RULE 8.7", "RULE 8.9"), record, "deviation-unrecorded"),
                ("wrong-owner", original.replace("Ingress_ReadData", "OtherRunnable"), record, "deviation-unrecorded"),
                ("wrong-file", original, record.replace("Ingress.c", "Other.c"), "deviation-unrecorded"),
                ("outside-entity", original + marker + "int Extra(void) { return 0; }\n", record, "deviation-unrecorded"),
            ):
                with self.subTest(case=label):
                    source.write_text(code, encoding="utf-8")
                    document.write_text(table, encoding="utf-8")
                    findings = c_standard.deviation_findings(root)
                    if expected is None:
                        self.assertEqual(findings, {})
                    else:
                        self.assertTrue(any(rule == expected for rule, _, _ in findings), findings)

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

    def test_header_file_scope_is_bound_to_directive_and_first_file_comment(self):
        with tempfile.TemporaryDirectory(prefix="header-deviation-") as temporary:
            root = Path(temporary)
            source = root / "runtime/os/include/Test_MemMap.h"
            source.parent.mkdir(parents=True)
            document = root / "docs/standards/test.md"
            document.parent.mkdir(parents=True)
            record = ("| MISRA C:2012 Directive 4.10 | `runtime/os/include/Test_MemMap.h` 中 "
                      "`@file` | repeated requests | Approved by maintainer |\n")
            marker = "MR12 DIR 4.10 VIOLATION: repeated mapping requests require re-entry."
            code = ("/** @file Re-entrant mapping.\n * " + marker + "\n */\n"
                    "#if defined(START)\n#undef START\n#endif\n")
            document.write_text(record, encoding="utf-8")
            source.write_text(code, encoding="utf-8")
            self.assertEqual(c_standard.deviation_findings(root), {})
            cases = (
                ("unapproved", record.replace("Approved by maintainer", "Pending"), code,
                 "deviation-unrecorded"),
                ("duplicate-record", record + record, code, "deviation-duplicate"),
                ("duplicate-comment", record, code.replace(marker, marker + "\n * " + marker),
                 "deviation-duplicate"),
                ("wrong-file", record.replace("Test_MemMap.h", "Other.h"), code,
                 "deviation-unrecorded"),
                ("wrong-rule", record.replace("4.10", "4.8"), code.replace("4.10", "4.8"),
                 "deviation-unrecorded"),
                ("wrong-kind", record.replace("Directive", "Rule"), code.replace("DIR", "RULE"),
                 "deviation-unrecorded"),
                ("not-file-comment", record, code.replace("@file", "@brief"), "deviation-unrecorded"),
                ("not-first", record, "#define OTHER 1\n" + code, "deviation-unrecorded"),
                ("stale", record, code.replace(marker, "No deviation marker."), "deviation-uncommented"),
            )
            for name, recorded, text, required in cases:
                with self.subTest(case=name):
                    document.write_text(recorded, encoding="utf-8")
                    source.write_text(text, encoding="utf-8")
                    if name in {"duplicate-record", "stale"}:
                        location = ("docs/standards/test.md", recorded.splitlines()[0])
                    else:
                        location = ("runtime/os/include/Test_MemMap.h",
                                    "/** @brief Re-entrant mapping." if name == "not-file-comment"
                                    else "/** @file Re-entrant mapping.")
                    self.assertIn((required, *location), c_standard.deviation_findings(root))
            source.unlink()
            source = source.with_suffix(".c")
            source.write_text(code, encoding="utf-8")
            document.write_text(record.replace("Test_MemMap.h", "Test_MemMap.c"), encoding="utf-8")
            self.assertIn(("deviation-unrecorded", "runtime/os/include/Test_MemMap.c",
                           "/** @file Re-entrant mapping."), c_standard.deviation_findings(root))

    def test_host_address_category_requires_exact_owner_expression_and_annotation(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            directory = root / "runtime/os/src/host/linux"
            directory.mkdir(parents=True)
            document = root / "docs/standards/os/native-facilities/addresses.md"
            document.parent.mkdir(parents=True)
            document.write_text("## 主机地址表示类别处置\n", encoding="utf-8")
            host = directory / "Os_HostLinux.c"
            stack = directory / "Os_StackLinux.c"
            mapping = "thread->mapping = mmap(NULL, reserve, (int)protection, (int)mapping_flags, -1, 0);"
            bounds = "*low = (uintptr_t)thread->mapping;"
            marker = "/* MR12 RULE 11.5 VIOLATION: page-aligned mapping bytes. */\n"
            original = (
                "HANDLE Os_HostCreateActor(void) {\n" + marker + mapping + "\n}\n"
                "int Os_HostStackRange(void) {\n"
                "/* MR12 RULE 11.4 VIOLATION: numeric bounds only. */\n" + bounds + "\n}\n"
            )
            stack.write_text(
                "void Os_StackRecordBuffer(void) {\n"
                "/* MR12 RULE 11.4 VIOLATION: disjoint stack regions. */\n"
                "current_stack->kernel_buffer_high = (uintptr_t)&top[1];\n}\n",
                encoding="utf-8",
            )
            for label, changed, expected_rule, expected_file in (
                ("accepted", original, None, None),
                ("missing-comment", original.replace(marker, ""), "deviation-uncommented", document),
                ("wrong-rule", original.replace("RULE 11.5", "RULE 11.4"), "deviation-unrecorded", host),
                ("wrong-expression", original.replace(bounds, "*low = (uintptr_t)thread->argument;"),
                 "deviation-category-pattern", host),
                ("wrong-owner", original.replace("Os_HostStackRange", "OtherStackRange"),
                 "deviation-category-pattern", host),
                ("outside-owner", original.replace(bounds, "") + bounds,
                 "deviation-category-pattern", host),
                ("duplicate", original.replace(bounds, bounds + bounds), "deviation-category-pattern", host),
                ("comment-owner", original.replace("int Os_HostStackRange(void) {", "/* Os_HostStackRange(void) { */ int Other(void) {"),
                 "deviation-category-pattern", host),
                ("extra-conversion", original + "void Other(void) {\n" + marker + mapping + "\n}\n",
                 "deviation-unrecorded", host),
            ):
                with self.subTest(case=label):
                    host.write_text(changed, encoding="utf-8")
                    findings = c_standard.deviation_findings(root)
                    if expected_rule is None:
                        self.assertEqual(findings, {})
                    else:
                        self.assertTrue(any(
                            rule == expected_rule and name == expected_file.relative_to(root).as_posix()
                            for rule, name, _ in findings
                        ), findings)
            host.write_text(original, encoding="utf-8")
            document.unlink()
            self.assertIn(
                ("deviation-category-record", "runtime/os/src/host/linux/Os_HostLinux.c", "Os_HostCreateActor"),
                c_standard.deviation_findings(root),
            )

    def test_windows_address_category_keeps_each_expression_bounded(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / "runtime/os/src/Os_Stack.c"
            source.parent.mkdir(parents=True)
            document = root / "docs/standards/os/native-facilities/addresses.md"
            document.parent.mkdir(parents=True)
            document.write_text("## 主机地址表示类别处置\n", encoding="utf-8")
            marker = "/* MR12 RULE 11.6 VIOLATION: native address metadata only. */\n"
            bounds = "record->reserve_low = (uintptr_t)region.AllocationBase;"
            original = (
                "void Os_StackRecordBuffer(void) {\n"
                "/* MR12 RULE 11.4 VIOLATION: disjoint numeric stack bounds. */\n"
                "current_stack->kernel_buffer_high = (uintptr_t)&top[1];\n}\n"
                "int Os_StackRegister(void) {\n" + marker + bounds + "\n"
                + marker + "if ((VirtualQuery((const void *)cursor, &region, sizeof(region)) != sizeof(region)) ||\n"
                "(region.RegionSize == 0u)) { return 0; }\n"
                "if (region.AllocationBase != allocation_base) { break; }\n}\n"
                "void Os_StackCheck(void) {\n" + marker
                + "invalid = VirtualQuery((const void *)sp, &region, sizeof(region)) != sizeof(region);\n}\n"
                "void Os_StackObserve(void) {\n" + marker
                + "invalid = VirtualQuery((const void *)sp, &region, sizeof(region)) != sizeof(region);\n}\n"
            )
            for label, changed, expected in (
                ("accepted-five", original, None),
                ("missing-comment", original.replace(marker + bounds, bounds), "deviation-uncommented"),
                ("wrong-rule", original.replace("RULE 11.6", "RULE 11.4", 1), "deviation-unrecorded"),
                ("outside-category", original.replace("(uintptr_t)region.AllocationBase;", "(uintptr_t)region.BaseAddress;"),
                 "deviation-category-pattern"),
                ("wrong-owner", original.replace("Os_StackObserve", "OtherObserver"), "deviation-category-pattern"),
                ("duplicate-expression", original.replace(bounds, bounds + bounds), "deviation-category-pattern"),
                ("extra-conversion", original + "void Other(void) {\n" + marker + bounds + "\n}\n",
                 "deviation-unrecorded"),
                ("allocation-identity-not-authorized", original.replace(
                    "if (region.AllocationBase != allocation_base)",
                    marker + "if ((uintptr_t)region.AllocationBase != record->reserve_low)"),
                 "deviation-unrecorded"),
            ):
                with self.subTest(case=label):
                    source.write_text(changed, encoding="utf-8")
                    findings = c_standard.deviation_findings(root)
                    if expected is None:
                        self.assertEqual(findings, {})
                        instances, errors = c_standard.host_address_instances(root)
                        self.assertEqual(len(instances), 5)
                        self.assertEqual(errors, {})
                        self.assertEqual(len({entry[1] for entry in instances.values()}), 5)
                    else:
                        self.assertTrue(any(rule == expected for rule, _, _ in findings), findings)

    def test_object_deviations_bind_declarators_instead_of_members_or_attributes(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / "runtime/bsw/src/Test.c"
            source.parent.mkdir(parents=True)
            document = root / "docs/standards/test.md"
            document.parent.mkdir(parents=True)
            marker = "/* MR12 RULE 8.9 VIOLATION: mapped storage must remain file static. */\n"
            source.write_text(
                marker + "static struct { int member; } state TEST_VAR_CLEARED;\n"
                + marker + 'OS_MEMMAP static const char bytes[] = "state";\n'
                + marker + "OS_MEMMAP static int counter;\n", encoding="utf-8",
            )
            record = (
                "| MISRA C:2012 Rule 8.9 | `runtime/bsw/src/Test.c` 中 `state`、`bytes`、`counter` | "
                "mapped objects | Approved by maintainer |\n"
            )
            document.write_text(record, encoding="utf-8")
            self.assertEqual(c_standard.deviation_findings(root), {})
            for wrong in ("member", "TEST_VAR_CLEARED"):
                with self.subTest(wrong=wrong):
                    document.write_text(record.replace("`state`", f"`{wrong}`"), encoding="utf-8")
                    findings = c_standard.deviation_findings(root)
                    self.assertEqual({key[0] for key in findings},
                                     {"deviation-unrecorded", "deviation-uncommented"})
                    self.assertIn(("deviation-unrecorded", "runtime/bsw/src/Test.c", marker.strip()), findings)

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


class ManualAssessmentTests(unittest.TestCase):
    # Derived from the hash-matched 2.21.0 addon/core checker catalog, not from
    # the documents under test. Production grouping uses coverage.rules.
    UNCOVERED = frozenset([
        "D1.1", "D2.1", "D3.1", "D4.1", "D4.2", "D4.3", "D4.4", "D4.5", "D4.6", "D4.7", "D4.8", "D4.9",
        "D4.10", "D4.11", "D4.12", "D4.13", "D4.14", "D4.15", "D5.1", "D5.2", "D5.3",
        "R1.1", "R1.5", "R2.8", "R3.2", "R6.3", "R7.5", "R7.6", "R8.15", "R8.16", "R8.17", "R9.6", "R9.7",
        "R11.10", "R12.5", "R12.6", "R17.9", "R17.10", "R17.11", "R17.12", "R17.13", "R18.9", "R18.10",
        "R21.22", "R21.23", "R21.24", "R21.25", "R21.26", "R22.11", "R22.12", "R22.13", "R22.14",
        "R22.15", "R22.16", "R22.17", "R22.18", "R22.19", "R22.20",
        "R23.1", "R23.2", "R23.3", "R23.4", "R23.5", "R23.6", "R23.7", "R23.8",
    ])

    def setUp(self):
        inventory = c_standard.ROOT / ".agents/skills/misra-c2012/references/baseline-index.json"
        self.rules = [{"id": row["id"], "category": row["category"],
                       "automaticChecker": row["id"] not in self.UNCOVERED,
                       "assessment": "not_assessed"}
                      for row in json.loads(inventory.read_text())["guideline_metadata"]]
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        for name in c_standard.MANUAL_ASSESSMENT_SOURCES:
            destination = self.root / name
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(c_standard.ROOT / name, destination)

    def summary(self):
        return c_standard.manual_assessment(self.rules, self.root)

    def test_catalog_partition_and_raw_status_are_preserved(self):
        original = copy.deepcopy(self.rules)
        result = self.summary()
        manual = result["checkerUncovered"]
        self.assertEqual((manual["total"], result["checkerCovered"]["total"]), (66, 155))
        self.assertEqual({row["id"] for row in manual["records"]}, self.UNCOVERED)
        self.assertEqual(manual["classified"] + manual["unclassified"], 66)
        self.assertGreater(manual["unclassified"], 0)
        self.assertFalse(manual["classificationComplete"])
        self.assertEqual(result["conformanceAssessment"], "not_established")
        self.assertEqual(self.rules, original)

    def test_normal_coverage_entry_connects_records_without_assessing_raw_catalog(self):
        from autosar_tooling.c_check import coverage

        with tempfile.TemporaryDirectory() as temporary:
            addons = Path(temporary)
            available = [row["id"][1:] for row in self.rules if row["automaticChecker"]]
            (addons / "misra.py").write_text(
                "def getAddonRules(): return " + repr(available) + "\ndef getCppcheckRules(): return []\n"
            )
            result = coverage(addons)
        self.assertEqual(len(result["rules"]), 221)
        self.assertTrue(all(row["assessment"] == "not_assessed" for row in result["rules"]))
        self.assertFalse(result["completeCompliance"])
        manual = result["manualAssessment"]
        self.assertEqual(manual["format"], "misra-manual-assessment-v1")
        self.assertEqual(manual["checkerCovered"]["total"], 155)
        self.assertEqual(manual["checkerUncovered"]["total"], 66)
        self.assertFalse(manual["checkerUncovered"]["classificationComplete"])
        self.assertTrue(result["knownCheckerLimitations"])

    def test_missing_record_remains_unclassified(self):
        name = c_standard.MANUAL_ASSESSMENT_SOURCES[2]
        path = self.root / name
        before = self.summary()
        text = path.read_text()
        row = next(line for line in text.splitlines() if line.startswith("| R7.5 |"))
        path.write_text(text.replace(row + "\n", ""))
        after = self.summary()
        self.assertEqual(after["checkerUncovered"]["unclassified"], before["checkerUncovered"]["unclassified"] + 1)
        missing = next(row for row in after["checkerUncovered"]["records"] if row["id"] == "R7.5")
        self.assertEqual(missing["status"], "not_assessed")

    def test_duplicate_covered_category_status_and_block_controls(self):
        path = self.root / c_standard.MANUAL_ASSESSMENT_SOURCES[2]
        text = path.read_text()
        row = next(line for line in text.splitlines() if line.startswith("| R7.5 |"))
        mutations = (
            (text.replace(row, row + "\n" + row), "duplicate manual"),
            (text.replace("| R7.5 |", "| R7.4 |"), "checker-covered or unknown"),
            (text.replace("| R7.5 | Mandatory |", "| R7.5 | Advisory |"), "category differs"),
            (text.replace(row, row.replace("satisfied", "retained")), "Invalid or duplicate"),
            (text.replace(row, row.replace("satisfied", "approved")), "Invalid or duplicate"),
            (text.replace("<!-- misra-manual-end -->", ""), "Unclosed manual"),
        )
        for mutant, message in mutations:
            with self.subTest(message=message):
                path.write_text(mutant)
                with self.assertRaisesRegex(ValueError, message):
                    self.summary()
        path.write_text(text)

    def test_record_bytes_and_selection_have_reproducible_identity(self):
        before = self.summary()["inputIdentity"]
        self.assertEqual(before, self.summary()["inputIdentity"])
        path = self.root / c_standard.MANUAL_ASSESSMENT_SOURCES[2]
        path.write_text(path.read_text() + "\nChanged explanatory input.\n")
        after = self.summary()["inputIdentity"]
        self.assertNotEqual(before["documents"], after["documents"])
        self.assertEqual(before["selectionSha256"], after["selectionSha256"])
        self.assertRegex(after["parserSha256"], r"^[0-9a-f]{64}$")

    def test_record_line_endings_preserve_conclusions_but_change_byte_identity(self):
        before = self.summary()
        path = self.root / c_standard.MANUAL_ASSESSMENT_SOURCES[2]
        path.write_bytes(path.read_text().replace("\n", "\r\n").encode("utf-8"))
        after = self.summary()
        self.assertEqual(before["checkerUncovered"], after["checkerUncovered"])
        self.assertNotEqual(before["inputIdentity"]["documents"], after["inputIdentity"]["documents"])


if __name__ == "__main__":
    unittest.main()
