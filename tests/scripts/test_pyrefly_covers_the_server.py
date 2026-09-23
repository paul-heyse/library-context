"""`just py-check` type-checks the served package (increment-1 deep review F6): its editable
install must not make pyrefly treat it as a site package and skip it."""

from __future__ import annotations

import shutil
import subprocess
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]


def test_pyrefly_checks_the_server_source() -> None:
    pyrefly = shutil.which("pyrefly")
    assert pyrefly is not None, "pyrefly is a dev dependency"
    out = subprocess.run(
        [pyrefly, "dump-config"], cwd=REPO, capture_output=True, text=True, check=True
    )
    text = out.stdout + out.stderr
    assert "python/lctx_mcp/src/lctx_mcp/server.py" in text
    assert "Skipping include pattern" not in text
