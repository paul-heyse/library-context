"""Prospective host composition with real inert commands; no native service."""

import json
import os
import signal
import subprocess
import sys
import threading
import time
from pathlib import Path
from types import SimpleNamespace

import pytest

import harness
import run_delegation as delegated
import run_listing
import runs


def receipt(path: Path) -> dict:
    row = harness.read_json(path)
    assert isinstance(row, dict)
    return row


@pytest.fixture
def scope(tmp_path, monkeypatch):
    monkeypatch.setenv("LCTX_RUNS_ROOT", str(tmp_path / "runs"))
    directory = runs.new_run_dir()
    owner = runs.Owner(
        directory, ["inert"], label="parent", cwd=str(tmp_path), mode="foreground", stream=False
    )
    owner.record["delegation"] = delegated.initialize(directory, owner.record["owner"])
    owner.record["cleanup"] = {"status": "confirmed", "survivors": [], "observed": runs.now()}
    owner.write()
    descriptor = harness.hold_lock(directory / runs.OWNER_LOCK)
    monkeypatch.setenv("LCTX_RUN_DIR", str(directory))
    monkeypatch.setenv("LCTX_RUN_ID", directory.name)
    try:
        yield directory, owner
    finally:
        os.close(descriptor)


def test_writer_blocks_host_release_and_closed_admission_prevents_spawn(scope, monkeypatch):
    directory, owner = scope
    with delegated.writer_owner("capture", {"invocation": "a" * 32}):
        assert runs.cleanup_of(owner.record)["status"] == "confirmed"
        assert runs.host_cleanup(directory)["status"] == "unknown"
        assert runs.cleanup_protected(owner.record)
    assert runs.host_cleanup(directory)["status"] == "confirmed"
    delegated.close_admission(directory, owner.record)
    monkeypatch.setattr(
        subprocess, "Popen", lambda *a, **k: pytest.fail("native launch after closed admission")
    )
    with pytest.raises(InterruptedError, match="no longer admits"):
        harness.spawn_group(["never-launched"])
    owner.record["termination"] = None
    owner.record["owner"] = {**owner.record["owner"], "start_ticks": -1}
    owner.write()
    with pytest.raises(InterruptedError, match="no longer admits"):
        harness.spawn_group(["never-launched"])
    monkeypatch.setenv("LCTX_RUN_DIR", str(directory / "missing"))
    with pytest.raises(ValueError, match="receipt unavailable"):
        harness.spawn_group(["never-launched"])


def test_group_registration_precedes_exposure_and_guard_keeps_original_receipt(scope):
    directory, owner = scope
    child = harness.spawn_group([sys.executable, "-c", "import time;time.sleep(30)"])
    with harness.SpawnGuard(child, grace=0.1) as guard:
        registrations = list((directory / delegated.ENTRIES).iterdir())
        assert len(registrations) == 1
        row = receipt(registrations[0] / "record.json")
        assert row["child"]["pid"] == child.pid
        assert row["run_owner"] == owner.record["owner"]
        assert row["cleanup"]["status"] == "unknown"
        assert runs.host_cleanup(directory)["status"] == "unknown"
    assert guard.cleanup["status"] == "confirmed"
    assert receipt(registrations[0] / "record.json")["cleanup"] == guard.cleanup
    assert runs.host_cleanup(directory)["status"] == "confirmed"


def test_late_or_invalid_managed_parent_refuses_before_launch(scope, monkeypatch):
    _directory, owner = scope
    monkeypatch.setattr(
        subprocess, "Popen", lambda *a, **k: pytest.fail("unadmitted native launch")
    )
    owner.record["termination"] = "completed"
    owner.write()
    with pytest.raises(InterruptedError, match="no longer admits"):
        harness.spawn_group(["never-launched"])


def test_cancel_admission_waits_for_accepted_launch_publication(scope):
    directory, owner = scope
    registration = delegated.register("process-group", spawning=True)
    assert registration is not None
    closed = threading.Event()
    closing = threading.Thread(
        target=lambda: (delegated.close_admission(directory, owner.record), closed.set())
    )
    closing.start()
    try:
        assert not closed.wait(0.05)
        registration.finish({"status": "confirmed", "reason": "child not launched"})
        assert closed.wait(2)
        assert not receipt(directory / delegated.SCOPE)["accepting"]
    finally:
        registration.finish({"status": "confirmed", "reason": "child not launched"})
        closing.join(2)


