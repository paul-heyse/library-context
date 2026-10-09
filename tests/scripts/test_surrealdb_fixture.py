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


def test_systemd_route(tmp_path, monkeypatch):
    import os
    import socket

    monkeypatch.setattr(fx, "SYSTEMD_RUNTIME_ROOT", tmp_path)
    runtime = tmp_path / str(os.getuid())
    runtime.mkdir()
    source = {"sentinel": "preserved"}
    assert fx.systemd_environment(source) == source
    (runtime / "bus").write_text("not a socket")
    assert fx.systemd_environment(source) == source
    (runtime / "bus").unlink()
    with socket.socket(socket.AF_UNIX) as bus:
        bus.bind(str(runtime / "bus"))
        routed = fx.systemd_environment(source)
        assert routed == {
            **source,
            "XDG_RUNTIME_DIR": str(runtime),
            "DBUS_SESSION_BUS_ADDRESS": f"unix:path={runtime / 'bus'}",
        }
        assert source == {"sentinel": "preserved"}
        explicit_bus = {"DBUS_SESSION_BUS_ADDRESS": "unix:path=/explicit/selection"}
        assert (
            fx.systemd_environment(explicit_bus)["DBUS_SESSION_BUS_ADDRESS"]
            == explicit_bus["DBUS_SESSION_BUS_ADDRESS"]
        )
        explicit_runtime = {"XDG_RUNTIME_DIR": str(tmp_path / "missing-selected-runtime")}
        assert fx.systemd_environment(explicit_runtime) == explicit_runtime
        uid = os.getuid()
        monkeypatch.setattr(fx.os, "getuid", lambda: uid + 1)
        explicit_runtime = {"XDG_RUNTIME_DIR": str(runtime)}
        assert fx.systemd_environment(explicit_runtime) == explicit_runtime


# -- memory and requirements -------------------------------------------------------------------


def test_memory_cap_scales_the_tracked_threshold() -> None:
    assert fx.parse_memory("1G") == 1 << 30 and fx.parse_memory("512MiB") == 512 << 20
    assert fx.server_environment(1 << 30, "p")["SURREAL_MEMORY_THRESHOLD"] == "512MiB"
    assert fx.server_environment(4 << 30, "p")["SURREAL_MEMORY_THRESHOLD"] == "2048MiB"
    assert fx.memory_text(150 << 20) == "150M"
    for bad in ("lots", "10M"):
        with pytest.raises(ValueError):
            fx.parse_memory(bad)


