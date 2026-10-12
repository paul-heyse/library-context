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

DAEMON_CHECK = service.Installation.check_daemon
INSTALLATION_CHECK = service.Installation.check
OWNED_START = service._start_owned


def test_actual_service_cli_keeps_helper_owner_and_startup_retry_identity():
    code = """
import argparse, runpy, sys
from pathlib import Path
from types import SimpleNamespace
sys.path.insert(0,str(Path(sys.argv[1]).parent))
def observe_cli(self, *args, **kwargs):
    import surrealdb_service as owner
    import surrealdb_pd02_recovery as recovery
    assert owner is sys.modules['__main__']
    assert recovery.service is owner
    assert recovery.service.FixtureBlocked is owner.FixtureBlocked
    assert recovery.service.Installation is owner.Installation
    assert recovery.service._checked_generations is owner._checked_generations
    calls = []
    def check(**kwargs):
        calls.append('checked same generation')
        assert kwargs == {'allow_maintenance': True}
        if len(calls) == 1:
            raise owner.FixtureBlocked('readiness', 'successor still starting')
    owner.systemctl = lambda *args: SimpleNamespace(returncode=0)
    owner.time.sleep = lambda delay: None
    recovery.service._start_owned(SimpleNamespace(unit='exact-owned.service', check=check))
    assert calls == ['checked same generation', 'checked same generation']
    print('canonical CLI/helper owner; transient readiness retried')
    raise SystemExit(0)
argparse.ArgumentParser.parse_args = observe_cli
runpy.run_path(sys.argv[1], run_name='__main__')
"""
    result = subprocess.run([sys.executable,"-c",code,str(Path(service.__file__).resolve())],
        cwd=service.ROOT,capture_output=True,text=True,check=False)
    assert result.returncode == 0, result.stderr
    assert "canonical CLI/helper owner; transient readiness retried" in result.stdout


def test_owned_start_rechecks_same_generation_without_restarting_or_extending_deadline(recoverable,monkeypatch):
    installation,_archive,_actions=recoverable
    original=json.loads(json.dumps(installation.record))
    calls=[]
    checks=0
    def check(current,**kwargs):
        nonlocal checks
        checks+=1
        assert current.record == original and kwargs == {"allow_maintenance":True}
        if checks < 3:
            raise service.FixtureBlocked("readiness","same successor not ready yet")
        return {"outcome":"passed"}
    monkeypatch.setattr(service.Installation,"check",check)
    monkeypatch.setattr(service,"systemctl",lambda *args:calls.append(args) or SimpleNamespace(returncode=0))
    sleeps=[]
    monkeypatch.setattr(service.time,"sleep",lambda delay:sleeps.append(delay))
    OWNED_START(installation)
    assert calls == [("start",installation.unit)] and sleeps == [0.1,0.1]
    assert checks == 3 and service.READY_TIMEOUT == 40.0
    refusal=service.FixtureBlocked("identity","successor executable differs")
    monkeypatch.setattr(service.Installation,"check",lambda *a,**kw:(_ for _ in ()).throw(refusal))
    times=iter([100.0,139.0,140.0])
    monkeypatch.setattr(service.time,"monotonic",lambda:next(times))
    with pytest.raises(service.FixtureBlocked) as caught:
        OWNED_START(installation)
    assert caught.value is refusal and sleeps == [0.1,0.1,0.1]
    assert calls == [("start",installation.unit),("start",installation.unit)]


def assert_borrow_protected(path):
    with path.open("rb") as lock:
        with pytest.raises(BlockingIOError):
            fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)


def test_borrow_protects_before_proof_and_reuses_one_checked_generation(recoverable, monkeypatch):
    installation, _archive, _actions = recoverable
    proofs, queries = [], []
    monkeypatch.setattr(service.Installation, "check", INSTALLATION_CHECK)
    def daemon(current):
        assert_borrow_protected(current.directory / "admission.lock")
        assert_borrow_protected(current.directory / "borrowers.lock")
        proofs.append(current.id)
    monkeypatch.setattr(service.Installation, "check_daemon", daemon)
    def sql(current, statement, *, cfg):
        queries.append(cfg["database"])
        return [{"result": [{"generation": bytes(current.record["service_generation"]).hex(),
                             "schema_version": 3}]}]
    monkeypatch.setattr(service, "sql", sql)
    with service.borrow(installation) as checked:
        first = checked.readiness
        for _ in range(3):
            loaded = service.Installation.load({"LCTX_SURREAL_SERVICE_CONFIG":
                str(installation.directory / "installation.json")})
            assert loaded.check() == first
            assert checked.server_identity(loaded)["binary_sha256"] == service.BINARY_SHA256
        assert proofs == [installation.id] and queries == list(service.DATABASES)
    assert service.checked_generation(installation) is None
    with pytest.raises(service.FixtureBlocked, match="no longer protected"):
        checked.validate(installation)


