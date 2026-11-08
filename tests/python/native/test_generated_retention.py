"""Exact generated interfaces selected by the bounded Issue382 project decision."""

from __future__ import annotations

import json
import os
import re
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from autosar_tooling.c_retention import (
    PROFILES,
    TARGETS,
    check_cohort,
    check_project,
    records,
)
from autosar_tooling.c_standard import lex
from ecu_tools import build as ecu_build
from ecu_tools.process import OwnedProcess, ProcessSpec

ROOT = Path(__file__).resolve().parents[3]
def execute(argv: list[str], directory: Path, label: str, timeout: int = 60):
    (directory / "logs").mkdir(mode=0o700, exist_ok=True)
    result = OwnedProcess(
        ProcessSpec.seconds(argv, directory, timeout, directory / "logs", label)
    ).wait()
    output = result.stdout.read_text(errors="replace") + result.stderr.read_text(
        errors="replace"
    )
    return result, output


def compile_run(
    project: Path,
    directory: Path,
    source: str,
    label: str,
    cpp: bool = False,
    shadow: tuple[str, str] | None = None,
):
    target = json.loads((project / "target.json").read_text())
    path = directory / (label + (".cpp" if cpp else ".c"))
    path.write_text(source, encoding="utf-8")
    binary = directory / (label + (".exe" if os.name == "nt" else ""))
    argv = [os.environ["AUTOSAR_CC"], "-x", "c++" if cpp else "c"]
    argv += [
        "-std=c++11" if cpp and flag == "-std=c99" else flag
        for flag in target["compilerFlags"]
    ]
    if shadow:
        folder = directory / (label + "-headers")
        folder.mkdir()
        (folder / Path(shadow[0]).name).write_text(shadow[1], encoding="utf-8")
        argv.append("-I" + str(folder))
    argv += ["-I" + str(project / p) for p in target["includePaths"]]
    argv += [str(path), *(["-lstdc++"] if cpp else []), "-o", str(binary)]
    compiled, output = execute(argv, directory, label + "-compile")
    if not compiled.success:
        raise AssertionError(
            f"{label}: compile failed before behavioral control: {output}"
        )
    return execute([str(binary)], directory, label + "-run", 30)


def macro_value(
    row: dict, lookup: dict[str, dict], seen: tuple = ()
) -> tuple[int, int] | None:
    if row["name"].startswith(("Rte_Call_", "Rte_Inst_")):
        return None
    body = row["shape"].split(row["name"], 1)[1].strip()
    if match := re.fullmatch(r"UINT64_C\s*\(\s*(\d+)\s*\)", body):
        return int(match[1]), 8
    if match := re.fullmatch(r"(0x[0-9A-Fa-f]+|[0-9]+)([uUlL]*)", body):
        return int(match[1], 0), 8 if "ll" in match[2].lower() else 4
    if body in lookup and body not in seen:
        return macro_value(lookup[body], lookup, (*seen, row["name"]))
    raise AssertionError("No independent integer value/width oracle: " + row["name"])


def numeric_consumer(
    header: str, rows: list[dict], types: list[dict] | None = None
) -> str:
    lookup = {row["name"]: row for row in rows}
    text = f'#include "{Path(header).name}"\n#include <stdint.h>\n#include <stdio.h>\nint main(void) {{\n'
    for row in rows:
        value = macro_value(row, lookup)
        if value is None:
            continue
        number, width = value
        text += (
            f"    if ((uint64_t)({row['name']}) != UINT64_C({number}) || "
            f"sizeof({row['name']}) != {width}u) {{ "
            f'fprintf(stderr, "retained value/width {row["name"]}\\n"); return 1; }}\n'
        )
    for row in types or []:
        text += f'    if (sizeof({row["name"]}) != 1u) {{ fprintf(stderr, "retained type width {row["name"]}\\n"); return 2; }}\n'
    return text + "    return 0;\n}\n"


def changed_macro(source: str, name: str, replacement: str) -> str:
    matches = [
        t
        for t in lex(source)
        if re.match(r"\s*#\s*define\s+" + re.escape(name) + r"(?!\w)", t.text)
    ]
    assert len(matches) == 1
    token = matches[0]
    return source[: token.start] + replacement + source[token.end :]