def test_default_memory_and_explicit_overrides(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.delenv("LCTX_FIXTURE_MEMORY", raising=False)
    assert fx.configured_memory() == 16 << 30
    environment = fx.server_environment(fx.configured_memory(), "p")
    assert environment["SURREAL_MEMORY_THRESHOLD"] == "8192MiB"
    monkeypatch.setenv("LCTX_FIXTURE_MEMORY", "32G")
    assert fx.configured_memory() == 32 << 30
    assert fx.configured_memory("24G") == 24 << 30


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
    # Another boot is certainly dead, even with a pid that is alive now (ours) and another
    # namespace inode: its record and unit go.
    rebooted = {**ProcessIdentity.of().to_json(), "boot_id": "previous-boot", "pid_namespace": 1}
    directory = write_fixture(root, "rebooted", "run", rebooted)
    attachment = directory / "attachments" / "a1"
    attachment.mkdir(parents=True)
    write_json_atomic(
        attachment / "record.json",
        {"id": "a1", "owner": rebooted, "command": {"leader": rebooted}},
    )
    removed = fx.sweep(report=lambda _m: None)
    assert sorted(removed) == ["dead", "rebooted"]
    assert sorted(p.name for p in root.iterdir()) == ["foreign", "kept", "live"]
    assert ("stop", "lctx-fixture-dead.scope") in no_systemd
    assert ("stop", "lctx-fixture-rebooted.scope") in no_systemd
    assert ("stop", "lctx-fixture-foreign.scope") not in no_systemd


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


def test_failed_retention_is_cleaned_up_and_replaceable(tmp_path: Path, monkeypatch) -> None:
    server = fx.Server(tmp_path, {"id": "f1", "kind": "kept", "port": 1}, "secret")
    monkeypatch.setattr(fx.Server, "exited", lambda self: "exit 0")
    attachment = fx.Attachment(server, "a1", tmp_path / "a1", {"namespace": "ns"})
    (tmp_path / "a1").mkdir()
    path = attachment.retain_serving("pilot")
    assert path.parent.is_dir()
    with pytest.raises(fx.FixtureBlocked, match="did not write"):
        attachment.record_serving("pilot", ["producer"])
    assert not path.parent.exists()
    # A leftover directory without an identity (a producer killed mid-way) never blocks the name.
    path.parent.mkdir(parents=True)
    path = attachment.retain_serving("pilot")
    attachment.release()  # the producer failed; release abandons the unrecorded retention
    assert not path.parent.exists()
    (tmp_path / "serving" / "pilot").mkdir(parents=True)
    (tmp_path / "serving" / "pilot" / "identity.json").write_text("{}")
    other = fx.Attachment(server, "a2", tmp_path / "a2", {"namespace": "ns"})
    with pytest.raises(fx.FixtureBlocked, match="already retained"):
        other.retain_serving("pilot")


def test_setup_errors_are_classifiable_readiness_evidence() -> None:
    with pytest.raises(fx.FixtureBlocked) as raised, fx._setup("defining namespace"):
        raise fx.urllib.error.URLError("connection refused")
    assert raised.value.kind == "readiness" and "URLError" in raised.value.detail
    with pytest.raises(fx.FixtureBlocked) as raised, fx._setup("x"):
        raise fx.FixtureBlocked("oom", "kept")
    assert raised.value.kind == "oom"


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


def fixture_units(fixture_id: str) -> str:
    listed = fx.systemctl("list-units", "--all", "--plain", "--no-legend", f"*{fixture_id}*")
    return listed.stdout.strip()


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
    assert fixture_id in fx.sweep(report=lambda _m: None)
    assert not fixture_units(fixture_id)


@live
def test_cap_below_startup_rss_is_reported_as_oom_during_readiness(root: Path) -> None:
    launcher = cli("--memory", "150M", "--", "true")
    _out, err = launcher.communicate(timeout=120)
    assert launcher.returncode == fx.EXIT_BLOCKED
    assert "blocked: oom" in err and "during readiness" in err
    assert list(root.iterdir()) == []
    fixture_id = err.split("server ")[1].split()[0]
    assert not fixture_units(fixture_id)


@live
def test_run_owned_fixture_leaves_no_unit_and_restarts_on_its_port(root: Path) -> None:
    with fx.fixture(sweep_first=False, report=lambda _m: None) as attachment:
        server = attachment.server
        port, unit = server.port, server.unit
        attachment.restart()  # the serving:mcp journey restarts its run-owned fixture
        assert server.port == port and server.exited() is None
        assert server.query("RETURN 1;")[0]["result"] == 1
        assert fixture_units(server.id)
    assert not fixture_units(server.id), unit
    assert list(root.iterdir()) == []


@pytest.mark.parametrize("failure", ["identity", "receipt"])
def test_attached_post_spawn_failure_cleans_actual_process(tmp_path, monkeypatch, failure):
    from types import SimpleNamespace

    from harness import group_members

    spawned = []
    original_spawn = fx.spawn_group

    def spawn(*args, **kwargs):
        child = original_spawn(*args, **kwargs)
        spawned.append(child)
        return child

    def write_record(_record, **extra):
        if failure == "receipt" and _record and _record.get("leader"):
            raise OSError("injected attachment receipt error")

    attachment = SimpleNamespace(environment=lambda: {}, _write_record=write_record)
    monkeypatch.setattr(fx, "spawn_group", spawn)
    if failure == "identity":
        monkeypatch.setattr(
            fx.ProcessIdentity,
            "of",
            classmethod(lambda cls, pid=None: (_ for _ in ()).throw(OSError("identity"))),
        )
    with pytest.raises(OSError):
        fx.run_attached(attachment, ["sleep", "120"], cwd=tmp_path)
    assert spawned and group_members(spawned[0].pid) == []
    assert spawned[0].returncode is not None


@pytest.mark.parametrize("failure", ["survivor", "receipt", "identity"])
def test_failed_fixture_cleanup_retains_actual_group_and_recovers_selected(
    root, no_systemd, monkeypatch, failure
):
    import harness

    directory = write_fixture(root, "recovery", "run", dead_identity())
    server = fx.Server.open("recovery")
    server.report = lambda message: None
    path = directory / "attachments" / "a1"
    path.mkdir(parents=True)
    attachment = fx.Attachment(server, "a1", path, {"namespace": "ns"})
    attachment._write_record(None)
    original_signal = harness.signal_group
    original_write = attachment._write_record
    original_identity = fx.ProcessIdentity.of
    monkeypatch.setattr(harness, "signal_group", lambda *args, **kwargs: False)
    if failure == "receipt":
        calls = 0

        def write(*args, **kwargs):
            nonlocal calls
            if args[0] and args[0].get("leader"):
                calls += 1
            if calls == 1 and args[0] and args[0].get("leader"):
                raise OSError("initial receipt failure")
            return original_write(*args, **kwargs)

        monkeypatch.setattr(attachment, "_write_record", write)
    elif failure == "identity":
        calls = 0

        def identity(cls, pid=None):
            nonlocal calls
            calls += 1
            if calls == 1:
                raise OSError("initial identity failure")
            return original_identity(pid)

        monkeypatch.setattr(fx.ProcessIdentity, "of", classmethod(identity))
    record = None
    try:
        if failure == "survivor":
            command = [sys.executable, "-c", "import subprocess; subprocess.Popen(['sleep','120'])"]
            assert fx.run_attached(attachment, command, report=lambda message: None) == 1
        else:
            with pytest.raises(OSError, match="initial"):
                fx.run_attached(attachment, ["sleep", "120"], report=lambda message: None)
        record = json.loads((path / "record.json").read_text())
        assert record["command_cleanup"]["status"] == "failed"
        assert harness.group_members(record["command"]["leader"]["pid"])
        if failure == "survivor":
            assert record["child_exit_code"] == 0
        attachment.release()
        assert path.is_dir() and server.destroy() == "cleanup-unresolved"
        before = (path / "record.json").read_bytes()
        with pytest.raises(fx.FixtureFailed, match="prior command cleanup unresolved"):
            fx.run_attached(attachment, ["true"])
        assert (path / "record.json").read_bytes() == before
        assert fx.sweep(report=lambda message: None) == []
        assert (path / "record.json").read_bytes() == before
        assert fx.main(["--stop", "recovery", "--force"]) == 1
        assert path.is_dir() and directory.is_dir()
        monkeypatch.setattr(harness, "signal_group", original_signal)
        assert fx.main(["--stop", "recovery", "--force"]) == 0
        assert not directory.exists()
        assert harness.group_members(record["command"]["leader"]["pid"]) == []
    finally:
        monkeypatch.setattr(harness, "signal_group", original_signal)
        if record:
            original_signal(record["command"]["leader"]["pid"])


def test_unknown_legacy_fixture_cleanup_refuses_force_and_preserves_record(root, no_systemd):
    directory = write_fixture(root, "legacy_unknown", "run", dead_identity())
    path = directory / "attachments" / "a1"
    path.mkdir(parents=True)
    record = {"id": "a1", "command": {"argv": ["old-command"]}, "retained": "preserved"}
    fx.write_json_atomic(path / "record.json", record)
    before = (path / "record.json").read_bytes()
    assert fx.sweep(report=lambda message: None) == []
    assert (path / "record.json").read_bytes() == before
    assert fx.main(["--stop", "legacy_unknown", "--force"]) == 1
    recovered = json.loads((path / "record.json").read_text())
    assert recovered["retained"] == "preserved"
    assert (
        recovered["command"] == record["command"]
        and recovered["command_cleanup"]["status"] == "unknown"
    )


@pytest.mark.parametrize("identity_problem", ["foreign", "reused"])
def test_force_recovery_never_signals_unowned_actual_group(root, no_systemd, identity_problem):
    directory = write_fixture(root, "unowned", "run", dead_identity())
    path = directory / "attachments" / "a1"
    path.mkdir(parents=True)
    child = subprocess.Popen(["sleep", "120"], start_new_session=True)
    try:
        identity = {**ProcessIdentity.of(child.pid).to_json(), "pgid": child.pid}
        if identity_problem == "foreign":
            identity["pid_namespace"] += 1
        else:
            identity["start_ticks"] -= 1
        fx.write_json_atomic(
            path / "record.json",
            {"command": {"leader": identity}, "command_cleanup": {"status": "unknown"}},
        )
        assert fx.main(["--stop", "unowned", "--force"]) == 1
        assert child.poll() is None and path.is_dir()
        assert (
            json.loads((path / "record.json").read_text())["command_cleanup"]["status"] == "unknown"
        )
    finally:
        child.kill()
        child.wait()


def test_recovery_locks_freeze_attachments_and_preserve_live_owner_records(root, no_systemd):
    import os

    directory = write_fixture(root, "locked", "kept", dead_identity())
    path = directory / "attachments" / "a1"
    path.mkdir(parents=True)
    fx.write_json_atomic(
        path / "record.json", {"id": "a1", "command_cleanup": {"status": "confirmed"}}
    )
    before = (path / "record.json").read_bytes()
    server_lock = fx.hold_lock(directory / "owner.lock")
    try:
        assert fx.main(["--stop", "locked", "--force"]) == 1
        with pytest.raises(fx.FixtureBlocked, match="recovery is active"):
            fx.Attachment.create(fx.Server.open("locked"))
        assert (path / "record.json").read_bytes() == before
    finally:
        os.close(server_lock)
    attachment_lock = fx.hold_lock(path / "owner.lock")
    try:
        assert fx.main(["--stop", "locked", "--force"]) == 1
        assert (path / "record.json").read_bytes() == before
    finally:
        os.close(attachment_lock)
    assert fx.main(["--stop", "locked", "--force"]) == 0
    assert not directory.exists()


def test_incomplete_sweep_preserves_receipts_and_evidence_under_current_lock(
    root, no_systemd, monkeypatch
):
    import os

    for name in ("empty", "held", "receipt", "evidence", "appearing_receipt"):
        path = root / name
        path.mkdir()
        if name == "receipt":
            (path / "record.json").write_text("unreadable receipt")
        if name == "evidence":
            (path / "attachments").mkdir()
        os.utime(path, (0, 0))
    lock = fx.hold_lock(root / "held" / "owner.lock")
    os.utime(root / "held", (0, 0))
    # Opening the old lock may race publication of the initial receipt. The fresh
    # locked directory, rather than the inventory's missing record, decides deletion.
    original = fx.try_hold_lock

    def acquire(path):
        descriptor = original(path)
        if path.parent.name == "appearing_receipt":
            (path.parent / "record.json").write_text("new recovery evidence")
        return descriptor

    monkeypatch.setattr(fx, "try_hold_lock", acquire)
    try:
        assert fx.sweep(report=lambda message: None) == ["empty (incomplete)"]
        assert sorted(path.name for path in root.iterdir()) == [
            "appearing_receipt",
            "evidence",
            "held",
            "receipt",
        ]
    finally:
        os.close(lock)


def test_kept_sweep_rereads_attachment_after_lock_and_excludes_stop(root, no_systemd, monkeypatch):
    import os

    directory = write_fixture(root, "swept_kept", "kept", dead_identity())
    for name in ("held", "changed", "idle"):
        path = directory / "attachments" / name
        path.mkdir(parents=True)
        fx.write_json_atomic(path / "record.json", {"owner": dead_identity()})
    held = fx.hold_lock(directory / "attachments" / "held" / "owner.lock")
    original = fx.try_hold_lock

    def acquire(path):
        assert original(directory / "owner.lock") is None  # sweep owns SH throughout
        if path.parent.name == "changed":
            fx.write_json_atomic(
                path.parent / "record.json", {"command_cleanup": {"status": "unknown"}}
            )
        return original(path)

    monkeypatch.setattr(fx, "try_hold_lock", acquire)
    try:
        assert fx.sweep(report=lambda message: None) == ["swept_kept/idle"]
        assert sorted(path.name for path in (directory / "attachments").iterdir()) == [
            "changed",
            "held",
        ]
        assert (
            json.loads((directory / "attachments" / "changed" / "record.json").read_text())[
                "command_cleanup"
            ]["status"]
            == "unknown"
        )
    finally:
        os.close(held)


def test_run_sweep_recovers_only_under_fixture_and_attachment_locks(root, no_systemd, monkeypatch):
    directory = write_fixture(root, "swept_run", "run", dead_identity())
    path = directory / "attachments" / "a1"
    path.mkdir(parents=True)
    fx.write_json_atomic(path / "record.json", {"command": {"leader": dead_identity()}})
    original = fx._recover_attachment
    recovered = []

    def recover(attachment, record):
        assert fx.try_hold_lock(directory / "owner.lock") is None
        assert fx.try_hold_lock(attachment / "owner.lock") is None
        recovered.append(attachment)
        return original(attachment, record)

    monkeypatch.setattr(fx, "_recover_attachment", recover)
    assert fx.sweep(report=lambda message: None) == ["swept_run"]
    assert recovered == [path]


@pytest.mark.parametrize("operation", ["create", "destroy"])
@pytest.mark.parametrize("replacement", ["lock", "record"])
def test_stale_fixture_lock_or_record_cannot_create_or_destroy_replacement(
    root, no_systemd, monkeypatch, operation, replacement
):
    directory = write_fixture(root, "replacement", "kept", dead_identity())
    (directory / "owner.lock").touch()
    server = fx.Server.open("replacement")
    flock = fx.fcntl.flock
    changed = []

    def race(descriptor, mode):
        flock(descriptor, mode)
        if changed:
            return
        changed.append(True)
        if replacement == "lock":
            (directory / "owner.lock").rename(directory / "old-owner.lock")
            (directory / "owner.lock").touch()
        else:
            record = json.loads((directory / "record.json").read_text())
            record["created"] = "replacement identity"
            fx.write_json_atomic(directory / "record.json", record)

    monkeypatch.setattr(fx.fcntl, "flock", race)
    monkeypatch.setattr(
        fx.Attachment, "_create_owned", lambda server: pytest.fail("stale attachment creation")
    )
    if operation == "create":
        with pytest.raises(fx.FixtureBlocked, match="ownership path or record changed"):
            fx.Attachment.create(server)
    else:
        assert server.destroy(recover=True) == "cleanup-unresolved"
    assert changed and directory.is_dir() and not (directory / "attachments").exists()
    assert not any(call[0] == "stop" for call in no_systemd)
    if replacement == "record":
        assert (
            json.loads((directory / "record.json").read_text())["created"] == "replacement identity"
        )


def test_launch_intent_precedes_spawn_and_write_failure_never_spawns(root, no_systemd, monkeypatch):
    directory = write_fixture(root, "intent", "run", dead_identity())
    path = directory / "attachments" / "a1"
    path.mkdir(parents=True)
    attachment = fx.Attachment(fx.Server.open("intent"), "a1", path, {"namespace": "ns"})
    attachment._write_record(None)
    write, spawn = attachment._write_record, fx.spawn_group
    spawned = []

    def checked_spawn(*args, **kwargs):
        intent = json.loads((path / "record.json").read_text())
        assert intent["command_cleanup"]["status"] == "unknown"
        assert intent["command"]["leader"] is None
        spawned.append(args)
        return spawn(*args, **kwargs)

    monkeypatch.setattr(fx, "spawn_group", checked_spawn)

    def refuse_write(*args, **kwargs):
        raise OSError("intent publication refused")

    monkeypatch.setattr(attachment, "_write_record", refuse_write)
    with pytest.raises(OSError, match="intent publication"):
        fx.run_attached(attachment, ["true"])
    assert not spawned
    monkeypatch.setattr(attachment, "_write_record", write)
    with pytest.raises(fx.FixtureBlocked, match="cannot start"):
        fx.run_attached(attachment, [str(path / "missing-executable")])
    assert len(spawned) == 1
    assert (
        json.loads((path / "record.json").read_text())["command_cleanup"]["status"] == "confirmed"
    )
    attachment.release()
    attachment.server.destroy()
    assert not directory.exists()


def test_inventory_protects_unreadable_and_pending_records_independent_of_server(root, no_systemd):
    incomplete = root / "incomplete"
    incomplete.mkdir()
    for name in ("unreadable", "pending", "idle"):
        directory = write_fixture(root, name, "run", dead_identity())
        if name != "idle":
            path = directory / "attachments" / "a1"
            path.mkdir(parents=True)
            if name == "pending":
                fx.write_json_atomic(
                    path / "record.json", {"command_cleanup": {"status": "unknown"}}
                )
    rows = {row["id"]: row for row in fx.inventory()}
    assert all(rows[name]["protected"] for name in ("incomplete", "unreadable", "pending"))
    assert rows["idle"]["protected"] is False


def test_previous_boot_launch_intent_is_known_gone(root, no_systemd):
    previous = {**ProcessIdentity.of().to_json(), "boot_id": "previous-boot"}
    directory = write_fixture(root, "prior_intent", "run", previous)
    path = directory / "attachments" / "a1"
    path.mkdir(parents=True)
    fx.write_json_atomic(
        path / "record.json",
        {
            "owner": previous,
            "command": {"argv": ["old"], "leader": None},
            "command_cleanup": {"status": "unknown", "reason": "launch pending"},
        },
    )
    assert fx.sweep(report=lambda message: None) == ["prior_intent"]


@pytest.mark.parametrize(
    "ended,child,outcome,exit_code",
    [
        (None, 0, "passed", 0),
        (None, 127, "failed", 127),
        ("oom-kill", 9, "blocked", 75),
        ("result resources", 0, "blocked", 75),
        ("signal 9", 0, "failed", 1),
        ("exit 0", 37, "failed", 37),
    ],
)
def test_shared_server_end_preserves_child_and_unknown_cause(
    tmp_path, monkeypatch, ended, child, outcome, exit_code
):
    server = fx.Server(tmp_path, {"id": "f1", "memory_max": 1 << 30}, "secret")
    monkeypatch.setattr(fx.Server, "exited", lambda self: ended)
    observed = fx.server_end_outcome(server, child)
    assert observed["outcome"] == outcome and observed["exit_code"] == exit_code
    assert observed["child_exit_code"] == child
    assert fx._finish(server, child, lambda message: None) == exit_code
    if ended in ("signal 9", "exit 0"):
        assert observed["cause"]["kind"] == "server_end_unknown"
    cancelled = fx.server_end_outcome(server, child, cancelled=True)
    assert cancelled["outcome"] == "not_run" and cancelled["termination"] == "cancelled"


def test_sql_diagnostic_distinguishes_transport_and_indexed_statement_errors(tmp_path, monkeypatch):
    import io
    import urllib.error

    server = fx.Server(tmp_path, {"port": 1}, "credential-secret")

    class Response(io.BytesIO):
        status = 200

    body = [
        {"status": "OK", "result": "a legitimate string"},
        {"status": "ERR", "kind": "secret-kind", "result": "query secret-bind credential-secret"},
        {"status": "OK", "result": 3},
    ]
    monkeypatch.setattr(
        fx.urllib.request, "urlopen", lambda *a, **kw: Response(json.dumps(body).encode())
    )
    observed = server.diagnose("secret-query")
    assert observed["transport"]["status"] == "OK" and observed["outcome"] == "failed"
    assert [r["index"] for r in observed["statements"]] == [0, 1, 2]
    assert observed["statements"][0]["result"] == "a legitimate string"
    assert "result" not in observed["statements"][1]
    with pytest.raises(fx.FixtureQueryError) as raised:
        server.query("secret-query")
    assert "1:statement_refused" in str(raised.value)
    assert "secret" not in json.dumps(observed) + str(raised.value)

    def refused(*args, **kwargs):
        raise urllib.error.HTTPError(
            "secret-query", 400, "secret-bind", {}, io.BytesIO(b"credential-secret")
        )

    monkeypatch.setattr(fx.urllib.request, "urlopen", refused)
    observed = server.diagnose("secret-query")
    assert observed["transport"] == {
        "status": "ERR",
        "category": "http_refused",
        "http_status": 400,
    }
    assert not observed["statements"] and "secret" not in json.dumps(observed)

    def disconnected(*args, **kwargs):
        import http.client

        raise http.client.IncompleteRead(b"credential-secret")

    monkeypatch.setattr(fx.urllib.request, "urlopen", disconnected)
    observed = server.diagnose("secret-query")
    assert observed["transport"] == {"status": "ERR", "category": "connection"}
    assert "secret" not in json.dumps(observed)


def test_unknown_readiness_end_passes_through_setup_as_failed(tmp_path, monkeypatch):
    server = fx.Server(tmp_path, {"id": "f1"}, "secret")
    monkeypatch.setattr(fx.Server, "exited", lambda self: "signal 15")
    monkeypatch.setattr(fx.Server, "_log_tail", lambda *args: "private native error")
    with pytest.raises(fx.FixtureFailed) as raised, fx._setup("fixture startup"):
        server.ready()
    assert raised.value.kind == "server_end_unknown"
    assert "unknown" in raised.value.detail and "private" not in raised.value.detail


def test_checked_retained_inspection_preserves_configs_and_rejects_changed_content(
    tmp_path, monkeypatch
):
    server = fx.Server(
        tmp_path,
        {"id": "f1", "kind": "kept", "port": 1, "binary": {"sha256": fx.BINARY_SHA256}},
        "secret",
    )
    monkeypatch.setattr(fx.Server, "exited", lambda self: None)
    monkeypatch.setattr(fx.Server, "ready", lambda self: None)
    monkeypatch.setattr(
        fx.Server,
        "query",
        lambda *args, **kwargs: [{"result": {"databases": {"core": "DEFINE DATABASE core"}}}],
    )
    directory = tmp_path / "serving" / "synthetic"
    directory.mkdir(parents=True)
    config = directory / "viewer.json"
    selection = directory / "selected.json"
    config.write_text(json.dumps({"selection": str(selection)}))
    selection.write_text(json.dumps({"synthetic": "non-sensitive"}))
    identity = {
        "fixture": "f1",
        "name": "synthetic",
        "configuration": {
            "path": str(config),
            "sha256": fx.file_sha256(config),
            "selection": str(selection),
            "selection_sha256": fx.file_sha256(selection),
            "server": {"binary_sha256": fx.BINARY_SHA256},
        },
        "content": {"database": {"namespace": "retained", "database": "core"}},
    }
    fx.write_json_atomic(directory / "identity.json", identity)
    before = {path: path.read_bytes() for path in directory.iterdir()}
    assert fx.inspection_scope(server, config) == ("retained", "core")
    assert all(path.read_bytes() == data for path, data in before.items())
    selection.write_text("changed synthetic content")
    with pytest.raises(fx.FixtureBlocked, match="changed or vanished"):
        fx.inspection_scope(server, config)


@live
def test_disposable_diagnostics_scope_syntax_and_native_mcp(root: Path, tmp_path):
    """One revealing owned fixture: useful inspection, mixed outcomes and real MCP semantics."""
    import urllib.error
    import urllib.request

    binary = fx.verify_binary()
    for sql, accepted in (("RETURN 1;", True), ("SELECT FROM ;", False)):
        checked = subprocess.run(
            [str(binary), "validate", "--stdin"], input=sql, capture_output=True, text=True
        )
        assert (checked.returncode == 0) is accepted
    with fx.fixture(sweep_first=False, report=lambda message: None) as attachment:
        server, ns = attachment.server, attachment.namespace

        def query(sql):
            return server.diagnose(sql, namespace=ns, database="core")

        assert (
            query(
                "DEFINE TABLE probe SCHEMAFULL; DEFINE FIELD value ON probe TYPE int; "
                "DEFINE INDEX probe_value ON probe FIELDS value; CREATE probe:one SET value=1;"
            )["outcome"]
            == "passed"
        )
        assert fx.inspection_scope(server, attachment.config_path) == (ns, "core")
        info = query("INFO FOR TABLE probe;")
        assert "probe_value" in info["statements"][0]["result"]["indexes"]
        assert query("SELECT * FROM probe WHERE value=1 LIMIT 1 EXPLAIN;")["outcome"] == "passed"
        mixed = query("RETURN 1; CREATE probe:bad SET value='non-sensitive-invalid'; RETURN 3;")
        assert [row["status"] for row in mixed["statements"]] == ["OK", "ERR", "OK"]
        assert "non-sensitive-invalid" not in json.dumps(mixed)
        malformed = query("SELECT FROM ;")
        assert malformed["transport"]["category"] == "http_refused"
        other = fx.Attachment.create(server)
        try:
            assert other.namespace != ns
            assert (
                server.diagnose("SELECT * FROM probe;", namespace=other.namespace, database="core")[
                    "outcome"
                ]
                == "failed"
            )
        finally:
            other.release()
        with pytest.raises(fx.FixtureBlocked, match=r"not owned|no longer owned"):
            fx.inspection_scope(server, other.config_path)
        with pytest.raises(fx.FixtureBlocked, match="not owned"):
            fx.inspection_scope(server, tmp_path / "operator.json")

        command, child_env = fx.mcp_invocation(
            attachment, ["mcp", "get", "lctx_fixture", "--json"], non_sensitive=True
        )
        assert "--no-daemon" in command
        assert server.password not in json.dumps(command)
        assert fx.MCP_AUTH_ENV not in __import__("os").environ
        configured = subprocess.run(
            command, env=attachment.environment() | child_env, capture_output=True, text=True
        )
        assert configured.returncode == 0, configured.stderr
        assert server.endpoint + "/mcp" in configured.stdout
        assert child_env[fx.MCP_AUTH_ENV] not in configured.stdout + configured.stderr

        # Exercise the installed native client discovery without a generative turn. Selecting
        # one server avoids connecting unrelated registrations; invocation overrides do not
        # mutate or clear the user's configured inventory.
        native_command, native_env = fx.mcp_invocation(
            attachment, ["app-server", "--stdio"], non_sensitive=True
        )
        native = subprocess.Popen(
            native_command,
            env=attachment.environment() | native_env,
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
            text=True,
            start_new_session=True,
        )
        with fx.SpawnGuard(native) as native_guard:

            def native_rpc(identifier, method, params):
                native.stdin.write(
                    json.dumps({"id": identifier, "method": method, "params": params}) + "\n"
                )
                native.stdin.flush()
                while True:
                    line = native.stdout.readline()
                    assert line, "native Codex client ended before its response"
                    message = json.loads(line)
                    if message.get("id") == identifier:
                        assert "error" not in message, message
                        return message["result"]

            native_rpc(
                1,
                "initialize",
                {
                    "clientInfo": {"name": "disposable-fixture-control", "version": "1"},
                    "capabilities": {"experimentalApi": True},
                },
            )
            native.stdin.write(json.dumps({"method": "initialized"}) + "\n")
            native.stdin.flush()
            status = native_rpc(
                2,
                "mcpServerStatus/list",
                {"serverName": "lctx_fixture", "detail": "toolsAndAuthOnly"},
            )
            assert len(status["data"]) == 1 and status["data"][0]["name"] == "lctx_fixture"
            assert {"query", "info", "list"} <= set(status["data"][0]["tools"])
            assert child_env[fx.MCP_AUTH_ENV] not in json.dumps(status)
            native.stdin.close()
            while fx.observe_exit(native) is None:
                time.sleep(0.05)
        assert native.returncode == 0
        assert native_guard.cleanup["status"] == "confirmed"

        session = None
        serial = 0

        def rpc(method, params=None, *, authorization=None, notification=False):
            nonlocal session, serial
            serial += 1
            payload = {"jsonrpc": "2.0", "method": method}
            if not notification:
                payload["id"] = serial
            if params is not None:
                payload["params"] = params
            request = server.request("/mcp", json.dumps(payload).encode(), None, None)
            request.add_header("Content-Type", "application/json")
            request.add_header("Accept", "application/json, text/event-stream")
            if session:
                request.add_header("Mcp-Session-Id", session)
            if authorization:
                request.add_header("Authorization", authorization)
            with urllib.request.urlopen(request) as response:
                session = response.headers.get("Mcp-Session-Id") or session
                body = response.read().decode()
            if not body:
                return None
            lines = [line[6:] for line in body.splitlines() if line.startswith("data: ")]
            return json.loads(lines[-1] if lines else body)

        initialized = rpc(
            "initialize",
            {
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": {"name": "disposable-fixture-control", "version": "1"},
            },
        )
        assert initialized["result"]["capabilities"]["tools"] is not None
        rpc("notifications/initialized", notification=True)
        tools = rpc("tools/list")["result"]["tools"]
        assert {"query", "info", "list"} <= {tool["name"] for tool in tools}
        scope = {"namespace": ns, "database": "core"}
        info = rpc("tools/call", {"name": "info", "arguments": {"target": "probe", **scope}})[
            "result"
        ]
        assert info.get("isError") is not True
        explain = rpc(
            "tools/call",
            {
                "name": "query",
                "arguments": {"query": "SELECT * FROM probe LIMIT 1 EXPLAIN;", **scope},
            },
        )["result"]
        assert explain.get("isError") is not True
        mixed = rpc(
            "tools/call",
            {
                "name": "query",
                "arguments": {"query": "RETURN 1; THROW 'synthetic-error'; RETURN 3;", **scope},
            },
        )["result"]
        assert mixed.get("isError") is not True
        assert mixed["structuredContent"]["has_errors"] is True
        assert [row["status"] for row in mixed["structuredContent"]["results"]] == [
            "ok",
            "error",
            "ok",
        ]
        with pytest.raises(urllib.error.HTTPError) as refused:
            rpc("tools/list", authorization="Basic aW52YWxpZDppbnZhbGlk")
        assert refused.value.code in (401, 403)
        assert query("RETURN 1;")["outcome"] == "passed"
        endpoint = server.endpoint
    assert list(root.iterdir()) == []
    with pytest.raises(urllib.error.URLError):
        urllib.request.urlopen(endpoint + "/ready")


@live
def test_native_mcp_launcher_interruption_removes_child_fixture_and_auth(root: Path):
    launcher = cli(
        "--requires",
        "none",
        "--mcp",
        "--non-sensitive",
        "--",
        "app-server",
        "--stdio",
        stdin=subprocess.PIPE,
    )
    assert (
        launcher.stderr is not None and launcher.stdin is not None and launcher.stdout is not None
    )
    try:
        line = launcher.stderr.readline()
        assert " ready: " in line, line
        fixture_id = line.split()[1]
        while "fixture MCP:" not in launcher.stderr.readline():
            assert launcher.poll() is None
        launcher.stdin.write(
            json.dumps(
                {
                    "id": 1,
                    "method": "initialize",
                    "params": {
                        "clientInfo": {"name": "disposable-interrupt-control", "version": "1"}
                    },
                }
            )
            + "\n"
        )
        launcher.stdin.flush()
        assert json.loads(launcher.stdout.readline())["id"] == 1
        record = next((root / fixture_id / "attachments").glob("*/record.json"))
        argv = json.loads(record.read_text())["command"]["argv"]
        assert fx.MCP_AUTH_ENV in " ".join(argv)
        assert "Basic " not in " ".join(argv)
        launcher.send_signal(signal.SIGINT)
        _stdout, _stderr = launcher.communicate()
        assert launcher.returncode == 130
        assert not fixture_units(fixture_id) and not server_processes(fixture_id)
        assert list(root.iterdir()) == []
    finally:
        if launcher.poll() is None:
            launcher.send_signal(signal.SIGTERM)
            launcher.wait()


@live
def test_unknown_native_server_end_is_failed_with_actual_child_exit(root: Path):
    with (
        pytest.raises(RuntimeError, match="cause unknown"),
        fx.fixture(sweep_first=False, report=lambda message: None) as attachment,
    ):
        server = attachment.server
        child_code = fx.run_attached(attachment, [sys.executable, "-c", "raise SystemExit(37)"])
        assert child_code == 37
        server.process.terminate()
        server.process.wait()
        observed = fx.server_end_outcome(server, child_code)
        assert observed["outcome"] == "failed" and observed["cause"]["kind"] == "server_end_unknown"
        assert observed["child_exit_code"] == observed["exit_code"] == 37
    assert list(root.iterdir()) == []