@pytest.mark.parametrize("change", ["descriptor", "runtime", "admission"])
def test_checked_generation_refuses_changed_premises(recoverable, monkeypatch, change):
    installation, _archive, _actions = recoverable
    with service.borrow(installation) as checked:
        if change == "descriptor":
            row = {**installation.record, "native_schema_version": 99}
            service.private_json(installation.directory / "installation.json", row)
        elif change == "runtime":
            path = installation.runtime_path()
            path.write_bytes(path.read_bytes() + b" ")
        else:
            service.private_json(installation.directory / "maintenance.json", {"token": "new"})
        with pytest.raises(service.FixtureBlocked, match="no longer protected"):
            checked.validate(installation)


def test_failed_borrow_releases_locks_and_preserves_readiness_repair(recoverable, monkeypatch):
    installation, _archive, _actions = recoverable
    def refuse(self, **kw):
        assert_borrow_protected(self.directory / "borrowers.lock")
        raise service.FixtureBlocked("schema", "exact schema mismatch", "just service maintenance --check")
    monkeypatch.setattr(service.Installation, "check", refuse)
    with pytest.raises(service.FixtureBlocked) as error, service.borrow(installation):
        pytest.fail("failed check cannot yield a witness")
    assert error.value.repair == "just service maintenance --check"
    assert not service.lock_held(installation.directory / "borrowers.lock")
    assert not service.lock_held(installation.directory / "admission.lock")


@pytest.mark.parametrize("change", ["daemon", "executable", "owner"])
def test_checked_generation_rejects_process_executable_or_process_handoff(recoverable, monkeypatch, change):
    installation, _archive, _actions = recoverable
    binary = Path(installation.record["binary"]["path"])
    actual_stat = Path.stat
    monkeypatch.setattr(Path, "stat", lambda path, *a, **kw:
        actual_stat(binary, *a, **kw) if str(path) == "/proc/1234/exe"
        else actual_stat(path, *a, **kw))
    alive = True
    installation._daemon_identity = (SimpleNamespace(pid=1234, alive=lambda: alive),
                                    service.executable_identity(binary.stat()))
    with service.borrow(installation) as checked:
        checked.validate(installation)
        if change == "daemon":
            alive = False
        elif change == "executable":
            replacement = binary.with_name("replacement")
            replacement.write_bytes(binary.read_bytes())
            os.replace(replacement, binary)
        else:
            checked.owner_pid += 1
        with pytest.raises(service.FixtureBlocked, match="no longer protected"):
            checked.validate(installation)


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
    monkeypatch.setattr(service, "_unit_path", lambda unit_name=service.UNIT: unit)
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
        service,
        "local_urlopen",
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
    service._stop_owned(SimpleNamespace(unit=service.UNIT))
    state["MainPID"] = "123"
    with pytest.raises(service.FixtureBlocked, match="stop is not confirmed"):
        service._stop_owned(SimpleNamespace(unit=service.UNIT))


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
def server_handoff_ready(recoverable, tmp_path, monkeypatch):
    installation, _, actions = recoverable
    generation = owned_server_descriptor(installation.directory, tmp_path)
    previous = Path(installation.record["binary"]["path"])
    service._unit_path().write_text(service.unit_text(installation.directory, previous, 29999))
    monkeypatch.setattr(service, "verify_binary", lambda *args, **kw: previous)
    monkeypatch.setattr(service.Installation, "check_daemon", lambda inst: None)
    state={"drift":None,"markers":{db:{"generation":bytes(installation.record["service_generation"]).hex(),
        "schema":"a"*64,"schema_version":3,"admission_open":False} for db in service.DATABASES}}
    def start(inst):
        actions.append("owned-server-start")
        if state["drift"]:
            field,value=state["drift"];state["markers"]["validation"][field]=value
    monkeypatch.setattr(service, "_owned_daemon_start", start)
    monkeypatch.setattr(service, "_stopped_attachment_commands", lambda inst: actions.append("attachments-drained"))
    monkeypatch.setattr(service, "systemctl", lambda *args: actions.append(args[0]) or subprocess.CompletedProcess(args, 0))
    monkeypatch.setattr(service, "sql", lambda inst, statement, *, cfg:
        [{"status":"OK","result":[dict(state["markers"][cfg["database"]])]}])
    return installation, generation, actions, state, None, b"", None








