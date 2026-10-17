"""Explicit durable installation contracts; no server/database mutations in unit controls."""

from __future__ import annotations

import contextlib
import fcntl
import io
import json
import os
import shutil
import signal
import subprocess
import sys
import tarfile
import time
import uuid
from pathlib import Path
from types import SimpleNamespace
from typing import Any

import pytest

import surrealdb_service as service
from surrealdb_service import _run_installer
from surrealdb_service import verify_binary as verify_official_binary

DAEMON_CHECK = service.Installation.check_daemon
INSTALLATION_CHECK = service.Installation.check
OWNED_START = service._start_owned
DISPOSAL_INSTALLER_RUN = service._run_disposal_installer


def required_record(value: dict[str, Any] | None) -> dict[str, Any]:
    assert value is not None, "expected owned receipt"
    return value


@pytest.fixture
def regeneration_ready(recoverable, monkeypatch, tmp_path):
    import service_regeneration as regeneration

    installation, _, actions = recoverable
    root = installation.directory
    candidate = tmp_path / "new-lctx"
    candidate.write_bytes(b"new immutable installer")
    candidate.chmod(0o700)
    server = {
        "path": str(tmp_path / "new-surreal"),
        "sha256": "a" * 64,
        "http_version": service.VERSION,
        "identity": "b" * 64,
    }
    state = {
        "inventory": {
            "namespaces": {service.NAMESPACE: "defined"},
            "databases": {db: "defined" for db in service.DATABASES},
        },
        "fail": None,
    }
    for db in service.DATABASES:
        for privileged in (False, True):
            path = installation.runtime_path(db, installer=privileged)
            value = json.loads(path.read_text())
            value.update(
                username=f"{db}_writer",
                password="old-secret",
                viewer_username=f"{db}_viewer",
                viewer_password="old-viewer",
                selection=str(root / f"{db}-selected.json"),
            )
            service.private_json(path, value)
    monkeypatch.setattr(service, "_owned_server_generation", lambda *a: server)
    monkeypatch.setattr(
        service,
        "_installation_executor",
        lambda inst, **kw: setattr(inst, "_daemon_identity", ("test", "test")) or True,
    )
    monkeypatch.setattr(
        service,
        "_campaign_current_quiescence",
        lambda *a, **kw: {"attachments": [], "coordination": {}, "executor_active": True},
    )
    monkeypatch.setattr(service, "systemctl", lambda *a: subprocess.CompletedProcess(a, 0))
    monkeypatch.setattr(service, "_owned_daemon_start", lambda inst: actions.append("start"))
    monkeypatch.setattr(regeneration.Storage, "publish", lambda *a, **kw: "test-owned-evidence")

    def sql(inst, statement, *, cfg):
        if statement.startswith("INFO"):
            return [
                {"result": {"namespaces": state["inventory"]["namespaces"]}},
                {"result": {"databases": state["inventory"]["databases"]}},
            ]
        return [{"result": None}]

    def cli(inst, action, token):
        assert service.maintenance_owned(inst, {"LCTX_SURREAL_MAINTENANCE_TOKEN": token})
        actions.append(action)
        if state["fail"] == action:
            raise service.FixtureFailed("installer", "controlled init failure")
        if action == "init":
            (root / "data/store").mkdir(exist_ok=True)
            (root / "data/store/current").write_text("new generation data")

    monkeypatch.setattr(service, "sql", sql)
    monkeypatch.setattr(regeneration, "_cli", cli)
    return installation, candidate, tmp_path / "server-record.json", state, actions


def test_regeneration_refuses_unexpected_scopes_and_identity_before_discard(regeneration_ready):
    import service_regeneration as regeneration

    installation, candidate, descriptor, state, actions = regeneration_ready
    for identity, scopes in (("foreign", "main,validation"), (installation.id, "validation")):
        with pytest.raises(service.FixtureBlocked, match="exact installation ID"):
            regeneration.regenerate(
                installation, identity, scopes=scopes, installer=candidate, descriptor=descriptor
            )
    state["inventory"]["databases"]["operator"] = "protected"
    with pytest.raises(service.FixtureBlocked, match="unexpected scopes"):
        regeneration.regenerate(
            installation,
            installation.id,
            scopes="main,validation",
            installer=candidate,
            descriptor=descriptor,
        )
    assert not actions
    assert (installation.directory / "data/store/MANIFEST").read_text() == "cold-owned-data"


@pytest.mark.parametrize(
    "kind,purpose", [("stable", service.LIFECYCLE_PURPOSE), ("campaign", None)]
)
def test_regeneration_refuses_unauthorized_borrowed_origin_before_discard(
    regeneration_ready, kind, purpose
):
    import service_regeneration as regeneration

    installation, candidate, descriptor, _, actions = regeneration_ready
    root = installation.directory
    installation.record.update(
        kind=kind, purpose=purpose, binary_origin=str(root.parent / "origin")
    )
    installation.record["unit"] = service.installation_unit(installation.record)
    service.private_json(root / "installation.json", installation.record)
    with pytest.raises(service.FixtureBlocked, match="not a lifecycle campaign"):
        regeneration.regenerate(
            installation,
            installation.id,
            scopes="main,validation",
            installer=candidate,
            descriptor=descriptor,
        )
    assert not actions
    assert not (root / "maintenance.json").exists()
    assert not (root / "regenerations").exists()
    assert (root / "data/store/MANIFEST").read_text() == "cold-owned-data"


@pytest.mark.parametrize(
    "damage", [None, "namespace", "database", "user", "table", "unknown", "missing"]
)
def test_regeneration_only_recognizes_pristine_generated_bootstrap(
    regeneration_ready, monkeypatch, damage
):
    import service_regeneration as regeneration

    installation, _, _, _, actions = regeneration_ready
    namespaces = {
        service.NAMESPACE: "defined",
        "main": "DEFINE NAMESPACE main COMMENT 'Default namespace generated by SurrealDB'",
    }
    namespace = {
        "accesses": {},
        "databases": {
            "main": "DEFINE DATABASE main COMMENT 'Default database generated by SurrealDB'"
        },
        "users": {},
    }
    database = {
        name: {}
        for name in (
            "accesses",
            "analyzers",
            "apis",
            "buckets",
            "configs",
            "functions",
            "models",
            "modules",
            "params",
            "sequences",
            "tables",
            "users",
        )
    }
    if damage == "namespace":
        namespaces["main"] = "DEFINE NAMESPACE main"
    elif damage == "database":
        namespace["databases"]["operator"] = "protected"
    elif damage == "user":
        namespace["users"]["operator"] = "protected"
    elif damage == "table":
        database["tables"]["operator"] = "protected"
    elif damage == "unknown":
        database["unknown"] = {}
    elif damage == "missing":
        del database["tables"]

    def sql(inst, statement, *, cfg):
        assert inst is installation
        if cfg["namespace"] == service.NAMESPACE:
            assert statement == "INFO FOR ROOT; INFO FOR NS;"
            return [
                {
                    "result": {
                        "namespaces": namespaces,
                        "defaults": {"namespace": "main", "database": "main"},
                    }
                },
                {"result": {"databases": {db: "defined" for db in service.DATABASES}}},
            ]
        assert cfg["namespace"] == "main"
        if cfg["database"] is None:
            assert statement == "INFO FOR NS;"
            return [{"result": namespace}]
        assert cfg["database"] == "main" and statement == "INFO FOR DB;"
        return [{"result": database}]

    monkeypatch.setattr(service, "sql", sql)
    if damage:
        with pytest.raises(service.FixtureBlocked, match="unexpected scopes"):
            regeneration._inventory(installation)
    else:
        assert regeneration._inventory(installation) == {
            "namespace": service.NAMESPACE,
            "databases": sorted(service.DATABASES),
            "bootstrap": {
                "namespace": namespaces["main"],
                "database": namespace["databases"]["main"],
                "contents": "empty",
            },
        }
        assert service._server_environment("installer", "secret")["SURREAL_NO_DEFAULTS"] == "true"
    assert not actions
    assert (installation.directory / "data/store/MANIFEST").read_text() == "cold-owned-data"


@pytest.mark.parametrize("borrowed", [False, True])
def test_regeneration_failure_resumes_without_discarding_successor(
    regeneration_ready, monkeypatch, borrowed
):
    import service_regeneration as regeneration

    installation, candidate, descriptor, state, actions = regeneration_ready
    root = installation.directory
    origin = root
    if borrowed:
        origin = root.parent / "original-service"
        installation.record.update(
            kind="campaign", purpose=service.LIFECYCLE_PURPOSE, binary_origin=str(origin)
        )
        installation.record["unit"] = service.installation_unit(installation.record)
        service.private_json(root / "installation.json", installation.record)
    server = service._owned_server_generation(origin, descriptor)
    validated_origins = []

    def owned_server(directory, selected_descriptor):
        # A provenance validator must be invoked against the unchanged authorized owner,
        # including when the installed descriptor has already become the successor.
        assert directory == origin
        assert selected_descriptor == descriptor
        validated_origins.append(directory)
        return server

    monkeypatch.setattr(service, "_owned_server_generation", owned_server)
    previous = installation.record["service_generation"]
    state["fail"] = "check"
    with pytest.raises(service.FixtureFailed, match="controlled init failure"):
        regeneration.regenerate(
            installation,
            installation.id,
            scopes="main,validation",
            installer=candidate,
            descriptor=descriptor,
        )
    assert not (root / "data/store/MANIFEST").exists()
    assert (root / "data/store/current").read_text() == "new generation data"
    assert not (root / "main-selected.json").exists()
    assert "open-admission" not in actions
    marker = json.loads((root / "maintenance.json").read_text())
    dead = {**service.ProcessIdentity.of().to_json(), "boot_id": str(uuid.uuid4())}
    marker["owner"] = dead
    service.private_json(root / "maintenance.json", marker)
    state["fail"] = None
    successor = service.Installation(root, json.loads((root / "installation.json").read_text()))
    # The real descriptor validates full binary provenance; this protocol control
    # supplies its already-checked private descriptor boundary.
    monkeypatch.setattr(
        service, "_descriptor", lambda path: json.loads((path / "installation.json").read_text())
    )
    actions.clear()
    result = regeneration.regenerate(
        successor,
        successor.id,
        scopes="main,validation",
        installer=candidate,
        descriptor=descriptor,
    )
    assert result["outcome"] == "passed"
    assert actions == ["stop", "start", "check", "ready", "open-admission"]
    assert (root / "data/store/current").read_text() == "new generation data"
    assert not (root / "maintenance.json").exists()
    final = json.loads((root / "installation.json").read_text())
    assert validated_origins == [origin, origin]
    assert service._server_binary_origin(service.Installation(root, final)) == origin
    _, plan = regeneration.load_plan(service.Installation(root, final), final["regeneration"])
    assert plan["previous"].get("binary_origin") == plan["successor"].get("binary_origin")
    assert final.get("binary_origin") == (str(origin) if borrowed else None)
    assert plan["server"] == server
    assert final["service_generation"] != previous
    assert final["installation_id"] == installation.id
    assert not list((root / "regenerations").glob("*/staged/*"))
    assert (
        regeneration.completed_discard(root, final["regeneration"])["generation"]
        == bytes(previous).hex()
    )


@pytest.mark.parametrize("failure", ["shutdown", "partial-config", "open-admission"])
def test_regeneration_interrupted_boundaries_keep_exact_resume(
    regeneration_ready, monkeypatch, failure
):
    import service_regeneration as regeneration

    installation, candidate, descriptor, state, actions = regeneration_ready
    root = installation.directory
    stop, write = service._stop_owned, service._write_private

    def fail_stop(inst):
        stop(inst)
        raise service.FixtureBlocked("stop", "interrupted shutdown")

    def fail_write(path, content):
        write(path, content)
        if path == root / "main-runtime.json":
            raise service.FixtureBlocked("write", "interrupted config installation")

    if failure == "shutdown":
        monkeypatch.setattr(service, "_stop_owned", fail_stop)
    elif failure == "partial-config":
        monkeypatch.setattr(service, "_write_private", fail_write)
    else:
        state["fail"] = "open-admission"
    with pytest.raises((service.FixtureBlocked, service.FixtureFailed)):
        regeneration.regenerate(
            installation,
            installation.id,
            scopes="main,validation",
            installer=candidate,
            descriptor=descriptor,
        )
    marker = json.loads((root / "maintenance.json").read_text())
    journal = Path(marker["regeneration"]["journal"])
    phase = json.loads((journal.parent / "progress.json").read_text())["phase"]
    assert (
        phase
        == {
            "shutdown": "discarding",
            "partial-config": "discarded",
            "open-admission": "initialized",
        }[failure]
    )
    if failure == "open-admission":
        assert actions[-1] == "close-admission"
    marker["owner"] = {**service.ProcessIdentity.of().to_json(), "boot_id": str(uuid.uuid4())}
    service.private_json(root / "maintenance.json", marker)
    monkeypatch.setattr(service, "_stop_owned", stop)
    monkeypatch.setattr(service, "_write_private", write)
    monkeypatch.setattr(
        service, "_descriptor", lambda path: json.loads((path / "installation.json").read_text())
    )
    state["fail"] = None
    current = service.Installation(root, json.loads((root / "installation.json").read_text()))
    assert (
        regeneration.regenerate(
            current,
            current.id,
            scopes="main,validation",
            installer=candidate,
            descriptor=descriptor,
        )["outcome"]
        == "passed"
    )
    assert (root / "data/store/current").read_text() == "new generation data"


def attachment_runtime(installation, directory):
    """Current private attachment identity, independent from native completion evidence."""
    (directory / "scratch").mkdir(mode=0o700, exist_ok=True)
    value = {
        "installation_id": installation.id,
        "fixture": installation.id,
        "attachment": directory.name,
        "attempt_id": directory.name,
        "service_generation": installation.record["service_generation"],
        "namespace": service.NAMESPACE,
        "database": "validation",
        "endpoint": installation.record["endpoint"],
        "grpc_endpoint": installation.record["grpc_endpoint"],
        "scratch": str(directory / "scratch"),
        "admin_password": "private test credential",
    }
    service.private_json(directory / "runtime.json", value)
    return value


@pytest.mark.parametrize("native", [False, True])
def test_regeneration_pending_attachment_uses_exact_runtime_without_claiming_native_completion(
    regeneration_ready, monkeypatch, native
):
    import service_regeneration as regeneration

    installation, _, _, _, _ = regeneration_ready
    directory = installation.directory / "attachments" / str(uuid.uuid4())
    directory.mkdir(mode=0o700)
    attachment_runtime(installation, directory)
    row = {
        "id": directory.name,
        "fixture": installation.id,
        "namespace": service.NAMESPACE,
        "database": "validation",
        "released": True,
        "command_cleanup": {"status": "unknown", "reason": "unchanged original"},
    }
    if native:
        row["native_drain"] = {
            "attachment": directory.name,
            "generation": bytes(installation.record["service_generation"]).hex(),
            "directory": str(directory / "native-drain"),
        }
    service.private_json(directory / "record.json", row)
    original = (directory / "record.json").read_bytes()
    monkeypatch.setattr(service, "borrowers", lambda _: [row])
    regeneration._pending_bindings(installation)
    assert (directory / "record.json").read_bytes() == original
    assert row["command_cleanup"]["status"] == "unknown"
    assert ("native_drain" in row) == native


