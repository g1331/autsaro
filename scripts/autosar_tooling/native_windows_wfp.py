"""Dynamic WFP isolation for explicitly authorized exact private executable paths.

Preparation only reads AppIDs and hash roots. Activation uses a dynamic session;
it never changes Windows Firewall profiles or installs persistent policy.
"""

from __future__ import annotations

import ctypes
import hashlib
import json
import os
import stat
import uuid
from ctypes import wintypes as w
from pathlib import Path

from autosar_tooling.native_token import process_identity, token_identity


class Guid(ctypes.Structure):
    _fields_ = [("data1", ctypes.c_uint32), ("data2", ctypes.c_uint16),
                ("data3", ctypes.c_uint16), ("data4", ctypes.c_ubyte * 8)]

    @classmethod
    def parse(cls, value):
        return cls.from_buffer_copy(uuid.UUID(str(value)).bytes_le)


class DisplayData(ctypes.Structure):
    _fields_ = [("name", w.LPWSTR), ("description", w.LPWSTR)]


class ByteBlob(ctypes.Structure):
    _fields_ = [("size", ctypes.c_uint32), ("data", ctypes.POINTER(ctypes.c_ubyte))]


class ValueUnion(ctypes.Union):
    # Every SDK arm is at most a pointer or a 32-bit scalar; UINT64 is indirect.
    _fields_ = [("uint8", ctypes.c_ubyte), ("uint32", ctypes.c_uint32),
                ("pointer", ctypes.c_void_p), ("blob", ctypes.POINTER(ByteBlob))]


class Value(ctypes.Structure):
    _anonymous_ = ("value",)
    _fields_ = [("type", ctypes.c_uint32), ("value", ValueUnion)]


class Session(ctypes.Structure):
    _fields_ = [("key", Guid), ("display", DisplayData), ("flags", ctypes.c_uint32),
                ("wait", ctypes.c_uint32), ("pid", ctypes.c_uint32),
                ("sid", ctypes.c_void_p), ("username", w.LPWSTR), ("kernel", w.BOOL)]


class Sublayer(ctypes.Structure):
    _fields_ = [("key", Guid), ("display", DisplayData), ("flags", ctypes.c_uint32),
                ("provider", ctypes.POINTER(Guid)), ("data", ByteBlob),
                ("weight", ctypes.c_uint16)]


class Condition(ctypes.Structure):
    _fields_ = [("key", Guid), ("match", ctypes.c_uint32), ("value", Value)]


class Action(ctypes.Structure):
    _fields_ = [("type", ctypes.c_uint32), ("key", Guid)]


class Context(ctypes.Union):
    _fields_ = [("raw", ctypes.c_uint64), ("provider", Guid)]


class Filter(ctypes.Structure):
    _fields_ = [("key", Guid), ("display", DisplayData), ("flags", ctypes.c_uint32),
                ("provider", ctypes.POINTER(Guid)), ("data", ByteBlob),
                ("layer", Guid), ("sublayer", Guid), ("weight", Value),
                ("count", ctypes.c_uint32), ("conditions", ctypes.POINTER(Condition)),
                ("action", Action), ("context", Context),
                ("reserved", ctypes.POINTER(Guid)), ("id", ctypes.c_uint64),
                ("effectiveWeight", Value)]


LAYERS = {
    "ALE_AUTH_CONNECT_V4": "c38d57d1-05a7-4c33-904f-7fbceee60e82",
    "ALE_AUTH_CONNECT_V6": "4a72393b-319f-44bc-84c3-ba54dcb3b6b4",
}
APP_ID = Guid.parse("d78e1e87-8644-4ea5-9437-d809ecefc971")
FLAGS = Guid.parse("632ce23b-5167-435c-86d7-e903684aa80c")
FILTER_NOT_FOUND = 0x80320003
SUBLAYER_NOT_FOUND = 0x80320007


def checked(result, operation):
    if result:
        raise OSError(result, f"{operation} failed with WFP status 0x{result:08x}")


