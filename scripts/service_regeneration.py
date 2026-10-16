"""Explicit discard/rebuild of both scopes of one authenticated project installation.

The immutable plan is a resume authority, never a rollback asset. Old native outcomes
remain evidence; separate discard acknowledgements release their current obligations.
"""

from __future__ import annotations

import contextlib
import hashlib
import json
import os
import re
import secrets
import shutil
from collections.abc import Mapping
from pathlib import Path
from typing import Any

import surrealdb_service as service
from harness import ProcessIdentity, read_json, try_hold_lock
from storage_lifecycle import Storage, admission, durable_json, fsync_directory, private_directory
from storage_service import prepare, tools_root

OPERATION = "project installation regeneration"


def _read(path: Path) -> dict[str, Any]:
    value, _ = service._private_campaign_record(path)
    return value


def _inventory(installation: service.Installation) -> dict[str, Any]:
    """Inspect actual scopes; installation metadata alone cannot authorize discard."""
    cfg = {**installation.runtime("main", installer=True), "database": None}
    rows = service.sql(installation, "INFO FOR ROOT; INFO FOR NS;", cfg=cfg)
    try:
        namespaces = rows[0]["result"]["namespaces"]
        databases = rows[1]["result"]["databases"]
        if (
            len(rows) != 2
            or not isinstance(namespaces, dict)
            or not isinstance(databases, dict)
            or set(namespaces) != {service.NAMESPACE}
            or set(databases) != set(service.DATABASES)
        ):
            raise ValueError
    except KeyError, IndexError, TypeError, ValueError:
        raise service.FixtureBlocked(
            "scope", "actual namespaces/databases differ; unexpected scopes are protected"
        ) from None
    return {"namespace": service.NAMESPACE, "databases": sorted(databases)}


def _pending_bindings(installation: service.Installation) -> None:
    generation = bytes(installation.record["service_generation"]).hex()
    for row in service.borrowers(installation):
        native = row.get("native_drain") or {}
        directory = installation.directory / "attachments" / row["id"]
        if (
            row.get("fixture") != installation.id
            or native.get("generation") != generation
            or native.get("attachment") != row["id"]
            or native.get("directory") != str(directory / "native-drain")
        ):
            raise service.FixtureBlocked(
                "identity", "pending attachment scope or generation differs"
            )


def _indexes(root: Path) -> dict[str, Any]:
    result = {}
    paths = [
        *(root / "maintenance-commands").glob("*.json"),
        *(root / "development").glob("*/record.json"),
    ]
    for path in paths:
        result[path.relative_to(root).as_posix()] = {
            "record": _read(path),
            "sha256": service.file_sha256(path),
        }
    return result


def _retire_indexes(root: Path, journal: Path, plan: Mapping[str, Any]) -> None:
    # These are active-generation discovery bindings, not original run receipts.
    # Preserve their authenticated bytes with the operation evidence after discard.
    for relative, value in plan["indexes"].items():
        source, target = root / relative, journal.parent / "retired-indexes" / relative
        if (
            not source.exists()
            and target.exists()
            and service.file_sha256(target) == value["sha256"]
        ):
            continue
        if _read(source) != value["record"] or service.file_sha256(source) != value["sha256"]:
            raise service.FixtureBlocked("identity", "predecessor run discovery binding changed")
        private_directory(target.parent)
        os.replace(source, target)
        fsync_directory(source.parent)
        fsync_directory(target.parent)


def _files() -> list[str]:
    return [
        "server.env",
        *(f"{db}-{kind}.json" for db in service.DATABASES for kind in ("runtime", "installer")),
    ]


