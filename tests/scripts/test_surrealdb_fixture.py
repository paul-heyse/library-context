"""Shared validation attachment and truthful child ownership controls (no disposable DBs)."""

from __future__ import annotations

import io
import json
import os
import sys
import uuid
from types import SimpleNamespace

import pytest

import surrealdb_fixture as fx
import surrealdb_service as service
from harness import group_members


@pytest.fixture
def installed(tmp_path, monkeypatch):
    monkeypatch.setenv("LCTX_STORAGE_STATE", str(tmp_path / "storage"))
    root = tmp_path / "service"
    root.mkdir()
    (root / "attachments").mkdir()
    (root / "serving").mkdir()
    (root / "admission.lock").touch()
    record = {
        "schema": 1,
        "phase": "ready",
        "installation_id": str(uuid.uuid4()),
        "service_generation": list(range(32)),
        "unit": service.UNIT,
        "namespace": service.NAMESPACE,
        "databases": ["main", "validation"],
        "export_batch_size": 1,
        "state_root": str(root),
        "endpoint": "http://127.0.0.1:29000",
        "grpc_endpoint": "grpc://127.0.0.1:29000",
        "binary": {
            "path": "/unavailable",
            "sha256": service.BINARY_SHA256,
            "version": service.VERSION,
        },
        "installer": "/unavailable/lctx",
    }
    service.private_json(root / "installation.json", record)
    for database in service.DATABASES:
        cfg = {
            "endpoint": record["grpc_endpoint"],
            "namespace": service.NAMESPACE,
            "database": database,
            "cache_database": database,
            "service_generation": record["service_generation"],
            "authentication": "database",
            "username": f"{database}_writer",
            "password": "private-password",
            "viewer_username": f"{database}_viewer",
            "viewer_password": "viewer-password",
            "selection": str(root / f"{database}-selected.json"),
            "reuse": {
                "database": database,
                "capacity_bytes": service.PRODUCT_CAPACITY_BYTES,
                "lease_directory": str(root / "product-leases" / database),
            },
        }
        service.private_json(root / f"{database}-runtime.json", cfg)
    monkeypatch.setenv("LCTX_SURREAL_SERVICE_CONFIG", str(root / "installation.json"))
    monkeypatch.setattr(service.Installation, "check", lambda self, **kw: {"outcome": "passed"})
    monkeypatch.setattr(service, "_run_installer", lambda *a, **kw: None)
    return fx.Server(service.Installation.load())


def test_missing_installation_blocks_without_provisioning(tmp_path, monkeypatch):
    root = tmp_path / "absent"
    monkeypatch.setenv("LCTX_SURREAL_SERVICE_CONFIG", str(root / "installation.json"))
    ready = fx.substrate_readiness()
    assert not ready.ready and ready.repair.startswith("just service install")
    with pytest.raises(fx.FixtureBlocked, match="no installed service"), fx.fixture():
        pytest.fail("missing installation cannot yield a fixture")
    assert not root.exists()


def test_two_attachments_share_stable_db_and_have_distinct_attempts(installed, monkeypatch):
    queries = []
    monkeypatch.setattr(fx.Server, "query", lambda *a, **kw: queries.append(a))
    first = fx.Attachment.create(installed)
    second = fx.Attachment.create(installed)
    try:
        assert first.id != second.id
        assert first.config["namespace"] == second.config["namespace"] == "library_context"
        assert first.config["database"] == second.config["database"] == "validation"
        assert first.config["authentication"] == "database"
        assert first.environment()["LCTX_SURREAL_ATTEMPT_ID"] == first.id
        compiler = json.loads(first.compiler_config_path.read_text())
        assert compiler["database"] == compiler["cache_database"] == "validation"
        assert compiler["username"] == "validation_writer"
        assert compiler["service_generation"] == list(range(32))
        assert compiler["reuse"] == installed.installation.runtime()["reuse"]
        assert compiler["reuse"]["lease_directory"].startswith(str(installed.directory))
        assert not queries  # Attachment must never execute DDL or create/drop DB state.
        first.release()
        assert second.config_path.exists()
        assert fx.inventory()[0]["unit"] == service.UNIT
    finally:
        first.release()
        second.release()
    assert not service.borrowers(installed.installation)
    assert installed.directory.exists()
    assert first.directory.exists()  # Receipts retained, persisted data never swept.


def test_service_scope_credentials_private_and_identity_checked(installed):
    path = installed.installation.runtime_path()
    assert path.stat().st_mode & 0o777 == 0o600
    cfg = json.loads(path.read_text())
    cfg["database"] = "main"
    service.private_json(path, cfg)
    with pytest.raises(fx.FixtureBlocked, match="does not match"):
        installed.installation.runtime()


