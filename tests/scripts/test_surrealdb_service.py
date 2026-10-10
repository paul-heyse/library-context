"""Explicit durable installation contracts; no server/database mutations in unit controls."""

from __future__ import annotations

import contextlib
import fcntl
import io
import json
import os
import shutil
import subprocess
import sys
import tarfile
import uuid
from pathlib import Path
from types import SimpleNamespace

import pytest

import surrealdb_service as service
from surrealdb_service import _run_installer, verify_binary as verify_official_binary


def test_default_state_is_host_owned_and_shared_across_checkouts():
    assert service.state_root({"HOME": "/home/test"}) == Path(
        "/home/test/.local/state/library-context/surrealdb"
    )
    assert service.state_root({"HOME": "/home/test", "XDG_STATE_HOME": "/data/state"}) == Path(
        "/data/state/library-context/surrealdb"
    )


def test_service_unit_keeps_durability_timeouts_without_parallelism_or_memory_caps(tmp_path):
    unit = service.unit_text(tmp_path, Path("/tools/surreal"), 28000)
    assert "rocksdb://" in unit and '"20s"' in unit and '"10s"' in unit
    assert "KillMode=control-group" in unit and "Restart=no" in unit
    assert "MemoryMax" not in unit and "CPUQuota" not in unit and "TasksMax" not in unit
    assert "--threads" not in unit and "MEMORY_THRESHOLD" not in unit
    assert f"EnvironmentFile={tmp_path}/server.env\n" in unit


def test_systemd_accepts_environment_file_as_raw_absolute_path(tmp_path):
    tool = shutil.which("systemd-analyze")
    if not tool:
        pytest.skip("systemd-analyze is not installed")
    root = tmp_path / "state with spaces"
    root.mkdir()
    (root / "server.env").write_text("LCTX_OWNED_TEST=1\n")
    path = tmp_path / ("lctx-unit-control-" + str(uuid.uuid4()) + ".service")
    path.write_text(service.unit_text(root, Path("/bin/true"), 29000))
    result = subprocess.run(
        [tool, "--user", "verify", str(path)],
        capture_output=True,
        text=True,
        env=service.systemd_environment(),
    )
    assert result.returncode == 0, result.stderr
    assert not any(
        str(path) in line and "EnvironmentFile=" in line for line in result.stderr.splitlines()
    ), result.stderr


def test_check_and_status_do_not_create_missing_state(tmp_path, monkeypatch):
    root = tmp_path / "missing"
    monkeypatch.setenv("LCTX_SURREAL_SERVICE_CONFIG", str(root / "installation.json"))
    assert service.main(["check"]) == 75
    assert service.main(["status"]) == 75
    assert not root.exists()


def test_binary_wrong_digest_refuses_without_running(tmp_path):
    binary = tmp_path / "surreal"
    binary.write_bytes(b"wrong")
    with pytest.raises(service.FixtureBlocked, match="sha256"):
        service.verify_binary({"LCTX_SURREAL_BIN": str(binary)}, runner=pytest.fail)


def test_binary_version_checked_after_exact_digest(tmp_path, monkeypatch):
    binary = tmp_path / "surreal"
    binary.write_bytes(b"test")
    monkeypatch.setattr(service, "BINARY_SHA256", service.file_sha256(binary))
    with pytest.raises(service.FixtureBlocked, match=r"3.2.0"):
        service.verify_binary(
            {"LCTX_SURREAL_BIN": str(binary)},
            runner=lambda *a, **kw: subprocess.CompletedProcess(a, 0, "3.2.0", ""),
        )


def test_install_refuses_preexisting_unowned_unit(tmp_path, monkeypatch):
    binary = tmp_path / "surreal"
    binary.touch(mode=0o700)
    installer = tmp_path / "lctx"
    installer.touch(mode=0o700)
    root = tmp_path / "state"
    monkeypatch.setattr(service, "verify_binary", lambda *a, **kw: binary)
    monkeypatch.setattr(service, "systemd_available", lambda **kw: None)
    monkeypatch.setattr(
        service,
        "unit_properties",
        lambda *a, **kw: {"LoadState": "loaded", "ActiveState": "active"},
    )
    with pytest.raises(service.FixtureBlocked, match="refusing adoption"):
        service.install(
            installer, env={"LCTX_SURREAL_SERVICE_CONFIG": str(root / "installation.json")}
        )
    assert not (root / "installation.json").exists()


def test_private_credentials_never_use_tracked_repository_state(tmp_path):
    path = tmp_path / "private.json"
    service.private_json(path, {"password": "secret"})
    assert path.stat().st_mode & 0o777 == 0o600
    assert json.loads(path.read_text())["password"] == "secret"


def test_native_installer_failure_is_failed_not_inferred_infrastructure(tmp_path, monkeypatch):
    installer = tmp_path / "lctx"
    installer.touch(mode=0o700)
    installation = service.Installation(tmp_path, {})
    monkeypatch.setattr(service.Installation, "runtime", lambda *a, **kw: {})
    monkeypatch.setattr(
        service,
        "subprocess",
        type(
            "Runner", (), {"run": staticmethod(lambda *a, **kw: subprocess.CompletedProcess(a, 1))}
        ),
    )
    with pytest.raises(service.FixtureFailed, match="native drain failed"):
        service._run_installer(installation, installer, "drain")


def test_native_export_batches_keep_individual_rows_restorable():
    assert service.EXPORT_BATCH_SIZE == 1
    assert service.SERVER_SETTINGS["SURREAL_EXPORT_BATCH_SIZE"] == "1"
    assert service.SERVER_SETTINGS["SURREAL_GRPC_MAX_MESSAGE_SIZE"] == "128MiB"
    assert not any(
        "THREAD" in key or "MEMORY" in key or "CPU" in key for key in service.SERVER_SETTINGS
    )


@pytest.fixture
def recoverable(tmp_path, monkeypatch):
    root = tmp_path / "state"
    root.mkdir(mode=0o700)
    for name in (
        "attachments",
        "serving",
        "data/store",
        "product-leases/main",
        "product-leases/validation",
    ):
        (root / name).mkdir(parents=True, mode=0o700)
    (root / "admission.lock").touch(mode=0o600)
    (root / "borrowers.lock").touch(mode=0o600)
    (root / "data/store/MANIFEST").write_text("cold-owned-data")
    binary = tmp_path / "surreal"
    binary.write_bytes(b"exact-native-binary")
    installer = tmp_path / "lctx"
    installer.write_bytes(b"exact-native-installer")
    unit = tmp_path / "unit"
    unit.write_text("exact-owned-unit")
    monkeypatch.setattr(service, "BINARY_SHA256", service.file_sha256(binary))
    monkeypatch.setattr(service, "_unit_path", lambda: unit)
    record = {
        "schema": 1,
        "phase": "ready",
        "installation_id": str(uuid.uuid4()),
        "service_generation": list(range(32)),
        "unit": service.UNIT,
        "namespace": service.NAMESPACE,
        "databases": list(service.DATABASES),
        "export_batch_size": 1,
        "state_root": str(root),
        "endpoint": "http://127.0.0.1:29999",
        "grpc_endpoint": "grpc://127.0.0.1:29999",
        "installer": str(installer),
        "binary": {
            "path": str(binary),
            "sha256": service.BINARY_SHA256,
            "version": service.VERSION,
        },
    }
    service.private_json(root / "installation.json", record)
    for database in service.DATABASES:
        for role in ("runtime", "installer"):
            service.private_json(
                root / f"{database}-{role}.json",
                {
                    "private": "credential",
                    "database": database,
                    "endpoint": record["grpc_endpoint"],
                    "namespace": service.NAMESPACE,
                    "cache_database": database,
                    "service_generation": record["service_generation"],
                    "authentication": "root" if role == "installer" else "database",
                    "reuse": {
                        "database": database,
                        "capacity_bytes": service.PRODUCT_CAPACITY_BYTES,
                        "lease_directory": str(root / "product-leases" / database),
                    },
                },
            )
    (root / "server.env").write_text("SURREAL_PASS=private-root-secret\n")
    (root / "server.env").chmod(0o600)
    service.private_json(root / "main-selected.json", {"publication": "retained-selection"})
    service.private_json(root / "serving/control.json", {"handle": "exact-retained-handle"})
    actions = []
    monkeypatch.setattr(
        service.Installation,
        "check",
        lambda self, **kw: actions.append("ready") or {"outcome": "passed"},
    )
    monkeypatch.setattr(
        service, "_run_installer", lambda inst, exe, action="init": actions.append(action)
    )
    monkeypatch.setattr(service, "_stop_owned", lambda inst: actions.append("stop"))
    monkeypatch.setattr(service, "_start_owned", lambda inst: actions.append("start"))
    monkeypatch.setattr(
        service,
        "_schema_identities",
        lambda inst: {db: {"schema": "a" * 64, "schema_version": 3} for db in service.DATABASES},
    )
    return service.Installation(root, record), tmp_path / "protected.tar", actions


def test_maintenance_command_retains_attachment_scope_over_inherited_configs(
    recoverable, monkeypatch
):
    installation, _archive, actions = recoverable
    monkeypatch.setattr(service, "verify_binary", lambda env: Path(installation.record["binary"]["path"]))
    monkeypatch.setenv(
        "LCTX_SURREAL_SERVICE_CONFIG", str(installation.directory / "installation.json")
    )
    monkeypatch.setenv("LCTX_COMPILER_RUNTIME_CONFIG", "/foreign/compiler.json")
    monkeypatch.setenv("LCTX_SURREAL_TEST_CONFIG", "/foreign/test.json")
    monkeypatch.setattr(
        service.Installation,
        "runtime",
        lambda self, database="validation", **kw: {
            "endpoint": self.grpc_endpoint,
            "namespace": service.NAMESPACE,
            "database": database,
            "username": "validation_writer",
            "password": "private-control-password",
            "service_generation": self.record["service_generation"],
            "authentication": "database",
        },
    )
    observed = installation.directory.parent / "child-environment.json"
    script = (
        "import json,os,pathlib; "
        f"pathlib.Path({str(observed)!r}).write_text(json.dumps({{"
        "key:os.environ[key] for key in ('LCTX_COMPILER_RUNTIME_CONFIG',"
        "'LCTX_SURREAL_TEST_CONFIG','LCTX_SURREAL_INSTALLER_CONFIG',"
        "'LCTX_SURREAL_MAINTENANCE_TOKEN')}))"
    )
    assert (
        service.main(["maintenance", "--native-clients", "--", sys.executable, "-c", script]) == 0
    )
    environment = json.loads(observed.read_text())
    compiler = Path(environment["LCTX_COMPILER_RUNTIME_CONFIG"])
    test = Path(environment["LCTX_SURREAL_TEST_CONFIG"])
    assert compiler.parent == test.parent
    assert compiler.parent.parent == installation.directory / "attachments"
    assert json.loads(compiler.read_text())["database"] == "validation"
    assert json.loads(test.read_text())["database"] == "validation"
    assert environment["LCTX_SURREAL_INSTALLER_CONFIG"] == str(
        installation.runtime_path(installer=True)
    )
    assert environment["LCTX_SURREAL_MAINTENANCE_TOKEN"]
    assert actions[-1] == "ready"
    assert not service.borrowers(installation)
    assert not (installation.directory / "maintenance.json").exists()


