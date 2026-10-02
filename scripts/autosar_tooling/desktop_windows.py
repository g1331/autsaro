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
from contextlib import ExitStack
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


class StartupEx(ctypes.Structure):
    _fields_ = [
        ("startup", Startup),
        ("attributes", w.LPVOID),
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


class JobAccounting(ctypes.Structure):
    _fields_ = [
        ("total_user_time", ctypes.c_int64),
        ("total_kernel_time", ctypes.c_int64),
        ("period_user_time", ctypes.c_int64),
        ("period_kernel_time", ctypes.c_int64),
        ("page_faults", w.DWORD),
        ("total_processes", w.DWORD),
        ("active_processes", w.DWORD),
        ("terminated_processes", w.DWORD),
    ]


def run(
    binary: Path,
    installed: bool = False,
    source_checkout: Path | None = None,
) -> int:
    if os.name != "nt":
        raise RuntimeError("Native desktop verification requires Windows")
    binary = binary.resolve(strict=True)
    source_checkout = source_checkout.resolve() if source_checkout is not None else None
    if installed and (
        binary.is_relative_to(ROOT)
        or (source_checkout is not None and binary.is_relative_to(source_checkout))
    ):
        raise RuntimeError(
            "Installed verification requires a binary outside the checkout"
        )
    from autosar_tooling import native_profile

    scratch = Path(tempfile.mkdtemp(prefix="autosar-native-windows-"))
    mode = "installed" if installed else "dev"
    print(f"Isolated native test directory ({mode}): {scratch}", flush=True)
    native_profile.prepare(scratch, "windows", installed, source_checkout)
    app_cwd = scratch / "app-work" if installed else ROOT
    if installed:
        app_cwd.mkdir(exist_ok=True)
    environment = native_profile.app_environment(scratch, installed)
    environment.pop("AUTOSAR_XSD_ARCHIVE", None)
    environment.pop("AUTOSAR_MOD_ARCHIVE", None)
    environment["WEBVIEW2_USER_DATA_FOLDER"] = str(scratch / "webview-profile")
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
    kernel.ResumeThread.restype = w.DWORD
    kernel.TerminateProcess.argtypes = [w.HANDLE, w.UINT]
    kernel.CloseHandle.argtypes = [w.HANDLE]
    kernel.WaitForSingleObject.argtypes = [w.HANDLE, w.DWORD]
    kernel.WaitForSingleObject.restype = w.DWORD
    kernel.GetExitCodeProcess.argtypes = [w.HANDLE, ctypes.POINTER(w.DWORD)]
    kernel.TerminateJobObject.argtypes = [w.HANDLE, w.UINT]
    kernel.QueryInformationJobObject.argtypes = [
        w.HANDLE,
        ctypes.c_int,
        w.LPVOID,
        w.DWORD,
        ctypes.POINTER(w.DWORD),
    ]
    kernel.InitializeProcThreadAttributeList.argtypes = [
        w.LPVOID,
        w.DWORD,
        w.DWORD,
        ctypes.POINTER(ctypes.c_size_t),
    ]
    kernel.UpdateProcThreadAttribute.argtypes = [
        w.LPVOID,
        w.DWORD,
        ctypes.c_size_t,
        w.LPVOID,
        ctypes.c_size_t,
        w.LPVOID,
        ctypes.POINTER(ctypes.c_size_t),
    ]
    kernel.DeleteProcThreadAttributeList.argtypes = [w.LPVOID]
    kernel.DeleteProcThreadAttributeList.restype = None

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
    desktop = None
    job = None
    processes = []
    try:
        desktop = checked(user.CreateDesktopW(desktop_name, None, None, 0, 0x1FF, None))
        job = checked(kernel.CreateJobObjectW(None, None))
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

        def launch(command, launch_environment, cwd=ROOT, stdout=None):
            with ExitStack() as resources:
                startup = StartupEx()
                startup.startup.cb = ctypes.sizeof(startup)
                startup.startup.lpDesktop = f"{station}\\{desktop_name}"
                process = Process()
                block = ctypes.create_unicode_buffer(
                    "\0".join(
                        f"{key}={value}"
                        for key, value in sorted(
                            launch_environment.items(), key=lambda item: item[0].upper()
                        )
                    )
                    + "\0\0"
                )
                command_line = ctypes.create_unicode_buffer(
                    subprocess.list2cmdline(command)
                )
                attributes = None
                inherited_handles = []
                try:
                    if stdout is not None:
                        import msvcrt

                        null_input = resources.enter_context(open(os.devnull, "rb"))
                        handles = (w.HANDLE * 2)(
                            msvcrt.get_osfhandle(null_input.fileno()),
                            msvcrt.get_osfhandle(stdout.fileno()),
                        )
                        for handle in handles:
                            was_inheritable = os.get_handle_inheritable(handle)
                            os.set_handle_inheritable(handle, True)
                            inherited_handles.append((handle, was_inheritable))
                        required = ctypes.c_size_t()
                        kernel.InitializeProcThreadAttributeList(
                            None, 1, 0, ctypes.byref(required)
                        )
                        attributes = ctypes.create_string_buffer(required.value)
                        checked(
                            kernel.InitializeProcThreadAttributeList(
                                attributes, 1, 0, ctypes.byref(required)
                            )
                        )
                        startup.attributes = ctypes.cast(attributes, w.LPVOID)
                        checked(
                            kernel.UpdateProcThreadAttribute(
                                attributes,
                                0,
                                0x00020002,
                                handles,
                                ctypes.sizeof(handles),
                                None,
                                None,
                            )
                        )
                        startup.startup.dwFlags |= 0x100
                        startup.startup.hStdInput = handles[0]
                        startup.startup.hStdOutput = handles[1]
                        startup.startup.hStdError = handles[1]
                    # Suspended + Unicode environment + no console window. Assign
                    # the job before any child, including the CDP driver, can spawn.
                    checked(
                        kernel.CreateProcessW(
                            None,
                            command_line,
                            None,
                            None,
                            stdout is not None,
                            0x08080404,
                            block,
                            str(cwd),
                            ctypes.cast(ctypes.byref(startup), ctypes.POINTER(Startup)),
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
                finally:
                    if startup.attributes:
                        kernel.DeleteProcThreadAttributeList(attributes)
                    for handle, was_inheritable in inherited_handles:
                        os.set_handle_inheritable(handle, was_inheritable)
                return process

        node = shutil.which("node")
        if node is None:
            raise RuntimeError("Node.js is required for the external native CDP driver")
        vite = None
        if not installed:
            with socket.socket() as socket_handle:
                socket_handle.bind(("127.0.0.1", 1420))
            vite = launch(
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
        environment["WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS"] = (
            f"--remote-debugging-port={debug_port}"
        )
        if not installed:
            deadline = time.monotonic() + 40
            while True:
                try:
                    with urllib.request.urlopen("http://127.0.0.1:1420", timeout=1):
                        break
                except OSError:
                    if time.monotonic() > deadline:
                        raise RuntimeError(
                            "Isolated Vite process failed to become ready"
                        )
                    time.sleep(0.2)
        app = launch([str(binary)], environment, cwd=app_cwd)
        launch_receipt = {
            "installed": installed,
            "mode": mode,
            "binary": str(binary),
            "cwd": str(app_cwd),
            "appPid": app.pid,
            "desktop": name(desktop),
            "vitePid": vite.pid if vite is not None else None,
            "environment": {
                key: environment[key]
                for key in (
                    "AUTOSAR_CONFIG_DIR",
                    "AUTOSAR_CC",
                    "AUTOSAR_OBJDUMP",
                    "AUTOSAR_NM",
                    "AUTOSAR_GIT",
                    "AUTOSAR_PYTHON",
                    "PATH",
                    "APPDATA",
                    "LOCALAPPDATA",
                    "WEBVIEW2_USER_DATA_FOLDER",
                    "WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS",
                )
                if key in environment
            },
        }
        receipt_path = scratch / "native-launch.json"
        receipt_path.write_text(json.dumps(launch_receipt, indent=2), encoding="utf-8")
        print(
            f"Native launch ({mode}): {binary}; cwd={app_cwd}; pid={app.pid}",
            flush=True,
        )
        deadline = time.monotonic() + 40
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
        canceled = 0
        observed = {}

        def window_text(window, class_name=False):
            buffer = ctypes.create_unicode_buffer(256)
            method = user.GetClassNameW if class_name else user.GetWindowTextW
            method(window, buffer, len(buffer))
            return buffer.value

        def poll(process):
            status = kernel.WaitForSingleObject(process.process, 0)
            if status == 0x102:
                return None
            if status != 0:
                raise ctypes.WinError(ctypes.get_last_error())
            exit_code = w.DWORD()
            checked(kernel.GetExitCodeProcess(process.process, ctypes.byref(exit_code)))
            return exit_code.value

        driver_log = (scratch / "native-driver.log").open("w", encoding="utf-8")
        try:
            driver = launch(
                [
                    node,
                    str(ROOT / "scripts/autosar_tooling/desktop_cdp.mjs"),
                    target["webSocketDebuggerUrl"],
                    str(scratch),
                ],
                os.environ.copy(),
                stdout=driver_log,
            )
            launch_receipt["driverPid"] = driver.pid
            receipt_path.write_text(
                json.dumps(launch_receipt, indent=2), encoding="utf-8"
            )
            deadline = time.monotonic() + 900
            while (driver_result := poll(driver)) is None:
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
            if driver_result:
                driver_log.flush()
                print(
                    (scratch / "native-driver.log").read_text(encoding="utf-8")[-8000:],
                    flush=True,
                )
                return driver_result
            if canceled < 1:
                raise RuntimeError(
                    "The native discard confirmation was not actually canceled"
                )
        finally:
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
        remaining_job_processes = None
        if job is not None:
            if not kernel.TerminateJobObject(job, 1):
                cleanup_errors.append("Unable to terminate the isolated process job")
            deadline = time.monotonic() + 5
            while True:
                accounting = JobAccounting()
                if not kernel.QueryInformationJobObject(
                    job, 1, ctypes.byref(accounting), ctypes.sizeof(accounting), None
                ):
                    cleanup_errors.append("Unable to confirm the isolated job is empty")
                    break
                remaining_job_processes = accounting.active_processes
                if remaining_job_processes == 0:
                    break
                if time.monotonic() >= deadline:
                    cleanup_errors.append(
                        f"Isolated job still owns {remaining_job_processes} processes"
                    )
                    break
                time.sleep(0.05)
            if not kernel.CloseHandle(job):
                cleanup_errors.append("Unable to close the isolated process job")
        for process in processes:
            if kernel.WaitForSingleObject(process.process, 5000) != 0:
                cleanup_errors.append(
                    f"Process {process.pid} did not terminate after closing the job"
                )
            if not kernel.CloseHandle(process.thread):
                cleanup_errors.append(
                    f"Unable to close process {process.pid} thread handle"
                )
            if not kernel.CloseHandle(process.process):
                cleanup_errors.append(f"Unable to close process {process.pid} handle")
        if desktop is not None and not user.CloseDesktop(desktop):
            cleanup_errors.append("Unable to close the isolated desktop")
        if not user.CloseDesktop(original):
            cleanup_errors.append(
                "Unable to close the input desktop observation handle"
            )
        (scratch / "native-cleanup.json").write_text(
            json.dumps(
                {
                    "installed": installed,
                    "processIds": [process.pid for process in processes],
                    "remainingJobProcesses": remaining_job_processes,
                    "errors": cleanup_errors,
                },
                indent=2,
            ),
            encoding="utf-8",
        )
        if cleanup_errors:
            raise RuntimeError("; ".join(cleanup_errors))
    print(
        f"windows_isolated_native_ipc PASS ({mode}): temporary files {scratch}",
        flush=True,
    )
    return 0
