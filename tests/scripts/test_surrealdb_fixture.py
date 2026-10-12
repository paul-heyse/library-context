"""Shared validation attachment and truthful child ownership controls (no disposable DBs)."""

from __future__ import annotations

import io
import json
import os
import sys
import uuid
from types import SimpleNamespace
from pathlib import Path

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
    def checked(self, **kw):
        if witness := service.checked_generation(self):
            witness.validate(self)
        else:
            service._installed_binary(self)
        return {"outcome": "passed"}
    monkeypatch.setattr(service.Installation, "check", checked)
    monkeypatch.setattr(service, "verify_binary", lambda env: Path(env["LCTX_SURREAL_BIN"]))
    monkeypatch.setattr(service, "_run_installer", lambda *a, **kw: None)
    return fx.Server(service.Installation.load())


def install_fake_patched_generation(installed, tmp_path, *, cpu_count=7):
    import surrealdb_server as server
    from storage_service import prepare_server
    selected = server.recipe(cpu_count=cpu_count)
    provenance = server.build_provenance(installed.directory, selected,
        "rustc 1.99.0-nightly\ncommit-date: 2026-09-28\nhost: x86_64-unknown-linux-gnu",
        "cargo 1.99.0-nightly", cache_record_sha256="a" * 64)
    executable = tmp_path / "fake-reviewed-server"
    executable.write_text("#!/bin/sh\nprintf '" + selected["pin"]["cli_version"] + "\\n'\n")
    executable.chmod(0o700)
    generation = prepare_server(installed.directory, executable, provenance)
    installed.record["binary"] = service._generation_binary(generation)
    service.private_json(installed.directory / "installation.json", installed.record)
    return generation


def complete_retained_output(attachment):
    viewer = attachment.retain_serving("retained")
    selection = viewer.parent / "selected.json"
    service.private_json(selection, {"semantic": "immutable-semantic", "realization": "immutable-view"})
    service.private_json(viewer, {"selection": str(selection)})
    return attachment.record_serving("retained", ["producer"])


@pytest.mark.parametrize("patched", [False, True])
def test_attachment_retained_inputs_and_readiness_report_verified_actual_server(installed, tmp_path, patched, capfd):
    generation = install_fake_patched_generation(installed, tmp_path) if patched else None
    binary = installed.record["binary"]
    expected = {"version": binary["version"], "binary_sha256": binary["sha256"]}
    if generation:
        expected["generation"] = generation["identity"]
        assert binary["sha256"] != service.BINARY_SHA256
        assert binary["version"] != service.VERSION
    attachment = fx.Attachment.create(installed)
    try:
        assert attachment.config["server"] == expected
        identity = complete_retained_output(attachment)
        assert identity["configuration"]["server"] == identity["inputs"]["server"] == expected
        readiness = fx.substrate_readiness()
        assert readiness.ready
        assert expected["version"] in readiness.detail and expected["binary_sha256"] in readiness.detail
        if generation:
            assert generation["identity"] in readiness.detail
    finally:
        attachment.release()


def test_retained_publication_reuse_records_current_server_change_without_replay(installed, tmp_path, capfd):
    producer = fx.Attachment.create(installed)
    try:
        identity = complete_retained_output(producer)
    finally:
        producer.release()
    immutable = (installed.directory / "serving/retained/identity.json").read_bytes()
    generation = install_fake_patched_generation(installed, tmp_path)
    consumer = fx.Attachment.create(installed)
    try:
        reuse = consumer.use_serving("retained")
        assert reuse["inputs_changed_since_production"]
        assert reuse["current_inputs"]["server"]["generation"] == generation["identity"]
        assert reuse["skipped_obligations"][0]["produced_at"]["server"] == identity["inputs"]["server"]
        assert reuse["content"] == identity["content"]
        assert (installed.directory / "serving/retained/identity.json").read_bytes() == immutable
    finally:
        consumer.release()


def test_attachment_refuses_unverified_owned_server_identity(installed, tmp_path, capfd):
    install_fake_patched_generation(installed, tmp_path)
    installed.record["binary"]["sha256"] = "d" * 64
    with pytest.raises(service.FixtureBlocked, match="owned generation"):
        fx.Attachment.create(installed)
    assert not service.borrowers(installed.installation)


def test_missing_installation_blocks_without_provisioning(tmp_path, monkeypatch):
    root = tmp_path / "absent"
    monkeypatch.setenv("LCTX_SURREAL_SERVICE_CONFIG", str(root / "installation.json"))
    ready = fx.substrate_readiness()
    assert not ready.ready and ready.repair.startswith("just service install")
    with pytest.raises(fx.FixtureBlocked, match="no installed service"), fx.fixture():
        pytest.fail("missing installation cannot yield a fixture")
    assert not root.exists()


def test_attachment_child_setup_refuses_changed_generation(installed):
    attachment = fx.Attachment.create(installed)
    try:
        attachment.environment({})
        changed = {**installed.record, "native_schema_version": 99}
        service.private_json(installed.directory / "installation.json", changed)
        with pytest.raises(fx.FixtureBlocked, match="no longer protected"):
            attachment.environment({})
    finally:
        attachment.release()


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
        fx.service, "local_urlopen", lambda *a, **kw: Response(json.dumps(body).encode())
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


def native_receipt(attachment, *, state="complete", remote="confirmed", failures=None):
    """Protocol fixture only; real native finality requires the Rust producer."""
    record = json.loads((attachment.directory / "record.json").read_text())
    row = {"schema": 1, "kind": "native-process-owner", "owner": "protocol control",
        "attachment": attachment.id, "generation": record["native_drain"]["generation"],
        "process": {"pid": 123, "start_ticks": 456}, "registration": 0,
        "state": state, "signal": "SIGINT", "completion": None}
    if state == "complete":
        row["completion"] = {"local": "terminal", "remote": remote, "storage": [],
            "committed": [{"kind": "accepted effect", "identity": "exact-attempt"}],
            "failures": failures or []}
    path = attachment.directory / "native-drain/123-456-0.json"
    fx.write_json_atomic(path, row)
    return path, row


@pytest.mark.parametrize("state,remote,failures", [
    ("started", "confirmed", []), ("complete", "unknown", []),
    ("complete", "confirmed", [{"step": "session close", "error": "unknown"}]),
])
def test_native_unknown_retains_attachment_despite_confirmed_process_cleanup(
    installed, state, remote, failures
):
    attachment = fx.Attachment.create(installed)
    unrelated = fx.Attachment.create(installed)
    native_receipt(attachment, state=state, remote=remote, failures=failures)
    attachment._write_record(None, command_cleanup={"status": "confirmed"}, child_exit_code=0)
    try:
        attachment.release()
        row = json.loads((attachment.directory / "record.json").read_text())
        assert row["released"] is False
        assert row["child_exit_code"] == 0 and row["command_cleanup"]["status"] == "confirmed"
        assert row["native_completion"]["status"] == "unknown"
        assert unrelated.environment()["LCTX_FIXTURE_ATTACHMENT"] == unrelated.id
        with pytest.raises(fx.FixtureFailed, match="prior command cleanup unresolved"):
            fx.run_attached(attachment, ["true"])
    finally:
        unrelated.release()
        attachment.release()


def test_confirmed_native_completion_releases_and_retains_committed_identity(installed):
    attachment = fx.Attachment.create(installed)
    native_receipt(attachment)
    attachment._write_record(None, command_cleanup={"status": "confirmed"}, child_exit_code=1)
    attachment.release()
    row = json.loads((attachment.directory / "record.json").read_text())
    assert row["released"] is True and row["child_exit_code"] == 1
    assert row["native_completion"]["status"] == "confirmed"
    assert row["native_completion"]["owners"][0]["completion"]["committed"][0]["identity"] == "exact-attempt"


