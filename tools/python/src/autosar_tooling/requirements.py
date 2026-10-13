"""Account one specification's official requirement IDs against their dispositions.

The official items are the TRACE and TRACEABLE-TABLE elements of the document in
AUTOSAR_CP_MOD_SpecificationsARXML.zip. An item is disposed of only by IDs that follow
a disposition marker in docs/standards.md or the test trees (ADR 0017); every mention,
marked or not, is checked for stale IDs. The result is computed on each run and never
written to the repository.
"""

from __future__ import annotations

import json
import re
import sys
import zipfile
from dataclasses import dataclass
from pathlib import Path
from xml.etree import ElementTree

from autosar_tooling.config import ROOT, archive_path

ARCHIVE_VARIABLE = "AUTOSAR_SPECIFICATIONS_ARCHIVE"
SOURCES = ("docs/standards.md", "core/tests", "tests")
NAMESPACE = "{http://autosar.org/schema/r4.0}"
# A cited ID may continue with further numbers of the same width, e.g.
# "SWS_CANIF_00001／00003", "SWS_CANIF_00001, 00005" (also across a comment
# line break) or the range "SWS_CANIF_00907–00910".
CONTINUATION = re.compile(r"(?:\s*[,，、／/]\s*(?:(?://|\*|#)\s*)?|\s*([–-])\s*)(\d+)(?!\d)")
# Only the IDs right after a marker are disposed of. The group names are the categories.
MARKER = re.compile(
    r"(?:(?P<verified>Verifies:)|(?P<inspected>已核对（[^）\n]+）[：:])"
    r"|(?P<not_applicable>不适用（[^）\n]+）[：:])|(?P<gap>差距\s*#\d+[：:]))[ \t]*`?"
)
# Between the full IDs after one marker: spaces or backticks on the same line, or a list
# separator that may continue on the next comment line.
SEPARATOR = re.compile(r"`?(?:[ \t]*[,，、／/][ \t]*(?:\n[ \t]*(?://|\*|#)?[ \t]*)?|[ \t]+)`?|`")
CATEGORIES = ("verified", "inspected", "not_applicable", "gap")


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


@dataclass(frozen=True)
class Specification:
    # Upper-case ID -> official spelling and life-cycle state ("valid" unless marked otherwise).
    items: dict[str, tuple[str, str]]
    # Upper-case IDs with the same prefixes defined by other documents of the archive.
    elsewhere: frozenset[str]


def prefix(identifier: str) -> str:
    return identifier.rstrip("0123456789")


def load(archive: Path, document: str) -> Specification:
    name = document.removeprefix("AUTOSAR_CP_").removesuffix(".pdf").removesuffix(".arxml")
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


def citations(text: str, path: str, prefixes: set[str]) -> list[Citation]:
    """IDs with one of the prefixes; a range yields its endpoints and the numbers between."""
    alternatives = "|".join(re.escape(item) for item in sorted(prefixes, key=len, reverse=True))
    # The ID must start a word: "CONSTR_" (a TPS prefix) must not match inside "SWS_CANIF_CONSTR_00001".
    head = re.compile(rf"(?<![A-Za-z0-9_])({alternatives})(\d+)(?!\d)", re.IGNORECASE)
    groups = {}
    for match in head.finditer(text):
        stem, width = match.group(1).upper(), len(match.group(2))
        numbers = [(int(match.group(2)), False)]
        position = match.end()
        while (more := CONTINUATION.match(text, position)) and len(more.group(2)) == width:
            if more.group(1):
                numbers.extend((number, True) for number in range(numbers[-1][0] + 1, int(more.group(2))))
            numbers.append((int(more.group(2)), False))
            position = more.end()
        groups[match.start()] = (position, [f"{stem}{number:0{width}d}" for number, _ in numbers], [interior for _, interior in numbers])
    categories = {}
    for marker in MARKER.finditer(text):
        position = marker.end()
        while position in groups:
            categories[position] = marker.lastgroup
            position = groups[position][0]
            if not (separator := SEPARATOR.match(text, position)):
                break
            position = separator.end()
    return [
        Citation(identifier, path, text.count("\n", 0, start) + 1, interior, categories.get(start))
        for start, (_, identifiers, interiors) in groups.items()
        for identifier, interior in zip(identifiers, interiors, strict=True)
    ]


def collect(root: Path, prefixes: set[str]) -> list[Citation]:
    found = []
    for source in SOURCES:
        location = root / source
        for path in sorted([location] if location.is_file() else location.rglob("*")):
            if path.is_file():
                text = path.read_bytes().decode("utf-8", errors="replace")
                found.extend(citations(text, path.relative_to(root).as_posix(), prefixes))
    return found


def account(specification: Specification, cited: list[Citation]) -> tuple[dict[str, int], list[str], list[Citation]]:
    """Return the items per category, the undisposed items in ID order and the citations of IDs that exist nowhere."""
    counts = {
        category: len({item.identifier for item in cited if item.category == category} & specification.items.keys())
        for category in CATEGORIES
    }
    disposed = {citation.identifier for citation in cited if citation.category}
    undisposed = [specification.items[key][0] for key in sorted(specification.items.keys() - disposed)]
    stale = [
        citation
        for citation in cited
        if not citation.interior
        and citation.identifier not in specification.items
        and citation.identifier not in specification.elsewhere
    ]
    return counts, undisposed, stale


def check(document: str, root: Path = ROOT, stale_only: bool = False) -> int:
    records = json.loads((root / "core/resources/official.json").read_text(encoding="utf-8"))
    entry = next(item for item in records if item["environment"] == ARCHIVE_VARIABLE)
    archive = archive_path(ARCHIVE_VARIABLE, entry["path"], root)
    try:
        if not archive.is_file():
            raise RequirementError(f"{archive} is missing; run `python -m autosar_tooling fetch-official` or set {ARCHIVE_VARIABLE}")
        specification = load(archive, document)
    except (RequirementError, zipfile.BadZipFile, ElementTree.ParseError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    counts, undisposed, stale = account(specification, collect(root, {prefix(key) for key in specification.items}))
    print(
        f"{document}: {len(specification.items)} official items; "
        + ", ".join(f"{category.replace('_', ' ')} {count}" for category, count in counts.items())
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
    return 1 if stale or (undisposed and not stale_only) else 0
