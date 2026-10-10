"""Public CLI scope, partial effects and archive trust boundaries."""

from __future__ import annotations

import contextlib
import hashlib
import json
import subprocess

import pytest

import storage
import storage_lifecycle as lifecycle


@pytest.fixture
def store(tmp_path, monkeypatch):
    repo = tmp_path / "repository"
    (repo / ".config").mkdir(parents=True)
    (repo / ".config/storage.toml").write_bytes(
        (lifecycle.ROOT / ".config/storage.toml").read_bytes()
    )
    subprocess.run(["git", "init", "--quiet", str(repo)], check=True)
    return lifecycle.Storage(root=repo)


def test_global_options_work_before_and_after_command(store):
    for arguments in (
        ["--repo", str(store.root), "--json", "plan", "--scope", "host"],
        ["plan", "--repo", str(store.root), "--scope", "host", "--json"],
    ):
        parsed = storage.parser().parse_args(arguments)
        assert parsed.repo == store.root
        assert parsed.scope == "host"
        assert parsed.json


def test_partial_apply_keeps_successful_actions_and_blocks_shared(store, monkeypatch, capsys):
    local = store.root / "scratch"
    shared = store.root.parent / "shared"
    local.mkdir()
    shared.mkdir()
    local_id = store.publish(local, "scratch", {"kind": "task", "path": str(store.root)}, "task")
    shared_id = store.publish(
        shared, "native-content", {"kind": "native", "path": str(shared)}, "task"
    )
    called = []

    def retire(object_id, **_):
        called.append(object_id)
        return {"id": object_id, "action": "retired"}

    monkeypatch.setattr(store, "retire", retire)
    monkeypatch.setattr(storage, "Storage", lambda **_: store)
    assert storage.main(["apply", local_id, shared_id, "--repo", str(store.root), "--json"]) == 75
    result = json.loads(capsys.readouterr().out)
    assert called == [local_id]
    assert [action["outcome"] for action in result["actions"]] == ["passed", "blocked"]
    assert result["outcome"] == "blocked"


def test_default_repository_plan_does_not_include_shared(store):
    path = store.root.parent / "shared"
    path.mkdir()
    object_id = store.publish(
        path, "native-content", {"kind": "native", "path": str(path)}, "current"
    )
    assert storage.scoped_plan(store, (), "repo")["dispositions"] == []
    assert storage.scoped_plan(store, (), "host")["dispositions"][0]["id"] == object_id


def test_schedule_installation_requires_explicit_host_scope(store, capsys):
    assert storage.main(["automation", "install", "--repo", str(store.root)]) == 75
    assert "--scope host" in json.loads(capsys.readouterr().out)["reason"]


@pytest.mark.parametrize("change", ["identity", "digest"])
def test_restore_rejects_changed_recorded_archive_before_dispatch(
    store, tmp_path, monkeypatch, change
):
    import compile_profile

    archive = tmp_path / "capture.tar.zst"
    archive.write_bytes(b"original archive")
    info = archive.stat()
    proof = {
        "path": str(archive),
        "replay_qualified": True,
        "replay": {"isolation": {"outcome": "passed"}},
        "physical_identity": lifecycle.physical(archive),
        "file_signature": [info.st_size, info.st_mtime_ns],
        "sha256": hashlib.sha256(archive.read_bytes()).hexdigest(),
    }
    if change == "identity":
        archive.unlink()
        archive.write_bytes(b"different archive")
    else:
        archive.write_bytes(b"changed! archive")
        proof["file_signature"] = [archive.stat().st_size, archive.stat().st_mtime_ns]
    monkeypatch.setattr(
        compile_profile, "restore_profile", lambda *_: pytest.fail("untrusted restore dispatched")
    )
    with pytest.raises(lifecycle.Blocked, match="changed"):
        storage.restore_verified(store, {"archive": proof}, tmp_path / "restored")


