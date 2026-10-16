"""Draw the AUTOSAR layered view with each delivered module's requirement accounting.

Two SVG files come from one module table and one accounting run:

- architecture.svg: the delivered CAN stack and its neighbours, for the README;
- architecture-full.svg: every SWS document of the pinned archive, grouped by layer and
  bus, with the roadmap stage that would take on each module not provided yet.

A delivered module's ring spans all items of its SWS: verified (Verifies: and 已核对),
known gaps, items not reviewed yet, and the rest (not applicable to the supported
configuration) as the track. The number is the verified share. The figures are computed
from the archive on each run, like the requirements command, and are never committed.
"""

from __future__ import annotations

import math
import re
import sys
import zipfile
from dataclasses import dataclass
from pathlib import Path
from xml.sax.saxutils import escape

from autosar_tooling import requirements
from autosar_tooling.config import ROOT, archive_path

DELIVERED, SUBSTITUTE, ABSENT = "delivered", "substitute", "absent"


@dataclass(frozen=True)
class Module:
    label: str
    document: str  # SWS document name without the SWS_ prefix
    status: str = ABSENT
    note: str = ""  # roadmap stage (docs/roadmap/candidates.md) or what substitutes the module


def m(label: str, document: str, note: str = "", status: str = ABSENT) -> Module:
    return Module(label, document, status, note)


def d(label: str, document: str) -> Module:
    return Module(label, document, DELIVERED)


