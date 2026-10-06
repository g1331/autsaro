"""Resize only the verified application's window on its private Windows desktop."""

from __future__ import annotations

import ctypes
import json
from ctypes import wintypes as w
from pathlib import Path


def sample(scratch: Path, pid: int, windows: list[dict], user, isolated_station: bool) -> None:
    request_path = scratch / "native-window-request.json"
    if not request_path.is_file():
        return
    if not isolated_station:
        raise RuntimeError("Native window sizing requires the owned private WindowStation")
    request = json.loads(request_path.read_text(encoding="utf-8"))
    user.GetWindowRect.argtypes = [w.HWND, ctypes.POINTER(w.RECT)]
    user.SetWindowPos.argtypes = [w.HWND, w.HWND, ctypes.c_int, ctypes.c_int, ctypes.c_int, ctypes.c_int, w.UINT]
    user.SetThreadDpiAwarenessContext.argtypes = [w.HANDLE]
    user.SetThreadDpiAwarenessContext.restype = w.HANDLE
    owned = [item["window"] for item in windows if user.IsWindowVisible(item["window"])]
    if len(owned) != 1:
        raise RuntimeError("Native size request requires exactly one visible application-owned window")
    window = owned[0]
    owner = w.DWORD()
    user.GetWindowThreadProcessId(window, ctypes.byref(owner))
    if owner.value != pid:
        raise RuntimeError("Native resize window no longer belongs to the owned application")
    # This changes only the controller thread's coordinate interpretation, never
    # the user's DPI or display settings. SetWindowPos explicitly forbids focus.
    previous = user.SetThreadDpiAwarenessContext(w.HANDLE(-4))
    if not previous:
        raise ctypes.WinError(ctypes.get_last_error())
    try:
        if not user.SetWindowPos(window, None, 0, 0, request["width"], request["height"], 0x0016):
            raise ctypes.WinError(ctypes.get_last_error())
        rectangle = w.RECT()
        if not user.GetWindowRect(window, ctypes.byref(rectangle)):
            raise ctypes.WinError(ctypes.get_last_error())
        observed = [rectangle.right - rectangle.left, rectangle.bottom - rectangle.top]
        if observed != [request["width"], request["height"]]:
            raise RuntimeError(f"Owned native window did not reach requested physical dimensions: {observed}")
        receipt = {**request, "scope": "private-window-station", "appPid": pid, "window": window, "physicalDimensions": observed, "noActivate": True}
        destination = scratch / "native-window-result.json"
        staging = destination.with_suffix(".next.json")
        staging.write_text(json.dumps(receipt), encoding="utf-8")
        staging.replace(destination)
        request_path.unlink()
    finally:
        if not user.SetThreadDpiAwarenessContext(previous):
            raise ctypes.WinError(ctypes.get_last_error())