def test_fixture_never_exposes_root_or_cross_scope_diagnostics(installed):
    with pytest.raises(fx.FixtureBlocked, match="maintenance"):
        installed.diagnose("REMOVE DATABASE main;")
    with pytest.raises(fx.FixtureBlocked, match="canonical"):
        installed.request("/sql", b"RETURN 1", "library_context", "main")
    request = installed.request("/sql", b"RETURN 1", "library_context", "validation")
    assert request.get_header("Surreal-auth-db") == "validation"
    assert request.get_header("Surreal-auth-ns") == "library_context"
    attachment = fx.Attachment.create(installed)
    try:
        with pytest.raises(fx.FixtureBlocked, match="maintenance"):
            fx.mcp_invocation(attachment, [], non_sensitive=True)
    finally:
        attachment.release()


def test_maintenance_closes_admission_and_does_not_cancel_existing_borrower(installed, monkeypatch):
    attachment = fx.Attachment.create(installed)
    original = service.Installation.check
    monkeypatch.setattr(service.time, "sleep", lambda _: attachment.release())
    with service.maintenance(installed.installation) as env:
        assert (installed.directory / "maintenance.json").exists()
        with pytest.raises(fx.FixtureBlocked, match="admission is closed"):
            fx.Attachment.create(installed)
        assert service.maintenance_owned(installed.installation, env)
        diagnostic = fx.Attachment.create(installed, maintenance_env=env)
        diagnostic.release()
    assert not (installed.directory / "maintenance.json").exists()
    assert original is not None


def test_unknown_dead_borrower_keeps_maintenance_closed(installed):
    attachment = fx.Attachment.create(installed)
    attachment._write_record(None, command_cleanup={"status": "unknown"})
    attachment.release()
    with (
        pytest.raises(fx.FixtureBlocked, match="unreleased borrower"),
        service.maintenance(installed.installation),
    ):
        pytest.fail("unknown cleanup cannot drain")
    assert (installed.directory / "maintenance.json").exists()
    assert fx.inventory()[0]["protected"]


def test_run_attached_records_intent_and_cleanup_before_release(installed, monkeypatch):
    attachment = fx.Attachment.create(installed)
    spawned = []
    spawn = fx.spawn_group

    def checked_spawn(*args, **kwargs):
        row = json.loads((attachment.directory / "record.json").read_text())
        assert row["command"]["leader"] is None
        assert row["command_cleanup"]["status"] == "unknown"
        spawned.append(args)
        return spawn(*args, **kwargs)

    monkeypatch.setattr(fx, "spawn_group", checked_spawn)
    try:
        assert fx.run_attached(attachment, [sys.executable, "-c", "pass"]) == 0
        row = json.loads((attachment.directory / "record.json").read_text())
        assert row["child_exit_code"] == 0 and row["command_cleanup"]["status"] == "confirmed"
    finally:
        attachment.release()
    assert spawned and not service.borrowers(installed.installation)


@pytest.mark.parametrize("failure", ["receipt", "identity"])
def test_attached_post_spawn_failure_cleans_actual_process(tmp_path, monkeypatch, failure):
    spawned = []
    original_spawn = fx.spawn_group

    def spawn(*args, **kwargs):
        child = original_spawn(*args, **kwargs)
        spawned.append(child)
        return child

    def write_record(record, **extra):
        if failure == "receipt" and record and record.get("leader"):
            raise OSError("injected attachment receipt error")

    from typing import cast

    attachment = cast(
        fx.Attachment, SimpleNamespace(environment=lambda: {}, _write_record=write_record)
    )
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


def test_unresolved_child_cleanup_retains_exact_receipt(installed, monkeypatch):
    import harness

    attachment = fx.Attachment.create(installed)
    signal_group = harness.signal_group
    monkeypatch.setattr(harness, "signal_group", lambda *a, **kw: False)
    row = None
    try:
        assert (
            fx.run_attached(
                attachment,
                [sys.executable, "-c", "import subprocess; subprocess.Popen(['sleep','120'])"],
                report=lambda _: None,
            )
            == 1
        )
        row = read = json.loads((attachment.directory / "record.json").read_text())
        assert row["command_cleanup"]["status"] == "failed"
        assert group_members(row["command"]["leader"]["pid"])
        attachment.release()
        assert service.borrowers(installed.installation)
        with pytest.raises(fx.FixtureFailed, match="prior command cleanup unresolved"):
            fx.run_attached(attachment, ["true"])
        assert json.loads((attachment.directory / "record.json").read_text()) == read
    finally:
        monkeypatch.setattr(harness, "signal_group", signal_group)
        if row:
            signal_group(row["command"]["leader"]["pid"])