@pytest.mark.parametrize(
    "damage",
    [
        "fixture",
        "installation_id",
        "attachment",
        "attempt_id",
        "service_generation",
        "namespace",
        "database",
        "endpoint",
        "grpc_endpoint",
        "scratch",
        "runtime-symlink",
        "scratch-symlink",
        "native-null",
        "native-generation",
        "native-directory",
        "row-namespace",
        "row-database",
    ],
)
def test_regeneration_pending_attachment_refuses_foreign_or_malformed_runtime(
    regeneration_ready, monkeypatch, damage
):
    import service_regeneration as regeneration

    installation, _, _, _, _ = regeneration_ready
    directory = installation.directory / "attachments" / str(uuid.uuid4())
    directory.mkdir(mode=0o700)
    runtime = attachment_runtime(installation, directory)
    row = {
        "id": directory.name,
        "fixture": installation.id,
        "namespace": service.NAMESPACE,
        "database": "validation",
    }
    if damage.startswith("row-"):
        row[damage.removeprefix("row-")] = "foreign"
    elif damage.startswith("native-"):
        row["native_drain"] = (
            None
            if damage == "native-null"
            else {
                "attachment": directory.name,
                "generation": (
                    "0" * 64
                    if damage == "native-generation"
                    else bytes(installation.record["service_generation"]).hex()
                ),
                "directory": (
                    "/foreign/native-drain"
                    if damage == "native-directory"
                    else str(directory / "native-drain")
                ),
            }
        )
    elif damage == "runtime-symlink":
        (directory / "runtime.json").rename(directory / "redirect.json")
        (directory / "runtime.json").symlink_to(directory / "redirect.json")
    elif damage == "scratch-symlink":
        (directory / "scratch").rename(directory / "redirect-scratch")
        (directory / "scratch").symlink_to(directory / "redirect-scratch")
    else:
        runtime[damage] = [0] * 32 if damage == "service_generation" else "foreign"
        service.private_json(directory / "runtime.json", runtime)
    monkeypatch.setattr(service, "borrowers", lambda _: [row])
    with pytest.raises(service.FixtureBlocked, match="runtime scope or generation differs"):
        regeneration._pending_bindings(installation)


@pytest.mark.parametrize("native", [False, True])
def test_regeneration_attachment_discard_preserves_unknown_and_rejects_tampering(
    regeneration_ready, monkeypatch, native
):
    import service_regeneration as regeneration
    import surrealdb_fixture as fixture

    installation, candidate, descriptor, _, _ = regeneration_ready
    root = installation.directory
    generation = bytes(installation.record["service_generation"]).hex()
    regeneration.regenerate(
        installation,
        installation.id,
        scopes="main,validation",
        installer=candidate,
        descriptor=descriptor,
    )
    final = json.loads((root / "installation.json").read_text())
    directory = root / "attachments" / str(uuid.uuid4())
    directory.mkdir(mode=0o700)
    row = {
        "id": directory.name,
        "fixture": installation.id,
        "namespace": service.NAMESPACE,
        "database": "validation",
        "released": False,
        "owner": {**service.ProcessIdentity.of().to_json(), "boot_id": str(uuid.uuid4())},
        "command_cleanup": {"status": "unknown", "reason": "original uncertainty"},
        "native_drain": {
            "generation": generation,
            "attachment": directory.name,
            "directory": str(directory / "native-drain"),
        },
    }
    if not native:
        del row["native_drain"]
    service.private_json(directory / "record.json", row)
    attachment_runtime(installation, directory)
    monkeypatch.setattr(fixture, "_complete_attachment_obligations", lambda *a, **kw: None)
    acknowledged = regeneration.acknowledge_attachment(directory, final["regeneration"])
    assert acknowledged["command_cleanup"] == row["command_cleanup"]
    assert acknowledged.get("native_drain") == row.get("native_drain")
    assert ("native_drain" in acknowledged) == native
    assert regeneration.completed_attachment(directory, acknowledged)
    assert not fixture.native_protected(directory, acknowledged, {"status": "unknown"})
    changed = {
        **acknowledged,
        "native_drain": {**row.get("native_drain", {}), "generation": "0" * 64},
    }
    assert not regeneration.completed_attachment(directory, changed)
    assert fixture.native_protected(directory, changed, {"status": "unknown"})
    receipt_path = directory / "installation-discard.json"
    receipt = json.loads(receipt_path.read_text())
    receipt["binding"]["generation"] = "0" * 64
    receipt_path.chmod(0o600)
    service.private_json(receipt_path, receipt)
    receipt_path.chmod(0o400)
    forged = {
        **acknowledged,
        "installation_discard": {
            "path": str(receipt_path),
            "sha256": service.file_sha256(receipt_path),
        },
    }
    assert not regeneration.completed_attachment(directory, forged)


def test_regeneration_acknowledges_missing_native_binding_only_after_whole_executor_discard(
    regeneration_ready, monkeypatch
):
    import service_regeneration as regeneration
    import surrealdb_fixture as fixture

    installation, candidate, descriptor, _, actions = regeneration_ready
    directory = installation.directory / "attachments" / str(uuid.uuid4())
    directory.mkdir(mode=0o700)
    runtime = attachment_runtime(installation, directory)
    row = {
        "id": directory.name,
        "fixture": installation.id,
        "namespace": service.NAMESPACE,
        "database": "validation",
        "released": True,
        "interrupted": True,
        "child_exit_code": -15,
        "owner": {**service.ProcessIdentity.of().to_json(), "boot_id": str(uuid.uuid4())},
        "command_cleanup": {"status": "unknown", "reason": "original uncertainty"},
    }
    service.private_json(directory / "record.json", row)
    original_stop = service._stop_owned
    stops_before = actions.count("stop")

    def stop(current):
        assert json.loads((directory / "record.json").read_text()) == row
        assert not (directory / "installation-discard.json").exists()
        original_stop(current)

    monkeypatch.setattr(service, "_stop_owned", stop)
    monkeypatch.setattr(fixture, "_complete_attachment_obligations", lambda *a, **kw: None)
    regeneration.regenerate(
        installation,
        installation.id,
        scopes="main,validation",
        installer=candidate,
        descriptor=descriptor,
    )
    acknowledged = json.loads((directory / "record.json").read_text())
    receipt = json.loads((directory / "installation-discard.json").read_text())
    assert receipt["original"] == row
    assert "native_drain" not in acknowledged
    assert acknowledged["command_cleanup"] == row["command_cleanup"]
    assert regeneration.completed_attachment(directory, acknowledged)
    assert "private test credential" not in (directory / "installation-discard.json").read_text()
    assert actions.count("stop") > stops_before
    runtime["database"] = "foreign"
    service.private_json(directory / "runtime.json", runtime)
    assert not regeneration.completed_attachment(directory, acknowledged)


@pytest.mark.parametrize("native", [False, True])
def test_regeneration_existing_native_bound_discard_ack_remains_current_without_new_binding(
    regeneration_ready, monkeypatch, native
):
    import service_regeneration as regeneration
    import surrealdb_fixture as fixture

    installation, candidate, descriptor, _, _ = regeneration_ready
    directory = installation.directory / "attachments" / str(uuid.uuid4())
    directory.mkdir(mode=0o700)
    attachment_runtime(installation, directory)
    row = {
        "id": directory.name,
        "fixture": installation.id,
        "namespace": service.NAMESPACE,
        "database": "validation",
        "released": False,
        "owner": {**service.ProcessIdentity.of().to_json(), "boot_id": str(uuid.uuid4())},
        "command_cleanup": {"status": "unknown", "reason": "original uncertainty"},
    }
    if native:
        row["native_drain"] = {
            "generation": bytes(installation.record["service_generation"]).hex(),
            "attachment": directory.name,
            "directory": str(directory / "native-drain"),
        }
    service.private_json(directory / "record.json", row)
    monkeypatch.setattr(fixture, "_complete_attachment_obligations", lambda *a, **kw: None)
    regeneration.regenerate(
        installation,
        installation.id,
        scopes="main,validation",
        installer=candidate,
        descriptor=descriptor,
    )
    acknowledged = json.loads((directory / "record.json").read_text())
    receipt_path = directory / "installation-discard.json"
    receipt = json.loads(receipt_path.read_text())
    del receipt["binding"]
    receipt_path.chmod(0o600)
    service.private_json(receipt_path, receipt)
    receipt_path.chmod(0o400)
    acknowledged["installation_discard"]["sha256"] = service.file_sha256(receipt_path)
    service.private_json(directory / "record.json", acknowledged)
    old_bytes = receipt_path.read_bytes()
    assert regeneration.completed_attachment(directory, acknowledged) is native
    if native:
        # The predecessor executor was discarded and installation now names its successor.
        final = json.loads((installation.directory / "installation.json").read_text())
        assert final["service_generation"] != installation.record["service_generation"]
        assert regeneration.acknowledge_attachment(directory, final["regeneration"]) == acknowledged
        assert receipt_path.read_bytes() == old_bytes
        native_drain = row["native_drain"]
        assert isinstance(native_drain, dict)
        changed = {
            **acknowledged,
            "native_drain": {**native_drain, "directory": "/foreign/native-drain"},
        }
        assert not regeneration.completed_attachment(directory, changed)


def test_regeneration_skips_settled_native_attachment_without_obsolete_runtime_or_scratch(
    regeneration_ready,
):
    import service_regeneration as regeneration
    import surrealdb_fixture as fixture

    installation, candidate, descriptor, _, _ = regeneration_ready
    directory = installation.directory / "attachments" / str(uuid.uuid4())
    directory.mkdir(mode=0o700)
    native = directory / "native-drain"
    native.mkdir(mode=0o700)
    generation = bytes(installation.record["service_generation"]).hex()
    row = {
        "id": directory.name,
        "fixture": installation.id,
        "namespace": service.NAMESPACE,
        "database": "validation",
        "released": True,
        "owner": {**service.ProcessIdentity.of().to_json(), "boot_id": str(uuid.uuid4())},
        "command_cleanup": {"status": "confirmed", "survivors": []},
        "native_drain": {
            "generation": generation,
            "attachment": directory.name,
            "directory": str(native),
        },
    }
    service.private_json(directory / "record.json", row)
    service.private_json(
        native / "1-2-0.json",
        {
            "schema": 1,
            "kind": "native-process-owner",
            "attachment": directory.name,
            "generation": generation,
            "owner": "completed-control",
            "process": {"pid": 1, "start_ticks": 2},
            "registration": 0,
            "state": "complete",
            "completion": {
                "local": "terminal",
                "remote": "confirmed",
                "failures": [],
                "committed": [],
                "storage": [],
            },
        },
    )
    original = (directory / "record.json").read_bytes()
    assert fixture.native_completion(directory, row)["status"] == "confirmed"
    assert not service.borrowers(installation)
    result = regeneration.regenerate(
        installation,
        installation.id,
        scopes="main,validation",
        installer=candidate,
        descriptor=descriptor,
    )
    assert result["outcome"] == "passed"
    assert result["admission_open"] is True
    assert (directory / "record.json").read_bytes() == original
    assert not (directory / "installation-discard.json").exists()
    assert not (directory / "runtime.json").exists()
    assert not (directory / "scratch").exists()


def test_regeneration_unsettled_attachment_without_runtime_still_refuses_before_effects(
    regeneration_ready,
):
    import service_regeneration as regeneration

    installation, candidate, descriptor, _, actions = regeneration_ready
    directory = installation.directory / "attachments" / str(uuid.uuid4())
    directory.mkdir(mode=0o700)
    row = {
        "id": directory.name,
        "fixture": installation.id,
        "namespace": service.NAMESPACE,
        "database": "validation",
        "released": False,
        "owner": {**service.ProcessIdentity.of().to_json(), "boot_id": str(uuid.uuid4())},
    }
    service.private_json(directory / "record.json", row)
    original = (directory / "record.json").read_bytes()
    assert service.borrowers(installation)
    with pytest.raises(service.FixtureBlocked, match="runtime scope or generation differs"):
        regeneration.regenerate(
            installation,
            installation.id,
            scopes="main,validation",
            installer=candidate,
            descriptor=descriptor,
        )
    assert actions == []
    assert (installation.directory / "data/store/MANIFEST").read_text() == "cold-owned-data"
    assert (directory / "record.json").read_bytes() == original


def archive_member(archive: tarfile.TarFile, name: str):
    member = archive.extractfile(name)
    assert member is not None, f"expected archive file: {name}"
    return member


def test_current_installer_schema_requires_format9_and_exact_digest():
    service._current_schema({"schema_version": 9, "schema": "a" * 64})
    for version in (None, 3, 7, 8, "9"):
        with pytest.raises(service.FixtureBlocked, match="current native format9 schema"):
            service._current_schema({"schema_version": version, "schema": "a" * 64})
    for digest in (None, "a" * 63, "A" * 64):
        with pytest.raises(service.FixtureBlocked, match="current native format9 schema"):
            service._current_schema({"schema_version": 9, "schema": digest})


@pytest.fixture
def installer_handoff_ready(recoverable, tmp_path, monkeypatch):
    import storage_service

    installation, _, actions = recoverable
    old = Path(installation.record["installer"])
    old.chmod(0o700)
    original = storage_service.prepare(installation.directory, old)
    installation.record.update(
        installer=original["path"], installer_generation=original, native_schema_version=9
    )
    service.private_json(installation.directory / "installation.json", installation.record)
    candidate = tmp_path / "replacement-lctx"
    candidate.write_bytes(b"current fixed installer")
    candidate.chmod(0o700)
    state = {
        "failure": None,
        "ignore_close": None,
        "markers": {
            db: {
                "generation": bytes(installation.record["service_generation"]).hex(),
                "schema": "a" * 64,
                "schema_version": 9,
                "admission_open": True,
            }
            for db in service.DATABASES
        },
    }
    monkeypatch.setattr(service.Installation, "check_daemon", lambda inst: None)
    monkeypatch.setattr(
        service,
        "sql",
        lambda inst, query, *, cfg: [{"result": [dict(state["markers"][cfg["database"]])]}],
    )

    def cli(inst, generation, database, action, token):
        assert service.lock_held(inst.directory / "admission.lock")
        assert service.lock_held(inst.directory / "borrowers.lock")
        assert service.file_sha256(Path(generation["path"])) == generation["sha256"]
        assert generation != original, "no operation invokes the broken predecessor CLI"
        actions.append((database, action))
        if action == "schema":
            return {"schema": "a" * 64, "schema_version": 9}
        marker = json.loads((inst.directory / "maintenance.json").read_text())
        assert token == marker["token"] and service.maintenance_owned(
            inst, {"LCTX_SURREAL_MAINTENANCE_TOKEN": token}
        )
        if state["failure"] == (database, action):
            raise service.FixtureFailed("installer", "controlled second scope failure")
        if action == "close-admission" and state["ignore_close"] != database:
            state["markers"][database]["admission_open"] = False
        if action == "open-admission":
            state["markers"][database]["admission_open"] = True
        return {
            "ready": action in {"open-admission", "check"},
            "namespace": service.NAMESPACE,
            "database": database,
        }

    monkeypatch.setattr(service, "_installer_handoff_cli", cli)
    return installation, candidate, actions, state


def _ended_installer_handoff_owner(installation):
    path = installation.directory / "maintenance.json"
    marker = json.loads(path.read_text())
    marker["owner"]["start_ticks"] += 1
    service.private_json(path, marker)


def test_installer_handoff_changes_only_executable_fields_after_both_checks(
    installer_handoff_ready,
):
    installation, candidate, actions, state = installer_handoff_ready
    before = json.loads(json.dumps(installation.record))
    protected = {path: path.read_bytes() for path in installation.directory.glob("*-*.json")}
    result = service.handoff_installer(installation, candidate)
    assert result["outcome"] == "passed"
    assert {key for key in before if before[key] != installation.record[key]} == {
        "installer",
        "installer_generation",
    }
    assert (
        json.loads((installation.directory / "installation.json").read_text())
        == installation.record
    )
    assert all(path.read_bytes() == raw for path, raw in protected.items())
    assert all(value["admission_open"] for value in state["markers"].values())
    assert actions.index(("validation", "check")) < actions.index(("main", "open-admission"))
    assert not (installation.directory / "maintenance.json").exists()
    journal = next((installation.directory / "installer-handoffs").glob("*/plan.json"))
    import base64

    plan = json.loads(journal.read_text())
    assert json.loads(base64.b64decode(plan["previous_bytes"])) == before
    assert json.loads(base64.b64decode(plan["successor_bytes"])) == installation.record
    assert (journal.parent / "receipt.json").exists()


