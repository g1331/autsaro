"""One argument-vector command inside a deadline-bound owned scope."""

from __future__ import annotations

import os
import tempfile
import time
from dataclasses import dataclass
from pathlib import Path

from ecu_tools.owner import Owner, OwnershipError


@dataclass(frozen=True)
class ProcessSpec:
    argv: tuple[str, ...]
    cwd: Path
    env: dict[str, str] | None
    deadline_ns: int
    log_directory: Path
    stage: str
    stdin_file: Path | None = None

    @classmethod
    def seconds(
        cls,
        argv: list[str],
        cwd: Path,
        seconds: int,
        log_directory: Path,
        stage: str,
        env: dict[str, str] | None = None,
        stdin_file: Path | None = None,
    ) -> ProcessSpec:
        return cls(
            tuple(argv),
            cwd,
            env,
            time.monotonic_ns() + seconds * 1_000_000_000,
            log_directory,
            stage,
            stdin_file,
        )


@dataclass(frozen=True)
class ProcessResult:
    scope: str
    pid: int
    pgid: int | None
    exit_code: int | None
    status: str
    stdout: Path
    stderr: Path

    @property
    def success(self) -> bool:
        return self.status == "exited" and self.exit_code == 0


class OwnedProcess:
    def __init__(
        self,
        spec: ProcessSpec,
        *,
        owner: Owner | None = None,
        parent: str | None = None,
    ) -> None:
        if not spec.argv or not Path(spec.argv[0]).is_absolute():
            raise OwnershipError("ProcessSpec requires an absolute executable argv")
        if spec.deadline_ns <= time.monotonic_ns():
            raise OwnershipError(f"{spec.stage}: deadline expired before launch")
        self.spec = spec
        self.owner = owner
        self.own_root = False
        self.windows = None
        self.finished = False
        if os.name == "nt":
            from ecu_tools.windows_job import WindowsJob

            capture = Path(
                tempfile.mkdtemp(prefix="ecu-command-", dir=spec.log_directory)
            )
            self.registration = {
                "scope": f"windows-job-{capture.name}",
                "pid": 0,
                "pgid": None,
                "stdout": str(capture / "stdout.log"),
                "stderr": str(capture / "stderr.log"),
            }
            self.windows = WindowsJob(
                list(spec.argv),
                spec.cwd,
                spec.env,
                Path(self.registration["stdout"]),
                Path(self.registration["stderr"]),
                spec.stdin_file,
            )
            self.registration["pid"] = self.windows.pid
        else:
            if owner is None:
                owner = (
                    Owner.inherited()
                    if os.environ.get("ECU_OWNER_SOCKET")
                    else Owner.start()
                )
                self.own_root = owner.process is not None
            self.owner = owner
            try:
                self.registration = owner.reserve(
                    list(spec.argv),
                    spec.cwd,
                    spec.deadline_ns,
                    spec.log_directory,
                    parent=parent or os.environ.get("ECU_OWNER_SCOPE"),
                    stdin_file=spec.stdin_file,
                    env=spec.env,
                )
                owner.request("release", scope=self.registration["scope"])
            except BaseException:
                if hasattr(self, "registration"):
                    try:
                        owner.request(
                            "close",
                            scope=self.registration["scope"],
                            reason="cancelled",
                        )
                    except OwnershipError:
                        pass
                if self.own_root:
                    owner.close()
                raise

    @property
    def scope(self) -> str:
        return str(self.registration["scope"])

    def _result(self, reply: dict[str, object]) -> ProcessResult:
        return ProcessResult(
            self.scope,
            int(self.registration["pid"]),
            int(self.registration["pgid"])
            if self.registration["pgid"] is not None
            else None,
            int(reply["exit_code"]) if reply["exit_code"] is not None else None,
            str(reply["status"]),
            Path(self.registration["stdout"]),
            Path(self.registration["stderr"]),
        )

    def wait(self) -> ProcessResult:
        if self.finished:
            raise OwnershipError("owned scope already completed")
        try:
            if self.windows is not None:
                status, code = self.windows.wait(self.spec.deadline_ns)
                return self._result({"status": status, "exit_code": code})
            assert self.owner is not None
            while True:
                response = self.owner.request("closed", scope=self.scope)
                if response["op"] == "closed":
                    return self._result(response)
                if response["op"] != "running":
                    raise OwnershipError(f"unexpected supervisor state: {response}")
                time.sleep(
                    min(
                        0.02,
                        max(0, (self.spec.deadline_ns - time.monotonic_ns()) / 1e9),
                    )
                )
        finally:
            self.finished = True
            if self.own_root:
                assert self.owner is not None
                self.owner.close()

    def cancel(self) -> ProcessResult:
        if self.finished:
            raise OwnershipError("owned scope already completed")
        try:
            if self.windows is not None:
                self.windows.close()
                return self._result(
                    {"status": "cancelled", "exit_code": self.windows.poll()}
                )
            assert self.owner is not None
            return self._result(
                self.owner.request("close", scope=self.scope, reason="cancelled")
            )
        finally:
            self.finished = True
            if self.own_root:
                assert self.owner is not None
                self.owner.close()


def run_bounded(
    spec: ProcessSpec, *, owner: Owner | None = None, parent: str | None = None
) -> ProcessResult:
    process = OwnedProcess(spec, owner=owner, parent=parent)
    result = process.wait()
    if not result.success:
        raise OwnershipError(
            f"{spec.stage}: argv={list(spec.argv)!r} deadline_ns={spec.deadline_ns} "
            f"status={result.status} exit_code={result.exit_code} "
            f"stdout={result.stdout} stderr={result.stderr}"
        )
    return result