def test_native_receipt_binding_and_hard_stop_without_receipt_remain_unknown(installed):
    attachment = fx.Attachment.create(installed)
    path, row = native_receipt(attachment)
    row["generation"] = "f" * 64
    fx.write_json_atomic(path, row)
    assert fx.native_completion(attachment.directory,
        json.loads((attachment.directory / "record.json").read_text()))["status"] == "unknown"
    path.unlink()
    attachment._write_record(None, command_cleanup={"status": "confirmed"}, child_exit_code=-9)
    attachment.release()
    row = json.loads((attachment.directory / "record.json").read_text())
    assert row["released"] is False
    assert row["native_completion"]["status"] == "not_observed"


def pd02_manifest(attachment, *, table=None, sequence=0, policy="signin", session=None):
    """Finite protocol fixture; this is not actual server/remote-finality evidence."""
    import surrealdb_pd02_recovery as recovery
    table = table or "pd02_" + "a" * 64
    record = fx.read_json(attachment.directory / "record.json")
    manifest = {"schema": 1, "kind": "PD02-wire-request", "attachment": attachment.id,
        "generation": record["native_drain"]["generation"], "namespace": service.NAMESPACE,
        "database": "validation", "table": table, "sequence": sequence,
        "session": None if policy == "attach_session" else session or str(uuid.UUID(int=7)),
        "process": {"pid":123,"start_ticks":456}, "policy": policy,
        "effects": recovery.POLICIES[policy], "live": "absent_after_join",
        "session_policy": "volatile_named_session_after_join", "sql_blake3": "b" * 64}
    path = attachment.directory / "pd02-recovery" / f"{table[5:]}-{sequence}.json"
    recovery.immutable(path, manifest)
    return path, manifest


def pd02_dead_attachment(installed, monkeypatch):
    import surrealdb_pd02_recovery as recovery
    attachment = fx.Attachment.create(installed)
    path, native = native_receipt(attachment, remote="unknown")
    native["owner"] = "PD02 installed fixture wire owner"
    fx.write_json_atomic(path, native)
    attachment._write_record(None, command_cleanup={"status":"confirmed","survivors":[]}, child_exit_code=0)
    pd02_manifest(attachment, policy="attach_session")
    pd02_manifest(attachment, sequence=1, policy="signin")
    pd02_manifest(attachment, sequence=2, policy="use")
    pd02_manifest(attachment, sequence=3, policy="committed_before_disconnect")
    attachment.release()
    row = fx.read_json(attachment.directory / "record.json")
    row["owner"] = {**service.ProcessIdentity.of().to_json(),"pid":99999999,"start_ticks":1}
    fx.write_json_atomic(attachment.directory / "record.json", row)
    monkeypatch.setattr(fx, "_command_observation", lambda row: {"status":"gone"})
    return attachment, path, path.read_bytes()


def test_pd02_manifest_finite_policy_refuses_sql_injection_and_incomplete_session(installed):
    import surrealdb_pd02_recovery as recovery
    attachment = fx.Attachment.create(installed)
    try:
        pd02_manifest(attachment, policy="attach_session")
        row = fx.read_json(attachment.directory / "record.json")
        with pytest.raises(fx.FixtureBlocked, match="incomplete"):
            recovery.manifests(attachment.directory, row)
        path, manifest = pd02_manifest(attachment, sequence=1, policy="signin")
        pd02_manifest(attachment, sequence=2, policy="use")
        assert recovery.manifests(attachment.directory, row)["sessions"] == [str(uuid.UUID(int=7))]
        path.chmod(0o600)
        manifest["table"] += "; REMOVE DATABASE validation"
        fx.write_json_atomic(path, manifest)
        path.chmod(0o400)
        with pytest.raises(fx.FixtureBlocked, match="identity/policy"):
            recovery.manifests(attachment.directory, row)
    finally:
        attachment.release()


@pytest.mark.parametrize("change", ["unknown_policy", "effect", "generation", "symlink", "process"])
def test_pd02_manifest_identity_and_request_policies_are_not_sql_authority(installed, change):
    import surrealdb_pd02_recovery as recovery
    attachment = fx.Attachment.create(installed)
    try:
        pd02_manifest(attachment, policy="attach_session")
        path, row = pd02_manifest(attachment, sequence=1, policy="signin")
        path.chmod(0o600)
        if change == "unknown_policy": row["policy"] = "execute_serialized_sql"
        elif change == "effect": row["effects"] = {"foreign":"absent"}
        elif change == "generation": row["generation"] = "f" * 64
        elif change == "process": row["process"]["pid"] = -1
        else:
            path.unlink()
            path.symlink_to(attachment.directory / "record.json")
        if change != "symlink":
            fx.write_json_atomic(path, row)
            path.chmod(0o400)
        with pytest.raises((fx.FixtureBlocked, ValueError)):
            recovery.manifests(attachment.directory, fx.read_json(attachment.directory / "record.json"))
    finally:
        attachment.release()


def test_pd02_graceful_join_requires_exact_invocation_clean_zero_and_positive_log(installed, monkeypatch):
    import surrealdb_pd02_recovery as recovery
    old = {**service.ProcessIdentity.of().to_json(), "pid":99999999,"start_ticks":1}
    plan = {"daemon":{"invocation":"a"*32,"process":old}}
    commands = []
    state = {"InvocationID":"a"*32,"MainPID":"0","ActiveState":"inactive",
        "Result":"success","ExecMainCode":"1","ExecMainStatus":"0"}
    monkeypatch.setattr(service, "unit_properties", lambda *args: state)
    monkeypatch.setattr(service, "systemctl", lambda *args: commands.append(args))
    monkeypatch.setattr(recovery, "journal_join", lambda *args: {"exact":"positive"})
    assert recovery.joined_exit(installed.installation, plan)["journal"] == {"exact":"positive"}
    assert not commands  # Already exited only qualifies through clean0 + positive join proof.
    for field, wrong in (("ExecMainCode","2"),("ExecMainStatus","9"),("Result","signal"),("InvocationID","b"*32)):
        before = dict(state)
        state[field] = wrong
        with pytest.raises(fx.FixtureBlocked):
            recovery.joined_exit(installed.installation, plan)
        state.clear()
        state.update(before)


def test_pd02_join_signals_without_systemd_stop_or_timeout_abort(installed, monkeypatch):
    import surrealdb_pd02_recovery as recovery
    current = service.ProcessIdentity.of()
    states = iter([
        {"InvocationID":"a"*32,"MainPID":str(current.pid),"ActiveState":"active"},
        {"InvocationID":"a"*32,"MainPID":str(current.pid),"ActiveState":"deactivating"},
    ])
    monkeypatch.setattr(service, "unit_properties", lambda *args: next(states))
    commands = []
    monkeypatch.setattr(service, "systemctl", lambda *args: commands.append(args) or SimpleNamespace(returncode=0))
    monkeypatch.setattr(recovery.time, "sleep", lambda _: (_ for _ in ()).throw(KeyboardInterrupt()))
    with pytest.raises(KeyboardInterrupt):
        recovery.joined_exit(installed.installation, {"daemon":{"invocation":"a"*32,"process":current.to_json()}})
    assert commands == [("kill","--signal=SIGTERM","--kill-whom=main",installed.unit)]


def test_pd02_positive_journal_is_bound_to_invocation_and_unit(installed, monkeypatch):
    import surrealdb_pd02_recovery as recovery
    invocation = "a" * 32
    output = {"_SYSTEMD_INVOCATION_ID":invocation,"_SYSTEMD_USER_UNIT":installed.unit,
        "MESSAGE":"INFO surrealdb::net: Web server stopped. Bye!"}
    commands = []
    monkeypatch.setattr(recovery.subprocess, "run", lambda args, **kw:
        commands.append(args) or SimpleNamespace(returncode=0,stdout=json.dumps(output)+"\n"))
    assert recovery.journal_join(installed.installation, invocation)["entries"] == [output]
    assert "_SYSTEMD_INVOCATION_ID=" + invocation in commands[0]
    output["_SYSTEMD_INVOCATION_ID"] = "b" * 32
    with pytest.raises(fx.FixtureBlocked, match="differs"):
        recovery.journal_join(installed.installation, invocation)


