"""Read only the native run's isolated WindowStation or private Xvfb clipboard."""

from __future__ import annotations

import ctypes
import hashlib
import json
import os
import shutil
from ctypes import wintypes as w
from pathlib import Path


def sample(scratch: Path, environment: dict[str, str], isolated_station: bool = False) -> None:
    request_path = scratch / "native-clipboard-request.json"
    if not request_path.is_file():
        return
    request = json.loads(request_path.read_text(encoding="utf-8"))
    if os.name == "nt":
        if not isolated_station:
            raise RuntimeError("Refusing to read the user's shared WinSta0 clipboard")
        user = ctypes.WinDLL("user32", use_last_error=True)
        kernel = ctypes.WinDLL("kernel32", use_last_error=True)
        user.OpenClipboard.argtypes = [w.HWND]
        user.GetClipboardData.argtypes = [w.UINT]
        user.GetClipboardData.restype = w.HANDLE
        kernel.GlobalLock.argtypes = [w.HANDLE]
        kernel.GlobalLock.restype = w.LPVOID
        kernel.GlobalUnlock.argtypes = [w.HANDLE]
        if not user.OpenClipboard(None):
            # Clipboard ownership can still be transitioning after a real copy.
            # Leave the request pending; the controller will retry on its next tick.
            return
        try:
            handle = user.GetClipboardData(13)
            if not handle:
                return
            pointer = kernel.GlobalLock(handle)
            if not pointer:
                raise ctypes.WinError(ctypes.get_last_error())
            try:
                text = ctypes.wstring_at(pointer)
            finally:
                kernel.GlobalUnlock(handle)
        finally:
            user.CloseClipboard()
        scope = "private-window-station"
    else:
        from ecu_tools.owner import OwnershipError
        from ecu_tools.process import OwnedProcess, ProcessSpec

        program = shutil.which("xclip")
        if program is None:
            raise RuntimeError("Private Xvfb clipboard verification requires external driver tool xclip")
        if not environment.get("DISPLAY", "").startswith(":") or environment.get("WAYLAND_DISPLAY"):
            raise RuntimeError("Clipboard driver must use the run's private Xvfb display")
        result = OwnedProcess(ProcessSpec.seconds(
            [str(Path(program).resolve()), "-selection", "clipboard", "-out"],
            scratch, 10, scratch, "private-clipboard-read", env=environment,
        )).wait()
        if (
            result.status == "exited"
            and result.exit_code == 1
            and result.stdout.read_bytes() == b""
            and result.stderr.read_bytes() == b"Error: target STRING not available\n"
        ):
            # The real copy has not exposed its text target yet. Keep the same
            # bounded request pending; every other failure remains fail-closed.
            return
        if not result.success:
            raise OwnershipError(f"Private clipboard read failed: {result}")
        text = result.stdout.read_text(encoding="utf-8")
        scope = "private-xvfb-display"
    # Clipboard text may use platform CRLF. Compare complete logical text,
    # while source/package byte checks remain byte-for-byte elsewhere.
    text = text.replace("\r\n", "\n")
    content = scratch / "native-clipboard.txt"
    staged = scratch / "native-clipboard.next.txt"
    digest = hashlib.sha256(text.encode()).hexdigest()
    if digest != request["expectedSha256"]:
        return
    staged.write_text(text, encoding="utf-8", newline="")
    staged.replace(content)
    receipt = {**request, "scope": scope, "characters": len(text), "sha256": digest, "contents": str(content), "textNormalization": "CRLF-to-LF"}
    result_path = scratch / "native-clipboard-result.json"
    staged_result = scratch / "native-clipboard-result.next.json"
    staged_result.write_text(json.dumps(receipt), encoding="utf-8")
    staged_result.replace(result_path)
    request_path.unlink()