@pytest.mark.parametrize("field,value", [("generation", "d" * 64), ("admission_open", True), ("schema", "d" * 64)])
def test_server_handoff_refuses_changed_scope_after_restart_and_preserves_journal(server_handoff_ready, field, value):
    installation, generation, actions, state, journal, original_bytes, _ = server_handoff_ready
    state["drift"] = (field,value)
    with pytest.raises(service.FixtureBlocked, match="scope identity or closed admission"):
        service.handoff_server(installation, Path(generation["descriptor_path"]))
    marker = json.loads((installation.directory / "maintenance.json").read_text())
    assert marker["server_handoff"]["phase"] == "installed"
    assert "owned-server-start" in actions and "open-admission" not in actions
    plan=json.loads(Path(marker["server_handoff"]["journal"]).read_text())
    assert plan["scopes"]["validation"][field] != value


@pytest.mark.parametrize("lock", ["admission.lock", "borrowers.lock"])
def test_server_handoff_refuses_live_owner_and_held_locks_before_stop(server_handoff_ready, lock):
    installation, generation, actions, state, journal, original_bytes, _ = server_handoff_ready
    held = service.hold_lock(installation.directory / lock)
    try:
        with pytest.raises(service.FixtureBlocked, match="admission|borrower lease"):
            service.handoff_server(installation, Path(generation["descriptor_path"]))
    finally:
        os.close(held)
    assert "stop" not in actions
    assert not (installation.directory/"maintenance.json").exists()
    marker = {"operation":service.SERVER_HANDOFF_OPERATION,"owner":service.ProcessIdentity.of().to_json()}
    service.private_json(installation.directory / "maintenance.json", marker)
    with pytest.raises(service.FixtureBlocked, match="dead exact maintenance owner"):
        service.handoff_server(installation, Path(generation["descriptor_path"]))
    assert "stop" not in actions




@pytest.mark.parametrize("flag", ["--recover", "--restart", "--check"])
def test_server_handoff_cli_is_mutually_exclusive(flag):
    args = ["maintenance", "--server-generation", "/descriptor", flag]
    with pytest.raises(SystemExit):
        service.main(args)


