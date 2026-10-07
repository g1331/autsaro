"""Change actual GTK system media preferences on only the owned Xvfb screen."""

from __future__ import annotations

import ctypes
import json
import os
import re
import shutil
import signal
from pathlib import Path

from ecu_tools.process import OwnedProcess, ProcessSpec, run_bounded


class PrivateMedia:
    def __init__(self, scratch: Path, environment: dict[str, str]):
        display = json.loads((scratch / "native-display.json").read_text())["display"]
        if environment.get("DISPLAY") != display or environment.get("WAYLAND_DISPLAY"):
            raise RuntimeError("System media control requires the run's private Xvfb display")
        programs = {name: shutil.which(name) for name in ("xsettingsd", "dump_xsettings")}
        if not all(programs.values()):
            raise RuntimeError("Private GTK media verification requires external driver tools xsettingsd and dump_xsettings")
        self.scratch = scratch
        self.environment = environment
        self.dump = str(Path(programs["dump_xsettings"]).resolve())
        self.libc = ctypes.CDLL(None, use_errno=True)
        self.libc.pidfd_open.argtypes = [ctypes.c_int, ctypes.c_uint]
        self.libc.pidfd_open.restype = ctypes.c_int
        self.libc.pidfd_send_signal.argtypes = [
            ctypes.c_int, ctypes.c_int, ctypes.c_void_p, ctypes.c_uint
        ]
        self.libc.pidfd_send_signal.restype = ctypes.c_int
        self.config = scratch / "private-xsettings.conf"
        self.write("light", False)
        self.process = OwnedProcess(ProcessSpec.seconds(
            [str(Path(programs["xsettingsd"]).resolve()), "--config", str(self.config)],
            scratch, 1800, scratch, "private-xsettings", env=environment,
        ))
        try:
            self.pidfd = self.libc.pidfd_open(self.process.registration["pid"], 0)
            if self.pidfd < 0:
                error = ctypes.get_errno()
                raise OSError(error, f"pidfd_open: {os.strerror(error)}")
        except BaseException:
            self.process.cancel()
            raise

    def write(self, scheme: str, reduce_motion: bool) -> None:
        theme = "Adwaita-dark" if scheme == "dark" else "Adwaita"
        staging = self.config.with_suffix(".next.conf")
        staging.write_text(f'Net/ThemeName "{theme}"\nGtk/EnableAnimations {int(not reduce_motion)}\n', encoding="utf-8")
        staging.replace(self.config)

    def sample(self) -> None:
        request_path = self.scratch / "native-media-request.json"
        if not request_path.is_file():
            return
        request = json.loads(request_path.read_text(encoding="utf-8"))
        scheme = request["scheme"]
        reduced = request["reduceMotion"]
        if scheme not in ("light", "dark") or not isinstance(reduced, bool):
            raise RuntimeError("Unsupported private system media request")
        self.write(scheme, reduced)
        # pidfd retains the exact daemon identity; never signal a recycled PID.
        if self.libc.pidfd_send_signal(self.pidfd, signal.SIGHUP, None, 0) != 0:
            error = ctypes.get_errno()
            raise OSError(error, f"pidfd_send_signal: {os.strerror(error)}")
        observed = run_bounded(ProcessSpec.seconds(
            [self.dump], self.scratch, 5, self.scratch, "private-xsettings-observation", env=self.environment,
        ))
        if observed.status != "exited" or observed.exit_code != 0:
            raise RuntimeError(f"Private XSettings observation failed: {observed}")
        text = observed.stdout.read_text(encoding="utf-8")
        theme = re.search(r'^Net/ThemeName\s+"([^"]+)"\s*$', text, re.MULTILINE)
        animations = re.search(r'^Gtk/EnableAnimations\s+([01])\s*$', text, re.MULTILINE)
        expected = "Adwaita-dark" if scheme == "dark" else "Adwaita"
        if not theme or theme[1] != expected or not animations or animations[1] != str(int(not reduced)):
            return  # The actual X11 property has not acknowledged the reload yet.
        receipt = {**request, "scope": "private-xvfb-xsettings", "display": self.environment["DISPLAY"], "daemonPid": self.process.registration["pid"], "observed": text}
        destination = self.scratch / "native-media-result.json"
        staging = destination.with_suffix(".next.json")
        staging.write_text(json.dumps(receipt), encoding="utf-8")
        staging.replace(destination)
        request_path.unlink()

    def close(self) -> None:
        os.close(self.pidfd)