def test_installer_handoff_second_schema_mismatch_has_no_pointer_or_native_effect(
    installer_handoff_ready,
):
    installation, candidate, actions, state = installer_handoff_ready
    before = (installation.directory / "installation.json").read_bytes()
    state["markers"]["validation"]["schema"] = "b" * 64
    with pytest.raises(service.FixtureBlocked, match="candidate schema differs"):
        service.handoff_installer(installation, candidate)
    assert (installation.directory / "installation.json").read_bytes() == before
    assert actions == [("main", "schema")]
    assert not (installation.directory / "maintenance.json").exists()


def test_installer_handoff_second_drain_failure_keeps_exact_old_pointer_and_both_closed(
    installer_handoff_ready,
):
    installation, candidate, actions, state = installer_handoff_ready
    before = (installation.directory / "installation.json").read_bytes()
    state["failure"] = ("validation", "drain")
    with pytest.raises(service.FixtureFailed, match="second scope"):
        service.handoff_installer(installation, candidate)
    assert (installation.directory / "installation.json").read_bytes() == before
    assert not any(row["admission_open"] for row in state["markers"].values())
    assert not any(action == "open-admission" for _, action in actions)
    assert (installation.directory / "maintenance.json").exists()
    import storage_service

    dependencies = storage_service.dependencies(installation)["dependencies"]
    assert sum(row["role"] == "unresolved-installer-handoff" for row in dependencies) == 2


@pytest.mark.parametrize("refused_database", service.DATABASES)
def test_installer_handoff_ordinary_readiness_refusal_closes_both_and_retains_journal(
    installer_handoff_ready, monkeypatch, refused_database
):
    installation, candidate, actions, state = installer_handoff_ready
    ordinary_reads = []

    def observed_sql(inst, query, *, cfg):
        marker = state["markers"][cfg["database"]]
        if cfg["authentication"] == "database":
            ordinary_reads.append(cfg["database"])
            assert all(value["admission_open"] for value in state["markers"].values())
            if cfg["database"] == refused_database:
                raise service.FixtureBlocked("query", "controlled ordinary credential refusal")
            return [{"result": [{key: marker[key] for key in ("generation", "schema_version")}]}]
        return [{"result": [dict(marker)]}]

    monkeypatch.setattr(service, "sql", observed_sql)
    monkeypatch.setattr(service.Installation, "check", INSTALLATION_CHECK)
    with pytest.raises(service.FixtureBlocked, match="ordinary credential refusal"):
        service.handoff_installer(installation, candidate)
    assert ordinary_reads == list(
        service.DATABASES[: service.DATABASES.index(refused_database) + 1]
    )
    assert ("validation", "open-admission") in actions
    assert not any(value["admission_open"] for value in state["markers"].values())
    assert (installation.directory / "maintenance.json").exists()
    journal = next((installation.directory / "installer-handoffs").glob("*/plan.json"))
    assert (journal.parent / "checked.json").exists()
    assert not (journal.parent / "receipt.json").exists()
    assert (
        json.loads((installation.directory / "installation.json").read_bytes())
        == installation.record
    )


@pytest.mark.parametrize("after_publication", [False, True])
def test_installer_handoff_exact_resume_before_and_after_pointer_publication(
    installer_handoff_ready, monkeypatch, after_publication
):
    installation, candidate, _actions, state = installer_handoff_ready
    previous = (installation.directory / "installation.json").read_bytes()
    write = service._write_private

    def interrupt(path, content):
        if after_publication:
            write(path, content)
        raise KeyboardInterrupt("controlled publication interruption")

    monkeypatch.setattr(service, "_write_private", interrupt)
    with pytest.raises(KeyboardInterrupt):
        service.handoff_installer(installation, candidate)
    assert (
        (installation.directory / "installation.json").read_bytes() != previous
    ) == after_publication
    assert not any(value["admission_open"] for value in state["markers"].values())
    journal = next((installation.directory / "installer-handoffs").glob("*/plan.json"))
    exact_plan = journal.read_bytes()
    _ended_installer_handoff_owner(installation)
    monkeypatch.setattr(service, "_write_private", write)
    installation = service.Installation(
        installation.directory,
        json.loads((installation.directory / "installation.json").read_text()),
    )
    assert service.handoff_installer(installation, candidate)["outcome"] == "passed"
    assert journal.read_bytes() == exact_plan
    assert all(row["admission_open"] for row in state["markers"].values())


def test_installer_handoff_refuses_unacknowledged_close_without_normalizing_open_marker(
    installer_handoff_ready,
):
    installation, candidate, _, state = installer_handoff_ready
    before = (installation.directory / "installation.json").read_bytes()
    state["ignore_close"] = "validation"
    with pytest.raises(service.FixtureBlocked, match="admission acknowledgement differs"):
        service.handoff_installer(installation, candidate)
    assert (installation.directory / "installation.json").read_bytes() == before
    assert state["markers"]["validation"]["admission_open"] is True
    assert (installation.directory / "maintenance.json").exists()


def test_installer_handoff_resume_uses_exact_owned_candidate_after_checkout_source_disappears(
    installer_handoff_ready,
):
    installation, candidate, _, state = installer_handoff_ready
    state["failure"] = ("validation", "drain")
    with pytest.raises(service.FixtureFailed):
        service.handoff_installer(installation, candidate)
    journal = next((installation.directory / "installer-handoffs").glob("*/plan.json"))
    original_plan = journal.read_bytes()
    owned = Path(json.loads(original_plan)["candidate"]["path"])
    candidate.unlink()
    _ended_installer_handoff_owner(installation)
    state["failure"] = None
    assert service.handoff_installer(installation, owned)["outcome"] == "passed"
    assert journal.read_bytes() == original_plan


def test_installer_handoff_refuses_inherited_native_owner(installer_handoff_ready, monkeypatch):
    installation, candidate, actions, _ = installer_handoff_ready
    monkeypatch.setenv("LCTX_NATIVE_DRAIN_DIRECTORY", "/another/native-owner")
    with pytest.raises(service.FixtureBlocked, match="inherited native owner"):
        service.handoff_installer(installation, candidate)
    assert not actions and not (installation.directory / "maintenance.json").exists()


@pytest.mark.parametrize("change", ["candidate", "descriptor", "config"])
def test_installer_handoff_resume_refuses_changed_checkpoint(installer_handoff_ready, change):
    installation, candidate, actions, state = installer_handoff_ready
    state["failure"] = ("validation", "drain")
    with pytest.raises(service.FixtureFailed):
        service.handoff_installer(installation, candidate)
    _ended_installer_handoff_owner(installation)
    if change == "candidate":
        candidate.write_bytes(b"different candidate at same path")
    elif change == "descriptor":
        service.private_json(
            installation.directory / "installation.json", {**installation.record, "foreign": True}
        )
    else:
        path = installation.runtime_path("validation", installer=True)
        service.private_json(path, {**json.loads(path.read_text()), "foreign": True})
    before = (installation.directory / "installation.json").read_bytes()
    installation = service.Installation(installation.directory, json.loads(before))
    previous_actions = list(actions)
    with pytest.raises(service.FixtureBlocked, match=r"checkpoint|previous or successor"):
        service.handoff_installer(installation, candidate)
    assert (installation.directory / "installation.json").read_bytes() == before
    assert (installation.directory / "maintenance.json").exists()
    assert actions == previous_actions, "changed checkpoint must not direct native cleanup effects"


@pytest.mark.parametrize("owner", ["live", "specialized", "foreign", "borrower"])
def test_installer_handoff_refuses_existing_owner_before_candidate_effects(
    installer_handoff_ready, owner, monkeypatch
):
    installation, candidate, actions, _ = installer_handoff_ready
    identity = service.ProcessIdentity.of().to_json()
    if owner == "borrower":
        monkeypatch.setattr(service, "borrowers", lambda inst: [{"id": "unresolved"}])
    else:
        if owner == "specialized":
            identity["start_ticks"] += 1
        if owner == "foreign":
            identity["pid_namespace"] += 1
        service.private_json(
            installation.directory / "maintenance.json",
            {
                "owner": identity,
                "operation": "native named attachment recovery"
                if owner == "specialized"
                else service.INSTALLER_HANDOFF_OPERATION,
                "installer_handoff": {"operation": "a" * 64},
            },
        )
    with pytest.raises(service.FixtureBlocked, match=r"owner|borrowers"):
        service.handoff_installer(installation, candidate)
    assert actions == []


@pytest.mark.parametrize("name", ["admission.lock", "borrowers.lock"])
def test_installer_handoff_refuses_held_real_host_lock(installer_handoff_ready, name):
    installation, candidate, actions, _ = installer_handoff_ready
    with (installation.directory / name).open("r+") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        with pytest.raises(service.FixtureBlocked, match=r"admission|borrower lease"):
            service.handoff_installer(installation, candidate)
    assert actions == []


def test_installer_handoff_cli_executes_verified_fd_and_checks_exact_acknowledgement(
    recoverable, tmp_path, monkeypatch
):
    import storage_service

    installation, _, _ = recoverable
    source = tmp_path / "finite-metadata-cli"
    source.write_text(
        '#!/bin/sh\nprintf \'%s\\n\' \'{"schema_version":9,"schema":"' + "a" * 64 + "\"}'\n"
    )
    source.chmod(0o700)
    candidate = storage_service.prepare(installation.directory, source)
    invoke = subprocess.run
    calls = []

    def observed(args, **kwargs):
        (descriptor,) = kwargs["pass_fds"]
        assert args[0] == f"/proc/self/fd/{descriptor}"
        assert service.executable_identity(os.fstat(descriptor)) == service.executable_identity(
            Path(candidate["path"]).stat()
        )
        assert kwargs["env"]["LCTX_SURREAL_MAINTENANCE_TOKEN"] == "actual-token"
        calls.append(args[-1])
        return invoke(args, **kwargs)

    monkeypatch.setattr(service.subprocess, "run", observed)
    assert service._installer_handoff_cli(
        installation, candidate, "main", "schema", "actual-token"
    ) == {"schema_version": 9, "schema": "a" * 64}
    with pytest.raises(service.FixtureBlocked, match="acknowledgement differs"):
        service._installer_handoff_cli(installation, candidate, "main", "drain", "actual-token")
    assert calls == ["schema", "drain"]


@pytest.mark.parametrize("flag", ["--recover", "--stabilize-installer", "--server-generation"])
def test_installer_handoff_cli_is_mutually_exclusive(flag):
    args = ["maintenance", "--handoff-installer", "/current/lctx", flag]
    if flag == "--server-generation":
        args.append("/server/record.json")
    with pytest.raises(SystemExit) as error:
        service.main(args)
    assert error.value.code == 2


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
    result = subprocess.run(
        [sys.executable, "-c", code, str(Path(service.__file__).resolve())],
        cwd=service.ROOT,
        capture_output=True,
        text=True,
        check=False,
    )
    assert result.returncode == 0, result.stderr
    assert "canonical CLI/helper owner; transient readiness retried" in result.stdout


def test_owned_start_rechecks_same_generation_without_restarting_or_extending_deadline(
    recoverable, monkeypatch
):
    installation, _archive, _actions = recoverable
    original = json.loads(json.dumps(installation.record))
    calls = []
    checks = 0

    def check(current, **kwargs):
        nonlocal checks
        checks += 1
        assert current.record == original and kwargs == {"allow_maintenance": True}
        if checks < 3:
            raise service.FixtureBlocked("readiness", "same successor not ready yet")
        return {"outcome": "passed"}

    monkeypatch.setattr(service.Installation, "check", check)
    monkeypatch.setattr(
        service, "systemctl", lambda *args: calls.append(args) or SimpleNamespace(returncode=0)
    )
    sleeps = []
    monkeypatch.setattr(service.time, "sleep", lambda delay: sleeps.append(delay))
    OWNED_START(installation)
    assert calls == [("start", installation.unit)] and sleeps == [0.1, 0.1]
    assert checks == 3 and service.READY_TIMEOUT == 40.0
    refusal = service.FixtureBlocked("identity", "successor executable differs")
    monkeypatch.setattr(
        service.Installation, "check", lambda *a, **kw: (_ for _ in ()).throw(refusal)
    )
    times = iter([100.0, 139.0, 140.0])
    monkeypatch.setattr(service.time, "monotonic", lambda: next(times))
    with pytest.raises(service.FixtureBlocked) as caught:
        OWNED_START(installation)
    assert caught.value is refusal and sleeps == [0.1, 0.1, 0.1]
    assert calls == [("start", installation.unit), ("start", installation.unit)]


def assert_borrow_protected(path):
    with path.open("rb") as lock, pytest.raises(BlockingIOError):
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
        return [
            {
                "result": [
                    {
                        "generation": bytes(current.record["service_generation"]).hex(),
                        "schema_version": 3,
                    }
                ]
            }
        ]

    monkeypatch.setattr(service, "sql", sql)
    with service.borrow(installation) as checked:
        assert checked is not None
        first = checked.readiness
        for _ in range(3):
            loaded = service.Installation.load(
                {"LCTX_SURREAL_SERVICE_CONFIG": str(installation.directory / "installation.json")}
            )
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
        assert checked is not None
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
        raise service.FixtureBlocked(
            "schema", "exact schema mismatch", "just service maintenance --check"
        )

    monkeypatch.setattr(service.Installation, "check", refuse)
    with pytest.raises(service.FixtureBlocked) as error, service.borrow(installation):
        pytest.fail("failed check cannot yield a witness")
    assert error.value.repair == "just service maintenance --check"
    assert not service.lock_held(installation.directory / "borrowers.lock")
    assert not service.lock_held(installation.directory / "admission.lock")


@pytest.mark.parametrize("change", ["daemon", "executable", "owner"])
def test_checked_generation_rejects_process_executable_or_process_handoff(
    recoverable, monkeypatch, change
):
    installation, _archive, _actions = recoverable
    binary = Path(installation.record["binary"]["path"])
    actual_stat = Path.stat
    monkeypatch.setattr(
        Path,
        "stat",
        lambda path, *a, **kw: (
            actual_stat(binary, *a, **kw)
            if str(path) == "/proc/1234/exe"
            else actual_stat(path, *a, **kw)
        ),
    )
    alive = True
    installation._daemon_identity = (
        SimpleNamespace(pid=1234, alive=lambda: alive),
        service.executable_identity(binary.stat()),
    )
    with service.borrow(installation) as checked:
        assert checked is not None
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
    monkeypatch.setattr(
        service, "verify_binary", lambda env: Path(installation.record["binary"]["path"])
    )
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
    with (
        pytest.raises(service.FixtureBlocked, match="unit differs"),
        service.maintenance(installation, restart=True),
    ):
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
    with (
        pytest.raises(service.FixtureBlocked, match="start failed"),
        service.maintenance(installation, restart=True),
    ):
        pytest.fail("failed start was admitted")
    assert "open-admission" not in actions
    assert (installation.directory / "maintenance.json").exists()


