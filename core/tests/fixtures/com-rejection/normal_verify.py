"""The shipped verify CLI must consume both normal native diagnostic streams."""
from __future__ import annotations

import json
import shutil
import sys
from pathlib import Path

sys.dont_write_bytecode = True
source, directory = map(Path, sys.argv[1:])
directory.mkdir(mode=0o700)
project = directory / "project"
shutil.copytree(source, project)
sys.path.insert(0, str(project / "tools"))
from ecu_tools.build import query

built = directory / "build"
stdout = query([sys.executable, "-I", "-S", str(project / "tools/ecu-tool.py"),
                "verify", "--project", str(project), "--build-directory", str(built)],
               directory, "public-verify", timeout=300)
assert "ECU_HANDOFF_VERIFY PASS:" in stdout and "probe=normal PASS ticks=20" in stdout, stdout
diagnostics = json.loads((built / "logs/probe-com-rejections.json").read_text())
assert diagnostics == {"records": [], "complete": True}, diagnostics
for label in ("echo", "arbitration", "ncr-recovery", "malformed"):
    diagnostics = json.loads((built / f"logs/protocol-{label}-com-rejections.json").read_text())
    assert diagnostics["complete"], diagnostics
print("normal verify PASS: guarded HostBatch independent oracle and normal probe diagnostics parsed/saved", flush=True)