def api():
    if os.name != "nt" or ctypes.sizeof(ctypes.c_void_p) != 8:
        raise RuntimeError("Private WFP controller requires native Win64")
    layouts = {Session: 72, Sublayer: 72, Condition: 40, Filter: 200,
               ByteBlob: 16, Value: 16, Action: 20, Context: 16}
    for structure, size in layouts.items():
        if ctypes.sizeof(structure) != size:
            raise RuntimeError(f"Win64 SDK ABI mismatch: {structure.__name__}")
    library = ctypes.WinDLL("fwpuclnt", use_last_error=True)
    signatures = {
        "FwpmGetAppIdFromFileName0": [w.LPCWSTR, ctypes.POINTER(ctypes.POINTER(ByteBlob))],
        "FwpmFreeMemory0": [ctypes.POINTER(ctypes.c_void_p)],
        "FwpmEngineOpen0": [w.LPCWSTR, ctypes.c_uint32, ctypes.c_void_p, ctypes.POINTER(Session), ctypes.POINTER(w.HANDLE)],
        "FwpmEngineClose0": [w.HANDLE],
        "FwpmTransactionBegin0": [w.HANDLE, ctypes.c_uint32],
        "FwpmTransactionCommit0": [w.HANDLE],
        "FwpmSubLayerAdd0": [w.HANDLE, ctypes.POINTER(Sublayer), ctypes.c_void_p],
        "FwpmSubLayerGetByKey0": [w.HANDLE, ctypes.POINTER(Guid), ctypes.POINTER(ctypes.POINTER(Sublayer))],
        "FwpmFilterAdd0": [w.HANDLE, ctypes.POINTER(Filter), ctypes.c_void_p, ctypes.POINTER(ctypes.c_uint64)],
        "FwpmFilterGetByKey0": [w.HANDLE, ctypes.POINTER(Guid), ctypes.POINTER(ctypes.POINTER(Filter))],
        "FwpmFilterCreateEnumHandle0": [w.HANDLE, ctypes.c_void_p, ctypes.POINTER(w.HANDLE)],
        "FwpmFilterEnum0": [w.HANDLE, w.HANDLE, ctypes.c_uint32,
                           ctypes.POINTER(ctypes.POINTER(ctypes.POINTER(Filter))), ctypes.POINTER(ctypes.c_uint32)],
        "FwpmFilterDestroyEnumHandle0": [w.HANDLE, w.HANDLE],
    }
    for name, arguments in signatures.items():
        function = getattr(library, name)
        function.argtypes = arguments
        function.restype = None if name == "FwpmFreeMemory0" else ctypes.c_uint32
    return library


def free(library, pointer):
    library.FwpmFreeMemory0(ctypes.cast(ctypes.byref(pointer), ctypes.POINTER(ctypes.c_void_p)))


def exact_program(path: Path, root: Path):
    for original in (root, path):
        if ".." in original.parts:
            raise RuntimeError(f"WFP executable scope contains path traversal: {original}")
        absolute = original.absolute()
        for entry in (absolute, *absolute.parents):
            if entry.lstat().st_file_attributes & stat.FILE_ATTRIBUTE_REPARSE_POINT:
                raise RuntimeError(f"WFP executable scope contains a reparse point: {entry}")
    root = root.resolve(strict=True)
    path = path.resolve(strict=True)
    if not root.is_dir() or not path.is_relative_to(root) or path == root or not path.is_file():
        raise RuntimeError(f"WFP executable must be an existing private-root descendant: {path}")
    with path.open("rb") as stream:
        digest = hashlib.file_digest(stream, "sha256").hexdigest()
    return path, digest


def hash_root(root: Path, private_root: Path) -> dict:
    """Hash the complete immutable private binary tree, including non-EXE inputs."""
    root = root.absolute()
    if ".." in root.parts:
        raise RuntimeError(f"WFP hash root contains path traversal: {root}")
    for entry in (root, *root.parents):
        if entry.lstat().st_file_attributes & stat.FILE_ATTRIBUTE_REPARSE_POINT:
            raise RuntimeError(f"WFP hash root contains a reparse point: {entry}")
    root = root.resolve(strict=True)
    private_root = private_root.resolve(strict=True)
    if not root.is_dir() or root == private_root or not root.is_relative_to(private_root):
        raise RuntimeError("WFP hash root must be a private-root directory descendant")
    entries = []
    for path in sorted(root.rglob("*")):
        if path.lstat().st_file_attributes & stat.FILE_ATTRIBUTE_REPARSE_POINT:
            raise RuntimeError(f"WFP hash root contains a reparse point: {path}")
        if path.is_file():
            with path.open("rb") as stream:
                digest = hashlib.file_digest(stream, "sha256").hexdigest()
            entries.append({"path": path.relative_to(root).as_posix(), "sha256": digest,
                            "size": path.stat().st_size})
        elif not path.is_dir():
            raise RuntimeError(f"WFP hash root contains a nonregular object: {path}")
    content = json.dumps(entries, sort_keys=True, separators=(",", ":")).encode()
    return {"path": str(root), "sha256": hashlib.sha256(content).hexdigest(), "files": entries}


