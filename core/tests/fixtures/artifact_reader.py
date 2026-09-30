"""Independent consumer of delivered host BSW/RTE implementation descriptions."""

import re
import sys
from pathlib import Path
from xml.etree import ElementTree as ET

NS = {"a": "http://autosar.org/schema/r4.0"}


def text(node, path):
    found = node.find(path, NS)
    assert found is not None and found.text, f"missing {path}"
    return found.text


def artifact_path(descriptor):
    return text(descriptor, "a:DOMAIN").replace(".", "/") + "/" + text(descriptor, "a:SHORT-LABEL")


def inspect(project, probe):
    root = ET.parse(project / "descriptions/Host_Implementation.arxml").getroot()
    objects = {}

    def visit(node, parent):
        name = node.find("a:SHORT-NAME", NS)
        path = parent
        if name is not None:
            path += "/" + name.text
            assert path not in objects, f"duplicate object {path}"
            objects[path] = node
        for child in node:
            visit(child, path)

    visit(root, "")
    for node in root.iter():
        if node.tag.endswith("REF"):
            target = objects.get(node.text)
            assert target is not None, f"broken reference {node.text}"
            assert target.tag.rsplit("}", 1)[-1] == node.attrib["DEST"], "reference kind"
    modules = root.findall(".//a:BSW-MODULE-DESCRIPTION", NS)
    assert {text(m, "a:SHORT-NAME") for m in modules} == {
        "Can", "CanIf", "PduR", "Com", "CanTp", "Dcm", "Dem", "NvM", "LSduR",
        "Security", "Os", "Arti", "Rte",
    }, "actual module roles"
    implementations = root.findall(".//a:BSW-IMPLEMENTATION", NS)
    assert len(implementations) == 13
    for impl in implementations:
        assert text(impl, "a:PROGRAMMING-LANGUAGE") == "C"
        assert text(impl, "a:AR-RELEASE-VERSION") == "4.10.0"
        assert text(impl, "a:SW-VERSION") == ("1.0.0" if text(impl, "a:SHORT-NAME") == "ArtiHostImplementation" else "0.1.0")
        paths = impl.findall(".//a:AUTOSAR-ENGINEERING-OBJECT", NS)
        assert paths
        for descriptor in paths:
            name = artifact_path(descriptor)
            assert not Path(name).is_absolute() and ":" not in name and ".." not in name.split("/")
            assert (project / name).is_file(), f"missing source dependency {name}"
            assert text(descriptor, "a:CATEGORY") == ("SWSRC" if name.endswith(".c") else "SWHDR")
    rte = objects["/HostArtifacts/RteHostImplementation"]
    generated = rte.findall("a:GENERATED-ARTIFACTS/a:DEPENDENCY-ON-ARTIFACT", NS)
    actual = {artifact_path(d.find("a:ARTIFACT-DESCRIPTOR", NS)) for d in generated}
    expected = {"src/Rte.c", "src/Rte_OsService.c"}
    expected |= {p.relative_to(project).as_posix() for p in (project / "include").glob("Rte*.h")}
    assert actual == expected, "RTE generated file closure"
    assert all(text(d, "a:USAGES/a:USAGE") == "COMPILE" for d in generated)
    for module in ("Os", "Rte"):
        section = objects[f"/HostArtifacts/{module}HostImplementation/CodeMapping/CODE"]
        assert text(section, "a:SYMBOL") == "CODE"
    entries = root.findall(".//a:BSW-MODULE-ENTRY", NS)
    expected_rte = {
        "Rte_Read_RxValue_Value", "Rte_Write_TxValue_Value", "Rte_Call_ApplicationValue_ReadData",
        "DcmService_ReadData", "Com_SendSignal", "Com_ReceiveSignal",
        "Ecu_TargetInitializeRte", "Ecu_TargetReadDid", "OsService_SystemCounter_GetCounterValue",
        "OsService_SystemCounter_GetElapsedValue", "Rte_Call_OsService_GetCounterValue",
        "Rte_Call_OsService_GetElapsedValue",
    }
    rte_refs = objects["/HostArtifacts/Rte"].findall("a:IMPLEMENTED-ENTRYS/a:BSW-MODULE-ENTRY-REF-CONDITIONAL/a:BSW-MODULE-ENTRY-REF", NS)
    assert {n.text.rsplit("/", 1)[-1] for n in rte_refs} == expected_rte, "actual RTE entries"
    assert len(entries) == 29, "selected 17 BSW and 12 RTE contracts"
    expected_bsw = {
        "Can": {"Can_Init", "Can_MainFunction_Wakeup", "Can_MainFunction_Read", "Can_MainFunction_Write"},
        "CanIf": {"CanIf_Init", "CanIf_Transmit"},
        "PduR": {"PduR_Init", "PduR_Transmit"},
        "Com": {"Com_Init", "Com_AdvanceTime", "Com_GetSignal", "Com_SetSignal", "Com_TriggerTransmit"},
        "CanTp": {"CanTp_Init", "CanTp_AdvanceTime"},
        "Dcm": {"Dcm_Init", "Dcm_AdvanceTime"},
    }
    for module, expected in expected_bsw.items():
        refs = objects[f"/HostArtifacts/{module}"].findall("a:IMPLEMENTED-ENTRYS/a:BSW-MODULE-ENTRY-REF-CONDITIONAL/a:BSW-MODULE-ENTRY-REF", NS)
        assert {r.text.rsplit("/", 1)[-1] for r in refs} == expected, f"actual BSW entries {module}"
    headers = ["Can.h", "CanIf.h", "PduR.h", "Com.h", "CanTp.h", "Dcm.h", "SchM_Can.h",
               "Rte_EchoApplication.h", "Rte_DcmService.h", "Ecu_Target.h", "Ecu_TargetConfig.h", "Rte_Os.h"]
    source = "#include <limits.h>\n" + "\n".join(f'#include "{h}"' for h in headers) + "\n"

    def native(node):
        ref = text(node, "a:SW-DATA-DEF-PROPS/a:SW-DATA-DEF-PROPS-VARIANTS/a:SW-DATA-DEF-PROPS-CONDITIONAL/a:BASE-TYPE-REF")
        return text(objects[ref], "a:NATIVE-DECLARATION")

    for node in root.findall(".//a:SW-BASE-TYPE", NS):
        native_type = text(node, "a:NATIVE-DECLARATION")
        bits = text(node, "a:BASE-TYPE-SIZE")
        name = text(node, "a:SHORT-NAME")
        source += f"typedef char size_{name}[(sizeof({native_type}) * CHAR_BIT == {bits}) ? 1 : -1];\n"
    for index, entry in enumerate(entries):
        name = text(entry, "a:SHORT-NAME")
        assert re.fullmatch(r"[A-Za-z_][A-Za-z_0-9]*", name)
        result = entry.find("a:RETURN-TYPE", NS)
        return_type = "void" if result is None else native(result)
        args = entry.findall("a:ARGUMENTS/a:SW-SERVICE-ARG", NS)
        parameters = ", ".join(native(a) for a in args) or "void"
        source += f"static {return_type} (*const entry_{index})({parameters}) = &{name};\n"
    source += "static void check_artifact_entries(void) {\n"
    for index in range(len(entries)):
        source += f"    if (entry_{index} == 0) ExitProcess(7u);\n"
    source += "}\n"
    probe.write_text(source, encoding="utf-8")
    print("artifact_reader modules=13 entries=29 rte_closure=exact refs=closed")


if __name__ == "__main__":
    inspect(Path(sys.argv[1]), Path(sys.argv[2]))
