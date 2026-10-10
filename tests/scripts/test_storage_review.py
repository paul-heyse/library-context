"""Independent regressions for retirement recovery, citations and declared scope ancestry."""

from __future__ import annotations

import subprocess

import pytest

import storage_lifecycle as lifecycle
from harness import ProcessIdentity, write_json_atomic


@pytest.fixture
def storage(tmp_path, monkeypatch):
    root = tmp_path / "repo"
    (root / ".config").mkdir(parents=True)
    (root / ".config/storage.toml").write_text(
        'schema=1\n[categories.scratch]\nowner="task"\ngrace_days=0\n'
        '[categories.run-receipt]\nowner="run"\ngrace_days=0\n'
    )
    subprocess.run(["git", "init", "-q", str(root)], check=True)
    (root / ".gitignore").write_text("/build/\n")
    monkeypatch.setenv("LCTX_STORAGE_STATE", str(tmp_path / "state"))
    monkeypatch.setenv("LCTX_STORAGE_CONFIG", str(tmp_path / "absent-host.toml"))
    return lifecycle.Storage(root=root)


def task(storage, path, *, parent=None):
    path.mkdir(parents=True, exist_ok=True)
    (path / "output").write_text("output")
    ident = storage.publish(
        path,
        "scratch",
        {
            "kind": "task",
            "path": str(storage.root.parent / "removed-task"),
            "reference": "review-task-lifetime",
        },
        "review-consumer",
        managed=True,
        parent=parent,
    )
    storage.release(ident, "review-consumer", "consumer completed")
    return ident


def test_deleted_run_owner_can_finalize_interrupted_retirement(storage, monkeypatch):
    path = storage.root / "build/run-output"
    path.mkdir(parents=True)
    (path / "owner.lock").touch()
    write_json_atomic(
        path / "record.json",
        {
            "schema": 2,
            "id": path.name,
            "owner": ProcessIdentity.of().to_json(),
            "child": None,
            "termination": "completed",
            "ended": lifecycle.now(),
            "cleanup": {"status": "confirmed", "observed": lifecycle.now()},
        },
    )
    ident = storage.publish(
        path, "run-receipt", {"kind": "run", "path": str(path)}, "review-consumer", managed=True
    )
    storage.release(ident, "review-consumer", "receipt consumer completed")
    original = lifecycle._delete

    def crash_after_deletion(*args):
        original(*args)
        raise RuntimeError("simulated crash after actual deletion")

    with monkeypatch.context() as patch:
        patch.setattr(lifecycle, "_delete", crash_after_deletion)
        with pytest.raises(RuntimeError, match="after actual deletion"):
            storage.retire(ident)
    assert not path.exists()
    assert not (path.parent / f".lctx-retired-{ident}").exists()
    assert storage.get(ident).get("retirement")
    assert storage.retire(ident)["action"] == "retired"
    assert storage.get(ident)["retired_at"]
    assert not (storage.state / "journals" / f"{ident}.json").exists()


def test_tracked_relative_short_component_citation_protects(storage):
    path = storage.root / "build/task123456789/logs"
    ident = task(storage, path)
    (storage.root / "README.md").write_text("Keep [capture logs](build/task123456789/logs).\n")
    subprocess.run(["git", "-C", str(storage.root), "add", "README.md"], check=True)
    decision = storage.plan([ident])["dispositions"][0]
    assert decision["disposition"] != "eligible"
    assert any("reference" in reason or "citation" in reason for reason in decision["reasons"])
    with pytest.raises(lifecycle.Blocked):
        storage.retire(ident)
    assert (path / "output").is_file()


def test_declared_three_level_ancestry_allows_released_leaf(storage):
    grandparent = task(storage, storage.root / "build/outputs")
    parent = task(storage, storage.root / "build/outputs/task", parent=grandparent)
    leaf = task(storage, storage.root / "build/outputs/task/logs", parent=parent)
    decision = storage.plan([leaf])["dispositions"][0]
    assert decision["disposition"] == "eligible", decision["reasons"]
    assert storage.retire(leaf)["action"] == "retired"
    assert (storage.root / "build/outputs/task/output").is_file()
