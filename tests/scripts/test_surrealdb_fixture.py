"""Native fixture boundary: pinned-binary readiness, memory policy, sweep ownership rules and two
live lifecycle checks (launcher SIGKILL, OOM during readiness)."""

from __future__ import annotations

import json
import signal
import subprocess
import sys
import time
from pathlib import Path

import pytest

import surrealdb_fixture as fx
from harness import ProcessIdentity, write_json_atomic

SCRIPT = Path(fx.__file__).resolve()


def completed(stdout: str = "", returncode: int = 0) -> subprocess.CompletedProcess:
    return subprocess.CompletedProcess([], returncode, stdout, "")


@pytest.fixture
def root(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> Path:
    path = tmp_path / "fixtures"
    path.mkdir()
    monkeypatch.setenv("LCTX_FIXTURES_ROOT", str(path))
    return path


@pytest.fixture
def no_systemd(monkeypatch: pytest.MonkeyPatch) -> list[tuple[str, ...]]:
    calls: list[tuple[str, ...]] = []

    def fake(*args: str, runner=None) -> subprocess.CompletedProcess:
        calls.append(args)
        if args[0] == "list-units":
            return completed("")
        return completed("ActiveState=inactive\nResult=success\nLoadState=not-found\n")

    monkeypatch.setattr(fx, "systemctl", fake)
    return calls


def dead_identity() -> dict:
    child = subprocess.Popen(["sleep", "0.2"])
    identity = ProcessIdentity.of(child.pid).to_json()
    child.wait()
    return identity


# -- pinned binary and readiness ---------------------------------------------------------------


def test_binary_with_wrong_digest_is_blocked_with_repair(tmp_path: Path) -> None:
    binary = tmp_path / "surreal"
    binary.write_bytes(b"not the pinned server")
    ready = fx.substrate_readiness(env={"LCTX_SURREAL_BIN": str(binary)}, runner=pytest.fail)
    assert not ready.ready and ready.outcome == "blocked"
    assert "sha256" in ready.detail and fx.RELEASE_URL in ready.repair
    assert ready.message().startswith("native-store: blocked: install the pinned binary")


def test_version_is_checked_after_the_digest(tmp_path: Path, monkeypatch) -> None:
    binary = tmp_path / "surreal"
    binary.write_bytes(b"x")
    monkeypatch.setattr(fx, "BINARY_SHA256", fx.file_sha256(binary))
    env = {"LCTX_SURREAL_BIN": str(binary)}
    wrong = fx.substrate_readiness(env=env, runner=lambda *a, **k: completed("3.2.0 for linux"))
    assert not wrong.ready and "3.2.0" in wrong.detail
    runs = iter([completed(fx.CLI_VERSION + "\n"), completed("", 1)])
    no_manager = fx.substrate_readiness(env=env, runner=lambda *a, **k: next(runs))
    assert not no_manager.ready and no_manager.repair == fx.SYSTEMD_REPAIR
    runs = iter([completed(fx.CLI_VERSION), completed("PATH=/bin")])
    assert fx.substrate_readiness(env=env, runner=lambda *a, **k: next(runs)).ready


def test_explicit_binary_replaces_the_default_candidates() -> None:
    assert fx.binary_candidates({"LCTX_SURREAL_BIN": "/x/surreal"}) == [Path("/x/surreal")]
    defaults = fx.binary_candidates({"HOME": "/home/u"})
    assert defaults[0].parts[-4:] == ("surreal", "v3.3.0", "linux-amd64", "surreal")
    assert defaults[1] == Path("/home/u/.surrealdb/surreal")


# -- memory and requirements -------------------------------------------------------------------


def test_memory_cap_scales_the_tracked_threshold() -> None:
    assert fx.parse_memory("1G") == 1 << 30 and fx.parse_memory("512MiB") == 512 << 20
    assert fx.server_environment(1 << 30, "p")["SURREAL_MEMORY_THRESHOLD"] == "512MiB"
    assert fx.server_environment(4 << 30, "p")["SURREAL_MEMORY_THRESHOLD"] == "2048MiB"
    assert fx.memory_text(150 << 20) == "150M"
    for bad in ("lots", "10M"):
        with pytest.raises(ValueError):
            fx.parse_memory(bad)


def test_python_ownership_is_explicit_or_evident_from_the_command() -> None:
    assert fx.python_requirements(["cargo", "nextest", "run"], []) == []
    assert fx.python_requirements(["uv", "run", "pytest"], []) == ["native-python"]
    assert fx.python_requirements(["cargo", "test"], ["native"]) == ["native"]
    assert fx.python_requirements(["uv", "run", "x"], ["none"]) == []


# -- sweep and inventory -----------------------------------------------------------------------


def write_fixture(root: Path, fixture_id: str, kind: str, owner: dict, **extra) -> Path:
    directory = root / fixture_id
    directory.mkdir()
    (directory / "server.env").write_text("SURREAL_PASS=x\n")
    write_json_atomic(
        directory / "record.json",
        {
            "schema": 1,
            "id": fixture_id,
            "kind": kind,
            "checkout": str(fx.ROOT),
            "unit": f"lctx-fixture-{fixture_id}.{'service' if kind == 'kept' else 'scope'}",
            "port": 1,
            "memory_max": 1 << 30,
            "created": "2026-10-07T00:00:00+00:00",
            "owner": owner,
            "server": None,
            **extra,
        },
    )
    return directory


def test_sweep_removes_only_dead_run_owned_state(root: Path, no_systemd) -> None:
    write_fixture(root, "dead", "run", dead_identity())
    write_fixture(root, "live", "run", ProcessIdentity.of().to_json())
    write_fixture(root, "kept", "kept", dead_identity())
    foreign = {**dead_identity(), "pid_namespace": 1}
    write_fixture(root, "foreign", "run", foreign)
    removed = fx.sweep(report=lambda _m: None)
    assert removed == ["dead"]
    assert sorted(p.name for p in root.iterdir()) == ["foreign", "kept", "live"]
    assert ("stop", "lctx-fixture-dead.scope") in no_systemd


def test_lock_held_owner_is_alive_across_pid_namespaces(root: Path, no_systemd) -> None:
    directory = write_fixture(root, "sibling", "run", {**dead_identity(), "pid_namespace": 1})
    holder = subprocess.Popen(
        [
            sys.executable,
            "-c",
            "import fcntl,sys,time;f=open(sys.argv[1],'w');"
            "fcntl.flock(f,fcntl.LOCK_EX);print('held',flush=True);time.sleep(30)",
            str(directory / "owner.lock"),
        ],
        stdout=subprocess.PIPE,
        text=True,
    )
    try:
        assert holder.stdout is not None and holder.stdout.readline().strip() == "held"
        record = json.loads((directory / "record.json").read_text())
        assert fx.owner_alive(record["owner"], directory / "owner.lock")
        assert fx.sweep(report=lambda _m: None) == []
    finally:
        holder.kill()
        holder.wait()
    assert not fx.owner_alive(record["owner"], directory / "owner.lock")


def test_dead_run_owned_fixture_terminates_its_surviving_tree(root: Path, no_systemd) -> None:
    survivor = subprocess.Popen(["sleep", "30"], start_new_session=True)
    try:
        directory = write_fixture(root, "orphan", "run", dead_identity())
        attachment = directory / "attachments" / "a1"
        attachment.mkdir(parents=True)
        write_json_atomic(
            attachment / "record.json",
            {
                "id": "a1",
                "owner": dead_identity(),
                "command": {"leader": ProcessIdentity.of(survivor.pid).to_json()},
            },
        )
        assert fx.sweep(report=lambda _m: None) == ["orphan"]
        assert survivor.wait(15) is not None
    finally:
        if survivor.poll() is None:
            survivor.kill()


def test_kept_attachment_with_live_consumers_is_preserved(root: Path, no_systemd, monkeypatch):
    monkeypatch.setattr(fx.Server, "exited", lambda self: "exit 0")
    directory = write_fixture(root, "kept", "kept", dead_identity())
    consumer = subprocess.Popen(["sleep", "30"], start_new_session=True)
    try:
        for name, leader in (("busy", ProcessIdentity.of(consumer.pid).to_json()), ("idle", None)):
            path = directory / "attachments" / name
            path.mkdir(parents=True)
            write_json_atomic(
                path / "record.json",
                {"id": name, "owner": dead_identity(), "command": {"leader": leader}},
            )
        assert fx.sweep(report=lambda _m: None) == ["kept/idle"]
        assert (directory / "attachments" / "busy").is_dir()
    finally:
        consumer.kill()
        consumer.wait()


def test_inventory_never_sweeps(root: Path, no_systemd) -> None:
    write_fixture(root, "dead", "run", dead_identity())
    rows = fx.inventory()
    assert [row["id"] for row in rows] == ["dead"] and rows[0]["owner_alive"] is False
    assert (root / "dead").is_dir()
    assert not any(call[0] in ("stop", "reset-failed") for call in no_systemd)


# -- live: the actual native substrate ---------------------------------------------------------

live = pytest.mark.skipif(
    not fx.substrate_readiness().ready, reason="native fixture substrate not ready"
)


def cli(*args: str, **kwargs) -> subprocess.Popen:
    return subprocess.Popen(
        [sys.executable, str(SCRIPT), "--no-sweep", *args],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        **kwargs,
    )


def server_processes(fixture_id: str) -> list[str]:
    result = subprocess.run(
        ["pgrep", "-f", f"fixtures/{fixture_id}/data/store"], capture_output=True, text=True
    )
    return result.stdout.split()


@live
def test_launcher_sigkill_leaves_no_server(root: Path) -> None:
    launcher = cli("--requires", "none", "--", "sleep", "60")
    assert launcher.stderr is not None
    line = launcher.stderr.readline()
    assert " ready: " in line, line
    fixture_id = line.split()[1]
    assert server_processes(fixture_id)
    launcher.send_signal(signal.SIGKILL)
    launcher.wait()
    deadline = time.monotonic() + 10
    while server_processes(fixture_id) and time.monotonic() < deadline:
        time.sleep(0.1)
    assert not server_processes(fixture_id)
    assert fx.sweep(report=lambda _m: None) == [fixture_id]


@live
def test_cap_below_startup_rss_is_reported_as_oom_during_readiness(root: Path) -> None:
    launcher = cli("--memory", "150M", "--", "true")
    _out, err = launcher.communicate(timeout=120)
    assert launcher.returncode == fx.EXIT_BLOCKED
    assert "blocked: oom" in err and "during readiness" in err
    assert list(root.iterdir()) == []
