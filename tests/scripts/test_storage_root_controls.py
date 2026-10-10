"""Coordinator acceptance for indexed lifetimes and bounded manager metadata."""

from __future__ import annotations

import contextlib
import datetime as dt
import os
import signal
import subprocess
import sys
import time
from pathlib import Path

import pytest
import test_storage_lifecycle as base_controls
from test_storage_lifecycle import publish, release

import storage_lifecycle as lifecycle

storage = base_controls.storage


def test_publish_uses_bounded_path_index_after_initialization(storage, monkeypatch):
    first = publish(storage, storage.root / "first")

    def forbidden():
        raise AssertionError("producer rescanned unrelated host history")

    monkeypatch.setattr(storage, "records", forbidden)
    assert (
        storage.publish(
            storage.root / "first", "scratch", storage.get(first)["owner"], "control", managed=True
        )
        == first
    )
    path = storage.root / "second"
    path.mkdir()
    storage.publish(path, "scratch", storage.get(first)["owner"], "control", managed=True)


def test_old_release_does_not_replace_reused_path_index(storage):
    path = storage.root / "reused"
    old = publish(storage, path)
    release(storage, old)
    storage.retire(old, references=[])
    new = publish(storage, path)
    storage.release(old, "control", "late receipt bookkeeping")
    assert (
        storage.publish(path, "scratch", storage.get(new)["owner"], "control", managed=True) == new
    )
    assert len([row for row in storage.records() if not row.get("retired_at")]) == 1


def test_closed_metadata_expiry_preserves_referenced_tombstone_and_journals(storage):
    ident = publish(storage, storage.root / "closed")
    release(storage, ident)
    storage.retire(ident, references=[])
    row = storage.get(ident)
    row["retired_at"] = (dt.datetime.now(dt.UTC) - dt.timedelta(days=91)).isoformat()
    storage.save(row)
    lifecycle.durable_json(storage.state / "queue" / (ident + ".json"), {"id": ident})
    storage.trim_metadata([(storage.root / "evidence.md", ident)])
    assert storage.get(ident)
    storage.trim_metadata([])
    assert not (storage.state / "objects" / (ident + ".json")).exists()
    assert not (storage.state / "queue" / (ident + ".json")).exists()


@pytest.mark.parametrize("value", ["inf", "nan", "-1"])
def test_nonfinite_or_negative_policy_duration_refuses(storage, value):
    storage.config.write_text(
        'schema=1\n[categories.scratch]\nowner="task"\ngrace_days=' + value + "\n"
    )
    with pytest.raises(lifecycle.Invalid):
        lifecycle.Storage(root=storage.root, state=storage.state)


@pytest.mark.parametrize("stop", ["normal", "terminate", "kill"])
def test_actual_managed_launcher_cleanup_and_durable_hold(storage, stop):
    path = storage.root / "isolated-target"
    identity = publish(storage, path)
    release(storage, identity)
    marker = storage.root / "child-ready"
    # The leader leaves a live descendant when exiting normally. A dead launcher must
    # never make that descendant's build lifetime eligible solely by dropping its lock.
    child = (
        "import subprocess,sys,time; "
        'p=subprocess.Popen([sys.executable,"-c","import time; time.sleep(120)"], '
        "stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL); "
        'open(sys.argv[1],"w").write(str(p.pid)); '
        'time.sleep(0.3 if sys.argv[2]=="normal" else 120)'
    )
    command = (
        "import sys; from pathlib import Path; import build_environment as b; "
        'b.ROOT=Path(sys.argv[1]); sys.argv=["launcher","--",*sys.argv[2:]]; b.main()'
    )
    environment = dict(
        os.environ, PYTHONPATH=str(Path(lifecycle.__file__).parent), LCTX_CARGO_TARGET_DIR=str(path)
    )
    launcher = subprocess.Popen(
        [
            sys.executable,
            "-c",
            command,
            str(storage.root),
            sys.executable,
            "-c",
            child,
            str(marker),
            stop,
        ],
        env=environment,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    descendant = None
    try:
        deadline = time.monotonic() + 10
        while not marker.exists():
            if launcher.poll() is not None or time.monotonic() > deadline:
                raise AssertionError(launcher.communicate(timeout=2))
            time.sleep(0.01)
        descendant = int(marker.read_text())
        assert any(
            key.startswith("launcher:") and not value.get("released_at")
            for key, value in storage.get(identity)["obligations"].items()
        )
        with pytest.raises(lifecycle.Blocked):
            storage.retire(identity, references=[])
        if stop == "terminate":
            launcher.terminate()
        elif stop == "kill":
            launcher.kill()
        launcher.communicate(timeout=20)
        holds = storage.get(identity)["obligations"]
        assert any(
            key.startswith("launcher:") and not value.get("released_at")
            for key, value in holds.items()
        ) == (stop == "kill")
        if stop == "kill":
            with pytest.raises(lifecycle.Blocked):
                storage.retire(identity, references=[])
        else:
            assert launcher.returncode == (0 if stop == "normal" else 130)
            # A zombie has stopped reading output; the host reaper may reap it later.
            stat = Path(f"/proc/{descendant}/stat")
            assert not stat.exists() or stat.read_text().split(") ", 1)[1].startswith("Z ")
    finally:
        if launcher.poll() is None:
            launcher.kill()
        if descendant is not None:
            with contextlib.suppress(ProcessLookupError):
                os.kill(descendant, signal.SIGKILL)
        launcher.communicate(timeout=5)