# Every SWS document of R24-11 CP exactly once; load_documents() refuses any difference.
GROUPS: list[tuple[str, list[Module]]] = [
    ("System Services", [
        d("Os", "OS"), d("BswM", "BSWModeManager"), d("Det", "DefaultErrorTracer"),
        m("EcuM", "ECUStateManager", "R8", SUBSTITUTE), m("WdgM", "WatchdogManager", "R20"),
        m("WdgIf", "WatchdogInterface", "R20"), m("FiM", "FunctionInhibitionManager", "R20"),
        m("HwTm", "HWTestManager", "R20"), m("StbM", "SynchronizedTimeBaseManager", "R21"),
        m("Tm", "TimeService", "R21"), m("IdsM", "IntrusionDetectionSystemManager", "R19"),
        m("Firewall", "Firewall", "R19"), m("SwCluC", "SoftwareClusterConnection", "—")]),
    ("Diagnostics", [
        d("Dcm", "DiagnosticCommunicationManager"), m("Dem", "DiagnosticEventManager", "R7"),
        m("Dlt", "DiagnosticLogAndTrace", "R22"), m("Xcp", "XCP", "R22"),
        m("J1939Dcm", "SAEJ1939DiagnosticCommunicationManager", "R16")]),
    ("Memory", [
        m("NvM", "NVRAMManager", "R7"), m("BulkNvM", "BulkNvDataManager", "R18"),
        m("MemIf", "MemoryAbstractionInterface", "R18"), m("MemAcc", "MemoryAccess", "R18"),
        m("Fee", "FlashEEPROMEmulation", "R18"), m("Ea", "EEPROMAbstraction", "R18")]),
    ("Communication", [
        d("Com", "COM"), d("PduR", "PDURouter"), d("LSduR", "LSduRouter"), d("ComM", "COMManager"),
        m("Nm", "NetworkManagementInterface", "R8"), m("IpduM", "IPDUMultiplexer", "R17"),
        m("LdCom", "LargeDataCOM", "R17"), m("SecOC", "SecureOnboardCommunication", "R19"),
        m("Mirror", "BusMirroring", "R22"), m("ComXf", "COMBasedTransformer", "R13"),
        m("E2EXf", "E2ETransformer", "R13"), m("SomeIpXf", "SOMEIPTransformer", "R13")]),
    ("CAN", [
        d("CanIf", "CANInterface"), d("CanTp", "CANTransportLayer"),
        m("CanSM", "CANStateManager", "R8", SUBSTITUTE), m("CanNm", "CANNetworkManagement", "R8"),
        m("CanTrcv", "CANTransceiverDriver", "R10"), m("CanXLTrcv", "CANXLTransceiverDriver", "R11"),
        m("CanTSyn", "TimeSyncOverCAN", "R21"), m("J1939Tp", "SAEJ1939TransportLayer", "R16"),
        m("J1939Nm", "SAEJ1939NetworkManagement", "R16"), m("J1939Rm", "SAEJ1939RequestManager", "R16"),
        m("J1939FSP", "SAEJ1939FunctionalSafetyCommProtocol", "R16"), m("TtcanIf", "TTCANInterface", "—")]),
    ("LIN", [
        m("LinIf", "LINInterface", "R14"), m("LinSM", "LINStateManager", "R14"),
        m("LinTrcv", "LINTransceiverDriver", "R14")]),
    ("FlexRay", [
        m("FrIf", "FlexRayInterface", "R15"), m("FrSM", "FlexRayStateManager", "R15"),
        m("FrNm", "FlexRayNetworkManagement", "R15"), m("FrTp", "FlexRayISOTransportLayer", "R15"),
        m("FrArTp", "FlexRayARTransportLayer", "R15"), m("FrTrcv", "FlexRayTransceiverDriver", "R15"),
        m("FrTSyn", "TimeSyncOverFlexRay", "R21")]),
    ("Ethernet", [
        m("EthIf", "EthernetInterface", "R12"), m("EthSM", "EthernetStateManager", "R12"),
        m("EthSwt", "EthernetSwitchDriver", "R12"), m("EthTrcv", "EthernetTransceiverDriver", "R12"),
        m("TcpIp", "TcpIp", "R12"), m("SoAd", "SocketAdaptor", "R12"), m("DoIP", "DiagnosticOverIP", "R12"),
        m("Sd", "ServiceDiscovery", "R13"), m("SomeIpTp", "SOMEIPTransportProtocol", "R13"),
        m("UdpNm", "UDPNetworkManagement", "R12"), m("EthTSyn", "TimeSyncOverEthernet", "R21"),
        m("MACsec", "MACsecKeyAgreement", "R19"), m("1722Tp", "IEEE1722TransportLayer", "R25"),
        m("DDS", "DataDistributionService", "R25"),
        m("WEthTrcv", "WirelessEthernetTransceiverDriver", "R28")]),
    ("Crypto", [
        m("Csm", "CryptoServiceManager", "R19"), m("CryIf", "CryptoInterface", "R19"),
        m("KeyM", "KeyManager", "R19")]),
    ("V2X / Charging", [
        m("V2xM", "V2XManagement", "R27"), m("V2xBtp", "V2XBasicTransport", "R27"),
        m("V2xGn", "V2XGeoNetworking", "R27"), m("V2xFac", "V2XFacilities", "R27"),
        m("V2xDm", "V2XDataManager", "R27"), m("CnV2xM", "ChineseV2XManagement", "R27"),
        m("CnV2xMsg", "ChineseV2XMessage", "R27"), m("CnV2xNet", "ChineseV2XNetwork", "R27"),
        m("CnV2xSec", "ChineseV2XSecurity", "R27"), m("ISO15118", "ISO15118Charging", "R26")]),
    ("I/O", [m("IoHwAb", "IOHardwareAbstraction", "R9")]),
    ("Libraries", [
        m("Crc", "CRCLibrary", "—"), m("E2E", "E2ELibrary", "R13"), m("Bfx", "BFXLibrary", "—"),
        m("Efx", "EFXLibrary", "—"), m("Ifl", "IFLLibrary", "—"), m("Ifx", "IFXLibrary", "—"),
        m("Mfl", "MFLLibrary", "—"), m("Mfx", "MFXLibrary", "—"), m("Msfl", "MSFLibrary", "—"),
        m("McLib", "BSWMulticoreLibrary", "R29")]),
]
MCAL = [
    d("Can", "CANDriver"), m("CanXL", "CANXLDriver", "R11"), m("Ttcan", "TTCANDriver", "—"),
    m("Lin", "LINDriver", "R14"), m("Fr", "FlexRayDriver", "R15"), m("Eth", "EthernetDriver", "R12"),
    m("WEth", "WirelessEthernetDriver", "R28"), m("CV2x", "CellularV2XDriver", "R27"),
    m("Crypto", "CryptoDriver", "R19"), m("Mcu", "MCUDriver", "R10"), m("Port", "PortDriver", "R9"),
    m("Dio", "DIODriver", "R9"), m("Adc", "ADCDriver", "R9"), m("Gpt", "GPTDriver", "R9"),
    m("Pwm", "PWMDriver", "R9"), m("Icu", "ICUDriver", "R9"), m("Ocu", "OCUDriver", "R9"),
    m("Spi", "SPIHandlerDriver", "R9"), m("I2c", "I2CDriver", "R9"), m("Wdg", "WatchdogDriver", "R20"),
    m("Fls", "FlashDriver", "R18"), m("Eep", "EEPROMDriver", "R18"), m("Mem", "MemoryDriver", "R18"),
    m("CorTst", "CoreTest", "R20"), m("FlsTst", "FlashTest", "R20"), m("RamTst", "RAMTest", "R20"),
]
# The project code that stands in for a standard module on the host targets.
SUBSTITUTES = {"EcuM": "Ecu_Target", "CanSM": "Ecu_HostBusSM"}
# Specifications that are not BSW modules; BSWGeneral is accounted per module elsewhere.
RTE = d("RTE", "RTE")
CROSS = [
    d("StdTypes", "StandardTypes"), d("PlatformTypes", "PlatformTypes"),
    d("ComStackTypes", "CommunicationStackTypes"), d("MemMap", "MemoryMapping"),
    m("ARTI", "ARTI", "R22"), m("BSW General", "BSWGeneral", "per module"),
]
BSW = [module for _, modules in GROUPS for module in modules] + MCAL
TABLE = [*BSW, RTE, *CROSS]
BY_LABEL = {module.label: module for module in TABLE}

