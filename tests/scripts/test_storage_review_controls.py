"""Independent regressions for destructive boundaries and qualification dependencies."""

import os
import re
from pathlib import Path
from types import SimpleNamespace

import pytest

import storage
import storage_lifecycle as lifecycle
from harness import ProcessIdentity, hold_lock, write_json_atomic


def test_preflight_blocks_same_device_mount_before_touching_content(tmp_path, monkeypatch):
    root = tmp_path / "owned"
    mounted = root / "mounted"
    mounted.mkdir(parents=True)
    external_content = mounted / "keep"
    external_content.write_text("external mounted content")
    assert root.stat().st_dev == mounted.stat().st_dev
    monkeypatch.setattr(lifecycle, "_mountpoints", lambda: {mounted})
    with pytest.raises(lifecycle.Blocked, match="mount boundary"):
        lifecycle._preflight(root, root.stat().st_dev)
    assert external_content.read_text() == "external mounted content"


def test_fd_deletion_checks_mount_identity_before_entering_child(tmp_path, monkeypatch):
    root = tmp_path / "owned"
    mounted = root / "mounted"
    mounted.mkdir(parents=True)
    external_content = mounted / "keep"
    external_content.write_text("external mounted content")
    read_text = Path.read_text

    def changed_mount_id(path, *args, **kwargs):
        text = read_text(path, *args, **kwargs)
        if str(path).startswith("/proc/self/fdinfo/"):
            descriptor = int(path.name)
            if os.readlink(f"/proc/self/fd/{descriptor}") == str(mounted):
                return re.sub(r"(?m)^mnt_id:\s*\d+", "mnt_id:\t999999999", text)
        return text

    monkeypatch.setattr(Path, "read_text", changed_mount_id)
    with pytest.raises(lifecycle.Blocked, match="mount boundary"):
        lifecycle._delete(root, root.stat().st_dev)
    assert external_content.read_text() == "external mounted content"


def test_mountinfo_paths_decode_escaped_spaces(tmp_path, monkeypatch):
    mounted = tmp_path / "mount with spaces"
    read_text = Path.read_text

    def mount_table(path, *args, **kwargs):
        if path == Path("/proc/self/mountinfo"):
            escaped = str(mounted).replace(" ", "\\040")
            return f"101 100 0:1 / {escaped} rw - tmpfs tmpfs rw\n"
        return read_text(path, *args, **kwargs)

    monkeypatch.setattr(Path, "read_text", mount_table)
    assert mounted in lifecycle._mountpoints()


@pytest.mark.parametrize(
    "name", ["docs.py", "compile_profile_capture.py", "compile_profile_tools.py"]
)
def test_implementation_identity_tracks_all_producer_and_reader_sources(
    tmp_path, monkeypatch, name
):
    root = tmp_path / "repository"
    (root / "scripts").mkdir(parents=True)
    (root / ".config").mkdir()
    (root / ".config/storage.toml").write_text("schema=1\n")
    for relative in (
        "justfile",
        ".python-version",
        "tools/compile-profile/measureme.Cargo.lock",
        "tools/compile-profile/measureme-nightly-cpuid.patch",
    ):
        path = root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text("qualified")
    producer = root / "scripts" / name
    producer.write_text("qualified")
    monkeypatch.setattr(storage, "ROOT", root)
    monkeypatch.setattr(storage, "Storage", lambda: SimpleNamespace(host={"repositories": []}))
    before = storage.implementation_identity()
    producer.write_text("changed lifetime or replay behavior")
    assert storage.implementation_identity() != before


@pytest.fixture
def legacy_store(tmp_path, monkeypatch):
    root = tmp_path / "repository"
    (root / ".config").mkdir(parents=True)
    (root / ".config/storage.toml").write_text(
        'schema=1\n[categories.run-receipt]\nowner="run"\n'
        '[categories.profile-raw]\nowner="profile"\n'
        '[categories.profile-report]\nowner="profile"\n'
        '[categories.attachment]\nowner="attachment"\n'
        '[categories.docs-output]\nowner="docs"\n'
        '[categories.task-build]\nowner="task"\n'
    )
    monkeypatch.setenv("LCTX_STORAGE_STATE", str(tmp_path / "state"))
    monkeypatch.setenv("LCTX_STORAGE_CONFIG", str(tmp_path / "absent-host.toml"))
    monkeypatch.delenv(lifecycle.ADMISSION_ENV, raising=False)
    return lifecycle.Storage(root=root)


def terminal_run(path):
    path.mkdir(parents=True)
    (path / "owner.lock").touch()
    write_json_atomic(
        path / "record.json",
        {
            "schema": 2,
            "id": path.name,
            "owner": ProcessIdentity.of().to_json(),
            "termination": "completed",
            "cleanup": {"status": "confirmed", "observed": lifecycle.now()},
            "ended": lifecycle.now(),
        },
    )


def protected_legacy(store, path, kind, category, owner):
    return store.publish(
        path,
        category,
        {"kind": kind, "path": str(owner)},
        "legacy-unresolved",
        requires="external-owner",
        managed=False,
    )