def test_server_handoff_durably_links_plan_and_unit_before_pointer_start(server_handoff_ready, monkeypatch):
    import storage_lifecycle
    installation, generation, actions, state, journal, original_bytes, _ = server_handoff_ready
    trace = []
    durable = storage_lifecycle.durable_json
    fsync = storage_lifecycle.fsync_directory
    private = service._write_private
    start = service._owned_daemon_start
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
    monkeypatch.setattr(service, "_write_private", unit)
    monkeypatch.setattr(service, "_owned_daemon_start", lambda inst: trace.append(("start", None)) or start(inst))
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
    monkeypatch.setattr(service.Installation,"check_daemon",DAEMON_CHECK)
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
            return ("surreal\0--log=warn\0rocksdb://" + str(installation.directory / "data/store") + "\0" + installation.endpoint.removeprefix("http://") + "\0").encode()
        if str(path) == "/proc/1234/environ":
            return b"\0".join((key + "=" + value).encode() for key, value in service.SERVER_SETTINGS.items())
        return read_bytes(path)
    monkeypatch.setattr(Path, "resolve", resolved)
    monkeypatch.setattr(Path, "read_bytes", bytes_for)
    actual_stat = Path.stat
    monkeypatch.setattr(Path, "stat", lambda path, *a, **kw:
                        actual_stat(binary, *a, **kw) if str(path) == "/proc/1234/exe"
                        else actual_stat(path, *a, **kw))
    monkeypatch.setattr(service.ProcessIdentity, "of", lambda pid=None: SimpleNamespace(pid=1234, alive=lambda: True))
    reported = service.VERSION
    monkeypatch.setattr(service, "local_urlopen", lambda *args, **kw:
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
    monkeypatch.setattr(service, "_owned_daemon_start", lambda inst: actions.append("owned-server-start"))
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
    assert result["admission_open"]
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




















































def test_failed_install_metadata_can_retry_corrected_candidate_before_owned_directories(tmp_path, monkeypatch):
    import storage_service

    root = tmp_path / "installation"
    candidate = tmp_path / "lctx"
    candidate.write_bytes(b"candidate")
    candidate.chmod(0o700)
    monkeypatch.setattr(service, "verify_binary", lambda env: candidate)
    monkeypatch.setattr(service, "systemd_available", lambda **kw: None)
    monkeypatch.setattr(service, "unit_properties", lambda *a: {"LoadState": "not-found"})
    monkeypatch.setattr(service, "_unit_path", lambda unit_name=service.UNIT: tmp_path / "service-unit")
    monkeypatch.setattr(service, "_available_port", lambda: 29000)
    monkeypatch.setattr(storage_service, "prepare", lambda directory, source: {
        "path": str(source), "sha256": service.file_sha256(source)})
    valid = False

    def metadata(*args, **kwargs):
        if not valid:
            raise service.FixtureBlocked("schema", "unsupported candidate metadata")
        return {"schema_version": 6, "schema": "b" * 64}

    monkeypatch.setattr(service, "_installer_cli", metadata)
    monkeypatch.setattr(service, "systemctl", lambda *a: subprocess.CompletedProcess(a, 1, "", "test stop"))
    env = {"LCTX_SURREAL_SERVICE_CONFIG": str(root / "installation.json")}
    with pytest.raises(service.FixtureBlocked, match="unsupported candidate metadata"):
        service.install(candidate, env=env)
    assert not (root / "installation.json").exists()
    assert all(not (root / name).exists() for name in ("data", "tmp", "attachments", "serving"))
    valid = True
    with pytest.raises(service.FixtureBlocked, match="daemon-reload failed"):
        service.install(candidate, env=env)
    assert json.loads((root / "installation.json").read_text())["native_schema_version"] == 6
    assert all((root / name).is_dir() for name in ("data", "tmp", "attachments", "serving"))


def test_owned_http_bypasses_proxies_and_refuses_redirected_credentials(monkeypatch):
    import http.server
    import threading
    import urllib.error
    import urllib.request

    target_calls = []
    proxy_calls = []

    class Target(http.server.BaseHTTPRequestHandler):
        def do_POST(self):
            self.rfile.read(int(self.headers.get("Content-Length", "0")))
            target_calls.append((self.path, self.headers.get("Authorization")))
            if self.path == "/redirect":
                self.send_response(302)
                self.send_header("Location", proxy_url + "/sink")
                body = b""
            else:
                self.send_response(200)
                body = b'[{"status":"OK","result":1}]'
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)

        def log_message(self, *args):
            pass

    class Proxy(http.server.BaseHTTPRequestHandler):
        def do_POST(self):
            proxy_calls.append(self.path)
            self.send_response(502)
            self.send_header("Content-Length", "0")
            self.end_headers()

        do_GET = do_POST

        def log_message(self, *args):
            pass

    servers = [http.server.ThreadingHTTPServer(("127.0.0.1", 0), handler)
               for handler in (Target, Proxy)]
    target_url, proxy_url = [f"http://127.0.0.1:{server.server_port}" for server in servers]
    threads = [threading.Thread(target=server.serve_forever, kwargs={"poll_interval": 0.01})
               for server in servers]
    for thread in threads:
        thread.start()
    try:
        for name in list(os.environ):
            if name.lower().endswith("_proxy"):
                monkeypatch.delenv(name)
        monkeypatch.setenv("http_proxy", proxy_url)
        monkeypatch.setenv("NO_PROXY", "")
        cfg = {"username": "synthetic-user", "password": "synthetic-password",
               "database": "validation", "authentication": "database"}
        rows = service.sql(SimpleNamespace(endpoint=target_url), "RETURN 1;", cfg=cfg)
        assert rows == [{"status": "OK", "result": 1}]
        request = urllib.request.Request(target_url + "/redirect", data=b"synthetic admin SQL",
                                         headers={"Authorization": "Basic synthetic-only"})
        with pytest.raises(urllib.error.HTTPError) as refused:
            service.local_urlopen(request, timeout=2)
        assert refused.value.code == 302
        refused.value.close()
        assert [path for path, _auth in target_calls] == ["/sql", "/redirect"]
        assert target_calls[0][1].startswith("Basic ")
        assert proxy_calls == [], "neither proxy routing nor redirect may forward local requests"
    finally:
        for server in servers:
            server.shutdown()
            server.server_close()
        for thread in threads:
            thread.join()