SEGMENTS = (("verified", "#2f9e5b"), ("gap", "#e0603a"), ("open", "#8aa0bf"))
LEGEND = (*SEGMENTS, ("outside", "#e5e7eb"))
LEGEND_TEXT = {"verified": "verified", "gap": "known gap", "open": "not yet reviewed",
               "outside": "outside the supported configuration"}
FONT = 'font-family="Segoe UI, Helvetica, Arial, sans-serif"'


@dataclass(frozen=True)
class Tally:
    total: int
    verified: int
    gap: int
    open: int

    @property
    def percent(self) -> int:
        return round(100 * self.verified / self.total)


def documents(archive: Path) -> set[str]:
    """The SWS document names of the archive, without the SWS_ prefix."""
    pattern = re.compile(r"/AUTOSAR_CP_SWS_(\w+)\.arxml$")
    with zipfile.ZipFile(archive) as bundle:
        return {match.group(1) for name in bundle.namelist() if (match := pattern.search(name))}


def tally(archive: Path, root: Path, document: str) -> Tally:
    specification = requirements.load(archive, f"SWS_{document}")
    found = requirements.collect(root, {requirements.prefix(key) for key in specification.items})
    counts, undisposed, _, _ = requirements.account(specification, found)
    return Tally(len(specification.items), counts["verified"] + counts["inspected"], counts["gap"], len(undisposed))


def check_table(official: set[str]) -> None:
    listed = [module.document for module in TABLE]
    duplicated = sorted({name for name in listed if listed.count(name) > 1})
    missing, unknown = sorted(official - set(listed)), sorted(set(listed) - official)
    if duplicated or missing or unknown:
        raise requirements.RequirementError(
            f"the module table must list each SWS document once: missing {missing}, unknown {unknown}, duplicated {duplicated}"
        )


