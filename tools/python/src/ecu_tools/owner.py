"""Short-lived POSIX supervisor for registered, cooperative process groups.

The supervisor is the parent of each launcher, not the workbench. A launcher
cannot execute its command until its group has been registered and its guardian
has mirrored the PID/PGID. Only this supervisor can release or close a scope.
"""

from __future__ import annotations

import argparse
import ctypes
import json
import os
import secrets
import signal
import socket
import stat
import subprocess
import sys
import tempfile
import threading
import time
from contextlib import ExitStack
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Self


class OwnershipError(RuntimeError):
    """A command failed or its owned processes could not be confirmed closed."""


def _group_alive(pgid: int) -> bool:
    try:
        os.killpg(pgid, 0)
        return True
    except ProcessLookupError:
        return False


def _signal_group(pgid: int, sig: signal.Signals) -> None:
    try:
        os.killpg(pgid, sig)
    except ProcessLookupError:
        pass


def _reap_group(pgid: int) -> None:
    if sys.platform != "linux":
        return
    while True:
        try:
            pid, _ = os.waitpid(-pgid, os.WNOHANG)
        except ChildProcessError:
            return
        if pid == 0:
            return


def _escaped_child(scope: str, pgid: int, registered_roots: set[int]) -> bool:
    if sys.platform != "linux":
        return False
    tasks = Path(f"/proc/{os.getpid()}/task")
    pids: set[str] = set()
    try:
        for record in tasks.glob("*/children"):
            try:
                pids.update(record.read_text(encoding="ascii").split())
            except FileNotFoundError:
                # Connection threads can exit after the proc snapshot; a
                # vanished task is not an unobservable live child.
                continue
    except OSError:
        return True  # An unobservable adopted child cannot establish clean closure.
    marker = f"ECU_OWNER_SCOPE={scope}".encode()
    for item in pids:
        pid = int(item)
        try:
            if os.getpgid(pid) == pgid:
                continue
            fields = Path(f"/proc/{pid}/stat").read_bytes().rsplit(b")", 1)[-1].split()
            if (
                fields[0] == b"Z"
                and int(fields[1]) == os.getpid()
                and pid not in registered_roots
            ):
                # A subreaper can adopt an exited member of another scope.
                # Reap that exact child; never steal a launcher's Popen status.
                reaped, _ = os.waitpid(pid, os.WNOHANG)
                if reaped == pid:
                    continue
            if marker in Path(f"/proc/{pid}/environ").read_bytes().split(b"\0"):
                return True
        except ProcessLookupError:
            continue
        except OSError:
            return True
    return False


@dataclass
class Scope:
    identity: str
    parent: str | None
    deadline_ns: int
    gate_deadline_ns: int
    child: subprocess.Popen[bytes]
    pgid: int
    gate: int | None
    stdout: Path
    stderr: Path
    completion: str
    released: bool = False
    result: dict[str, Any] | None = None
    closing: bool = False