def test_recovery_reconciles_only_named_readers_under_both_host_locks(recoverable, monkeypatch):
    installation, _archive, actions = recoverable
    monkeypatch.setenv(
        "LCTX_SURREAL_SERVICE_CONFIG", str(installation.directory / "installation.json")
    )
    monkeypatch.setattr(service.Installation, "check_daemon", lambda self: actions.append("daemon"))
    pin, backup = "a" * 64, "b" * 64

    def reconcile(current, database, pins, holds):
        assert service.lock_held(current.directory / "admission.lock")
        assert service.lock_held(current.directory / "borrowers.lock")
        assert database == "validation" and pins == [pin] and holds == [backup]
        actions.append("reconcile")

    monkeypatch.setattr(service, "_reconcile_readers", reconcile)
    assert (
        service.main(
            [
                "maintenance",
                "--recover",
                "--reconcile-database",
                "validation",
                "--reconcile-pin",
                pin,
                "--reconcile-backup-hold",
                backup,
            ]
        )
        == 0
    )
    assert actions == [
        "daemon",
        "close-admission",
        "reconcile",
        "drain",
        "ready",
        "check",
        "open-admission",
        "ready",
    ]
    assert not (installation.directory / "maintenance.json").exists()


def test_named_reader_recovery_refuses_unknown_child_cleanup(recoverable, monkeypatch):
    installation, _archive, actions = recoverable
    monkeypatch.setenv(
        "LCTX_SURREAL_SERVICE_CONFIG", str(installation.directory / "installation.json")
    )
    monkeypatch.setattr(service.Installation, "check_daemon", lambda self: None)
    attachment = installation.directory / "attachments" / str(uuid.uuid4())
    attachment.mkdir()
    service.private_json(
        attachment / "record.json",
        {
            "released": True,
            "command": {"argv": ["native-control"]},
            "command_cleanup": {"status": "unknown"},
        },
    )
    monkeypatch.setattr(
        service,
        "_reconcile_readers",
        lambda *a: pytest.fail("uncertain child allowed reconciliation"),
    )
    assert (
        service.main(
            [
                "maintenance",
                "--recover",
                "--reconcile-database",
                "validation",
                "--reconcile-pin",
                "a" * 64,
            ]
        )
        == 75
    )
    assert actions == ["close-admission", "close-admission"]
    assert (installation.directory / "maintenance.json").exists()


@pytest.mark.parametrize(
    "field,value",
    [
        ("endpoint", "grpc://127.0.0.1:30001"),
        ("namespace", "foreign"),
        ("database", "main"),
        ("cache_database", "main"),
        ("service_generation", [255] * 32),
        ("authentication", "database"),
        ("reuse", {"database": "validation", "capacity_bytes": 1, "lease_directory": "/foreign"}),
    ],
)
def test_recovery_preflights_every_installer_scope_before_native_effects(
    recoverable,
    monkeypatch,
    field,
    value,
):
    installation, _archive, actions = recoverable
    monkeypatch.setenv(
        "LCTX_SURREAL_SERVICE_CONFIG", str(installation.directory / "installation.json")
    )
    path = installation.runtime_path("validation", installer=True)
    config = json.loads(path.read_text())
    config[field] = value
    service.private_json(path, config)
    monkeypatch.setattr(
        service.Installation,
        "check_daemon",
        lambda self: pytest.fail("invalid target reached daemon check"),
    )
    assert (
        service.main(
            [
                "maintenance",
                "--recover",
                "--reconcile-database",
                "validation",
                "--reconcile-pin",
                "a" * 64,
            ]
        )
        == 75
    )
    assert not actions
    assert (installation.directory / "maintenance.json").exists()


def test_installer_preflights_second_scope_before_first_subprocess(recoverable, monkeypatch):
    installation, _archive, _actions = recoverable
    installer = Path(installation.record["installer"])
    installer.chmod(0o700)
    path = installation.runtime_path("validation", installer=True)
    service.private_json(
        path, json.loads(installation.runtime_path("main", installer=True).read_text())
    )
    monkeypatch.setattr(
        service.subprocess,
        "run",
        lambda *a, **kw: pytest.fail("swapped config reached native installer"),
    )
    with pytest.raises(service.FixtureBlocked, match="does not match installation"):
        _run_installer(installation, installer, "close-admission")


def test_reconciliation_revalidates_selected_installer_before_subprocess(recoverable, monkeypatch):
    installation, _archive, _actions = recoverable
    path = installation.runtime_path("validation", installer=True)
    config = json.loads(path.read_text())
    config["service_generation"] = [255] * 32
    service.private_json(path, config)
    monkeypatch.setattr(
        service.subprocess,
        "run",
        lambda *a, **kw: pytest.fail("stale config reached native reconciliation"),
    )
    with pytest.raises(service.FixtureBlocked, match="does not match installation"):
        service._reconcile_readers(installation, "validation", ["a" * 64], [])


def test_named_reconciliation_failure_preserves_closed_admission(recoverable, monkeypatch):
    installation, _archive, actions = recoverable
    monkeypatch.setenv(
        "LCTX_SURREAL_SERVICE_CONFIG", str(installation.directory / "installation.json")
    )
    monkeypatch.setattr(service.Installation, "check_daemon", lambda self: None)

    def refuse(*args):
        raise service.FixtureFailed("reconciliation", "named native guard refused")

    monkeypatch.setattr(service, "_reconcile_readers", refuse)
    assert (
        service.main(
            [
                "maintenance",
                "--recover",
                "--reconcile-database",
                "validation",
                "--reconcile-pin",
                "a" * 64,
            ]
        )
        == 1
    )
    assert actions == ["close-admission", "close-admission"]
    marker = json.loads((installation.directory / "maintenance.json").read_text())
    assert marker["reconciliation"]["pins"] == ["a" * 64]
    assert marker["reconciliation"]["database"] == "validation"


@pytest.mark.parametrize(
    "args",
    [
        ["maintenance", "--reconcile-database", "validation", "--reconcile-pin", "a" * 64],
        ["maintenance", "--recover", "--reconcile-pin", "a" * 64],
        ["maintenance", "--recover", "--reconcile-database", "validation"],
        [
            "maintenance",
            "--recover",
            "--reconcile-database",
            "validation",
            "--reconcile-pin",
            "invalid",
        ],
    ],
)
def test_named_reconciliation_requires_explicit_recovery_scope_and_inventory(args):
    with pytest.raises(SystemExit) as error:
        service.main(args)
    assert error.value.code == 2


def test_protected_backup_is_cold_private_complete_and_reopens_after_check(recoverable):
    installation, archive, actions = recoverable
    result = service.backup_service(installation, archive)
    assert actions[:5] == ["close-admission", "drain", "ready", "check", "stop"]
    assert actions[-4:] == ["start", "drain", "ready", "open-admission"]
    assert archive.stat().st_mode & 0o777 == 0o600
    assert not (installation.directory / "maintenance.json").exists()
    with tarfile.open(archive) as tar:
        names = set(tar.getnames())
        assert {
            "state/server.env",
            "assets/surreal",
            "assets/lctx",
            "assets/service-unit",
            "state/main-selected.json",
            "state/serving/control.json",
            "state/data/store/MANIFEST",
        } <= names
        assert not any(
            name in names
            for name in ("state/admission.lock", "state/borrowers.lock", "state/maintenance.json")
        )
        source = tar.extractfile("state/server.env")
        assert source is not None
        assert b"private-root-secret" in source.read()
    pointer = result["recovery"]
    assert pointer["sha256"] == service.file_sha256(archive)
    assert (
        pointer
        == json.loads((installation.directory / "installation.json").read_text())["recovery"]
    )
    assert "private-root-secret" not in json.dumps(pointer) and "credential" not in json.dumps(
        pointer
    )


def test_restore_default_validates_without_daemon_or_owned_state_effects(recoverable):
    installation, archive, actions = recoverable
    service.backup_service(installation, archive)
    actions.clear()
    before = {
        str(p.relative_to(installation.directory)): p.read_bytes()
        for p in installation.directory.rglob("*")
        if p.is_file()
    }
    result = service.restore_service(installation, archive)
    assert result["outcome"] == "passed" and result["applied"] is False
    assert not actions
    after = {
        str(p.relative_to(installation.directory)): p.read_bytes()
        for p in installation.directory.rglob("*")
        if p.is_file()
    }
    assert before == after


def test_restore_apply_preserves_live_lock_inodes_and_removes_only_checked_predecessor(recoverable):
    installation, archive, actions = recoverable
    service.backup_service(installation, archive)
    locks = {
        name: (installation.directory / name).stat().st_ino
        for name in ("admission.lock", "borrowers.lock")
    }
    (installation.directory / "data/store/MANIFEST").write_text("newer-data")
    actions.clear()
    result = service.restore_service(installation, archive, apply=True)
    assert result["applied"]
    assert (installation.directory / "data/store/MANIFEST").read_text() == "cold-owned-data"
    assert {name: (installation.directory / name).stat().st_ino for name in locks} == locks
    assert (
        actions.index("stop")
        < actions.index("start")
        < actions.index("check")
        < actions.index("open-admission")
    )
    assert not list(installation.directory.glob(".recovery-*"))
    assert not (installation.directory / "maintenance.json").exists()


def test_restore_failure_retains_predecessor_and_closed_admission(recoverable, monkeypatch):
    installation, archive, actions = recoverable
    service.backup_service(installation, archive)
    (installation.directory / "data/store/MANIFEST").write_text("predecessor-current-consumer")

    def fail_start(inst):
        actions.append("failed-start")
        raise service.FixtureBlocked("launch", "failed replacement")

    monkeypatch.setattr(service, "_start_owned", fail_start)
    actions.clear()
    with pytest.raises(service.FixtureBlocked, match="failed replacement"):
        service.restore_service(installation, archive, apply=True)
    assert "open-admission" not in actions
    assert (installation.directory / "maintenance.json").exists()
    previous = list(installation.directory.glob(".recovery-predecessor-*"))
    assert len(previous) == 1
    assert (previous[0] / "data/store/MANIFEST").read_text() == "predecessor-current-consumer"
    marker = json.loads((installation.directory / "maintenance.json").read_text())
    assert marker["recovery"]["predecessors"] == [str(previous[0])]
    assert marker["recovery"]["archive"] == str(archive)
    assert (installation.directory / "installation.json").exists()


def test_recovery_retires_only_durably_named_owned_predecessors(recoverable):
    installation, _archive, _actions = recoverable
    named = installation.directory / (".recovery-predecessor-" + str(uuid.uuid4()))
    named.mkdir()
    unmentioned = installation.directory / (".recovery-predecessor-" + str(uuid.uuid4()))
    unmentioned.mkdir()
    marker = {"recovery": {"kind": "restore", "predecessors": [str(named)], "staging": []}}
    service._retire_recovery_predecessors(installation, marker)
    assert not named.exists() and unmentioned.exists()
    unsafe = installation.directory.parent / (".recovery-predecessor-" + str(uuid.uuid4()))
    unsafe.mkdir()
    marker["recovery"]["predecessors"] = [str(unmentioned), str(unsafe)]
    with pytest.raises(service.FixtureBlocked, match="outside its exact owned scope"):
        service._retire_recovery_predecessors(installation, marker)
    assert unmentioned.exists() and unsafe.exists()