def export_interfaces(project: Path, directory: Path, profile: str) -> None:
    scopes = (
        ["EchoApplication", "DcmService"]
        if profile in {"standard-ecu", "user-application"}
        else [
            "Ingress",
            "Process",
            "Observe",
            *(["DcmService"] if profile == "component-pipeline" else []),
        ]
    )
    scopes.append("ComMService")
    headers = "#define RTE_CORE\n" + "".join(
        f'#include "Rte_{scope}.h"\n' for scope in scopes
    )
    headers += '#include "Rte_Main.h"\n#include "SchM_Dcm.h"\n'
    provider = (
        headers
        + """#include "Ecu_Target.h"
#include "Com.h"
#include "Can_HostLock.h"
int Ecu_TargetIsOwner(void) { return 1; }
Std_ReturnType Ecu_TargetCheckLifecycleContext(boolean starting) { (void)starting; return E_OK; }
uint8 Com_SendSignal(Com_SignalIdType signal, const void *data) { (void)signal; (void)data; return E_OK; }
uint8 Com_ReceiveSignal(Com_SignalIdType signal, void *data) { (void)signal; (void)data; return COM_SERVICE_NOT_AVAILABLE; }
void Can_HostAreaEnter(Can_HostAreaType area) { (void)area; }
void Can_HostAreaExit(Can_HostAreaType area) { (void)area; }
"""
    )
    if "EchoApplication" in scopes:
        provider += "void EchoApplication_Periodic(void) {}\nStd_ReturnType EchoApplication_ReadData(Dcm_DataElement_ApplicationValueType Data) { Data[0] = 1u; return E_OK; }\n"
    else:
        provider += "void Ingress_Periodic(void) {}\nvoid Process_Periodic(void) {}\nvoid Observe_Periodic(void) {}\nvoid Process_Transform(uint32 Input, uint32 *Output, uint32 *State) { *Output = Input; *State = Input; }\n"
        if "DcmService" in scopes:
            provider += "Std_ReturnType Ingress_ReadData(Dcm_DataElement_ApplicationValueType Data) { Data[0] = 1u; return E_OK; }\n"
    stubs = directory / "retained-stubs.c"
    stubs.write_text(provider, encoding="utf-8")
    target = json.loads((project / "target.json").read_text())
    common = [
        os.environ["AUTOSAR_CC"],
        *target["compilerFlags"],
        *["-I" + str(project / p) for p in target["includePaths"]],
    ]
    objects = []
    for label, path in [
        ("rte", project / "generated/src/Rte.c"),
        ("schm", project / "generated/src/SchM_Main.c"),
        ("stubs", stubs),
    ]:
        obj = directory / (label + ".o")
        result, output = execute(
            [*common, "-c", str(path), "-o", str(obj)], directory, "exports-" + label
        )
        assert result.success, output
        objects.append(str(obj))
    for cpp in [False, True]:
        source = headers + "#include <stdio.h>\n"
        if "DcmService" in scopes:
            source += (
                '#ifdef __cplusplus\nextern "C"\n#endif\nint retained_alias(void);\n'
            )
            alias = directory / ("alias.cpp" if cpp else "alias.c")
            alias.write_text(
                """#include "Rte_DcmService.h"
#ifdef __cplusplus
extern "C"
#endif
int retained_alias(void) {
    Std_ReturnType (*volatile short_call)(Dcm_DataElement_ApplicationValueType) = Rte_Call_DataServices_ApplicationValue_ReadData;
    Std_ReturnType (*volatile qualified_call)(Dcm_DataElement_ApplicationValueType) = Rte_Call_DcmService_DataServices_ApplicationValue_ReadData;
    Dcm_DataElement_ApplicationValueType data = {0u, 0u, 0u, 0u};
    return short_call == qualified_call && short_call(data) == E_OK && data[0] == 1u ? 0 : 1;
}
""",
                encoding="utf-8",
            )
        for scope in scopes:
            source += (
                f"static const Rte_CDS_{scope} *const initial_{scope} = Rte_Inst_{scope};\n"
            )
        source += """int main(void) {
    if (Rte_Start() != E_OK) { return 1; }
    SchM_Init(&Ecu_SchMConfig); SchM_Start(); SchM_StartTiming();
"""
        for scope in scopes:
            source += (
                f"    const Rte_CDS_{scope} *volatile instance_{scope} = Rte_Inst_{scope};\n"
                f"    if (instance_{scope} != &Rte_Instance_{scope} || "
                f"initial_{scope} != instance_{scope}) {{ return 2; }}\n"
            )
        source += """    Std_ReturnType (*volatile publish)(Rte_ModeType_DcmAuthenticationState_Physical) = SchM_Switch_Dcm_DcmAuthenticationState_Physical;
    Rte_ModeType_DcmAuthenticationState_Physical (*volatile authentication)(void) = SchM_Mode_Dcm_DcmAuthenticationState_Physical;
    Rte_ModeType_DcmDiagnosticSessionControl (*volatile session)(void) = SchM_Mode_Dcm_DcmDiagnosticSessionControl;
    if (publish(1u) != E_OK || authentication() != 1u || session() != 0u) { return 3; }
"""
        if "DcmService" in scopes:
            source += "    if (retained_alias() != 0) { return 4; }\n"
        source += "    SchM_Deinit(); return Rte_Stop() == E_OK ? 0 : 5;\n}\n"
        path = directory / ("exports.cpp" if cpp else "exports.c")
        path.write_text(source, encoding="utf-8")
        argv = [os.environ["AUTOSAR_CC"], "-x", "c++" if cpp else "c"]
        argv += [
            "-std=c++11" if cpp and flag == "-std=c99" else flag
            for flag in target["compilerFlags"]
        ]
        argv += ["-I" + str(project / p) for p in target["includePaths"]]
        consumer = directory / ("consumer-cpp.o" if cpp else "consumer-c.o")
        result, output = execute(
            [*argv, "-c", str(path), "-o", str(consumer)],
            directory,
            "exports-consumer-" + str(cpp),
        )
        assert result.success, output
        linked = [*objects, str(consumer)]
        if "DcmService" in scopes:
            alias_obj = directory / ("alias-cpp.o" if cpp else "alias-c.o")
            result, output = execute(
                [*argv, "-c", str(alias), "-o", str(alias_obj)],
                directory,
                "exports-alias-" + str(cpp),
            )
            assert result.success, output
            linked.append(str(alias_obj))
        binary = directory / (
            "exports-" + str(cpp) + (".exe" if os.name == "nt" else "")
        )
        result, output = execute(
            [
                os.environ["AUTOSAR_CC"],
                *linked,
                *(["-lstdc++"] if cpp else []),
                "-o",
                str(binary),
            ],
            directory,
            "exports-link-" + str(cpp),
        )
        assert result.success, output
        result, output = execute(
            [str(binary)], directory, "exports-run-" + str(cpp), 30
        )
        assert result.success, output


