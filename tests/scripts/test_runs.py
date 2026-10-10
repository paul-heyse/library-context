"""Run handles: records, derived state, cancellation, interruption, pruning and nesting."""

from __future__ import annotations

import argparse
import json
import os
import signal
import subprocess
import sys
import time
from pathlib import Path

import pytest

import runs
from harness import ProcessIdentity, group_members

SCRIPT = Path(runs.__file__).resolve()
SCRIPTS = SCRIPT.parent


@pytest.fixture
def root(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> Path:
    path = tmp_path / "runs"
    monkeypatch.setenv("LCTX_RUNS_ROOT", str(path))
    monkeypatch.setenv("LCTX_STORAGE_STATE", str(tmp_path / "storage"))
    monkeypatch.delenv("LCTX_RUN_DIR", raising=False)
    monkeypatch.delenv("LCTX_RUN_ID", raising=False)
    return path


def cli(*args: str, env: dict[str, str] | None = None) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [sys.executable, str(SCRIPT), *args],
        capture_output=True,
        text=True,
        env=env,
        timeout=60,
    )


def status(ref: str) -> dict:
    result = cli("status", ref, "--json")
    assert result.returncode == 0, result.stderr
    return json.loads(result.stdout)


def record_of(ref: str) -> dict:
    record = runs.load_record(runs.resolve(ref))
    assert record is not None
    return record


def start_background(*command: str, label: str | None = None) -> str:
    extra = ["--label", label] if label else []
    result = cli("run", "--background", "--json", *extra, "--", *command)
    assert result.returncode == 0, result.stderr
    info = json.loads(result.stdout)
    assert info["started"] is True
    return info["id"]


def wait_for(predicate, timeout: float = 20.0) -> None:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if predicate():
            return
        time.sleep(0.05)
    raise AssertionError("condition not reached")


def test_foreground_preserves_failure_streams_output_and_records(root: Path) -> None:
    result = cli("run", "--label", "fails", "--", "bash", "-c", "echo out; echo err >&2; exit 7")
    assert result.returncode == 7
    assert "out" in result.stdout and "err" in result.stdout
    (run_dir,) = list(root.iterdir())
    record = json.loads((run_dir / "record.json").read_text())
    assert set(record) == {
        "schema", "id", "argv", "cwd", "label", "mode", "owner", "child", "phase",
        "current_command", "waiting_reason", "started", "ended", "exit", "termination", "logs",
        "launch_error", "supervisor_error", "cleanup",
    }  # fmt: skip
    assert record["id"] == run_dir.name and record["label"] == "fails"
    assert record["argv"] == ["bash", "-c", "echo out; echo err >&2; exit 7"]
    assert record["exit"] == {"code": 7} and record["termination"] == "completed"
    assert record["phase"] == "ended" and record["ended"] is not None
    assert record["child"]["pgid"] == record["child"]["pid"]
    assert (run_dir / "output.log").read_text() == "out\nerr\n"
    assert status(run_dir.name)["state"] == "completed"


def test_child_signal_is_preserved(root: Path) -> None:
    result = cli("run", "--", "bash", "-c", "kill -TERM $$")
    assert result.returncode == 128 + signal.SIGTERM
    record = record_of("last")
    assert record["exit"] == {"signal": int(signal.SIGTERM), "name": "SIGTERM"}


def test_missing_command_is_a_recorded_launch_error(root: Path) -> None:
    result = cli("run", "--", "definitely-not-a-command-xyz")
    assert result.returncode == 127
    record = record_of("last")
    assert record["exit"] is None and record["termination"] == "completed"
    assert record["cleanup"]["status"] == "confirmed"
    assert "launch_error" in record
    background = cli("run", "--background", "--", "definitely-not-a-command-xyz")
    assert background.returncode == 127


def test_background_run_is_observed_cancelled_and_others_survive(root: Path) -> None:
    first = start_background("sleep", "120", label="first")
    second = start_background("sleep", "120", label="second")
    observed = status(first)
    assert observed["state"] == "running" and observed["phase"] == "running"
    pgid = observed["child"]["pgid"]
    assert group_members(pgid)

    result = cli("cancel", first, "--json")
    assert result.returncode == 0, result.stderr
    cancelled = json.loads(result.stdout)
    assert cancelled["state"] == "cancelled" and cancelled["termination"] == "cancelled"
    assert cancelled["exit"]["name"] == "SIGTERM"
    assert group_members(pgid) == []

    survivor = status(second)
    assert survivor["state"] == "running" and group_members(survivor["child"]["pgid"])
    assert cli("cancel", second).returncode == 0
    assert status(second)["state"] == "cancelled"


