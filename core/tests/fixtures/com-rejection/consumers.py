"""Real normal mains, real lower refusals, delivered tool parser and compiled mutations."""
from __future__ import annotations

import hashlib
import json
import shutil
import sys
from pathlib import Path

sys.dont_write_bytecode = True
source, output = map(Path, sys.argv[1:])
sys.path.insert(0, str(source / "tools"))
from ecu_tools.build import build, query


def seal(project):
    names = (project / "meta/files.list").read_text().splitlines() + ["meta/files.list"]
    (project / "meta/files.sha256").write_text("".join(
        f"{hashlib.sha256((project / name).read_bytes()).hexdigest()}  {name}\n" for name in names))


def edit(project, name, before, after, count=1):
    path = project / name
    text = path.read_text()
    assert text.count(before) == count, (name, before, text.count(before))
    path.write_text(text.replace(before, after))


def run_case(name, mode, mutation=None, *, killed=False, output_failure=False, isr_records=False, capacity=False):
    directory = output / name
    directory.mkdir(mode=0o700, parents=True)
    project = directory / "project"
    shutil.copytree(source, project)
    # The fixed reference input has COM Tx/PduR source handle 1 and a 10 ms runnable.
    app = next((project / "app").rglob("Ingress.c"))
    text = app.read_text()
    text = '#include "Com.h"\n#include "CanIf.h"\n#include <assert.h>\n' + text
    needle = "    (void)Ingress_RecordActor();"
    assert text.count(needle) == 1
    text = text.replace(needle, needle + '\n    assert(CanIf_SetPduMode(0u, CANIF_TX_OFFLINE) == E_OK);\n    assert(Com_TriggerIPDUSend(1u) == E_OK);')
    app.write_text(text)
    observer = project / "integration/src/Ecu_ComRejection.c"
    text = observer.read_text()
    text = '#include "Com.h"\n#include <assert.h>\n' + text
    assert text.count("void Ecu_ComRecordRejection(PduIdType id, Std_ReturnType result)\n{") == 1
    text = text.replace("void Ecu_ComRecordRejection(PduIdType id, Std_ReturnType result)\n{",
                        "void Ecu_ComRecordRejection(PduIdType id, Std_ReturnType result)\n{\n    assert(Com_GetStatus() == COM_INIT);")
    observer.write_text(text)
    if mutation:
        mutation(project)
    seal(project)
    binary = build(project, directory / "build", mode)
    print(f"compiled mutation={name}", flush=True)
    stdin = directory / "commands"
    stdin.write_text("BEGIN 20\nCOMMIT\n")
    try:
        stdout = query([str(binary)], directory, "normal", stdin_file=stdin if mode == "host-batch" else None)
        diagnostics = json.loads((directory / "normal-com-rejections.json").read_text())
        # The configured 10 ms alarm first releases at 10, then 20 ms: explicit plus periodic refusal.
        expected = [{"pdu": 1, "result": 1}] * 4
        if isr_records:
            expected += [{"pdu": 7, "result": 3}] * 20
        observed = diagnostics["records"]
        assert diagnostics["complete"] and sorted(observed, key=lambda x: (x["pdu"], x["result"])) == expected, diagnostics
        assert "OUT " not in stdout and "ecu_output " not in stdout, stdout
        assert ("COMMIT_OK " if mode == "host-batch" else "ecu_probe completed=20") in stdout, stdout
    except (AssertionError, RuntimeError, ValueError) as error:
        if capacity:
            assert "COM refusal diagnostics incomplete" in str(error), error
            diagnostics = json.loads((directory / "normal-com-rejections.json").read_text())
            assert diagnostics == {"records": [{"pdu": 7, "result": 3}] * 4096, "complete": False}, diagnostics
            print(f"consumer={mode} PASS capacity loss explicit, process success, diagnostics incomplete", flush=True)
            return
        if not killed:
            raise
        if output_failure:
            assert "status=exited exit=7" in str(error) and "lifecycle=Closed state=Failed reason=7 " in str(error), error
        print(f"killed mutation={name}: {error}", flush=True)
    else:
        assert not killed and not capacity, f"mutation or incomplete control survived: {name}"
        print(f"consumer={mode} PASS exact id/result/count, no output, next periodic, normal tool", flush=True)


run_case("host", "host-batch")
run_case("probe", "probe")


def record_from_tick_isr(project):
    path = project / "os/src/Os_Time.c"
    text = '#include "Ecu_ComRejection.h"\n' + path.read_text()
    needle = "OS_MEMMAP void vApplicationTickHook(void) { Os_TimeTick(); }"
    assert text.count(needle) == 1
    path.write_text(text.replace(needle, "OS_MEMMAP void vApplicationTickHook(void) { Ecu_ComRecordRejection(7u, 3u); Os_TimeTick(); }"))


run_case("task-and-isr", "probe", record_from_tick_isr, isr_records=True)


def fill_capacity(project):
    app = next((project / "app").rglob("Ingress.c"))
    text = '#include "Ecu_ComRejection.h"\n' + app.read_text()
    needle = "    (void)Ingress_RecordActor();"
    app.write_text(text.replace(needle, needle + "\n    for (unsigned i = 0u; i < 4097u; ++i) { Ecu_ComRecordRejection(7u, 3u); }"))


for mode in ("host-batch", "probe"):
    run_case(mode + "-capacity", mode, fill_capacity, capacity=True)


for name, before, after, count in [
    ("trigger-notify", "observer(PduId, result);", "(void)observer;", 1),
    ("periodic-notify", "observer(id, result);", "(void)observer;", 1),
    ("condition", "if (result != E_OK)", "if (result == E_OK)", 2),
    ("pdu", "observer(PduId, result);", "observer((PduIdType)(PduId + 1u), result);", 1),
    ("result", "observer(id, result);", "observer(id, E_OK);", 1),
]:
    run_case(name, "probe", lambda p, b=before, a=after, c=count: edit(p, "bsw/src/Com.c", b, a, c), killed=True)


def remove_drain(project, mode):
    paths = ["Ecu_HostBridge.c", "ecu_host_batch.c"] if mode == "host-batch" else ["ecu_probe.c"]
    for path in paths:
        target = project / "integration/src" / path
        text = target.read_text()
        target.write_text(text.replace("Ecu_ComDrainRejections(FALSE)", "1").replace("Ecu_ComDrainRejections(TRUE)", "1"))


for mode in ("host-batch", "probe"):
    run_case(mode + "-drain", mode, lambda p, m=mode: remove_drain(p, m), killed=True)
    run_case(mode + "-output", mode,
             lambda p: edit(p, "integration/src/Ecu_ComRejection.c",
                            "return (text.failed == 0) ? Os_HostWriteBytes(1, text.data, text.length) : 0;",
                            "(void)text; return 0;", 2), killed=True, output_failure=True)
