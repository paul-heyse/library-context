"""Executable closure controls; these do not claim native service qualification."""

import json
import shutil
import subprocess
from pathlib import Path
from types import SimpleNamespace

import pytest

import storage_service as assets
from storage_lifecycle import Blocked, durable_json


def test_owned_executable_survives_checkout_and_state_replacement(tmp_path):
    checkout = tmp_path / "checkout"
    checkout.mkdir()
    executable = checkout / "lctx"
    executable.write_text("#!/bin/sh\nprintf 'owned-installer\\n'\n")
    executable.chmod(0o700)
    state = tmp_path / "surrealdb"
    state.mkdir()
    generation = assets.prepare(state, executable)
    assert not Path(generation["path"]).is_relative_to(state)
    assert assets.digest(executable) == generation["sha256"]
    shutil.rmtree(checkout)
    shutil.rmtree(state)
    result = subprocess.run([generation["path"]], capture_output=True, text=True, check=True)
    assert result.stdout == "owned-installer\n"
    assert Path(generation["path"]).stat().st_mode & 0o777 == 0o500


def test_corrupt_generation_or_symlink_cannot_replace_owned_bytes(tmp_path):
    executable = tmp_path / "lctx"
    executable.write_bytes(b"test")
    executable.chmod(0o700)
    state = tmp_path / "surrealdb"
    generation = assets.prepare(state, executable)
    target = Path(generation["path"])
    target.chmod(0o700)
    target.write_bytes(b"corrupt")
    with pytest.raises(Blocked, match="conflicts"):
        assets.prepare(state, executable)
    link = tmp_path / "link"
    link.symlink_to(executable)
    with pytest.raises(Blocked, match="regular"):
        assets.prepare(state, link)


def test_legacy_backup_rebinds_same_digest_and_preserves_service_identity(tmp_path):
    source = tmp_path / "lctx"
    source.write_bytes(b"same-version")
    source.chmod(0o700)
    generation = assets.prepare(tmp_path / "service", source)
    current = {"installer": generation["path"], "installer_generation": generation}
    archived = {
        "installer": "/removed-checkout/target/release/lctx",
        "installation_id": "fixed",
        "service_generation": [7] * 32,
    }
    rebound = assets.rebind_restored(archived, current, generation["sha256"])
    assert rebound["installer"] == generation["path"]
    assert rebound["installation_id"] == archived["installation_id"]
    assert rebound["service_generation"] == archived["service_generation"]
    with pytest.raises(Blocked, match="differs"):
        assets.rebind_restored(archived, current, "0" * 64)


def test_sanitized_observation_keeps_unresolved_transfer_dependencies(tmp_path):
    root = tmp_path / "service"
    root.mkdir(mode=0o700)
    installation = SimpleNamespace(
        directory=root, id="owned", record={"installer": "/checkout/lctx", "password": "private"}
    )
    durable_json(
        root / "installer-transfer.json",
        {"previous": "/checkout/lctx", "replacement": "/stable/lctx"},
    )
    row = assets.dependencies(installation)
    assert {item["path"] for item in row["dependencies"]} == {"/checkout/lctx", "/stable/lctx"}
    assert "private" not in json.dumps(row)
    assert not row["stable_installer"]