def test_restore_staging_failure_has_durable_repair_owner_before_copy(recoverable, monkeypatch):
    installation, archive, actions = recoverable
    service.backup_service(installation, archive)

    def fail_copy(source, target, *args, **kwargs):
        target.mkdir()
        (target / "partial").write_text("retained partial staging")
        raise OSError("staging storage unavailable")

    monkeypatch.setattr(service.shutil, "copytree", fail_copy)
    actions.clear()
    with pytest.raises(OSError, match="staging storage unavailable"):
        service.restore_service(installation, archive, apply=True)
    marker = json.loads((installation.directory / "maintenance.json").read_text())
    stage = Path(marker["recovery"]["staging"][0])
    assert (stage / "partial").exists()
    assert (installation.directory / "data/store/MANIFEST").read_text() == "cold-owned-data"
    assert "stop" not in actions and "open-admission" not in actions


def test_incomplete_installation_proves_daemon_before_any_admin_query(recoverable, monkeypatch):
    installation, _archive, _actions = recoverable
    installation.record["phase"] = "installing"
    service.private_json(installation.directory / "installation.json", installation.record)
    installer = Path(installation.record["installer"])
    installer.chmod(0o700)
    monkeypatch.setattr(
        service, "verify_binary", lambda *a, **kw: Path(installation.record["binary"]["path"])
    )
    monkeypatch.setattr(service, "systemd_available", lambda **kw: None)
    monkeypatch.setattr(service, "systemctl", lambda *a, **kw: subprocess.CompletedProcess(a, 0))
    monkeypatch.setattr(
        service.urllib.request,
        "urlopen",
        lambda *a, **kw: contextlib.nullcontext(SimpleNamespace(status=200)),
    )

    def refuse(self):
        raise service.FixtureBlocked("identity", "not the exact owned daemon")

    monkeypatch.setattr(service.Installation, "check_daemon", refuse)
    monkeypatch.setattr(
        service, "sql", lambda *a, **kw: pytest.fail("admin query preceded daemon identity proof")
    )
    with pytest.raises(service.FixtureBlocked, match="exact owned daemon"):
        service.install(
            installer,
            env={"LCTX_SURREAL_SERVICE_CONFIG": str(installation.directory / "installation.json")},
        )


def test_dead_bootstrap_starts_exact_service_before_native_close(recoverable, monkeypatch):
    installation, _archive, actions = recoverable
    binary = Path(installation.record["binary"]["path"])
    service._unit_path().write_text(service.unit_text(installation.directory, binary, 29999))
    monkeypatch.setattr(service, "verify_binary", lambda *a, **kw: binary)
    monkeypatch.setattr(
        service, "unit_properties", lambda *a: {"ActiveState": "failed", "MainPID": "0"}
    )
    checks = 0

    def daemon(self):
        nonlocal checks
        checks += 1
        if checks == 1:
            raise service.FixtureBlocked("readiness", "owned service is failed")
        actions.append("daemon")

    monkeypatch.setattr(service.Installation, "check_daemon", daemon)
    with service.maintenance(installation, restart=True):
        actions.append("command")
    assert actions[:5] == ["stop", "start", "daemon", "close-admission", "drain"]
    assert not (installation.directory / "maintenance.json").exists()


def test_dead_bootstrap_refuses_changed_unit_without_start(recoverable, monkeypatch):
    installation, _archive, actions = recoverable
    monkeypatch.setattr(
        service.Installation,
        "check_daemon",
        lambda self: (_ for _ in ()).throw(service.FixtureBlocked("readiness", "failed")),
    )
    monkeypatch.setattr(
        service, "unit_properties", lambda *a: {"ActiveState": "failed", "MainPID": "0"}
    )
    monkeypatch.setattr(
        service, "verify_binary", lambda *a, **kw: Path(installation.record["binary"]["path"])
    )
    with pytest.raises(service.FixtureBlocked, match="unit differs"):
        with service.maintenance(installation, restart=True):
            pytest.fail("changed unit was admitted")
    assert "start" not in actions
    assert (installation.directory / "maintenance.json").exists()


def test_dead_bootstrap_start_failure_preserves_closed_maintenance(recoverable, monkeypatch):
    installation, _archive, actions = recoverable
    binary = Path(installation.record["binary"]["path"])
    service._unit_path().write_text(service.unit_text(installation.directory, binary, 29999))
    monkeypatch.setattr(service, "verify_binary", lambda *a, **kw: binary)
    monkeypatch.setattr(
        service, "unit_properties", lambda *a: {"ActiveState": "failed", "MainPID": "0"}
    )
    monkeypatch.setattr(
        service.Installation,
        "check_daemon",
        lambda self: (_ for _ in ()).throw(service.FixtureBlocked("readiness", "failed")),
    )
    monkeypatch.setattr(
        service,
        "_start_owned",
        lambda inst: (_ for _ in ()).throw(service.FixtureBlocked("launch", "start failed")),
    )
    with pytest.raises(service.FixtureBlocked, match="start failed"):
        with service.maintenance(installation, restart=True):
            pytest.fail("failed start was admitted")
    assert "open-admission" not in actions
    assert (installation.directory / "maintenance.json").exists()


def test_stop_accepts_failed_unit_only_when_no_main_process_or_descendants(monkeypatch):
    monkeypatch.setattr(service, "systemctl", lambda *a: subprocess.CompletedProcess(a, 0))
    state = {"ActiveState": "failed", "MainPID": "0", "ControlGroup": ""}
    monkeypatch.setattr(service, "unit_properties", lambda *a: state)
    service._stop_owned(SimpleNamespace())
    state["MainPID"] = "123"
    with pytest.raises(service.FixtureBlocked, match="stop is not confirmed"):
        service._stop_owned(SimpleNamespace())


@pytest.mark.parametrize("path", ["../escape", "/absolute", "state/../escape", "state/./escape"])
def test_recovery_rejects_unsafe_paths(path):
    with pytest.raises(service.FixtureBlocked, match="unsafe path"):
        service._safe_recovery_path(path)


@pytest.mark.parametrize("kind", ["symlink", "hardlink", "traversal", "digest", "generation"])
def test_recovery_rejects_tampered_assets_before_stop(recoverable, kind):
    installation, archive, actions = recoverable
    service.backup_service(installation, archive)
    with tarfile.open(archive) as tar:
        contents = {}
        for member in tar.getmembers():
            source = tar.extractfile(member)
            assert source is not None
            contents[member.name] = source.read()
    metadata = json.loads(contents["recovery.json"])
    target = "state/data/store/MANIFEST"
    if kind == "generation":
        metadata["identity"]["service_generation"] = [255] * 32
    if kind == "digest":
        contents[target] = b"wrong-same-size"
        metadata["files"][target]["size"] = len(contents[target])
    if kind == "traversal":
        contents["state/../escape"] = contents.pop(target)
        metadata["files"]["state/../escape"] = metadata["files"].pop(target)
        target = "state/../escape"
    contents["recovery.json"] = json.dumps(metadata).encode()
    evil = archive.with_name("tampered.tar")
    with tarfile.open(evil, "w") as tar:
        for name, data in contents.items():
            header = tarfile.TarInfo(name)
            header.size = len(data)
            if name == target and kind in ("symlink", "hardlink"):
                header.type = tarfile.SYMTYPE if kind == "symlink" else tarfile.LNKTYPE
                header.linkname = "/outside"
                header.size = 0
            tar.addfile(header, io.BytesIO(data) if header.isfile() else None)
    evil.chmod(0o600)
    actions.clear()
    with pytest.raises(service.FixtureBlocked):
        service.restore_service(installation, evil, apply=True)
    assert not actions


def test_backup_failure_restarts_but_preserves_closed_maintenance(recoverable, monkeypatch):
    installation, archive, actions = recoverable

    def refuse(inst):
        raise service.FixtureBlocked("recovery", "unsafe source")

    monkeypatch.setattr(service, "_recovery_sources", refuse)
    with pytest.raises(service.FixtureBlocked, match="unsafe source"):
        service.backup_service(installation, archive)
    assert "stop" in actions and "start" in actions and "open-admission" not in actions
    assert (installation.directory / "maintenance.json").exists()
    assert not archive.exists()


@pytest.mark.parametrize("shared", [False, True])
def test_backup_refuses_overwrite_inside_state_and_live_product_lease(recoverable, shared):
    installation, archive, actions = recoverable
    with pytest.raises(service.FixtureBlocked, match="outside"):
        service.backup_service(installation, installation.directory / "backup.tar")
    archive.touch()
    with pytest.raises(service.FixtureBlocked, match="already exists"):
        service.backup_service(installation, archive)
    archive.unlink()
    lease = installation.directory / "product-leases/main/lifecycle.lock"
    fd = service.hold_lock(lease)
    if shared:
        fcntl.flock(fd, fcntl.LOCK_SH)
    try:
        with pytest.raises(service.FixtureBlocked, match="product lease"):
            service.backup_service(installation, archive)
    finally:
        os.close(fd)
    assert "stop" not in actions


def test_protected_recovery_captures_declared_external_selection_and_coordination(recoverable):
    installation, archive, _actions = recoverable
    external = installation.directory.parent / "external-selected.json"
    service.private_json(external, {"publication": "original-external-selection"})
    cfg = installation.directory / "main-runtime.json"
    service.private_json(
        cfg, {"private": "credential", "database": "main", "selection": str(external)}
    )
    coordination = installation.directory.parent / "owned-extra-coordination"
    coordination.mkdir(mode=0o700)
    (coordination / "lifecycle.lock").touch(mode=0o600)
    (coordination / "state.json").write_text("original-coordination")
    installation.record["recovery_assets"] = [{"kind": "coordination", "path": str(coordination)}]
    service.private_json(installation.directory / "installation.json", installation.record)
    service.backup_service(installation, archive)
    external.write_text("new-selection")
    (coordination / "state.json").write_text("new-coordination")
    result = service.restore_service(installation, archive, apply=True)
    assert result["applied"]
    assert json.loads(external.read_text())["publication"] == "original-external-selection"
    assert (coordination / "state.json").read_text() == "original-coordination"
    assert not list(installation.directory.parent.glob(".recovery-*"))


def test_backup_detects_held_external_selection_lifecycle_lock(recoverable):
    installation, archive, actions = recoverable
    external = installation.directory.parent / "selected.json"
    service.private_json(installation.directory / "main-runtime.json", {"selection": str(external)})
    fd = service.hold_lock(external.with_suffix(".selection.lock"))
    try:
        with pytest.raises(service.FixtureBlocked, match="product lease"):
            service.backup_service(installation, archive)
    finally:
        os.close(fd)
    assert "stop" not in actions and not archive.exists()


def test_recovery_preserves_absent_external_selection_and_refuses_symlink_alias(recoverable):
    installation, archive, _actions = recoverable
    external = installation.directory.parent / "not-selected-yet.json"
    cfg = installation.directory / "main-runtime.json"
    service.private_json(
        cfg, {"private": "credential", "database": "main", "selection": str(external)}
    )
    service.backup_service(installation, archive)
    external.write_text("later-selection")
    service.restore_service(installation, archive, apply=True)
    assert not external.exists()
    alias = installation.directory.parent / "alias"
    alias.symlink_to(installation.directory, target_is_directory=True)
    with pytest.raises(service.FixtureBlocked, match="outside"):
        service.backup_service(installation, alias / "recursive.tar")