class Registry:
    def __init__(self, token: str, path: Path) -> None:
        self.token = token
        self.path = path
        self.lock = threading.RLock()
        self.scopes: dict[str, Scope] = {}
        self.closing = False
        self.terminate = threading.Event()

    def _reserve(self, request: dict[str, Any]) -> dict[str, Any]:
        if self.closing:
            raise OwnershipError("root_closing: no new scopes are accepted")
        parent = request.get("parent")
        if parent is not None:
            ancestor = self.scopes.get(parent)
            if ancestor is None or ancestor.closing or ancestor.result is not None:
                raise OwnershipError("parent scope is not live")
        else:
            ancestor = None
        deadline = int(request["deadline_ns"])
        if ancestor is not None:
            deadline = min(deadline, ancestor.deadline_ns)
        if deadline <= time.monotonic_ns():
            raise OwnershipError("deadline expired before registration")
        completion = request.get("completion", "require_tree_exit")
        if completion not in ("require_tree_exit", "close_tree_on_exit"):
            raise OwnershipError("unknown completion policy")
        argv = request["argv"]
        if (
            not isinstance(argv, list)
            or not argv
            or not all(isinstance(item, str) for item in argv)
        ):
            raise OwnershipError("argv must be a non-empty argument vector")
        if not Path(argv[0]).is_absolute():
            raise OwnershipError("executable must be an absolute path")
        cwd = Path(request["cwd"])
        if not cwd.is_dir():
            raise OwnershipError(f"command working directory does not exist: {cwd}")
        directory = Path(request["log_directory"])
        metadata = directory.lstat()
        if not stat.S_ISDIR(metadata.st_mode) or (metadata.st_mode & 0o077):
            raise OwnershipError(
                "log_directory must be a private, non-symlink directory"
            )
        stdin_path = request.get("stdin_file")
        stdin_fifo = request.get("stdin_fifo", False)
        if not isinstance(stdin_fifo, bool) or (stdin_fifo and stdin_path is None):
            raise OwnershipError("stdin_fifo requires an explicit FIFO path")
        if stdin_path is not None:
            selected_input = Path(stdin_path)
            if not selected_input.is_absolute() or selected_input.is_symlink():
                raise OwnershipError("stdin requires an absolute non-symlink path")
            input_metadata = selected_input.lstat()
            if stdin_fifo:
                parent_metadata = selected_input.parent.lstat()
                if (
                    not stat.S_ISFIFO(input_metadata.st_mode)
                    or not stat.S_ISDIR(parent_metadata.st_mode)
                    or parent_metadata.st_mode & 0o077
                ):
                    raise OwnershipError("stdin_fifo requires an owner-private FIFO")
            elif not stat.S_ISREG(input_metadata.st_mode):
                raise OwnershipError("stdin_file requires an absolute regular file")
        capture = Path(tempfile.mkdtemp(prefix="ecu-command-", dir=directory))
        identity = secrets.token_hex(16)
        stdout = capture / "stdout.log"
        stderr = capture / "stderr.log"
        read_gate, write_gate = os.pipe()
        environment = os.environ.copy()
        environment.update(request.get("env") or {})
        environment["ECU_OWNER_SCOPE"] = identity
        environment["ECU_OWNER_SOCKET"] = str(self.path)
        environment["ECU_OWNER_TOKEN"] = self.token
        try:
            with (
                os.fdopen(
                    os.open(stdout, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600), "wb"
                ) as out,
                os.fdopen(
                    os.open(stderr, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600), "wb"
                ) as err,
                open(stdin_path or os.devnull, "rb") as input_stream,
            ):
                child = subprocess.Popen(
                    [
                        sys.executable,
                        "-I",
                        "-S",
                        str(Path(__file__).resolve().with_name("launch.py")),
                        str(read_gate),
                        "--",
                        *argv,
                    ],
                    cwd=cwd,
                    env=environment,
                    stdin=input_stream,
                    stdout=out,
                    stderr=err,
                    pass_fds=(read_gate,),
                    start_new_session=True,
                )
            os.close(read_gate)
            read_gate = -1
            registration_end = min(deadline, time.monotonic_ns() + 30_000_000_000)
            while time.monotonic_ns() < registration_end:
                try:
                    if os.getpgid(child.pid) == child.pid:
                        break
                except ProcessLookupError:
                    pass
                if child.poll() is not None:
                    raise OwnershipError("launcher exited before group registration")
                time.sleep(0.005)
            else:
                raise OwnershipError("launcher group registration timed out")
            self.scopes[identity] = Scope(
                identity,
                parent,
                deadline,
                min(deadline, time.monotonic_ns() + 30_000_000_000),
                child,
                child.pid,
                write_gate,
                stdout,
                stderr,
                completion,
            )
            return {
                "op": "registered",
                "scope": identity,
                "pid": child.pid,
                "pgid": child.pid,
                "deadline_ns": deadline,
                "stdout": str(stdout),
                "stderr": str(stderr),
            }
        except Exception:
            os.close(write_gate)
            if "child" in locals():
                try:
                    _signal_group(child.pid, signal.SIGKILL)
                    child.kill()
                except ProcessLookupError:
                    pass
                child.wait(timeout=2)
            raise
        finally:
            if read_gate >= 0:
                os.close(read_gate)

    def _release(self, scope: Scope) -> dict[str, Any]:
        if (
            scope.closing
            or scope.result is not None
            or scope.released
            or scope.gate is None
        ):
            raise OwnershipError("scope cannot be released")
        os.write(scope.gate, b"1")
        os.close(scope.gate)
        scope.gate = None
        scope.released = True
        return {"op": "released", "scope": scope.identity}

    def _finish(self, scope: Scope, reason: str) -> dict[str, Any]:
        if scope.result is not None:
            return scope.result
        scope.child.poll()
        if scope.child.returncode is not None:
            _reap_group(scope.pgid)
        orphaned = reason == "normal" and _group_alive(scope.pgid)
        scope.closing = True
        if scope.gate is not None:
            os.close(scope.gate)
            scope.gate = None
        if reason != "normal" or orphaned:
            _signal_group(scope.pgid, signal.SIGTERM)
        end = time.monotonic() + (2 if reason != "normal" or orphaned else 0.1)
        while time.monotonic() < end:
            scope.child.poll()
            if scope.child.returncode is not None:
                _reap_group(scope.pgid)
            if not _group_alive(scope.pgid):
                break
            time.sleep(0.01)
        if _group_alive(scope.pgid):
            _signal_group(scope.pgid, signal.SIGKILL)
            end = time.monotonic() + 0.2
            while time.monotonic() < end:
                scope.child.poll()
                if scope.child.returncode is not None:
                    _reap_group(scope.pgid)
                if not _group_alive(scope.pgid):
                    break
                time.sleep(0.01)
        scope.child.poll()
        if scope.child.returncode is not None:
            _reap_group(scope.pgid)
        # Poll every registered launcher through Popen before reaping adopted
        # descendants, so a sibling's original nonzero exit remains observable.
        registered_roots = {
            entry.child.pid
            for entry in self.scopes.values()
            if entry.child.poll() is None
        }
        unconfirmed = _group_alive(scope.pgid) or _escaped_child(
            scope.identity, scope.pgid, registered_roots
        )
        status = (
            "cleanup_unconfirmed"
            if unconfirmed
            else "orphaned_members"
            if orphaned and scope.completion == "require_tree_exit"
            else reason
            if reason != "normal"
            else "exited"
        )
        scope.result = {
            "op": "closed",
            "scope": scope.identity,
            "status": status,
            "exit_code": scope.child.returncode,
            "stdout": str(scope.stdout),
            "stderr": str(scope.stderr),
            "descendants_reclaimed": reason == "normal"
            and orphaned
            and not unconfirmed,
        }
        return scope.result

    def _close(self, identity: str, reason: str) -> dict[str, Any]:
        scope = self.scopes[identity]
        descendant_results = []
        live_descendants = False
        for child in list(self.scopes.values()):
            if child.parent == identity and child.result is None:
                live = child.child.poll() is None
                live_descendants |= live
                child_reason = "cancelled" if reason == "normal" and live else reason
                descendant_results.append(self._close(child.identity, child_reason))
        result = self._finish(scope, reason)
        if result["status"] == "cleanup_unconfirmed" or any(
            child["status"] == "cleanup_unconfirmed" for child in descendant_results
        ):
            result["status"] = "cleanup_unconfirmed"
            result["descendants_reclaimed"] = False
        elif reason == "normal" and (
            live_descendants
            or any(child["descendants_reclaimed"] for child in descendant_results)
        ):
            if scope.completion == "require_tree_exit":
                result["status"] = "orphaned_members"
            result["descendants_reclaimed"] = True
        return result

    def watchdog(self) -> None:
        while not self.terminate.wait(0.02):
            with self.lock:
                now = time.monotonic_ns()
                for scope in list(self.scopes.values()):
                    if scope.result is None and (
                        now >= scope.deadline_ns
                        or (not scope.released and now >= scope.gate_deadline_ns)
                    ):
                        self._close(scope.identity, "timeout")

    def dispatch(self, request: dict[str, Any]) -> dict[str, Any]:
        if request.get("token") != self.token:
            raise OwnershipError("invalid owner capability")
        op = request.get("op")
        with self.lock:
            if op == "ready":
                return {"op": "ready"}
            if op == "reserve":
                return self._reserve(request)
            if op == "shutdown":
                self.closing = True
                results = [
                    self._close(scope.identity, "cancelled")
                    for scope in list(self.scopes.values())
                    if scope.result is None
                ]
                return {
                    "op": "closed",
                    "status": "cleanup_unconfirmed"
                    if any(item["status"] == "cleanup_unconfirmed" for item in results)
                    else "closed",
                }
            identity = request.get("scope")
            if not isinstance(identity, str) or identity not in self.scopes:
                raise OwnershipError("unknown scope")
            scope = self.scopes[identity]
            if op == "release":
                return self._release(scope)
            if op == "close":
                return self._close(identity, str(request.get("reason", "cancelled")))
            if op == "closed":
                if scope.result is not None:
                    return scope.result
                exit_code = scope.child.poll()
                if exit_code is None and time.monotonic_ns() < scope.deadline_ns:
                    return {"op": "running", "scope": identity}
                return self._close(
                    identity, "timeout" if exit_code is None else "normal"
                )
        raise OwnershipError(f"unknown owner operation: {op}")