def test_pd02_reconcile_checks_all_owned_effects_and_live_before_deletion(installed, monkeypatch):
    import surrealdb_pd02_recovery as recovery
    table = "pd02_" + "a" * 64
    manifest = {"tables":[table],"effects":{table:{"committed":"present","later":"absent"}}}
    calls = []
    values = iter([{"lives":{}},True,False,["committed"]])
    monkeypatch.setattr(service.Installation, "check_daemon", lambda self: None)
    monkeypatch.setattr(service.Installation, "runtime", lambda self,*a,**kw: {})
    monkeypatch.setattr(service, "sql", lambda inst, sql, **kw:
        calls.append(sql) or [{"status":"OK","result":next(values)}])
    outcomes = recovery.reconcile(installed.installation, manifest, {"pid":1,"start_ticks":2})
    assert outcomes[table]["effects"] == manifest["effects"][table]
    assert calls == [f"INFO FOR TABLE {table}", f"RETURN record::exists({table}:committed)",
        f"RETURN record::exists({table}:later)", f"SELECT VALUE record::id(id) FROM {table}"]
    values = iter([{"lives":{}},False])
    with pytest.raises(fx.FixtureBlocked, match="disagrees"):
        recovery.reconcile(installed.installation, manifest, {})
    assert not any("REMOVE" in sql for sql in calls)


@pytest.mark.parametrize("blocker", ["live_owner","foreign_owner","missing_owner","unnamed_borrower","borrower_lease","command_unknown"])
def test_pd02_recovery_refuses_live_or_unnamed_owners_before_server_actions(installed, monkeypatch, blocker):
    import surrealdb_pd02_recovery as recovery
    attachment, _, _ = pd02_dead_attachment(installed, monkeypatch)
    other = None
    held = None
    row = fx.read_json(attachment.directory / "record.json")
    if blocker == "live_owner":
        row["owner"] = service.ProcessIdentity.of().to_json()
        fx.write_json_atomic(attachment.directory / "record.json", row)
    elif blocker == "foreign_owner":
        row["owner"] = service.ProcessIdentity.of().to_json()
        row["owner"]["pid_namespace"] += 1
        fx.write_json_atomic(attachment.directory / "record.json", row)
    elif blocker == "missing_owner":
        row.pop("owner")
        fx.write_json_atomic(attachment.directory / "record.json", row)
    elif blocker == "unnamed_borrower": other = fx.Attachment.create(installed)
    elif blocker == "borrower_lease":
        import fcntl
        held = os.open(installed.directory / "borrowers.lock", os.O_RDWR)
        fcntl.flock(held, fcntl.LOCK_SH)
    else: monkeypatch.setattr(fx, "_command_observation", lambda row: {"status":"unknown"})
    commands = []
    monkeypatch.setattr(service, "systemctl", lambda *args: commands.append(args))
    try:
        with pytest.raises(fx.FixtureBlocked):
            recovery.recover(installed.installation, attachment.id)
        assert not commands
        assert fx.read_json(attachment.directory / "record.json")["released"] is False
    finally:
        if other: other.release()
        if held is not None: os.close(held)


def pd02_join_protocol_fixture(installed, process):
    import surrealdb_pd02_recovery as recovery
    entries = [{"_SYSTEMD_INVOCATION_ID":"a"*32,"_SYSTEMD_USER_UNIT":installed.unit,
        "_PID":str(process.pid),"MESSAGE":recovery.JOIN_MESSAGE}]
    text = json.dumps(entries[0]) + "\n"
    return {"state":{"InvocationID":"a"*32,"MainPID":"0","ActiveState":"inactive",
        "Result":"success","ExecMainCode":"1","ExecMainStatus":"0"},
        "journal":{"text":text,"sha256":recovery.digest_bytes(text.encode()),"entries":entries}}


def pd02_mock_recovery_ready(installed, monkeypatch):
    import surrealdb_pd02_recovery as recovery
    attachment, native_path, original_native = pd02_dead_attachment(installed, monkeypatch)
    installation = installed.installation
    installation.directory.chmod(0o700)
    old = service.ProcessIdentity.of()
    new = SimpleNamespace(to_json=lambda: {"pid":99999998,"start_ticks":7})
    installation._daemon_identity = (old, ())
    monkeypatch.setattr(recovery, "validate_server", lambda inst: None)
    monkeypatch.setattr(service.Installation, "check_daemon", lambda inst: None)
    monkeypatch.setattr(service, "unit_properties", lambda *args: {"InvocationID":"a"*32,"MainPID":str(old.pid)})
    joined = pd02_join_protocol_fixture(installed, old)
    monkeypatch.setattr(recovery, "joined_exit", lambda *args: joined)
    monkeypatch.setattr(service, "_start_owned", lambda inst: setattr(inst,"_daemon_identity",(new,())))
    monkeypatch.setattr(service.Installation, "runtime", lambda *a, **kw: {})
    monkeypatch.setattr(service, "sql", lambda inst, statement, **kw: [{"status":"OK","result":None}])
    table = "pd02_" + "a"*64
    monkeypatch.setattr(recovery, "reconcile", lambda *args: {table:{"live":"absent",
        "effects":{"committed":"present","later":"absent"},"records":["committed"]}})
    return attachment, native_path, original_native


def test_pd02_host_recovery_preserves_original_unknown_and_independent_receipt(installed, monkeypatch):
    import surrealdb_pd02_recovery as recovery
    attachment, native_path, original_native = pd02_mock_recovery_ready(installed, monkeypatch)
    installation = installed.installation
    result = recovery.recover(installation, attachment.id)
    assert result["outcome"] == "recovered"
    assert native_path.read_bytes() == original_native
    row = fx.read_json(attachment.directory / "record.json")
    assert row["released"] is True
    assert fx.native_completion(attachment.directory, row)["status"] == "unknown"
    assert not fx.native_protected(attachment.directory, row, fx.native_completion(attachment.directory,row))
    assert not service.borrowers(installation)
    import storage_owners
    from storage_lifecycle import Storage
    assert storage_owners.observe(Storage().get(attachment._storage_id))["state"] == "released"
    assert Storage().get(attachment._storage_id)["obligations"][f"attachment:{attachment.id}"]["until"]
    assert not (installation.directory / "maintenance.json").exists()
    assert fx.inventory()[0]["native_completion"]["status"] == "unknown"
    assert fx.inventory()[0]["protected"] is False
    # Changed original native evidence cannot be masked by an old host receipt.
    native_path.write_bytes(original_native + b" ")
    assert recovery.recovered(attachment.directory, row) is False
    assert service.borrowers(installation)[0]["id"] == attachment.id
    assert storage_owners.observe(Storage().get(attachment._storage_id))["state"] == "unresolved"


@pytest.mark.parametrize("phase", ["join", "reconcile", "remove", "drain", "open"])
def test_pd02_interrupted_recovery_preserves_unknown_and_resumes_exact_owner(installed, monkeypatch, phase):
    import surrealdb_pd02_recovery as recovery
    attachment, native_path, original = pd02_mock_recovery_ready(installed, monkeypatch)
    installation = installed.installation
    target, name = {"join":(recovery,"joined_exit"), "reconcile":(recovery,"reconcile"),
        "remove":(service,"sql"), "drain":(service,"_run_installer"),
        "open":(service,"_run_installer")}[phase]
    original_function = getattr(target, name)
    fired = False
    def interrupt_once(*args, **kw):
        nonlocal fired
        chosen = phase not in {"drain", "open"} or args[2] == ("drain" if phase == "drain" else "open-admission")
        if not fired and chosen:
            fired = True
            raise KeyboardInterrupt()
        return original_function(*args, **kw)
    monkeypatch.setattr(target, name, interrupt_once)
    with pytest.raises(KeyboardInterrupt):
        recovery.recover(installation, attachment.id)
    marker_path = installation.directory / "maintenance.json"
    marker = fx.read_json(marker_path)
    assert marker["attachment"] == attachment.id
    assert marker["operation"] == recovery.OPERATION
    assert native_path.read_bytes() == original
    row = fx.read_json(attachment.directory / "record.json")
    assert fx.native_completion(attachment.directory, row)["status"] == "unknown"
    if phase != "open": assert row["released"] is False
    assert not service.lock_held(installation.directory / "admission.lock")
    # A real CLI resume is a new process. Simulate only its dead host identity;
    # do not mutate the original request/native or daemon recovery evidence.
    marker["owner"] = {**service.ProcessIdentity.of().to_json(), "pid":99999999}
    fx.write_json_atomic(marker_path, marker)
    assert recovery.recover(installation, attachment.id)["outcome"] == "recovered"
    assert native_path.read_bytes() == original
    assert not marker_path.exists()