def _configs(installation: service.Installation, successor: dict[str, Any]) -> dict[str, str]:
    root = installation.directory
    username, password = "library_context_installer", secrets.token_urlsafe(32)
    values = {
        "server.env": "".join(
            f"{k}={v}\n"
            for k, v in {
                "SURREAL_USER": username,
                "SURREAL_PASS": password,
                **service.SERVER_SETTINGS,
            }.items()
        )
    }
    for db in service.DATABASES:
        previous = installation.runtime(db)
        if previous.get("selection") != str(root / f"{db}-selected.json"):
            raise service.FixtureBlocked("scope", "external selection state is protected")
        cfg = {
            **previous,
            "service_generation": successor["service_generation"],
            "password": secrets.token_urlsafe(32),
            "viewer_password": secrets.token_urlsafe(32),
        }
        values[f"{db}-runtime.json"] = json.dumps(cfg, sort_keys=True)
        values[f"{db}-installer.json"] = json.dumps(
            {**cfg, "username": username, "password": password, "authentication": "root"},
            sort_keys=True,
        )
    return values


def load_plan(
    installation: service.Installation, reference: Mapping[str, Any]
) -> tuple[Path, dict[str, Any]]:
    operation = reference.get("operation")
    if not isinstance(operation, str) or not re.fullmatch("[a-f0-9]{64}", operation):
        raise service.FixtureBlocked("identity", "regeneration operation identity is unavailable")
    path = installation.directory / "regenerations" / operation / "plan.json"
    plan = _read(path)
    if (
        reference.get("journal") != str(path)
        or reference.get("journal_sha256") != service.file_sha256(path)
        or plan.get("operation") != operation
        or plan.get("installation_id") != installation.id
        or plan.get("schema") != 1
        or plan.get("kind") != OPERATION
    ):
        raise service.FixtureBlocked("identity", "immutable regeneration journal differs")
    return path, plan


def completed_discard(directory: Path, reference: Mapping[str, Any]) -> dict[str, Any]:
    """Validate a private, exact whole-target discard proof without querying old data."""
    installation = service.Installation(directory, _read(directory / "installation.json"))
    path, plan = load_plan(installation, reference)
    proof = _read(path.parent / "discard.json")
    if (
        proof.get("schema") != 1
        or proof.get("kind") != "project-installation-discard"
        or proof.get("installation_id") != installation.id
        or proof.get("generation") != bytes(plan["previous"]["service_generation"]).hex()
        or proof.get("successor_generation") != bytes(plan["successor"]["service_generation"]).hex()
        or proof.get("reference") != dict(reference)
        or proof.get("outcome") != "passed"
        or proof.get("steps") != {"executor-shutdown": "passed", "datastore-disposal": "passed"}
        or proof.get("scopes")
        != {"namespace": service.NAMESPACE, "databases": sorted(service.DATABASES)}
    ):
        raise service.FixtureBlocked("identity", "regeneration discard proof differs")
    return proof


def completed_attachment(directory: Path, record: Mapping[str, Any] | None) -> bool:
    """Separate current cleanup disposition; original native uncertainty is retained."""
    try:
        import surrealdb_fixture as fixture

        if not isinstance(record, dict) or record.get("released") is not True:
            return False
        path = directory / "installation-discard.json"
        receipt = _read(path)
        if record.get("installation_discard") != {
            "path": str(path),
            "sha256": service.file_sha256(path),
        }:
            return False
        proof = completed_discard(directory.parent.parent, receipt["reference"])
        original = receipt["original"]
        reconstructed = {
            key: value
            for key, value in record.items()
            if key not in {"released", "released_at", "installation_discard"}
        }
        expected = {
            key: value for key, value in original.items() if key not in {"released", "released_at"}
        }
        native = original["native_drain"]
        return (
            receipt.get("schema") == 1
            and receipt.get("kind") == "project-installation-attachment-discard"
            and receipt.get("local_owner") == fixture.scope_local_owner(directory)
            and directory.name == original["id"] == record["id"]
            and directory.parent.name == "attachments"
            and original["fixture"] == proof["installation_id"]
            and native["generation"] == proof["generation"]
            and native["attachment"] == original["id"]
            and native["directory"] == str(directory / "native-drain")
            and reconstructed == expected
        )
    except OSError, ValueError, KeyError, TypeError, AttributeError, service.FixtureBlocked:
        return False