def prepare(programs: list[Path], private_root: Path, *, hash_roots: list[Path] | None = None,
            activation_context: dict | None = None) -> dict:
    """Resolve actual WFP app IDs and byte hashes; never open an engine/session."""
    library = api()
    entries = []
    seen = set()
    for program in programs:
        path, digest = exact_program(program, private_root)
        identity = str(path).casefold()
        if identity in seen:
            raise RuntimeError(f"Duplicate private WFP executable: {path}")
        seen.add(identity)
        blob = ctypes.POINTER(ByteBlob)()
        checked(library.FwpmGetAppIdFromFileName0(str(path), ctypes.byref(blob)), "FwpmGetAppIdFromFileName0")
        try:
            app_id = ctypes.string_at(blob.contents.data, blob.contents.size)
            entries.append({"path": str(path), "sha256": digest, "appIdHex": app_id.hex()})
        finally:
            free(library, blob)
    if not entries:
        raise RuntimeError("Private WFP isolation requires at least one executable")
    return {"method": "dynamic-wfp-exact-app-non-loopback", "status": "prepared-not-activated",
            "privateRoot": str(private_root.resolve(strict=True)), "programs": entries,
            "hashRoots": [hash_root(root, private_root) for root in hash_roots or []],
            "callerSid": token_identity()["sid"], "layers": LAYERS,
            "activationContext": activation_context,
            "condition": "ALE_APP_ID EQUAL AND FLAGS IS_LOOPBACK NONE_SET",
            "action": "BLOCK", "profilesChanged": False, "activationExercised": False}


def confirm_absence(filters: list[dict], sublayer_key: str) -> dict:
    """Query only exact owned keys after explicit close or controller RPC rundown."""
    library = api()
    probe = w.HANDLE()
    checked(library.FwpmEngineOpen0(None, 10, None, None, ctypes.byref(probe)),
            "FwpmEngineOpen0 cleanup query")
    try:
        for entry in filters:
            key = Guid.parse(entry["key"])
            pointer = ctypes.POINTER(Filter)()
            result = library.FwpmFilterGetByKey0(probe, ctypes.byref(key), ctypes.byref(pointer))
            if pointer:
                free(library, pointer)
            if result != FILTER_NOT_FOUND:
                checked(result, "FwpmFilterGetByKey0 cleanup query")
                raise RuntimeError(f"Owned dynamic WFP filter remains: {entry['id']}")
        key = Guid.parse(sublayer_key)
        pointer = ctypes.POINTER(Sublayer)()
        result = library.FwpmSubLayerGetByKey0(probe, ctypes.byref(key), ctypes.byref(pointer))
        if pointer:
            free(library, pointer)
        if result != SUBLAYER_NOT_FOUND:
            checked(result, "FwpmSubLayerGetByKey0 cleanup query")
            raise RuntimeError("Owned dynamic WFP sublayer remains")
    finally:
        checked(library.FwpmEngineClose0(probe), "FwpmEngineClose0 cleanup query")
    return {"status": "confirmed-absent", "filterIds": [entry["id"] for entry in filters],
            "sublayerKey": sublayer_key}