def test_automation_status_reports_stale_qualification_and_actual_unit_status(store, monkeypatch):
    lifecycle.durable_json(
        store.state / "acceptance.json", {"outcome": "passed", "implementation_identity": "stale"}
    )
    monkeypatch.setattr(storage, "implementation_identity", lambda: "current")

    def run(command, **_):
        assert command[:3] == ["systemctl", "--user", "show"]
        return subprocess.CompletedProcess(
            command, 0, "LoadState=loaded\nActiveState=inactive\nUnitFileState=disabled\n", ""
        )

    monkeypatch.setattr(storage.subprocess, "run", run)
    result = storage.automation(store, "status")
    assert result["qualification"] == {
        "present": True,
        "valid": False,
        "reason": (
            "SM8 matching-source qualification and independent review required; "
            "automation remains disabled"
        ),
    }
    assert result["enabled"] is False
    assert result["active"] is False
    assert all(unit["outcome"] == "passed" for unit in result["systemd"].values())


def test_inventory_uses_current_service_acquisition_vllm_and_tools_roots(
    store, tmp_path, monkeypatch
):
    monkeypatch.setenv("XDG_STATE_HOME", str(tmp_path / "xdg-state"))
    monkeypatch.delenv("LCTX_SURREAL_SERVICE_CONFIG", raising=False)
    monkeypatch.setenv("LCTX_COMPILE_PROFILE_TOOLS_ROOT", str(tmp_path / "tools"))
    paths = {row["path"] for row in storage.known_scopes(store)}
    assert str(tmp_path / "xdg-state/library-context/surrealdb") in paths
    assert str(store.root / "build/envs") in paths
    assert str(store.root / "build/sources") in paths
    assert str(store.root / "services/vllm/.venv") in paths
    assert str(tmp_path / "tools") in paths


def test_archive_destination_admitted_before_owner_callback(tmp_path, monkeypatch, capsys):
    import compile_profile

    # This test qualifies admission wiring, never reader/replay support.
    store = lifecycle.Storage()
    raw = tmp_path / "raw"
    raw.mkdir()
    destination = tmp_path / "archives"
    destination.mkdir()
    object_id = store.publish(
        raw,
        "profile-raw",
        {"kind": "profile", "root": str(store.root), "path": str(tmp_path / "owner")},
        "replay",
        requires="restorable-replay",
    )
    store.retain(object_id, "replay", "restorable-replay", ["compiler-summary"])
    monkeypatch.setattr(lifecycle, "owner_guard", lambda *_: contextlib.nullcontext())
    import compile_profile_tools

    monkeypatch.setattr(
        compile_profile_tools,
        "prepare_reader_generation",
        lambda _: {"generation_root": str(tmp_path / "readers")},
    )

    def archive(source, target, operations, **_):
        # An exclusive second admission must reuse the outer exclusive descriptor.
        with lifecycle.admission([target], exclusive=True, state=store.state, blocking=False):
            assert target == destination / (object_id + ".tar.zst")
        return {"outcome": "blocked", "reason": "reader intentionally not run by wiring control"}

    monkeypatch.setattr(compile_profile, "archive_profile", archive)
    assert storage.main(["archive", object_id, "--destination", str(destination)]) == 75
    assert "reader intentionally" in json.loads(capsys.readouterr().out)["reason"]


def test_empty_repository_sweep_does_not_sweep_shared_and_records_health(
    store, monkeypatch, capsys
):
    shared = store.root.parent / "shared"
    shared.mkdir()
    store.publish(shared, "native-content", {"kind": "native", "path": str(shared)}, "current")
    monkeypatch.setattr(storage, "Storage", lambda **_: store)
    monkeypatch.setattr(store, "sweep", lambda *_: pytest.fail("empty repo selection swept host"))
    assert storage.main(["sweep"]) == 0
    assert json.loads(capsys.readouterr().out)["actions"] == []
    assert lifecycle.read_json(store.state / "last-sweep.json")["outcome"] == "passed"


def test_missing_repository_policy_returns_structured_failure(tmp_path, capsys):
    repository = tmp_path / "missing-repository"
    assert storage.main(["plan", "--repo", str(repository)]) == 1
    result = json.loads(capsys.readouterr().out)
    assert result["outcome"] == "failed"
    assert result["operation"] == "plan"
    assert result["policy_revision"] is None
    assert result["scope"]["repository"] == str(repository)