def acknowledge_attachment(directory: Path, reference: Mapping[str, Any]) -> dict[str, Any]:
    import surrealdb_fixture as fixture

    if not directory.is_absolute() or any(p.is_symlink() for p in (directory, *directory.parents)):
        raise service.FixtureBlocked("identity", "regeneration attachment path must be physical")
    lock = try_hold_lock(directory / "owner.lock")
    if lock is None:
        raise service.FixtureBlocked("drainage", "regeneration attachment has a live owner")
    try:
        row = _read(directory / "record.json")
        if not completed_attachment(directory, row):
            proof = completed_discard(directory.parent.parent, reference)
            native = row.get("native_drain", {})
            owner = ProcessIdentity.from_json(row["owner"])
            if (
                owner.foreign()
                or owner.alive()
                or row.get("fixture") != proof["installation_id"]
                or native.get("generation") != proof["generation"]
            ):
                raise service.FixtureBlocked(
                    "identity", "regeneration attachment owner or generation differs"
                )
            receipt = {
                "schema": 1,
                "kind": "project-installation-attachment-discard",
                "local_owner": fixture.scope_local_owner(directory),
                "reference": dict(reference),
                "original": row,
            }
            path = directory / "installation-discard.json"
            if path.exists():
                if _read(path) != receipt:
                    raise service.FixtureBlocked(
                        "identity", "regeneration attachment acknowledgement differs"
                    )
            else:
                durable_json(path, receipt)
                path.chmod(0o400)
            row = {
                **row,
                "released": True,
                "released_at": fixture.now(),
                "installation_discard": {"path": str(path), "sha256": service.file_sha256(path)},
            }
            if not completed_attachment(directory, row):
                raise service.FixtureBlocked(
                    "identity", "regeneration attachment acknowledgement is invalid"
                )
            durable_json(directory / "record.json", row)
        fixture._complete_attachment_obligations(
            directory, "authenticated whole project datastore discard", owner_lease=lock
        )
        return row
    finally:
        os.close(lock)


def _cli(installation: service.Installation, action: str, token: str) -> None:
    # Existing managed maintenance owner enrolls before launch and drains the waitable
    # child before reaping. Tokens stay in the private environment, never argv/receipts.
    previous = os.environ.get("LCTX_SURREAL_MAINTENANCE_TOKEN")
    os.environ["LCTX_SURREAL_MAINTENANCE_TOKEN"] = token
    try:
        for db in service.DATABASES:
            installation.runtime(db, installer=True)
            args = [
                installation.record["installer"],
                "store",
                "--runtime-config",
                str(installation.runtime_path(db, installer=True)),
                action,
            ]
            if action == "init":
                args.append("--keep-closed")
            if service._run_maintenance_installer(installation, args, action, db):
                raise service.FixtureFailed(
                    "installer", f"regeneration {action} failed for {db}; admission stays closed"
                )
    finally:
        if previous is None:
            os.environ.pop("LCTX_SURREAL_MAINTENANCE_TOKEN", None)
        else:
            os.environ["LCTX_SURREAL_MAINTENANCE_TOKEN"] = previous


def _bootstrap(installation: service.Installation) -> None:
    for db in service.DATABASES:
        cfg = installation.runtime(db)
        service.sql(
            installation,
            f"DEFINE NAMESPACE IF NOT EXISTS {service.NAMESPACE}; USE NS {service.NAMESPACE}; "
            f"DEFINE DATABASE IF NOT EXISTS {db} STRICT; USE DB {db}; "
            f"DEFINE USER OVERWRITE {cfg['username']} ON DATABASE "
            f"PASSWORD {json.dumps(cfg['password'])} ROLES OWNER; "
            f"DEFINE USER OVERWRITE {cfg['viewer_username']} ON DATABASE "
            f"PASSWORD {json.dumps(cfg['viewer_password'])} ROLES VIEWER;",
            cfg=installation.runtime(db, installer=True),
        )


