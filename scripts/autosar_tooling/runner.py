"""Live developer output layered on the existing owned process contract."""

from __future__ import annotations

import codecs
import sys
import threading
from pathlib import Path

from ecu_tools.process import OwnedProcess, ProcessResult, ProcessSpec


def run(spec: ProcessSpec, *, json_output: bool = False) -> ProcessResult:
    process = OwnedProcess(spec)
    stopped = threading.Event()
    errors: list[Exception] = []

    def tail() -> None:
        paths = [Path(process.registration[key]) for key in ("stdout", "stderr")]
        offsets = [0, 0]
        decoders = [codecs.getincrementaldecoder("utf-8")("replace") for _ in paths]
        try:
            while True:
                final = stopped.is_set()
                for index, path in enumerate(paths):
                    data = b""
                    if path.exists():
                        with path.open("rb") as source:
                            source.seek(offsets[index])
                            data = source.read()
                        offsets[index] += len(data)
                    text = decoders[index].decode(data, final=final)
                    if text:
                        stream = sys.stderr if json_output or index == 1 else sys.stdout
                        stream.write(text)
                        stream.flush()
                if final:
                    return
                stopped.wait(0.1)
        except (OSError, UnicodeError) as error:
            errors.append(error)

    thread = threading.Thread(target=tail, daemon=True)
    thread.start()
    try:
        result = process.wait()
    except BaseException:
        # wait() marks completion even if interrupted; closing the Windows Job
        # still owns and reclaims the interrupted process tree.
        if process.windows is not None:
            process.windows.close()
        elif not process.finished:
            process.cancel()
        raise
    finally:
        stopped.set()
        thread.join()
    if errors:
        raise RuntimeError(f"Could not stream logs: {errors[0]}")
    return result