def test_killed_background_owner_takes_its_command_down(root: Path) -> None:
    """The detached owner ties its command by a parent-death signal: no orphaned work."""
    ref = start_background("sleep", "120")
    record = status(ref)
    owner, pgid = record["owner"]["pid"], record["child"]["pgid"]
    os.kill(owner, signal.SIGKILL)
    wait_for(lambda: not ProcessIdentity.from_json(record["owner"]).alive())
    wait_for(lambda: group_members(pgid) == [])
    interrupted = status(ref)
    assert interrupted["state"] == "interrupted" and interrupted["termination"] is None
    assert runs.resolve(ref) not in runs.prune_candidates(0, None)


def test_killed_foreground_launcher_takes_its_command_down(root: Path) -> None:
    """A tool timeout SIGKILLs the foreground launcher; the command (verify, nextest…) must not
    keep running in its own session."""
    launcher = subprocess.Popen(
        [sys.executable, str(SCRIPT), "run", "--label", "fg", "--", "sleep", "120"],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )

    def started() -> bool:
        try:
            return (runs.load_record(runs.resolve("fg")) or {}).get("child") is not None
        except runs.RunNotFound:
            return False

    try:
        wait_for(started)
        pgid = record_of("fg")["child"]["pgid"]
        assert group_members(pgid)
    finally:
        launcher.kill()
        launcher.wait()
    wait_for(lambda: group_members(pgid) == [])
    assert status("fg")["state"] == "interrupted"


def test_survivors_that_ignore_termination_are_discoverable_and_cancellable(root: Path) -> None:
    ref = start_background("sh", "-c", "trap '' TERM; sleep 120")
    record = status(ref)
    owner, pgid = record["owner"]["pid"], record["child"]["pgid"]
    os.kill(owner, signal.SIGKILL)
    wait_for(lambda: status(ref)["state"] == "interrupted")
    interrupted = status(ref)
    assert interrupted["survivors"] == group_members(pgid) != []
    finalized = json.loads(cli("cancel", ref, "--grace", "1", "--json").stdout)
    assert finalized["termination"] == "cancelled"
    assert finalized["cancel"]["owner_dead"] is True
    assert group_members(pgid) == []


def test_interrupted_run_without_survivors_finalizes_as_interrupted(root: Path) -> None:
    ref = start_background("sleep", "120")
    record = status(ref)
    os.kill(record["owner"]["pid"], signal.SIGKILL)
    os.killpg(record["child"]["pgid"], signal.SIGKILL)
    wait_for(lambda: not group_members(record["child"]["pgid"]))
    wait_for(lambda: status(ref)["state"] == "interrupted")
    finalized = json.loads(cli("cancel", ref, "--json").stdout)
    assert finalized["termination"] == "interrupted"


def test_launcher_termination_stops_the_command_group(root: Path) -> None:
    ref = start_background("sleep", "120")
    record = status(ref)
    os.kill(record["owner"]["pid"], signal.SIGTERM)
    wait_for(lambda: status(ref)["termination"] is not None)
    final = status(ref)
    assert final["termination"] == "cancelled"
    assert final["cancel"]["via"] == "SIGTERM to the launcher"
    assert group_members(record["child"]["pgid"]) == []


def test_completion_terminates_leaked_group_members(root: Path) -> None:
    result = cli("run", "--", "bash", "-c", "sleep 120 & exit 0")
    assert result.returncode == 0
    record = record_of("last")
    assert record["survivors_terminated"]
    assert group_members(record["child"]["pgid"]) == []


