"""Verify native IPC on a separate, never switched Windows desktop.

The child uses STARTUPINFO.lpDesktop and a private WebView2 data directory.
No windows are created on the input desktop. All children belong to a
kill-on-close job. CDP drives actual path-entry UI and the WebView; all
application IPC reaches Rust without replacing the native transport.
"""

import ctypes
import json
import os
import shutil
import socket
import subprocess
import tempfile
import time
import urllib.request
import uuid
from ctypes import wintypes as w
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


class Startup(ctypes.Structure):
    _fields_ = [
        ("cb", w.DWORD),
        ("lpReserved", w.LPWSTR),
        ("lpDesktop", w.LPWSTR),
        ("lpTitle", w.LPWSTR),
        ("dwX", w.DWORD),
        ("dwY", w.DWORD),
        ("dwXSize", w.DWORD),
        ("dwYSize", w.DWORD),
        ("dwXCountChars", w.DWORD),
        ("dwYCountChars", w.DWORD),
        ("dwFillAttribute", w.DWORD),
        ("dwFlags", w.DWORD),
        ("wShowWindow", w.WORD),
        ("cbReserved2", w.WORD),
        ("lpReserved2", ctypes.POINTER(w.BYTE)),
        ("hStdInput", w.HANDLE),
        ("hStdOutput", w.HANDLE),
        ("hStdError", w.HANDLE),
    ]


class Process(ctypes.Structure):
    _fields_ = [
        ("process", w.HANDLE),
        ("thread", w.HANDLE),
        ("pid", w.DWORD),
        ("tid", w.DWORD),
    ]


class Limits(ctypes.Structure):
    _fields_ = [
        ("process_time", ctypes.c_int64),
        ("job_time", ctypes.c_int64),
        ("flags", w.DWORD),
        ("minimum", ctypes.c_size_t),
        ("maximum", ctypes.c_size_t),
        ("active", w.DWORD),
        ("affinity", ctypes.c_size_t),
        ("priority", w.DWORD),
        ("scheduling", w.DWORD),
    ]


class ExtendedLimits(ctypes.Structure):
    _fields_ = [
        ("basic", Limits),
        ("io", ctypes.c_uint64 * 6),
        ("process_memory", ctypes.c_size_t),
        ("job_memory", ctypes.c_size_t),
        ("peak_process", ctypes.c_size_t),
        ("peak_job", ctypes.c_size_t),
    ]