def test_stop_accepts_failed_unit_only_when_no_main_process_or_descendants(monkeypatch, tmp_path):
    monkeypatch.setattr(service, "systemctl", lambda *a: subprocess.CompletedProcess(a, 0))
    state = {"ActiveState": "failed", "MainPID": "0", "ControlGroup": ""}
    monkeypatch.setattr(service, "unit_properties", lambda *a: state)
    service._stop_owned(service.Installation(tmp_path, {"unit": service.UNIT}))
    state["MainPID"] = "123"
    with pytest.raises(service.FixtureBlocked, match="stop is not confirmed"):
        service._stop_owned(service.Installation(tmp_path, {"unit": service.UNIT}))


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
    provenance = server.build_provenance(
        directory,
        selected,
        "rustc 1.99.0-nightly\ncommit-date: 2026-09-28\nhost: x86_64-unknown-linux-gnu",
        "cargo 1.99.0-nightly",
        cache_record_sha256="a" * 64,
    )
    executable = scratch / ("fake-reviewed-server" + suffix)
    executable.write_text(
        "#!/bin/sh\n# " + suffix + "\nprintf '" + selected["pin"]["cli_version"] + "\\n'\n"
    )
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
    state = {
        "drift": None,
        "markers": {
            db: {
                "generation": bytes(installation.record["service_generation"]).hex(),
                "schema": "a" * 64,
                "schema_version": 3,
                "admission_open": False,
            }
            for db in service.DATABASES
        },
    }

    def start(inst):
        actions.append("owned-server-start")
        if state["drift"]:
            field, value = state["drift"]
            state["markers"]["validation"][field] = value

    monkeypatch.setattr(service, "_owned_daemon_start", start)
    monkeypatch.setattr(
        service, "_stopped_attachment_commands", lambda inst: actions.append("attachments-drained")
    )
    monkeypatch.setattr(
        service,
        "systemctl",
        lambda *args: actions.append(args[0]) or subprocess.CompletedProcess(args, 0),
    )
    monkeypatch.setattr(
        service,
        "sql",
        lambda inst, statement, *, cfg: [
            {"status": "OK", "result": [dict(state["markers"][cfg["database"]])]}
        ],
    )
    return installation, generation, actions, state, None, b"", None


@pytest.mark.parametrize(
    "field,value", [("generation", "d" * 64), ("admission_open", True), ("schema", "d" * 64)]
)
def test_server_handoff_refuses_changed_scope_after_restart_and_preserves_journal(
    server_handoff_ready, field, value
):
    installation, generation, actions, state, _journal, _original_bytes, _ = server_handoff_ready
    state["drift"] = (field, value)
    with pytest.raises(service.FixtureBlocked, match="scope identity or closed admission"):
        service.handoff_server(installation, Path(generation["descriptor_path"]))
    marker = json.loads((installation.directory / "maintenance.json").read_text())
    assert marker["server_handoff"]["phase"] == "installed"
    assert "owned-server-start" in actions and "open-admission" not in actions
    plan = json.loads(Path(marker["server_handoff"]["journal"]).read_text())
    assert plan["scopes"]["validation"][field] != value


@pytest.mark.parametrize("lock", ["admission.lock", "borrowers.lock"])
def test_server_handoff_refuses_live_owner_and_held_locks_before_stop(server_handoff_ready, lock):
    installation, generation, actions, _state, _journal, _original_bytes, _ = server_handoff_ready
    held = service.hold_lock(installation.directory / lock)
    try:
        with pytest.raises(service.FixtureBlocked, match=r"admission|borrower lease"):
            service.handoff_server(installation, Path(generation["descriptor_path"]))
    finally:
        os.close(held)
    assert "stop" not in actions
    assert not (installation.directory / "maintenance.json").exists()
    marker = {
        "operation": service.SERVER_HANDOFF_OPERATION,
        "owner": service.ProcessIdentity.of().to_json(),
    }
    service.private_json(installation.directory / "maintenance.json", marker)
    with pytest.raises(service.FixtureBlocked, match="dead exact maintenance owner"):
        service.handoff_server(installation, Path(generation["descriptor_path"]))
    assert "stop" not in actions


@pytest.mark.parametrize("flag", ["--recover", "--restart", "--check"])
def test_server_handoff_cli_is_mutually_exclusive(flag):
    args = ["maintenance", "--server-generation", "/descriptor", flag]
    with pytest.raises(SystemExit):
        service.main(args)


def test_server_handoff_durably_links_plan_and_unit_before_pointer_start(
    server_handoff_ready, monkeypatch
):
    import storage_lifecycle

    installation, generation, _actions, _state, _journal, _original_bytes, _ = server_handoff_ready
    trace: list[tuple[str, Path | None]] = []
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
    monkeypatch.setattr(
        service, "_owned_daemon_start", lambda inst: trace.append(("start", None)) or start(inst)
    )
    service.handoff_server(installation, Path(generation["descriptor_path"]))
    plan = Path(installation.record["server_handoff"]["journal"])
    marker = trace.index(("durable", installation.directory / "maintenance.json"))
    plan_position = trace.index(("durable", plan))
    for path in (plan.parent, plan.parent.parent, installation.directory):
        assert ("fsync", path) in trace[plan_position:marker]
    assert (
        trace.index(("unit", service._unit_path()))
        < trace.index(("durable", installation.directory / "installation.json"))
        < trace.index(("start", None))
    )


def test_owned_patched_binary_is_descriptor_bound_without_weakening_official_pin(
    server_handoff_ready,
):
    installation, generation, _actions, _state, _journal, _original_bytes, _ = server_handoff_ready
    service.handoff_server(installation, Path(generation["descriptor_path"]))
    assert service._descriptor(installation.directory)["binary"] == service._generation_binary(
        generation
    )
    assert service._installed_binary(installation) == Path(generation["path"])
    with pytest.raises(service.FixtureBlocked, match="sha256"):
        verify_official_binary({"LCTX_SURREAL_BIN": generation["path"]}, runner=pytest.fail)
    installation.record["binary"]["sha256"] = "d" * 64
    with pytest.raises(service.FixtureBlocked, match="owned generation"):
        service._installed_binary(installation)


def test_check_daemon_checks_patched_actual_http_version_after_exact_executable(
    server_handoff_ready, monkeypatch
):
    installation, generation, _actions, _state, _journal, _original_bytes, _ = server_handoff_ready
    monkeypatch.setattr(service.Installation, "check_daemon", DAEMON_CHECK)
    installation.record["binary"] = service._generation_binary(generation)
    binary = Path(generation["path"])
    monkeypatch.setattr(service, "_installed_binary", lambda inst: binary)
    monkeypatch.setattr(
        service, "unit_properties", lambda *args: {"ActiveState": "active", "MainPID": "1234"}
    )
    resolve = Path.resolve
    read_bytes = Path.read_bytes

    def resolved(path, *args, **kw):
        return binary if str(path) == "/proc/1234/exe" else resolve(path, *args, **kw)

    def bytes_for(path):
        if str(path) == "/proc/1234/cmdline":
            return (
                "surreal\0--log=warn\0rocksdb://"
                + str(installation.directory / "data/store")
                + "\0"
                + installation.endpoint.removeprefix("http://")
                + "\0"
            ).encode()
        if str(path) == "/proc/1234/environ":
            return b"\0".join(
                (key + "=" + value).encode() for key, value in service.SERVER_SETTINGS.items()
            )
        return read_bytes(path)

    monkeypatch.setattr(Path, "resolve", resolved)
    monkeypatch.setattr(Path, "read_bytes", bytes_for)
    actual_stat = Path.stat
    monkeypatch.setattr(
        Path,
        "stat",
        lambda path, *a, **kw: (
            actual_stat(binary, *a, **kw)
            if str(path) == "/proc/1234/exe"
            else actual_stat(path, *a, **kw)
        ),
    )
    monkeypatch.setattr(
        service.ProcessIdentity,
        "of",
        lambda pid=None: SimpleNamespace(pid=1234, alive=lambda: True),
    )
    reported = service.VERSION
    monkeypatch.setattr(
        service,
        "local_urlopen",
        lambda *args, **kw: contextlib.nullcontext(
            SimpleNamespace(status=200, read=lambda: reported.encode())
        ),
    )
    with pytest.raises(service.FixtureBlocked, match="server version differs"):
        installation.check_daemon()
    reported = generation["http_version"]
    installation.check_daemon()


def test_standalone_server_handoff_reopens_only_after_both_unchanged_scopes(
    recoverable, tmp_path, monkeypatch
):
    installation, _, actions = recoverable
    generation = owned_server_descriptor(installation.directory, tmp_path)
    previous = Path(installation.record["binary"]["path"])
    service._unit_path().write_text(service.unit_text(installation.directory, previous, 29999))
    monkeypatch.setattr(service, "verify_binary", lambda *a, **kw: previous)
    monkeypatch.setattr(
        service.Installation, "check_daemon", lambda inst: actions.append("daemon-ready")
    )
    monkeypatch.setattr(
        service, "_owned_daemon_start", lambda inst: actions.append("owned-server-start")
    )
    monkeypatch.setattr(
        service, "_stopped_attachment_commands", lambda inst: actions.append("attachments-drained")
    )
    monkeypatch.setattr(
        service,
        "systemctl",
        lambda *args: actions.append(args[0]) or subprocess.CompletedProcess(args, 0),
    )
    admitted = True

    def installer(inst, exe, action):
        nonlocal admitted
        actions.append(action)
        if action in ("close-admission", "open-admission"):
            admitted = action == "open-admission"

    def scopes(inst, statement, *, cfg):
        assert statement.startswith("SELECT generation,schema,schema_version,admission_open")
        actions.append(("scope", cfg["database"], admitted))
        return [
            {
                "status": "OK",
                "result": [
                    {
                        "generation": bytes(inst.record["service_generation"]).hex(),
                        "schema": "a" * 64,
                        "schema_version": 3,
                        "admission_open": admitted,
                    }
                ],
            }
        ]

    monkeypatch.setattr(service, "_run_installer", installer)
    monkeypatch.setattr(service, "sql", scopes)
    result = service.handoff_server(installation, Path(generation["descriptor_path"]))
    assert result["admission_open"]
    assert (
        actions.index("close-admission")
        < actions.index("stop")
        < actions.index("owned-server-start")
    )
    assert actions.index(("scope", "main", False)) < actions.index("open-admission")
    assert actions.index(("scope", "validation", False)) < actions.index("open-admission")
    assert not (installation.directory / "maintenance.json").exists()


def test_patched_server_cold_archive_restores_without_source_or_build_checkout(
    recoverable, tmp_path
):
    installation, archive, _actions = recoverable
    generation = owned_server_descriptor(installation.directory, tmp_path)
    installation.record["binary"] = service._generation_binary(generation)
    service.private_json(installation.directory / "installation.json", installation.record)
    service._unit_path().write_text(
        service.unit_text(installation.directory, Path(generation["path"]), 29999)
    )
    assert not Path(generation["provenance"]["source_path"]).exists()
    assert not Path(generation["provenance"]["build_path"]).exists()
    assert not Path(generation["provenance"]["cache_lineage"]["path"]).exists()
    assert not Path(generation["provenance"]["cache_lineage"]["record"]).exists()
    assert not Path(generation["provenance"]["dependency_overrides"][0]["path"]).exists()
    assert not Path(generation["provenance"]["dependency_overrides"][0]["archive_path"]).exists()
    service.backup_service(installation, archive)
    with tarfile.open(archive, "r:") as contents:
        descriptor = json.load(archive_member(contents, "assets/server-generation.json"))
        assert descriptor == generation
        assert (
            archive_member(contents, "assets/surreal").read()
            == Path(generation["path"]).read_bytes()
        )
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
        payload = {
            item.name: archive_member(contents, item.name).read() for item in contents.getmembers()
        }
    modified = {**generation, "provenance": {**generation["provenance"], "patch_sha256": "d" * 64}}
    payload["assets/server-generation.json"] = json.dumps(modified).encode()
    metadata = json.loads(payload["recovery.json"])
    metadata["files"]["assets/server-generation.json"].update(
        sha256=service.hashlib.sha256(payload["assets/server-generation.json"]).hexdigest(),
        size=len(payload["assets/server-generation.json"]),
    )
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


def test_patched_server_status_names_owned_generation_and_retained_dependencies(
    server_handoff_ready, monkeypatch, capfd
):
    installation, generation, _actions, _state, _journal, _original_bytes, _ = server_handoff_ready
    service.handoff_server(installation, Path(generation["descriptor_path"]))
    monkeypatch.setenv(
        "LCTX_SURREAL_SERVICE_CONFIG", str(installation.directory / "installation.json")
    )
    monkeypatch.setattr(service, "unit_properties", lambda *args: {"ActiveState": "active"})
    assert service.main(["status"]) == 0
    result = json.loads(capfd.readouterr().out)
    assert result["server"]["generation"] == generation["identity"]
    assert result["server"]["version"] == generation["http_version"]
    assert generation["provenance"]["build_path"] in {
        row["path"] for row in result["storage"]["dependencies"]
    }


def retained_handoff_chain_after_native_rebuild(server_handoff_ready, tmp_path):
    installation, first, _actions, state, *_ = server_handoff_ready

    def rebuild_native_generation(value):
        installation.record["service_generation"] = [value] * 32
        for database in service.DATABASES:
            state["markers"][database]["generation"] = bytes(
                installation.record["service_generation"]
            ).hex()
            for installer in (False, True):
                path = installation.runtime_path(database, installer=installer)
                config = json.loads(path.read_text())
                config["service_generation"] = installation.record["service_generation"]
                service.private_json(path, config)
        service.private_json(installation.directory / "installation.json", installation.record)

    service.handoff_server(installation, Path(first["descriptor_path"]))
    first_reference = dict(installation.record["server_handoff"])
    first_path = Path(first_reference["journal"])
    first_bytes = first_path.read_bytes()
    rebuild_native_generation(82)
    second = owned_server_descriptor(installation.directory, tmp_path, "-second")
    service.handoff_server(installation, Path(second["descriptor_path"]))
    second_path = Path(installation.record["server_handoff"]["journal"])
    second_bytes = second_path.read_bytes()
    rebuild_native_generation(83)
    return installation, first, second, {first_path: first_bytes, second_path: second_bytes}


def test_retained_server_handoff_chain_survives_native_generation_rebuild(
    server_handoff_ready, tmp_path, monkeypatch, capfd
):
    from storage_service import dependencies

    installation, first, second, originals = retained_handoff_chain_after_native_rebuild(
        server_handoff_ready, tmp_path
    )
    descriptor = (installation.directory / "installation.json").read_bytes()
    paths = {item["path"] for item in dependencies(installation)["dependencies"]}
    assert {str(path.parent) for path in originals} <= paths
    assert {first["path"], second["path"]} <= paths
    monkeypatch.setenv(
        "LCTX_SURREAL_SERVICE_CONFIG", str(installation.directory / "installation.json")
    )
    monkeypatch.setattr(service, "unit_properties", lambda *args: {"ActiveState": "active"})
    assert service.main(["status"]) == 0
    result = json.loads(capfd.readouterr().out)
    assert result["server"]["generation"] == second["identity"]
    assert {item["path"] for item in result["storage"]["dependencies"]} == paths
    assert (installation.directory / "installation.json").read_bytes() == descriptor
    assert {path: path.read_bytes() for path in originals} == originals


@pytest.mark.parametrize("change", ["current_binary", "previous_binary", "installation", "digest"])
def test_retained_server_handoff_chain_refuses_physical_or_journal_substitution(
    server_handoff_ready, tmp_path, change
):
    from storage_service import dependencies

    installation, first, _second, _originals = retained_handoff_chain_after_native_rebuild(
        server_handoff_ready, tmp_path
    )
    reference = installation.record["server_handoff"]
    journal = Path(reference["journal"])
    plan = json.loads(journal.read_text())
    if change == "current_binary":
        installation.record["binary"] = service._generation_binary(first)
    elif change == "digest":
        reference["journal_sha256"] = "0" * 64
    else:
        if change == "installation":
            plan["installation_id"] = str(uuid.uuid4())
        else:
            predecessor = json.loads(Path(plan["previous_handoff"]["journal"]).read_text())
            plan["previous_binary"] = predecessor["previous_binary"]
        service.private_json(journal, plan)
        reference["journal_sha256"] = service.file_sha256(journal)
    with pytest.raises(service.FixtureBlocked, match="handoff"):
        dependencies(installation)


