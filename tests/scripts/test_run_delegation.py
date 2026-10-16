"""Prospective host composition with real inert commands; no native service."""

import itertools
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


@pytest.fixture
def stop_scope(scope, monkeypatch):
    """Pure owner-issuance fixture. It never signals a real process."""
    directory, parent = scope
    identity = {**parent.record["owner"], "pid": parent.record["owner"]["pid"] + 10_000_000}
    held = set()
    monkeypatch.setattr(harness.ProcessIdentity, "alive", lambda self: True)
    monkeypatch.setattr(delegated, "lock_held", lambda path: path.parent.name in held)
    clock = [100.0]
    monkeypatch.setattr(delegated.time, "monotonic", lambda: clock[0])
    signals = []
    monkeypatch.setattr(
        delegated, "_signal_owner", lambda owner, lock, signum: signals.append(signum) or True
    )

    def add(name, *, live=True, kind="ordinary-writer", child=None, owner=None):
        path = directory / delegated.ENTRIES / name
        path.mkdir()
        row = {
            "schema": 1,
            "id": name,
            "run": directory.name,
            "run_owner": parent.record["owner"],
            "owner": owner or identity,
            "kind": kind,
            "child": child,
            "spawning": bool(child),
            "ended": None if live else "original-completed",
            "cleanup": {"status": "unknown" if live else "confirmed"},
        }
        harness.write_json_atomic(path / "record.json", row)
        if live:
            held.add(name)
        return path

    return SimpleNamespace(
        directory=directory,
        record=parent.record,
        add=add,
        clock=clock,
        signals=signals,
        held=held,
        identity=identity,
    )


@pytest.mark.parametrize("order", sorted(set(itertools.permutations((False, True, True)))))
def test_stop_groups_completed_and_live_owner_independently_of_order(stop_scope, order):
    scope = stop_scope
    paths = [scope.add(str(index), live=live) for index, live in enumerate(order)]
    originals = [path.joinpath("record.json").read_bytes() for path in paths]
    assert delegated.request_stop(scope.directory, scope.record)["status"] == "active"
    assert scope.signals == [signal.SIGTERM]
    scope.clock[0] = 104.0
    delegated.request_stop(scope.directory, scope.record)
    assert scope.signals == [signal.SIGTERM]
    scope.clock[0] = 110.0
    delegated.request_stop(scope.directory, scope.record)
    assert scope.signals == [signal.SIGTERM, signal.SIGKILL]
    scope.clock[0] = 120.0
    delegated.request_stop(scope.directory, scope.record)
    assert scope.signals == [signal.SIGTERM, signal.SIGKILL]
    for path, original in zip(paths, originals, strict=True):
        recovery = receipt(path / "recovery.json")
        assert recovery["owner_request"]["status"] == "sent"
        assert recovery["requested_monotonic"] == 100.0
        assert recovery["owner_signal"]["status"] == "sent"
        assert path.joinpath("record.json").read_bytes() == original


def test_completed_only_owner_creates_no_stop_request(stop_scope):
    scope = stop_scope
    path = scope.add("completed", live=False)
    assert delegated.request_stop(scope.directory, scope.record)["status"] == "confirmed"
    assert not (path / "recovery.json").exists()
    assert scope.signals == []


def test_concurrent_stop_callers_issue_once_and_preserve_sent_time(stop_scope, monkeypatch):
    scope = stop_scope
    scope.add("first")
    scope.add("second")
    entered, release = threading.Event(), threading.Event()
    second_entered = threading.Event()
    results = []

    def signal_owner(owner, lock, signum):
        scope.signals.append(signum)
        entered.set()
        assert release.wait(2)
        return True

    monkeypatch.setattr(delegated, "_signal_owner", signal_owner)
    first = threading.Thread(
        target=lambda: results.append(delegated.request_stop(scope.directory, scope.record))
    )

    def second_request():
        second_entered.set()
        results.append(delegated.request_stop(scope.directory, scope.record))

    second = threading.Thread(target=second_request)
    first.start()
    try:
        assert entered.wait(2)
        second.start()
        assert second_entered.wait(2)
    finally:
        release.set()
        first.join(3)
        if second.ident is not None:
            second.join(3)
    assert not first.is_alive() and not second.is_alive()
    assert len(results) == 2 and all(row["status"] == "active" for row in results)
    assert scope.signals == [signal.SIGTERM]