class GeneratedRetentionTests(unittest.TestCase):
    def test_exact_records_values_kinds_widths_and_set(self):
        all_rows = records()
        check_cohort(all_rows)
        altered = [*all_rows[:-1], {**all_rows[-1], "name": "UnapprovedName"}]
        with self.assertRaisesRegex(ValueError, "cohort identity"):
            check_cohort(altered)
        samples = Path(os.environ["AUTOSAR_C_ANALYSIS_SAMPLES"])
        self.assertEqual(
            {p.name for p in samples.iterdir() if p.is_dir()}, set(PROFILES.values())
        )
        for short, profile in PROFILES.items():
            project = samples / profile
            self.assertIn(
                json.loads((project / "target.json").read_text())["target"], TARGETS
            )
            selected = [r for r in all_rows if short in r["profiles"].split(",")]
            self.assertEqual(check_project(project, selected), [])
            macros = [r for r in selected if r["rule"] == "2.5"]
            with tempfile.TemporaryDirectory(prefix="generated-retention-") as tmp:
                directory = Path(tmp)
                for index, header in enumerate(
                    sorted({r["delivered"] for r in macros})
                ):
                    members = [r for r in macros if r["delivered"] == header]
                    if all(
                        macro_value(r, {m["name"]: m for m in members}) is None
                        for r in members
                    ):
                        continue
                    source = numeric_consumer(
                        header,
                        members,
                        [
                            r
                            for r in selected
                            if r["rule"] == "2.3" and r["delivered"] == header
                        ],
                    )
                    for cpp in [False, True]:
                        result, output = compile_run(
                            project,
                            directory,
                            source,
                            f"values-{index}-{int(cpp)}",
                            cpp,
                        )
                        self.assertTrue(result.success, f"{profile}: {output}")
                export_interfaces(project, directory, profile)
                for index, (header, name, replacement) in enumerate(
                    [
                        (
                            "generated/include/Rte.h",
                            "RTE_E_TIMEOUT",
                            "#define RTE_E_TIMEOUT 130U\n",
                        ),
                        (
                            "generated/include/Os_Counter.h",
                            "OSMAXALLOWEDVALUE_SystemCounter",
                            "#define OSMAXALLOWEDVALUE_SystemCounter 65535u\n",
                        ),
                    ]
                ):
                    original = (project / header).read_text()
                    changed = changed_macro(original, name, replacement)
                    self.assertIn(
                        (header, name, "shape"),
                        check_project(project, selected, {header: changed}),
                    )
                    members = [r for r in macros if r["delivered"] == header]
                    result, output = compile_run(
                        project,
                        directory,
                        numeric_consumer(header, members),
                        f"numeric-mutant-{index}",
                        shadow=(header, changed),
                    )
                    self.assertEqual(result.exit_code, 1, output)
                    self.assertIn("retained value/width " + name, output)
                header = "generated/include/Rte_Dcm_Type.h"
                name = "Rte_ModeType_DcmEcuReset"
                original = (project / header).read_text()
                self.assertEqual(original.count("typedef uint8 " + name + ";"), 1)
                changed = original.replace(
                    "typedef uint8 " + name + ";", "typedef uint16 " + name + ";"
                )
                self.assertIn(
                    (header, name, "shape"),
                    check_project(project, selected, {header: changed}),
                )
                members = [r for r in macros if r["delivered"] == header]
                types = [
                    r
                    for r in selected
                    if r["rule"] == "2.3" and r["delivered"] == header
                ]
                result, output = compile_run(
                    project,
                    directory,
                    numeric_consumer(header, members, types),
                    "type-width-mutant",
                    shadow=(header, changed),
                )
                self.assertEqual(result.exit_code, 2, output)
                self.assertIn("retained type width " + name, output)
                header = "generated/include/Rte.h"
                changed = changed_macro(
                    (project / header).read_text(),
                    "RTE_E_TIMEOUT",
                    "#define RTE_E_TIMEOUT() 129U\n",
                )
                self.assertIn(
                    (header, "RTE_E_TIMEOUT", "shape"),
                    check_project(project, selected, {header: changed}),
                )
                extra = (project / header).read_text() + (
                    "\n/* MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused. */\n"
                    "#define Unapproved_Retention 7u\n"
                )
                self.assertIn(
                    (header, "Unapproved_Retention", "cohort"),
                    check_project(project, selected, {header: extra}),
                )
                wrong_marker = (
                    (project / header)
                    .read_text()
                    .replace("MR12 RULE 2.5 VIOLATION:", "MR12 RULE 2.3 VIOLATION:")
                )
                self.assertIn(
                    (header, "RTE_E_TIMEOUT", "marker"),
                    check_project(project, selected, {header: wrong_marker}),
                )