def test_active_server_handoff_resume_refuses_native_generation_rebuild(server_handoff_ready):
    from storage_service import dependencies

    installation, generation, actions, state, *_ = server_handoff_ready
    state["drift"] = ("schema", "d" * 64)
    with pytest.raises(service.FixtureBlocked, match="scope identity"):
        service.handoff_server(installation, Path(generation["descriptor_path"]))
    marker_path = installation.directory / "maintenance.json"
    marker = json.loads(marker_path.read_text())
    journal = Path(marker["server_handoff"]["journal"])
    original = journal.read_bytes()
    marker["owner"] = {**service.ProcessIdentity.of().to_json(), "pid": 99999997, "start_ticks": 1}
    service.private_json(marker_path, marker)
    installation.record["service_generation"] = [84] * 32
    before = list(actions)
    with pytest.raises(service.FixtureBlocked, match="checkpointed native generation"):
        service.handoff_server(installation, Path(generation["descriptor_path"]))
    with pytest.raises(service.FixtureBlocked, match="active server handoff native generation"):
        dependencies(installation)
    assert actions == before and journal.read_bytes() == original
    assert json.loads(marker_path.read_text()) == marker


def test_failed_install_metadata_can_retry_corrected_candidate_before_owned_directories(
    tmp_path, monkeypatch
):
    import storage_service

    root = tmp_path / "installation"
    candidate = tmp_path / "lctx"
    candidate.write_bytes(b"candidate")
    candidate.chmod(0o700)
    monkeypatch.setattr(service, "verify_binary", lambda env: candidate)
    monkeypatch.setattr(service, "systemd_available", lambda **kw: None)
    monkeypatch.setattr(service, "unit_properties", lambda *a: {"LoadState": "not-found"})
    monkeypatch.setattr(
        service, "_unit_path", lambda unit_name=service.UNIT: tmp_path / "service-unit"
    )
    monkeypatch.setattr(service, "_available_port", lambda: 29000)
    monkeypatch.setattr(
        storage_service,
        "prepare",
        lambda directory, source: {"path": str(source), "sha256": service.file_sha256(source)},
    )
    valid = False

    def metadata(*args, **kwargs):
        if not valid:
            raise service.FixtureBlocked("schema", "unsupported candidate metadata")
        return {"schema_version": 9, "schema": "b" * 64}

    monkeypatch.setattr(service, "_installer_cli", metadata)
    monkeypatch.setattr(
        service, "systemctl", lambda *a: subprocess.CompletedProcess(a, 1, "", "test stop")
    )
    env = {"LCTX_SURREAL_SERVICE_CONFIG": str(root / "installation.json")}
    with pytest.raises(service.FixtureBlocked, match="unsupported candidate metadata"):
        service.install(candidate, env=env)
    assert not (root / "installation.json").exists()
    assert all(not (root / name).exists() for name in ("data", "tmp", "attachments", "serving"))
    valid = True
    with pytest.raises(service.FixtureBlocked, match="daemon-reload failed"):
        service.install(candidate, env=env)
    assert json.loads((root / "installation.json").read_text())["native_schema_version"] == 9
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

        def log_message(self, format, *args):
            pass

    class Proxy(http.server.BaseHTTPRequestHandler):
        def do_POST(self):
            proxy_calls.append(self.path)
            self.send_response(502)
            self.send_header("Content-Length", "0")
            self.end_headers()

        do_GET = do_POST

        def log_message(self, format, *args):
            pass

    servers = [
        http.server.ThreadingHTTPServer(("127.0.0.1", 0), handler) for handler in (Target, Proxy)
    ]
    target_url, proxy_url = [f"http://127.0.0.1:{server.server_port}" for server in servers]
    threads = [
        threading.Thread(target=server.serve_forever, kwargs={"poll_interval": 0.01})
        for server in servers
    ]
    for thread in threads:
        thread.start()
    try:
        for name in list(os.environ):
            if name.lower().endswith("_proxy"):
                monkeypatch.delenv(name)
        monkeypatch.setenv("http_proxy", proxy_url)
        monkeypatch.setenv("NO_PROXY", "")
        cfg = {
            "username": "synthetic-user",
            "password": "synthetic-password",
            "database": "validation",
            "authentication": "database",
        }
        rows = service.sql(
            service.Installation(Path("."), {"endpoint": target_url}), "RETURN 1;", cfg=cfg
        )
        assert rows == [{"status": "OK", "result": 1}]
        request = urllib.request.Request(
            target_url + "/redirect",
            data=b"synthetic admin SQL",
            headers={"Authorization": "Basic synthetic-only"},
        )
        with pytest.raises(urllib.error.HTTPError) as refused:
            service.local_urlopen(request, timeout=2)
        assert refused.value.code == 302
        refused.value.close()
        assert [path for path, _auth in target_calls] == ["/sql", "/redirect"]
        assert target_calls[0][1] is not None
        assert target_calls[0][1].startswith("Basic ")
        assert proxy_calls == [], "neither proxy routing nor redirect may forward local requests"
    finally:
        for server in servers:
            server.shutdown()
            server.server_close()
        for thread in threads:
            thread.join()


@pytest.mark.parametrize(
    "url",
    [
        "https://127.0.0.1:8000/sql",
        "http://example.com:8000/sql",
        "http://user:secret@127.0.0.1:8000/sql",
        "http://127.0.0.1/sql",
    ],
)
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
        service.install(
            tmp_path / "lctx",
            env={
                "HOME": str(tmp_path),
                "LCTX_SURREAL_SERVICE_CONFIG": str(stable / "installation.json"),
            },
            campaign=True,
        )
    assert not stable.exists()


def test_campaign_descriptor_unit_is_derived_and_cannot_alias_stable(recoverable):
    installation, _archive, _actions = recoverable
    record = {**installation.record, "kind": "campaign"}
    record["unit"] = service.installation_unit(record)
    assert record["unit"] == f"library-context-surrealdb-campaign-{installation.id}.service"
    service.private_json(installation.directory / "installation.json", record)
    loaded = service.Installation.load(
        {"LCTX_SURREAL_SERVICE_CONFIG": str(installation.directory / "installation.json")}
    )
    assert loaded.unit != service.UNIT
    for invalid in (service.UNIT, "other.service", "../escape.service"):
        service.private_json(
            installation.directory / "installation.json", {**record, "unit": invalid}
        )
        with pytest.raises(service.FixtureBlocked, match="incompatible"):
            service.Installation.load(
                {"LCTX_SURREAL_SERVICE_CONFIG": str(installation.directory / "installation.json")}
            )


def test_campaign_start_and_recovery_assets_use_only_its_unit(recoverable, monkeypatch):
    installation, _archive, _actions = recoverable
    installation.record.update(kind="campaign")
    installation.record["unit"] = service.installation_unit(installation.record)
    calls = []
    monkeypatch.setattr(
        service,
        "systemctl",
        lambda *args, **kw: calls.append(args) or subprocess.CompletedProcess(args, 0),
    )
    monkeypatch.setattr(
        service,
        "unit_properties",
        lambda unit, *names: (
            calls.append(("show", unit)) or {"ActiveState": "inactive", "MainPID": "0"}
        ),
    )
    # The low-level start helper is independent of schema migration readiness.
    monkeypatch.setattr(service.Installation, "check_daemon", lambda self: None)
    service._owned_daemon_start(installation)
    assert calls == [("start", installation.unit)]
    campaign_unit = installation.directory.parent / "campaign-unit"
    campaign_unit.write_text("campaign")
    observed = []
    monkeypatch.setattr(
        service, "_unit_path", lambda unit=service.UNIT: observed.append(unit) or campaign_unit
    )
    assert service._recovery_sources(installation)["assets/service-unit"] == campaign_unit
    assert observed == [installation.unit]


def test_campaign_stop_confirms_only_its_owned_unit(tmp_path, monkeypatch):
    record = {"kind": "campaign", "installation_id": str(uuid.uuid4())}
    record["unit"] = service.installation_unit(record)
    installation = service.Installation(tmp_path, record)
    calls = []
    monkeypatch.setattr(
        service,
        "systemctl",
        lambda *args, **kw: calls.append(args) or subprocess.CompletedProcess(args, 0),
    )
    monkeypatch.setattr(
        service,
        "unit_properties",
        lambda unit, *names: (
            calls.append(("show", unit)) or {"ActiveState": "inactive", "MainPID": "0"}
        ),
    )
    service._stop_owned(installation)
    assert calls == [("stop", installation.unit), ("show", installation.unit)]


def test_backup_hashes_exact_final_archive_and_copied_assets_without_rereads(
    recoverable, monkeypatch
):
    import hashlib

    installation, archive, _actions = recoverable
    original_digest = service.file_sha256
    # Backup may verify identity through other owners, but cannot prehash tar assets
    # or reread its finished archive to compute the pointer.
    monkeypatch.setattr(
        service, "file_sha256", lambda path: pytest.fail(f"unneeded prehash/reread: {path}")
    )
    result = service.backup_service(installation, archive)
    data = archive.read_bytes()
    assert result["recovery"]["sha256"] == hashlib.sha256(data).hexdigest()
    with tarfile.open(archive, "r:") as tar:
        assert tar.getnames()[-1] == "recovery.json"
        metadata = json.load(archive_member(tar, "recovery.json"))
        for name, expected in metadata["files"].items():
            body = archive_member(tar, name).read()
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


@pytest.mark.parametrize(
    "fault", ["asset", "write", "finalize", "flush", "file-fsync", "directory-fsync"]
)
def test_backup_fault_never_publishes_prefix_digest_and_restarts_owned_service(
    recoverable, monkeypatch, fault
):
    installation, archive, actions = recoverable
    if fault == "asset":
        asset_read = service._AssetHashReader.read

        def short(self, count=-1):
            return asset_read(self, min(count, 1)) if count > 1 else asset_read(self, count)

        monkeypatch.setattr(service._AssetHashReader, "read", short)
    elif fault == "write":
        monkeypatch.setattr(
            service._ArchiveHashWriter,
            "write",
            lambda self, data: (_ for _ in ()).throw(OSError("write failure")),
        )
    elif fault == "finalize":
        original = tarfile.TarFile.close

        def close(self):
            original(self)
            if self.mode == "w":
                raise OSError("finalization failure")

        monkeypatch.setattr(tarfile.TarFile, "close", close)
    elif fault == "flush":
        monkeypatch.setattr(
            service._ArchiveHashWriter,
            "flush",
            lambda self: (_ for _ in ()).throw(OSError("flush failure")),
        )
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


@pytest.fixture
def lifecycle_campaign_ready(recoverable, monkeypatch, tmp_path):
    """Finite service-owner effects; this fixture starts no daemon."""
    from storage_lifecycle import Storage

    installation, _, actions = recoverable
    monkeypatch.setenv("LCTX_STORAGE_STATE", str(tmp_path / "lifecycle-storage"))
    installation.record.update(
        kind="campaign",
        purpose=service.LIFECYCLE_PURPOSE,
        capabilities=list(service.DEVELOPMENT_CAPABILITIES),
    )
    installation.record["unit"] = service.installation_unit(installation.record)
    from storage_service import prepare

    candidate = Path(installation.record["installer"])
    candidate.chmod(0o700)
    generation = prepare(installation.directory, candidate)
    installation.record.update(installer=generation["path"], installer_generation=generation)
    service._unit_path(installation.unit).write_text(
        service.unit_text(
            installation.directory,
            Path(installation.record["binary"]["path"]),
            29999,
            development=True,
        )
    )
    installation.record["campaign_storage"] = {
        "consumer": "lifecycle-campaign:" + installation.id,
        "content": "content-object",
        "server": "server-object",
    }
    service.private_json(installation.directory / "installation.json", installation.record)
    monkeypatch.setattr(
        service.Installation, "check_daemon", lambda inst: actions.append("daemon-check")
    )
    monkeypatch.setattr(
        service,
        "unit_properties",
        lambda *args: (
            {"ActiveState": "inactive", "MainPID": "0"}
            if any(action in ("stop", "confirmed-stop") for action in actions)
            else {"ActiveState": "active", "MainPID": "123"}
        ),
    )
    monkeypatch.setattr(
        service,
        "sql",
        lambda inst, sql, *, cfg: [
            {
                "result": {"namespaces": {service.NAMESPACE: "owned"}}
                if sql == "INFO FOR ROOT"
                else {"databases": {"validation": "owned", "random_beta_suite": "owned"}}
            }
        ],
    )
    monkeypatch.setattr(
        service,
        "systemctl",
        lambda *args, **kw: actions.append(tuple(args)) or subprocess.CompletedProcess(args, 0),
    )
    monkeypatch.setattr(
        Storage,
        "release",
        lambda self, object_id, consumer, reason: actions.append(
            ("storage-release", object_id, consumer)
        ),
    )

    def logical(inst, action, receipt):
        service._run_installer(inst, Path(inst.record["installer"]), action)
        return {"outcome": "passed", "local_drained": True, "interrupted": False}

    monkeypatch.setattr(service, "_run_disposal_installer", logical)
    return installation, actions


def disposal_probe_installer(installation, tmp_path, body):
    """A finite host-process control, with no native service or SDK execution."""
    from storage_service import prepare

    executable = tmp_path / "finite-disposal-installer"
    executable.write_text("#!" + sys.executable + "\n" + body)
    executable.chmod(0o700)
    generation = prepare(installation.directory, executable)
    installation.record.update(installer=generation["path"], installer_generation=generation)
    service.private_json(installation.directory / "installation.json", installation.record)


def test_campaign_disposal_logical_deadline_drains_children_then_discards_exact_executor(
    lifecycle_campaign_ready, tmp_path, monkeypatch
):
    import runs

    installation, actions = lifecycle_campaign_ready
    monkeypatch.setattr(service, "_run_disposal_installer", DISPOSAL_INSTALLER_RUN)
    monkeypatch.setattr(service, "DISPOSAL_LOGICAL_TIMEOUT", 1.0)
    # Drive only the service deadline after the real validation process has forked.
    # Host startup/registration time cannot turn the naturally exiting main scope
    # into the deadline witness intended for the blocked validation child group.
    import harness

    clock = [0.0]
    starts = {}
    observe = harness.observe_exit

    def observe_ready_child(child):
        code = observe(child)
        if code is None:
            starts.setdefault(child.pid, time.monotonic())
            if time.monotonic() - starts[child.pid] > 10:
                raise AssertionError("owned disposal process did not reach its ready marker")
            runtime = Path(child.args[child.args.index("--runtime-config") + 1])
            ready = runtime.parent / ("worker-ready-" + child.args[-1])
            if runtime.name.startswith("validation-") and ready.exists():
                clock[0] += 2.0
        return code

    monkeypatch.setattr(harness, "observe_exit", observe_ready_child)
    monkeypatch.setattr(
        service,
        "time",
        SimpleNamespace(
            monotonic=lambda: clock[0],
            sleep=time.sleep,
            time=time.time,
        ),
    )
    monkeypatch.setenv("LCTX_RUNS_ROOT", str(tmp_path / "logical-runs"))
    disposal_probe_installer(
        installation,
        tmp_path,
        """import os, signal, sys
from pathlib import Path
runtime = Path(sys.argv[sys.argv.index("--runtime-config") + 1])
assert os.environ["LCTX_SURREAL_SERVICE_CONFIG"] == str(runtime.parent / "installation.json")
if Path(sys.argv[sys.argv.index("--runtime-config") + 1]).name.startswith("main-"):
    sys.exit(0)
os.fork()
(runtime.parent / ("worker-ready-" + sys.argv[-1])).touch()
while True: signal.pause()
""",
    )
    receipt = service.dispose_campaign(installation, installation.id)
    assert receipt["outcome"] == "passed" and receipt["steps"]["executor-shutdown"] == "passed"
    assert receipt["steps"]["close-admission"] == "failed" and receipt["steps"]["drain"] == "failed"
    timed_out = [row for row in receipt["logical_commands"] if row.get("stopped") == "deadline"]
    assert len(timed_out) == 2
    for row in timed_out:
        record = runs.load_record(Path(row["run_path"]))
        assert record is not None and record["exit"]["signal"] in {signal.SIGTERM, signal.SIGKILL}
        assert row["outcome"] == "failed" and row["child_cleanup"]["status"] == "confirmed"
        assert not runs.cleanup_protected(record)
    assert "stop" in actions and not (installation.directory / "data").exists()