def test_prune_removes_only_completed_unretained_runs(root: Path) -> None:
    for _ in range(3):
        assert cli("run", "--", "true").returncode == 0
    retained = runs.resolve("last").name
    # Explicit owner/consumer release, not age/count selection, authorizes retirement.
    from storage_lifecycle import Storage

    storage = Storage()
    for row in storage.records():
        if row["owner"].get("path", "").endswith(retained):
            continue
        for consumer in row["obligations"]:
            storage.release(row["id"], consumer, "test consumer completed")
    assert cli("retain", retained).returncode == 0
    running = start_background("sleep", "120")
    interrupted = start_background("sleep", "120")
    try:
        os.kill(status(interrupted)["owner"]["pid"], signal.SIGKILL)
        wait_for(lambda: status(interrupted)["state"] == "interrupted")

        result = cli("prune", "--keep", "0", "--json")
        removed = set(json.loads(result.stdout)["removed"])
        assert len(removed) == 2 and retained not in removed, result.stdout
        left = {d.name for d in root.iterdir()}
        assert {retained, running, interrupted} <= left and not removed & left
    finally:
        for ref in (running, interrupted):
            cli("cancel", ref)


def test_prune_keep_and_age(root: Path) -> None:
    for _ in range(3):
        cli("run", "--", "true")
    assert len(runs.prune_candidates(1, None)) == 2
    assert runs.prune_candidates(None, 3600.0) == []
    assert len(runs.prune_candidates(None, None)) == 0  # default keeps 20
    assert runs.parse_duration("1d12h") == 1.5 * 86400
    with pytest.raises(argparse.ArgumentTypeError):
        runs.parse_duration("7 days")


def test_nested_run_reuses_a_live_enclosing_run_and_progress_folds(root: Path) -> None:
    inner = (
        f"import sys; sys.path.insert(0, {str(SCRIPTS)!r}); import runs;"
        "assert runs.set_progress(current_command='cargo nextest', waiting_reason='lock');"
        "print(runs.summary_path())"
    )
    command = [sys.executable, str(SCRIPT), "run", "--", sys.executable, "-c", inner]
    result = cli("run", "--label", "outer", "--", *command)
    assert result.returncode == 0, result.stdout + result.stderr
    assert len(list(root.iterdir())) == 1  # the nested run created no record
    run_dir = runs.resolve("last")
    assert str(run_dir / "summary.json") in result.stdout
    record = record_of(run_dir.name)
    assert record["current_command"] == "cargo nextest"
    assert record["waiting_reason"] == "lock"
    assert "inside run" in (run_dir / "output.log").read_text()


def test_stale_run_variables_start_a_new_run(root: Path) -> None:
    cli("run", "--", "true")
    stale = runs.resolve("last")
    env = {**os.environ, "LCTX_RUN_DIR": str(stale), "LCTX_RUN_ID": stale.name}
    assert cli("run", "--", "true", env=env).returncode == 0
    assert len(list(root.iterdir())) == 2


def test_outside_a_run_progress_is_refused(root: Path) -> None:
    assert runs.current_run() is None
    assert runs.set_progress(current_command="x") is False
    assert runs.summary_path() is None


def test_unknown_reference_is_an_error(root: Path) -> None:
    assert cli("status", "nothing").returncode == 2


def test_owner_lock_answers_where_the_identity_cannot(tmp_path: Path) -> None:
    """A sandboxed tool call sees the owner in another pid namespace; the flock still answers."""
    me = ProcessIdentity.of().to_json()
    record = {"owner": {**me, "pid_namespace": me["pid_namespace"] + 1}, "termination": None}
    holder = subprocess.Popen(
        [
            sys.executable,
            "-c",
            "import fcntl, os, sys, time;"
            "fd = os.open(sys.argv[1], os.O_WRONLY | os.O_CREAT); fcntl.flock(fd, fcntl.LOCK_EX);"
            "print('held', flush=True); time.sleep(60)",
            str(tmp_path / runs.OWNER_LOCK),
        ],
        stdout=subprocess.PIPE,
        text=True,
    )
    try:
        assert holder.stdout is not None
        assert holder.stdout.readline().strip() == "held"
        assert runs.state_of(tmp_path, record) == "running"
    finally:
        holder.kill()
        holder.wait()
    assert runs.state_of(tmp_path, record) == "interrupted"


def test_cancel_request_alone_stops_the_run(root: Path) -> None:
    """The owner honours `cancel.json` itself, so cancellation works where signals cannot reach."""
    ref = start_background("sleep", "120")
    run_dir = runs.resolve(ref)
    (run_dir / runs.CANCEL).write_text('{"requested": "test"}')
    wait_for(lambda: status(ref)["termination"] is not None)
    assert status(ref)["termination"] == "cancelled"


