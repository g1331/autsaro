"""Keep the installed application non-elevated when its isolated controller uses UAC."""

from __future__ import annotations

import ctypes
import json
import os
import stat
import tempfile
from ctypes import wintypes as w
from pathlib import Path


class SidAttributes(ctypes.Structure):
    _fields_ = [("sid", w.LPVOID), ("attributes", w.DWORD)]


def token_identity(token=None):
    """Read TokenUser, elevation and mandatory integrity from the actual token."""
    kernel = ctypes.WinDLL("kernel32", use_last_error=True)
    advapi = ctypes.WinDLL("advapi32", use_last_error=True)
    kernel.GetCurrentProcess.restype = w.HANDLE
    kernel.CloseHandle.argtypes = [w.HANDLE]
    kernel.LocalFree.argtypes = [w.LPVOID]
    kernel.LocalFree.restype = w.LPVOID
    advapi.OpenProcessToken.argtypes = [w.HANDLE, w.DWORD, ctypes.POINTER(w.HANDLE)]
    advapi.GetTokenInformation.argtypes = [w.HANDLE, ctypes.c_int, w.LPVOID, w.DWORD, ctypes.POINTER(w.DWORD)]
    advapi.ConvertSidToStringSidW.argtypes = [w.LPVOID, ctypes.POINTER(w.LPWSTR)]
    advapi.GetSidSubAuthorityCount.argtypes = [w.LPVOID]
    advapi.GetSidSubAuthorityCount.restype = ctypes.POINTER(w.BYTE)
    advapi.GetSidSubAuthority.argtypes = [w.LPVOID, w.DWORD]
    advapi.GetSidSubAuthority.restype = ctypes.POINTER(w.DWORD)
    owned = w.HANDLE()
    text = w.LPWSTR()
    try:
        if token is None:
            MediumToken.check(advapi.OpenProcessToken(kernel.GetCurrentProcess(), 8, ctypes.byref(owned)))
            token = owned
        length = w.DWORD()
        advapi.GetTokenInformation(token, 1, None, 0, ctypes.byref(length))
        user = ctypes.create_string_buffer(length.value)
        MediumToken.check(advapi.GetTokenInformation(token, 1, user, length, ctypes.byref(length)))
        sid = ctypes.cast(user, ctypes.POINTER(SidAttributes)).contents.sid
        MediumToken.check(advapi.ConvertSidToStringSidW(sid, ctypes.byref(text)))
        elevation = w.DWORD()
        MediumToken.check(advapi.GetTokenInformation(token, 20, ctypes.byref(elevation), ctypes.sizeof(elevation), ctypes.byref(length)))
        advapi.GetTokenInformation(token, 25, None, 0, ctypes.byref(length))
        label = ctypes.create_string_buffer(length.value)
        MediumToken.check(advapi.GetTokenInformation(token, 25, label, length, ctypes.byref(length)))
        integrity_sid = ctypes.cast(label, ctypes.POINTER(SidAttributes)).contents.sid
        count = advapi.GetSidSubAuthorityCount(integrity_sid).contents.value
        integrity = advapi.GetSidSubAuthority(integrity_sid, count - 1).contents.value
        kind = w.DWORD()
        MediumToken.check(advapi.GetTokenInformation(token, 8, ctypes.byref(kind), ctypes.sizeof(kind), ctypes.byref(length)))
        level = w.DWORD()
        if kind.value == 2:
            MediumToken.check(advapi.GetTokenInformation(token, 9, ctypes.byref(level), ctypes.sizeof(level), ctypes.byref(length)))
        return {"sid": text.value, "elevated": bool(elevation.value), "integrityRid": integrity,
                "tokenType": kind.value, "impersonationLevel": level.value if kind.value == 2 else None}
    finally:
        if text:
            kernel.LocalFree(ctypes.cast(text, w.LPVOID))
        if owned:
            kernel.CloseHandle(owned)


