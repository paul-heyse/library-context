"""Shared harness identity, process-group and record behavior."""

import dataclasses
import subprocess
import sys

import pytest

from harness import (
    OUTCOMES,
    TERMINATIONS,
    ProcessIdentity,
    group_members,
    read_json,
    signal_group,
    spawn_group,
    write_json_atomic,
)


def test_identity_tracks_pid_reuse_boot_and_namespace():
    me = ProcessIdentity.of()
    assert me.alive() and not me.foreign()
    assert ProcessIdentity.from_json(me.to_json()) == me
    assert not dataclasses.replace(me, start_ticks=me.start_ticks + 1).alive()
    rebooted = dataclasses.replace(me, boot_id="00000000-0000-0000-0000-000000000000")
    # A previous boot is certainly dead (sweepable), not unknowable.
    assert rebooted.previous_boot() and not rebooted.foreign() and not rebooted.alive()
    other_namespace = dataclasses.replace(me, pid_namespace=me.pid_namespace + 1)
    assert other_namespace.foreign() and not other_namespace.alive()


def test_exited_process_is_not_alive_even_before_it_is_reaped():
    child = subprocess.Popen([sys.executable, "-c", "pass"])
    identity = ProcessIdentity.of(child.pid)
    child.wait()
    assert not identity.alive()


def test_group_signal_reaches_descendants_and_reports_no_survivors():
    leader = spawn_group(["bash", "-c", "sleep 30 & sleep 30 & wait"])
    try:
        for _ in range(100):
            if len(group_members(leader.pid)) >= 3:
                break
            subprocess.run(["sleep", "0.02"], check=True)
        assert len(group_members(leader.pid)) >= 3
        assert signal_group(leader.pid, grace=5.0)
        assert group_members(leader.pid) == []
    finally:
        leader.wait(timeout=5)


def test_group_signal_escalates_when_sigterm_is_ignored():
    leader = spawn_group(["bash", "-c", "trap '' TERM; sleep 30 & wait"])
    try:
        subprocess.run(["sleep", "0.2"], check=True)
        assert signal_group(leader.pid, grace=0.3)
        assert group_members(leader.pid) == []
    finally:
        leader.wait(timeout=5)


def test_atomic_json_replaces_without_leaving_temporaries(tmp_path):
    path = tmp_path / "nested" / "record.json"
    write_json_atomic(path, {"phase": "running"})
    write_json_atomic(path, {"phase": "completed"})
    assert read_json(path) == {"phase": "completed"}
    assert [entry.name for entry in path.parent.iterdir()] == ["record.json"]
    assert read_json(tmp_path / "absent.json") is None


def test_outcome_and_termination_vocabularies_are_distinct():
    assert OUTCOMES == ("passed", "failed", "blocked", "not_run")
    assert TERMINATIONS == ("completed", "interrupted", "cancelled")
    assert not set(OUTCOMES) & set(TERMINATIONS)


def test_spawn_guard_cleans_group_when_identity_work_raises():
    from harness import SpawnGuard

    child = spawn_group(["sh", "-c", "sleep 120 & wait"])
    with pytest.raises(RuntimeError, match="identity"), SpawnGuard(child, grace=0.1) as guard:
        raise RuntimeError("identity")
    assert guard.cleanup["status"] == "confirmed"
    assert group_members(child.pid) == []
    assert child.returncode is not None


def test_recorded_group_refuses_reused_and_foreign_identity():
    from harness import recorded_group

    child = spawn_group(["sleep", "120"])
    try:
        identity = ProcessIdentity.of(child.pid).to_json() | {"pgid": child.pid}
        assert recorded_group(identity)["status"] == "owned"
        assert (
            recorded_group(identity | {"start_ticks": identity["start_ticks"] + 1})["status"]
            == "unknown"
        )
        assert (
            recorded_group(identity | {"pid_namespace": identity["pid_namespace"] + 1})["status"]
            == "unknown"
        )
    finally:
        signal_group(child.pid, grace=0.1)
        child.wait()
    assert recorded_group(identity)["status"] == "gone"


@pytest.mark.parametrize(
    "data",
    [
        b"",
        b"a",
        b"a\n",
        b"a\nb",
        b"a\nb\n",
        b"a\r\nb\r\n",
        "one\n\u96ea\nlast".encode(),
        b"old\n" + b"z" * 300,
    ],
)
@pytest.mark.parametrize("lines", [0, 1, 2, 10])
def test_tail_bytes_preserves_suffix(tmp_path, data, lines):
    from harness import tail_bytes

    path = tmp_path / "output.log"
    path.write_bytes(data)
    expected = b"" if lines == 0 else b"".join(data.splitlines(keepends=True)[-lines:])
    assert tail_bytes(path, lines, block_size=7) == expected


def test_tail_reads_suffix_not_large_log(tmp_path, monkeypatch):
    from pathlib import Path

    from harness import tail_bytes

    path = tmp_path / "output.log"
    path.write_bytes(b"old line\n" * 100000 + b"last\npartial")
    original_open = Path.open
    read_sizes = []

    class Observed:
        def __init__(self, handle):
            self.handle = handle

        def __enter__(self):
            return self

        def __exit__(self, *_args):
            self.handle.close()

        def read(self, size=-1):
            assert size >= 0, "must not hydrate the whole log"
            read_sizes.append(size)
            return self.handle.read(size)

        def seek(self, *args):
            return self.handle.seek(*args)

    monkeypatch.setattr(
        Path, "open", lambda self, *a, **kw: Observed(original_open(self, *a, **kw))
    )
    assert tail_bytes(path, 2, block_size=256) == b"last\npartial"
    assert sum(read_sizes) <= 257