def test_pd02_log_preparation_is_explicit_campaign_only_and_preserves_default(installed):
    import surrealdb_pd02_recovery as recovery
    assert '"--log=warn"' in service.unit_text(installed.directory, Path("/checked/server"), 29000)
    assert '"--log=warn,surrealdb::net=info"' in service.unit_text(installed.directory,
        Path("/checked/server"),29000,log_filter="warn,surrealdb::net=info")
    with pytest.raises(fx.FixtureBlocked, match="campaign-only"):
        recovery.prepare_logs(installed.installation)
    with pytest.raises(fx.FixtureBlocked, match="unrecognized"):
        service.unit_text(installed.directory, Path("/checked/server"), 29000, log_filter="trace")


@pytest.mark.parametrize("interrupt", [None, "reload", "join", "open"])
def test_pd02_log_preparation_journals_exact_unit_binary_and_rechecks(installed, tmp_path, monkeypatch, interrupt):
    import surrealdb_pd02_recovery as recovery
    generation = install_fake_patched_generation(installed, tmp_path)
    installation = installed.installation
    installation.directory.chmod(0o700)
    installation.record.update(kind="campaign")
    installation.record["unit"] = service.installation_unit(installation.record)
    service.private_json(installation.directory / "installation.json", installation.record)
    unit = tmp_path / "owned-unit.service"
    unit.write_text(service.unit_text(installation.directory, Path(generation["path"]), 29000))
    monkeypatch.setattr(service,"_unit_path",lambda *a:unit)
    process = service.ProcessIdentity.of()
    installation._daemon_identity = (process, ())
    monkeypatch.setattr(service.Installation,"check_daemon",lambda self:None)
    monkeypatch.setattr(service,"unit_properties",lambda *args:{"InvocationID":"a"*32,"MainPID":str(process.pid)})
    monkeypatch.setattr(recovery,"joined_exit",lambda *args:pd02_join_protocol_fixture(installed,process))
    actions = []
    monkeypatch.setattr(service,"_stop_owned",lambda inst:actions.append("ordinary preparation stop"))
    monkeypatch.setattr(service,"_start_owned",lambda inst:actions.append("checked preparation start"))
    monkeypatch.setattr(service,"_run_installer",lambda inst,binary,action:actions.append(action))
    monkeypatch.setattr(service,"systemctl",lambda *args:SimpleNamespace(returncode=0))
    before = dict(installation.record["binary"])
    if interrupt:
        target, name = {"reload":(service,"systemctl"), "join":(recovery,"joined_exit"),
            "open":(service,"_run_installer")}[interrupt]
        original = getattr(target, name)
        def interrupted(*args, **kw):
            selected = interrupt == "join" or interrupt == "reload" and args[0] == "daemon-reload" or interrupt == "open" and args[2] == "open-admission"
            if selected: raise KeyboardInterrupt()
            return original(*args, **kw)
        monkeypatch.setattr(target,name,interrupted)
        with pytest.raises(KeyboardInterrupt):
            recovery.prepare_logs(installation)
        marker_path = installation.directory / "maintenance.json"
        marker = fx.read_json(marker_path)
        assert marker["operation"] == "PD02 campaign log preparation"
        assert marker["phase"] in {"drained","smoke_joining","smoke_joined"}
        marker["owner"] = {**service.ProcessIdentity.of().to_json(),"pid":99999999}
        fx.write_json_atomic(marker_path,marker)
        monkeypatch.setattr(target,name,original)
    result = recovery.prepare_logs(installation)
    assert result["outcome"] == "prepared"
    assert installation.record["binary"] == before
    assert installation.record["server_log_filter"] == "warn,surrealdb::net=info"
    if not interrupt:
        assert actions == ["close-admission","drain","ordinary preparation stop",
            "checked preparation start","close-admission","drain", "checked preparation start",
            "close-admission","drain","open-admission"]
    receipt = fx.read_json(Path(result["receipt"]))
    assert "abandoned-request finality not_observed" in receipt["registry_join_evidence"]
    plan = fx.read_json(Path(result["receipt"]).with_name("plan.json"))
    assert plan["previous_descriptor"]["binary"] == plan["successor_descriptor"]["binary"]
    assert "--log=warn,surrealdb::net=info" in plan["successor_unit"]
    assert unit.read_text() == plan["successor_unit"]
    assert not (installation.directory / "maintenance.json").exists()

    recovery.validate_server(installation)


@pytest.mark.parametrize("result", [[], [{"status":"ERR"}], [{"status":"OK"},{"status":"OK"}]])
def test_pd02_unchecked_cleanup_terminal_cannot_become_immutable_authority(installed, monkeypatch, result):
    import surrealdb_pd02_recovery as recovery
    attachment, native, original = pd02_mock_recovery_ready(installed, monkeypatch)
    monkeypatch.setattr(service,"sql",lambda *args,**kw:result)
    with pytest.raises(fx.FixtureBlocked, match="checked terminal"):
        recovery.recover(installed.installation,attachment.id)
    assert not list((attachment.directory / "pd02-host-recovery").glob("removed-*.json"))
    assert native.read_bytes() == original
    assert fx.read_json(attachment.directory / "record.json")["released"] is False
    assert (installed.directory / "maintenance.json").exists()


@pytest.mark.parametrize("change", ["unit", "pid", "digest", "transaction_error"])
def test_pd02_retained_join_requires_unit_pid_digest_and_no_unresolved_cleanup(installed, change):
    import surrealdb_pd02_recovery as recovery
    process = service.ProcessIdentity.of()
    joined = pd02_join_protocol_fixture(installed,process)
    daemon = {"invocation":"a"*32,"process":process.to_json()}
    if change == "digest": joined["journal"]["sha256"] = "b"*64
    else:
        entry = joined["journal"]["entries"][0]
        if change == "unit": entry["_SYSTEMD_USER_UNIT"] = "unrelated.service"
        elif change == "pid": entry["_PID"] = "99999999"
        else: entry["MESSAGE"] = "Error cancelling gRPC transaction exact-owned: failed; " + recovery.JOIN_MESSAGE
        joined["journal"]["text"] = json.dumps(entry) + "\n"
        joined["journal"]["sha256"] = recovery.digest_bytes(joined["journal"]["text"].encode())
    assert recovery.valid_join_evidence(joined,daemon,installed.unit) is False


def native_join_protocol_fixture(installed, process):
    import surrealdb_pd02_recovery as recovery
    joined = pd02_join_protocol_fixture(installed, process)
    joined["source_proof"] = recovery.SOURCE_JOIN
    joined["journal"]["entries"][0]["MESSAGE"] = "SIGTERM received. Waiting for graceful shutdown; another signal closes transport immediately but still waits for query finality."
    text = json.dumps(joined["journal"]["entries"][0]) + "\n"
    joined["journal"].update(text=text,sha256=recovery.digest_bytes(text.encode()))
    return joined


