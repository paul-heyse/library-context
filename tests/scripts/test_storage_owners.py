"""Storage uses producer authority, including real owner locks, rather than cached state."""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

import pytest

import runs
import storage_owners as owners
from harness import ProcessIdentity, write_json_atomic


def ended_run(path: Path) -> dict:
    path.mkdir()
    write_json_atomic(
        path / "record.json",
        {
            "schema": 2,
            "id": path.name,
            "owner": ProcessIdentity.of().to_json(),
            "child": None,
            "termination": "completed",
            "cleanup": {"status": "confirmed"},
            "ended": "2026-10-09T00:00:00+00:00",
        },
    )
    return {"owner": {"kind": "run", "path": str(path)}, "path": str(path)}


def test_retention_and_cleanup_are_read_from_run_owner(tmp_path):
    row = ended_run(tmp_path / "run")
    with owners.owner_guard(row):
        assert owners.observe(row)["state"] == "released"
        (tmp_path / "run" / "retain").touch()
        assert owners.observe(row)["state"] == "retained"
    (tmp_path / "run" / "retain").unlink()
    value = runs.load_record(tmp_path / "run")
    assert value is not None
    value["cleanup"] = {"status": "unknown"}
    write_json_atomic(tmp_path / "run" / "record.json", value)
    assert owners.observe(row)["protected"]


def test_real_owner_lock_blocks_effect_and_observation(tmp_path):
    row = ended_run(tmp_path / "run")
    lock = tmp_path / "run" / "owner.lock"
    child = subprocess.Popen(
        [
            sys.executable,
            "-c",
            "import fcntl,sys,time; f=open(sys.argv[1],'w'); "
            "fcntl.flock(f,fcntl.LOCK_EX); print('ready',flush=True); time.sleep(60)",
            str(lock),
        ],
        stdout=subprocess.PIPE,
        text=True,
    )
    try:
        assert child.stdout is not None
        assert child.stdout.readline().strip() == "ready"
        assert owners.observe(row)["state"] == "active"
        with pytest.raises(owners.OwnerBusy), owners.owner_guard(row):
            pytest.fail("effect was admitted")
    finally:
        child.terminate()
        child.wait()
    with owners.owner_guard(row):
        assert owners.observe(row)["state"] == "released"


def test_unknown_native_and_environment_stay_protected(tmp_path):
    for kind in ("native", "external", "environment", "unknown"):
        assert owners.observe(
            {"owner": {"kind": kind, "path": str(tmp_path)}, "path": str(tmp_path)}
        )["protected"]


def test_recovery_cleanup_date_not_old_child_exit_starts_lifetime(tmp_path):
    row = ended_run(tmp_path / "run")
    value = runs.load_record(tmp_path / "run")
    assert value is not None
    value["ended"] = "2020-01-01T00:00:00+00:00"
    value["cleanup"]["observed"] = "2026-10-09T01:02:03+00:00"
    write_json_atomic(tmp_path / "run" / "record.json", value)
    with owners.owner_guard(row):
        assert owners.observe(row)["released_at"] == value["cleanup"]["observed"]


def test_superseded_acquisition_pin_does_not_imply_released_readers(tmp_path):
    project = tmp_path / "project"
    project.mkdir()
    (project / "pyproject.toml").write_text('[tool.lctx.source]\ncommit="current"\n')
    (project / "uv.lock").touch()
    (project / ".python-version").write_text("3.14.7\n")
    row = {
        "owner": {"kind": "acquisition", "path": str(project)},
        "category": "acquired-source",
        "path": str(tmp_path / "current"),
    }
    assert owners.observe(row)["state"] == "warm"
    row["path"] = str(tmp_path / "previous")
    assert owners.observe(row)["state"] == "unresolved"


def test_category_temporary_policy_is_prospective_and_none_remains_indefinite(
    tmp_path, monkeypatch
):
    import storage_lifecycle

    RealStorage = storage_lifecycle.Storage
    config = tmp_path / "storage.toml"
    policy = (Path(__file__).parents[2] / ".config/storage.toml").read_text()
    config.write_text(policy.replace("temporary_days = 90", "temporary_days = 3"))
    monkeypatch.setattr(storage_lifecycle, "Storage", lambda: RealStorage(config=config))
    first = tmp_path / "first"
    owner = ended_run(first)["owner"]
    ident = owners.enroll(first, "run-receipt", owner, "temporary", temporary_days=90)
    with owners.owner_guard({"owner": owner, "path": str(first)}):
        assert owners.complete(ident, "temporary")
    assert ident is not None
    old = RealStorage(config=config).get(ident)["obligations"]["temporary"]
    assert old["temporary_days"] == 3 and old["until"]
    config.write_text(policy.replace("temporary_days = 90", "temporary_days = 7"))
    second = tmp_path / "second"
    owner = ended_run(second)["owner"]
    current = owners.enroll(second, "run-receipt", owner, "temporary", temporary_days=90)
    indefinite = owners.enroll(second, "run-receipt", owner, "named-evidence")
    assert current is not None and indefinite is not None
    store = RealStorage(config=config)
    assert store.get(ident)["obligations"]["temporary"] == old
    assert store.get(current)["obligations"]["temporary"]["temporary_days"] == 7
    assert store.get(indefinite)["obligations"]["named-evidence"]["temporary_days"] is None