@pytest.mark.parametrize(
    "ended,child,outcome,code",
    [
        (None, 0, "passed", 0),
        (None, 2, "failed", 2),
        ("oom-kill", 0, "blocked", 75),
        ("signal 9", 2, "failed", 2),
        ("exit 0", 0, "failed", 1),
    ],
)
def test_server_end_preserves_child_and_unknown_cause(
    installed, monkeypatch, ended, child, outcome, code
):
    monkeypatch.setattr(fx.Server, "exited", lambda self: ended)
    observed = fx.server_end_outcome(installed, child)
    assert observed["outcome"] == outcome and observed["exit_code"] == code
    assert observed["child_exit_code"] == child
    cancelled = fx.server_end_outcome(installed, child, cancelled=True)
    assert cancelled["outcome"] == "not_run"


def test_sql_diagnostic_preserves_indexed_errors_without_native_secrets(installed, monkeypatch):
    class Response(io.BytesIO):
        status = 200

    monkeypatch.setattr(service, "maintenance_owned", lambda *a, **kw: True)
    body = [
        {"status": "OK", "result": "legitimate string"},
        {"status": "ERR", "result": "secret-query credential-secret"},
        {"status": "OK", "result": 3},
    ]
    monkeypatch.setattr(
        fx.urllib.request, "urlopen", lambda *a, **kw: Response(json.dumps(body).encode())
    )
    observed = installed._diagnose("secret-query")
    assert observed["outcome"] == "failed"
    assert [r["index"] for r in observed["statements"]] == [0, 1, 2]
    assert observed["statements"][0]["result"] == "legitimate string"
    assert "secret" not in json.dumps(observed)
    with pytest.raises(fx.FixtureQueryError, match="1:statement_refused"):
        installed.query("secret-query")


def test_inventory_filters_worktrees_and_never_stops_service(installed, monkeypatch):
    first = fx.Attachment.create(installed)
    first._write_record(None, checkout="/checkout/a")
    second = fx.Attachment.create(installed)
    second._write_record(None, checkout="/checkout/b")
    try:
        rows = fx.inventory({**os.environ, "LCTX_FIXTURE_CHECKOUT": "/checkout/a"})
        assert [r["id"] for r in rows] == [first.id]
    finally:
        first.release()
        second.release()


def test_old_provisioning_flags_are_not_available():
    for argument in ("--keep", "--stop", "--restart", "--memory", "--sweep"):
        with pytest.raises(SystemExit) as error:
            fx.main([argument])
        assert error.value.code == 2


def test_native_drain_precedes_disruptive_effect_and_reopening(installed, monkeypatch):
    events = []
    monkeypatch.setattr(
        service, "_run_installer", lambda installation, installer, action: events.append(action)
    )
    with service.maintenance(installed.installation):
        events.append("effect")
    assert events == ["close-admission", "drain", "effect", "drain", "open-admission"]
    assert not (installed.directory / "maintenance.json").exists()


def test_refused_native_drain_never_performs_effect_and_keeps_admission_closed(
    installed, monkeypatch
):
    def refuse(*args):
        raise fx.FixtureBlocked("drainage", "native effects remain unresolved")

    monkeypatch.setattr(service, "_run_installer", refuse)
    with (
        pytest.raises(fx.FixtureBlocked, match="native effects"),
        service.maintenance(installed.installation),
    ):
        pytest.fail("disruptive operation must not begin")
    assert (installed.directory / "maintenance.json").exists()


def test_retained_output_name_cannot_be_overwritten_by_another_attempt(installed):
    first = fx.Attachment.create(installed)
    second = fx.Attachment.create(installed)
    try:
        first.retain_serving("retained")
        with pytest.raises(fx.FixtureBlocked, match="already owned"):
            second.retain_serving("retained")
        with pytest.raises(fx.FixtureBlocked, match="belongs to another"):
            second.record_serving("retained", ["true"])
    finally:
        first.release()
        second.release()