@pytest.fixture
def upgradable(recoverable, monkeypatch):
    import storage_lifecycle
    installation, archive, actions = recoverable
    Path(installation.record["installer"]).chmod(0o700)
    candidate = installation.directory.parent / "lctx-v4"
    candidate.write_bytes(b"new-owned-installer")
    candidate.chmod(0o700)
    for db in service.DATABASES:
        for installer in (False, True):
            cfg = installation.runtime(db, installer=installer)
            cfg.update(username="owned_root" if installer else db + "_writer",
                       password="old_root_password" if installer else db + "_old_password",
                       viewer_username=db + "_viewer", viewer_password=db + "_old_viewer")
            service.private_json(installation.runtime_path(db, installer=installer), cfg)
    service._upgrade_write_private(installation.directory / "server.env",
        "SURREAL_USER=owned_root\nSURREAL_PASS=old_root_password\n")
    monkeypatch.setattr(storage_lifecycle, "admission", lambda *a, **kw: contextlib.nullcontext())
    monkeypatch.setattr(service, "_bootstrap_owned", lambda inst: actions.append("bootstrap"))
    monkeypatch.setattr(service, "_upgrade_restart", lambda inst: actions.append("restart-upgrade"))
    monkeypatch.setattr(service, "_schema_identities", lambda inst: {
        db: {"schema": service.UPGRADE_SOURCE_SCHEMA, "schema_version": 3} for db in service.DATABASES})
    def cli(inst, generation, db, action, **kw):
        assert service.lock_held(inst.directory / "admission.lock")
        if action != "schema":
            assert service.lock_held(inst.directory / "borrowers.lock")
        actions.append((action, db))
        if action in ("upgrade", "check"):
            assert inst.record["installer"] != generation["path"]
        return {"schema_version": 4, "schema": "b" * 64} if action == "schema" else {"ready": action != "upgrade"}
    monkeypatch.setattr(service, "_upgrade_cli", cli)
    monkeypatch.setattr(service, "sql", lambda inst, statement, **kw: [{"status": "OK", "result": [{
        "schema_version": 4, "schema": "b" * 64,
        "generation": bytes(inst.record["service_generation"]).hex()}]}])
    accepted = {"root": "old_root_password"}
    monkeypatch.setattr(service, "_upgrade_authenticates", lambda inst, cfg:
                        cfg["password"] == accepted["root"])
    def rotate(inst, root_cfg, old, new, *, scope, role, operation):
        assert service.lock_held(inst.directory / "borrowers.lock")
        marker = json.loads((inst.directory / "maintenance.json").read_text())
        plan = json.loads(Path(marker["upgrade"]["journal"]).read_text())
        assert new["password"] in json.dumps(plan)
        actions.append(("rotate", scope, new["username"]))
        if scope == "ROOT":
            accepted["root"] = new["password"]
    monkeypatch.setattr(service, "_upgrade_rotate", rotate)
    return installation, candidate, actions


def dead_upgrade_owner(installation):
    path = installation.directory / "maintenance.json"
    marker = json.loads(path.read_text())
    marker["owner"]["boot_id"] = "previous-test-boot"
    service.private_json(path, marker)
    return marker


def test_explicit_upgrade_checkpoints_preserve_generation_and_handoff_last(upgradable):
    installation, candidate, actions = upgradable
    previous = installation.record["installer"]
    generation = installation.record["service_generation"].copy()
    result = service.upgrade_installer(installation, candidate)
    assert result["native_schema_version"] == 4
    assert installation.record["native_schema_version"] == 4
    assert installation.record["service_generation"] == generation
    assert installation.record["installer"] != previous
    assert Path(previous).exists()
    assert actions.index("drain") < actions.index(("upgrade", "main"))
    assert actions.index("restart-upgrade") < actions.index(("upgrade", "main"))
    assert actions.index(("check", "validation")) < actions.index("open-admission")
    assert actions.index(("rotate", "ROOT", "owned_root")) < actions.index(("upgrade", "main"))
    assert actions[actions.index(("upgrade", "main")) - 1] == "restart-upgrade"
    rotations = [item for item in actions if isinstance(item, tuple) and item[0] == "rotate"]
    assert rotations[-1][1:] == ("ROOT", "owned_root")
    assert len(rotations) == 5
    assert not (installation.directory / "maintenance.json").exists()
    plan = next((installation.directory / "native-upgrades").glob("*/plan.json"))
    assert plan.stat().st_mode & 0o077 == 0
    assert "old_root_password" not in json.dumps(result)
    for db in service.DATABASES:
        assert installation.runtime(db)["password"] != db + "_old_password"
        assert installation.runtime(db, installer=True)["password"] != "old_root_password"


def test_upgrade_unknown_native_effect_restarts_before_same_operation_replay(upgradable, monkeypatch):
    installation, candidate, actions = upgradable
    original = service._upgrade_cli
    once = True
    operations = []
    def fail(inst, generation, db, action, **kw):
        nonlocal once
        if action == "upgrade":
            operations.append(kw["operation"])
            if once:
                once = False
                actions.append("unknown-native-effect")
                raise service.FixtureFailed("upgrade", "unknown acknowledgment")
        return original(inst, generation, db, action, **kw)
    monkeypatch.setattr(service, "_upgrade_cli", fail)
    with pytest.raises(service.FixtureFailed, match="unknown acknowledgment"):
        service.upgrade_installer(installation, candidate)
    marker = dead_upgrade_owner(installation)
    assert marker["upgrade"]["checkpoints"]["drained"]
    assert "native_schema_version" not in installation.record
    actions.clear()
    service.upgrade_installer(installation, candidate)
    assert actions[0] == "restart-upgrade"
    assert "drain" not in actions and "close-admission" not in actions
    assert len(set(operations)) == 1


def test_upgrade_refuses_wrong_candidate_changed_journal_and_generic_recovery(upgradable, monkeypatch, capsys):
    installation, candidate, actions = upgradable
    original = service._upgrade_cli
    def fail(inst, generation, db, action, **kw):
        if action == "upgrade":
            raise service.FixtureFailed("upgrade", "intentional failed checkpoint")
        return original(inst, generation, db, action, **kw)
    monkeypatch.setattr(service, "_upgrade_cli", fail)
    with pytest.raises(service.FixtureFailed):
        service.upgrade_installer(installation, candidate)
    marker = dead_upgrade_owner(installation)
    other = candidate.with_name("different-candidate")
    other.write_bytes(b"different")
    other.chmod(0o700)
    actions.clear()
    with pytest.raises(service.FixtureBlocked, match="exact checkpointed"):
        service.upgrade_installer(installation, other)
    assert actions == []
    monkeypatch.setenv("LCTX_SURREAL_SERVICE_CONFIG", str(installation.directory / "installation.json"))
    assert service.main(["maintenance", "--recover"]) == 75
    assert "same --upgrade-installer" in capsys.readouterr().err
    journal = Path(marker["upgrade"]["journal"])
    journal.write_text(journal.read_text() + " ")
    with pytest.raises(service.FixtureBlocked, match="immutable upgrade journal"):
        service.upgrade_installer(installation, candidate)
    assert actions == []


def test_upgrade_rotated_root_acknowledgment_resume_uses_private_plan(upgradable, monkeypatch):
    installation, candidate, actions = upgradable
    rotate = service._upgrade_rotate
    failed = False
    def lost_reply(*args, **kw):
        nonlocal failed
        rotate(*args, **kw)
        if kw["scope"] == "ROOT" and not failed:
            failed = True
            raise service.FixtureFailed("upgrade", "root reply lost")
    monkeypatch.setattr(service, "_upgrade_rotate", lost_reply)
    with pytest.raises(service.FixtureFailed, match="root reply lost"):
        service.upgrade_installer(installation, candidate)
    dead_upgrade_owner(installation)
    actions.clear()
    service.upgrade_installer(installation, candidate)
    assert actions[0] == "restart-upgrade"
    assert [a for a in actions if isinstance(a, tuple) and a[0] == "upgrade"] == [
        ("upgrade", "main"), ("upgrade", "validation")]
    assert installation.record["native_schema_version"] == 4


def test_upgrade_source_mismatch_and_unresolved_attachment_preserve_closed_assets(upgradable, monkeypatch):
    installation, candidate, actions = upgradable
    monkeypatch.setattr(service, "_schema_identities", lambda inst: {
        db: {"schema": "c" * 64, "schema_version": 3} for db in service.DATABASES})
    with pytest.raises(service.FixtureBlocked, match="exact schema3"):
        service.upgrade_installer(installation, candidate)
    assert not any(isinstance(a, tuple) and a[0] == "upgrade" for a in actions)
    assert (installation.directory / "maintenance.json").exists()
    from storage_service import dependencies
    protected = dependencies(installation)["dependencies"]
    assert {row["role"] for row in protected} >= {"unresolved-native-upgrade"}
    assert all("password" not in row for row in protected)


@pytest.mark.parametrize("flag", ["--recover", "--restart", "--check", "--stabilize-installer"])
def test_upgrade_cli_is_mutually_exclusive(flag):
    with pytest.raises(SystemExit):
        service.main(["maintenance", "--upgrade-installer", "/candidate", flag])


def test_upgrade_rotation_reconciles_lost_reply_without_reissuing_ddl(monkeypatch, tmp_path):
    old = {"username": "root", "password": "old", "authentication": "root", "database": "main"}
    new = {**old, "password": "new"}
    accepted = {"password": "old"}
    statements = []
    operation = "f" * 64
    comment = f"lctx-native-upgrade/{operation}/ROOT/root"
    monkeypatch.setattr(service, "_upgrade_authenticates", lambda inst, cfg:
                        cfg["password"] == accepted["password"])
    monkeypatch.setattr(service, "_upgrade_user_definition", lambda *a:
                        {"name": "root", "roles": ["OWNER"], "comment": comment})
    def sql(inst, statement, **kw):
        statements.append(statement)
        assert "DEFINE USER OVERWRITE root ON ROOT PASSWORD" in statement
        accepted["password"] = "new"
        raise service.FixtureBlocked("query", "unknown acknowledgment")
    monkeypatch.setattr(service, "sql", sql)
    service._upgrade_rotate(SimpleNamespace(), old, old, new, scope="ROOT", role="OWNER", operation=operation)
    service._upgrade_rotate(SimpleNamespace(), new, old, new, scope="ROOT", role="OWNER", operation=operation)
    assert len(statements) == 1


def test_upgrade_rotation_unknown_definition_and_transport_do_not_assume_success(monkeypatch):
    old = {"username": "root", "password": "old"}
    new = {**old, "password": "new"}
    monkeypatch.setattr(service, "_upgrade_authenticates", lambda inst, cfg: cfg == new)
    monkeypatch.setattr(service, "_upgrade_user_definition", lambda *a:
                        {"name": "root", "roles": ["OWNER"], "comment": "unrelated"})
    with pytest.raises(service.FixtureBlocked, match="unexpected user definition"):
        service._upgrade_rotate(SimpleNamespace(), old, old, new,
                                scope="ROOT", role="OWNER", operation="f" * 64)


def test_upgrade_cli_rechecks_physical_staged_generation_before_child(tmp_path, monkeypatch):
    target = tmp_path / "physical"
    target.write_bytes(b"candidate")
    target.chmod(0o700)
    alias = tmp_path / "candidate"
    alias.symlink_to(target)
    monkeypatch.setattr(service.subprocess, "run", lambda *a, **kw: pytest.fail("no child before identity admission"))
    with pytest.raises(service.FixtureBlocked, match="generation changed"):
        service._upgrade_cli(None, {"path": str(alias), "sha256": service.file_sha256(target)}, "main", "schema")