def process_identity(pid: int | None = None) -> dict:
    """Observe a live process's PID/creation/image and actual token through handles."""
    pid = os.getpid() if pid is None else pid
    kernel = ctypes.WinDLL("kernel32", use_last_error=True)
    advapi = ctypes.WinDLL("advapi32", use_last_error=True)
    kernel.OpenProcess.argtypes = [w.DWORD, w.BOOL, w.DWORD]
    kernel.OpenProcess.restype = w.HANDLE
    kernel.CloseHandle.argtypes = [w.HANDLE]
    kernel.GetProcessTimes.argtypes = [w.HANDLE, *([ctypes.POINTER(w.FILETIME)] * 4)]
    kernel.QueryFullProcessImageNameW.argtypes = [w.HANDLE, w.DWORD, w.LPWSTR, ctypes.POINTER(w.DWORD)]
    kernel.WaitForSingleObject.argtypes = [w.HANDLE, w.DWORD]
    advapi.OpenProcessToken.argtypes = [w.HANDLE, w.DWORD, ctypes.POINTER(w.HANDLE)]
    handle = kernel.OpenProcess(0x101000, False, pid)  # synchronize/query-limited only.
    MediumToken.check(handle)
    token = w.HANDLE()
    try:
        if kernel.WaitForSingleObject(handle, 0) != 0x102:
            raise RuntimeError(f"Authorized controller/caller process is no longer live: {pid}")
        times = [w.FILETIME() for _ in range(4)]
        MediumToken.check(kernel.GetProcessTimes(handle, *(ctypes.byref(value) for value in times)))
        image = ctypes.create_unicode_buffer(32768)
        length = w.DWORD(len(image))
        MediumToken.check(kernel.QueryFullProcessImageNameW(handle, 0, image, ctypes.byref(length)))
        MediumToken.check(advapi.OpenProcessToken(handle, 8, ctypes.byref(token)))
        return {"pid": pid, "creationFiletime": (times[0].dwHighDateTime << 32) | times[0].dwLowDateTime,
                "image": image.value, "token": token_identity(token)}
    finally:
        if token:
            kernel.CloseHandle(token)
        kernel.CloseHandle(handle)


def duplicate_medium_caller(expected: dict):
    """Duplicate only the actual live hashed medium caller's PRIMARY token."""
    if process_identity(expected["pid"]) != expected:
        raise RuntimeError("Original medium caller PID/creation/image/token changed")
    kernel = ctypes.WinDLL("kernel32", use_last_error=True)
    advapi = ctypes.WinDLL("advapi32", use_last_error=True)
    kernel.OpenProcess.argtypes = [w.DWORD, w.BOOL, w.DWORD]
    kernel.OpenProcess.restype = w.HANDLE
    kernel.CloseHandle.argtypes = [w.HANDLE]
    kernel.WaitForSingleObject.argtypes = [w.HANDLE, w.DWORD]
    kernel.GetProcessTimes.argtypes = [w.HANDLE, *([ctypes.POINTER(w.FILETIME)] * 4)]
    advapi.OpenProcessToken.argtypes = [w.HANDLE, w.DWORD, ctypes.POINTER(w.HANDLE)]
    advapi.DuplicateTokenEx.argtypes = [w.HANDLE, w.DWORD, w.LPVOID, ctypes.c_int,
                                      ctypes.c_int, ctypes.POINTER(w.HANDLE)]
    process = kernel.OpenProcess(0x101000, False, expected["pid"])
    MediumToken.check(process)
    source, result = w.HANDLE(), w.HANDLE()
    try:
        times = [w.FILETIME() for _ in range(4)]
        MediumToken.check(kernel.GetProcessTimes(process, *(ctypes.byref(value) for value in times)))
        created = (times[0].dwHighDateTime << 32) | times[0].dwLowDateTime
        if created != expected["creationFiletime"] or kernel.WaitForSingleObject(process, 0) != 0x102:
            raise RuntimeError("Original medium caller stable process handle is stale/dead")
        MediumToken.check(advapi.OpenProcessToken(process, 0xA, ctypes.byref(source)))  # QUERY|DUPLICATE.
        identity = token_identity(source)
        if (identity != expected["token"] or identity["elevated"]
                or not 0x2000 <= identity["integrityRid"] < 0x3000 or identity["tokenType"] != 1):
            raise RuntimeError("Hashed original caller must own the actual non-elevated medium PRIMARY token")
        MediumToken.check(advapi.DuplicateTokenEx(source, 0xB, None, 2, 1, ctypes.byref(result)))
        if token_identity(result) != identity or kernel.WaitForSingleObject(process, 0) != 0x102:
            raise RuntimeError("Duplicated medium PRIMARY token/caller lifetime changed")
        return result
    except BaseException:
        if result:
            kernel.CloseHandle(result)
        raise
    finally:
        if source:
            kernel.CloseHandle(source)
        kernel.CloseHandle(process)