def test_restart_releases_producer_before_maintenance_and_creates_fresh_consumer(
    installed, monkeypatch
):
    attachment = fx.Attachment.create(installed)
    old_id = attachment.id
    old_directory = attachment.directory
    events = []
    attachment.extra_env["LCTX_NATIVE_SERVING_CONFIG"] = str(old_directory / "scratch/serving.json")
    monkeypatch.setattr(
        service, "_run_installer", lambda installation, installer, action: events.append(action)
    )

    def restart(server, env=None):
        assert not service.borrowers(server.installation)
        assert service.maintenance_owned(server.installation, env)
        assert json.loads((old_directory / "record.json").read_text())["released"]
        events.append("restart")

    monkeypatch.setattr(fx.Server, "restart", restart)
    try:
        attachment.restart()
        assert events == ["close-admission", "drain", "restart", "drain", "open-admission"]
        assert attachment.id != old_id
        assert attachment.environment()["LCTX_SURREAL_ATTEMPT_ID"] == attachment.id
        assert attachment.extra_env["LCTX_NATIVE_SERVING_CONFIG"] == str(
            old_directory / "scratch/serving.json"
        )
        assert [row["id"] for row in service.borrowers(installed.installation)] == [attachment.id]
    finally:
        attachment.release()


def test_maintenance_native_clients_are_ordinary_and_host_exclusive(installed, monkeypatch):
    events = []
    monkeypatch.setattr(
        service, "_run_installer", lambda installation, installer, action: events.append(action)
    )
    with service.maintenance(installed.installation, native_clients=True) as env:
        assert events == ["close-admission", "drain", "open-admission"]
        assert service.maintenance_owned(installed.installation, env)
        assert env["LCTX_SURREAL_INSTALLER_CONFIG"].endswith("validation-installer.json")
        with pytest.raises(fx.FixtureBlocked, match="admission is closed"):
            fx.Attachment.create(installed)
        owned = fx.Attachment.create(installed, maintenance_env=env)
        assert owned.config["authentication"] == "database"
        events.append("ordinary-client-control")
        owned.release()
    assert events == [
        "close-admission",
        "drain",
        "open-admission",
        "ordinary-client-control",
        "close-admission",
        "drain",
        "open-admission",
    ]
    assert not (installed.directory / "maintenance.json").exists()


def test_failed_native_client_control_drains_and_keeps_admission_closed(installed, monkeypatch):
    events = []
    monkeypatch.setattr(
        service, "_run_installer", lambda installation, installer, action: events.append(action)
    )
    with (
        pytest.raises(RuntimeError, match="control failed"),
        service.maintenance(installed.installation, native_clients=True),
    ):
        raise RuntimeError("control failed")
    assert events == [
        "close-admission",
        "drain",
        "open-admission",
        "close-admission",
        "drain",
        "close-admission",
    ]
    assert (installed.directory / "maintenance.json").exists()


def test_native_client_maintenance_requires_explicit_command():
    for args in (
        ["maintenance", "--native-clients"],
        ["maintenance", "--native-clients", "--restart"],
        ["maintenance", "--native-clients", "--recover"],
    ):
        with pytest.raises(SystemExit) as error:
            service.main(args)
        assert error.value.code == 2


def test_local_attachment_lifecycle_does_not_retire_native_service(installed):
    import storage_owners
    from storage_lifecycle import Storage

    attachment = fx.Attachment.create(installed)
    assert attachment._storage_id is not None
    row = Storage().get(attachment._storage_id)
    assert storage_owners.observe(row)["state"] == "active"
    attachment.release()
    row = Storage().get(row["id"])
    assert storage_owners.observe(row)["state"] == "released"
    assert row["obligations"][f"attachment:{attachment.id}"]["until"]
    assert installed.directory.is_dir()
    assert (installed.directory / "installation.json").is_file()
    assert attachment.directory.is_dir()  # Seven-day local grace; no native retirement.


def test_attachment_recovery_completes_declared_local_obligation_after_process_death(installed):
    import signal

    import storage_owners
    from storage_lifecycle import Storage

    read_fd, write_fd = os.pipe()
    child = os.fork()
    if child == 0:
        os.close(read_fd)
        try:
            attachment = fx.Attachment.create(installed)
            os.write(write_fd, attachment.id.encode())
            os.close(write_fd)
            while True:
                signal.pause()
        finally:
            os._exit(1)
    os.close(write_fd)
    try:
        with os.fdopen(read_fd, "rb") as ready:
            attachment_id = ready.read().decode()
        assert attachment_id
        row = next(row for row in Storage().records() if row["owner"].get("kind") == "attachment")
        assert storage_owners.observe(row)["state"] == "active"
        with pytest.raises(fx.FixtureBlocked, match="live owner"):
            fx.recover(attachment_id)
    finally:
        os.kill(child, signal.SIGKILL)
        os.waitpid(child, 0)
    fx.recover(attachment_id)
    row = Storage().get(row["id"])
    assert storage_owners.observe(row)["state"] == "released"
    assert row["obligations"][f"attachment:{attachment_id}"]["until"]
    assert (installed.directory / "installation.json").is_file()
