"""Shared harness identity, process-group and record behavior."""

import dataclasses
import subprocess
import sys

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
    assert rebooted.foreign() and not rebooted.alive()
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