def test_label_resolves_to_its_newest_run(root: Path) -> None:
    cli("run", "--label", "again", "--", "true")
    cli("run", "--label", "again", "--", "false")
    assert status("again")["exit"] == {"code": 1}


@pytest.mark.parametrize("failure", ["identity", "receipt", "progress", "final_receipt"])
def test_post_spawn_failure_cleans_actual_group(root, monkeypatch, failure, capsys):
    run_dir = runs.new_run_dir()
    owner = runs.Owner(
        run_dir, ["sleep", "120"], label=None, cwd=str(root.parent), mode="foreground", stream=False
    )
    spawned = []
    original_spawn = runs.spawn_group
    original_write = owner.write
    writes = 0

    def spawn(*args, **kwargs):
        child = original_spawn(*args, **kwargs)
        spawned.append(child)
        return child

    def write():
        nonlocal writes
        writes += 1
        if (failure == "receipt" and writes == 2) or (failure == "final_receipt" and writes >= 3):
            raise OSError("injected record failure")
        original_write()

    monkeypatch.setattr(runs, "spawn_group", spawn)
    monkeypatch.setattr(owner, "write", write)
    if failure == "identity":
        monkeypatch.setattr(
            runs.ProcessIdentity,
            "of",
            classmethod(lambda cls, pid=None: (_ for _ in ()).throw(OSError("identity"))),
        )
    if failure == "progress":
        monkeypatch.setattr(
            owner, "_fold_progress", lambda: (_ for _ in ()).throw(ValueError("progress"))
        )
    if failure == "final_receipt":
        owner.argv = ["true"]
    assert owner.run() != 0
    assert spawned and group_members(spawned[0].pid) == []
    assert spawned[0].returncode is not None
    assert owner.record["supervisor_error"]
    assert owner.record["cleanup"]["status"] == "confirmed"
    assert not runs.lock_held(run_dir / runs.OWNER_LOCK)
    if failure == "final_receipt":
        assert owner.record["exit"] == {"code": 0}
        assert "receipt failed" in capsys.readouterr().err


def _historical_record(run_dir, child):
    owner = ProcessIdentity.of().to_json() | {"start_ticks": 0}
    record = {
        "schema": 1,
        "id": run_dir.name,
        "owner": owner,
        "child": child,
        "termination": "completed",
        "exit": {"code": 7},
        "ended": runs.now(),
        "extension": {"unknown": "preserve me"},
    }
    runs.write_json_atomic(run_dir / runs.RECORD, record)
    (run_dir / runs.OUTPUT).write_bytes(b"retained capture\x00")
    return record


def test_legacy_observation_preserves_bytes_and_selected_recovery_preserves_exit(root):
    child = runs.spawn_group(["true"])
    identity = ProcessIdentity.of(child.pid).to_json() | {"pgid": child.pid}
    child.wait()
    run_dir = runs.new_run_dir()
    original = _historical_record(run_dir, identity)
    before = (run_dir / runs.RECORD).read_bytes()
    capture = (run_dir / runs.OUTPUT).read_bytes()
    observed = runs.view(run_dir)
    assert observed["cleanup"]["status"] == "unknown"
    assert run_dir not in runs.prune_candidates(0, None)
    assert (run_dir / runs.RECORD).read_bytes() == before
    final = runs.cancel(run_dir, grace=0.1)
    assert final["cleanup"]["status"] == "confirmed" and final["schema"] == 2
    assert final["exit"] == original["exit"] and final["extension"] == original["extension"]
    assert (run_dir / runs.OUTPUT).read_bytes() == capture
    assert run_dir in runs.prune_candidates(0, None)


@pytest.mark.parametrize("foreign", [False, True])
def test_recovery_does_not_signal_foreign_or_reused_pid(root, foreign):
    child = runs.spawn_group(["sleep", "120"])
    try:
        identity = ProcessIdentity.of(child.pid).to_json() | {"pgid": child.pid}
        if foreign:
            identity["pid_namespace"] += 1
        else:
            identity["start_ticks"] += 1
        run_dir = runs.new_run_dir()
        _historical_record(run_dir, identity)
        recovered = runs.cancel(run_dir, grace=0.1)
        assert recovered["cleanup"]["status"] == "unknown"
        assert child.poll() is None
        assert run_dir not in runs.prune_candidates(0, None)
    finally:
        runs.signal_group(child.pid, grace=0.1)
        child.wait()