def native_mock_recovery_ready(installed, tmp_path, monkeypatch):
    import base64
    import surrealdb_pd02_recovery as recovery
    installation = installed.installation
    installation.directory.chmod(0o700)
    installation.record.update(kind="campaign")
    installation.record["unit"] = service.installation_unit(installation.record)
    install_fake_patched_generation(installed,tmp_path,cpu_count=32)
    installer = tmp_path / "original-installer"
    installer.write_text("original immutable installer")
    installation.record.update(installer=str(installer),installer_generation={"path":str(installer),"sha256":service.file_sha256(installer)})
    service.private_json(installation.directory / "installation.json",installation.record)
    attachment = fx.Attachment.create(installed)
    native_path, native = native_receipt(attachment,state="started")
    native["owner"] = "native compiler store"
    fx.write_json_atomic(native_path,native)
    attachment._write_record(None,command_cleanup={"status":"confirmed","survivors":[]},child_exit_code=100)
    attachment.release()
    row = fx.read_json(attachment.directory / "record.json")
    row["owner"] = {**service.ProcessIdentity.of().to_json(),"pid":99999999,"start_ticks":1}
    fx.write_json_atomic(attachment.directory / "record.json",row)
    monkeypatch.setattr(fx,"_command_observation",lambda row:{"status":"gone"})
    old = service.ProcessIdentity.of()
    new = service.ProcessIdentity.from_json({**old.to_json(),"pid":99999998,"start_ticks":7})
    installation._daemon_identity = (old,(1,2,3,4,5))
    unit = tmp_path / "native-campaign.service"
    unit.write_text(service.unit_text(installation.directory,Path(installation.record["binary"]["path"]),29000))
    monkeypatch.setattr(service,"_unit_path",lambda _:unit)
    monkeypatch.setattr(service.Installation,"check_daemon",lambda inst:None)
    monkeypatch.setattr(service,"unit_properties",lambda *a:{"InvocationID":("a" if installation._daemon_identity[0] == old else "b")*32,
        "MainPID":str(installation._daemon_identity[0].pid),"ActiveState":"active"})
    monkeypatch.setattr(recovery,"joined_exit",lambda *a,**kw:native_join_protocol_fixture(installed,old))
    monkeypatch.setattr(service,"_start_owned",lambda inst:setattr(inst,"_daemon_identity",(new,(1,2,3,4,6))))
    argv = b"\0".join(value.encode() for value in [installation.record["binary"]["path"],"start","--log=warn","127.0.0.1:29000",f"rocksdb://{installation.directory / 'data/store'}"])
    monkeypatch.setattr(recovery,"native_daemon_snapshot",lambda *a:{"argv":base64.b64encode(argv).decode(),"executable_identity":[1,2,3,4,5]})
    monkeypatch.setattr(service,"sql",lambda *a,**kw:pytest.fail("generic recovery has no direct SQL/deletion authority"))
    return attachment,native_path,native_path.read_bytes()


def test_native_host_source_proof_is_distinct_from_positive_log(installed, monkeypatch):
    import surrealdb_pd02_recovery as recovery
    process = service.ProcessIdentity.of()
    joined = native_join_protocol_fixture(installed,process)
    daemon = {"invocation":"a"*32,"process":process.to_json()}
    assert recovery.valid_join_evidence(joined,daemon,installed.unit,source_correlated=True)
    assert not recovery.valid_join_evidence(joined,daemon,installed.unit)
    text = joined["journal"]["text"]
    monkeypatch.setattr(recovery.subprocess,"run",lambda *a,**kw:SimpleNamespace(returncode=0,stdout=text))
    assert recovery.journal_join(installed.installation,"a"*32,process.pid,source_correlated=True)["text"] == text
    with pytest.raises(fx.FixtureBlocked,match="positive"):
        recovery.journal_join(installed.installation,"a"*32,process.pid)
    joined["source_proof"] = {**recovery.SOURCE_JOIN,"patch":"wrong"}
    assert not recovery.valid_join_evidence(joined,daemon,installed.unit,source_correlated=True)


@pytest.mark.parametrize("message,priority",[
    ("Error cancelling gRPC transaction exact: failed",None),
    ("Error cleaning up live query exact on shutdown: failed",None),
    ("Error killing subscription-owned live query exact: failed",None),
    ("Error cleaning up the live queries of an unfinished gRPC streaming query: failed",None),
    ("gRPC delivery-loss cleanup unresolved; client acknowledgement remains unknown",None),
    ("2026-10-11 \x1b[31mERROR\x1b[0m arbitrary cleanup component: failed","6"),
    ("structured error without rendered level","3"),
])
def test_native_host_join_rejects_nonpropagated_cleanup_errors(installed,message,priority):
    import surrealdb_pd02_recovery as recovery
    process=service.ProcessIdentity.of()
    joined=native_join_protocol_fixture(installed,process)
    entry={**joined["journal"]["entries"][0],"MESSAGE":message}
    if priority is not None: entry["PRIORITY"]=priority
    joined["journal"]["entries"].append(entry)
    text="".join(json.dumps(row)+"\n" for row in joined["journal"]["entries"])
    joined["journal"].update(text=text,sha256=recovery.digest_bytes(text.encode()))
    assert not recovery.valid_join_evidence(joined,{"invocation":"a"*32,"process":process.to_json()},installed.unit,source_correlated=True)


def test_native_host_recovery_preserves_started_bytes_and_request_unknown(installed,tmp_path,monkeypatch):
    import surrealdb_pd02_recovery as recovery
    from storage_lifecycle import Storage
    import storage_owners
    attachment,native,original = native_mock_recovery_ready(installed,tmp_path,monkeypatch)
    calls = []
    monkeypatch.setattr(service,"_run_installer",lambda inst,path,action:calls.append(action))
    result = recovery.recover(installed.installation,attachment.id,native=True)
    assert result["outcome"] == "recovered"
    assert native.read_bytes() == original
    row = fx.read_json(attachment.directory / "record.json")
    assert row["released"] and fx.native_completion(attachment.directory,row)["status"] == "unknown"
    receipt = fx.read_json(Path(row["host_recovery"]["path"]))
    assert receipt["original_request_outcomes"].startswith("unknown;")
    assert receipt["tables_removed"] == [] and receipt["reconciliation"] is None
    assert calls == ["close-admission","close-admission","drain","open-admission"]
    assert recovery.native_recovered(attachment.directory,row)
    assert not service.borrowers(installed.installation)
    assert storage_owners.observe(Storage().get(attachment._storage_id))["state"] == "released"
    assert Storage().get(attachment._storage_id)["obligations"][f"attachment:{attachment.id}"]["until"]
    native.write_bytes(original+b" ")
    assert not recovery.native_recovered(attachment.directory,row)
    assert service.borrowers(installed.installation)[0]["id"] == attachment.id


def failed_wire_owner(path):
    row = fx.read_json(path)
    row.update(owner="S09 finite accepted-query wire owner", state="complete", completion={
        "local":"terminal", "remote":"confirmed", "storage":[], "committed":[],
        "failures":[{"step":"S09 actual wire control", "error":"logical IAM refusal"}],
    })
    fx.write_json_atomic(path,row)


def test_native_host_recovers_failed_wire_owner_without_rewriting_failure(installed,tmp_path,monkeypatch):
    import surrealdb_pd02_recovery as recovery
    attachment,path,_ = native_mock_recovery_ready(installed,tmp_path,monkeypatch)
    failed_wire_owner(path)
    original = path.read_bytes()
    marker_path = installed.directory / "maintenance.json"
    marker = {"operation":"maintenance", "token":"original-closed-owner", "owner":{
        **service.ProcessIdentity.of().to_json(), "pid":99999997, "start_ticks":1}}
    service.private_json(marker_path,marker)
    prior = marker_path.read_bytes()
    monkeypatch.setattr(service,"_run_installer",lambda *a:None)
    assert recovery.recover_native(installed.installation,[attachment.id])["outcome"] == "recovered"
    assert path.read_bytes() == original
    row = fx.read_json(attachment.directory / "record.json")
    assert row["released"] and fx.native_completion(attachment.directory,row)["status"] == "unknown"
    plan = fx.read_json(attachment.directory / "native-host-recovery" / "plan.json")
    import base64
    assert base64.b64decode(plan["transferred_maintenance"]["bytes"]) == prior
    assert plan["transferred_maintenance"]["record"] == marker
    assert plan["named_scope"][attachment.id]["classification"].startswith("Failed terminal")
    assert recovery.native_recovered(attachment.directory,row)
    assert not marker_path.exists() and not service.borrowers(installed.installation)