def secure_private_tree(path: Path, caller_sid: str, *, writable: bool = False):
    """Set caller-specific inheritance on a newly owned private tree only."""
    if not caller_sid.startswith("S-1-") or any(character not in "S0123456789-" for character in caller_sid):
        raise ValueError("A literal TokenUser SID is required")
    path = path.absolute()
    for entry in (path, *path.rglob("*")):
        if entry.lstat().st_file_attributes & stat.FILE_ATTRIBUTE_REPARSE_POINT:
            raise RuntimeError(f"Private ACL scope contains a reparse point: {entry}")
    advapi = ctypes.WinDLL("advapi32", use_last_error=True)
    kernel = ctypes.WinDLL("kernel32", use_last_error=True)
    kernel.LocalFree.argtypes = [w.LPVOID]
    kernel.LocalFree.restype = w.LPVOID
    advapi.ConvertStringSecurityDescriptorToSecurityDescriptorW.argtypes = [w.LPCWSTR, w.DWORD, ctypes.POINTER(w.LPVOID), ctypes.POINTER(w.DWORD)]
    advapi.GetSecurityDescriptorDacl.argtypes = [w.LPVOID, ctypes.POINTER(w.BOOL), ctypes.POINTER(w.LPVOID), ctypes.POINTER(w.BOOL)]
    advapi.GetSecurityDescriptorSacl.argtypes = advapi.GetSecurityDescriptorDacl.argtypes
    advapi.SetNamedSecurityInfoW.argtypes = [w.LPWSTR, ctypes.c_int, w.DWORD, w.LPVOID, w.LPVOID, w.LPVOID, w.LPVOID]
    advapi.SetNamedSecurityInfoW.restype = w.DWORD
    access = "0x1301bf" if writable else "0x1200a9"  # Modify or Read/Execute.
    sddl = f"D:P(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)(A;OICI;FA;;;OW)(A;OICI;{access};;;{caller_sid})"
    if writable:
        sddl += "S:(ML;OICI;NW;;;ME)"
    descriptor = w.LPVOID()
    try:
        MediumToken.check(advapi.ConvertStringSecurityDescriptorToSecurityDescriptorW(sddl, 1, ctypes.byref(descriptor), None))
        present, defaulted = w.BOOL(), w.BOOL()
        dacl, sacl = w.LPVOID(), w.LPVOID()
        MediumToken.check(advapi.GetSecurityDescriptorDacl(descriptor, ctypes.byref(present), ctypes.byref(dacl), ctypes.byref(defaulted)))
        information = 4 | 0x80000000  # DACL + protected DACL; owner unchanged.
        if writable:
            MediumToken.check(advapi.GetSecurityDescriptorSacl(descriptor, ctypes.byref(present), ctypes.byref(sacl), ctypes.byref(defaulted)))
            information |= 0x10  # LABEL_SECURITY_INFORMATION, not privileged SACL.
        result = advapi.SetNamedSecurityInfoW(str(path), 1, information, None, None, dacl, sacl)
        if result:
            raise ctypes.WinError(result)
    finally:
        if descriptor:
            kernel.LocalFree(descriptor)
    return {"path": str(path), "callerSid": caller_sid, "access": "modify" if writable else "read-execute", "sddl": sddl}


def prepare_private_scratch(cwd: Path, caller_sid: str):
    """Grant write access only inside this process's newly created app scratch."""
    root = cwd.absolute().parent
    kernel = ctypes.WinDLL("kernel32", use_last_error=True)
    kernel.GetCurrentProcess.restype = w.HANDLE
    creation, exit_time, kernel_time, user_time = (w.FILETIME() for _ in range(4))
    kernel.GetProcessTimes.argtypes = [w.HANDLE, ctypes.POINTER(w.FILETIME), ctypes.POINTER(w.FILETIME), ctypes.POINTER(w.FILETIME), ctypes.POINTER(w.FILETIME)]
    MediumToken.check(kernel.GetProcessTimes(kernel.GetCurrentProcess(), ctypes.byref(creation), ctypes.byref(exit_time), ctypes.byref(kernel_time), ctypes.byref(user_time)))
    created_ns = ((creation.dwHighDateTime << 32) | creation.dwLowDateTime) * 100 - 11644473600000000000
    if (cwd.name != "app-work" or root.parent != Path(tempfile.gettempdir()).absolute()
            or not root.name.startswith("autosar-native-windows-")
            or root.stat().st_ctime_ns < created_ns):
        raise RuntimeError("Medium token ACL scope must be this controller's new private scratch")
    grants = [secure_private_tree(root, caller_sid)]
    for name in ("app-work", "app-home", "app-temp", "app-config", "app-local",
                 "config", "data", "webview-profile", "projects", "deliveries"):
        directory = root / name
        directory.mkdir(exist_ok=True)
        grants.append(secure_private_tree(directory, caller_sid, writable=True))
    (root / "caller-sid-access.json").write_text(json.dumps(grants, indent=2), encoding="utf-8")
    return grants