def test_owner_signal_refuses_replacement_after_pidfd_open(monkeypatch, tmp_path):
    identity = harness.ProcessIdentity.of()
    closed = []
    monkeypatch.setattr(os, "pidfd_open", lambda pid: 123456)
    monkeypatch.setattr(harness.ProcessIdentity, "alive", lambda self: False)
    monkeypatch.setattr(delegated, "lock_held", lambda path: True)
    monkeypatch.setattr(
        signal, "pidfd_send_signal", lambda *args: pytest.fail("replacement signalled")
    )
    monkeypatch.setattr(os, "close", closed.append)
    assert not delegated._signal_owner(identity, tmp_path / "owner.lock", signal.SIGTERM)
    assert closed == [123456]


def test_selected_recovery_drains_authenticated_group_after_delegated_owner_death(scope):
    directory, owner = scope
    script = (
        "import os,sys;sys.path.insert(0,sys.argv[1]);from harness import spawn_group;"
        "spawn_group([sys.executable,'-c','import time;time.sleep(90)']);os._exit(0)"
    )
    subprocess.run([sys.executable, "-c", script, str(Path(runs.__file__).parent)], check=True)
    assert runs.host_cleanup(directory)["status"] == "unknown"
    try:
        observed = delegated.request_stop(directory, owner.record, grace=0.1)
        assert observed["status"] == "confirmed"
        path = next((directory / delegated.ENTRIES).iterdir())
        assert receipt(path / "record.json")["cleanup"]["status"] == "unknown"
        assert receipt(path / "recovery.json")["cleanup"]["status"] == "confirmed"
        assert runs.host_cleanup(directory)["status"] == "confirmed"
    finally:
        delegated.request_stop(directory, owner.record, grace=0.1)


def test_publication_failure_after_spawn_still_drains_owned_group(scope, monkeypatch):
    directory, _ = scope
    write = delegated.write_json_atomic
    exposed = []

    def fail_started(path, row):
        if path.name == "record.json" and row.get("child"):
            exposed.append(row["child"])
            raise OSError("receipt publication failure")
        return write(path, row)

    monkeypatch.setattr(delegated, "write_json_atomic", fail_started)
    with pytest.raises(OSError, match="publication"):
        harness.spawn_group([sys.executable, "-c", "import time;time.sleep(30)"])
    assert exposed
    assert all(harness.recorded_group(child)["status"] == "gone" for child in exposed)
    # Original incomplete publication is retained and cannot prove no exposure.
    assert runs.host_cleanup(directory)["status"] == "unknown"


def test_background_cancel_joins_registered_fixture_style_group_and_writer(tmp_path, monkeypatch):
    monkeypatch.setenv("LCTX_RUNS_ROOT", str(tmp_path / "runs"))
    witness = tmp_path / "writer.json"
    script = r"""
import json,os,pathlib,signal,sys,time
sys.path.insert(0,sys.argv[1])
from harness import spawn_group,SpawnGuard,observe_exit,ProcessIdentity
from run_delegation import writer_owner
stopped=[]
signal.signal(signal.SIGTERM,lambda number,frame:stopped.append(number))
with writer_owner('fixture-capture'):
 child=spawn_group([sys.executable,'-c','import time;time.sleep(90)'])
 with SpawnGuard(child,grace=0.1):
  pathlib.Path(sys.argv[2]).write_text(json.dumps(ProcessIdentity.of(child.pid).to_json()))
  while observe_exit(child) is None and not stopped:time.sleep(.01)
"""
    started = subprocess.run(
        [
            sys.executable,
            str(Path(runs.__file__)),
            "run",
            "--background",
            "--json",
            "--",
            sys.executable,
            "-c",
            script,
            str(Path(runs.__file__).parent),
            str(witness),
        ],
        capture_output=True,
        text=True,
        check=True,
    )
    ident = json.loads(started.stdout)["id"]
    directory = runs.resolve(ident)
    try:
        deadline = time.monotonic() + 10
        while not witness.exists() and time.monotonic() < deadline:
            time.sleep(0.02)
        assert witness.exists()
        nested = harness.ProcessIdentity.from_json(json.loads(witness.read_text()))
        assert nested.alive()
        result = subprocess.run(
            [sys.executable, str(Path(runs.__file__)), "cancel", ident, "--json"],
            capture_output=True,
            text=True,
            timeout=30,
            check=True,
        )
        observed = json.loads(result.stdout)
        assert observed["cleanup"]["status"] == "confirmed"
        assert observed["host_cleanup"]["status"] == "confirmed"
        assert not nested.alive()
        assert not observed["protected"]
    finally:
        runs.cancel(directory, grace=0.1)