@pytest.mark.parametrize("change",["owner","state","local","remote","storage","committed","empty_failure","step","extra"])
def test_native_host_refuses_ineligible_failed_wire_owner_before_effect(installed,tmp_path,monkeypatch,change):
    import surrealdb_pd02_recovery as recovery
    attachment,path,_ = native_mock_recovery_ready(installed,tmp_path,monkeypatch)
    failed_wire_owner(path)
    row = fx.read_json(path)
    if change == "owner": row["owner"] = "arbitrary completed owner"
    elif change == "state": row["state"] = "started"
    elif change == "empty_failure": row["completion"]["failures"] = []
    elif change == "step": row["completion"]["failures"][0]["step"] = "storage cleanup"
    elif change == "extra": row["completion"]["extra"] = True
    elif change in {"storage","committed"}: row["completion"][change] = ["unresolved"]
    else: row["completion"][change] = "unknown"
    fx.write_json_atomic(path,row)
    monkeypatch.setattr(service,"_run_installer",lambda *a:pytest.fail("ineligible owner cannot trigger effects"))
    with pytest.raises(fx.FixtureBlocked):
        recovery.recover_native(installed.installation,[attachment.id])


@pytest.mark.parametrize("change",["live","foreign","specialized","extra","empty_token"])
def test_native_host_refuses_untransferable_ordinary_maintenance(installed,tmp_path,monkeypatch,change):
    import surrealdb_pd02_recovery as recovery
    attachment,path,_ = native_mock_recovery_ready(installed,tmp_path,monkeypatch)
    failed_wire_owner(path)
    marker = {"operation":"maintenance", "token":"original", "owner":{
        **service.ProcessIdentity.of().to_json(), "pid":99999997, "start_ticks":1}}
    if change == "live": marker["owner"] = service.ProcessIdentity.of().to_json()
    elif change == "foreign": marker["owner"]["pid_namespace"] += 1
    elif change == "specialized": marker["operation"] = service.SERVER_HANDOFF_OPERATION
    elif change == "extra": marker["recovery"] = {"unknown":True}
    else: marker["token"] = ""
    service.private_json(installed.directory / "maintenance.json",marker)
    monkeypatch.setattr(service,"_run_installer",lambda *a:pytest.fail("untransferable owner cannot trigger effects"))
    with pytest.raises(fx.FixtureBlocked):
        recovery.recover_native(installed.installation,[attachment.id])
    assert fx.read_json(installed.directory / "maintenance.json") == marker


def test_native_host_resumes_between_preservation_and_maintenance_transfer(installed,tmp_path,monkeypatch):
    import surrealdb_pd02_recovery as recovery
    attachment,path,_ = native_mock_recovery_ready(installed,tmp_path,monkeypatch)
    failed_wire_owner(path)
    original = path.read_bytes()
    marker_path = installed.directory / "maintenance.json"
    marker = {"operation":"maintenance", "token":"prior-owner", "owner":{
        **service.ProcessIdentity.of().to_json(), "pid":99999997, "start_ticks":1}}
    service.private_json(marker_path,marker)
    prior = marker_path.read_bytes()
    publish = recovery.immutable
    def interrupt_after_preservation(destination,value):
        publish(destination,value)
        if destination.name == "plan.json":
            raise KeyboardInterrupt()
    monkeypatch.setattr(recovery,"immutable",interrupt_after_preservation)
    monkeypatch.setattr(service,"_run_installer",lambda *a:pytest.fail("preservation has not claimed maintenance"))
    with pytest.raises(KeyboardInterrupt):
        recovery.recover_native(installed.installation,[attachment.id])
    assert marker_path.read_bytes() == prior and path.read_bytes() == original
    monkeypatch.setattr(recovery,"immutable",publish)
    monkeypatch.setattr(service,"_run_installer",lambda *a:None)
    assert recovery.recover_native(installed.installation,[attachment.id])["outcome"] == "recovered"
    assert path.read_bytes() == original and not marker_path.exists()


@pytest.mark.parametrize("phase",["join","drain","open"])
def test_native_host_interruption_retains_unknown_and_resumes_exact_plan(installed,tmp_path,monkeypatch,phase):
    import surrealdb_pd02_recovery as recovery
    attachment,native,original = native_mock_recovery_ready(installed,tmp_path,monkeypatch)
    joined = recovery.joined_exit
    def interrupt_join(*a,**kw):
        if phase == "join": raise KeyboardInterrupt()
        return joined(*a,**kw)
    def interrupt_installer(inst,path,action):
        if action == {"drain":"drain","open":"open-admission"}.get(phase): raise KeyboardInterrupt()
    monkeypatch.setattr(recovery,"joined_exit",interrupt_join)
    monkeypatch.setattr(service,"_run_installer",interrupt_installer)
    with pytest.raises(KeyboardInterrupt): recovery.recover(installed.installation,attachment.id,native=True)
    assert native.read_bytes() == original
    marker_path = installed.directory / "maintenance.json"
    marker = fx.read_json(marker_path)
    marker["owner"] = {**service.ProcessIdentity.of().to_json(),"pid":99999997,"start_ticks":1}
    fx.write_json_atomic(marker_path,marker)
    monkeypatch.setattr(recovery,"joined_exit",joined)
    monkeypatch.setattr(service,"_run_installer",lambda *a:None)
    assert recovery.recover(installed.installation,attachment.id,native=True)["outcome"] == "recovered"
    assert native.read_bytes() == original and not marker_path.exists()


@pytest.mark.parametrize("change",["owner","terminal","generation","live","recipe","unit","installer","stable"])
def test_native_host_refuses_unbound_owners_or_server_before_effect(installed,tmp_path,monkeypatch,change):
    import surrealdb_pd02_recovery as recovery
    attachment,path,_ = native_mock_recovery_ready(installed,tmp_path,monkeypatch)
    row = fx.read_json(path)
    if change == "owner": row["owner"] = "other owner"
    if change == "terminal": row.update(state="complete",completion={"remote":"unknown"})
    if change == "generation": row["generation"] = "f"*64
    if change == "live":
        current=service.ProcessIdentity.of()
        path.unlink()
        row.update(process={"pid":current.pid,"start_ticks":current.start_ticks})
        path=path.with_name(f"{current.pid}-{current.start_ticks}-0.json")
    fx.write_json_atomic(path,row)
    if change == "recipe": installed.record["binary"]["generation"]["provenance"]["recipe_identity"] = "f"*64
    if change == "unit": service._unit_path(installed.unit).write_text("foreign unit")
    if change == "installer": Path(installed.record["installer"]).write_text("changed")
    if change == "stable": installed.record["kind"] = "stable"
    monkeypatch.setattr(service,"_run_installer",lambda *a:pytest.fail("must refuse before any native operation"))
    with pytest.raises(fx.FixtureBlocked): recovery.recover(installed.installation,attachment.id,native=True)
    assert not (attachment.directory / "native-host-recovery" / "receipt.json").exists()


