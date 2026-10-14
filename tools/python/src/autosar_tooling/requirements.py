"""Account one specification's official requirement IDs against their dispositions.

The official items are the TRACE and TRACEABLE-TABLE elements of the document in
AUTOSAR_CP_MOD_SpecificationsARXML.zip. An item is disposed of only by IDs that follow
a disposition marker (ADR 0017): Verifies: in the test trees; 已核对, 不适用 or 差距 in
the documents under docs/standards/. A configuration-conditional 不适用 holds only while
a Rejects: test proves that the product refuses each named ECUC parameter. Every mention,
marked or not, is checked for stale IDs. The result is computed on each run and never
written to the repository.

SWS_BSWGeneral obligates every BSW module, so its items are accounted per module: a marker
disposes of them only with a module qualifier before its colon, Verifies[CanIf]: for one
module or Verifies[*]: for a check that covers every delivered module. Another module's
qualified disposition, or an unqualified one, disposes of nothing for the module accounted.

A completed module commits its document's catalog to core/resources/requirements: the
item IDs and life-cycle states, the IDs other documents define with the same prefixes,
and the archive's SHA-256, never specification text. The required checks account every
committed catalog without the archive; the acceptance run confirms each catalog still
matches the archive. A general document's catalog also lists the modules whose accounting is
complete; the required checks account each of them.
"""

from __future__ import annotations

import hashlib
import json
import re
import sys
import zipfile
from collections.abc import Iterable
from dataclasses import dataclass
from pathlib import Path
from xml.etree import ElementTree

from autosar_tooling.config import ROOT, archive_path

ARCHIVE_VARIABLE = "AUTOSAR_SPECIFICATIONS_ARCHIVE"
# The standards baseline: an index and one document per module (ADR 0017).
STANDARDS = "docs/standards"
CATALOGS = "core/resources/requirements"
SOURCES = (STANDARDS, "core/tests", "tests")
NAMESPACE = "{http://autosar.org/schema/r4.0}"
# Documents whose items apply to each BSW module; their markers carry a module qualifier.
GENERAL = frozenset({"SWS_BSWGeneral"})
EVERY_MODULE = "*"
MODULE = re.compile(r"[A-Za-z][A-Za-z0-9_]*")
QUALIFIER = rf"(?:\[(?P<module>{MODULE.pattern}|\*)\])?"
# A cited ID may continue with further numbers of the same width, e.g.
# "SWS_CANIF_00001／00003", "SWS_CANIF_00001, 00005" (also across a comment
# line break) or the range "SWS_CANIF_00907–00910".
CONTINUATION = re.compile(r"(?:\s*[,，、／/]\s*(?:(?://|\*|#)\s*)?|\s*([–-])\s*)(\d+)(?!\d)")
# Only the IDs right after a marker are disposed of; the group names are the categories.
# Tests prove behaviour and the standards documents record review, inapplicability and
# gaps, so each marker counts only in its own kind of source.
TEST_MARKER = re.compile(rf"(?P<verified>Verifies){QUALIFIER}:[ \t]*`?")
DOCUMENT_MARKER = re.compile(
    r"(?:(?P<inspected>已核对（[^）\n]+）)|(?P<configuration>不适用（配置[：:][ \t]*(?P<governing>[^）\n]+)）)"
    rf"|(?P<owner>不适用（归属[：:][ \t]*[^）\n]+）)|(?P<gap>差距[ \t]*#\d+)){QUALIFIER}[：:][ \t]*`?"
)
# A Rejects: test proves that the product refuses a configuration before generation.
REJECTS = re.compile(r"Rejects:[ \t]*`?")
PARAMETER = re.compile(r"(?<![A-Za-z0-9_])ECUC_[A-Za-z0-9]+_\d+(?!\d)", re.IGNORECASE)
# Between the full IDs after one marker: spaces or backticks on the same line, or a list
# separator that may continue on the next comment line (CRLF in a Windows checkout).
SEPARATOR = re.compile(r"`?(?:[ \t]*[,，、／/][ \t]*(?:\r?\n[ \t]*(?://|\*|#)?[ \t]*)?|[ \t]+)`?|`")
CATEGORIES = {
    "verified": "verified",
    "inspected": "inspected",
    "configuration": "not applicable (configuration)",
    "owner": "not applicable (owner)",
    "gap": "gap",
}


class RequirementError(Exception):
    pass