@pytest.mark.parametrize("failure_stage", ("before_signal", "after_signal", "partial_sent"))
def test_persistence_failure_retries_without_false_grace(stop_scope, monkeypatch, failure_stage):
    scope = stop_scope
    first, second = scope.add("first"), scope.add("second")
    write = delegated.write_json_atomic
    failed = []

    def fail_once(path, value):
        if path.name == "recovery.json" and not failed:
            status = value.get("owner_signal", {}).get("status")
            if (
                (failure_stage == "before_signal" and status == "attempted")
                or (failure_stage == "after_signal" and status == "sent")
                or (failure_stage == "partial_sent" and path.parent == second and status == "sent")
            ):
                failed.append(True)
                raise OSError("selected receipt failure")
        return write(path, value)

    monkeypatch.setattr(delegated, "write_json_atomic", fail_once)
    observed = delegated.request_stop(scope.directory, scope.record)
    assert observed["status"] == "active" and "receipt failure" in observed["reason"]
    assert scope.signals == ([] if failure_stage == "before_signal" else [signal.SIGTERM])
    scope.clock[0] = 110.0
    delegated.request_stop(scope.directory, scope.record)
    if failure_stage == "partial_sent":
        assert scope.signals == [signal.SIGTERM, signal.SIGKILL]
        expected_time = 100.0
    else:
        assert scope.signals[-1] == signal.SIGTERM
        expected_time = 110.0
    for path in (first, second):
        assert receipt(path / "recovery.json")["requested_monotonic"] == expected_time


@pytest.mark.parametrize("signal_failure", ("raises", "not_sent"))
def test_failed_signal_does_not_start_grace_and_remains_retryable(
    stop_scope, monkeypatch, signal_failure
):
    scope = stop_scope
    path = scope.add("live")

    def fail(owner, lock, signum):
        if signal_failure == "raises":
            raise PermissionError("signal denied")
        return False

    monkeypatch.setattr(delegated, "_signal_owner", fail)
    observed = delegated.request_stop(scope.directory, scope.record)
    assert observed["status"] == "active"
    recovery = receipt(path / "recovery.json")
    assert "owner_request" not in recovery and "requested_monotonic" not in recovery
    expected_signal = "failed" if signal_failure == "raises" else "not_sent"
    assert recovery["owner_signal"]["status"] == expected_signal
    scope.clock[0] = 120.0
    monkeypatch.setattr(
        delegated,
        "_signal_owner",
        lambda owner, lock, signum: scope.signals.append(signum) or True,
    )
    delegated.request_stop(scope.directory, scope.record)
    assert scope.signals == [signal.SIGTERM]
    assert receipt(path / "recovery.json")["requested_monotonic"] == 120.0


def test_unproven_timestamp_cannot_suppress_initial_request(stop_scope):
    scope = stop_scope
    path = scope.add("live")
    harness.write_json_atomic(
        path / "recovery.json", {"requested": "unproven", "requested_monotonic": 1.0}
    )
    delegated.request_stop(scope.directory, scope.record)
    assert scope.signals == [signal.SIGTERM]
    assert receipt(path / "recovery.json")["requested_monotonic"] == 100.0


def test_observer_in_same_owner_group_protects_against_escalation(stop_scope):
    scope = stop_scope
    scope.add("ordinary")
    scope.add("observer", kind="profile-observer")
    delegated.request_stop(scope.directory, scope.record)
    scope.clock[0] = 120.0
    delegated.request_stop(scope.directory, scope.record)
    assert scope.signals == [signal.SIGTERM]


def test_foreign_and_calling_owners_are_not_signalled(stop_scope):
    scope = stop_scope
    scope.add("self", owner={**harness.ProcessIdentity.of().to_json(), "extension": "retained"})
    scope.add("foreign", owner={**scope.identity, "pid_namespace": -1})
    delegated.request_stop(scope.directory, scope.record)
    assert scope.signals == []


