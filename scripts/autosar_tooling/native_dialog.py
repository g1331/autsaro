"""Authorize only a private scenario's pending native file/directory selection."""

from __future__ import annotations

import json
import time
from pathlib import Path


def pending(scratch: Path) -> tuple[Path, dict] | None:
    request = scratch / "native-dialog-request.json"
    if not request.is_file():
        return None
    value = json.loads(request.read_text(encoding="utf-8"))
    selected = Path(value["path"]).resolve()
    if value.get("kind") not in ("file", "directory") or not selected.is_relative_to(scratch):
        raise RuntimeError("Native chooser authorization escapes private scratch")
    if not selected.exists():
        raise RuntimeError(f"Native chooser target does not exist: {selected}")
    return request, value


def complete(scratch: Path, request: Path, selection: dict) -> None:
    """Consume only after the chooser closed and its actual result was observed."""
    result = scratch / "native-dialog-result.json"
    deadline = time.monotonic() + 60
    while time.monotonic() < deadline:
        try:
            observed = json.loads(result.read_text(encoding="utf-8"))
        except FileNotFoundError:
            observed = None
        if observed is not None and observed.get("requestId") == selection["requestId"]:
            if (
                observed.get("kind") != selection["kind"]
                or observed.get("path") != selection["path"]
                or observed.get("selectedPath") != selection["path"]
            ):
                raise RuntimeError("Native chooser result differs from its authorization")
            destination = scratch / "native-dialog-consumed.json"
            staging = destination.with_suffix(".next.json")
            staging.write_text(json.dumps(observed), encoding="utf-8")
            staging.replace(destination)
            request.unlink()
            return
        time.sleep(0.1)
    raise RuntimeError("Native chooser closed without an observed authorized result")


def windows(scratch, window, user, callback, window_text):
    authorization = pending(scratch)
    if authorization is None or window_text(window) == "确认操作":
        return False
    request, value = authorization
    edits = []
    accept = []

    @callback
    def child(handle, _):
        kind = window_text(handle, True)
        if kind == "Edit" and user.IsWindowVisible(handle) and user.IsWindowEnabled(handle):
            edits.append(handle)
        if kind == "Button" and user.GetDlgCtrlID(handle) == 1:
            accept.append(handle)
        return True

    user.EnumChildWindows(window, child, 0)
    # The common chooser's file-name edit is control 1148. Do not type into
    # another Edit such as the address/search box or an unrelated dialog.
    filename = next((edit for edit in edits if user.GetDlgCtrlID(edit) == 1148), None)
    if filename is None or not accept:
        return False
    import ctypes

    text = ctypes.create_unicode_buffer(value["path"])
    user.SendMessageW(filename, 0x000C, 0, ctypes.cast(text, ctypes.c_void_p).value)
    user.SendMessageW(accept[0], 0x00F5, 0, 0)
    deadline = time.monotonic() + 10
    while user.IsWindowVisible(window):
        if time.monotonic() >= deadline:
            raise RuntimeError("Private chooser stayed visible after path selection")
        time.sleep(0.1)
    complete(scratch, request, value)
    return True