@pytest.fixture
def replacement_ready(upgradable, monkeypatch):
    installation, candidate, actions = upgradable
    original_cli = service._upgrade_cli
    def fail_initial(inst, generation, db, action, **kw):
        if action == "upgrade":
            raise service.FixtureFailed("upgrade", "initial declaration failure")
        return original_cli(inst, generation, db, action, **kw)
    monkeypatch.setattr(service, "_upgrade_cli", fail_initial)
    with pytest.raises(service.FixtureFailed, match="initial declaration"):
        service.upgrade_installer(installation, candidate)
    marker = dead_upgrade_owner(installation)
    journal = Path(marker["upgrade"]["journal"])
    original_bytes = journal.read_bytes()
    plan = json.loads(original_bytes)
    successor = candidate.with_name("lctx-v4-corrected")
    successor.write_bytes(b"corrected-owned-installer")
    successor.chmod(0o700)
    generation = bytes(plan["service_generation"]).hex()
    native = {"id": "native_upgrade:" + plan["operation"], "generation": generation,
              "source": service.UPGRADE_SOURCE_SCHEMA, "target": plan["target"]["schema"], "phase": "intent"}
    state = {"markers": {db: {"generation": generation, "schema": service.UPGRADE_SOURCE_SCHEMA,
                              "schema_version": 3, "admission_open": False} for db in service.DATABASES},
             "journals": {"main": {plan["operation"]: native}, "validation": {}},
             "target": "c" * 64, "fail_upgrade": False, "native_calls": []}
    def sql(inst, statement, *, cfg):
        assert service.lock_held(inst.directory / "admission.lock")
        assert service.lock_held(inst.directory / "borrowers.lock")
        db = cfg["database"]
        assert cfg["password"] in (plan["new_root"][db]["password"], plan["new_runtime"][db]["password"])
        if "admission_open FROM native_installation" in statement:
            actions.append(("inspect-marker", db))
            result = [state["markers"][db]]
        elif statement == "INFO FOR DB STRUCTURE;":
            actions.append(("inspect-inventory", db))
            result = {"tables": [{"name": "native_upgrade"}]} if state["journals"][db] else {"tables": []}
        elif "FROM native_upgrade:" in statement:
            operation = statement.split("native_upgrade:", 1)[1].removesuffix(";")
            actions.append(("inspect-intent", db, operation))
            row = state["journals"][db].get(operation)
            result = [row] if row else []
        else:
            result = [{"schema_version": 4, "schema": state["target"], "generation": generation}]
        return [{"status": "OK", "result": result}]
    def cli(inst, executable, db, action, **kw):
        if action == "schema":
            actions.append((action, db))
            return {"schema_version": 4, "schema": state["target"]}
        if action == "upgrade":
            migration = kw.get("native_operation", kw["operation"])
            state["native_calls"].append((db, kw["operation"], migration, kw["config"]))
            if state["fail_upgrade"]:
                state["journals"][db][migration] = {**native, "id": "native_upgrade:" + migration, "target": state["target"]}
                raise service.FixtureFailed("upgrade", "successor declaration failure")
        return original_cli(inst, executable, db, action, **kw)
    original_rotate = service._upgrade_rotate
    def rotate(*args, **kw):
        assert kw["operation"] == plan["operation"], "verify existing rotation identity"
        return original_rotate(*args, **kw)
    monkeypatch.setattr(service, "sql", sql)
    monkeypatch.setattr(service, "_upgrade_cli", cli)
    monkeypatch.setattr(service, "_upgrade_rotate", rotate)
    actions.clear()
    return installation, successor, actions, state, journal, original_bytes


def test_replacement_upgrade_preserves_private_predecessor_and_restarts_before_inspection(replacement_ready):
    installation, successor, actions, state, old_journal, original_bytes = replacement_ready
    old = json.loads(original_bytes)
    result = service.upgrade_installer(installation, successor, replace=True)
    assert result["native_schema_version"] == 4
    assert result["upgrade_operation"] != old["operation"]
    assert old_journal.read_bytes() == original_bytes
    assert Path(old["candidate"]["path"]).exists()
    assert actions[0] == "restart-upgrade"
    assert actions.index(("inspect-marker", "validation")) < actions.index(("schema", "main"))
    assert actions.index(("inspect-marker", "main")) < actions.index(("upgrade", "main"))
    current = json.loads(Path(installation.record["native_upgrade"]["journal"]).read_text())
    assert current["credential_operation"] == old["operation"]
    for key in ("old_runtime", "new_runtime", "old_root", "new_root", "old_environment", "new_environment"):
        assert current[key] == old[key]
    for db in service.DATABASES:
        assert installation.runtime(db) == old["new_runtime"][db]
    assert (installation.directory / "server.env").read_text() == old["new_environment"]
    from storage_service import dependencies
    protected = dependencies(installation)["dependencies"]
    predecessor = [row for row in protected if row["role"] == "native-upgrade-predecessor"]
    assert {row["path"] for row in predecessor} >= {str(old_journal), old["candidate"]["path"], old["previous_installer"]}
    bundles = {row["path"] for row in protected if row["role"] == "native-upgrade-private-assets"}
    assert bundles >= {str(old_journal.parent), str(Path(installation.record["native_upgrade"]["journal"]).parent)}
    assert all("password" not in row for row in protected)


@pytest.fixture
def published_replacement_ready(replacement_ready):
    installation, successor, actions, state, journal, original_bytes = replacement_ready
    old = json.loads(original_bytes)
    operation = old.get("native_operation", old["operation"])
    state["target"] = old["target"]["schema"]
    state["markers"]["main"].update(schema_version=4, schema=state["target"])
    state["journals"]["main"][operation]["phase"] = "published"
    state["journals"]["validation"][operation] = {
        **state["journals"]["main"][operation], "phase": "intent"}
    marker_path = installation.directory / "maintenance.json"
    marker = json.loads(marker_path.read_text())
    marker["upgrade"]["checkpoints"]["scope-main"] = True
    service.private_json(marker_path, marker)
    return replacement_ready


def test_published_replacement_keeps_native_operation_and_skips_completed_scope(published_replacement_ready):
    installation, successor, actions, state, journal, original_bytes = published_replacement_ready
    old = json.loads(original_bytes)
    result = service.upgrade_installer(installation, successor, replace=True)
    current = json.loads(Path(installation.record["native_upgrade"]["journal"]).read_text())
    assert result["upgrade_operation"] != old["operation"]
    assert current["native_operation"] == old["operation"]
    assert current["credential_operation"] == old["operation"]
    assert current["completed_scopes"] == ["main"]
    assert current["target"] == old["target"]
    assert journal.read_bytes() == original_bytes
    assert actions[0] == "restart-upgrade"
    assert ("upgrade", "main") not in actions
    assert len(state["native_calls"]) == 1
    db, host_operation, migration, config = state["native_calls"][0]
    assert (db, migration) == ("validation", old["operation"])
    assert host_operation == current["operation"] != migration
    assert config.parent.name == host_operation
    assert installation.record["native_upgrade"]["native_operation"] == migration


@pytest.mark.parametrize("change", ["hash", "complete-target"])
def test_published_replacement_refuses_any_complete_target_change(published_replacement_ready, monkeypatch, change):
    installation, successor, actions, state, journal, original_bytes = published_replacement_ready
    cli = service._upgrade_cli
    def schema(*args, **kw):
        result = cli(*args, **kw)
        if args[3] == "schema":
            return {**result, **({"schema": "d" * 64} if change == "hash" else {"other": "changed"})}
        return result
    monkeypatch.setattr(service, "_upgrade_cli", schema)
    with pytest.raises(service.FixtureBlocked, match="identical complete schema target"):
        service.upgrade_installer(installation, successor, replace=True)
    assert actions[0] == "restart-upgrade" and state["native_calls"] == []
    assert journal.read_bytes() == original_bytes
    assert len(list((installation.directory / "native-upgrades").glob("*/plan.json"))) == 1


@pytest.mark.parametrize("phase", ["declarations", "translated", "published"])
def test_published_replacement_still_refuses_advanced_unpublished_scope(published_replacement_ready, phase):
    installation, successor, actions, state, journal, original_bytes = published_replacement_ready
    old = json.loads(original_bytes)
    state["journals"]["validation"][old["operation"]]["phase"] = phase
    with pytest.raises(service.FixtureBlocked, match="advanced native"):
        service.upgrade_installer(installation, successor, replace=True)
    assert state["native_calls"] == [] and journal.read_bytes() == original_bytes


@pytest.mark.parametrize("change", ["checkpoint", "native-phase", "native-generation", "native-absent"])
def test_published_replacement_requires_every_completed_identity_proof(published_replacement_ready, change):
    installation, successor, actions, state, journal, original_bytes = published_replacement_ready
    old = json.loads(original_bytes)
    if change == "checkpoint":
        marker_path = installation.directory / "maintenance.json"
        marker = json.loads(marker_path.read_text())
        del marker["upgrade"]["checkpoints"]["scope-main"]
        service.private_json(marker_path, marker)
    elif change == "native-absent":
        state["journals"]["main"].clear()
    else:
        state["journals"]["main"][old["operation"]]["phase" if change == "native-phase" else "generation"] = (
            "intent" if change == "native-phase" else "d" * 64)
    with pytest.raises(service.FixtureBlocked, match="exact closed|advanced native"):
        service.upgrade_installer(installation, successor, replace=True)
    assert actions[0] == "restart-upgrade" and state["native_calls"] == []
    assert journal.read_bytes() == original_bytes


def test_published_replacement_failure_preserves_checkpoint_on_resume(published_replacement_ready):
    installation, successor, actions, state, journal, original_bytes = published_replacement_ready
    state["fail_upgrade"] = True
    with pytest.raises(service.FixtureFailed):
        service.upgrade_installer(installation, successor, replace=True)
    marker = dead_upgrade_owner(installation)
    original_operation = json.loads(original_bytes)["operation"]
    assert marker["upgrade"]["checkpoints"]["scope-main"]
    assert marker["upgrade"]["native_operation"] == original_operation
    state["fail_upgrade"] = False
    state["native_calls"].clear()
    actions.clear()
    service.upgrade_installer(installation, successor)
    assert actions[0] == "restart-upgrade" and ("upgrade", "main") not in actions
    assert [(db, native) for db, _, native, _ in state["native_calls"]] == [("validation", original_operation)]
    assert journal.read_bytes() == original_bytes


def test_published_successor_cannot_lose_immutable_scope_checkpoint(published_replacement_ready):
    installation, successor, actions, state, journal, original_bytes = published_replacement_ready
    state["fail_upgrade"] = True
    with pytest.raises(service.FixtureFailed):
        service.upgrade_installer(installation, successor, replace=True)
    marker = dead_upgrade_owner(installation)
    del marker["upgrade"]["checkpoints"]["scope-main"]
    service.private_json(installation.directory / "maintenance.json", marker)
    actions.clear()
    with pytest.raises(service.FixtureBlocked, match="immutable completed scope"):
        service.upgrade_installer(installation, successor)
    assert actions == []


def test_retained_migration_can_publish_later_scope_across_immutable_host_ancestors(published_replacement_ready):
    installation, successor, actions, state, journal, original_bytes = published_replacement_ready
    state["fail_upgrade"] = True
    with pytest.raises(service.FixtureFailed):
        service.upgrade_installer(installation, successor, replace=True)
    marker = dead_upgrade_owner(installation)
    first_journal = Path(marker["upgrade"]["journal"])
    first_bytes = first_journal.read_bytes()
    native_operation = marker["upgrade"]["native_operation"]
    # An exact resume completed validation and checkpointed it before a later host
    # check failed. Original native journal identity stays unchanged throughout.
    state["markers"]["validation"].update(schema_version=4, schema=state["target"])
    state["journals"]["validation"][native_operation]["phase"] = "published"
    marker["upgrade"]["checkpoints"]["scope-validation"] = True
    service.private_json(installation.directory / "maintenance.json", marker)
    third = successor.with_name("lctx-v4-host-repair")
    third.write_bytes(b"third-host-repair")
    third.chmod(0o700)
    state["fail_upgrade"] = False
    state["native_calls"].clear()
    service.upgrade_installer(installation, third, replace=True)
    current = json.loads(Path(installation.record["native_upgrade"]["journal"]).read_text())
    assert current["native_operation"] == native_operation
    assert set(current["completed_scopes"]) == set(service.DATABASES)
    assert len(current["predecessors"]) == 2
    assert state["native_calls"] == []
    assert journal.read_bytes() == original_bytes and first_journal.read_bytes() == first_bytes


