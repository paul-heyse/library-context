"""Installation receipts bind the readers to the selected format and source."""

import json
from pathlib import Path

import pytest

import compile_profile_tools as tools


def installed(root: Path) -> dict:
    (root / "bin").mkdir()
    binaries = {}
    for name in tools.NAMES:
        path = root / "bin" / name
        path.write_text("#!/bin/sh\nexit 0\n")
        path.chmod(0o750)
        binaries[name] = {"path": str(path), "sha256": tools.digest(path)}
    receipt = {
        "version": tools.VERSION,
        "revision": tools.REVISION,
        "format_version": tools.FORMAT_VERSION,
        "toolchain": tools.TOOLCHAIN,
        "lock_sha256": tools.digest(tools.LOCK),
        "patch_sha256": tools.digest(tools.PATCH),
        "binaries": binaries,
    }
    (root / "install.json").write_text(json.dumps(receipt))
    return receipt


def test_missing_install_is_blocked_with_repair(tmp_path):
    result = tools.check(tmp_path)
    assert result["status"] == "blocked"
    assert result["repair"] == "just compile-profile-tools sync"


@pytest.mark.parametrize("key", ["revision", "format_version", "lock_sha256", "patch_sha256"])
def test_incompatible_reader_contract_is_blocked(tmp_path, key):
    receipt = installed(tmp_path)
    receipt[key] = "wrong"
    (tmp_path / "install.json").write_text(json.dumps(receipt))
    assert tools.check(tmp_path)["status"] == "blocked"


def test_changed_executable_is_blocked(tmp_path):
    installed(tmp_path)
    (tmp_path / "bin/crox").write_text("different reader")
    result = tools.check(tmp_path)
    assert result["status"] == "blocked"
    assert any("binary identity differs" in error for error in result["errors"])


def test_current_install_sync_does_no_acquisition(tmp_path, monkeypatch):
    installed(tmp_path)

    def forbidden(*_args, **_kwargs):
        raise AssertionError("valid reader install must not launch another build")

    monkeypatch.setattr(tools.subprocess, "run", forbidden)
    assert tools.sync(tmp_path)["status"] == "passed"
