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
    assert record["exit"] == {"code": 127} and record["termination"] == "completed"
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


def test_killed_launcher_leaves_a_discoverable_interrupted_run(root: Path) -> None:
    ref = start_background("sleep", "120")
    record = status(ref)
    owner, pgid = record["owner"]["pid"], record["child"]["pgid"]
    os.kill(owner, signal.SIGKILL)
    wait_for(lambda: not ProcessIdentity.from_json(record["owner"]).alive())

    interrupted = status(ref)
    assert interrupted["state"] == "interrupted"
    assert interrupted["termination"] is None
    assert interrupted["survivors"] == group_members(pgid) != []
    assert runs.resolve(ref) not in runs.prune_candidates(0, None)

    finalized = json.loads(cli("cancel", ref, "--json").stdout)
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
    assert cli("retain", retained).returncode == 0
    running = start_background("sleep", "120")
    interrupted = start_background("sleep", "120")
    os.kill(status(interrupted)["owner"]["pid"], signal.SIGKILL)
    wait_for(lambda: status(interrupted)["state"] == "interrupted")

    result = cli("prune", "--keep", "0", "--json")
    removed = set(json.loads(result.stdout)["removed"])
    assert len(removed) == 2 and retained not in removed
    left = {d.name for d in root.iterdir()}
    assert {retained, running, interrupted} <= left and not removed & left

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