def native_legacy_peer(installed,tmp_path,monkeypatch):
    import runs
    daemon=installed.installation._daemon_identity
    installed.installation._daemon_identity=None
    try:
        peer=fx.Attachment.create(installed)
    finally:
        installed.installation._daemon_identity=daemon
    peer.release()
    row=fx.read_json(peer.directory / "record.json")
    row.pop("native_drain",None)
    row.pop("native_completion",None)
    row.update(child_exit_code=-15,command_cleanup={"status":"confirmed","survivors":[]},
        owner={**service.ProcessIdentity.of().to_json(),"pid":99999996,"start_ticks":1})
    fx.write_json_atomic(peer.directory / "record.json",row)
    monkeypatch.setenv("LCTX_RUNS_ROOT",str(tmp_path / "runs"))
    run_id="20261011T164850.001Z-0114a7"
    directory=runs.runs_root() / run_id
    directory.mkdir(parents=True)
    output=directory / "output.log"
    output.write_text("historical command output; no prerequest authority\n")
    service.private_json(directory / "record.json",{"id":run_id,"phase":"ended",
        "started":row["created"],"ended":row["released_at"],"cwd":row["checkout"],
        "cleanup":{"status":"confirmed","survivors":[]},"logs":{"output":str(output)}})
    return peer,run_id,row


def test_native_joint_recovery_preserves_legacy_local_release_and_native_not_observed(installed,tmp_path,monkeypatch):
    import surrealdb_pd02_recovery as recovery
    primary,native,original=native_mock_recovery_ready(installed,tmp_path,monkeypatch)
    peer,run_id,legacy=native_legacy_peer(installed,tmp_path,monkeypatch)
    result=recovery.recover_native(installed.installation,[peer.id,primary.id],legacy_runs={peer.id:run_id})
    assert result["outcome"] == "recovered" and native.read_bytes() == original
    row=fx.read_json(peer.directory / "record.json")
    assert row["released_at"] == legacy["released_at"] and row.get("native_drain") is None
    assert fx.native_completion(peer.directory,row)["status"] == "not_observed"
    assert fx.native_completion(primary.directory,fx.read_json(primary.directory / "record.json"))["status"] == "unknown"
    assert recovery.native_recovered(peer.directory,row) and not service.borrowers(installed.installation)
    plan=fx.read_json(primary.directory / "native-host-recovery/plan.json")
    assert set(plan["named_scope"]) == {peer.id,primary.id}
    assert plan["named_scope"][peer.id]["original_record"] == legacy
    assert "not retrospectively qualified" in plan["named_scope"][peer.id]["classification"]
    assert "no prerequest" in plan["legacy_runs"][peer.id]["evidence_scope"]
    row["child_exit_code"]=-9
    fx.write_json_atomic(peer.directory / "record.json",row)
    assert not recovery.native_recovered(primary.directory,fx.read_json(primary.directory / "record.json"))
    assert len(service.borrowers(installed.installation)) == 2


@pytest.mark.parametrize("phase",["peer_reference","open"])
def test_native_joint_resume_retains_common_scope_after_partial_publication(installed,tmp_path,monkeypatch,phase):
    import surrealdb_pd02_recovery as recovery
    primary,native,original=native_mock_recovery_ready(installed,tmp_path,monkeypatch)
    peer,run_id,legacy=native_legacy_peer(installed,tmp_path,monkeypatch)
    publish=recovery.durable_json
    def interrupted_publish(path,value):
        if phase == "peer_reference" and path == peer.directory / "record.json" and value.get("host_recovery"):
            raise KeyboardInterrupt()
        return publish(path,value)
    def interrupted_installer(inst,path,action):
        if phase == "open" and action == "open-admission": raise KeyboardInterrupt()
    monkeypatch.setattr(recovery,"durable_json",interrupted_publish)
    monkeypatch.setattr(service,"_run_installer",interrupted_installer)
    with pytest.raises(KeyboardInterrupt):
        recovery.recover_native(installed.installation,[peer.id,primary.id],legacy_runs={peer.id:run_id})
    assert native.read_bytes() == original
    marker_path=installed.directory / "maintenance.json"
    marker=fx.read_json(marker_path)
    marker["owner"]={**service.ProcessIdentity.of().to_json(),"pid":99999995,"start_ticks":1}
    fx.write_json_atomic(marker_path,marker)
    monkeypatch.setattr(recovery,"durable_json",publish)
    monkeypatch.setattr(service,"_run_installer",lambda *a:None)
    assert recovery.recover_native(installed.installation,[primary.id,peer.id],legacy_runs={peer.id:run_id})["outcome"] == "recovered"
    assert native.read_bytes() == original and fx.read_json(peer.directory / "record.json")["released_at"] == legacy["released_at"]
    assert not marker_path.exists()


@pytest.mark.parametrize("change",["unnamed","missing_run","receipt","live","cleanup"])
def test_native_joint_refuses_ineligible_or_unnamed_peer_before_effect(installed,tmp_path,monkeypatch,change):
    import surrealdb_pd02_recovery as recovery
    primary,_,_=native_mock_recovery_ready(installed,tmp_path,monkeypatch)
    peer,run_id,row=native_legacy_peer(installed,tmp_path,monkeypatch)
    selected=[primary.id,peer.id]
    mapping={peer.id:run_id}
    if change == "unnamed": selected=[primary.id];mapping={}
    if change == "missing_run": mapping={}
    if change == "receipt": service.private_json(peer.directory / "native-drain/unexpected.json",{})
    if change == "live": row["owner"]=service.ProcessIdentity.of().to_json()
    if change == "cleanup": row["command_cleanup"]["survivors"]=[{"pid":1}]
    fx.write_json_atomic(peer.directory / "record.json",row)
    monkeypatch.setattr(service,"_run_installer",lambda *a:pytest.fail("refuse before effects"))
    with pytest.raises(fx.FixtureBlocked): recovery.recover_native(installed.installation,selected,legacy_runs=mapping)


@pytest.fixture
def completed_discard(installed):
    """Local producer acknowledgment fixture; not native execution evidence."""
    import base64
    import hashlib
    directory=installed.directory/'attachments'/str(uuid.uuid4());directory.mkdir(mode=0o700)
    for path in (installed.directory, directory.parent):path.chmod(0o700)
    (directory/'native-drain').mkdir()
    original={'id':directory.name,'fixture':installed.id,'namespace':service.NAMESPACE,'database':'validation',
        'released':False,'interrupted':True,'command_cleanup':{'status':'unknown'},
        'command':{'argv':['original-immutable-command']},
        'native_drain':{'attachment':directory.name,'directory':str(directory/'native-drain'),
            'generation':bytes(installed.record['service_generation']).hex()}}
    def blob(value):
        raw=json.dumps(value,sort_keys=True).encode()
        return {'bytes':base64.b64encode(raw).decode(),'sha256':hashlib.sha256(raw).hexdigest()}
    plan={'path':'/removed-action/plan.json','sha256':'a'*64}
    joined={'path':'/removed-action/joined.json','sha256':'b'*64}
    scope={'installation_id':installed.id,'endpoint':installed.endpoint,'namespace':service.NAMESPACE,
        'databases':['main','validation'],'original_generation':original['native_drain']['generation'],
        'generation':list(bytes([3])*32),'target':{'schema_version':6,'schema':'c'*64}}
    checked={'installation_id':installed.id,'generation':scope['generation'],'target':scope['target'],
        'databases':scope['databases'],'original_native_outcome':'unknown',
        'disposition':'explicitly discarded and regenerated','plan':plan,'joined':joined}
    receipt={'schema':1,'kind':'explicit-scope-disposition','outcome':'discarded-and-regenerated',
        'local_owner':fx.scope_local_owner(directory),
        'original_native_outcome':'unknown','scope':scope,
        'original_attachment':{'id':directory.name,'native_drain':original['native_drain'],
            'record_identity':fx.scope_record_identity(original),
            'lifecycle':{key:original[key] for key in fx.SCOPE_LIFECYCLE_FIELDS if key in original}},
        'original_run':{'id':'owned-original-run','path':'/removed-checkout/build/runs/owned-original-run',
            'record_identity':'d'*64,'termination':'cancelled','exit':{'code':143},
            'cleanup':{'status':'confirmed','survivors':[]},'native_outcome':'unknown'},
        'proofs':{'plan':plan,'joined':{'reference':joined,'journal_sha256':'e'*64,
            'owner':{'unit':'owned-dedicated.service','invocation':'f'*32,'process':{'pid':123,'start_ticks':42}},
            'state':{'InvocationID':'f'*32,'MainPID':'0','ActiveState':'inactive','Result':'success',
                'ExecMainCode':'1','ExecMainStatus':'0'}},
            'original_removals':{db:blob({'database':db,'removed':True,'join':joined}) for db in scope['databases']},
            'checked':blob(checked),'complete':blob({**checked,'admission_open':True})}}
    row={**original,'released':True,'command_cleanup':{'status':'confirmed','survivors':[]}}
    path=directory/'scope-disposition.json'
    def publish():
        raw=json.dumps(receipt,sort_keys=True).encode()
        if path.exists():path.chmod(0o600)
        path.write_bytes(raw);path.chmod(0o400)
        row['scope_disposition']={'path':str(path),'sha256':hashlib.sha256(raw).hexdigest()}
        service.private_json(directory/'record.json',row)
    publish()
    return SimpleNamespace(directory=directory,row=row,receipt=receipt,publish=publish,blob=blob,path=path)