def _checkpoint(path: Path, progress: dict[str, Any], phase: str) -> None:
    progress["phase"] = phase
    durable_json(path, progress)


def regenerate(
    installation: service.Installation,
    identity: str,
    *,
    scopes: str,
    installer: Path,
    descriptor: Path,
) -> dict[str, Any]:
    root = installation.directory
    if identity != installation.id or scopes != "main,validation":
        raise service.FixtureBlocked(
            "scope", "regeneration requires the exact installation ID and main,validation"
        )
    if installation.record.get("recovery_assets"):
        raise service.FixtureBlocked("scope", "external recovery assets are protected")
    if any(
        key in os.environ
        for key in (
            "LCTX_NATIVE_DRAIN_DIRECTORY",
            "LCTX_FIXTURE_ATTACHMENT",
            "LCTX_SURREAL_GENERATION",
            "LCTX_SIGNAL_CONTROL_MODE",
        )
    ):
        raise service.FixtureBlocked(
            "ownership", "regeneration refuses inherited native owner bindings"
        )
    with (
        admission([root, installer, descriptor.parent, tools_root(root)]),
        contextlib.ExitStack() as stack,
    ):
        for name in ("admission.lock", "borrowers.lock"):
            lock = try_hold_lock(root / name)
            if lock is None:
                raise service.FixtureBlocked(
                    "drainage", "regeneration admission or borrower lease is held"
                )
            stack.callback(os.close, lock)
        current = service.Installation(root, service._descriptor(root))
        if current.record != installation.record:
            raise service.FixtureBlocked("identity", "installation changed before regeneration")
        candidate = prepare(root, installer)
        server = service._owned_server_generation(root, descriptor)
        marker_path = root / "maintenance.json"
        marker: dict[str, Any] | None = read_json(marker_path)
        plan: dict[str, Any]
        if marker:
            owner = ProcessIdentity.from_json(marker["owner"])
            if marker.get("operation") != OPERATION or owner.foreign() or owner.alive():
                raise service.FixtureBlocked(
                    "ownership", "resume requires the dead exact regeneration owner"
                )
            journal, plan = load_plan(current, marker["regeneration"])
            if plan["candidate"] != candidate or plan["server"] != server:
                raise service.FixtureBlocked(
                    "identity", "resume requires the exact installer and server candidates"
                )
        else:
            if current.record.get("phase") != "ready":
                raise service.FixtureBlocked(
                    "identity", "initial regeneration requires a ready installation"
                )
            service._installation_executor(current)
            if not getattr(current, "_daemon_identity", None):
                raise service.FixtureBlocked(
                    "readiness", "initial regeneration requires authenticated live inventory"
                )
            proof = service._campaign_current_quiescence(current, installation_regeneration=True)
            _pending_bindings(current)
            inventory = _inventory(current)
            operation = hashlib.sha256(secrets.token_bytes(32)).hexdigest()
            successor: dict[str, Any] = {
                **current.record,
                "phase": "regenerating",
                "service_generation": list(secrets.token_bytes(32)),
                "binary": service._generation_binary(server),
                "installer": candidate["path"],
                "installer_generation": candidate,
                "native_schema_version": 9,
            }
            for key in ("server_handoff", "installer_handoff", "regeneration", "binary_origin"):
                successor.pop(key, None)
            plan = {
                "schema": 1,
                "kind": OPERATION,
                "operation": operation,
                "installation_id": identity,
                "previous": current.record,
                "successor": successor,
                "candidate": candidate,
                "server": server,
                "scopes": inventory,
                "quiescence": proof,
                "previous_unit": service._unit_path(current.unit).read_text(),
                "successor_unit": service.unit_text(
                    root,
                    Path(server["path"]),
                    int(current.endpoint.rsplit(":", 1)[1]),
                    log_filter=current.record.get("server_log_filter", "warn"),
                    development=current.record.get("purpose") == service.LIFECYCLE_PURPOSE,
                ),
                "previous_files": {name: service.file_sha256(root / name) for name in _files()},
            }
            plan["indexes"] = _indexes(root)
            private_directory(root / "regenerations")
            directory = root / "regenerations" / operation
            private_directory(directory)
            private_directory(directory / "staged")
            configs = _configs(current, successor)
            for name, content in configs.items():
                service._write_private(directory / "staged" / name, content)
            plan["successor_files"] = {
                name: service.file_sha256(directory / "staged" / name) for name in configs
            }
            journal = directory / "plan.json"
            durable_json(journal, plan)
            journal.chmod(0o400)
            fsync_directory(directory)
            Storage().publish(
                directory,
                "native-content",
                {"kind": "native", "path": str(directory)},
                "project-regeneration:" + identity + ":" + operation,
                requires="raw-replay",
            )
            marker = {
                "operation": OPERATION,
                "regeneration": {
                    "operation": operation,
                    "journal": str(journal),
                    "journal_sha256": service.file_sha256(journal),
                },
            }
            _checkpoint(directory / "progress.json", {}, "prepared")
        reference: dict[str, Any] = marker["regeneration"]
        progress_path = journal.parent / "progress.json"
        progress = _read(progress_path)
        final: dict[str, Any] = {**plan["successor"], "phase": "ready", "regeneration": reference}
        if current.record not in (plan["previous"], plan["successor"], final):
            raise service.FixtureBlocked(
                "identity", "regeneration descriptor differs from its journal"
            )
        if service._unit_path(current.unit).read_text() not in (
            plan["previous_unit"],
            plan["successor_unit"],
        ):
            raise service.FixtureBlocked(
                "identity", "regeneration unit changed outside its journal"
            )
        phase = progress["phase"]
        if phase not in {
            "prepared",
            "discarding",
            "discarded",
            "installed",
            "initialized",
            "ready",
        }:
            raise service.FixtureBlocked("identity", "regeneration phase is invalid")
        if phase in {"installed", "initialized", "ready"}:
            # A dead coordinator can leave an enrolled installer child alive. Reobserve
            # the successor's actual run/host/executor owners before transferring its token.
            resumed_quiescence = service._campaign_current_quiescence(
                current, installation_regeneration=True
            )
            # Host exit alone cannot settle a submitted installer request. Join the
            # authenticated whole executor before reissuing idempotent current setup.
            service._stop_owned(current)
            indexes = progress.setdefault("successor_indexes", {})
            for name, value in _indexes(root).items():
                if name in indexes and indexes[name] != value:
                    raise service.FixtureBlocked("identity", "successor command binding changed")
                indexes[name] = value
            progress["successor_quiescence"] = resumed_quiescence
            durable_json(progress_path, progress)
            _retire_indexes(root, journal, {"indexes": indexes})
        marker.update(owner=ProcessIdentity.of().to_json(), token=secrets.token_urlsafe(32))
        durable_json(marker_path, marker)
        if phase in {"prepared", "discarding"}:
            # No old-schema CLI is invoked. Whole-executor shutdown fences native work,
            # including lost-response operations, after current local client drainage.
            current_quiescence = service._campaign_current_quiescence(
                current, installation_regeneration=True
            )
            _pending_bindings(current)
            if current_quiescence["executor_active"] and _inventory(current) != plan["scopes"]:
                raise service.FixtureBlocked("scope", "actual scope inventory changed")
            _checkpoint(progress_path, progress, "discarding")
            service._stop_owned(current)
            store = root / "data/store"
            if any(p.is_symlink() for p in (store, *store.parents)) or (
                store.exists() and (not store.is_dir() or store.stat().st_uid != os.getuid())
            ):
                raise service.FixtureBlocked("identity", "owned datastore path is not physical")
            if store.exists():
                shutil.rmtree(store)
            fsync_directory(store.parent)
            for db in service.DATABASES:
                selection = root / f"{db}-selected.json"
                if selection.is_symlink():
                    raise service.FixtureBlocked("identity", "selection pointer is not physical")
                selection.unlink(missing_ok=True)
            discard = {
                "schema": 1,
                "kind": "project-installation-discard",
                "installation_id": identity,
                "generation": bytes(plan["previous"]["service_generation"]).hex(),
                "successor_generation": bytes(plan["successor"]["service_generation"]).hex(),
                "reference": reference,
                "outcome": "passed",
                "steps": {"executor-shutdown": "passed", "datastore-disposal": "passed"},
                "scopes": plan["scopes"],
            }
            durable_json(journal.parent / "discard.json", discard)
            _checkpoint(progress_path, progress, "discarded")
            phase = "discarded"
        completed_discard(root, reference)
        _retire_indexes(root, journal, plan)
        if phase == "discarded":
            service._stop_owned(current)
            for name in _files():
                staged = journal.parent / "staged" / name
                if service.file_sha256(staged) != plan["successor_files"][
                    name
                ] or service.file_sha256(root / name) not in (
                    plan["previous_files"][name],
                    plan["successor_files"][name],
                ):
                    raise service.FixtureBlocked(
                        "identity", "regeneration configuration changed outside journal"
                    )
                service._write_private(root / name, staged.read_text())
            service._write_private(service._unit_path(current.unit), plan["successor_unit"])
            durable_json(root / "installation.json", plan["successor"])
            current = service.Installation(root, dict(plan["successor"]))
            if service.systemctl("daemon-reload").returncode:
                raise service.FixtureBlocked("systemd", "regeneration daemon-reload failed")
            _checkpoint(progress_path, progress, "installed")
            phase = "installed"
        else:
            current = service.Installation(root, dict(plan["successor"]))
        if service._unit_path(current.unit).read_text() != plan["successor_unit"]:
            raise service.FixtureBlocked(
                "identity", "successor unit differs from regeneration journal"
            )
        for name in _files():
            if service.file_sha256(root / name) != plan["successor_files"][name]:
                raise service.FixtureBlocked(
                    "identity", "successor configuration changed outside journal"
                )
        service._owned_daemon_start(current)
        if phase == "installed":
            _bootstrap(current)
            _cli(current, "init", marker["token"])
            _checkpoint(progress_path, progress, "initialized")
        _cli(current, "check", marker["token"])
        current.check(allow_maintenance=True)
        if _inventory(current) != plan["scopes"]:
            raise service.FixtureBlocked("scope", "successor has unexpected database scopes")
        for path in (root / "attachments").glob("*/record.json"):
            row = _read(path)
            if row.get("fixture") != identity:
                raise service.FixtureBlocked(
                    "identity", "foreign attachment in project installation"
                )
            if (
                row.get("native_drain", {}).get("generation")
                == bytes(plan["previous"]["service_generation"]).hex()
            ):
                acknowledge_attachment(path.parent, reference)
        try:
            _cli(current, "open-admission", marker["token"])
        except BaseException:
            with contextlib.suppress(Exception):
                _cli(current, "close-admission", marker["token"])
            raise
        durable_json(root / "installation.json", final)
        _checkpoint(progress_path, progress, "ready")
        # Current private configs are the only credential copies with a consumer.
        for name in _files():
            (journal.parent / "staged" / name).unlink(missing_ok=True)
        if (journal.parent / "staged").exists():
            (journal.parent / "staged").rmdir()
        marker_path.unlink()
        fsync_directory(root)
        return {
            "outcome": "passed",
            "installation_id": identity,
            "generation": bytes(final["service_generation"]).hex(),
            "server_generation": server["identity"],
            "regeneration": reference["operation"],
            "scopes": plan["scopes"],
            "admission_open": True,
        }