def test_failed_owner_does_not_starve_other_live_owner(stop_scope, monkeypatch):
    scope = stop_scope
    denied = scope.add("a-denied")
    other = scope.add("b-other", owner={**scope.identity, "pid": scope.identity["pid"] + 1})

    def send(owner, lock, signum):
        if lock.parent == denied:
            raise PermissionError("first owner denied")
        scope.signals.append((lock.parent, signum))
        return True

    monkeypatch.setattr(delegated, "_signal_owner", send)
    observed = delegated.request_stop(scope.directory, scope.record)
    assert observed["status"] == "active" and "denied" in observed["reason"]
    assert scope.signals == [(other, signal.SIGTERM)]
    assert "owner_request" not in receipt(denied / "recovery.json")


def test_failed_escalation_retries_with_original_grace(stop_scope, monkeypatch):
    scope = stop_scope
    path = scope.add("live")
    delegated.request_stop(scope.directory, scope.record)
    scope.clock[0] = 120.0
    monkeypatch.setattr(delegated, "_signal_owner", lambda *args: False)
    delegated.request_stop(scope.directory, scope.record)
    recovery = receipt(path / "recovery.json")
    assert recovery["requested_monotonic"] == 100.0
    assert "owner_escalated" not in recovery
    monkeypatch.setattr(
        delegated,
        "_signal_owner",
        lambda owner, lock, signum: scope.signals.append(signum) or True,
    )
    delegated.request_stop(scope.directory, scope.record)
    assert scope.signals == [signal.SIGTERM, signal.SIGKILL]
    assert receipt(path / "recovery.json")["requested_monotonic"] == 100.0


def test_finalizing_independent_observer_protects_other_owner(stop_scope, monkeypatch):
    scope = stop_scope
    scope.add("ordinary")
    observer_identity = {**scope.identity, "pid": scope.identity["pid"] + 1}
    observer = scope.add(
        "observer",
        live=False,
        kind="profile-observer",
        child=observer_identity,
        owner=observer_identity,
    )
    monkeypatch.setattr(
        delegated,
        "recorded_group",
        lambda *args, **kwargs: {"status": "owned", "members": []},
    )
    delegated.request_stop(scope.directory, scope.record)
    scope.clock[0] = 120.0
    delegated.request_stop(scope.directory, scope.record)
    assert scope.signals == [signal.SIGTERM]
    assert not (observer / "recovery.json").exists()


def test_dead_child_cleanup_does_not_hold_admission_gate(scope, monkeypatch):
    directory, parent = scope
    path = directory / delegated.ENTRIES / "dead"
    path.mkdir()
    dead = {**parent.record["owner"], "start_ticks": -1}
    harness.write_json_atomic(
        path / "record.json",
        {
            "schema": 1,
            "id": path.name,
            "run": directory.name,
            "run_owner": parent.record["owner"],
            "owner": dead,
            "kind": "ordinary-writer",
            "child": {**dead, "pgid": dead["pid"]},
            "spawning": True,
            "ended": None,
            "cleanup": {"status": "unknown"},
        },
    )
    monkeypatch.setattr(
        delegated,
        "recorded_group",
        lambda *args, **kwargs: {"status": "owned", "members": []},
    )
    entered, release = threading.Event(), threading.Event()
    result = []

    def cleanup(*args, **kwargs):
        entered.set()
        assert release.wait(2)
        return {"status": "confirmed", "survivors": []}

    monkeypatch.setattr(delegated, "cleanup_group", cleanup)
    thread = threading.Thread(
        target=lambda: result.append(delegated.request_stop(directory, parent.record))
    )
    thread.start()
    try:
        assert entered.wait(2)
        gate = harness.try_hold_lock(directory / "delegation.lock")
        assert gate is not None
        os.close(gate)
        # Another caller observes the child still draining without blocking on
        # its recovery lock, and without fabricating a completed original.
        assert delegated.request_stop(directory, parent.record)["status"] == "unknown"
    finally:
        release.set()
        thread.join(3)
    assert not thread.is_alive() and len(result) == 1
    assert receipt(path / "record.json")["cleanup"]["status"] == "unknown"
    assert receipt(path / "recovery.json")["cleanup"]["status"] == "confirmed"