def test_campaign_disposal_unconfirmed_logical_cleanup_blocks_then_replays_same_failed_attempt(
    lifecycle_campaign_ready, tmp_path, monkeypatch
):
    import runs

    installation, actions = lifecycle_campaign_ready
    monkeypatch.setattr(service, "_run_disposal_installer", DISPOSAL_INSTALLER_RUN)
    monkeypatch.setenv("LCTX_RUNS_ROOT", str(tmp_path / "logical-runs"))
    disposal_probe_installer(installation, tmp_path, "raise SystemExit(0)\n")
    original_guard = runs.SpawnGuard

    class UnconfirmedGuard(original_guard):
        def __exit__(self, *args):
            super().__exit__(*args)
            # The real group is drained; this control withholds that evidence from
            # the consumer to establish fail-closed behavior without leaked workers.
            self.cleanup = {"status": "unknown"}

    monkeypatch.setattr(runs, "SpawnGuard", UnconfirmedGuard)
    with pytest.raises(service.FixtureBlocked, match="local cleanup remains unresolved"):
        service.dispose_campaign(installation, installation.id)
    assert "stop" not in actions and (installation.directory / "data").exists()
    partial = service.read_json(installation.directory / "disposal.json")
    assert isinstance(partial, dict)
    assert partial["steps"]["close-admission"] == "failed"
    assert len(partial["logical_commands"]) == 1
    first = partial["logical_commands"][0]["run_path"]
    monkeypatch.setattr(runs, "SpawnGuard", original_guard)
    receipt = service.dispose_campaign(installation, installation.id)
    assert receipt["steps"]["close-admission"] == "failed", (
        "recovery never promotes the logical failure"
    )
    assert receipt["steps"]["executor-shutdown"] == "passed"
    assert [
        row["run_path"] for row in receipt["logical_commands"] if row["action"] == "close-admission"
    ] == [first]
    assert receipt["logical_commands"][0]["child_cleanup"]["status"] == "confirmed"


def test_campaign_disposal_sigterm_drains_child_and_requires_explicit_resume(
    lifecycle_campaign_ready, tmp_path, monkeypatch
):
    import runs

    installation, actions = lifecycle_campaign_ready
    monkeypatch.setattr(service, "_run_disposal_installer", DISPOSAL_INSTALLER_RUN)
    monkeypatch.setenv("LCTX_RUNS_ROOT", str(tmp_path / "logical-runs"))
    disposal_probe_installer(installation, tmp_path, "import signal\nwhile True: signal.pause()\n")
    child = os.fork()
    if child == 0:
        try:
            with pytest.raises(service.FixtureBlocked, match="cancelled; explicit retry"):
                service.dispose_campaign(installation, installation.id)
            assert "stop" not in actions and (installation.directory / "data").exists()
        except BaseException:
            import traceback

            traceback.print_exc()
            os._exit(1)
        os._exit(0)
    deadline = time.monotonic() + 10.0
    reaped = False
    try:
        while True:
            receipt = service.read_json(installation.directory / "disposal.json")
            commands = (receipt or {}).get("logical_commands", [])
            record = runs.load_record(Path(commands[0]["run_path"])) if commands else None
            if record and record.get("child"):
                break
            if time.monotonic() >= deadline:
                pytest.fail("finite disposal child did not launch")
            time.sleep(0.02)
        os.kill(child, signal.SIGTERM)
        status = os.waitpid(child, 0)[1]
        reaped = True
        assert status == 0
    finally:
        if not reaped:
            with contextlib.suppress(ProcessLookupError):
                os.kill(child, signal.SIGKILL)
            os.waitpid(child, 0)
    receipt = service.read_json(installation.directory / "disposal.json")
    assert isinstance(receipt, dict) and receipt["steps"]["close-admission"] == "failed"
    (command,) = receipt["logical_commands"]
    assert command["stopped"] == "cancelled" and command["child_cleanup"]["status"] == "confirmed"
    assert "executor-shutdown" not in receipt["steps"]


def test_campaign_disposal_inactive_retry_preserves_recorded_logical_outcomes(
    lifecycle_campaign_ready, monkeypatch
):
    installation, _ = lifecycle_campaign_ready
    receipt = {
        "schema": 1,
        "kind": "lifecycle-campaign-disposal",
        "installation_id": installation.id,
        "generation": bytes(installation.record["service_generation"]).hex(),
        "workers": [],
        "outcome": "unknown",
        "steps": {"close-admission": "failed", "drain": "passed"},
    }
    service.private_json(installation.directory / "disposal.json", receipt)
    monkeypatch.setattr(
        service, "unit_properties", lambda *args: {"ActiveState": "inactive", "MainPID": "0"}
    )
    result = service.dispose_campaign(installation, installation.id)
    assert result["outcome"] == "passed"
    assert result["steps"]["close-admission"] == "failed" and result["steps"]["drain"] == "passed"


@pytest.mark.parametrize("cancellation", ["signal", "cancel_file"])
def test_campaign_disposal_honors_current_cancellation_when_child_already_exited(
    lifecycle_campaign_ready, tmp_path, monkeypatch, cancellation
):
    import harness
    import runs

    installation, actions = lifecycle_campaign_ready
    monkeypatch.setattr(service, "_run_disposal_installer", DISPOSAL_INSTALLER_RUN)
    monkeypatch.setenv("LCTX_RUNS_ROOT", str(tmp_path / "logical-runs"))
    disposal_probe_installer(installation, tmp_path, "raise SystemExit(0)\n")
    owners = []
    original_init, original_observe = runs.Owner.__init__, harness.observe_exit

    def capture_owner(self, *args, **kwargs):
        original_init(self, *args, **kwargs)
        owners.append(self)

    injected = False

    def observe_with_current_cancellation(child):
        nonlocal injected
        code = original_observe(child)
        if code == 0 and not injected:
            injected = True
            owner = owners[-1]
            if cancellation == "signal":
                owner.received = signal.SIGTERM
            else:
                service.private_json(owner.dir / runs.CANCEL, {"requested": runs.now()})
        return code

    monkeypatch.setattr(runs.Owner, "__init__", capture_owner)
    monkeypatch.setattr(harness, "observe_exit", observe_with_current_cancellation)
    with pytest.raises(service.FixtureBlocked, match="cancelled; explicit retry"):
        service.dispose_campaign(installation, installation.id)
    assert injected and "stop" not in actions and (installation.directory / "data").exists()
    receipt = service.read_json(installation.directory / "disposal.json")
    assert isinstance(receipt, dict) and receipt["steps"]["close-admission"] == "failed"
    (command,) = receipt["logical_commands"]
    record = runs.load_record(Path(command["run_path"]))
    assert record is not None and record["exit"] == {"code": 0}
    assert command["interrupted"] and command["child_cleanup"]["status"] == "confirmed"


@pytest.mark.parametrize("change", ["id", "purpose", "kind"])
def test_lifecycle_campaign_refuses_wrong_target_before_effect(
    lifecycle_campaign_ready, monkeypatch, change
):
    installation, actions = lifecycle_campaign_ready
    identity = installation.id
    if change == "id":
        identity = str(uuid.uuid4())
    else:
        installation.record[change] = "foreign"
    before = (installation.directory / "data/store/MANIFEST").read_bytes()
    with pytest.raises(service.FixtureBlocked, match="exact owned lifecycle campaign"):
        service.dispose_campaign(installation, identity)
    assert actions == []
    assert (installation.directory / "data/store/MANIFEST").read_bytes() == before


def test_lifecycle_campaign_stop_failure_preserves_then_retry_discards_exact_target(
    lifecycle_campaign_ready, monkeypatch
):
    installation, actions = lifecycle_campaign_ready

    def refuse_stop(inst):
        actions.append("refused-stop")
        raise service.FixtureBlocked("drainage", "controlled stop refusal")

    monkeypatch.setattr(service, "_stop_owned", refuse_stop)
    with pytest.raises(service.FixtureBlocked, match="stop refusal"):
        service.dispose_campaign(installation, installation.id)
    assert (installation.directory / "data/store/MANIFEST").read_text() == "cold-owned-data"
    assert (installation.directory / "maintenance.json").is_file()
    assert not any(
        isinstance(action, tuple) and action[0] == "storage-release" for action in actions
    )
    original = service.read_json(installation.directory / "disposal.json")
    assert isinstance(original, dict)
    assert original["outcome"] == "unknown"
    assert original["steps"].get("executor-shutdown") != "passed"
    monkeypatch.setattr(service, "_stop_owned", lambda inst: actions.append("confirmed-stop"))
    receipt = service.dispose_campaign(installation, installation.id)
    assert receipt["outcome"] == "passed"
    assert receipt["steps"]["executor-shutdown"] == "passed"
    assert receipt["steps"]["datastore-disposal"] == "passed"
    assert receipt["inventory"]["databases"][service.NAMESPACE] == [
        "random_beta_suite",
        "validation",
    ]
    assert not (installation.directory / "data").exists()
    assert not (installation.directory / "maintenance.json").exists()
    assert not (installation.directory / "server.env").exists()
    assert not Path(installation.record["installer"]).exists()
    assert not service._unit_path(installation.unit).exists()
    release = [
        action for action in actions if isinstance(action, tuple) and action[0] == "storage-release"
    ]
    assert len(release) == 2
    assert actions.index("confirmed-stop") < actions.index(release[0])
    current = service.Installation.load(
        {"LCTX_SURREAL_SERVICE_CONFIG": str(installation.directory / "installation.json")},
        ready=False,
    )
    assert current.record["phase"] == "disposed"
    count = len(actions)
    assert service.dispose_campaign(current, current.id) == receipt
    assert len(actions) == count, "completed disposal must not repeat effects"


def test_lifecycle_campaign_sql_observes_each_requested_namespace(monkeypatch, tmp_path):
    observed = []

    def respond(request, **kwargs):
        observed.append(request.get_header("Surreal-ns"))
        return io.BytesIO(b'[{"status":"OK","result":{"databases":{}}}]')

    monkeypatch.setattr(service, "local_urlopen", respond)
    installation = service.Installation(tmp_path, {"endpoint": "http://127.0.0.1:29999"})
    cfg = {
        "username": "root",
        "password": "private",
        "authentication": "root",
        "database": "validation",
    }
    for namespace in (service.NAMESPACE, "owned_beta_suite_namespace"):
        service.sql(installation, "INFO FOR NS", cfg={**cfg, "namespace": namespace})
    assert observed == [service.NAMESPACE, "owned_beta_suite_namespace"]


def test_lifecycle_campaign_inventory_retains_distinct_suite_databases(
    lifecycle_campaign_ready, monkeypatch
):
    installation, _ = lifecycle_campaign_ready
    inventory = {
        service.NAMESPACE: {"validation": "owned"},
        "owned_beta_suite_namespace": {"owned_beta_suite_database": "owned"},
    }

    def query(inst, statement, *, cfg):
        return [
            {
                "result": {"namespaces": dict.fromkeys(inventory, "owned")}
                if statement == "INFO FOR ROOT"
                else {"databases": inventory[cfg["namespace"]]}
            }
        ]

    monkeypatch.setattr(service, "sql", query)
    receipt = service.dispose_campaign(installation, installation.id)
    assert receipt["inventory"]["databases"] == {
        namespace: sorted(databases) for namespace, databases in inventory.items()
    }


def test_lifecycle_campaign_unknown_native_drain_does_not_replace_executor_discard(
    lifecycle_campaign_ready, monkeypatch
):
    installation, actions = lifecycle_campaign_ready

    def drain(inst, exe, action="init"):
        actions.append(action)
        if action == "drain":
            raise service.FixtureBlocked("native", "controlled unresolved native drain")

    monkeypatch.setattr(service, "_run_installer", drain)
    receipt = service.dispose_campaign(installation, installation.id)
    assert receipt["outcome"] == "passed"
    assert receipt["steps"]["executor-shutdown"] == "passed"
    assert receipt["steps"]["datastore-disposal"] == "passed"
    assert not (installation.directory / "data").exists()
    assert "drain" in actions and "stop" in actions


@pytest.mark.parametrize("changed_unit", [False, True])
def test_lifecycle_campaign_already_inactive_requires_exact_unit_before_discard(
    lifecycle_campaign_ready, monkeypatch, changed_unit
):
    installation, actions = lifecycle_campaign_ready
    installation.record["capabilities"] = list(service.DEVELOPMENT_CAPABILITIES)
    service.private_json(installation.directory / "installation.json", installation.record)
    binary = Path(installation.record["binary"]["path"])
    monkeypatch.setattr(service, "_installed_binary", lambda inst: binary)
    unit = service._unit_path(installation.unit)
    unit.write_text(
        service.unit_text(installation.directory, binary, 29999, development=True)
        + ("# external replacement\n" if changed_unit else "")
    )

    def inactive(inst):
        raise service.FixtureBlocked("readiness", "owned daemon is inactive")

    monkeypatch.setattr(service.Installation, "check_daemon", inactive)
    monkeypatch.setattr(
        service, "unit_properties", lambda *args: {"ActiveState": "inactive", "MainPID": "0"}
    )
    monkeypatch.setattr(
        service,
        "sql",
        lambda *args, **kwargs: pytest.fail("inactive executor cannot answer inventory"),
    )
    if changed_unit:
        with pytest.raises(service.FixtureBlocked, match="unit differs"):
            service.dispose_campaign(installation, installation.id)
        assert "stop" not in actions
        assert (installation.directory / "data/store/MANIFEST").is_file()
    else:
        receipt = service.dispose_campaign(installation, installation.id)
        assert receipt["steps"]["executor-shutdown"] == "passed"
        assert receipt["steps"]["drain"] == "not_run"
        assert receipt["inventory"]["status"] == "not_run"
        assert not (installation.directory / "data").exists()


@pytest.mark.parametrize("change", ["generation", "schema", "kind", "steps", "unit"])
def test_lifecycle_campaign_resume_checkpoint_cannot_authorize_another_executor(
    lifecycle_campaign_ready, change
):
    installation, actions = lifecycle_campaign_ready
    receipt: dict[str, Any] = {
        "schema": 1,
        "kind": "lifecycle-campaign-disposal",
        "installation_id": installation.id,
        "generation": bytes(installation.record["service_generation"]).hex(),
        "workers": [],
        "outcome": "unknown",
        "steps": {"executor-shutdown": "passed"},
    }
    if change == "unit":
        service._unit_path(installation.unit).write_text("an externally replaced unit")
    elif change == "steps":
        receipt["steps"] = {"executor-shutdown": {"invented": "passed"}}
    else:
        receipt[change] = "foreign"
    service.private_json(installation.directory / "disposal.json", receipt)
    with pytest.raises(service.FixtureBlocked, match=r"disposal receipt|unit differs"):
        service.dispose_campaign(installation, installation.id)
    assert "stop" not in actions
    assert (installation.directory / "data/store/MANIFEST").is_file()