def test_cancel_escalates_ordinary_owner_that_ignores_term(tmp_path, monkeypatch):
    monkeypatch.setenv("LCTX_RUNS_ROOT", str(tmp_path / "runs"))
    ready = tmp_path / "ready"
    script = r"""
import pathlib,signal,sys,time
sys.path.insert(0,sys.argv[1])
from run_delegation import writer_owner
signal.signal(signal.SIGTERM,signal.SIG_IGN)
with writer_owner('ordinary-writer'),writer_owner('second-writer'):
 pathlib.Path(sys.argv[2]).touch()
 while True:time.sleep(.01)
"""
    launched = subprocess.run(
        [
            sys.executable,
            str(Path(runs.__file__)),
            "run",
            "--background",
            "--json",
            "--",
            sys.executable,
            "-c",
            script,
            str(Path(runs.__file__).parent),
            str(ready),
        ],
        capture_output=True,
        text=True,
        check=True,
    )
    directory = runs.resolve(json.loads(launched.stdout)["id"])
    try:
        deadline = time.monotonic() + 10
        while not ready.exists() and time.monotonic() < deadline:
            time.sleep(0.01)
        assert ready.exists()
        result = subprocess.run(
            [sys.executable, str(Path(runs.__file__)), "cancel", directory.name, "--json"],
            capture_output=True,
            text=True,
            timeout=20,
            check=True,
        )
        assert json.loads(result.stdout)["host_cleanup"]["status"] == "confirmed"
        receipts = [
            receipt(path / "recovery.json") for path in (directory / delegated.ENTRIES).iterdir()
        ]
        assert len({row["requested_monotonic"] for row in receipts}) == 1
        assert any(row.get("owner_escalated") for row in receipts)
    finally:
        runs.cancel(directory, grace=0.1)


def options(**overrides):
    return SimpleNamespace(
        **dict(limit=20, all=False, cursor=None, label=None, state=None) | overrides
    )


def history(tmp_path, monkeypatch, count=31):
    root = tmp_path / "runs"
    root.mkdir()
    monkeypatch.setenv("LCTX_RUNS_ROOT", str(root))
    for number in range(count):
        directory = root / f"20260101T000000.{number:03}Z-aaaaaa"
        directory.mkdir()
        harness.write_json_atomic(
            directory / "record.json",
            {
                "id": directory.name,
                "label": f"run-{number}",
                "termination": "completed",
                "cleanup": {"status": "confirmed"},
                "ended": "2026-01-01T00:00:00+00:00",
                "owner": {**harness.ProcessIdentity.of().to_json(), "boot_id": "previous"},
            },
        )
    return root


def test_recent_listing_hydrates_only_page_and_uses_one_snapshot(tmp_path, monkeypatch):
    history(tmp_path, monkeypatch)
    view, snapshot = runs.view, harness.ProcessSnapshot.capture
    opened, scans = [], []

    def observe(path, **kwargs):
        opened.append(path.name)
        return view(path, **kwargs)

    def capture():
        scans.append(True)
        return snapshot()

    monkeypatch.setattr(runs, "view", observe)
    monkeypatch.setattr(harness.ProcessSnapshot, "capture", capture)
    first = run_listing.page(options())
    assert len(first["runs"]) == 20
    assert len(opened) == 20 and len(scans) == 1
    assert first["next_cursor"] and not first["coverage"]["complete"]
    second = run_listing.page(
        SimpleNamespace(limit=20, all=False, cursor=first["next_cursor"], label=None, state=None)
    )
    assert len(second["runs"]) == 11 and second["coverage"]["complete"]
    assert not set(row["id"] for row in first["runs"]) & set(row["id"] for row in second["runs"])


def test_listing_invalidates_changed_membership_and_preserves_corrupt_receipt(
    tmp_path, monkeypatch
):
    root = history(tmp_path, monkeypatch)
    first = run_listing.page(options())
    corrupt = root / "20260101T000000.999Z-bbbbbb"
    corrupt.mkdir()
    (corrupt / "record.json").write_text("broken")
    with pytest.raises(ValueError, match="stale"):
        run_listing.page(
            SimpleNamespace(
                limit=20, all=False, cursor=first["next_cursor"], label=None, state=None
            )
        )
    whole = run_listing.page(
        SimpleNamespace(limit=20, all=True, cursor=None, label=None, state=None)
    )
    assert whole["coverage"]["complete"] and whole["next_cursor"] is None
    assert len(whole["runs"]) == 32
    assert whole["runs"][0]["corrupt"] and whole["runs"][0]["protected"]
    active = run_listing.page(options(all=True, state="active"))
    assert [row["id"] for row in active["runs"]] == [corrupt.name]
    assert active["coverage"]["unknown_records"] == 1
    (corrupt / "record.json").write_text("{}")
    labelled = run_listing.page(options(all=True, label="absent", state="active"))
    assert [row["id"] for row in labelled["runs"]] == [corrupt.name]
    assert labelled["runs"][0]["corrupt"] and labelled["runs"][0]["protected"]
