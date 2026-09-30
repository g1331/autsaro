"""Independent R24-11 ARTI consumer. Writes only a caller-owned temporary C header."""
import re
import sys
from pathlib import Path
from zipfile import ZipFile
from lxml import etree

NS = "http://autosar.org/schema/r4.0"
Q = "{" + NS + "}"

def identity(node):
    names = []
    for ancestor in list(node.iterancestors())[::-1] + [node]:
        name = ancestor.findtext(Q + "SHORT-NAME")
        if name:
            names.append(name)
    return "/" + "/".join(names)

def named(root):
    return {identity(node): node for node in root.iter() if isinstance(node.tag, str) and node.find(Q + "SHORT-NAME") is not None}

def check(project, root, os_name, header):
    with ZipFile(root / "docs/official/R24-11/CP/MethodologyAndTemplates/AUTOSAR_CP_MOD_ECUConfigurationParameters.zip") as archive:
        definitions = named(etree.fromstring(archive.read("AUTOSAR_CP_MOD_ECUConfigurationParameters.arxml")))
    description = etree.parse(str(project / "os/Os_Arti.arxml")).getroot()
    objects = {}
    for file in [project / "os/Os_Arti.arxml", *sorted((project / "inputs").glob("*.arxml"))]:
        for path, node in named(etree.parse(str(file)).getroot()).items():
            assert path not in objects, ("duplicate", path)
            objects[path] = node
    for node in description.iter():
        if not isinstance(node.tag, str):
            continue
        for reference in node:
            if not isinstance(reference.tag, str) or not reference.get("DEST"):
                continue
            target = definitions.get(reference.text) if reference.tag == Q + "DEFINITION-REF" else objects.get(reference.text)
            assert target is not None, ("broken reference", reference.text)
            assert etree.QName(target).localname == reference.get("DEST"), ("wrong reference kind", reference.text)
        definition_ref = node.find(Q + "DEFINITION-REF")
        if definition_ref is None:
            continue
        definition = definitions[definition_ref.text]
        if node.tag in (Q + "ECUC-NUMERICAL-PARAM-VALUE", Q + "ECUC-TEXTUAL-PARAM-VALUE", Q + "ECUC-REFERENCE-VALUE"):
            parent = node.getparent().getparent()
            owner = parent.findtext(Q + "DEFINITION-REF")
            assert definition_ref.text.rsplit("/", 1)[0] == owner, ("wrong owner", definition_ref.text, owner)
            value = node.findtext(Q + "VALUE")
            if value is not None and definition.tag == Q + "ECUC-ENUMERATION-PARAM-DEF":
                literals = definition.findall(".//" + Q + "ECUC-ENUMERATION-LITERAL-DEF/" + Q + "SHORT-NAME")
                assert value in [literal.text for literal in literals], ("unknown enum", definition_ref.text, value)
            if node.tag == Q + "ECUC-REFERENCE-VALUE":
                destination = definition.findtext(Q + "DESTINATION-REF")
                actual = objects[node.findtext(Q + "VALUE-REF")]
                if destination is not None:
                    assert actual.findtext(Q + "DEFINITION-REF") == destination, ("wrong destination definition", definition_ref.text)
        if node.tag in (Q + "ECUC-CONTAINER-VALUE", Q + "ECUC-MODULE-CONFIGURATION-VALUES"):
            for group, values in [("PARAMETERS", "PARAMETER-VALUES"), ("REFERENCES", "REFERENCE-VALUES")]:
                supplied = [child.findtext(Q + "DEFINITION-REF") for child in node.findall(Q + values + "/*")]
                for field in definition.findall(Q + group + "/*"):
                    if not isinstance(field.tag, str):
                        continue
                    count = supplied.count(identity(field))
                    low = int(field.findtext(Q + "LOWER-MULTIPLICITY") or "0")
                    high = field.findtext(Q + "UPPER-MULTIPLICITY")
                    assert count >= low and (high is None or count <= int(high)), ("multiplicity", identity(node), identity(field), count, low, high)
    nodes = description.findall(".//" + Q + "ECUC-CONTAINER-VALUE")
    definition_suffix = lambda node: node.findtext(Q + "DEFINITION-REF").rsplit("/", 1)[-1]
    task = [node for node in nodes if definition_suffix(node) == "ArtiOsTaskInstance"]
    assert len(task) == 1
    refs = task[0].findall(Q + "REFERENCE-VALUES/*")
    assert any(ref.findtext(Q + "VALUE-REF") == f"/Configuration/{os_name}/Task_Ecu" for ref in refs)
    assert task[0].findtext(Q + "PARAMETER-VALUES/" + Q + "ECUC-NUMERICAL-PARAM-VALUE/" + Q + "VALUE") == "0"
    isrs = [node for node in nodes if definition_suffix(node) == "ArtiOsIsrInstance"]
    assert len(isrs) == 1
    assert isrs[0].findtext(Q + "PARAMETER-VALUES/" + Q + "ECUC-NUMERICAL-PARAM-VALUE/" + Q + "VALUE") == "30"
    assert not any("Spinlock" in node.findtext(Q + "DEFINITION-REF") for node in nodes)
    hooks = [node for node in nodes if definition_suffix(node) == "ArtiHook"]
    events = []
    for hook in hooks:
        params = {p.findtext(Q + "DEFINITION-REF").rsplit("/", 1)[-1]: p.findtext(Q + "VALUE") for p in hook.findall(Q + "PARAMETER-VALUES/*")}
        assert params["ArtiHookInstance"] == os_name
        assert params["ArtiHookContext"] in ("NOSUSP", "SPRVSR", "USER")
        event = params["ArtiHookEventName"]
        assert event not in events, ("duplicate hook", event)
        events.append(event)
    mandatory = ["OsTask_" + name for name in ("Activate", "Start", "Preempt", "Wait", "Release", "Terminate")]
    mandatory += ["OsCat2Isr_Start", "OsCat2Isr_Stop"]
    mandatory += [f"OsHook_{hook}_{edge}" for hook in ("ErrorHook", "PreTaskHook", "PostTaskHook", "StartupHook", "ShutdownHook") for edge in ("Start", "Return")]
    services = ("GetISRID ControlIdle isOsStarted GetTaskID GetTaskState ActivateTask TerminateTask ChainTask GetResource ReleaseResource Schedule WaitEvent ClearEvent SetEvent GetEvent ShutdownOS StartOS GetActiveApplicationMode IncrementCounter GetCounterValue GetElapsedValue GetAlarmBase GetAlarm SetRelAlarm SetAbsAlarm CancelAlarm StartScheduleTableRel StartScheduleTableAbs StopScheduleTable NextScheduleTable GetScheduleTableStatus DisableInterruptSource EnableInterruptSource ClearPendingInterrupt EnableAllInterrupts DisableAllInterrupts ResumeAllInterrupts SuspendAllInterrupts ResumeOSInterrupts SuspendOSInterrupts").split()
    mandatory += [f"OsServiceCall_{service}_{edge}" for service in services for edge in ("Start", "Return") if not (service in ("StartOS", "ShutdownOS") and edge == "Return")]
    assert set(events) == set(mandatory), ("missing/extra hook", set(mandatory) - set(events), set(events) - set(mandatory))
    binding = (project / "os/src/Os_Arti.c").read_text()
    for event in events:
        assert re.search(r"ARTI_TRACE\(\s*(?:NOSUSP|SPRVSR|USER),\s*AR_CP_OS_\w+,\s*" + re.escape(os_name) + r",\s*0u,\s*" + re.escape(event) + r",", binding), ("missing literal hook", event)
    expressions = [node.findtext(Q + "PARAMETER-VALUES/" + Q + "ECUC-TEXTUAL-PARAM-VALUE/" + Q + "VALUE") for node in nodes if definition_suffix(node) == "ArtiExpression"]
    assert expressions and not any(re.search(r"\b[A-Za-z_]\w*\s*\(", value) for value in expressions), "function in debugger expression"
    c = ["/* Compiled by an independent consumer against delivered headers. */", "static void arti_expression_probe(void) {", "    volatile uintptr_t observed = 0u;"]
    for value in expressions:
        c.append("    observed = (uintptr_t)(" + value + ");")
    c += ["    (void)observed;", "}", ""]
    header.write_text("\n".join(c), encoding="utf-8")
    print(f"ARTI description PASS: hooks={len(events)} expressions={len(expressions)}")

if __name__ == "__main__":
    check(Path(sys.argv[1]), Path(sys.argv[2]), sys.argv[3], Path(sys.argv[4]))