def test_lifecycle_install_retains_purpose_consumer_before_failed_enable(tmp_path, monkeypatch):
    import storage_service
    from storage_lifecycle import Storage

    monkeypatch.setenv("LCTX_STORAGE_STATE", str(tmp_path / "storage"))
    root = tmp_path / "new-campaign"
    candidate = tmp_path / "lctx"
    candidate.write_bytes(b"finite installer input")
    candidate.chmod(0o700)
    monkeypatch.setattr(service, "verify_binary", lambda env: candidate)
    monkeypatch.setattr(service, "systemd_available", lambda **kw: None)
    monkeypatch.setattr(service, "unit_properties", lambda *a: {"LoadState": "not-found"})
    monkeypatch.setattr(service, "_unit_path", lambda unit_name=service.UNIT: tmp_path / "unit")
    monkeypatch.setattr(service, "_available_port", lambda: 29000)
    monkeypatch.setattr(
        storage_service,
        "prepare",
        lambda directory, source: {"path": str(source), "sha256": service.file_sha256(source)},
    )
    monkeypatch.setattr(
        service, "_installer_cli", lambda *a, **kw: {"schema_version": 9, "schema": "b" * 64}
    )
    enables = []

    def systemctl(*args):
        if args[0] == "enable":
            record = service.read_json(root / "installation.json")
            assert isinstance(record, dict)
            assert record["phase"] == "installing"
            assert record["purpose"] == service.LIFECYCLE_PURPOSE
            assert record["capabilities"] == list(service.DEVELOPMENT_CAPABILITIES)
            consumer = "lifecycle-campaign:" + record["installation_id"]
            assert record["campaign_storage"]["consumer"] == consumer
            storage = Storage().get(record["campaign_storage"]["content"])
            assert storage["owner"] == {"kind": "native", "path": str(root)}
            assert consumer in storage["obligations"], "storage admission must precede enable/start"
            enables.append(record["installation_id"])
            return subprocess.CompletedProcess(args, 1)
        return subprocess.CompletedProcess(args, 0)

    monkeypatch.setattr(service, "systemctl", systemctl)
    env = {"LCTX_SURREAL_SERVICE_CONFIG": str(root / "installation.json")}
    for _ in range(2):
        with pytest.raises(service.FixtureBlocked, match="owned service launch failed"):
            service.install(candidate, env=env, campaign=True, purpose=service.LIFECYCLE_PURPOSE)
        record = service.read_json(root / "installation.json")
        assert isinstance(record, dict)
        assert (root / "data").is_dir()
        assert record["phase"] == "installing"
        assert (
            record["campaign_storage"]["consumer"]
            == "lifecycle-campaign:" + record["installation_id"]
        )
    assert enables[0] == enables[1], "retry must retain exact target identity"


def test_borrowed_server_preserves_original_provenance_and_refuses_wrong_owner(
    tmp_path, monkeypatch, capfd
):
    from test_surrealdb_server import provenance

    import surrealdb_server
    from storage_service import dependencies

    origin, campaign = tmp_path / "original-service", tmp_path / "campaign"
    # This is a finite provenance-correspondence control, not evidence of a server build.
    generation = {
        "path": str(origin / "server"),
        "sha256": "a" * 64,
        "http_version": service.VERSION,
        "descriptor_path": str(origin / "record.json"),
        "provenance": provenance(origin),
    }
    monkeypatch.setattr(surrealdb_server, "validate_descriptor", lambda path: generation)
    assert (
        service._owned_server_generation(origin, Path(generation["descriptor_path"])) is generation
    )
    with pytest.raises(service.FixtureBlocked, match="provenance"):
        service._owned_server_generation(campaign, Path(generation["descriptor_path"]))
    record = {
        "kind": "campaign",
        "purpose": service.LIFECYCLE_PURPOSE,
        "installation_id": str(uuid.uuid4()),
        "phase": "ready",
        "installer": str(campaign / "lctx"),
        "binary_origin": str(origin),
        "binary": service._generation_binary(generation),
    }
    record["unit"] = service.installation_unit(record)
    installation = service.Installation(campaign, record)
    original = json.loads(json.dumps(generation))
    assert service._installed_binary(installation) == Path(generation["path"])
    expected = surrealdb_server.dependencies(generation)
    observed = dependencies(installation)["dependencies"]
    assert all(item in observed for item in expected), (
        "borrowing retains original executable and warm dependencies"
    )
    monkeypatch.setattr(service.Installation, "load", lambda **kwargs: installation)
    monkeypatch.setattr(service, "unit_properties", lambda *args: {"ActiveState": "active"})
    monkeypatch.setattr(service, "borrowers", lambda inst: [])
    monkeypatch.setattr(service, "development_workers", lambda inst: [])
    assert service.main(["status"]) == 0
    assert json.loads(capfd.readouterr().out)["storage"]["dependencies"] == observed
    assert generation == original, (
        "borrowing must not rewrite original provenance as a campaign build"
    )
    installation.record["binary_origin"] = str(campaign)
    with pytest.raises(service.FixtureBlocked, match="provenance"):
        service._installed_binary(installation)
    with pytest.raises(service.StorageBlocked, match="provenance"):
        dependencies(installation)
    installation.record.update(kind="stable", binary_origin=str(origin))
    with pytest.raises(service.FixtureBlocked, match="not a lifecycle campaign"):
        service._installed_binary(installation)
    with pytest.raises(service.FixtureBlocked, match="not a lifecycle campaign"):
        dependencies(installation)


def dead_campaign_owner():
    return {**service.ProcessIdentity.of().to_json(), "pid": 99999997, "start_ticks": 1}


def ordinary_campaign_marker(installation):
    marker: dict[str, Any] = {
        "operation": "maintenance",
        "owner": dead_campaign_owner(),
        "token": "prior-private-owner",
    }
    service.private_json(installation.directory / "maintenance.json", marker)
    return marker


def test_campaign_disposal_transfers_dead_ordinary_marker_using_current_quiescence(
    lifecycle_campaign_ready,
):
    import base64
    import hashlib

    installation, actions = lifecycle_campaign_ready
    marker = ordinary_campaign_marker(installation)
    original = (installation.directory / "maintenance.json").read_bytes()
    receipt = service.dispose_campaign(installation, installation.id)
    transfer = receipt["transferred_maintenance"]
    assert transfer["predecessor"] == {
        "record": marker,
        "bytes": base64.b64encode(original).decode(),
        "sha256": hashlib.sha256(original).hexdigest(),
    }
    assert transfer["current_quiescence"]["host"]["participants"] == []
    assert transfer["current_quiescence"]["executor_active"] is True
    assert receipt["outcome"] == "passed" and "stop" in actions
    assert "ancestry" not in transfer


@pytest.mark.parametrize(
    "change",
    ["live", "foreign", "specialized", "recovery", "extra", "empty_token", "public", "symlink"],
)
def test_campaign_disposal_refuses_ineligible_predecessor_before_transfer(
    lifecycle_campaign_ready, change
):
    installation, actions = lifecycle_campaign_ready
    marker = ordinary_campaign_marker(installation)
    path = installation.directory / "maintenance.json"
    if change == "live":
        marker["owner"] = service.ProcessIdentity.of().to_json()
    elif change == "foreign":
        marker["owner"]["pid_namespace"] += 1
    elif change == "specialized":
        marker["operation"] = "native-host-recovery"
    elif change == "recovery":
        marker["recovery"] = {"kind": "restore"}
    elif change == "extra":
        marker["extra"] = True
    elif change == "empty_token":
        marker["token"] = ""
    service.private_json(path, marker)
    if change == "public":
        path.chmod(0o644)
    elif change == "symlink":
        retained = path.with_name("retained-maintenance.json")
        path.rename(retained)
        path.symlink_to(retained)
    original = path.read_bytes()
    with pytest.raises(service.FixtureBlocked, match="maintenance"):
        service.dispose_campaign(installation, installation.id)
    assert path.read_bytes() == original and "stop" not in actions
    assert not (installation.directory / "disposal.json").exists()


def test_campaign_disposal_journals_before_transfer_and_resumes_same_predecessor(
    lifecycle_campaign_ready, monkeypatch
):
    installation, actions = lifecycle_campaign_ready
    marker = ordinary_campaign_marker(installation)
    write = service.private_json
    injected = False

    def interrupt_transfer(path, value):
        nonlocal injected
        if (
            path.name == "maintenance.json"
            and value.get("operation") == "dispose-campaign"
            and not injected
        ):
            injected = True
            journal = service.read_json(installation.directory / "disposal.json")
            assert journal is not None
            assert journal["transferred_maintenance"]["predecessor"]["record"] == marker
            assert journal["current_quiescence"]["executor_active"] is True
            raise InterruptedError("before atomic maintenance transfer")
        write(path, value)

    monkeypatch.setattr(service, "private_json", interrupt_transfer)
    with pytest.raises(InterruptedError):
        service.dispose_campaign(installation, installation.id)
    assert service.read_json(installation.directory / "maintenance.json") == marker
    assert "stop" not in actions
    receipt = service.dispose_campaign(installation, installation.id)
    assert receipt["outcome"] == "passed"
    assert receipt["transferred_maintenance"]["predecessor"]["record"] == marker


@pytest.mark.parametrize("live_axis", ["command", "owner"])
def test_campaign_disposal_reobserves_released_attachment_instead_of_cached_cleanup(
    lifecycle_campaign_ready, monkeypatch, live_axis
):
    import harness

    installation, actions = lifecycle_campaign_ready
    marker = ordinary_campaign_marker(installation)
    directory = installation.directory / "attachments" / str(uuid.uuid4())
    directory.mkdir(parents=True)
    child = harness.spawn_group([sys.executable, "-c", "import signal; signal.pause()"])
    with harness.SpawnGuard(child, grace=0.1):
        row = {
            "id": directory.name,
            "fixture": installation.id,
            "released": True,
            "owner": service.ProcessIdentity.of().to_json()
            if live_axis == "owner"
            else dead_campaign_owner(),
            "command": {
                "leader": {**service.ProcessIdentity.of(child.pid).to_json(), "pgid": child.pid}
            }
            if live_axis == "command"
            else None,
            "command_cleanup": {"status": "confirmed", "survivors": []},
        }
        service.private_json(directory / "record.json", row)
        monkeypatch.setattr(service, "borrowers", lambda inst: [])
        with pytest.raises(service.FixtureBlocked, match="current owner/group"):
            service.dispose_campaign(installation, installation.id)
        assert service.read_json(installation.directory / "maintenance.json") == marker
        assert "stop" not in actions


def test_campaign_disposal_ignores_unrelated_maintenance_run(
    lifecycle_campaign_ready, monkeypatch, tmp_path
):
    import runs

    installation, _ = lifecycle_campaign_ready
    ordinary_campaign_marker(installation)
    path = tmp_path / "unrelated-managed-run"
    path.mkdir()
    service.private_json(
        path / runs.RECORD,
        {
            "owner": service.ProcessIdentity.of().to_json(),
            "child": None,
            "argv": ["just", "service", "maintenance", "--check"],
        },
    )
    monkeypatch.setattr(runs, "all_runs", lambda: [path])
    assert service.dispose_campaign(installation, installation.id)["outcome"] == "passed"


def test_campaign_disposal_reobserves_exact_run_group_despite_confirmed_cleanup(
    lifecycle_campaign_ready, monkeypatch, tmp_path
):
    import harness
    import runs

    installation, actions = lifecycle_campaign_ready
    marker = ordinary_campaign_marker(installation)
    path = tmp_path / "exact-managed-run"
    path.mkdir()
    child = harness.spawn_group([sys.executable, "-c", "import signal; signal.pause()"])
    with harness.SpawnGuard(child, grace=0.1):
        service.private_json(
            path / runs.RECORD,
            {
                "owner": dead_campaign_owner(),
                "child": {**service.ProcessIdentity.of(child.pid).to_json(), "pgid": child.pid},
                "cleanup": {"status": "confirmed", "survivors": []},
                "argv": [
                    "/usr/bin/env",
                    "LCTX_SURREAL_SERVICE_CONFIG="
                    + str(installation.directory / "installation.json"),
                    "just",
                    "service",
                    "maintenance",
                ],
            },
        )
        monkeypatch.setattr(runs, "all_runs", lambda: [path])
        with pytest.raises(service.FixtureBlocked, match="current owner/group"):
            service.dispose_campaign(installation, installation.id)
        assert service.read_json(installation.directory / "maintenance.json") == marker
        assert "stop" not in actions


def test_campaign_host_selector_observes_actual_current_client(lifecycle_campaign_ready):
    import harness

    installation, _ = lifecycle_campaign_ready
    child = harness.spawn_group(
        [sys.executable, "-c", "import signal; signal.pause()"],
        env={
            **os.environ,
            "LCTX_SURREAL_SERVICE_CONFIG": str(installation.directory / "installation.json"),
        },
    )
    with (
        harness.SpawnGuard(child, grace=0.1),
        pytest.raises(service.FixtureBlocked, match="campaign-bound host participant"),
    ):
        service._campaign_host_participants(installation)


def test_ordinary_campaign_installer_enrolls_target_before_managed_spawn(
    lifecycle_campaign_ready, tmp_path, monkeypatch
):
    import runs

    installation, _ = lifecycle_campaign_ready
    monkeypatch.setenv("LCTX_RUNS_ROOT", str(tmp_path / "maintenance-runs"))
    ordinary_campaign_marker(installation)
    execute = runs.Owner.run
    observed = []

    def checked_spawn(owner):
        (path,) = (installation.directory / "maintenance-commands").glob("*.json")
        reference = service.read_json(path)
        assert reference is not None
        assert reference["installation_id"] == installation.id
        assert reference["generation"] == bytes(installation.record["service_generation"]).hex()
        assert reference["run_path"] == str(owner.dir)
        assert reference["owner"] == owner.record["owner"]
        assert reference["argv"] == owner.argv
        assert required_record(runs.load_record(owner.dir))["child"] is None
        observed.append(reference)
        return execute(owner)

    monkeypatch.setattr(runs.Owner, "run", checked_spawn)
    assert (
        service._run_maintenance_installer(
            installation, [sys.executable, "-c", "raise SystemExit(0)"], "check", "validation"
        )
        == 0
    )
    assert len(observed) == 1
    assert list((installation.directory / "maintenance-commands").glob("*.json")) == []
    record = runs.load_record(Path(observed[0]["run_path"]))
    assert record is not None
    assert record["cleanup"]["status"] == "confirmed" and runs.group_of(record)["status"] == "gone"


def completed_disposal_participant(tmp_path, monkeypatch):
    import runs

    monkeypatch.setenv("LCTX_RUNS_ROOT", str(tmp_path / "participant-runs"))
    path = runs.new_run_dir()
    owner = runs.Owner(
        path,
        [sys.executable, "-c", "raise SystemExit(0)"],
        label="additional-quiescence-only",
        cwd=str(service.ROOT),
        mode="foreground",
        stream=False,
    )
    assert owner.run() == 0
    return path


def test_campaign_disposal_additional_run_is_quiescence_obligation_without_ancestry_claim(
    lifecycle_campaign_ready, tmp_path, monkeypatch
):
    installation, actions = lifecycle_campaign_ready
    ordinary_campaign_marker(installation)
    path = completed_disposal_participant(tmp_path, monkeypatch)
    receipt = service.dispose_campaign(installation, installation.id, participant_runs=[path.name])
    (reference,) = receipt["participant_runs"]
    assert reference["source"] == "operator-supplied-additional-quiescence-obligation"
    assert reference["run_path"] == str(path)
    assert "argv" not in reference and "ancestry" not in reference
    assert receipt["current_quiescence"]["runs"][0]["group"]["status"] == "gone"
    assert receipt["outcome"] == "passed"
    assert (
        "storage-release",
        reference["storage_receipt"],
        f"lifecycle-disposal-participant:{installation.id}:{path.name}",
    ) in actions
    completed = service.Installation(
        installation.directory,
        required_record(service.read_json(installation.directory / "installation.json")),
    )
    monkeypatch.setattr(
        service,
        "_campaign_participant_run",
        lambda path: pytest.fail("terminal disposal must not require retired run contents"),
    )
    assert service.dispose_campaign(completed, installation.id) == receipt


