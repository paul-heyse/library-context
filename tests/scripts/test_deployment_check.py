"""Functional controls for the explicit task process owner; no analyzed fixtures execute."""

import importlib.util
import os
import sys
from pathlib import Path

_SPEC = importlib.util.spec_from_file_location(
    "deployment_check", Path(__file__).parents[2] / "scripts/deployment_check.py"
)
assert _SPEC is not None and _SPEC.loader is not None
_runner = importlib.util.module_from_spec(_SPEC)
_SPEC.loader.exec_module(_runner)


def test_actual_process_output_and_timeout(tmp_path):
    code, out, _ = _runner.bounded(
        [sys.executable, "-c", "print('observed')"], cwd=tmp_path, env=dict(os.environ), timeout=2
    )
    assert code == 0 and out == "observed\n"
    code, _, error = _runner.bounded(
        [sys.executable, "-c", "import time; time.sleep(30)"],
        cwd=tmp_path,
        env=dict(os.environ),
        timeout=0,
    )
    assert code == -1 and error == "task timeout"


def test_actual_output_is_bounded(tmp_path):
    code, out, error = _runner.bounded(
        [
            sys.executable,
            "-c",
            "import sys,time; sys.stdout.write('x'*1000000); sys.stdout.flush(); time.sleep(30)",
        ],
        cwd=tmp_path,
        env=dict(os.environ),
        timeout=2,
    )
    assert code == -1 and error == "task output limit" and len(out) == _runner.OUTPUT_LIMIT