class MediumToken:
    def __init__(self, caller_sid: str | None = None, *, caller_process: dict | None = None):
        if caller_process is None:
            raise PermissionError("A hashed actual original medium caller PID/creation/token is required")
        self.kernel = ctypes.WinDLL("kernel32", use_last_error=True)
        self.advapi = ctypes.WinDLL("advapi32", use_last_error=True)
        self.kernel.GetCurrentProcess.restype = w.HANDLE
        self.kernel.CloseHandle.argtypes = [w.HANDLE]
        self.advapi.OpenProcessToken.argtypes = [w.HANDLE, w.DWORD, ctypes.POINTER(w.HANDLE)]
        self.advapi.GetTokenInformation.argtypes = [w.HANDLE, ctypes.c_int, w.LPVOID, w.DWORD, ctypes.POINTER(w.DWORD)]
        self.handle = w.HANDLE()
        controller = w.HANDLE()
        linked = w.HANDLE()
        try:
            self.check(self.advapi.OpenProcessToken(self.kernel.GetCurrentProcess(), 8, ctypes.byref(controller)))
            length = w.DWORD()
            self.check(self.advapi.GetTokenInformation(controller, 19, ctypes.byref(linked), ctypes.sizeof(linked), ctypes.byref(length)))
            controller_identity = token_identity(controller)
            self.caller_sid = caller_sid or caller_process["token"]["sid"]
            if (controller_identity["sid"] != self.caller_sid
                    or caller_process["token"]["sid"] != self.caller_sid
                    or not controller_identity["elevated"]):
                raise RuntimeError("Actual high controller and hashed medium caller must be the same TokenUser")
            self.linked_identity = self.assert_ordinary(linked)
            # Linked handles may be identification-only: never upgrade or retry
            # them. Duplicate the original caller's independently observed primary.
            self.handle = duplicate_medium_caller(caller_process)
            self.assert_ordinary(self.handle)
        except BaseException:
            self.close()
            raise
        finally:
            if linked:
                self.kernel.CloseHandle(linked)
            if controller:
                self.kernel.CloseHandle(controller)

    @staticmethod
    def check(result):
        if not result:
            raise ctypes.WinError(ctypes.get_last_error())
        return result

    def assert_ordinary(self, token):
        identity = token_identity(token)
        if identity["elevated"] or not 0x2000 <= identity["integrityRid"] < 0x3000:
            raise RuntimeError(f"Installed acceptance refuses an elevated/non-medium application token: {identity}")
        if identity["sid"] != self.caller_sid:
            raise RuntimeError("Linked/product TokenUser differs from the hashed parent caller")
        return identity

    def process_evidence(self, process):
        token = w.HANDLE()
        self.check(self.advapi.OpenProcessToken(process, 8, ctypes.byref(token)))
        try:
            return self.assert_ordinary(token)
        finally:
            self.kernel.CloseHandle(token)

    def create(self, executable, command, environment, cwd, startup, process):
        prepare_private_scratch(Path(cwd), self.caller_sid)
        self.advapi.CreateProcessWithTokenW.argtypes = [
            w.HANDLE, w.DWORD, w.LPCWSTR, w.LPWSTR, w.DWORD, w.LPVOID,
            w.LPCWSTR, w.LPVOID, w.LPVOID,
        ]
        return self.check(self.advapi.CreateProcessWithTokenW(
            self.handle, 1, executable, command, 0x08080404, environment, cwd,
            ctypes.byref(startup), ctypes.byref(process),
        ))

    def create_non_gui(self, executable, command, environment, cwd, startup, process):
        """Return a same-SID medium, suspended, windowless child for Job attachment."""
        if startup.lpDesktop:
            raise RuntimeError("Non-GUI primitive must not select any desktop")
        prepare_private_scratch(Path(cwd), self.caller_sid)
        self.advapi.CreateProcessWithTokenW.argtypes = [
            w.HANDLE, w.DWORD, w.LPCWSTR, w.LPWSTR, w.DWORD, w.LPVOID,
            w.LPCWSTR, w.LPVOID, w.LPVOID,
        ]
        return self.check(self.advapi.CreateProcessWithTokenW(
            self.handle, 0, executable, command, 0x08000404, environment, cwd,
            ctypes.byref(startup), ctypes.byref(process),
        ))

    def close(self):
        if self.handle:
            self.kernel.CloseHandle(self.handle)
            self.handle = w.HANDLE()