class PrivateWfp:
    """All filters belong to one dynamic BFE session and one private sublayer."""

    def __init__(self, plan: dict):
        self.plan = plan
        self.library = api()
        self.handle = w.HANDLE()
        self.sublayer_key = Guid.parse(uuid.uuid4())
        self.filters = []

    def start(self, *, activation_authorized: bool = False):
        if not activation_authorized:
            raise PermissionError("Dynamic WFP activation requires a separate reviewed coordinator grant")
        if self.handle or self.filters:
            raise RuntimeError("Private WFP objects are single-session; prepare a fresh controller")
        controller = token_identity()
        if not controller["elevated"]:
            raise PermissionError("WFP activation requires the explicitly authorized high controller")
        if controller["sid"] != self.plan["callerSid"]:
            raise PermissionError("WFP activation TokenUser differs from the hashed actual caller")
        current = prepare([Path(entry["path"]) for entry in self.plan["programs"]],
                          Path(self.plan["privateRoot"]),
                          hash_roots=[Path(entry["path"]) for entry in self.plan["hashRoots"]],
                          activation_context=self.plan["activationContext"])
        if current != self.plan:
            raise RuntimeError("Private WFP paths, hashes or app IDs changed after preparation")
        session = Session()
        session.display.name = "Autosar native private executable isolation"
        session.flags = 1  # FWPM_SESSION_FLAG_DYNAMIC; no persistent/boot-time objects.
        session.wait = 15000
        context = self.plan["activationContext"]
        if context is None:
            raise PermissionError("Activation requires the hashed actual caller/controller session context")
        for name in ("callerProcess", "controllerProcess"):
            expected = context[name]
            if process_identity(expected["pid"]) != expected or expected["token"]["sid"] != controller["sid"]:
                raise PermissionError("Authorized caller/controller PID, creation or same-SID token changed")
        checked(self.library.FwpmEngineOpen0(None, 10, None, ctypes.byref(session), ctypes.byref(self.handle)), "FwpmEngineOpen0")
        try:
            checked(self.library.FwpmTransactionBegin0(self.handle, 0), "FwpmTransactionBegin0")
            sublayer = Sublayer()
            sublayer.key = self.sublayer_key
            sublayer.display.name = "Autosar private app block only"
            sublayer.weight = 65535
            checked(self.library.FwpmSubLayerAdd0(self.handle, ctypes.byref(sublayer), None), "FwpmSubLayerAdd0")
            for program in self.plan["programs"]:
                data = (ctypes.c_ubyte * (len(program["appIdHex"]) // 2)).from_buffer_copy(bytes.fromhex(program["appIdHex"]))
                blob = ByteBlob(len(data), data)
                for layer_name, layer_key in LAYERS.items():
                    conditions = (Condition * 2)()
                    conditions[0].key = APP_ID
                    conditions[0].match = 0  # FWP_MATCH_EQUAL.
                    conditions[0].value.type = 12  # FWP_BYTE_BLOB_TYPE.
                    conditions[0].value.blob = ctypes.pointer(blob)
                    conditions[1].key = FLAGS
                    conditions[1].match = 8  # FWP_MATCH_FLAGS_NONE_SET.
                    conditions[1].value.type = 3  # FWP_UINT32.
                    conditions[1].value.uint32 = 1  # FWP_CONDITION_FLAG_IS_LOOPBACK.
                    rule = Filter()
                    rule.key = Guid.parse(uuid.uuid4())
                    rule.display.name = f"Autosar private {layer_name}: {program['path']}"
                    rule.layer = Guid.parse(layer_key)
                    rule.sublayer = self.sublayer_key
                    rule.weight.type = 0  # FWP_EMPTY: BFE automatic weight.
                    rule.count = 2
                    rule.conditions = conditions
                    rule.action.type = 0x1001  # FWP_ACTION_BLOCK: default hard block.
                    identifier = ctypes.c_uint64()
                    checked(self.library.FwpmFilterAdd0(self.handle, ctypes.byref(rule), None, ctypes.byref(identifier)), "FwpmFilterAdd0")
                    self.filters.append((rule.key, identifier.value, program, layer_key))
            checked(self.library.FwpmTransactionCommit0(self.handle), "FwpmTransactionCommit0")
            observed = self.observe()
            return {"status": "active-observed", "method": self.plan["method"], "filters": observed,
                    "sublayerKey": str(uuid.UUID(bytes_le=bytes(self.sublayer_key))),
                    "sessionLifetime": "controller-handle/process", "profilesChanged": False,
                    "arbitration": self.assess_arbitration()}
        except BaseException as error:
            # Closing aborts a partial transaction and removes every dynamic object.
            try:
                self.close()
            except BaseException as cleanup_error:
                error.add_note(f"Dynamic WFP cleanup also failed: {cleanup_error}")
                raise error from cleanup_error
            raise

    def observe(self):
        if not self.handle:
            raise RuntimeError("Private WFP session is not active")
        observed = []
        for key, identifier, program, layer in self.filters:
            pointer = ctypes.POINTER(Filter)()
            checked(self.library.FwpmFilterGetByKey0(self.handle, ctypes.byref(key), ctypes.byref(pointer)), "FwpmFilterGetByKey0")
            try:
                rule = pointer.contents
                valid = (rule.id == identifier and bytes(rule.key) == bytes(key)
                         and bytes(rule.layer) == bytes(Guid.parse(layer))
                         and bytes(rule.sublayer) == bytes(self.sublayer_key)
                         and rule.flags == 0 and rule.action.type == 0x1001 and rule.count == 2)
                if valid:
                    conditions = {bytes(condition.key): condition for condition in rule.conditions[:2]}
                    app, flags = conditions.get(bytes(APP_ID)), conditions.get(bytes(FLAGS))
                    valid = (app is not None and flags is not None
                             and app.match == 0 and app.value.type == 12 and bool(app.value.blob)
                             and ctypes.string_at(app.value.blob.contents.data, app.value.blob.contents.size).hex() == program["appIdHex"]
                             and flags.match == 8 and flags.value.type == 3 and flags.value.uint32 == 1)
                if not valid:
                    raise RuntimeError(f"Observed WFP filter differs from exact private plan: {identifier}")
                observed.append({"id": identifier, "key": str(uuid.UUID(bytes_le=bytes(key))),
                                 "path": program["path"], "sha256": program["sha256"], "layer": layer})
            finally:
                free(self.library, pointer)
        return observed

    def assess_arbitration(self):
        """Read current equal/higher sublayer permit/callout candidates; mutate none."""
        if not self.handle:
            raise RuntimeError("Private WFP session is not active")
        enumerator = w.HANDLE()
        checked(self.library.FwpmFilterCreateEnumHandle0(self.handle, None, ctypes.byref(enumerator)),
                "FwpmFilterCreateEnumHandle0 arbitration")
        candidates = []
        total = 0
        layers = {bytes(Guid.parse(layer)) for layer in LAYERS.values()}
        try:
            while True:
                entries = ctypes.POINTER(ctypes.POINTER(Filter))()
                count = ctypes.c_uint32()
                checked(self.library.FwpmFilterEnum0(self.handle, enumerator, 256,
                                                   ctypes.byref(entries), ctypes.byref(count)),
                        "FwpmFilterEnum0 arbitration")
                try:
                    for index in range(count.value):
                        rule = entries[index].contents
                        total += 1
                        if (bytes(rule.layer) not in layers
                                or (rule.action.type != 0x1002 and not rule.action.type & 0x4000)):
                            continue
                        sublayer = ctypes.POINTER(Sublayer)()
                        checked(self.library.FwpmSubLayerGetByKey0(self.handle, ctypes.byref(rule.sublayer),
                                                                 ctypes.byref(sublayer)),
                                "FwpmSubLayerGetByKey0 arbitration")
                        try:
                            weight = sublayer.contents.weight
                            if weight >= 65535:
                                candidates.append({"filterId": rule.id, "name": rule.display.name,
                                                   "action": rule.action.type, "flags": rule.flags,
                                                   "sublayerWeight": weight,
                                                   "sublayerKey": str(uuid.UUID(bytes_le=bytes(rule.sublayer))),
                                                   "conditionCount": rule.count})
                        finally:
                            free(self.library, sublayer)
                finally:
                    if entries:
                        free(self.library, entries)
                if count.value < 256:
                    break
        finally:
            checked(self.library.FwpmFilterDestroyEnumHandle0(self.handle, enumerator),
                    "FwpmFilterDestroyEnumHandle0 arbitration")
        return {"enumeratedFilterCount": total, "equalHigherPermitOrCalloutCandidates": candidates,
                "sublayerWeight": 65535, "hostileHardPermitOverrideClaimed": False,
                "scope": "Current snapshot; equal-weight ordering and callout actions require live proof"}

    def close(self):
        """Close/rundown owns cleanup, including an uncommitted partial transaction."""
        if not self.handle:
            return {"status": "not-opened"}
        checked(self.library.FwpmEngineClose0(self.handle), "FwpmEngineClose0")
        self.handle = w.HANDLE()
        return confirm_absence(
            [{"key": str(uuid.UUID(bytes_le=bytes(key))), "id": identifier}
             for key, identifier, _, _ in self.filters],
            str(uuid.UUID(bytes_le=bytes(self.sublayer_key))),
        )