def test_failed_cleanup_is_protected_and_retry_keeps_child_exit(root, monkeypatch):
    child = runs.spawn_group(["sleep", "120"])
    run_dir = runs.new_run_dir()
    identity = ProcessIdentity.of(child.pid).to_json() | {"pgid": child.pid}
    _historical_record(run_dir, identity)
    cleanup = runs.cleanup_group
    try:
        monkeypatch.setattr(runs, "cleanup_group", lambda *_args, **_kw: {"status": "failed"})
        first = runs.cancel(run_dir, grace=0.1)
        assert first["cleanup"]["status"] == "failed" and child.poll() is None
        assert run_dir not in runs.prune_candidates(0, None)
        monkeypatch.setattr(runs, "cleanup_group", cleanup)
        second = runs.cancel(run_dir, grace=0.1)
        assert second["cleanup"]["status"] == "confirmed"
        assert second["exit"] == {"code": 7} and len(second["recovery"]) == 2
        assert group_members(child.pid) == []
    finally:
        cleanup(child.pid, grace=0.1)
        child.wait()


def test_live_owner_recovery_only_writes_request_and_preserves_legacy_record(root, monkeypatch):
    run_dir = runs.new_run_dir()
    record = _historical_record(run_dir, None)
    descriptor = runs.hold_lock(run_dir / runs.OWNER_LOCK)
    original = (run_dir / runs.RECORD).read_bytes()
    try:
        monkeypatch.setattr(runs, "_wait_terminal", lambda *_args: record)
        monkeypatch.setattr(
            runs, "signal_group", lambda *_args, **_kw: pytest.fail("live owner owns signalling")
        )
        result = runs.cancel(run_dir, grace=0.1)
        assert result["cancel_result"] == "requested; cleanup unresolved"
        assert (run_dir / runs.CANCEL).exists()
        assert (run_dir / runs.RECORD).read_bytes() == original
    finally:
        os.close(descriptor)


def test_prune_rechecks_retention_under_owner_lock(root, monkeypatch):
    assert cli("run", "--", "true").returncode == 0
    run_dir = runs.resolve("last")
    import storage_owners
    from storage_lifecycle import Storage

    storage = Storage()
    for row in storage.records():
        for consumer in row["obligations"]:
            storage.release(row["id"], consumer, "test consumer completed")
    acquire = storage_owners.try_hold_lock

    def retain_before_acquisition(path):
        (run_dir / runs.RETAIN).touch()
        return acquire(path)

    monkeypatch.setattr(storage_owners, "try_hold_lock", retain_before_acquisition)
    assert (
        runs.cmd_prune(argparse.Namespace(keep=0, older_than=None, dry_run=False, json=True)) == 0
    )
    assert (run_dir / runs.RECORD).exists()


def test_dead_recovery_rereads_after_exclusive_acquisition(root, monkeypatch):
    child = runs.spawn_group(["true"])
    identity = ProcessIdentity.of(child.pid).to_json() | {"pgid": child.pid}
    child.wait()
    run_dir = runs.new_run_dir()
    record = _historical_record(run_dir, identity)
    acquire = runs.try_hold_lock

    def publish_before_acquisition(path):
        # A preceding owner/recovery completes just before we acquire; stale observation must
        # not clobber its newer exit/extension. The real flock still protects our publication.
        runs.write_json_atomic(
            run_dir / runs.RECORD, record | {"exit": {"code": 23}, "newer": True}
        )
        return acquire(path)

    monkeypatch.setattr(runs, "try_hold_lock", publish_before_acquisition)
    recovered = runs.cancel(run_dir, grace=0.1)
    assert recovered["exit"] == {"code": 23} and recovered["newer"] is True