class Canvas:
    def __init__(self, width: int, height: int, tallies: dict[str, Tally]):
        self.width, self.height, self.tallies = width, height, tallies
        self.parts = [
            f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}">',
            f'<rect width="{width}" height="{height}" fill="#ffffff"/>',
        ]

    def text(self, x: float, y: float, value: str, size: float, fill: str = "#374151", weight: str = "400", anchor: str = "start") -> None:
        self.parts.append(
            f'<text x="{x:g}" y="{y:g}" text-anchor="{anchor}" {FONT} font-size="{size:g}" font-weight="{weight}" fill="{fill}">{escape(value)}</text>'
        )

    def panel(self, x: float, y: float, w: float, h: float, title: str = "", fill: str = "#f7f8fa", stroke: str = "#c9ced6") -> None:
        self.parts.append(f'<rect x="{x:g}" y="{y:g}" width="{w:g}" height="{h:g}" rx="8" fill="{fill}" stroke="{stroke}"/>')
        if title:
            self.text(x + 10, y + 19, title, 13, weight="700")

    def ring(self, cx: float, cy: float, document: str, r: float) -> None:
        counts = self.tallies[document]
        circumference = 2 * math.pi * r
        self.parts.append(f'<circle cx="{cx:g}" cy="{cy:g}" r="{r:g}" fill="none" stroke="#e5e7eb" stroke-width="{r / 3:.1f}"/>')
        start = 0.0
        for key, colour in SEGMENTS:
            length = circumference * getattr(counts, key) / counts.total
            if length > 0.3:
                self.parts.append(
                    f'<circle cx="{cx:g}" cy="{cy:g}" r="{r:g}" fill="none" stroke="{colour}" stroke-width="{r / 3:.1f}" '
                    f'stroke-dasharray="{length:.1f} {circumference:.1f}" stroke-dashoffset="{-start:.1f}" transform="rotate(-90 {cx:g} {cy:g})"/>'
                )
            start += length
        self.text(cx, cy + r / 4.5, f"{counts.percent}%" if r > 15 else str(counts.percent), r / 1.7, "#111827", "700", "middle")

    def module(self, x: float, y: float, w: float, module: Module, large: bool) -> None:
        if module.status == DELIVERED:
            counts = self.tallies[module.document]
            h = 56 if large else 36
            self.parts.append(
                f"<g><title>{escape(module.label)}: SWS_{module.document}, {counts.total} items; verified {counts.verified}, "
                f"known gaps {counts.gap}, not yet reviewed {counts.open}, outside the supported configuration "
                f"{counts.total - counts.verified - counts.gap - counts.open}</title>"
            )
            self.parts.append(f'<rect x="{x:g}" y="{y:g}" width="{w:g}" height="{h}" rx="7" fill="#ffffff" stroke="#9ca3af"/>')
            self.text(x + 10, y + h / 2 + 5, module.label, 15 if large else 13, "#111827", "700")
            self.ring(x + w - (30 if large else 20), y + h / 2, module.document, 19 if large else 13)
            self.parts.append("</g>")
            return
        substitute = module.status == SUBSTITUTE
        fill, stroke, ink = ("#eef4ff", "#6b8fd6", "#1e3a8a") if substitute else ("#f3f4f6", "#d1d5db", "#6b7280")
        h = 56 if large else 22
        self.parts.append(f"<g><title>{escape(module.label)}: SWS_{module.document}, {'host substitute' if substitute else 'not provided'}</title>")
        self.parts.append(f'<rect x="{x:g}" y="{y:g}" width="{w:g}" height="{h}" rx="6" fill="{fill}" stroke="{stroke}" stroke-dasharray="3 2"/>')
        if large:
            if substitute:
                note = f"host: {SUBSTITUTES[module.label]}" if module.label in SUBSTITUTES else module.note
            else:
                note = "not on the roadmap" if module.note == "—" else f"planned ({module.note})"
            self.text(x + 10, y + 25, module.label, 15, ink, "700" if substitute else "400")
            self.text(x + 10, y + 43, note, 11, ink)
        else:
            self.text(x + 7, y + 15, module.label, 11.5, ink)
            self.text(x + w - 6, y + 15, module.note, 10, "#3b5bab" if substitute else "#9ca3af", "600", "end")
        self.parts.append("</g>")

    def legend(self, y: float, note: str) -> None:
        x = 20.0
        for key, colour in LEGEND:
            self.parts.append(f'<rect x="{x:g}" y="{y - 7:g}" width="20" height="12" rx="3" fill="{colour}"/>')
            self.text(x + 26, y + 4, LEGEND_TEXT[key], 12)
            x += 40 + 7 * len(LEGEND_TEXT[key])
        for label, status in (("host substitute", SUBSTITUTE), ("not provided", ABSENT)):
            self.module(x + 10, y - 12, 120, Module(label, "", status, "Rn" if status == ABSENT else ""), False)
            x += 140
        self.text(20, y + 30, note, 11.5, "#6b7280")

    def svg(self) -> str:
        return "\n".join([*self.parts, "</svg>"]) + "\n"


