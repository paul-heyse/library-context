"""Explicit durable installation contracts; no server/database mutations in unit controls."""
from __future__ import annotations
import json
import io
import fcntl
import os
import subprocess
import shutil
import tarfile
import uuid
import contextlib
from types import SimpleNamespace
from pathlib import Path
import pytest
import surrealdb_service as service


def test_default_state_is_host_owned_and_shared_across_checkouts():
    assert service.state_root({"HOME": "/home/test"}) == Path("/home/test/.local/state/library-context/surrealdb")
    assert service.state_root({"HOME": "/home/test", "XDG_STATE_HOME": "/data/state"}) == Path("/data/state/library-context/surrealdb")


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
    result = subprocess.run([tool, "--user", "verify", str(path)], capture_output=True,
                            text=True, env=service.systemd_environment())
    assert result.returncode == 0, result.stderr
    assert not any(str(path) in line and "EnvironmentFile=" in line for line in result.stderr.splitlines()), result.stderr


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
        service.verify_binary({"LCTX_SURREAL_BIN": str(binary)}, runner=lambda *a, **kw: subprocess.CompletedProcess(a, 0, "3.2.0", ""))


def test_install_refuses_preexisting_unowned_unit(tmp_path, monkeypatch):
    binary = tmp_path / "surreal"
    binary.touch(mode=0o700)
    installer = tmp_path / "lctx"
    installer.touch(mode=0o700)
    root = tmp_path / "state"
    monkeypatch.setattr(service, "verify_binary", lambda *a, **kw: binary)
    monkeypatch.setattr(service, "systemd_available", lambda **kw: None)
    monkeypatch.setattr(service, "unit_properties", lambda *a, **kw: {"LoadState": "loaded", "ActiveState": "active"})
    with pytest.raises(service.FixtureBlocked, match="refusing adoption"):
        service.install(installer, env={"LCTX_SURREAL_SERVICE_CONFIG": str(root / "installation.json")})
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
    monkeypatch.setattr(service, "subprocess", type("Runner", (), {"run": staticmethod(lambda *a, **kw: subprocess.CompletedProcess(a, 1))}))
    with pytest.raises(service.FixtureFailed, match="native drain failed"):
        service._run_installer(installation, installer, "drain")


def test_native_export_batches_keep_individual_rows_restorable():
    assert service.EXPORT_BATCH_SIZE == 1
    assert service.SERVER_SETTINGS["SURREAL_EXPORT_BATCH_SIZE"] == "1"
    assert service.SERVER_SETTINGS["SURREAL_GRPC_MAX_MESSAGE_SIZE"] == "128MiB"
    assert not any("THREAD" in key or "MEMORY" in key or "CPU" in key for key in service.SERVER_SETTINGS)


@pytest.fixture
def recoverable(tmp_path, monkeypatch):
    root = tmp_path / "state"
    root.mkdir(mode=0o700)
    for name in ("attachments", "serving", "data/store", "product-leases/main", "product-leases/validation"):
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
    record = {"schema":1, "phase":"ready", "installation_id":str(uuid.uuid4()),
              "service_generation":list(range(32)), "unit":service.UNIT, "namespace":service.NAMESPACE,
              "databases":list(service.DATABASES), "export_batch_size":1,
              "state_root":str(root), "endpoint":"http://127.0.0.1:29999",
              "grpc_endpoint":"grpc://127.0.0.1:29999", "installer":str(installer),
              "binary":{"path":str(binary),"sha256":service.BINARY_SHA256,"version":service.VERSION}}
    service.private_json(root / "installation.json", record)
    for database in service.DATABASES:
        for role in ("runtime", "installer"):
            service.private_json(root / f"{database}-{role}.json", {"private":"credential", "database":database})
    (root / "server.env").write_text("SURREAL_PASS=private-root-secret\n")
    (root / "server.env").chmod(0o600)
    service.private_json(root / "main-selected.json", {"publication":"retained-selection"})
    service.private_json(root / "serving/control.json", {"handle":"exact-retained-handle"})
    actions = []
    monkeypatch.setattr(service.Installation, "check", lambda self, **kw: actions.append("ready") or {"outcome":"passed"})
    monkeypatch.setattr(service, "_run_installer", lambda inst, exe, action="init": actions.append(action))
    monkeypatch.setattr(service, "_stop_owned", lambda inst: actions.append("stop"))
    monkeypatch.setattr(service, "_start_owned", lambda inst: actions.append("start"))
    monkeypatch.setattr(service, "_schema_identities", lambda inst: {db:{"schema":"a"*64,"schema_version":3} for db in service.DATABASES})
    return service.Installation(root, record), tmp_path / "protected.tar", actions