@pytest.mark.parametrize("axis", ["owner", "group", "foreign", "malformed"])
def test_campaign_disposal_additional_run_refuses_unresolved_current_identity(
    lifecycle_campaign_ready, tmp_path, monkeypatch, axis
):
    import harness
    import runs

    installation, actions = lifecycle_campaign_ready
    marker = ordinary_campaign_marker(installation)
    path = completed_disposal_participant(tmp_path, monkeypatch)
    record = runs.load_record(path)
    assert record is not None
    with contextlib.ExitStack() as stack:
        if axis == "owner":
            lock = harness.hold_lock(path / runs.OWNER_LOCK)
            stack.callback(os.close, lock)
        elif axis == "group":
            child = harness.spawn_group([sys.executable, "-c", "import signal; signal.pause()"])
            stack.enter_context(harness.SpawnGuard(child, grace=0.1))
            record["child"] = {**service.ProcessIdentity.of(child.pid).to_json(), "pgid": child.pid}
        elif axis == "foreign":
            record["owner"]["pid_namespace"] += 1
        else:
            del record["owner"]["start_ticks"]
        service.private_json(path / runs.RECORD, record)
        with pytest.raises(service.FixtureBlocked):
            service.dispose_campaign(installation, installation.id, participant_runs=[path.name])
        assert service.read_json(installation.directory / "maintenance.json") == marker
        assert "stop" not in actions


def test_campaign_disposal_additional_run_retry_keeps_reference_and_refuses_replacement(
    lifecycle_campaign_ready, tmp_path, monkeypatch
):
    import runs

    installation, actions = lifecycle_campaign_ready
    ordinary_campaign_marker(installation)
    path = completed_disposal_participant(tmp_path, monkeypatch)

    def refuse_stop(inst):
        raise service.FixtureBlocked("drainage", "controlled stop refusal")

    monkeypatch.setattr(service, "_stop_owned", refuse_stop)
    with pytest.raises(service.FixtureBlocked, match="stop refusal"):
        service.dispose_campaign(installation, installation.id, participant_runs=[path.name])
    receipt = service.read_json(installation.directory / "disposal.json")
    assert receipt is not None
    marker = service.read_json(installation.directory / "maintenance.json")
    assert marker is not None
    assert receipt["maintenance_successor"] == marker
    from storage_lifecycle import Storage

    (reference,) = receipt["participant_runs"]
    consumer = f"lifecycle-disposal-participant:{installation.id}:{path.name}"
    assert (
        Storage().get(reference["storage_receipt"])["obligations"][consumer]["released_at"] is None
    )
    record = runs.load_record(path)
    assert record is not None
    record["argv"] = [sys.executable, "-c", "different-command"]
    service.private_json(path / runs.RECORD, record)
    with pytest.raises(service.FixtureBlocked, match="enrollment differs"):
        service.dispose_campaign(installation, installation.id)
    assert service.read_json(installation.directory / "maintenance.json") == marker
    assert "stop" not in actions


def test_campaign_disposal_retry_rejects_unjournaled_successor(
    lifecycle_campaign_ready, monkeypatch
):
    installation, actions = lifecycle_campaign_ready
    ordinary_campaign_marker(installation)
    monkeypatch.setattr(
        service,
        "_stop_owned",
        lambda inst: (_ for _ in ()).throw(
            service.FixtureBlocked("drainage", "controlled stop refusal")
        ),
    )
    with pytest.raises(service.FixtureBlocked, match="stop refusal"):
        service.dispose_campaign(installation, installation.id)
    marker = service.read_json(installation.directory / "maintenance.json")
    assert marker is not None
    marker["token"] = "replacement-private-marker"
    service.private_json(installation.directory / "maintenance.json", marker)
    with pytest.raises(service.FixtureBlocked, match="maintenance owner"):
        service.dispose_campaign(installation, installation.id)
    assert "stop" not in actions


def test_campaign_disposal_retry_reuses_journaled_successor_across_marker_write_crash(
    lifecycle_campaign_ready, monkeypatch
):
    installation, actions = lifecycle_campaign_ready
    ordinary_campaign_marker(installation)
    stop = service._stop_owned
    monkeypatch.setattr(
        service,
        "_stop_owned",
        lambda inst: (_ for _ in ()).throw(
            service.FixtureBlocked("drainage", "controlled stop refusal")
        ),
    )
    with pytest.raises(service.FixtureBlocked, match="stop refusal"):
        service.dispose_campaign(installation, installation.id)
    marker = service.read_json(installation.directory / "maintenance.json")
    assert marker is not None
    write = service.private_json
    injected = False

    def interrupt_retry(path, value):
        nonlocal injected
        if path.name == "maintenance.json" and not injected:
            injected = True
            journal = service.read_json(installation.directory / "disposal.json")
            assert journal is not None
            assert journal["maintenance_successor"] == marker == value
            raise InterruptedError("retry marker publication interrupted")
        write(path, value)

    monkeypatch.setattr(service, "private_json", interrupt_retry)
    with pytest.raises(InterruptedError):
        service.dispose_campaign(installation, installation.id)
    assert service.read_json(installation.directory / "maintenance.json") == marker
    monkeypatch.setattr(service, "_stop_owned", stop)
    assert service.dispose_campaign(installation, installation.id)["outcome"] == "passed"
    assert "stop" in actions


def test_campaign_disposal_releases_interrupted_maintenance_reference_without_rewriting_unknown(
    lifecycle_campaign_ready, tmp_path, monkeypatch
):
    import runs

    installation, actions = lifecycle_campaign_ready
    ordinary_campaign_marker(installation)
    monkeypatch.setenv("LCTX_RUNS_ROOT", str(tmp_path / "interrupted-maintenance-runs"))
    guard_type = runs.SpawnGuard

    class UnknownGuard(guard_type):
        def __exit__(self, *args):
            super().__exit__(*args)
            self.cleanup = {"status": "unknown", "reason": "withheld original evidence"}

    monkeypatch.setattr(runs, "SpawnGuard", UnknownGuard)
    service._run_maintenance_installer(
        installation, [sys.executable, "-c", "raise SystemExit(1)"], "check", "validation"
    )
    (reference_path,) = (installation.directory / "maintenance-commands").glob("*.json")
    reference = service.read_json(reference_path)
    assert reference is not None
    run_path = Path(reference["run_path"])
    assert required_record(runs.load_record(run_path))["cleanup"]["status"] == "unknown"
    assert runs.group_of(required_record(runs.load_record(run_path)))["status"] == "gone"
    monkeypatch.setattr(runs, "SpawnGuard", guard_type)
    assert service.dispose_campaign(installation, installation.id)["outcome"] == "passed"
    assert not reference_path.exists()
    assert required_record(runs.load_record(run_path))["cleanup"]["status"] == "unknown"
    assert (
        "storage-release",
        reference["storage"]["receipt"],
        reference["storage"]["consumer"],
    ) in actions


def test_campaign_host_selector_refuses_inaccessible_supported_python_adapter(
    lifecycle_campaign_ready, tmp_path, monkeypatch
):
    installation, _ = lifecycle_campaign_ready
    process = tmp_path / "99999996"
    process.mkdir()
    (process / "cmdline").write_bytes(os.fsencode(sys.executable) + b"\0-m\0pytest\0")
    read = Path.read_bytes
    iterate = Path.iterdir

    def inaccessible(path):
        if path == process / "environ":
            raise PermissionError("supported runtime environment inaccessible")
        return read(path)

    monkeypatch.setattr(Path, "read_bytes", inaccessible)
    monkeypatch.setattr(
        Path, "iterdir", lambda path: iter([process]) if path == Path("/proc") else iterate(path)
    )
    with pytest.raises(
        service.FixtureBlocked, match="supported host participant selector coverage is inaccessible"
    ):
        service._campaign_host_participants(installation)


def test_campaign_host_selector_recognizes_legacy_env_assignment(
    lifecycle_campaign_ready, tmp_path, monkeypatch
):
    installation, _ = lifecycle_campaign_ready
    process = tmp_path / str(os.getpid() + 123)
    process.mkdir()
    (process / "cmdline").write_bytes(
        b"/usr/bin/env\0LCTX_SURREAL_SERVICE_CONFIG="
        + os.fsencode(installation.directory / "installation.json")
        + b"\0native-client\0"
    )
    (process / "environ").write_bytes(b"")
    iterate = Path.iterdir
    identity = service.ProcessIdentity.of()
    monkeypatch.setattr(
        Path, "iterdir", lambda path: iter([process]) if path == Path("/proc") else iterate(path)
    )
    monkeypatch.setattr(service.ProcessIdentity, "of", classmethod(lambda cls, pid=None: identity))
    with pytest.raises(service.FixtureBlocked, match="campaign-bound host participant"):
        service._campaign_host_participants(installation)


def test_campaign_disposal_recognizes_only_its_actual_managed_dispatch_group(
    lifecycle_campaign_ready, tmp_path, monkeypatch
):
    import runs

    installation, _ = lifecycle_campaign_ready
    monkeypatch.setenv("LCTX_RUNS_ROOT", str(tmp_path / "dispatcher-runs"))
    script = tmp_path / "surrealdb_service.py"
    script.write_text(f"""import os, sys, time
from pathlib import Path
sys.path.insert(0, {str(service.ROOT / "scripts")!r})
import runs, surrealdb_service as service
path = Path(os.environ["LCTX_RUN_DIR"])
for _ in range(100):
    record = runs.load_record(path)
    if record and record.get("child") and "start_ticks" in record["child"]: break
    time.sleep(0.01)
installation = service.Installation(
    Path({str(installation.directory)!r}),
    {{"installation_id": {installation.id!r}}},
)
assert service._campaign_disposal_dispatch_run(installation, path, record)
record["argv"] = ["bash", "-c", "just", "service", "dispose-campaign", installation.id]
assert not service._campaign_disposal_dispatch_run(installation, path, record)
""")
    path = runs.new_run_dir()
    owner = runs.Owner(
        path,
        [sys.executable, str(script), "dispose-campaign", installation.id],
        label="finite-disposal-dispatch-control",
        cwd=str(service.ROOT),
        mode="foreground",
        stream=False,
    )
    assert owner.run() == 0
    assert runs.group_of(required_record(runs.load_record(path)))["status"] == "gone"


@pytest.mark.parametrize("changed", [False, True])
def test_campaign_host_selector_distinguishes_authenticated_executor(
    lifecycle_campaign_ready, changed
):
    import harness

    installation, _ = lifecycle_campaign_ready
    child = harness.spawn_group(
        [sys.executable, "-c", "import signal; signal.pause()"],
        env={
            **os.environ,
            "LCTX_SURREAL_SERVICE_CONFIG": str(installation.directory / "installation.json"),
        },
    )
    with harness.SpawnGuard(child, grace=0.1):
        identity = service.ProcessIdentity.of(child.pid)
        executable = service.executable_identity(Path(f"/proc/{child.pid}/exe").stat())
        installation._daemon_identity = (identity, () if changed else executable)
        if changed:
            with pytest.raises(service.FixtureBlocked, match="executor changed"):
                service._campaign_host_participants(installation)
        else:
            assert service._campaign_host_participants(installation)["participants"] == []


@pytest.mark.parametrize(
    "field,value",
    [
        ("pid", -1),
        ("start_ticks", True),
        ("pid_namespace", "invalid"),
        ("boot_id", "not-a-boot-id"),
    ],
)
def test_campaign_disposal_refuses_malformed_dead_predecessor_identity(
    lifecycle_campaign_ready, field, value
):
    installation, actions = lifecycle_campaign_ready
    marker = ordinary_campaign_marker(installation)
    marker["owner"][field] = value
    service.private_json(installation.directory / "maintenance.json", marker)
    with pytest.raises(service.FixtureBlocked, match="maintenance owner"):
        service.dispose_campaign(installation, installation.id)
    assert "stop" not in actions


def test_ordinary_campaign_installer_routes_both_scopes_through_managed_owner(
    lifecycle_campaign_ready, tmp_path, monkeypatch
):
    import harness
    import runs

    installation, _ = lifecycle_campaign_ready
    monkeypatch.setenv("LCTX_RUNS_ROOT", str(tmp_path / "installer-runs"))
    service.private_json(
        installation.directory / "maintenance.json",
        {
            "operation": "maintenance",
            "owner": service.ProcessIdentity.of().to_json(),
            "token": "current",
        },
    )
    installer = tmp_path / "finite-installer"
    installer.write_text("#!" + sys.executable + "\nraise SystemExit(0)\n")
    installer.chmod(0o700)
    gate = harness.hold_lock(installation.directory / "admission.lock")
    try:
        _run_installer(installation, installer, "check")
    finally:
        os.close(gate)
    records = [required_record(runs.load_record(path)) for path in runs.all_runs()]
    assert len(records) == 2
    assert {row["label"] for row in records} == {
        "lifecycle-maintenance-check-main",
        "lifecycle-maintenance-check-validation",
    }
    assert all(
        row["cleanup"]["status"] == "confirmed" and runs.group_of(row)["status"] == "gone"
        for row in records
    )
    assert list((installation.directory / "maintenance-commands").glob("*.json")) == []


def test_campaign_disposal_cli_projection_retains_transfer_identity_without_private_tokens():
    receipt = {
        "maintenance_successor": {"operation": "dispose-campaign", "token": "successor-secret"},
        "transferred_maintenance": {
            "predecessor": {
                "sha256": "a" * 64,
                "bytes": "private-raw-marker",
                "record": {"operation": "maintenance", "token": "predecessor-secret"},
            }
        },
    }
    projected = service._campaign_disposal_projection(receipt)
    assert "token" not in projected["maintenance_successor"]
    assert projected["transferred_maintenance"]["predecessor"] == {
        "sha256": "a" * 64,
        "record": {"operation": "maintenance"},
    }
    assert receipt["maintenance_successor"]["token"] == "successor-secret"
    assert (
        receipt["transferred_maintenance"]["predecessor"]["record"]["token"] == "predecessor-secret"
    )


def test_campaign_status_redacts_disposal_transfer_tokens(
    lifecycle_campaign_ready, monkeypatch, capsys
):
    installation, _ = lifecycle_campaign_ready
    service.private_json(
        installation.directory / "disposal.json",
        {
            "maintenance_successor": {"operation": "dispose-campaign", "token": "successor-secret"},
            "transferred_maintenance": {
                "predecessor": {
                    "sha256": "a" * 64,
                    "bytes": "private-raw-marker",
                    "record": {"operation": "maintenance", "token": "predecessor-secret"},
                }
            },
        },
    )
    monkeypatch.setattr(
        service.Installation, "load", classmethod(lambda cls, *args, **kwargs: installation)
    )
    monkeypatch.setattr(service, "borrowers", lambda inst: [])
    import storage_service

    monkeypatch.setattr(storage_service, "dependencies", lambda inst: [])
    assert service.main(["status"]) == 0
    output = capsys.readouterr().out
    assert all(
        secret not in output
        for secret in ["successor-secret", "predecessor-secret", "private-raw-marker"]
    )
    assert (
        json.loads(output)["disposal"]["transferred_maintenance"]["predecessor"]["sha256"]
        == "a" * 64
    )
