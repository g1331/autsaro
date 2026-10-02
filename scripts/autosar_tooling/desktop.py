"""Actual native UI verification; never use the user's active desktop."""

from __future__ import annotations

import json
import os
import shutil
import socket
import sys
import tempfile
import time
import urllib.error
import urllib.request
from pathlib import Path

from ecu_tools.process import OwnedProcess, ProcessSpec, run_bounded

from autosar_tooling.native_profile import app_environment, prepare

ROOT = Path(__file__).resolve().parents[2]


def executable(name: str) -> str:
    value = shutil.which(name)
    if value is None:
        raise RuntimeError(f"Required native desktop tool is missing: {name}")
    return str(Path(value).absolute())


def port() -> int:
    with socket.socket() as handle:
        handle.bind(("127.0.0.1", 0))
        return int(handle.getsockname()[1])


def ready(url: str) -> None:
    deadline = time.monotonic() + 60
    while time.monotonic() < deadline:
        try:
            with urllib.request.urlopen(url, timeout=1) as response:
                if response.status == 200:
                    return
        except (OSError, urllib.error.URLError):
            time.sleep(0.1)
    raise RuntimeError(f"Owned native desktop service did not become ready: {url}")


def run(
    platform: str,
    binary: Path,
    installed: bool = False,
    source_checkout: Path | None = None,
) -> int:
    if platform == "windows":
        if sys.platform != "win32":
            raise RuntimeError("Windows native desktop verification requires Windows")
        from autosar_tooling.desktop_windows import run as windows

        return windows(binary, installed, source_checkout)
    expected = "linux" if platform == "linux" else "darwin"
    if sys.platform != expected:
        raise RuntimeError(
            f"{platform} native desktop verification requires its native host"
        )
    binary = binary.resolve(strict=True)
    if installed and binary.is_relative_to(ROOT):
        raise RuntimeError(
            "Installed verification must launch an extracted app outside checkout"
        )
    if platform == "macos":
        console_uid = Path("/dev/console").stat().st_uid
        if console_uid == os.getuid() or os.getuid() == 0:
            raise RuntimeError(
                "macOS verification requires a distinct non-console GUI login session"
            )
    evidence = Path.home() / ".cache" / "autosar-tooling" / "native"
    evidence.mkdir(parents=True, exist_ok=True)
    scratch = Path(tempfile.mkdtemp(prefix=f"autosar-native-{platform}-", dir=evidence))
    print(f"Isolated native test directory: {scratch}", flush=True)
    prepare(scratch, platform, installed, source_checkout)
    environment = app_environment(scratch, installed)
    app_cwd = scratch / "app-work" if installed else ROOT
    (scratch / "application-launch.json").write_text(
        json.dumps(
            {"binary": str(binary), "cwd": str(app_cwd), "installed": installed}
        ),
        encoding="utf-8",
    )
    if platform == "macos":
        environment["HOME"] = str(scratch / "home")
        Path(environment["HOME"]).mkdir()
    children: list[OwnedProcess] = []
    driver = None
    display_lock = None
    try:
        if platform == "linux":
            import fcntl

            for number in range(180, 280):
                candidate = os.open(
                    evidence / f"display-{number}.lock", os.O_CREAT | os.O_RDWR, 0o600
                )
                try:
                    fcntl.flock(candidate, fcntl.LOCK_EX | fcntl.LOCK_NB)
                except BlockingIOError:
                    os.close(candidate)
                    continue
                if (
                    Path(f"/tmp/.X{number}-lock").exists()
                    or Path(f"/tmp/.X11-unix/X{number}").exists()
                ):
                    os.close(candidate)
                    continue
                display_lock = candidate
                display = number
                break
            else:
                raise RuntimeError("No private Xvfb display is available")
            environment.update(
                {
                    "GDK_BACKEND": "x11",
                    "WEBKIT_DISABLE_DMABUF_RENDERER": "1",
                }
            )
            environment.pop("WAYLAND_DISPLAY", None)
            xvfb = OwnedProcess(
                ProcessSpec.seconds(
                    [
                        executable("Xvfb"),
                        f":{display}",
                        "-displayfd",
                        "1",
                        "-screen",
                        "0",
                        "1440x1200x24",
                        "-nolisten",
                        "tcp",
                        "-nolisten",
                        "unix",
                    ],
                    ROOT,
                    1800,
                    scratch,
                    "native-xvfb",
                    env=environment,
                )
            )
            children.append(xvfb)
            # Hold the per-display lock and require this server's readiness receipt.
            display_log = Path(xvfb.registration["stdout"])
            until = time.monotonic() + 10
            while not display_log.read_text(encoding="ascii").strip():
                if time.monotonic() >= until:
                    raise RuntimeError(
                        f"Private Xvfb did not report a display; logs={scratch}"
                    )
                time.sleep(0.05)
            if int(display_log.read_text(encoding="ascii").strip()) != display:
                raise RuntimeError("Xvfb reported an unexpected private display")
            environment["DISPLAY"] = f":{display}"
            (scratch / "native-display.json").write_text(
                json.dumps({"display": environment["DISPLAY"]}), encoding="utf-8"
            )
            until = time.monotonic() + 10
            while True:
                probe = OwnedProcess(
                    ProcessSpec.seconds(
                        [executable("xdpyinfo"), "-display", f":{display}"],
                        ROOT,
                        5,
                        scratch,
                        "native-display-ready",
                        env=environment,
                    )
                ).wait()
                if probe.success:
                    break
                if probe.status != "exited":
                    raise RuntimeError(f"Private display probe cleanup failed: {probe}")
                if time.monotonic() >= until:
                    raise RuntimeError(
                        f"Private Xvfb failed to become reachable; logs={scratch}"
                    )
                time.sleep(0.05)
        node = executable("node")
        if not installed:
            vite = OwnedProcess(
                ProcessSpec.seconds(
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
                    ROOT,
                    1800,
                    scratch,
                    "native-vite",
                    env=environment,
                )
            )
            children.append(vite)
            ready("http://127.0.0.1:1420")
        endpoint_port = port()
        # Owner environments are overrides, not replacements; unset at exec time.
        native_command = [
            executable("env"),
            "-u",
            "AUTOSAR_XSD_ARCHIVE",
            "-u",
            "AUTOSAR_MOD_ARCHIVE",
        ]
        if installed:
            for name in (
                "PYTHONPATH",
                "PYTHONHOME",
                "VIRTUAL_ENV",
                "NODE_OPTIONS",
                "ECU_OWNER_SOCKET",
                "ECU_OWNER_TOKEN",
                "ECU_OWNER_SCOPE",
            ):
                native_command.extend(("-u", name))
            for name in os.environ:
                if name.startswith(("UV_", "CARGO_", "RUSTUP_", "NPM_CONFIG_")):
                    native_command.extend(("-u", name))
        if platform == "linux":
            native_port = port()
            server = OwnedProcess(
                ProcessSpec.seconds(
                    [
                        *native_command,
                        executable("tauri-driver"),
                        "--port",
                        str(endpoint_port),
                        "--native-port",
                        str(native_port),
                        "--native-driver",
                        executable("WebKitWebDriver"),
                    ],
                    app_cwd,
                    1800,
                    scratch,
                    "native-webdriver",
                    env=environment,
                )
            )
        else:
            environment["TAURI_WEBDRIVER_PORT"] = str(endpoint_port)
            server = OwnedProcess(
                ProcessSpec.seconds(
                    [*native_command, str(binary)],
                    app_cwd,
                    1800,
                    scratch,
                    "native-embedded-webdriver",
                    env=environment,
                )
            )
        children.append(server)
        endpoint = f"http://127.0.0.1:{endpoint_port}"
        ready(endpoint + "/status")
        driver = OwnedProcess(
            ProcessSpec.seconds(
                [
                    node,
                    str(ROOT / "scripts/autosar_tooling/desktop_webdriver.mjs"),
                    endpoint,
                    str(binary),
                    str(scratch),
                    platform,
                ],
                ROOT,
                1500,
                scratch,
                "native-ui-scenario",
                env=environment,
            )
        )
        if platform == "linux":
            xdotool = executable("xdotool")
            deadline = time.monotonic() + 1500
            seen: set[str] = set()
            while time.monotonic() < deadline:
                assert driver.owner is not None
                state = driver.owner.request("closed", scope=driver.scope)
                if state["op"] == "closed":
                    break
                found = OwnedProcess(
                    ProcessSpec.seconds(
                        [xdotool, "search", "--name", "确认操作"],
                        ROOT,
                        5,
                        scratch,
                        "native-dialogs",
                        env=environment,
                    )
                ).wait()
                if found.status != "exited" or found.exit_code not in (0, 1):
                    raise RuntimeError(f"Private dialog enumeration failed: {found}")
                for window in found.stdout.read_text(encoding="utf-8").split():
                    if window in seen:
                        continue
                    permission = scratch / "accept-generation-path.txt"
                    accepted = permission.is_file() and Path(
                        permission.read_text(encoding="utf-8")
                    ).resolve().is_relative_to(scratch)
                    action = ["key", "Escape"]
                    if accepted:
                        geometry = run_bounded(
                            ProcessSpec.seconds(
                                [xdotool, "getwindowgeometry", "--shell", window],
                                ROOT,
                                5,
                                scratch,
                                "native-dialog-geometry",
                                env=environment,
                            )
                        )
                        bounds = dict(
                            line.split("=", 1)
                            for line in geometry.stdout.read_text(
                                encoding="utf-8"
                            ).splitlines()
                            if "=" in line
                        )
                        # GTK lays out Cancel left and OK right, regardless of add order.
                        action = [
                            "mousemove",
                            "--window",
                            window,
                            str(int(bounds["WIDTH"]) * 3 // 4),
                            str(int(bounds["HEIGHT"]) - 16),
                            "click",
                            "1",
                        ]
                    sent = run_bounded(
                        ProcessSpec.seconds(
                            [xdotool, "windowfocus", "--sync", window, *action],
                            ROOT,
                            5,
                            scratch,
                            "native-dialog-action",
                            env=environment,
                        )
                    )
                    if not sent.success:
                        raise RuntimeError(f"Private dialog action failed: {sent}")
                    if accepted:
                        permission.unlink()
                    seen.add(window)
                time.sleep(0.1)
        result = driver.wait()
        print(result.stdout.read_text(encoding="utf-8"), end="", flush=True)
        if not result.success:
            print(result.stderr.read_text(encoding="utf-8"), end="", flush=True)
            raise RuntimeError(
                f"Native UI scenario failed: {result}; screenshots={scratch}"
            )
        print(f"{platform}_isolated_native_ipc PASS: {scratch}", flush=True)
        return 0
    finally:
        try:
            if driver is not None and not driver.finished:
                driver.cancel()
            for child in reversed(children):
                if not child.finished:
                    result = child.cancel()
                    if result.status not in ("cancelled", "exited"):
                        raise RuntimeError(
                            f"Native service cleanup could not be confirmed: {result}"
                        )
        finally:
            if display_lock is not None:
                os.close(display_lock)