def test_wholly_owned_host_completed_registration_cannot_suppress_cancel(scope, tmp_path):
    directory, owner = scope
    ready = tmp_path / "completed-first-ready"
    script = r"""
import os,pathlib,signal,sys,time
sys.path.insert(0,sys.argv[1])
from run_delegation import register
signal.signal(signal.SIGTERM,signal.SIG_IGN)
first=register('completed-setup')
first.finish({'status':'confirmed','survivors':[]})
active=register('active-writer')
# Force completed-first enumeration independently of UUID ordering.
import json
for registration,name in ((first,'a-completed'),(active,'b-active')):
 destination=registration.directory.parent/name
 registration.directory.rename(destination)
 p=destination/'record.json'
 r=json.loads(p.read_text());r['id']=name;p.write_text(json.dumps(r))
pathlib.Path(sys.argv[2]).touch()
while True:time.sleep(.01)
"""
    child = subprocess.Popen(
        [sys.executable, "-c", script, str(Path(runs.__file__).parent), str(ready)]
    )
    try:
        deadline = time.monotonic() + 10
        while not ready.exists() and child.poll() is None and time.monotonic() < deadline:
            time.sleep(0.01)
        assert ready.exists()
        identity = harness.ProcessIdentity.of(child.pid)
        assert delegated.request_stop(directory, owner.record, grace=0.05)["status"] == "active"
        deadline = time.monotonic() + 5
        while identity.alive() and time.monotonic() < deadline:
            delegated.request_stop(directory, owner.record, grace=0.05)
            time.sleep(0.01)
        child.wait(timeout=2)
        assert child.returncode == -signal.SIGKILL
        assert not identity.alive()
        assert delegated.request_stop(directory, owner.record, grace=0.05)["status"] == "confirmed"
        assert runs.host_cleanup(directory)["status"] == "confirmed"
        assert receipt(directory / delegated.ENTRIES / "a-completed" / "record.json")["ended"]
    finally:
        if child.poll() is None:
            # Popen owns this exact unreaped process; never signal a numeric historical PID.
            child.kill()
        child.wait(timeout=2)
        delegated.request_stop(directory, owner.record, grace=0.05)


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


def test_listing_continues_anchored_membership_and_preserves_corrupt_receipt(tmp_path, monkeypatch):
    root = history(tmp_path, monkeypatch)
    first = run_listing.page(options())
    corrupt = root / "20260101T000000.999Z-bbbbbb"
    corrupt.mkdir()
    (corrupt / "record.json").write_text("broken")
    second = run_listing.page(options(cursor=first["next_cursor"]))
    assert second["coverage"]["complete"]
    assert corrupt.name not in {row["id"] for row in second["runs"]}
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


def test_live_cursor_survives_deleted_anchor_and_rejects_query_or_root_change(
    tmp_path, monkeypatch
):
    root = history(tmp_path, monkeypatch)
    first = run_listing.page(options(limit=2))
    anchor = first["runs"][-1]["id"]
    (root / anchor / "record.json").unlink()
    (root / anchor).rmdir()
    # A backdated insertion in the remaining interval is observable; it does
    # not cause repetition of rows already returned above the last examined name.
    inserted = root / "20260101T000000.028Z-000000"
    inserted.mkdir()
    second = run_listing.page(options(cursor=first["next_cursor"], limit=40))
    assert inserted.name in {row["id"] for row in second["runs"]}
    assert not {row["id"] for row in first["runs"]} & {row["id"] for row in second["runs"]}
    with pytest.raises(ValueError, match="selection"):
        run_listing.page(options(cursor=first["next_cursor"], label="changed"))
    replacement = tmp_path / "replacement"
    replacement.mkdir()
    monkeypatch.setenv("LCTX_RUNS_ROOT", str(replacement))
    with pytest.raises(ValueError, match="root"):
        run_listing.page(options(cursor=first["next_cursor"]))
    import base64

    obsolete = base64.urlsafe_b64encode(json.dumps({"schema": 1}).encode()).decode()
    with pytest.raises(ValueError, match="obsolete"):
        run_listing.page(options(cursor=obsolete))