@pytest.mark.parametrize("refusal,status", [(False, "failed"), (True, "unknown")])
def test_terminal_child_keeps_failed_cleanup_recoverable(root, monkeypatch, refusal, status):
    import harness

    run_dir = runs.new_run_dir()
    owner = runs.Owner(
        run_dir,
        ["sh", "-c", "sleep 120 & exit 0"],
        label=None,
        cwd=str(root.parent),
        mode="foreground",
        stream=False,
    )
    cleanup = harness.cleanup_group

    def refuse(*_args, **_kwargs):
        if refusal:
            raise OSError("injected uncertain signalling")
        return False

    try:
        with monkeypatch.context() as patch:
            patch.setattr(harness, "signal_group", refuse)
            assert owner.run() == 1
        record = runs.load_record(run_dir)
        assert record is not None
        assert record["exit"] == {"code": 0}
        assert record["cleanup"]["status"] == status
        assert runs.group_of(record)["status"] == "owned"
        assert run_dir not in runs.prune_candidates(0, None)
        recovered = runs.cancel(run_dir, grace=0.1)
        assert recovered["cleanup"]["status"] == "confirmed"
        assert recovered["exit"] == {"code": 0}
        assert group_members(record["child"]["pgid"]) == []
    finally:
        if owner.record.get("child"):
            cleanup(owner.record["child"]["pgid"], grace=0.1)


@pytest.mark.parametrize("broken", [False, True])
def test_nonconsuming_or_broken_output_does_not_block_child_or_log(root, broken):
    size = 1024 * 1024
    launcher = subprocess.Popen(
        [
            sys.executable,
            str(SCRIPT),
            "run",
            "--",
            sys.executable,
            "-c",
            f"import sys; sys.stdout.buffer.write(b'x'*{size})",
        ],
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
    )
    try:
        assert launcher.stdout is not None
        if broken:
            launcher.stdout.close()
        assert launcher.wait(timeout=15) == 0
        run_dir = runs.resolve("last")
        assert (run_dir / runs.OUTPUT).read_bytes() == b"x" * size
        record = runs.load_record(run_dir)
        assert record is not None
        assert record["exit"] == {"code": 0}
        assert record["supervisor_error"] is None
        assert record["cleanup"]["status"] == "confirmed"
    finally:
        if launcher.poll() is None:
            launcher.kill()
            launcher.wait()
        if launcher.stdout is not None:
            launcher.stdout.close()


def test_cancellation_remains_usable_with_nonconsuming_output(root):
    launcher = subprocess.Popen(
        [
            sys.executable,
            str(SCRIPT),
            "run",
            "--label",
            "blocked-display",
            "--",
            sys.executable,
            "-c",
            "import sys,time; sys.stdout.buffer.write(b'x'*1048576); "
            "sys.stdout.flush(); time.sleep(120)",
        ],
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
    )
    try:
        wait_for(
            lambda: (
                root.exists()
                and any(
                    (d / runs.OUTPUT).exists() and (d / runs.OUTPUT).stat().st_size == 1048576
                    for d in root.iterdir()
                )
            )
        )
        result = cli("cancel", "blocked-display", "--json")
        assert result.returncode == 0, result.stderr
        assert json.loads(result.stdout)["cleanup"]["status"] == "confirmed"
        assert launcher.wait(timeout=15) != 0
        assert (runs.resolve("blocked-display") / runs.OUTPUT).stat().st_size == 1048576
    finally:
        if launcher.poll() is None:
            launcher.kill()
            launcher.wait()
        if launcher.stdout is not None:
            launcher.stdout.close()


def test_logs_tail_zero_is_empty(root):
    assert cli("run", "--", "printf", "one\\ntwo\\n").returncode == 0
    result = cli("logs", "last", "--tail", "0")
    assert result.returncode == 0 and result.stdout == ""


def test_run_owner_keeps_leader_unreaped_until_cleanup(root, monkeypatch):
    import harness

    run_dir = runs.new_run_dir()
    owner = runs.Owner(
        run_dir,
        ["sh", "-c", "sleep 120 & exit 9"],
        label=None,
        cwd=str(root.parent),
        mode="foreground",
        stream=False,
    )
    cleanup = harness.cleanup_group
    pins = []

    def check_pin(pgid, **kwargs):
        status = os.waitid(os.P_PID, pgid, os.WEXITED | os.WNOHANG | os.WNOWAIT)
        assert status is not None and status.si_status == 9
        pins.append(pgid)
        return cleanup(pgid, **kwargs)

    monkeypatch.setattr(harness, "cleanup_group", check_pin)
    assert owner.run() == 9
    record = runs.load_record(run_dir)
    assert record is not None
    assert record["exit"] == {"code": 9}
    assert record["cleanup"]["status"] == "confirmed"
    assert pins == [record["child"]["pid"]]
    assert group_members(pins[0]) == []