def _serve_connection(registry: Registry, connection: socket.socket) -> None:
    with connection:
        request: dict[str, Any] = {}
        try:
            connection.settimeout(30)
            data = bytearray()
            while not data.endswith(b"\n"):
                fragment = connection.recv(65536)
                if not fragment or len(data) + len(fragment) > 1_048_576:
                    raise OwnershipError("owner request incomplete or too large")
                data.extend(fragment)
            request = json.loads(data)
            response = registry.dispatch(request)
        except (OSError, ValueError, KeyError, TypeError, OwnershipError) as error:
            response = {"op": "error", "message": str(error)}
        try:
            connection.sendall(json.dumps(response).encode() + b"\n")
        except OSError:
            if request.get("op") == "reserve" and response.get("op") == "registered":
                with registry.lock:
                    registry._close(response["scope"], "cancelled")
        if request.get("op") == "shutdown" and response.get("op") == "closed":
            registry.terminate.set()


def serve(path: Path, token: str, guardian_pid: int) -> None:
    if sys.platform == "linux":
        if ctypes.CDLL(None, use_errno=True).prctl(36, 1, 0, 0, 0) != 0:
            raise OSError(ctypes.get_errno(), "enable subreaper")
    registry = Registry(token, path)
    threading.Thread(target=registry.watchdog, daemon=True).start()
    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as listener:
        listener.bind(str(path))
        path.chmod(0o600)
        listener.listen(32)
        listener.settimeout(0.1)
        while not registry.terminate.is_set():
            if os.getppid() != guardian_pid:
                with registry.lock:
                    registry.closing = True
                    for scope in list(registry.scopes.values()):
                        if scope.result is None:
                            result = registry._close(scope.identity, "cancelled")
                            if result["status"] == "cleanup_unconfirmed":
                                print(
                                    f"guardian_exited: cleanup_unconfirmed scope={scope.identity}",
                                    file=sys.stderr,
                                    flush=True,
                                )
                registry.terminate.set()
                break
            try:
                connection, _ = listener.accept()
            except TimeoutError:
                continue
            threading.Thread(
                target=_serve_connection, args=(registry, connection), daemon=True
            ).start()
    path.unlink(missing_ok=True)