@pytest.mark.parametrize("url", ["https://127.0.0.1:8000/sql", "http://example.com:8000/sql",
                                  "http://user:secret@127.0.0.1:8000/sql", "http://127.0.0.1/sql"])
def test_owned_http_refuses_foreign_or_credential_bearing_endpoints(url, monkeypatch):
    def unexpected(*args):
        pytest.fail("invalid owned endpoint must fail before opening a transport")
    monkeypatch.setattr(service.urllib.request, "build_opener", unexpected)
    with pytest.raises(ValueError, match="local HTTP endpoint"):
        service.local_urlopen(url, timeout=2)


def test_campaign_requires_explicit_nonstable_state_before_any_effect(tmp_path, monkeypatch):
    monkeypatch.setattr(service, "verify_binary", lambda *a, **kw: pytest.fail("binary access"))
    with pytest.raises(service.FixtureBlocked, match="explicit separate"):
        service.install(tmp_path / "lctx", env={"HOME": str(tmp_path)}, campaign=True)
    stable = service.state_root({"HOME": str(tmp_path)})
    with pytest.raises(service.FixtureBlocked, match="explicit separate"):
        service.install(tmp_path / "lctx", env={"HOME": str(tmp_path),
            "LCTX_SURREAL_SERVICE_CONFIG": str(stable / "installation.json")}, campaign=True)
    assert not stable.exists()


def test_campaign_descriptor_unit_is_derived_and_cannot_alias_stable(recoverable):
    installation, _archive, _actions = recoverable
    record = {**installation.record, "kind": "campaign"}
    record["unit"] = service.installation_unit(record)
    assert record["unit"] == f"library-context-surrealdb-campaign-{installation.id}.service"
    service.private_json(installation.directory / "installation.json", record)
    loaded = service.Installation.load({"LCTX_SURREAL_SERVICE_CONFIG":
        str(installation.directory / "installation.json")})
    assert loaded.unit != service.UNIT
    for invalid in (service.UNIT, "other.service", "../escape.service"):
        service.private_json(installation.directory / "installation.json", {**record, "unit": invalid})
        with pytest.raises(service.FixtureBlocked, match="incompatible"):
            service.Installation.load({"LCTX_SURREAL_SERVICE_CONFIG":
                str(installation.directory / "installation.json")})


def test_campaign_start_and_recovery_assets_use_only_its_unit(recoverable, monkeypatch):
    installation, _archive, _actions = recoverable
    installation.record.update(kind="campaign")
    installation.record["unit"] = service.installation_unit(installation.record)
    calls = []
    monkeypatch.setattr(service, "systemctl", lambda *args, **kw:
        calls.append(args) or subprocess.CompletedProcess(args, 0))
    monkeypatch.setattr(service, "unit_properties", lambda unit, *names:
        calls.append(("show", unit)) or {"ActiveState": "inactive", "MainPID": "0"})
    # The low-level start helper is independent of schema migration readiness.
    monkeypatch.setattr(service.Installation, "check_daemon", lambda self: None)
    service._owned_daemon_start(installation)
    assert calls == [("start", installation.unit)]
    campaign_unit = installation.directory.parent / "campaign-unit"
    campaign_unit.write_text("campaign")
    observed = []
    monkeypatch.setattr(service, "_unit_path", lambda unit=service.UNIT:
        observed.append(unit) or campaign_unit)
    assert service._recovery_sources(installation)["assets/service-unit"] == campaign_unit
    assert observed == [installation.unit]


def test_campaign_stop_confirms_only_its_owned_unit(tmp_path, monkeypatch):
    record = {"kind": "campaign", "installation_id": str(uuid.uuid4())}
    record["unit"] = service.installation_unit(record)
    installation = service.Installation(tmp_path, record)
    calls = []
    monkeypatch.setattr(service, "systemctl", lambda *args, **kw:
        calls.append(args) or subprocess.CompletedProcess(args, 0))
    monkeypatch.setattr(service, "unit_properties", lambda unit, *names:
        calls.append(("show", unit)) or {"ActiveState": "inactive", "MainPID": "0"})
    service._stop_owned(installation)
    assert calls == [("stop", installation.unit), ("show", installation.unit)]


