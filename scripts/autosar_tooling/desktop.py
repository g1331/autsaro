"""Actual native UI verification; never use the user's active desktop."""

from __future__ import annotations

import hashlib
import json
import os
import shutil
import socket
import stat
import sys
import tempfile
import time
import urllib.error
import urllib.request
from contextlib import ExitStack
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


def start_private_bus(
    scratch: Path, environment: dict[str, str],
) -> tuple[OwnedProcess, Path]:
    """Own one foreground, non-activating session bus before any GTK client."""
    private = Path(tempfile.mkdtemp(prefix="autosar-dbus-"))
    bus_socket = private / "bus.sock"
    configuration = private / "bus.conf"
    configuration.write_text(
        '<!DOCTYPE busconfig PUBLIC "-//freedesktop//DTD D-Bus Bus Configuration 1.0//EN"\n'
        ' "http://www.freedesktop.org/standards/dbus/1.0/busconfig.dtd">\n'
        "<busconfig>\n"
        "  <type>session</type>\n"
        f"  <listen>unix:path={bus_socket}</listen>\n"
        "  <auth>EXTERNAL</auth>\n"
        '  <policy context="default">\n'
        '    <deny user="*"/>\n'
        f'    <allow user="{os.getuid()}"/>\n'
        '    <allow send_destination="*"/>\n'
        '    <allow receive_sender="*"/>\n'
        '    <allow own="*"/>\n'
        '    <deny send_destination="org.freedesktop.DBus"\n'
        '          send_interface="org.freedesktop.DBus" send_member="StartServiceByName"/>\n'
        "  </policy>\n"
        "</busconfig>\n",
        encoding="utf-8",
    )
    binary = Path(executable("dbus-daemon")).resolve(strict=True)
    bus = OwnedProcess(ProcessSpec.seconds(
        [str(binary), "--nofork", "--nosyslog", f"--config-file={configuration}",
         "--print-address=1"],
        scratch, 1800, scratch, "native-private-dbus", env=environment,
    ))
    try:
        stdout = Path(bus.registration["stdout"])
        deadline = time.monotonic() + 10
        while not stdout.read_text(encoding="utf-8").strip():
            assert bus.owner is not None
            state = bus.owner.request("closed", scope=bus.scope)
            if state["op"] == "closed":
                raise RuntimeError(f"Private session bus exited before readiness: {bus.wait()}")
            if time.monotonic() >= deadline:
                raise RuntimeError("Private session bus did not publish its address")
            time.sleep(0.05)
        address = stdout.read_text(encoding="utf-8").strip()
        if address.split(",", 1)[0] != f"unix:path={bus_socket}" or not bus_socket.is_socket():
            raise RuntimeError("Private session bus published an unexpected address")
        (scratch / "native-bus.json").write_text(json.dumps({
            "binary": str(binary), "binarySha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
            "configuration": str(configuration), "configurationSha256": hashlib.sha256(configuration.read_bytes()).hexdigest(),
            "privateRoot": str(private), "rootMode": oct(private.stat().st_mode & 0o777),
            "rootUid": private.stat().st_uid, "address": address, "socket": str(bus_socket),
            "scope": bus.scope, "pid": bus.registration["pid"], "pgid": bus.registration["pgid"],
            "namespace": os.readlink("/proc/self/ns/net"), "userNamespace": os.readlink("/proc/self/ns/user"),
            "serviceActivation": False, "foreground": True,
        }, indent=2), encoding="utf-8")
        environment["DBUS_SESSION_BUS_ADDRESS"] = address
        return bus, bus_socket
    except BaseException:
        if not bus.finished:
            bus.cancel()
        raise