class Owner:
    """Guardian for one private supervisor; nested clients reuse the inherited one."""

    def __init__(
        self,
        socket_path: Path,
        token: str,
        process: subprocess.Popen[bytes] | None,
        directory: Path | None,
        directory_fd: int | None = None,
    ) -> None:
        self.socket_path = socket_path
        self.token = token
        self.process = process
        self.directory = directory
        self._directory_fd = directory_fd
        self.groups: dict[str, int] = {}

    @classmethod
    def start(cls) -> Owner:
        if os.name == "nt":
            raise OwnershipError("POSIX owner requires a Unix socket")
        directory = Path(tempfile.mkdtemp(prefix="ecu-owner-"))
        directory.chmod(0o700)
        path = directory / "owner.sock"
        with ExitStack() as handles:
            directory_fd = None
            if sys.platform == "linux" and len(os.fsencode(path)) >= 108:
                # Keep the socket private without imposing sockaddr_un's
                # pathname limit on the caller's temporary directory.
                directory_fd = os.open(directory, os.O_RDONLY | os.O_DIRECTORY)
                handles.callback(os.close, directory_fd)
                path = Path(f"/proc/{os.getpid()}/fd/{directory_fd}/owner.sock")
            token = secrets.token_hex(32)
            environment = os.environ.copy()
            with os.fdopen(
                os.open(
                    directory / "supervisor.log",
                    os.O_WRONLY | os.O_CREAT | os.O_EXCL,
                    0o600,
                ),
                "wb",
            ) as log:
                process = subprocess.Popen(
                    [
                        sys.executable,
                        "-I",
                        "-S",
                        str(Path(__file__).resolve()),
                        "--socket",
                        str(path),
                        "--guardian-pid",
                        str(os.getpid()),
                    ],
                    stdin=subprocess.PIPE,
                    stdout=log,
                    stderr=log,
                    start_new_session=True,
                    env=environment,
                )
            assert process.stdin is not None
            process.stdin.write((token + "\n").encode())
            process.stdin.close()
            owner = cls(path, token, process, directory, directory_fd)
            end = time.monotonic() + 30
            while time.monotonic() < end:
                if path.is_socket():
                    try:
                        if owner.request("ready", request_deadline=end) != {"op": "ready"}:
                            raise OwnershipError("supervisor_failed: invalid readiness response")
                    except OwnershipError as error:
                        # bind() publishes the socket before listen(). Only this
                        # pre-registration startup race may be retried.
                        if not isinstance(error.__cause__, ConnectionRefusedError):
                            process.kill()
                            process.wait()
                            raise
                    else:
                        handles.pop_all()
                        return owner
                if process.poll() is not None:
                    raise OwnershipError(
                        f"supervisor_failed: {directory / 'supervisor.log'}"
                    )
                time.sleep(0.01)
            process.kill()
            process.wait()
            raise OwnershipError(f"supervisor_failed: socket unavailable at {path}")

    @classmethod
    def inherited(cls) -> Owner:
        return cls(
            Path(os.environ["ECU_OWNER_SOCKET"]),
            os.environ["ECU_OWNER_TOKEN"],
            None,
            None,
        )

    def _fallback(self) -> None:
        for pgid in list(self.groups.values()):
            _signal_group(pgid, signal.SIGTERM)
        end = time.monotonic() + 2
        while time.monotonic() < end and any(
            _group_alive(pgid) for pgid in self.groups.values()
        ):
            time.sleep(0.01)
        for pgid in self.groups.values():
            _signal_group(pgid, signal.SIGKILL)

    def request(
        self, op: str, *, request_deadline: float | None = None, **fields: Any
    ) -> dict[str, Any]:
        def remaining_timeout() -> float:
            if request_deadline is None:
                return 30
            remaining = request_deadline - time.monotonic()
            if remaining <= 0:
                raise TimeoutError("supervisor request deadline expired")
            return remaining

        try:
            with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as connection:
                connection.settimeout(remaining_timeout())
                connection.connect(str(self.socket_path))
                connection.settimeout(remaining_timeout())
                connection.sendall(
                    json.dumps({"token": self.token, "op": op, **fields}).encode()
                    + b"\n"
                )
                data = bytearray()
                while not data.endswith(b"\n"):
                    connection.settimeout(remaining_timeout())
                    fragment = connection.recv(65536)
                    if not fragment:
                        raise OwnershipError("supervisor socket closed")
                    data.extend(fragment)
            response = json.loads(data)
        except (OSError, ValueError, OwnershipError) as error:
            self._fallback()
            raise OwnershipError(f"supervisor_failed: {error}") from error
        if response.get("op") == "error":
            raise OwnershipError(response["message"])
        if response.get("op") == "closed" and response.get("scope"):
            self.groups.pop(response["scope"], None)
        return response

    def reserve(
        self,
        argv: list[str],
        cwd: Path,
        deadline_ns: int,
        log_directory: Path,
        *,
        parent: str | None = None,
        env: dict[str, str] | None = None,
        stdin_file: Path | None = None,
        completion: str = "require_tree_exit",
    ) -> dict[str, Any]:
        registration = self.request(
            "reserve",
            parent=parent,
            deadline_ns=deadline_ns,
            argv=argv,
            cwd=str(cwd),
            log_directory=str(log_directory),
            env=env,
            stdin_file=str(stdin_file) if stdin_file is not None else None,
            completion=completion,
        )
        self.groups[registration["scope"]] = int(registration["pgid"])
        return registration

    def close(self) -> None:
        if self.process is None:
            return
        succeeded = False
        try:
            response = self.request("shutdown")
            if response["status"] != "closed":
                raise OwnershipError(response["status"])
            succeeded = True
        finally:
            try:
                try:
                    exit_code = self.process.wait(timeout=3)
                except subprocess.TimeoutExpired as error:
                    self.process.kill()
                    self.process.wait()
                    raise OwnershipError(
                        f"supervisor_failed: shutdown timed out; {self.directory}"
                    ) from error
                if succeeded and exit_code == 0 and self.directory is not None:
                    self.socket_path.unlink(missing_ok=True)
                    (self.directory / "supervisor.log").unlink(missing_ok=True)
                    self.directory.rmdir()
            finally:
                if self._directory_fd is not None:
                    os.close(self._directory_fd)
                    self._directory_fd = None

    def __enter__(self) -> Self:
        return self

    def __exit__(self, *_: object) -> None:
        self.close()


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--socket", type=Path, required=True)
    parser.add_argument("--guardian-pid", type=int, required=True)
    args = parser.parse_args()
    token = sys.stdin.readline().strip()
    if len(token) != 64:
        raise OwnershipError("supervisor capability missing")
    serve(args.socket, token, args.guardian_pid)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