def check_arti_address(project: Path, directory: Path, control: Path) -> None:
    """Use the actual GetAlarmBase consumer; only the compiler input is mutated."""
    project = project.resolve()
    directory = directory.resolve()
    control = control.resolve()
    original_query = ecu_build.query
    original = project / "generated/src/Os_Arti.c"
    source = original.read_text()
    needle = "const uint64 native_address = (uint64)address;"
    assert source.count(needle) == 1
    mutant = directory / "Os_Arti-truncated.c"
    mutant.write_text(
        source.replace(
            needle, "const uint64 native_address = (uint64)(uint32)(uintptr_t)address;"
        )
    )
    before = (project / "meta/files.sha256").read_bytes()
    for label in ["normal", "truncated"]:
        replaced = False

        def query(argv, logs, stage, variant=label, **options):
            nonlocal replaced
            if variant == "truncated" and stage == "native-c99-build":
                matches = [i for i, value in enumerate(argv) if Path(value) == original]
                assert len(matches) == 1
                argv = list(argv)
                argv[matches[0]] = str(mutant)
                replaced = True
            return original_query(argv, logs, stage, **options)

        with patch.object(ecu_build, "query", side_effect=query):
            binary = ecu_build.build(project, directory / label, "test", control)
        assert replaced == (label == "truncated")
        result, output = execute([str(binary), "Os"], directory, "arti-" + label, 60)
        if label == "normal":
            assert result.success and "arti_consumer os=Os" in output, output
        else:
            assert result.exit_code == 7 and "ARTI pointer record index=" in output, (
                output
            )
        assert (project / "meta/files.sha256").read_bytes() == before