def run(binary: Path) -> int:
    if os.name != "nt":
        raise RuntimeError("Native desktop verification requires Windows")
    binary = binary.resolve(strict=True)
    from autosar_tooling.desktop import write_resource_inputs

    scratch = Path(tempfile.mkdtemp(prefix="autosar-native-windows-"))
    print(f"Isolated native test directory: {scratch}", flush=True)
    inputs = scratch / "inputs"
    shutil.copytree(ROOT / "core/tests/fixtures/epic4/positive", inputs)
    paths = sorted(str(path) for path in inputs.glob("*.arxml"))
    (scratch / "inputs.json").write_text(json.dumps(paths), encoding="utf-8")
    write_resource_inputs(scratch)
    packager = ROOT / "core/target/debug/package_host_reference.exe"
    packaged = subprocess.run(
        [
            str(packager),
            str(scratch / "legacy"),
            "--target",
            "windows-x64-controlled-v1",
        ],
        cwd=ROOT,
        creationflags=subprocess.CREATE_NO_WINDOW,
        capture_output=True,
        timeout=180,
        check=False,
    )
    if packaged.returncode:
        raise RuntimeError(
            packaged.stdout.decode("utf-8", errors="replace")
            + packaged.stderr.decode("utf-8", errors="replace")
        )
    user = ctypes.WinDLL("user32", use_last_error=True)
    kernel = ctypes.WinDLL("kernel32", use_last_error=True)
    user.CreateDesktopW.argtypes = [
        w.LPCWSTR,
        w.LPCWSTR,
        w.LPVOID,
        w.DWORD,
        w.DWORD,
        w.LPVOID,
    ]
    user.CreateDesktopW.restype = w.HANDLE
    user.OpenInputDesktop.argtypes = [w.DWORD, w.BOOL, w.DWORD]
    user.OpenInputDesktop.restype = w.HANDLE
    user.GetThreadDesktop.argtypes = [w.DWORD]
    user.GetThreadDesktop.restype = w.HANDLE
    window_callback = ctypes.WINFUNCTYPE(w.BOOL, w.HWND, w.LPARAM)
    user.EnumDesktopWindows.argtypes = [w.HANDLE, window_callback, w.LPARAM]
    user.GetWindowThreadProcessId.argtypes = [w.HWND, ctypes.POINTER(w.DWORD)]
    user.GetProcessWindowStation.restype = w.HANDLE
    user.GetUserObjectInformationW.argtypes = [
        w.HANDLE,
        ctypes.c_int,
        w.LPVOID,
        w.DWORD,
        ctypes.POINTER(w.DWORD),
    ]
    user.CloseDesktop.argtypes = [w.HANDLE]
    user.SetThreadDesktop.argtypes = [w.HANDLE]
    user.GetWindowTextW.argtypes = [w.HWND, w.LPWSTR, ctypes.c_int]
    user.GetClassNameW.argtypes = [w.HWND, w.LPWSTR, ctypes.c_int]
    user.EnumChildWindows.argtypes = [w.HWND, window_callback, w.LPARAM]
    user.GetDlgCtrlID.argtypes = [w.HWND]
    user.SendMessageW.argtypes = [w.HWND, w.UINT, w.WPARAM, w.LPARAM]
    user.SendMessageW.restype = w.LPARAM
    kernel.GetCurrentThreadId.restype = w.DWORD
    kernel.CreateJobObjectW.argtypes = [w.LPVOID, w.LPCWSTR]
    kernel.CreateJobObjectW.restype = w.HANDLE
    kernel.SetInformationJobObject.argtypes = [
        w.HANDLE,
        ctypes.c_int,
        w.LPVOID,
        w.DWORD,
    ]
    kernel.AssignProcessToJobObject.argtypes = [w.HANDLE, w.HANDLE]
    kernel.CreateProcessW.argtypes = [
        w.LPCWSTR,
        w.LPWSTR,
        w.LPVOID,
        w.LPVOID,
        w.BOOL,
        w.DWORD,
        w.LPVOID,
        w.LPCWSTR,
        ctypes.POINTER(Startup),
        ctypes.POINTER(Process),
    ]
    kernel.ResumeThread.argtypes = [w.HANDLE]
    kernel.TerminateProcess.argtypes = [w.HANDLE, w.UINT]
    kernel.CloseHandle.argtypes = [w.HANDLE]
    kernel.WaitForSingleObject.argtypes = [w.HANDLE, w.DWORD]

    def checked(result):
        if not result:
            raise ctypes.WinError(ctypes.get_last_error())
        return result

    def name(handle):
        buffer = ctypes.create_unicode_buffer(256)
        needed = w.DWORD()
        checked(
            user.GetUserObjectInformationW(
                handle, 2, buffer, ctypes.sizeof(buffer), ctypes.byref(needed)
            )
        )
        return buffer.value

    def windows(handle, pid):
        found = []

        @window_callback
        def visit(window, _):
            owner = w.DWORD()
            thread = user.GetWindowThreadProcessId(window, ctypes.byref(owner))
            if owner.value == pid:
                found.append({"window": window, "thread": thread})
            return True

        checked(user.EnumDesktopWindows(handle, visit, 0))
        return found

    original_thread_desktop = checked(
        user.GetThreadDesktop(kernel.GetCurrentThreadId())
    )
    original = checked(user.OpenInputDesktop(0, False, 1))
    desktop_name = f"AutosarEpic4-{uuid.uuid4().hex}"
    desktop = checked(user.CreateDesktopW(desktop_name, None, None, 0, 0x1FF, None))
    job = checked(kernel.CreateJobObjectW(None, None))
    processes = []
    try:
        limits = ExtendedLimits()
        limits.basic.flags = 0x2000
        checked(
            kernel.SetInformationJobObject(
                job, 9, ctypes.byref(limits), ctypes.sizeof(limits)
            )
        )
        station = name(user.GetProcessWindowStation())
        input_name = name(original)
        if name(desktop) == input_name:
            raise RuntimeError("Isolation desktop equals the input desktop")
        checked(user.SetThreadDesktop(desktop))

        def launch(command, environment):
            startup = Startup()
            startup.cb = ctypes.sizeof(startup)
            startup.lpDesktop = f"{station}\\{desktop_name}"
            process = Process()
            block = ctypes.create_unicode_buffer(
                "\0".join(
                    f"{key}={value}"
                    for key, value in sorted(
                        environment.items(), key=lambda item: item[0].upper()
                    )
                )
                + "\0\0"
            )
            command_line = ctypes.create_unicode_buffer(
                subprocess.list2cmdline(command)
            )
            # Suspended + Unicode environment + no console window. Assign the
            # job before any child can spawn, then verify actual desktop identity.
            checked(
                kernel.CreateProcessW(
                    None,
                    command_line,
                    None,
                    None,
                    False,
                    0x08000404,
                    block,
                    str(ROOT),
                    ctypes.byref(startup),
                    ctypes.byref(process),
                )
            )
            processes.append(process)
            if not kernel.AssignProcessToJobObject(job, process.process):
                error = ctypes.get_last_error()
                checked(kernel.TerminateProcess(process.process, 1))
                raise ctypes.WinError(error)
            if kernel.ResumeThread(process.thread) == 0xFFFFFFFF:
                raise ctypes.WinError(ctypes.get_last_error())
            return process

        with socket.socket() as socket_handle:
            socket_handle.bind(("127.0.0.1", 1420))
        node = shutil.which("node")
        if node is None:
            raise RuntimeError("Node.js is required for the existing Vite/CDP test")
        launch(
            [
                node,
                str(ROOT / "ui/node_modules/vite/bin/vite.js"),
                "--host",
                "127.0.0.1",
                "--port",
                "1420",
                "--strictPort",
                "--config",
                str(ROOT / "ui/vite.config.ts"),
                str(ROOT / "ui"),
            ],
            os.environ.copy(),
        )
        with socket.socket() as socket_handle:
            socket_handle.bind(("127.0.0.1", 0))
            debug_port = socket_handle.getsockname()[1]
        environment = os.environ.copy()
        environment.pop("AUTOSAR_XSD_ARCHIVE", None)
        environment.pop("AUTOSAR_MOD_ARCHIVE", None)
        environment["APPDATA"] = str(scratch / "app-config")
        environment["AUTOSAR_CONFIG_DIR"] = str(scratch / "app-config")
        environment["LOCALAPPDATA"] = str(scratch / "app-local")
        environment["WEBVIEW2_USER_DATA_FOLDER"] = str(scratch / "webview-profile")
        environment["WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS"] = (
            f"--remote-debugging-port={debug_port}"
        )
        deadline = time.monotonic() + 40
        while True:
            try:
                with urllib.request.urlopen("http://127.0.0.1:1420", timeout=1):
                    break
            except OSError:
                if time.monotonic() > deadline:
                    raise RuntimeError("Isolated Vite process failed to become ready")
                time.sleep(0.2)
        app = launch([str(binary)], environment)
        while True:
            try:
                with urllib.request.urlopen(
                    f"http://127.0.0.1:{debug_port}/json/list", timeout=1
                ) as response:
                    pages = json.load(response)
                target = next(page for page in pages if page.get("type") == "page")
                break
            except (OSError, StopIteration):
                if time.monotonic() > deadline:
                    raise RuntimeError("Isolated native WebView did not expose CDP")
                time.sleep(0.2)
        name(desktop)
        isolated_windows = windows(desktop, app.pid)
        if not isolated_windows or windows(original, app.pid):
            raise RuntimeError(
                "Native app windows are not exclusively on the isolated desktop"
            )
        current = checked(user.OpenInputDesktop(0, False, 1))
        try:
            if name(current) != input_name:
                raise RuntimeError("The input desktop changed during the isolated test")
        finally:
            user.CloseDesktop(current)
        driver_log = (scratch / "native-driver.log").open("w", encoding="utf-8")
        driver = subprocess.Popen(
            [
                node,
                str(ROOT / "scripts/autosar_tooling/desktop_cdp.mjs"),
                target["webSocketDebuggerUrl"],
                str(scratch),
            ],
            cwd=ROOT,
            creationflags=subprocess.CREATE_NO_WINDOW,
            stdout=driver_log,
            stderr=subprocess.STDOUT,
        )
        canceled = 0
        observed = {}
        deadline = time.monotonic() + 900

        def window_text(window, class_name=False):
            buffer = ctypes.create_unicode_buffer(256)
            method = user.GetClassNameW if class_name else user.GetWindowTextW
            method(window, buffer, len(buffer))
            return buffer.value

        try:
            while driver.poll() is None:
                if time.monotonic() > deadline:
                    raise RuntimeError(
                        "Native UI driver exceeded its bounded verification time"
                    )
                for item in windows(desktop, app.pid):
                    window = item["window"]
                    if window_text(window, True) != "#32770":
                        continue
                    started = observed.setdefault(window, time.monotonic())
                    # Let the renderer show its pending confirmation state.
                    if time.monotonic() - started < 1:
                        continue
                    cancel_buttons = []
                    accept_buttons = []

                    @window_callback
                    def child(
                        child_window, _, accept=accept_buttons, cancel=cancel_buttons
                    ):
                        # DirectUI TaskDialog buttons can expose ID 0. Use the
                        # actual translated positive label on our owned dialog.
                        if window_text(child_window, True) == "Button" and (
                            user.GetDlgCtrlID(child_window) in (1, 6, 1000, 1004)
                            or window_text(child_window)
                            in ("确定", "确认", "是", "OK", "Ok", "Yes")
                        ):
                            accept.append(child_window)
                        if window_text(child_window, True) == "Button" and (
                            user.GetDlgCtrlID(child_window) == 2
                            or window_text(child_window) in ("取消", "Cancel")
                        ):
                            cancel.append(child_window)
                        return True

                    checked(user.EnumChildWindows(window, child, 0))
                    permission = scratch / "accept-generation-path.txt"
                    requested = (
                        permission.read_text(encoding="utf-8")
                        if permission.is_file()
                        else ""
                    )
                    # TaskDialog content is not necessarily exposed as child
                    # window text. The driver authorizes only this generation
                    # confirmation, with an output inside its own temp tree.
                    allowed = requested and Path(requested).resolve().is_relative_to(
                        scratch
                    )
                    if allowed and window_text(window) == "确认操作" and accept_buttons:
                        user.SendMessageW(accept_buttons[0], 0x00F5, 0, 0)
                        permission.unlink()
                        observed[window] = time.monotonic() + 900
                    elif cancel_buttons:
                        # Only an enumerated native dialog on our separate
                        # desktop, owned by this test's Tauri PID, is addressed.
                        user.SendMessageW(cancel_buttons[0], 0x00F5, 0, 0)
                        canceled += 1
                        observed[window] = time.monotonic() + 900
                time.sleep(0.05)
            if driver.returncode:
                driver_log.flush()
                print(
                    (scratch / "native-driver.log").read_text(encoding="utf-8")[-8000:],
                    flush=True,
                )
                return driver.returncode
            if canceled < 1:
                raise RuntimeError(
                    "The native discard confirmation was not actually canceled"
                )
        finally:
            if driver.poll() is None:
                driver.terminate()
                driver.wait(timeout=5)
            driver_log.close()
        current = checked(user.OpenInputDesktop(0, False, 1))
        try:
            if name(current) != input_name:
                raise RuntimeError("Input desktop changed by the test")
        finally:
            user.CloseDesktop(current)
    finally:
        cleanup_errors = []
        if not user.SetThreadDesktop(original_thread_desktop):
            cleanup_errors.append(
                "Unable to restore the test controller's original desktop"
            )
        if not kernel.CloseHandle(job):
            cleanup_errors.append("Unable to close the isolated process job")
        for process in processes:
            if kernel.WaitForSingleObject(process.process, 5000) != 0:
                cleanup_errors.append(
                    f"Process {process.pid} did not terminate after closing the job"
                )
            kernel.CloseHandle(process.thread)
            kernel.CloseHandle(process.process)
        if not user.CloseDesktop(desktop):
            cleanup_errors.append("Unable to close the isolated desktop")
        if not user.CloseDesktop(original):
            cleanup_errors.append(
                "Unable to close the input desktop observation handle"
            )
        if cleanup_errors:
            raise RuntimeError("; ".join(cleanup_errors))
    print(f"windows_isolated_native_ipc PASS: temporary files {scratch}", flush=True)
    return 0