@dataclass(frozen=True)
class Citation:
    identifier: str
    path: str
    line: int
    # Numbers inside a range dispose of the items that exist; official numbering has gaps.
    interior: bool = False
    # One of CATEGORIES when the citation follows a disposition marker.
    category: str | None = None
    # The marker's module qualifier, EVERY_MODULE for a check that covers all modules.
    module: str | None = None


@dataclass(frozen=True)
class Condition:
    """A configuration-conditional 不适用 and the ECUC parameters its reason names."""

    path: str
    line: int
    parameters: tuple[str, ...]


@dataclass
class Scan:
    citations: list[Citation]
    conditions: list[Condition]
    rejected: set[str]


@dataclass(frozen=True)
class Specification:
    # Upper-case ID -> official spelling and life-cycle state ("valid" unless marked otherwise).
    items: dict[str, tuple[str, str]]
    # Upper-case IDs with the same prefixes defined by other documents of the archive.
    elsewhere: frozenset[str]


def prefix(identifier: str) -> str:
    return identifier.rstrip("0123456789")


def document_name(document: str) -> str:
    return document.removeprefix("AUTOSAR_CP_").removesuffix(".pdf").removesuffix(".arxml")


def load(archive: Path, document: str) -> Specification:
    name = document_name(document)
    with zipfile.ZipFile(archive) as bundle:
        members = [item for item in bundle.namelist() if item.endswith(f"/AUTOSAR_CP_{name}.arxml")]
        if len(members) != 1:
            raise RequirementError(f"{document}: no single AUTOSAR_CP_{name}.arxml in {archive.name}; use a document name such as SWS_CANInterface")
        with bundle.open(members[0]) as source:
            root = ElementTree.parse(source).getroot()
        states = {
            reference.text: state.text.rsplit("/", 1)[-1]
            for info in root.iter(f"{NAMESPACE}LIFE-CYCLE-INFO")
            if (reference := info.find(f"{NAMESPACE}LC-OBJECT-REF")) is not None
            and (state := info.find(f"{NAMESPACE}LC-STATE-REF")) is not None
        }
        items = {}
        for tag in ("TRACE", "TRACEABLE-TABLE"):
            for element in root.iter(f"{NAMESPACE}{tag}"):
                identifier = element.findtext(f"{NAMESPACE}SHORT-NAME")
                items[identifier.upper()] = (identifier, states.get(identifier, "valid"))
        if not items:
            raise RequirementError(f"{members[0]} has no TRACE or TRACEABLE-TABLE items")
        # Another document can use the same prefix (TTCANInterface defines ECUC_CanIf_ items);
        # a citation of such an ID belongs to that document and is not stale.
        prefixes = "|".join(re.escape(item) for item in sorted({prefix(key) for key in items}, key=len, reverse=True))
        defined = re.compile(rf"<SHORT-NAME>((?:{prefixes})\d+)</SHORT-NAME>".encode(), re.IGNORECASE)
        elsewhere = frozenset(
            match.decode().upper()
            for member in bundle.namelist()
            if member.endswith(".arxml") and member != members[0]
            for match in defined.findall(bundle.read(member))
        )
    return Specification(items, elsewhere)