def summary() -> str:
    delivered = sum(module.status == DELIVERED for module in BSW)
    return f"Delivered: {delivered} of {len(BSW)} BSW modules (CAN only, host targets)."


DISCLAIMER = "Not a conformance, certification or MISRA claim."


def full(tallies: dict[str, Tally]) -> str:
    width, gap, column = 1712, 8, 116
    heights = [32 + sum(42 if item.status == DELIVERED else 26 for item in items) for _, items in GROUPS]
    body, body_h = 150, max(heights)
    mcal = body + body_h + 12
    per_row = (width - 56) // (column + 6)
    mcal_h = 28 + math.ceil(len(MCAL) / per_row) * 42
    micro = mcal + mcal_h + 12
    cross = micro + 54
    canvas = Canvas(width, cross + 160, tallies)
    canvas.text(20, 30, "AUTOSAR Classic Platform R24-11 — module coverage", 18, "#111827", "700")
    canvas.text(20, 52, f"{summary()} Each ring spans every item of the module's SWS; the number is the verified percentage.", 13, "#4b5563")
    canvas.panel(20, 66, width - 40, 34)
    canvas.text(width / 2, 88, "Application Layer — multiple SWCs: sender/receiver, synchronous client/server", 13, weight="700", anchor="middle")
    canvas.panel(20, 106, width - 40, 34, fill="#fff7f2", stroke="#f0b49a")
    canvas.text(width / 2, 128, "Runtime Environment (RTE + BSW Scheduler)", 13, weight="700", anchor="middle")
    canvas.ring(width - 44, 123, RTE.document, 13)
    x = 20.0
    for title, items in GROUPS:
        canvas.panel(x, body, column + 16, body_h, title)
        y = body + 28.0
        for item in items:
            canvas.module(x + 8, y, column, item, False)
            y += 42 if item.status == DELIVERED else 26
        x += column + 16 + gap
    canvas.panel(20, mcal, width - 40, mcal_h, "Microcontroller Abstraction Layer (MCAL)")
    for index, item in enumerate(MCAL):
        row, col = divmod(index, per_row)
        canvas.module(28 + col * (column + 6), mcal + 28 + row * 42, column, item, False)
    canvas.panel(20, micro, width - 40, 42, fill="#eef0f3", stroke="#9aa3af")
    canvas.text(width / 2, micro + 26, "Microcontroller — currently a Linux / Windows host process (FreeRTOS host port); first MCU target in R10", 13, weight="700", anchor="middle")
    canvas.panel(20, cross, width - 40, 62, "Cross-cutting specifications")
    for index, item in enumerate(CROSS):
        canvas.module(28 + index * (column + 50), cross + 24, column + 42, item, False)
    canvas.legend(cross + 92, f"Rn = roadmap stage that would take the module on (docs/roadmap); — = not on the roadmap. {DISCLAIMER}")
    return canvas.svg()