def test_published_replacement_refuses_cross_generation_pretranslation_ancestor(replacement_ready):
    installation, successor, actions, state, journal, original_bytes = replacement_ready
    state["fail_upgrade"] = True
    with pytest.raises(service.FixtureFailed):
        service.upgrade_installer(installation, successor, replace=True)
    marker = dead_upgrade_owner(installation)
    current_operation = marker["upgrade"]["operation"]
    # The current migration reached publication, but an older, different-target
    # predecessor still has only its original intent and must remain exact.
    state["markers"]["main"].update(schema_version=4, schema=state["target"])
    state["journals"]["main"][current_operation]["phase"] = "published"
    marker["upgrade"]["checkpoints"]["scope-main"] = True
    service.private_json(installation.directory / "maintenance.json", marker)
    old_operation = json.loads(original_bytes)["operation"]
    state["journals"]["main"][old_operation]["generation"] = "d" * 64
    third = successor.with_name("lctx-v4-ancestor-check")
    third.write_bytes(b"third-ancestor-check")
    third.chmod(0o700)
    state["fail_upgrade"] = False
    state["native_calls"].clear()
    with pytest.raises(service.FixtureBlocked, match="advanced native"):
        service.upgrade_installer(installation, third, replace=True)
    assert state["native_calls"] == [] and journal.read_bytes() == original_bytes
    assert len(list((installation.directory / "native-upgrades").glob("*/plan.json"))) == 2


@pytest.mark.parametrize("db", service.DATABASES)
@pytest.mark.parametrize("field,value", [("schema_version", 4), ("schema", "d" * 64),
                                         ("generation", "d" * 64), ("admission_open", True)])
def test_replacement_refuses_changed_source_before_successor_creation(replacement_ready, db, field, value):
    installation, successor, actions, state, journal, original_bytes = replacement_ready
    state["markers"][db][field] = value
    with pytest.raises(service.FixtureBlocked, match="exact closed schema3"):
        service.upgrade_installer(installation, successor, replace=True)
    assert actions[0] == "restart-upgrade"
    assert ("schema", "main") not in actions
    assert journal.read_bytes() == original_bytes
    assert len(list((installation.directory / "native-upgrades").glob("*/plan.json"))) == 1


@pytest.mark.parametrize("field,value", [("phase", "declarations"), ("phase", "translated"),
    ("phase", "published"), ("phase", "unknown"), ("target", "d" * 64), ("source", "d" * 64)])
def test_replacement_refuses_advanced_or_changed_native_journal(replacement_ready, field, value):
    installation, successor, actions, state, journal, original_bytes = replacement_ready
    old = json.loads(original_bytes)
    state["journals"]["main"][old["operation"]][field] = value
    with pytest.raises(service.FixtureBlocked, match="advanced native"):
        service.upgrade_installer(installation, successor, replace=True)
    assert actions[0] == "restart-upgrade" and ("schema", "main") not in actions
    assert journal.read_bytes() == original_bytes


def test_replacement_refuses_live_owner_and_changed_private_credentials(replacement_ready):
    installation, successor, actions, state, journal, original_bytes = replacement_ready
    marker_path = installation.directory / "maintenance.json"
    marker = json.loads(marker_path.read_text())
    marker["owner"] = service.ProcessIdentity.of().to_json()
    service.private_json(marker_path, marker)
    with pytest.raises(service.FixtureBlocked, match="still alive"):
        service.upgrade_installer(installation, successor, replace=True)
    assert actions == []
    dead_upgrade_owner(installation)
    cfg = installation.runtime("main")
    cfg["password"] = "changed-private-credential"
    service.private_json(installation.runtime_path("main"), cfg)
    with pytest.raises(service.FixtureBlocked, match="private authentication"):
        service.upgrade_installer(installation, successor, replace=True)
    assert actions == [] and journal.read_bytes() == original_bytes


def test_replacement_failure_remains_closed_and_exact_resume_uses_successor(replacement_ready):
    installation, successor, actions, state, journal, original_bytes = replacement_ready
    state["fail_upgrade"] = True
    with pytest.raises(service.FixtureFailed, match="successor declaration"):
        service.upgrade_installer(installation, successor, replace=True)
    marker = dead_upgrade_owner(installation)
    assert marker["upgrade"]["checkpoints"]["authentication-drained"]
    assert marker["upgrade"]["predecessors"][0]["journal"] == str(journal)
    assert journal.read_bytes() == original_bytes
    from storage_service import dependencies
    assert str(journal) in {row["path"] for row in dependencies(installation)["dependencies"]}
    state["fail_upgrade"] = False
    actions.clear()
    result = service.upgrade_installer(installation, successor)
    assert actions[0] == "restart-upgrade"
    assert result["upgrade_operation"] == marker["upgrade"]["operation"]


def test_replacement_can_explicitly_supersede_an_unpublished_successor(replacement_ready):
    installation, successor, actions, state, journal, original_bytes = replacement_ready
    state["fail_upgrade"] = True
    with pytest.raises(service.FixtureFailed):
        service.upgrade_installer(installation, successor, replace=True)
    first = dead_upgrade_owner(installation)
    first_journal = Path(first["upgrade"]["journal"])
    first_bytes = first_journal.read_bytes()
    next_candidate = successor.with_name("lctx-v4-next")
    next_candidate.write_bytes(b"next-corrected-installer")
    next_candidate.chmod(0o700)
    state.update(target="d" * 64, fail_upgrade=False)
    result = service.upgrade_installer(installation, next_candidate, replace=True)
    current = json.loads(Path(installation.record["native_upgrade"]["journal"]).read_text())
    assert len(current["predecessors"]) == 2
    assert result["upgrade_operation"] not in {entry["operation"] for entry in current["predecessors"]}
    assert journal.read_bytes() == original_bytes and first_journal.read_bytes() == first_bytes


@pytest.mark.parametrize("lock", ["admission.lock", "borrowers.lock"])
def test_replacement_refuses_held_owner_locks_before_restart(replacement_ready, lock):
    installation, successor, actions, state, journal, original_bytes = replacement_ready
    held = service.hold_lock(installation.directory / lock)
    try:
        with pytest.raises(service.FixtureBlocked, match="admission|borrower lease"):
            service.upgrade_installer(installation, successor, replace=True)
    finally:
        os.close(held)
    assert "restart-upgrade" not in actions and journal.read_bytes() == original_bytes


def test_replacement_refuses_wrong_staged_hash_and_accepts_changed_binary_same_target(replacement_ready, monkeypatch):
    import storage_service
    installation, successor, actions, state, journal, original_bytes = replacement_ready
    prepare = storage_service.prepare
    monkeypatch.setattr(storage_service, "prepare", lambda *args: {**prepare(*args), "sha256": "e" * 64})
    with pytest.raises(service.FixtureBlocked, match="candidate generation"):
        service.upgrade_installer(installation, successor, replace=True)
    assert actions[0] == "restart-upgrade" and ("schema", "main") not in actions
    dead_upgrade_owner(installation)
    monkeypatch.setattr(storage_service, "prepare", prepare)
    state["target"] = json.loads(original_bytes)["target"]["schema"]
    result = service.upgrade_installer(installation, successor, replace=True)
    assert result["upgrade_operation"] != json.loads(original_bytes)["operation"]
    current = json.loads(Path(installation.record["native_upgrade"]["journal"]).read_text())
    assert current["target"] == json.loads(original_bytes)["target"]
    assert journal.read_bytes() == original_bytes
    assert len(list((installation.directory / "native-upgrades").glob("*/plan.json"))) == 2


def test_replacement_crash_before_marker_switch_preserves_original_and_orphan_is_inert(replacement_ready, monkeypatch):
    import storage_lifecycle
    installation, successor, actions, state, journal, original_bytes = replacement_ready
    old_marker = json.loads((installation.directory / "maintenance.json").read_text())
    durable = storage_lifecycle.durable_json
    def fail_marker(path, content):
        if Path(path).name == "maintenance.json" and content["upgrade"]["operation"] != old_marker["upgrade"]["operation"]:
            raise OSError("simulated durable marker switch failure")
        return durable(path, content)
    monkeypatch.setattr(storage_lifecycle, "durable_json", fail_marker)
    with pytest.raises(OSError, match="marker switch"):
        service.upgrade_installer(installation, successor, replace=True)
    marker = json.loads((installation.directory / "maintenance.json").read_text())
    assert marker["upgrade"]["operation"] == old_marker["upgrade"]["operation"]
    assert journal.read_bytes() == original_bytes
    assert len(list((installation.directory / "native-upgrades").glob("*/plan.json"))) == 2
    assert not any(isinstance(action, tuple) and action[0] in ("rotate", "upgrade") for action in actions)


def test_upgrade_cli_failure_retains_unique_private_diagnostics_without_public_secrets(tmp_path, monkeypatch, capsys):
    operation = "e" * 64
    native_operation = "f" * 64
    directory = tmp_path / "service"
    journal = directory / "native-upgrades" / operation
    journal.mkdir(parents=True, mode=0o700)
    cfg = journal / "main-new-installer.json"
    service.private_json(cfg, {"password": "private-password"})
    candidate = tmp_path / "candidate"
    candidate.write_bytes(b"candidate")
    candidate.chmod(0o700)
    generation = {"path": str(candidate), "sha256": service.file_sha256(candidate)}
    installation = SimpleNamespace(directory=directory)
    def failed(args, **kw):
        assert args[args.index("--operation") + 1] == native_operation
        assert str(cfg) in args
        return subprocess.CompletedProcess(args, 12, "private-token", "SQL PASSWORD private-password")
    monkeypatch.setattr(service.subprocess, "run", failed)
    for _ in range(2):
        with pytest.raises(service.FixtureFailed) as error:
            service._upgrade_cli(installation, generation, "main", "upgrade", config=cfg,
                                 operation=operation, native_operation=native_operation)
        assert "private diagnostic:" in str(error.value)
        assert "private-token" not in str(error.value) and "private-password" not in str(error.value)
    files = list(journal.glob("failure-*.json"))
    assert len(files) == 2
    for path in files:
        assert path.stat().st_mode & 0o077 == 0
        record = json.loads(path.read_text())
        assert record["operation"] == operation and record["native_operation"] == native_operation
        assert record["returncode"] == 12 and record["stdout"] == "private-token"
        assert record["stderr"] == "SQL PASSWORD private-password"
    assert capsys.readouterr() == ("", "")