def test_manage_exact_run_keeps_legacy_hold_and_never_silently_promotes(legacy_store):
    path = legacy_store.root / "run"
    terminal_run(path)
    ident = protected_legacy(legacy_store, path, "run", "run-receipt", path)
    legacy_store.publish(
        path, "run-receipt", {"kind": "run", "path": str(path)}, "producer", managed=True
    )
    assert legacy_store.get(ident)["managed"] is False
    before = legacy_store.get(ident)["obligations"]
    row = legacy_store.manage_existing(ident)
    assert row["managed"] and row["exposure"] == "managed" and row["adopted_at"]
    assert row["obligations"] == before
    assert lifecycle._nonce(path) == row["nonce"]
    assert legacy_store.disposition(row, references=[])["disposition"] == "retained"


def test_manage_rejects_arbitrary_child_and_unresolved_cleanup(legacy_store):
    owner = legacy_store.root / "run"
    terminal_run(owner)
    child = owner / "arbitrary"
    child.mkdir()
    ident = protected_legacy(legacy_store, child, "run", "run-receipt", owner)
    with pytest.raises(lifecycle.Blocked, match="exact disposable scope"):
        legacy_store.manage_existing(ident)
    ident = protected_legacy(legacy_store, owner, "run", "run-receipt", owner)
    row = lifecycle.read_json(owner / "record.json")
    row["cleanup"] = {"status": "unknown"}
    write_json_atomic(owner / "record.json", row)
    with pytest.raises(lifecycle.Blocked, match="confirmed cleanup"):
        legacy_store.manage_existing(ident)
    assert not legacy_store.get(ident)["managed"]


def test_manage_external_profile_requires_authoritative_location_identity(legacy_store, tmp_path):
    owner = legacy_store.root / "profile-run"
    terminal_run(owner)
    raw = tmp_path / "external-capture"
    raw.mkdir()
    info = raw.stat()
    write_json_atomic(
        owner / "compile-profile-location.json",
        {
            "schema": 1,
            "path": str(raw),
            "device": info.st_dev,
            "inode": info.st_ino,
        },
    )
    ident = protected_legacy(legacy_store, raw, "profile", "profile-raw", owner)
    assert legacy_store.manage_existing(ident)["managed"]
    other = tmp_path / "unbound-capture"
    other.mkdir()
    other_id = protected_legacy(legacy_store, other, "profile", "profile-raw", owner)
    with pytest.raises(lifecycle.Blocked, match="exact disposable scope"):
        legacy_store.manage_existing(other_id)


@pytest.mark.parametrize("kind,category", [("attachment", "attachment"), ("docs", "docs-output")])
def test_manage_fixed_attachment_and_documentation_scopes(legacy_store, kind, category):
    path = legacy_store.root / "output"
    path.mkdir()
    owner = path if kind == "attachment" else legacy_store.root / "docs-owner"
    owner.mkdir(exist_ok=True)
    (owner / "owner.lock").touch()
    record = (
        {"released": True}
        if kind == "attachment"
        else {
            "phase": "finished",
            "cleanup": {"status": "confirmed"},
            "path": str(path),
        }
    )
    write_json_atomic(owner / "record.json", record)
    ident = protected_legacy(legacy_store, path, kind, category, owner)
    assert legacy_store.manage_existing(ident)["managed"]


def test_manage_refuses_missing_task_owner_and_replaced_identity(legacy_store):
    path = legacy_store.root / "task-output"
    path.mkdir()
    ident = protected_legacy(
        legacy_store, path, "task", "task-build", legacy_store.root / "absent-task"
    )
    with pytest.raises(lifecycle.Blocked, match="no qualified"):
        legacy_store.manage_existing(ident)
    run = legacy_store.root / "run"
    terminal_run(run)
    run_id = protected_legacy(legacy_store, run, "run", "run-receipt", run)
    run.rename(run.with_name("previous-run"))
    terminal_run(run)
    with pytest.raises(lifecycle.Blocked, match="physical identity"):
        legacy_store.manage_existing(run_id)


def test_manage_cannot_promote_while_actual_owner_lock_is_held(legacy_store):
    path = legacy_store.root / "run"
    terminal_run(path)
    ident = protected_legacy(legacy_store, path, "run", "run-receipt", path)
    descriptor = hold_lock(path / "owner.lock")
    try:
        with pytest.raises(lifecycle.Blocked, match="busy"):
            legacy_store.manage_existing(ident)
    finally:
        os.close(descriptor)
    assert legacy_store.get(ident)["managed"] is False


def test_old_tombstone_update_cannot_replace_reused_path_index(legacy_store):
    path = legacy_store.root / "run"
    terminal_run(path)
    previous = protected_legacy(legacy_store, path, "run", "run-receipt", path)
    legacy_store.manage_existing(previous)
    legacy_store.release(previous, "legacy-unresolved", "explicit owner release")
    assert legacy_store.retire(previous, references=[])["action"] == "retired"
    terminal_run(path)
    owner = {"kind": "run", "path": str(path)}
    current = legacy_store.publish(path, "run-receipt", owner, "new-consumer", managed=True)
    legacy_store.release(previous, "legacy-unresolved", "idempotent historical release")
    again = legacy_store.publish(path, "run-receipt", owner, "new-consumer", managed=True)
    assert again == current
