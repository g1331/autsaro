"""BSWMD interfaces retain their actual linked memory sections."""
from __future__ import annotations

import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from ecu_tools.build import check_object


class ObjectSectionTests(unittest.TestCase):
    def check(self, linux: bool, changed: str | None = None, missing: str | None = None):
        sections = {".os_vec": (1, "DATA", 256), ".os_code": (2, "CODE, READONLY", 64),
                    ".rte_code": (3, "CODE, READONLY", 64), ".text": (4, "CODE, READONLY", 64)}
        symbols = {"Os_InterruptVectorTable": ".os_vec", "Os_TaskEntry_Main": ".os_code",
                   **dict.fromkeys(("ErrorHook", "PreTaskHook", "PostTaskHook", "StartupHook", "ShutdownHook"),
                                   ".os_code"), "Rte_Start": ".rte_code", "Ecu_DetRuntimeCallout": ".text"}
        if changed:
            symbols[changed] = ".rte_code" if changed == "Ecu_DetRuntimeCallout" else ".text"
        if missing:
            del symbols[missing]
        section_text = "\n".join(f" {index - 1} {name} {size:08x}\n ALLOC, {flags}"
                                 for name, (index, flags, size) in sections.items())
        symbol_text = "\n".join(f"00000000 g F {section} 00000010 {name}" if linux
                                else f"[0](sec {sections[section][0]}) 0x0000 {name}"
                                for name, section in symbols.items())
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            description = root / "generated/descriptions/Host_Implementation.arxml"
            description.parent.mkdir(parents=True)
            names = ["Rte_Start", "Ecu_DetRuntimeCallout", "ErrorHook", "PreTaskHook", "PostTaskHook",
                     "StartupHook", "ShutdownHook"]
            references = "".join("<BSW-MODULE-ENTRY-REF-CONDITIONAL><BSW-MODULE-ENTRY-REF>"
                                 f"/{'AUTOSAR_Rte' if name.startswith('Rte_') else 'Autsaro_Rte'}"
                                 f"/BswModuleEntrys/{name}</BSW-MODULE-ENTRY-REF>"
                                 "</BSW-MODULE-ENTRY-REF-CONDITIONAL>" for name in names)
            description.write_text('<AUTOSAR xmlns="http://autosar.org/schema/r4.0">'
                                   '<BSW-MODULE-DESCRIPTION><SHORT-NAME>Rte</SHORT-NAME>'
                                   f'<IMPLEMENTED-ENTRYS>{references}</IMPLEMENTED-ENTRYS>'
                                   '</BSW-MODULE-DESCRIPTION></AUTOSAR>', encoding="utf-8")
            target = {"target": "linux-x64-controlled-v1" if linux else "windows-x64-controlled-v1",
                      "requiredSections": list(sections)}
            with patch("ecu_tools.build.query", side_effect=["elf64-x86-64" if linux else "pei-x86-64",
                                                            symbol_text, section_text]):
                check_object(root / "binary", root / "objdump", target, root, root)

    def test_integration_callouts_and_rte_entries_use_their_own_sections(self):
        for linux in [True, False]:
            with self.subTest(linux=linux):
                self.check(linux)

    def test_missing_or_misplaced_entries_are_rejected(self):
        for linux in [True, False]:
            for name in ["Ecu_DetRuntimeCallout", "Rte_Start"]:
                for mutation in ["changed", "missing"]:
                    with (self.subTest(linux=linux, name=name, mutation=mutation),
                          self.assertRaisesRegex(ValueError, f"{name}; expected")):
                        self.check(linux, **{mutation: name})
            with (self.subTest(linux=linux, name="ErrorHook"),
                  self.assertRaisesRegex(ValueError, "OS hook is outside its code section: ErrorHook")):
                self.check(linux, changed="ErrorHook")