def run(
    platform: str,
    binary: Path,
    installed: bool = False,
    source_checkout: Path | None = None,
    builtin_only: bool = False,
    *,
    caller_process: dict | None = None,
) -> int:
    if platform == "windows":
        if sys.platform != "win32":
            raise RuntimeError("Windows native desktop verification requires Windows")
        from autosar_tooling.desktop_windows import run as windows

        return windows(
            binary,
            installed,
            source_checkout,
            builtin_only,
            caller_process=caller_process,
        )
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
    evidence = (
        Path.home() / ".cache" / "an"
        if platform == "linux"
        else Path.home() / ".cache" / "autosar-tooling" / "native"
    )
    evidence.mkdir(mode=0o700, parents=True, exist_ok=True)
    prefix = "n-" if platform == "linux" else f"autosar-native-{platform}-"
    scratch = Path(tempfile.mkdtemp(prefix=prefix, dir=evidence))
    scratch_identity = scratch.lstat()
    if platform == "linux":
        for directory, identity in ((evidence, evidence.lstat()), (scratch, scratch_identity)):
            if (
                not stat.S_ISDIR(identity.st_mode)
                or identity.st_uid != os.getuid()
                or identity.st_mode & 0o077
            ):
                raise RuntimeError(f"Native scratch must be a private owned directory: {directory}")
    print(f"Isolated native test directory: {scratch}", flush=True)
    prepare(scratch, platform, installed, source_checkout, builtin_only)
    environment = app_environment(scratch, installed, builtin_only)
    app_cwd = scratch / "app-work" if installed else ROOT
    (scratch / "application-launch.json").write_text(
        json.dumps(
            {
                "binary": str(binary), "cwd": str(app_cwd), "installed": installed,
                "mode": "builtin-only" if builtin_only else "oracle",
                "scratch": {
                    "path": str(scratch),
                    "uid": scratch_identity.st_uid,
                    "mode": oct(scratch_identity.st_mode & 0o777),
                    "device": scratch_identity.st_dev,
                    "inode": scratch_identity.st_ino,
                },
            }
        ),
        encoding="utf-8",
    )
    if platform == "macos":
        environment["HOME"] = str(scratch / "home")
        Path(environment["HOME"]).mkdir()
    children: list[OwnedProcess] = []
    driver = None
    display_lock = None
    media = None
    bus = None
    bus_socket = None
    previous_bus_address = environment.get("DBUS_SESSION_BUS_ADDRESS")
    try:
        if platform == "linux":
            import fcntl
            bus, bus_socket = start_private_bus(scratch, environment)
            children.append(bus)

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
            if builtin_only:
                from autosar_tooling.native_media import PrivateMedia

                media = PrivateMedia(scratch, environment)
                children.append(media.process)
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
        if builtin_only:
            from autosar_tooling.native_builtin import EXECUTION_VARIABLES
            native_command.extend(("-u", "GTK_THEME"))

            for name in EXECUTION_VARIABLES:
                if name not in ("AUTOSAR_XSD_ARCHIVE", "AUTOSAR_MOD_ARCHIVE"):
                    native_command.extend(("-u", name))
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
                if builtin_only:
                    assert media is not None
                    media.sample()
                    from autosar_tooling.native_memory import sample

                    sample(scratch, server.registration["pid"], "native-server-subtree")
                    from autosar_tooling.native_clipboard import (
                        sample as read_clipboard,
                    )

                    read_clipboard(scratch, environment)
                from autosar_tooling.native_dialog import pending

                authorization = pending(scratch)
                if authorization is not None:
                    request_path, selection = authorization
                    chooser = OwnedProcess(
                        ProcessSpec.seconds(
                            [xdotool, "search", "--onlyvisible", "--name",
                             "^(Open|Save|Select|Choose|打开|保存|选择)"],
                            ROOT, 5, scratch, "native-path-dialog", env=environment,
                        )
                    ).wait()
                    if chooser.status != "exited" or chooser.exit_code not in (0, 1):
                        raise RuntimeError(f"Private chooser enumeration failed: {chooser}")
                    windows = chooser.stdout.read_text(encoding="utf-8").split()
                    if len(windows) == 1:
                        window = windows[0]
                        owner_pid = run_bounded(ProcessSpec.seconds(
                            [xdotool, "getwindowpid", window], ROOT, 5, scratch,
                            "native-path-window-pid", env=environment,
                        ))
                        if not owner_pid.success:
                            raise RuntimeError(f"Private chooser PID could not be observed: {owner_pid}")
                        chooser_pid = int(owner_pid.stdout.read_text(encoding="utf-8").strip())
                        lineage = []
                        cursor = chooser_pid
                        while True:
                            fields = Path(f"/proc/{cursor}/stat").read_text(
                                encoding="utf-8",
                            ).rsplit(")", 1)[1].split()
                            lineage.append({"pid": cursor, "creationTicks": int(fields[19])})
                            if cursor == server.registration["pid"]:
                                break
                            parent_pid = int(fields[1])
                            if parent_pid <= 1 or parent_pid == cursor:
                                raise RuntimeError("Native chooser is outside the owned server subtree")
                            cursor = parent_pid
                        with (scratch / "native-dialog-windows.jsonl").open(
                            "a", encoding="utf-8",
                        ) as observed:
                            observed.write(json.dumps({
                                **selection,
                                "window": window,
                                "lineage": lineage,
                                "bootId": Path("/proc/sys/kernel/random/boot_id").read_text().strip(),
                            }) + "\n")
                        focused = run_bounded(ProcessSpec.seconds(
                            [xdotool, "windowfocus", "--sync", window, "getwindowfocus"],
                            ROOT, 5, scratch, "native-path-focus", env=environment,
                        ))
                        if not focused.success or focused.stdout.read_text(encoding="utf-8").strip() != window:
                            raise RuntimeError("Private chooser did not receive X11 focus")
                        # Paste the authorized URI into GtkFileChooser's file list.
                        # Location-entry completion is navigation, not folder acceptance.
                        uri = Path(selection["path"]).as_uri()
                        uri_input = scratch / "native-file-uri.txt"
                        uri_input.write_text(uri, encoding="utf-8")
                        clipboard = OwnedProcess(ProcessSpec.seconds(
                            [executable("xclip"), "-quiet", "-selection", "clipboard",
                             "-i", str(uri_input)],
                            ROOT, 60, scratch, "native-file-uri-clipboard", env=environment,
                        ))
                        children.append(clipboard)
                        copied = run_bounded(ProcessSpec.seconds(
                            [executable("xclip"), "-selection", "clipboard", "-out"],
                            ROOT, 5, scratch, "native-file-uri-ready", env=environment,
                        ))
                        if not copied.success or copied.stdout.read_text(encoding="utf-8") != uri:
                            raise RuntimeError("Private chooser URI clipboard did not become ready")
                        actions = [
                            ["key", "--clearmodifiers", "alt+Home"],
                            ["sleep", "0.2"],
                            ["key", "--clearmodifiers", "ctrl+v"],
                            ["sleep", "0.2"],
                            ["key", "--clearmodifiers", "Return"],
                        ]
                        for action in actions:
                            selected = run_bounded(ProcessSpec.seconds(
                                [xdotool, *action], ROOT, 5, scratch,
                                "native-path-selection", env=environment,
                            ))
                            if not selected.success:
                                raise RuntimeError(f"Private chooser selection failed: {selected}")
                        closed_deadline = time.monotonic() + 10
                        directory_accept_sent = False
                        while True:
                            remaining = OwnedProcess(ProcessSpec.seconds(
                                [xdotool, "search", "--onlyvisible", "--name",
                                 "^(Open|Save|Select|Choose|打开|保存|选择)"],
                                ROOT, 5, scratch, "native-path-completion",
                                env=environment,
                            )).wait()
                            if remaining.status != "exited" or remaining.exit_code not in (0, 1):
                                raise RuntimeError(f"Private chooser completion failed: {remaining}")
                            if window not in remaining.stdout.read_text(encoding="utf-8").split():
                                break
                            if selection["kind"] == "directory" and not directory_accept_sent:
                                # A populated folder is first navigated into; activate
                                # its genuine Open button only while the chooser remains.
                                for action in (["sleep", "0.2"],
                                               ["key", "--clearmodifiers", "alt+o"]):
                                    accepted = run_bounded(ProcessSpec.seconds(
                                        [xdotool, *action], ROOT, 5, scratch,
                                        "native-folder-accept", env=environment,
                                    ))
                                    if not accepted.success:
                                        raise RuntimeError(f"Private folder accept failed: {accepted}")
                                directory_accept_sent = True
                            if time.monotonic() >= closed_deadline:
                                raise RuntimeError("Private chooser stayed visible after path selection")
                            time.sleep(0.1)
                        if clipboard is not None:
                            clipboard_closed = clipboard.cancel()
                            if clipboard_closed.status not in ("cancelled", "exited"):
                                raise RuntimeError(
                                    f"Private chooser clipboard cleanup could not be confirmed: {clipboard_closed}"
                                )
                        from autosar_tooling.native_dialog import complete

                        complete(scratch, request_path, selection)
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
        cleanup_results = []

        def cancel_scope(child: OwnedProcess, role: str) -> None:
            if not child.finished:
                result = child.cancel()
                cleanup_results.append(result)
                if result.status not in ("cancelled", "exited"):
                    raise RuntimeError(f"Native {role} cleanup could not be confirmed: {result}")

        def close_display_lock() -> None:
            nonlocal display_lock
            if display_lock is not None:
                os.close(display_lock)
                display_lock = None

        def observe_bus_closure() -> None:
            if bus_socket is not None:
                socket_absent = not bus_socket.exists()
                (scratch / "native-bus-closed.json").write_text(json.dumps({
                    "socket": str(bus_socket), "socketAbsent": socket_absent,
                    "scopeClosures": [
                        {"scope": item.scope, "pid": item.pid, "pgid": item.pgid,
                         "status": item.status, "exitCode": item.exit_code,
                         "descendantsReclaimed": item.descendants_reclaimed,
                         "stdout": str(item.stdout), "stderr": str(item.stderr)}
                        for item in cleanup_results
                    ],
                }, indent=2), encoding="utf-8")
                if not socket_absent:
                    raise RuntimeError("Owned private D-Bus socket remained after close")

        def restore_bus_environment() -> None:
            if previous_bus_address is None:
                environment.pop("DBUS_SESSION_BUS_ADDRESS", None)
            else:
                environment["DBUS_SESSION_BUS_ADDRESS"] = previous_bus_address

        # ExitStack finishes every owned closure even if a prior one raises.
        with ExitStack() as cleanup:
            cleanup.callback(restore_bus_environment)
            cleanup.callback(observe_bus_closure)
            if bus is not None:
                cleanup.callback(cancel_scope, bus, "service")
            cleanup.callback(close_display_lock)
            for child in children:
                if child is bus:
                    continue
                if media is not None and child is media.process:
                    cleanup.callback(media.close)
                cleanup.callback(cancel_scope, child, "service")
            if driver is not None:
                cleanup.callback(cancel_scope, driver, "scenario")