def test_follow_forwards_partial_utf8_then_resets_rotation_and_truncation(tmp_path):
    from harness import iter_file_chunks

    path = tmp_path / "output.log"
    text = "\u96ea".encode()
    path.write_bytes(b"old output\n" + text[:1])
    active = True
    chunks = iter_file_chunks(path, follow=True, alive=lambda: active)
    assert next(chunks) == b"old output\n" + text[:1]
    with path.open("ab") as handle:
        handle.write(text[1:] + b"\n")
    assert next(chunks) == text[1:] + b"\n"
    path.write_bytes(b"new")
    assert next(chunks) == b"new"
    path.rename(tmp_path / "previous.log")
    path.write_bytes(b"replacement\r\n")
    assert next(chunks) == b"replacement\r\n"
    active = False
    assert list(chunks) == []


def test_display_failure_is_best_effort_and_blocked_observer_detaches(tmp_path):
    import os

    from harness import FileDisplay

    path = tmp_path / "output.log"
    path.write_bytes(b"x" * 1024 * 1024)
    read_end, write_end = os.pipe()
    observer = FileDisplay(path, sink_fd=write_end).start()
    try:
        assert observer.process is not None
        assert observer.process.pid not in group_members(os.getpgrp())
        observer.close()
        assert observer.process.poll() is not None
    finally:
        observer.close()
        os.close(write_end)
        os.close(read_end)
    FileDisplay(path, sink_fd=-1).start().close()


@pytest.mark.parametrize(
    "command,expected", [(["sh", "-c", "exit 23"], 23), (["sh", "-c", "kill -TERM $$"], -15)]
)
def test_exit_observation_keeps_original_leader_waitable(command, expected):
    import os
    import time

    from harness import SpawnGuard, observe_exit

    child = spawn_group(command)
    identity = ProcessIdentity.of(child.pid)
    try:
        with SpawnGuard(child, grace=0.1) as guard:
            code = None
            while code is None:
                code = observe_exit(child)
                time.sleep(0.01)
            assert code == expected
            assert child.returncode is None
            # A second waitid sees the same terminal event: observation did not consume it.
            status = os.waitid(os.P_PID, child.pid, os.WEXITED | os.WNOHANG | os.WNOWAIT)
            assert status is not None and status.si_pid == child.pid
            assert ProcessIdentity.of(child.pid) == identity
        assert guard.cleanup["status"] == "confirmed"
        assert guard.returncode == child.returncode == expected
        with pytest.raises(ChildProcessError):
            os.waitid(os.P_PID, child.pid, os.WEXITED | os.WNOHANG | os.WNOWAIT)
    finally:
        child.wait()


def test_exited_leader_pins_identity_during_actual_descendant_cleanup(monkeypatch):
    import os
    import time

    import harness

    child = spawn_group(["sh", "-c", "sleep 120 & exit 17"])
    cleanup = harness.cleanup_group
    observed = []

    def check_pin(pgid, **kwargs):
        status = os.waitid(os.P_PID, child.pid, os.WEXITED | os.WNOHANG | os.WNOWAIT)
        assert status is not None and status.si_status == 17
        assert child.returncode is None
        assert group_members(pgid) and child.pid not in group_members(pgid)
        observed.append(pgid)
        result = cleanup(pgid, **kwargs)
        # Cleanup itself must not consume the leader event either.
        assert os.waitid(os.P_PID, child.pid, os.WEXITED | os.WNOHANG | os.WNOWAIT) is not None
        return result

    try:
        monkeypatch.setattr(harness, "cleanup_group", check_pin)
        with harness.SpawnGuard(child, grace=0.1) as guard:
            while harness.observe_exit(child) is None:
                time.sleep(0.01)
        assert observed == [child.pid]
        assert guard.returncode == 17 and guard.cleanup["status"] == "confirmed"
        assert group_members(child.pid) == []
    finally:
        cleanup(child.pid, grace=0.1)
        child.wait()


@pytest.mark.parametrize("external_reaper", [False, True])
def test_guard_refuses_numeric_group_after_leader_pin_is_lost(monkeypatch, external_reaper):
    import os

    import harness

    original = spawn_group(["true"])
    unrelated = spawn_group(["sleep", "120"])
    guard = harness.SpawnGuard(original, grace=0.1)
    try:
        if external_reaper:
            os.waitpid(original.pid, 0)  # Leave Popen's cache unset, as another reaper would.
        else:
            original.wait()
        # Model the numeric-ID reuse at the exact authority seam with a real unrelated group.
        # The lost original pin must refuse cleanup before discovering/sampling these members.
        guard.pgid = unrelated.pid
        monkeypatch.setattr(
            harness,
            "cleanup_group",
            lambda *_args, **_kw: pytest.fail("lost pin must not authorize current numeric group"),
        )
        guard.__exit__(None, None, None)
        assert guard.cleanup["status"] == "unknown"
        assert unrelated.poll() is None
        assert group_members(unrelated.pid) == [unrelated.pid]
    finally:
        signal_group(unrelated.pid, grace=0.1)
        unrelated.wait()
        original.wait()