def test_upgrade_restart_after_core_dump_confirms_stop_before_daemon_only_readiness(tmp_path, monkeypatch):
    actions = []
    binary = tmp_path / "surreal"
    installation = SimpleNamespace(directory=tmp_path / "service", endpoint="grpc://localhost:8000",
                                   record={"binary": {"path": str(binary), "sha256": service.BINARY_SHA256,
                                                      "version": service.VERSION}})
    unit = tmp_path / "owned.service"
    unit.write_text(service.unit_text(installation.directory, binary,
        int(installation.endpoint.rsplit(":", 1)[1])))
    monkeypatch.setattr(service, "verify_binary", lambda env: binary)
    monkeypatch.setattr(service, "_unit_path", lambda: unit)
    monkeypatch.setattr(service, "_stopped_attachment_commands", lambda inst: actions.append("attachments-drained"))
    alive = False
    def systemctl(command, name):
        nonlocal alive
        actions.append(command)
        alive = command == "start"
        return subprocess.CompletedProcess([], 0)
    def properties(*args):
        actions.append("confirmed-core-dump-stop")
        assert not alive
        return {"ActiveState": "failed", "MainPID": "0", "ControlGroup": ""}
    def daemon(inst):
        assert alive, "no schema/native observation before owned daemon restart"
        actions.append("daemon-ready")
    monkeypatch.setattr(service, "systemctl", systemctl)
    monkeypatch.setattr(service, "unit_properties", properties)
    installation.check_daemon = lambda: daemon(installation)
    installation.check = lambda **kw: pytest.fail("partial upgrade cannot use whole-schema readiness")
    actions.clear()
    service._upgrade_restart(installation)
    assert actions == ["attachments-drained", "stop", "confirmed-core-dump-stop", "start", "daemon-ready"]


@pytest.mark.parametrize("replacement", [False, True])
def test_upgrade_marker_references_only_durably_linked_candidate_and_journal(request, monkeypatch, replacement):
    import storage_lifecycle
    import storage_service
    if replacement:
        installation, candidate, *_ = request.getfixturevalue("replacement_ready")
    else:
        installation, candidate, _ = request.getfixturevalue("upgradable")
    trace = []
    fsync = storage_lifecycle.fsync_directory
    durable = storage_lifecycle.durable_json
    def flushed(path):
        fsync(path)
        trace.append(("fsync", Path(path)))
    def written(path, content):
        trace.append(("write", Path(path), content.get("upgrade", {}).get("operation")))
        return durable(path, content)
    monkeypatch.setattr(storage_lifecycle, "fsync_directory", flushed)
    monkeypatch.setattr(storage_service, "fsync_directory", flushed)
    monkeypatch.setattr(storage_lifecycle, "durable_json", written)
    result = service.upgrade_installer(installation, candidate, replace=replacement)
    operation = result["upgrade_operation"]
    directory = installation.directory / "native-upgrades" / operation
    publication = trace.index(("write", installation.directory / "maintenance.json", operation))
    plan_write = trace.index(("write", directory / "plan.json", None))
    for path in (directory, directory.parent, installation.directory):
        assert ("fsync", path) in trace[plan_write:publication], "persist journal ancestor before publishing marker"
    generation = Path(installation.record["installer"])
    chain = (generation.parent, generation.parent.parent, generation.parent.parent.parent,
             generation.parent.parent.parent.parent)
    position = -1
    for path in chain:
        position = trace.index(("fsync", path), position + 1)
        assert position < publication, "persist candidate ancestor chain before publishing marker"


@pytest.mark.parametrize("flag", ["--recover", "--restart", "--check", "--stabilize-installer", "--upgrade-installer"])
def test_replacement_cli_is_mutually_exclusive(flag):
    args = ["maintenance", "--replace-upgrade-installer", "/candidate", flag]
    if flag == "--upgrade-installer":
        args.append("/other")
    with pytest.raises(SystemExit):
        service.main(args)


def owned_server_descriptor(directory, scratch, suffix=""):
    import surrealdb_server as server
    from storage_service import prepare_server
    selected = server.recipe(cpu_count=7)
    provenance = server.build_provenance(directory, selected,
        "rustc 1.99.0-nightly\ncommit-date: 2026-09-28\nhost: x86_64-unknown-linux-gnu",
        "cargo 1.99.0-nightly", cache_record_sha256="a" * 64)
    executable = scratch / ("fake-reviewed-server" + suffix)
    executable.write_text("#!/bin/sh\n# " + suffix + "\nprintf '" + selected["pin"]["cli_version"] + "\\n'\n")
    executable.chmod(0o700)
    return prepare_server(directory, executable, provenance)


@pytest.fixture
def server_handoff_ready(published_replacement_ready, tmp_path, monkeypatch):
    installation, successor, actions, state, journal, original_bytes = published_replacement_ready
    generation = owned_server_descriptor(installation.directory, tmp_path)
    previous = Path(installation.record["binary"]["path"])
    service._unit_path().write_text(service.unit_text(installation.directory, previous, 29999))
    monkeypatch.setattr(service, "verify_binary", lambda *args, **kw: previous)
    monkeypatch.setattr(service, "_stopped_attachment_commands", lambda inst: actions.append("attachments-drained"))
    monkeypatch.setattr(service, "systemctl", lambda *args: actions.append(args[0]) or subprocess.CompletedProcess(args, 0))
    state["fail_start"] = False
    def start(inst):
        assert service.lock_held(inst.directory / "admission.lock")
        assert service.lock_held(inst.directory / "borrowers.lock")
        assert service._installed_binary(inst) == Path(generation["path"])
        assert service._unit_path().read_text() == service.unit_text(inst.directory, Path(generation["path"]), 29999)
        actions.append("owned-server-start")
        if state["fail_start"]:
            raise service.FixtureBlocked("launch", "injected owned server start failure")
    monkeypatch.setattr(service, "_upgrade_daemon_start", start)
    actions.clear()
    return installation, generation, actions, state, journal, original_bytes, successor


def test_server_handoff_preserves_closed_mixed_upgrade_and_both_server_generations(server_handoff_ready):
    installation, generation, actions, state, journal, original_bytes, _ = server_handoff_ready
    before = json.loads((installation.directory / "maintenance.json").read_text())["upgrade"]
    previous = installation.record["binary"].copy()
    private = {path: path.read_bytes() for path in installation.directory.glob("*-*.json")}
    environment = (installation.directory / "server.env").read_bytes()
    result = service.handoff_server(installation, Path(generation["descriptor_path"]))
    assert result["native_upgrade_pending"] and not result["admission_open"]
    marker = json.loads((installation.directory / "maintenance.json").read_text())
    assert marker["operation"] == service.UPGRADE_OPERATION and marker["upgrade"] == before
    assert "server_handoff" not in marker
    assert installation.record.get("native_schema_version", 3) == 3
    assert installation.record["binary"] == service._generation_binary(generation)
    assert journal.read_bytes() == original_bytes
    assert (installation.directory / "server.env").read_bytes() == environment
    assert all(path.read_bytes() == content for path, content in private.items())
    assert actions.index("stop") < actions.index("daemon-reload") < actions.index("owned-server-start")
    assert actions.index("owned-server-start") < actions.index(("inspect-marker", "main"))
    assert "open-admission" not in actions and "close-admission" not in actions and "drain" not in actions
    assert state["native_calls"] == []
    handoff = installation.record["server_handoff"]
    plan = json.loads(Path(handoff["journal"]).read_text())
    assert plan["previous_binary"] == previous and plan["generation"] == generation
    assert plan["upgrade"] == before
    assert (Path(handoff["journal"]).parent / "checked.json").is_file()
    from storage_service import dependencies
    protected = {row["path"] for row in dependencies(installation)["dependencies"]}
    assert {previous["path"], generation["path"], generation["descriptor_path"],
            generation["provenance"]["source_path"], generation["provenance"]["build_path"],
            generation["provenance"]["cache_lineage"]["path"], generation["provenance"]["cache_lineage"]["record"],
            generation["provenance"]["dependency_overrides"][0]["path"],
            generation["provenance"]["dependency_overrides"][0]["archive_path"],
            str(Path(handoff["journal"]).parent), str(journal)} <= protected


def test_server_handoff_failure_restarts_exact_generation_before_resume_and_preserves_native_identity(server_handoff_ready):
    installation, generation, actions, state, journal, original_bytes, successor = server_handoff_ready
    state["fail_start"] = True
    with pytest.raises(service.FixtureBlocked, match="injected owned server start"):
        service.handoff_server(installation, Path(generation["descriptor_path"]))
    marker = dead_upgrade_owner(installation)
    assert marker["server_handoff"]["phase"] == "installed"
    assert marker["upgrade"]["checkpoints"]["scope-main"]
    with pytest.raises(service.FixtureBlocked, match="--server-generation"):
        service.upgrade_installer(installation, successor)
    state["fail_start"] = False
    actions.clear()
    service.handoff_server(installation, Path(generation["descriptor_path"]))
    assert actions.index("stop") < actions.index("owned-server-start") < actions.index(("inspect-marker", "main"))
    assert state["native_calls"] == [] and journal.read_bytes() == original_bytes
    assert json.loads((installation.directory / "maintenance.json").read_text())["upgrade"]["operation"] == json.loads(original_bytes)["operation"]


def test_server_handoff_crash_between_unit_and_installation_is_resumable(server_handoff_ready, monkeypatch):
    import storage_lifecycle
    installation, generation, actions, state, journal, original_bytes, _ = server_handoff_ready
    previous = installation.record["binary"].copy()
    durable = storage_lifecycle.durable_json
    def fail(path, content):
        if Path(path).name == "installation.json" and content.get("binary", {}).get("generation"):
            raise OSError("injected unit-before-installation crash")
        return durable(path, content)
    monkeypatch.setattr(storage_lifecycle, "durable_json", fail)
    with pytest.raises(OSError, match="unit-before-installation"):
        service.handoff_server(installation, Path(generation["descriptor_path"]))
    assert json.loads((installation.directory / "installation.json").read_text())["binary"] == previous
    assert generation["path"] in service._unit_path().read_text()
    dead_upgrade_owner(installation)
    installation = service.Installation(installation.directory,
        json.loads((installation.directory / "installation.json").read_text()))
    monkeypatch.setattr(storage_lifecycle, "durable_json", durable)
    actions.clear()
    service.handoff_server(installation, Path(generation["descriptor_path"]))
    assert actions.index("stop") < actions.index("owned-server-start")
    assert journal.read_bytes() == original_bytes


@pytest.mark.parametrize("field,value", [("generation", "d" * 64), ("admission_open", True), ("schema", "d" * 64)])
def test_server_handoff_refuses_changed_scope_after_restart_and_preserves_journal(server_handoff_ready, field, value):
    installation, generation, actions, state, journal, original_bytes, _ = server_handoff_ready
    state["markers"]["validation"][field] = value
    with pytest.raises(service.FixtureBlocked, match="scope identity or closed admission"):
        service.handoff_server(installation, Path(generation["descriptor_path"]))
    marker = json.loads((installation.directory / "maintenance.json").read_text())
    assert marker["server_handoff"]["phase"] == "installed"
    assert "owned-server-start" in actions and "open-admission" not in actions
    assert journal.read_bytes() == original_bytes and state["native_calls"] == []


@pytest.mark.parametrize("lock", ["admission.lock", "borrowers.lock"])
def test_server_handoff_refuses_live_owner_and_held_locks_before_stop(server_handoff_ready, lock):
    installation, generation, actions, state, journal, original_bytes, _ = server_handoff_ready
    held = service.hold_lock(installation.directory / lock)
    try:
        with pytest.raises(service.FixtureBlocked, match="admission|borrower lease"):
            service.handoff_server(installation, Path(generation["descriptor_path"]))
    finally:
        os.close(held)
    assert "stop" not in actions and journal.read_bytes() == original_bytes
    marker = json.loads((installation.directory / "maintenance.json").read_text())
    marker["owner"] = service.ProcessIdentity.of().to_json()
    service.private_json(installation.directory / "maintenance.json", marker)
    with pytest.raises(service.FixtureBlocked, match="dead exact maintenance owner"):
        service.handoff_server(installation, Path(generation["descriptor_path"]))
    assert "stop" not in actions