def focused(tallies: dict[str, Tally]) -> str:
    width = 1440
    canvas = Canvas(width, 740, tallies)

    def place(label: str, x: float, y: float, w: float = 132) -> None:
        canvas.module(x, y, w, BY_LABEL[label], True)

    canvas.text(20, 26, summary(), 14, "#111827", "700")
    canvas.panel(20, 40, width - 40, 56)
    canvas.text(width / 2, 74, "Application Layer — multiple SWCs: sender/receiver, synchronous client/server", 16, weight="700", anchor="middle")
    canvas.panel(20, 108, width - 40, 56, fill="#fff7f2", stroke="#f0b49a")
    canvas.text(width / 2, 142, "Runtime Environment (RTE + BSW Scheduler)", 16, weight="700", anchor="middle")
    canvas.ring(width - 60, 136, RTE.document, 19)
    top, row1, row2, bottom = 176, 404, 508, 612
    canvas.panel(20, top, 320, bottom - top - 12, "System Services")
    for label, x, y in (("Os", 36, 36), ("BswM", 184, 36), ("Det", 36, 104), ("EcuM", 184, 104), ("Dem", 36, 172)):
        place(label, x, top + y)
    canvas.panel(30, row2, 300, 92, "Microcontroller Drivers", fill="#ffffff")
    canvas.module(46, row2 + 30, 268, Module("Mcu · Gpt · Wdg", "", ABSENT, "R10"), True)
    x = 352
    canvas.panel(x, top, 164, row1 - top - 12, "Memory Services")
    place("NvM", x + 16, top + 36)
    canvas.panel(x, row1, 164, 92, "Memory HW Abstraction")
    place("MemIf", x + 16, row1 + 30)
    canvas.panel(x, row2, 164, 92, "Memory Drivers")
    place("Fls", x + 16, row2 + 30)
    x = 528
    canvas.panel(x, top, 600, row1 - top - 12, "Communication Services (CAN)")
    for index, label in enumerate(("Com", "PduR", "LSduR", "CanTp")):
        place(label, x + 16 + index * 146, top + 36)
    for index, label in enumerate(("Dcm", "ComM", "CanSM", "CanNm")):
        place(label, x + 16 + index * 146, top + 104)
    canvas.panel(x, row1, 600, 92, "Communication HW Abstraction")
    place("CanIf", x + 16, row1 + 30)
    place("CanTrcv", x + 162, row1 + 30)
    canvas.panel(x, row2, 600, 92, "Communication Drivers")
    place("Can", x + 16, row2 + 30)
    canvas.text(x + 170, row2 + 62, "virtual controller on the host · 11-bit Classical CAN", 12, "#6b7280")
    x = 1140
    canvas.panel(x, top, 130, bottom - top - 12, "I/O")
    place("IoHwAb", x + 10, row1 + 30, 110)
    place("Dio", x + 10, row2 + 30, 110)
    x = 1282
    canvas.panel(x, top, 138, bottom - top - 12, "Complex Drivers")
    canvas.module(x + 10, top + 36, 118, Module("HostBusSM", "", SUBSTITUTE, "CanSM substitute"), True)
    canvas.panel(20, bottom, width - 40, 50, fill="#eef0f3", stroke="#9aa3af")
    canvas.text(width / 2, bottom + 30, "Microcontroller — currently a Linux / Windows host process (FreeRTOS host port); first MCU target in R10", 15, weight="700", anchor="middle")
    canvas.legend(
        bottom + 82,
        "Each ring spans every item of the module's R24-11 SWS; the number is the verified share. "
        f"Most of each standard lies outside the supported configuration. {DISCLAIMER}",
    )
    return canvas.svg()


def generate(output: Path, root: Path = ROOT) -> int:
    archive = archive_path(requirements.ARCHIVE_VARIABLE, requirements.pinned(root)["path"], root)
    try:
        if not archive.is_file():
            raise requirements.RequirementError(
                f"{archive} is missing; run `python -m autosar_tooling fetch-official` or set {requirements.ARCHIVE_VARIABLE}"
            )
        check_table(documents(archive))
        tallies = {item.document: tally(archive, root, item.document) for item in TABLE if item.status == DELIVERED}
    except (requirements.RequirementError, zipfile.BadZipFile) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    output.mkdir(parents=True, exist_ok=True)
    for name, render in (("architecture.svg", focused), ("architecture-full.svg", full)):
        (output / name).write_text(render(tallies), encoding="utf-8", newline="\n")
        print(f"Wrote {output / name}")
    for item in TABLE:
        if item.status == DELIVERED:
            counts = tallies[item.document]
            print(f"  {item.label}: {counts.verified}/{counts.total} verified, {counts.gap} gaps, {counts.open} not yet reviewed")
    return 0