def test_backup_hashes_exact_final_archive_and_copied_assets_without_rereads(recoverable, monkeypatch):
    import hashlib
    installation, archive, _actions = recoverable
    original_digest = service.file_sha256
    # Backup may verify identity through other owners, but cannot prehash tar assets
    # or reread its finished archive to compute the pointer.
    monkeypatch.setattr(service, "file_sha256", lambda path: pytest.fail(f"unneeded prehash/reread: {path}"))
    result = service.backup_service(installation, archive)
    data = archive.read_bytes()
    assert result["recovery"]["sha256"] == hashlib.sha256(data).hexdigest()
    with tarfile.open(archive, "r:") as tar:
        assert tar.getnames()[-1] == "recovery.json"
        metadata = json.load(tar.extractfile("recovery.json"))
        for name, expected in metadata["files"].items():
            body = tar.extractfile(name).read()
            assert expected["sha256"] == hashlib.sha256(body).hexdigest()
            assert expected["size"] == len(body)
    monkeypatch.setattr(service, "file_sha256", original_digest)
    assert service.restore_service(installation, archive)["outcome"] == "passed"


def test_archive_hash_writer_counts_short_writes_and_refuses_nonprogress():
    import hashlib
    class Short(io.BytesIO):
        def write(self, data):
            return super().write(data[:37])
    output = Short()
    writer = service._ArchiveHashWriter(output)
    body = bytes(range(256)) * 101
    assert writer.write(body) == len(body)
    writer.flush()
    assert writer.count == len(body)
    assert writer.digest.hexdigest() == hashlib.sha256(output.getvalue()).hexdigest()
    with pytest.raises(io.UnsupportedOperation):
        writer.seek(0)
    for count in (None, 0, -1, 100, True):
        broken = SimpleNamespace(write=lambda value, count=count: count)
        with pytest.raises(OSError, match="valid progress"):
            service._ArchiveHashWriter(broken).write(b"short")


@pytest.mark.parametrize("fault", ["asset", "write", "finalize", "flush", "file-fsync", "directory-fsync"])
def test_backup_fault_never_publishes_prefix_digest_and_restarts_owned_service(recoverable, monkeypatch, fault):
    installation, archive, actions = recoverable
    if fault == "asset":
        original = service._AssetHashReader.read
        def short(self, count=-1):
            return original(self, min(count, 1)) if count > 1 else original(self, count)
        monkeypatch.setattr(service._AssetHashReader, "read", short)
    elif fault == "write":
        monkeypatch.setattr(service._ArchiveHashWriter, "write", lambda self, data:
            (_ for _ in ()).throw(OSError("write failure")))
    elif fault == "finalize":
        original = tarfile.TarFile.close
        def close(self):
            original(self)
            if self.mode == "w":
                raise OSError("finalization failure")
        monkeypatch.setattr(tarfile.TarFile, "close", close)
    elif fault == "flush":
        monkeypatch.setattr(service._ArchiveHashWriter, "flush", lambda self:
            (_ for _ in ()).throw(OSError("flush failure")))
    else:
        original = os.fsync
        def fsync(fd):
            target = Path(f"/proc/self/fd/{fd}").resolve()
            if target == (archive if fault == "file-fsync" else archive.parent):
                raise OSError("durability failure")
            original(fd)
        monkeypatch.setattr(os, "fsync", fsync)
    with pytest.raises((OSError, service.FixtureBlocked)):
        service.backup_service(installation, archive)
    assert "recovery" not in installation.record
    assert "recovery" not in json.loads((installation.directory / "installation.json").read_text())
    assert "start" in actions and "open-admission" not in actions
    assert (installation.directory / "maintenance.json").exists()


@pytest.mark.parametrize("change", ["replace", "append", "add-directory"])
def test_backup_refuses_copy_time_asset_or_shape_change(recoverable, monkeypatch, change):
    installation, archive, actions = recoverable
    original = service._AssetHashReader.read
    changed = False
    def read(self, count=-1):
        nonlocal changed
        body = original(self, count)
        if body and not changed:
            changed = True
            path = Path(os.readlink(f"/proc/self/fd/{self.stream.fileno()}"))
            if change == "replace":
                replacement = path.with_name(".replacement")
                replacement.write_bytes(path.read_bytes())
                os.replace(replacement, path)
            elif change == "append":
                with path.open("ab") as output:
                    output.write(b"changed")
            else:
                (installation.directory / "new-empty-directory").mkdir()
        return body
    monkeypatch.setattr(service._AssetHashReader, "read", read)
    with pytest.raises(service.FixtureBlocked, match="changed"):
        service.backup_service(installation, archive)
    assert "recovery" not in installation.record
    assert "start" in actions and "open-admission" not in actions
