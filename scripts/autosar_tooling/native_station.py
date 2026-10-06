"""Exclusive noninteractive WindowStation: desktop and clipboard are both private."""

from __future__ import annotations

import ctypes
import uuid
from ctypes import wintypes as w


class SecurityAttributes(ctypes.Structure):
    _fields_ = [("length", w.DWORD), ("descriptor", w.LPVOID), ("inherit", w.BOOL)]


class SidAttributes(ctypes.Structure):
    _fields_ = [("sid", w.LPVOID), ("attributes", w.DWORD)]


class PrivateStation:
    def __init__(self, user):
        self.user = user
        self.handle = None
        self.descriptor = w.LPVOID()
        self.kernel = ctypes.WinDLL("kernel32", use_last_error=True)
        self.advapi = ctypes.WinDLL("advapi32", use_last_error=True)
        user.GetProcessWindowStation.restype = w.HANDLE
        user.CreateWindowStationW.argtypes = [w.LPCWSTR, w.DWORD, w.DWORD, ctypes.POINTER(SecurityAttributes)]
        user.CreateWindowStationW.restype = w.HANDLE
        user.SetProcessWindowStation.argtypes = [w.HANDLE]
        user.CloseWindowStation.argtypes = [w.HANDLE]
        self.kernel.GetCurrentProcess.restype = w.HANDLE
        self.kernel.CloseHandle.argtypes = [w.HANDLE]
        self.kernel.LocalFree.argtypes = [w.LPVOID]
        self.kernel.LocalFree.restype = w.LPVOID
        self.advapi.OpenProcessToken.argtypes = [w.HANDLE, w.DWORD, ctypes.POINTER(w.HANDLE)]
        self.advapi.GetTokenInformation.argtypes = [w.HANDLE, ctypes.c_int, w.LPVOID, w.DWORD, ctypes.POINTER(w.DWORD)]
        self.advapi.ConvertSidToStringSidW.argtypes = [w.LPVOID, ctypes.POINTER(w.LPWSTR)]
        self.advapi.ConvertStringSecurityDescriptorToSecurityDescriptorW.argtypes = [w.LPCWSTR, w.DWORD, ctypes.POINTER(w.LPVOID), ctypes.POINTER(w.DWORD)]
        self.original = self.checked(user.GetProcessWindowStation())
        token = w.HANDLE()
        sid_string = w.LPWSTR()
        try:
            self.checked(self.advapi.OpenProcessToken(self.kernel.GetCurrentProcess(), 8, ctypes.byref(token)))
            length = w.DWORD()
            self.advapi.GetTokenInformation(token, 1, None, 0, ctypes.byref(length))
            buffer = ctypes.create_string_buffer(length.value)
            self.checked(self.advapi.GetTokenInformation(token, 1, buffer, length, ctypes.byref(length)))
            sid = ctypes.cast(buffer, ctypes.POINTER(SidAttributes)).contents.sid
            self.checked(self.advapi.ConvertSidToStringSidW(sid, ctypes.byref(sid_string)))
            self.checked(self.advapi.ConvertStringSecurityDescriptorToSecurityDescriptorW(
                f"D:P(A;;GA;;;{sid_string.value})S:(ML;;NW;;;ME)", 1, ctypes.byref(self.descriptor), None,
            ))
            self.security = SecurityAttributes(ctypes.sizeof(SecurityAttributes), self.descriptor, False)
            # An exclusive named station needs an elevated native controller.
            # The unprivileged logon-generated station can already exist; never
            # reuse that station because its clipboard is not owned by this run.
            station_name = f"AutosarNative-{uuid.uuid4().hex}"
            self.handle = self.checked(user.CreateWindowStationW(station_name, 1, 0x37F, ctypes.byref(self.security)))
            self.checked(user.SetProcessWindowStation(self.handle))
        except BaseException:
            self.close()
            raise
        finally:
            if sid_string:
                self.kernel.LocalFree(ctypes.cast(sid_string, w.LPVOID))
            if token:
                self.kernel.CloseHandle(token)

    @staticmethod
    def checked(result):
        if not result:
            raise ctypes.WinError(ctypes.get_last_error())
        return result

    def input_desktop(self):
        # OpenInputDesktop operates on the calling process's station. Observe
        # WinSta0 temporarily without switching or sending any user input.
        self.checked(self.user.SetProcessWindowStation(self.original))
        try:
            return self.checked(self.user.OpenInputDesktop(0, False, 1))
        finally:
            self.checked(self.user.SetProcessWindowStation(self.handle))

    def restore(self):
        self.checked(self.user.SetProcessWindowStation(self.original))

    def close(self):
        if self.handle:
            self.restore()
            self.checked(self.user.CloseWindowStation(self.handle))
            self.handle = None
        if self.descriptor:
            self.kernel.LocalFree(self.descriptor)
            self.descriptor = w.LPVOID()