def test_protected_backup_is_cold_private_complete_and_reopens_after_check(recoverable):
    installation, archive, actions = recoverable
    result = service.backup_service(installation, archive)
    assert actions[:5] == ["close-admission", "drain", "ready", "check", "stop"]
    assert actions[-4:] == ["start", "drain", "ready", "open-admission"]
    assert archive.stat().st_mode & 0o777 == 0o600
    assert not (installation.directory / "maintenance.json").exists()
    with tarfile.open(archive) as tar:
        names = set(tar.getnames())
        assert {"state/server.env", "assets/surreal", "assets/lctx", "assets/service-unit",
                "state/main-selected.json", "state/serving/control.json", "state/data/store/MANIFEST"} <= names
        assert not any(name in names for name in ("state/admission.lock", "state/borrowers.lock", "state/maintenance.json"))
        source = tar.extractfile("state/server.env")
        assert source is not None
        assert b"private-root-secret" in source.read()
    pointer = result["recovery"]
    assert pointer["sha256"] == service.file_sha256(archive)
    assert pointer == json.loads((installation.directory / "installation.json").read_text())["recovery"]
    assert "private-root-secret" not in json.dumps(pointer) and "credential" not in json.dumps(pointer)


def test_restore_default_validates_without_daemon_or_owned_state_effects(recoverable):
    installation, archive, actions = recoverable
    service.backup_service(installation, archive)
    actions.clear()
    before = {str(p.relative_to(installation.directory)):p.read_bytes() for p in installation.directory.rglob("*") if p.is_file()}
    result = service.restore_service(installation, archive)
    assert result["outcome"] == "passed" and result["applied"] is False
    assert not actions
    after = {str(p.relative_to(installation.directory)):p.read_bytes() for p in installation.directory.rglob("*") if p.is_file()}
    assert before == after


def test_restore_apply_preserves_live_lock_inodes_and_removes_only_checked_predecessor(recoverable):
    installation, archive, actions = recoverable
    service.backup_service(installation, archive)
    locks = {name:(installation.directory / name).stat().st_ino for name in ("admission.lock", "borrowers.lock")}
    (installation.directory / "data/store/MANIFEST").write_text("newer-data")
    actions.clear()
    result = service.restore_service(installation, archive, apply=True)
    assert result["applied"]
    assert (installation.directory / "data/store/MANIFEST").read_text() == "cold-owned-data"
    assert {name:(installation.directory / name).stat().st_ino for name in locks} == locks
    assert actions.index("stop") < actions.index("start") < actions.index("check") < actions.index("open-admission")
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
    marker = {"recovery":{"kind":"restore", "predecessors":[str(named)], "staging":[]}}
    service._retire_recovery_predecessors(installation, marker)
    assert not named.exists() and unmentioned.exists()
    unsafe = installation.directory.parent / (".recovery-predecessor-" + str(uuid.uuid4()))
    unsafe.mkdir()
    marker["recovery"]["predecessors"] = [str(unmentioned),str(unsafe)]
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
    monkeypatch.setattr(service, "verify_binary", lambda *a, **kw: Path(installation.record["binary"]["path"]))
    monkeypatch.setattr(service, "systemd_available", lambda **kw: None)
    monkeypatch.setattr(service, "systemctl", lambda *a, **kw: subprocess.CompletedProcess(a, 0))
    monkeypatch.setattr(service.urllib.request, "urlopen", lambda *a, **kw: contextlib.nullcontext(SimpleNamespace(status=200)))
    def refuse(self):
        raise service.FixtureBlocked("identity", "not the exact owned daemon")
    monkeypatch.setattr(service.Installation, "check_daemon", refuse)
    monkeypatch.setattr(service, "sql", lambda *a, **kw: pytest.fail("admin query preceded daemon identity proof"))
    with pytest.raises(service.FixtureBlocked, match="exact owned daemon"):
        service.install(installer, env={"LCTX_SURREAL_SERVICE_CONFIG":str(installation.directory / "installation.json")})


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
        metadata["identity"]["service_generation"] = [255]*32
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
    service.private_json(external, {"publication":"original-external-selection"})
    cfg = installation.directory / "main-runtime.json"
    service.private_json(cfg, {"private":"credential", "database":"main", "selection":str(external)})
    coordination = installation.directory.parent / "owned-extra-coordination"
    coordination.mkdir(mode=0o700)
    (coordination / "lifecycle.lock").touch(mode=0o600)
    (coordination / "state.json").write_text("original-coordination")
    installation.record["recovery_assets"] = [{"kind":"coordination", "path":str(coordination)}]
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
    service.private_json(installation.directory / "main-runtime.json", {"selection":str(external)})
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
    service.private_json(cfg, {"private":"credential", "database":"main", "selection":str(external)})
    service.backup_service(installation, archive)
    external.write_text("later-selection")
    service.restore_service(installation, archive, apply=True)
    assert not external.exists()
    alias = installation.directory.parent / "alias"
    alias.symlink_to(installation.directory, target_is_directory=True)
    with pytest.raises(service.FixtureBlocked, match="outside"):
        service.backup_service(installation, alias / "recursive.tar")