def scan(text: str, path: str, prefixes: Iterable[str]) -> Scan:
    """IDs with one of the prefixes, their disposition markers, and the Rejects: parameters."""
    alternatives = "|".join(re.escape(item) for item in sorted(prefixes, key=len, reverse=True))
    # The ID must start a word: "CONSTR_" (a TPS prefix) must not match inside "SWS_CANIF_CONSTR_00001".
    head = re.compile(rf"(?<![A-Za-z0-9_])({alternatives})(\d+)(?!\d)", re.IGNORECASE)
    groups = {}
    for match in head.finditer(text):
        stem, width = match.group(1).upper(), len(match.group(2))
        # A range yields its endpoints and the numbers between.
        numbers = [(int(match.group(2)), False)]
        position = match.end()
        while (more := CONTINUATION.match(text, position)) and len(more.group(2)) == width:
            if more.group(1):
                numbers.extend((number, True) for number in range(numbers[-1][0] + 1, int(more.group(2))))
            numbers.append((int(more.group(2)), False))
            position = more.end()
        groups[match.start()] = (position, [f"{stem}{number:0{width}d}" for number, _ in numbers], [interior for _, interior in numbers])

    def line(position: int) -> int:
        return text.count("\n", 0, position) + 1

    document = path.startswith(f"{STANDARDS}/")
    categories, conditions, rejected = {}, [], set()
    for marker in (DOCUMENT_MARKER if document else TEST_MARKER).finditer(text):
        category = next(name for name in CATEGORIES if marker.groupdict().get(name))
        position, listed = marker.end(), []
        while position in groups:
            listed.append(position)
            if not (separator := SEPARATOR.match(text, groups[position][0])):
                break
            position = separator.end()
        categories.update(dict.fromkeys(listed, (category, marker.group("module"))))
        if category == "configuration" and listed:
            parameters = tuple(item.upper() for item in PARAMETER.findall(marker.group("governing")))
            conditions.append(Condition(path, line(marker.start()), parameters))
    if not document:
        for marker in REJECTS.finditer(text):
            position = marker.end()
            while parameter := PARAMETER.match(text, position):
                rejected.add(parameter.group().upper())
                if not (separator := SEPARATOR.match(text, parameter.end())):
                    break
                position = separator.end()
    citations = [
        Citation(identifier, path, line(start), interior, *categories.get(start, (None, None)))
        for start, (_, identifiers, interiors) in groups.items()
        for identifier, interior in zip(identifiers, interiors, strict=True)
    ]
    return Scan(citations, conditions, rejected)


def collect(root: Path, prefixes: set[str]) -> Scan:
    found = Scan([], [], set())
    for source in SOURCES:
        location = root / source
        for path in sorted([location] if location.is_file() else location.rglob("*")):
            if path.is_file():
                text = path.read_bytes().decode("utf-8", errors="replace")
                part = scan(text, path.relative_to(root).as_posix(), prefixes)
                found.citations += part.citations
                found.conditions += part.conditions
                found.rejected |= part.rejected
    return found


def account(
    specification: Specification, found: Scan, module: str | None = None
) -> tuple[dict[str, int], list[str], list[Citation], list[tuple[Condition, str]]]:
    """Return the items per category, the undisposed items in ID order, the citations of IDs
    that exist nowhere, and each configuration condition without a rejection test. For a
    general document, only the dispositions qualified with the module or EVERY_MODULE count."""
    cited = found.citations
    disposals = [
        citation
        for citation in cited
        if citation.category and (module is None or citation.module in (module, EVERY_MODULE))
    ]
    counts = {
        category: len({item.identifier for item in disposals if item.category == category} & specification.items.keys())
        for category in CATEGORIES
    }
    disposed = {citation.identifier for citation in disposals}
    undisposed = [specification.items[key][0] for key in sorted(specification.items.keys() - disposed)]
    stale = [
        citation
        for citation in cited
        if not citation.interior
        and citation.identifier not in specification.items
        and citation.identifier not in specification.elsewhere
    ]
    unproven = [
        (condition, parameter)
        for condition in found.conditions
        for parameter in condition.parameters or ("names no ECUC parameter",)
        if parameter not in found.rejected
    ]
    return counts, undisposed, stale, unproven


def serialize(specification: Specification, digest: str, modules: Iterable[str] = ()) -> str:
    """The committed catalog: IDs and life-cycle states only, never specification text, and
    for a general document the modules whose accounting is complete."""
    data = {
        "archive": digest,
        "items": dict(item for _, item in sorted(specification.items.items())),
        "elsewhere": sorted(specification.elsewhere),
    }
    if modules := sorted(modules):
        data["modules"] = modules
    return json.dumps(data, indent=2) + "\n"


def parse(text: str) -> tuple[Specification, str, list[str]]:
    """The specification, archive SHA-256 and registered modules of a committed catalog."""
    data = json.loads(text)
    items = {identifier.upper(): (identifier, state) for identifier, state in data["items"].items()}
    return Specification(items, frozenset(item.upper() for item in data["elsewhere"])), data["archive"], list(data.get("modules", []))


def pinned(root: Path) -> dict:
    records = json.loads((root / "core/resources/official.json").read_text(encoding="utf-8"))
    return next(item for item in records if item["environment"] == ARCHIVE_VARIABLE)