def test_server_handoff_exact_resume_refuses_other_generation_and_generic_recovery(server_handoff_ready, tmp_path, monkeypatch, capfd):
    installation, generation, actions, state, journal, original_bytes, _ = server_handoff_ready
    state["fail_start"] = True
    with pytest.raises(service.FixtureBlocked):
        service.handoff_server(installation, Path(generation["descriptor_path"]))
    dead_upgrade_owner(installation)
    other = owned_server_descriptor(installation.directory, tmp_path, "-other")
    actions.clear()
    with pytest.raises(service.FixtureBlocked, match="exact checkpointed server generation"):
        service.handoff_server(installation, Path(other["descriptor_path"]))
    assert "stop" not in actions
    monkeypatch.setenv("LCTX_SURREAL_SERVICE_CONFIG", str(installation.directory / "installation.json"))
    assert service.main(["maintenance", "--recover"]) == 75
    assert "--server-generation" in capfd.readouterr().err
    assert journal.read_bytes() == original_bytes


@pytest.mark.parametrize("flag", ["--recover", "--restart", "--check", "--upgrade-installer", "--replace-upgrade-installer"])
def test_server_handoff_cli_is_mutually_exclusive(flag):
    args = ["maintenance", "--server-generation", "/descriptor", flag]
    if flag in ("--upgrade-installer", "--replace-upgrade-installer"):
        args.append("/candidate")
    with pytest.raises(SystemExit):
        service.main(args)


def test_server_handoff_durably_links_plan_and_unit_before_pointer_start(server_handoff_ready, monkeypatch):
    import storage_lifecycle
    installation, generation, actions, state, journal, original_bytes, _ = server_handoff_ready
    trace = []
    durable = storage_lifecycle.durable_json
    fsync = storage_lifecycle.fsync_directory
    private = service._upgrade_write_private
    start = service._upgrade_daemon_start
    def written(path, row):
        result = durable(path, row)
        trace.append(("durable", Path(path)))
        return result
    def flushed(path):
        fsync(path)
        trace.append(("fsync", Path(path)))
    def unit(path, content):
        private(path, content)
        trace.append(("unit", Path(path)))
    monkeypatch.setattr(storage_lifecycle, "durable_json", written)
    monkeypatch.setattr(storage_lifecycle, "fsync_directory", flushed)
    monkeypatch.setattr(service, "_upgrade_write_private", unit)
    monkeypatch.setattr(service, "_upgrade_daemon_start", lambda inst: trace.append(("start", None)) or start(inst))
    service.handoff_server(installation, Path(generation["descriptor_path"]))
    plan = Path(installation.record["server_handoff"]["journal"])
    marker = trace.index(("durable", installation.directory / "maintenance.json"))
    plan_position = trace.index(("durable", plan))
    for path in (plan.parent, plan.parent.parent, installation.directory):
        assert ("fsync", path) in trace[plan_position:marker]
    assert trace.index(("unit", service._unit_path())) < trace.index(("durable", installation.directory / "installation.json")) < trace.index(("start", None))


def test_owned_patched_binary_is_descriptor_bound_without_weakening_official_pin(server_handoff_ready):
    installation, generation, actions, state, journal, original_bytes, _ = server_handoff_ready
    service.handoff_server(installation, Path(generation["descriptor_path"]))
    assert service._descriptor(installation.directory)["binary"] == service._generation_binary(generation)
    assert service._installed_binary(installation) == Path(generation["path"])
    with pytest.raises(service.FixtureBlocked, match="sha256"):
        verify_official_binary({"LCTX_SURREAL_BIN": generation["path"]}, runner=pytest.fail)
    installation.record["binary"]["sha256"] = "d" * 64
    with pytest.raises(service.FixtureBlocked, match="owned generation"):
        service._installed_binary(installation)


def test_check_daemon_checks_patched_actual_http_version_after_exact_executable(server_handoff_ready, monkeypatch):
    installation, generation, actions, state, journal, original_bytes, _ = server_handoff_ready
    installation.record["binary"] = service._generation_binary(generation)
    binary = Path(generation["path"])
    monkeypatch.setattr(service, "_installed_binary", lambda inst: binary)
    monkeypatch.setattr(service, "unit_properties", lambda *args: {"ActiveState": "active", "MainPID": "1234"})
    resolve = Path.resolve
    read_bytes = Path.read_bytes
    def resolved(path, *args, **kw):
        return binary if str(path) == "/proc/1234/exe" else resolve(path, *args, **kw)
    def bytes_for(path):
        if str(path) == "/proc/1234/cmdline":
            return ("surreal\0rocksdb://" + str(installation.directory / "data/store") + "\0" + installation.endpoint.removeprefix("http://") + "\0").encode()
        if str(path) == "/proc/1234/environ":
            return b"\0".join((key + "=" + value).encode() for key, value in service.SERVER_SETTINGS.items())
        return read_bytes(path)
    monkeypatch.setattr(Path, "resolve", resolved)
    monkeypatch.setattr(Path, "read_bytes", bytes_for)
    reported = service.VERSION
    monkeypatch.setattr(service.urllib.request, "urlopen", lambda *args, **kw:
        contextlib.nullcontext(SimpleNamespace(status=200, read=lambda: reported.encode())))
    with pytest.raises(service.FixtureBlocked, match="server version differs"):
        installation.check_daemon()
    reported = generation["http_version"]
    installation.check_daemon()


def test_standalone_server_handoff_reopens_only_after_both_unchanged_scopes(recoverable, tmp_path, monkeypatch):
    installation, _, actions = recoverable
    generation = owned_server_descriptor(installation.directory, tmp_path)
    previous = Path(installation.record["binary"]["path"])
    service._unit_path().write_text(service.unit_text(installation.directory, previous, 29999))
    monkeypatch.setattr(service, "verify_binary", lambda *a, **kw: previous)
    monkeypatch.setattr(service.Installation, "check_daemon", lambda inst: actions.append("daemon-ready"))
    monkeypatch.setattr(service, "_upgrade_daemon_start", lambda inst: actions.append("owned-server-start"))
    monkeypatch.setattr(service, "_stopped_attachment_commands", lambda inst: actions.append("attachments-drained"))
    monkeypatch.setattr(service, "systemctl", lambda *args: actions.append(args[0]) or subprocess.CompletedProcess(args, 0))
    admitted = True
    def installer(inst, exe, action):
        nonlocal admitted
        actions.append(action)
        if action in ("close-admission", "open-admission"):
            admitted = action == "open-admission"
    def scopes(inst, statement, *, cfg):
        assert statement.startswith("SELECT generation,schema,schema_version,admission_open")
        actions.append(("scope", cfg["database"], admitted))
        return [{"status": "OK", "result": [{"generation": bytes(inst.record["service_generation"]).hex(),
            "schema": "a" * 64, "schema_version": 3, "admission_open": admitted}]}]
    monkeypatch.setattr(service, "_run_installer", installer)
    monkeypatch.setattr(service, "sql", scopes)
    result = service.handoff_server(installation, Path(generation["descriptor_path"]))
    assert result["admission_open"] and not result["native_upgrade_pending"]
    assert actions.index("close-admission") < actions.index("stop") < actions.index("owned-server-start")
    assert actions.index(("scope", "main", False)) < actions.index("open-admission")
    assert actions.index(("scope", "validation", False)) < actions.index("open-admission")
    assert not (installation.directory / "maintenance.json").exists()


def test_patched_server_cold_archive_restores_without_source_or_build_checkout(recoverable, tmp_path):
    installation, archive, actions = recoverable
    generation = owned_server_descriptor(installation.directory, tmp_path)
    installation.record["binary"] = service._generation_binary(generation)
    service.private_json(installation.directory / "installation.json", installation.record)
    service._unit_path().write_text(service.unit_text(installation.directory, Path(generation["path"]), 29999))
    assert not Path(generation["provenance"]["source_path"]).exists()
    assert not Path(generation["provenance"]["build_path"]).exists()
    assert not Path(generation["provenance"]["cache_lineage"]["path"]).exists()
    assert not Path(generation["provenance"]["cache_lineage"]["record"]).exists()
    assert not Path(generation["provenance"]["dependency_overrides"][0]["path"]).exists()
    assert not Path(generation["provenance"]["dependency_overrides"][0]["archive_path"]).exists()
    service.backup_service(installation, archive)
    with tarfile.open(archive, "r:") as contents:
        descriptor = json.load(contents.extractfile("assets/server-generation.json"))
        assert descriptor == generation
        assert contents.extractfile("assets/surreal").read() == Path(generation["path"]).read_bytes()
        assert not any("server-build" in name for name in contents.getnames())
    assert service.restore_service(installation, archive)["outcome"] == "passed"
    (installation.directory / "data/store/MANIFEST").write_text("changed-cold-data")
    result = service.restore_service(installation, archive, apply=True)
    assert result["applied"]
    assert (installation.directory / "data/store/MANIFEST").read_text() == "cold-owned-data"
    assert installation.record["binary"] == service._generation_binary(generation)
    assert service._installed_binary(installation) == Path(generation["path"])


def test_patched_server_archive_refuses_changed_provenance_before_stop(recoverable, tmp_path):
    installation, archive, actions = recoverable
    generation = owned_server_descriptor(installation.directory, tmp_path)
    installation.record["binary"] = service._generation_binary(generation)
    service.private_json(installation.directory / "installation.json", installation.record)
    service.backup_service(installation, archive)
    with tarfile.open(archive, "r:") as contents:
        payload = {item.name: contents.extractfile(item).read() for item in contents.getmembers()}
    modified = {**generation, "provenance": {**generation["provenance"], "patch_sha256": "d" * 64}}
    payload["assets/server-generation.json"] = json.dumps(modified).encode()
    metadata = json.loads(payload["recovery.json"])
    metadata["files"]["assets/server-generation.json"].update(
        sha256=service.hashlib.sha256(payload["assets/server-generation.json"]).hexdigest(),
        size=len(payload["assets/server-generation.json"]))
    payload["recovery.json"] = json.dumps(metadata).encode()
    with tarfile.open(archive, "w:") as contents:
        for name, data in payload.items():
            item = tarfile.TarInfo(name)
            item.size, item.mode = len(data), 0o600
            contents.addfile(item, io.BytesIO(data))
    actions.clear()
    with pytest.raises(service.FixtureBlocked, match="server generation provenance"):
        service.restore_service(installation, archive)
    assert "stop" not in actions


def test_patched_server_status_names_owned_generation_and_retained_dependencies(server_handoff_ready, monkeypatch, capfd):
    installation, generation, actions, state, journal, original_bytes, _ = server_handoff_ready
    service.handoff_server(installation, Path(generation["descriptor_path"]))
    monkeypatch.setenv("LCTX_SURREAL_SERVICE_CONFIG", str(installation.directory / "installation.json"))
    monkeypatch.setattr(service, "unit_properties", lambda *args: {"ActiveState": "active"})
    assert service.main(["status"]) == 0
    result = json.loads(capfd.readouterr().out)
    assert result["server"]["generation"] == generation["identity"]
    assert result["server"]["version"] == generation["http_version"]
    assert generation["provenance"]["build_path"] in {row["path"] for row in result["storage"]["dependencies"]}
