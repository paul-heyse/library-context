from __future__ import annotations

import os
import subprocess
import sys
from pathlib import Path

import pytest

from build_environment import ROOT, normalized_env, shell_changes


@pytest.mark.parametrize("value", ["", " ", "target", "target/../target", "/other/checkout/target"])
def test_default_and_foreign_targets_are_removed(tmp_path: Path, value: str) -> None:
    source = {"CARGO_TARGET_DIR": value, "CARGO_BUILD_TARGET_DIR": value, "KEEP": "1"}
    assert normalized_env(source, tmp_path) == {"KEEP": "1"}
    assert source["CARGO_TARGET_DIR"] == value


def test_explicit_targets_and_non_target_configuration(tmp_path: Path) -> None:
    source = {"CARGO_TARGET_DIR": "target/measure", "SCCACHE_SERVER_UDS": "/cache/socket"}
    assert normalized_env(source, tmp_path) == source
    source["LCTX_CARGO_TARGET_DIR"] = "/explicit/external target"
    assert normalized_env(source, tmp_path)["CARGO_TARGET_DIR"] == "/explicit/external target"


def test_shell_unsets_and_quotes_and_exec_propagates(tmp_path: Path) -> None:
    source = dict(os.environ, CARGO_TARGET_DIR=str(ROOT / "target"))
    shell = shell_changes(source, normalized_env(source))
    result = subprocess.check_output(
        ["bash", "-c", f'{shell}\n test -z "${{CARGO_TARGET_DIR+x}}" && echo cleared'],
        env=source,
        text=True,
    )
    assert result.strip() == "cleared"
    assert (
        shell_changes({}, {"CARGO_TARGET_DIR": "a b'c"}) == "export CARGO_TARGET_DIR='a b'\"'\"'c'"
    )
    result = subprocess.check_output(
        [
            sys.executable,
            str(ROOT / "scripts/build_environment.py"),
            "--",
            sys.executable,
            "-c",
            "import os; print(os.getenv('CARGO_TARGET_DIR', 'cleared'))",
        ],
        cwd=tmp_path,
        env=source,
        text=True,
    )
    assert result.strip() == "cleared"