def report(document: str, specification: Specification, root: Path, stale_only: bool, module: str | None = None) -> int:
    found = collect(root, {prefix(key) for key in specification.items})
    counts, undisposed, stale, unproven = account(specification, found, module)
    scope = "" if module is None else " for every module" if module == EVERY_MODULE else f" for {module}"
    print(
        f"{document}{scope}: {len(specification.items)} official items; "
        + ", ".join(f"{CATEGORIES[category]} {count}" for category, count in counts.items())
        + f", undisposed {len(undisposed)}; {len(stale)} stale citations"
    )
    if undisposed and not stale_only:
        print("\nUndisposed items:")
        for identifier in undisposed:
            state = specification.items[identifier.upper()][1]
            print(f"  {identifier}" + ("" if state == "valid" else f" ({state})"))
    if stale:
        print("\nStale citations (not an official item of R24-11):")
        for citation in stale:
            print(f"  {citation.path}:{citation.line}: {citation.identifier}")
    if unproven and not stale_only:
        print("\nConfiguration conditions without a Rejects: test:")
        for condition, parameter in unproven:
            print(f"  {condition.path}:{condition.line}: {parameter}")
    return 1 if stale or ((undisposed or unproven) and not stale_only) else 0


def check(document: str, root: Path = ROOT, stale_only: bool = False, update: bool = False, module: str | None = None) -> int:
    """Account a document from the archive. With update, write its catalog; otherwise an
    existing catalog must match the archive. A general document is accounted for the module,
    or for each module its catalog registers; update registers the module."""
    general = document_name(document) in GENERAL
    archive = archive_path(ARCHIVE_VARIABLE, pinned(root)["path"], root)
    try:
        if module is not None and (not general or not MODULE.fullmatch(module)):
            raise RequirementError(f"--module takes a module abbreviation such as CanIf and applies only to {', '.join(sorted(GENERAL))}")
        if general and update and module is None:
            raise RequirementError(f"{document} is accounted per module; register one with --update --module <Mip>")
        if not archive.is_file():
            raise RequirementError(f"{archive} is missing; run `python -m autosar_tooling fetch-official` or set {ARCHIVE_VARIABLE}")
        specification = load(archive, document)
    except (RequirementError, zipfile.BadZipFile, ElementTree.ParseError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    catalog = root / CATALOGS / f"{document_name(document)}.json"
    registered = parse(catalog.read_text(encoding="utf-8"))[2] if general and catalog.is_file() else []
    status = 0
    if update or catalog.is_file():
        with archive.open("rb") as source:
            expected = serialize(specification, hashlib.file_digest(source, "sha256").hexdigest(), {*registered, *([module] if update and module else [])})
        if update:
            catalog.parent.mkdir(parents=True, exist_ok=True)
            catalog.write_text(expected, encoding="utf-8", newline="\n")
            print(f"Wrote {CATALOGS}/{catalog.name}")
        elif catalog.read_text(encoding="utf-8") != expected:
            print(f"error: {CATALOGS}/{catalog.name} differs from {archive.name}; run `python -m autosar_tooling requirements --update {document_name(document)}`", file=sys.stderr)
            status = 1
    if not general:
        return report(document, specification, root, stale_only) | status
    modules = [module] if module else registered
    if not modules and not stale_only:
        print(f"error: {document} is accounted per module; name one with --module <Mip>", file=sys.stderr)
        return 1
    for item in modules or [EVERY_MODULE]:
        status |= report(document, specification, root, stale_only, item)
    return status


def check_catalogs(root: Path = ROOT) -> int:
    """Account every committed catalog completely, without the archive."""
    digest = pinned(root)["sha256"]
    catalogs = sorted((root / CATALOGS).glob("*.json"))
    if not catalogs:
        print(f"No requirement catalogs in {CATALOGS}.")
    status = 0
    for path in catalogs:
        try:
            specification, source, modules = parse(path.read_text(encoding="utf-8"))
        except (ValueError, KeyError, TypeError, AttributeError) as error:
            print(f"error: {CATALOGS}/{path.name} is not a requirement catalog: {error!r}", file=sys.stderr)
            status = 1
            continue
        if source != digest:
            print(f"error: {CATALOGS}/{path.name} comes from archive {source}, but official.json pins {digest}; run `python -m autosar_tooling requirements --update {path.stem}`", file=sys.stderr)
            status = 1
        if path.stem not in GENERAL:
            status |= report(path.stem, specification, root, stale_only=False)
            continue
        if not modules:
            print(f"error: {CATALOGS}/{path.name} registers no module", file=sys.stderr)
            status = 1
        for module in modules:
            status |= report(path.stem, specification, root, stale_only=False, module=module)
    return status