def test_completed_scope_disposition_preserves_unknown_without_action_or_current_source_dependency(completed_discard,monkeypatch):
    import builtins
    case=completed_discard
    importer=builtins.__import__
    def no_history(name,*args,**kwargs):
        if name.startswith('surrealdb_'):
            pytest.fail('completed receipt must not import an action or daemon recovery module')
        return importer(name,*args,**kwargs)
    monkeypatch.setattr(builtins,'__import__',no_history)
    assert fx.completed_scope_disposition(case.directory,case.row)
    assert not fx.native_protected(case.directory,case.row,{'status':'unknown','owners':[]})
    assert fx.native_completion(case.directory,case.row)=={'status':'not_observed','owners':[]}
    assert case.receipt['original_native_outcome']=='unknown'


@pytest.mark.parametrize('change',['fixture','native-generation','command','original-identity','final-generation',
    'unknown','scope','joined-state','removed-peer','checked-target','incomplete','proof-bytes','unreleased','directory-lifetime'])
def test_completed_disposition_rejects_misbound_record_or_proof_even_with_matching_local_digest(completed_discard,change):
    import base64
    case=completed_discard
    if change=='fixture':case.row['fixture']=str(uuid.uuid4())
    elif change=='native-generation':case.row['native_drain']={**case.row['native_drain'],'generation':'9'*64}
    elif change=='command':case.row['command']={'argv':['different-current-owner']}
    elif change=='original-identity':case.receipt['original_attachment']['record_identity']='9'*64
    elif change=='final-generation':case.receipt['scope']['generation']=list(bytes([8])*32)
    elif change=='unknown':case.receipt['original_native_outcome']='confirmed'
    elif change=='scope':case.receipt['scope']['databases']=['main']
    elif change=='joined-state':case.receipt['proofs']['joined']['state']['MainPID']='123'
    elif change=='removed-peer':case.receipt['proofs']['original_removals']['main']=case.blob(
        {'database':'validation','removed':True,'join':case.receipt['proofs']['joined']['reference']})
    elif change=='checked-target':
        value=json.loads(base64.b64decode(case.receipt['proofs']['checked']['bytes']));value['target']['schema']='9'*64
        case.receipt['proofs']['checked']=case.blob(value)
    elif change=='incomplete':
        value=json.loads(base64.b64decode(case.receipt['proofs']['complete']['bytes']));value['admission_open']=False
        case.receipt['proofs']['complete']=case.blob(value)
    elif change=='proof-bytes':case.receipt['proofs']['complete']['bytes']=base64.b64encode(b'{}').decode()
    elif change=='directory-lifetime':case.receipt['local_owner']['inode']+=1
    else:case.row['released']=False
    case.publish()
    assert not fx.completed_scope_disposition(case.directory,case.row)
    assert fx.native_protected(case.directory,case.row,{'status':'unknown','owners':[]})


@pytest.mark.parametrize('change',['path','checksum','writable','symlink'])
def test_completed_disposition_confines_immutable_local_owned_path(completed_discard,tmp_path,change):
    case=completed_discard
    if change=='path':case.row['scope_disposition']['path']=str(tmp_path/'other.json')
    elif change=='checksum':case.row['scope_disposition']['sha256']='0'*64
    elif change=='writable':case.path.chmod(0o600)
    else:
        target=tmp_path/'receipt-copy.json';target.write_bytes(case.path.read_bytes());target.chmod(0o400)
        case.path.unlink();case.path.symlink_to(target)
    assert not fx.completed_scope_disposition(case.directory,case.row)


def test_completed_disposition_validates_dedicated_revised_current_scope_removals(completed_discard):
    import base64
    case=completed_discard
    reference={'path':'/retired-oneoff/initialization-revision.json','sha256':'7'*64}
    generation=list(bytes([5])*32)
    case.receipt['proofs']['initialization_revisions']=[{'reference':reference,'discarded_generation':generation,
        'predecessor':case.receipt['proofs']['plan'], 'generation':case.receipt['scope']['generation'],
        'target':case.receipt['scope']['target'],
        'removals':{db:case.blob({'database':db,'removed':True,'revision':reference,'discarded_generation':generation})
            for db in case.receipt['scope']['databases']}}]
    checked=json.loads(base64.b64decode(case.receipt['proofs']['checked']['bytes']))
    checked['initialization_revision']=reference
    case.receipt['proofs']['checked']=case.blob(checked)
    case.receipt['proofs']['complete']=case.blob({**checked,'admission_open':True})
    case.publish()
    assert fx.completed_scope_disposition(case.directory,case.row)
    case.receipt['proofs']['initialization_revisions'][0]['removals']['main']=case.blob(
        {'database':'main','removed':True,'revision':reference,'discarded_generation':list(bytes([9])*32)})
    case.publish()
    assert not fx.completed_scope_disposition(case.directory,case.row)


@pytest.mark.parametrize('change', ['predecessor', 'generation-chain', 'missing-removal', 'latest-reference', 'latest-generation', 'latest-target', 'reordered', 'duplicate'])
def test_completed_disposition_authenticates_every_initialization_revision(completed_discard, change):
    import base64
    case = completed_discard
    revisions = []
    predecessor = case.receipt['proofs']['plan']
    generation = [5] * 32
    for index in range(2):
        reference = {'path':f'/retired-oneoff/revision-{index + 1}.json', 'sha256':str(index + 6) * 64}
        following = [6] * 32 if index == 0 else case.receipt['scope']['generation']
        revisions.append({'reference':reference, 'predecessor':predecessor,
            'discarded_generation':generation, 'generation':following, 'target':case.receipt['scope']['target'],
            'removals':{db:case.blob({'database':db, 'removed':True, 'revision':reference,
                'discarded_generation':generation}) for db in case.receipt['scope']['databases']}})
        predecessor, generation = reference, following
    case.receipt['proofs']['initialization_revisions'] = revisions
    checked = json.loads(base64.b64decode(case.receipt['proofs']['checked']['bytes']))
    checked['initialization_revision'] = predecessor
    case.receipt['proofs']['checked'] = case.blob(checked)
    case.receipt['proofs']['complete'] = case.blob({**checked, 'admission_open':True})
    case.publish()
    assert fx.completed_scope_disposition(case.directory, case.row)
    if change == 'predecessor': revisions[1]['predecessor'] = case.receipt['proofs']['plan']
    elif change == 'generation-chain': revisions[1]['discarded_generation'] = [8] * 32
    elif change == 'missing-removal': del revisions[0]['removals']['main']
    elif change == 'latest-reference': revisions[1]['reference']['sha256'] = '9' * 64
    elif change == 'latest-generation': revisions[1]['generation'] = [8] * 32
    elif change == 'latest-target': revisions[1]['target'] = {'schema_version':6, 'schema':'9' * 64}
    elif change == 'reordered': revisions.reverse()
    else: revisions.append(revisions[-1])
    case.publish()
    assert not fx.completed_scope_disposition(case.directory, case.row)
