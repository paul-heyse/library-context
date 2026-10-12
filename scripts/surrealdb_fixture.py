#!/usr/bin/env python3
"""Attachment-only native validation controls on the installed durable SurrealDB service.

``just fixture -- CMD`` borrows stable validation storage with a fresh logical attempt.
It never starts a server, creates a namespace/database, or deletes persisted content.
Disruptive diagnostics/restart require ``just service maintenance -- just fixture ...``.
Missing/incompatible installation is blocked with an explicit installation repair.
"""

from __future__ import annotations

import argparse
import base64
import contextlib
import hashlib
import http.client
import json
import os
import re
import shlex
import signal
import stat
import subprocess
import sys
import time
import urllib.error
import urllib.request
import uuid
from collections.abc import Callable, Iterator, Mapping, Sequence
from dataclasses import dataclass, field
from datetime import UTC, datetime
from pathlib import Path
from typing import Any

import surrealdb_service as service
from harness import (
    ProcessIdentity,
    SpawnGuard,
    cleanup_group,
    hold_lock,
    lock_held,
    observe_exit,
    read_json,
    recorded_group,
    spawn_group,
    try_hold_lock,
    write_json_atomic,
)
from surrealdb_service import (
    FixtureBlocked,
    FixtureFailed,
    Installation,
    file_sha256,
    systemctl,
    unit_properties,
)

ROOT = Path(__file__).resolve().parent.parent
EXIT_BLOCKED = 75
STOP_GRACE = 8.0
ADMIN_USER = "validation_writer"
ATTACHMENT_VARIABLES = (
    "LCTX_SURREAL_TEST_CONFIG",
    "LCTX_COMPILER_RUNTIME_CONFIG",
    "LCTX_SURREAL_SERVICE_CONFIG",
    "LCTX_SURREAL_ATTEMPT_ID",
    "LCTX_SURREAL_INSTALLATION_ID",
    "LCTX_SURREAL_GENERATION",
    "LCTX_FIXTURE_ID",
    "LCTX_FIXTURE_ATTACHMENT",
    "LCTX_NATIVE_DRAIN_DIRECTORY",
    "LCTX_PD02_RECOVERY_DIRECTORY",
)


def _stderr(message: str) -> None:
    print(message, file=sys.stderr, flush=True)


def now() -> str:
    return datetime.now(UTC).isoformat(timespec="seconds")


def _identifier(value: str) -> str:
    if not value or not value.isascii() or not all(c.isalnum() or c == "_" for c in value):
        raise ValueError("expected an ASCII identifier")
    return value


def fixtures_root(env: Mapping[str, str] | None = None) -> Path:
    return service.state_root(env) / "attachments"


def owner_alive(identity: Mapping[str, Any] | None, lock: Path) -> bool:
    if lock_held(lock):
        return True
    try:
        return bool(identity) and ProcessIdentity.from_json(identity).alive()
    except ValueError, TypeError, KeyError:
        return False


class FixtureQueryError(RuntimeError):
    """Sanitized request/statement evidence; never retain SQL or native error bodies."""

    def __init__(self, diagnostic: dict[str, Any]) -> None:
        self.diagnostic = diagnostic
        transport = diagnostic["transport"]
        if transport["status"] != "OK":
            detail = f"transport {transport['category']}"
            if transport.get("http_status") is not None:
                detail += f" HTTP {transport['http_status']}"
        else:
            detail = "statements " + ", ".join(
                f"{row['index']}:{row['category']}"
                for row in diagnostic["statements"]
                if row["status"] != "OK"
            )
        super().__init__(f"fixture query failed: {detail}")


@dataclass(frozen=True)
class Readiness:
    """Same shape as workspace_env.Readiness; the repair is the route text, not a sync route."""

    requirement: str
    ready: bool
    detail: str
    repair_route: str | None = None

    @property
    def outcome(self) -> str:
        return "passed" if self.ready else "blocked"

    @property
    def repair(self) -> str:
        return self.repair_route or ""

    def message(self) -> str:
        if self.ready:
            return f"{self.requirement}: ready ({self.detail})"
        return f"{self.requirement}: blocked: {self.repair} ({self.detail})"


def _git(*args: str) -> str:
    result = subprocess.run(
        ["git", "-C", str(ROOT), *args], capture_output=True, timeout=60, check=False
    )
    return result.stdout.decode(errors="replace").strip() if result.returncode == 0 else ""


def _server_identity(installation: Installation) -> dict[str, str]:
    if checked := service.checked_generation(installation):
        return checked.server_identity(installation)
    service._installed_binary(installation)
    binary = installation.record["binary"]
    identity = {"version": binary["version"], "binary_sha256": binary["sha256"]}
    if generation := binary.get("generation"):
        identity["generation"] = generation["identity"]
    return identity


def source_inputs(installation: Installation) -> dict[str, Any]:
    """The producing checkout and exact verified installed server identity."""
    diff = subprocess.run(
        ["git", "-C", str(ROOT), "diff", "HEAD", "--binary"],
        capture_output=True,
        timeout=120,
        check=False,
    ).stdout
    untracked = _git("ls-files", "--others", "--exclude-standard")
    digest = hashlib.sha256(diff + b"\0" + untracked.encode()).hexdigest()
    return {
        "head": _git("rev-parse", "HEAD"),
        "uncommitted_sha256": digest,
        "server": _server_identity(installation),
    }


# ---------------------------------------------------------------------------------------------
# The server substrate


def _command_observation(record: Mapping[str, Any] | None) -> dict[str, Any]:
    record = record or {}
    command, cleanup = record.get("command") or {}, record.get("command_cleanup") or {}
    if not command and not cleanup:
        return {"status": "gone", "members": []}
    if not cleanup and not any(command.get(key) for key in ("leader", "argv", "started")):
        return {"status": "gone", "members": []}
    if cleanup.get("status") == "confirmed":
        return {"status": "gone", "members": []}
    leader = command.get("leader")
    if (not leader or "start_ticks" not in leader) and record.get("owner"):
        with contextlib.suppress(ValueError, KeyError, TypeError):
            if ProcessIdentity.from_json(record["owner"]).previous_boot():
                return {"status": "gone", "members": [], "reason": "previous boot"}
    if leader:
        leader = {**leader, "pgid": leader.get("pgid", leader.get("pid"))}
    return recorded_group(leader, members=cleanup.get("members", []))


def _cleanup_pending(record: Mapping[str, Any] | None) -> bool:
    cleanup = (record or {}).get("command_cleanup")
    if cleanup is not None:
        if _command_observation(record).get("reason") == "previous boot":
            return False
        return cleanup.get("status") != "confirmed"
    return _command_observation(record)["status"] == "unknown"


def native_failure_diagnostics(owner: Mapping[str, Any]) -> list[dict[str, Any]]:
    """Explain persisted typed causes; no field in this projection authorizes release."""
    completion = owner.get("completion")
    failures = completion.get("failures") if isinstance(completion, dict) else None
    if not isinstance(failures, list):
        return []
    result = []
    for failure in failures:
        diagnostic = failure.get("diagnostic") if isinstance(failure, dict) else None
        causes = diagnostic.get("causes") if isinstance(diagnostic, dict) else None
        valid = (
            isinstance(diagnostic, dict)
            and diagnostic.get("schema") == 1
            and type(diagnostic.get("truncated")) is bool
            and isinstance(causes, list)
            and 0 < len(causes) <= 16
        )
        if valid:
            for cause in causes:
                if not isinstance(cause, dict):
                    valid = False
                    break
                identities, issuance = cause.get("identities"), cause.get("issuance")
                valid = (
                    cause.get("class") in {"model", "infrastructure", "native", "opaque"}
                    and cause.get("code")
                    in {
                        "completion",
                        "context",
                        "serving",
                        "resource",
                        "invalid",
                        "limit",
                        "frontier",
                        "schema",
                        "identity",
                        "conflict",
                        "codec",
                        "transport",
                        "unconfirmed",
                        "refused",
                        "contention",
                        "state",
                        "contract",
                        "io",
                        "exhausted",
                        "native_request",
                        "native_attempt",
                        "native_lifecycle",
                        "opaque",
                    }
                    and isinstance(cause.get("stage"), str)
                    and len(cause["stage"].encode()) <= 256
                    and isinstance(identities, list)
                    and len(identities) <= 2
                    and all(
                        isinstance(identity, dict)
                        and identity.get("kind") in {"operation", "request", "attempt", "lifecycle"}
                        and isinstance(identity.get("value"), str)
                        and re.fullmatch("[a-f0-9]{64}", identity["value"])
                        for identity in identities
                    )
                    and (
                        issuance is None
                        or (
                            isinstance(issuance, dict)
                            and isinstance(issuance.get("generation"), str)
                            and re.fullmatch("[a-f0-9]{64}", issuance["generation"])
                            and type(issuance.get("era")) is int
                            and issuance["era"] > 0
                        )
                    )
                    and cause.get("lifecycle")
                    in {
                        None,
                        "reader_pin",
                        "backup_hold",
                        "retirement_invocation",
                        "retirement_successor",
                        "cleanup_obligation",
                        "history_invocation",
                    }
                    and cause.get("inspect")
                    in {None, "native_request", "native_attempt", "native_lifecycle"}
                )
                if not valid:
                    break
        # Old/missing diagnostic fields stay unavailable, never a successful outcome.
        result.append(
            {
                "status": "typed" if valid else "unavailable" if diagnostic is None else "invalid",
                "diagnostic": diagnostic if valid else None,
            }
        )
    return result


def native_completion(directory: Path, record: Mapping[str, Any] | None) -> dict[str, Any]:
    """Observe actual Rust owner receipts, separately from child/process cleanup."""
    binding = (record or {}).get("native_drain")
    if binding is None:
        return {"status": "not_observed", "owners": []}
    root = directory / "native-drain"
    if (
        not isinstance(binding, dict)
        or binding.get("directory") != str(root)
        or not re.fullmatch("[a-f0-9]{64}", binding.get("generation", ""))
        or not root.is_dir()
        or root.is_symlink()
        or binding.get("attachment") != (record or {}).get("id")
    ):
        return {"status": "unknown", "reason": "native owner binding differs", "owners": []}
    owners = []
    status = "confirmed"
    try:
        for path in sorted(root.iterdir()):
            # An interrupted atomic publication or malformed producer is unknown.
            row = read_json(path) if path.is_file() and not path.is_symlink() else None
            if (
                not isinstance(row, dict)
                or row.get("schema") != 1
                or row.get("kind") != "native-process-owner"
                or row.get("attachment") != binding["attachment"]
                or row.get("generation") != binding.get("generation")
                or not isinstance(row.get("owner"), str)
                or not row["owner"]
                or not re.fullmatch(r"[1-9][0-9]*-[0-9]+-[0-9]+\.json", path.name)
                or row.get("process")
                != {
                    "pid": int(path.name.split("-")[0]),
                    "start_ticks": int(path.name.split("-")[1]),
                }
                or type(row.get("registration")) is not int
                or row["registration"] != int(path.stem.split("-")[2])
            ):
                return {
                    "status": "unknown",
                    "reason": "native owner receipt differs",
                    "owners": owners,
                }
            owners.append(
                {"path": str(path), **row, "failure_diagnostics": native_failure_diagnostics(row)}
            )
            completion = row.get("completion")
            if (
                row.get("state") != "complete"
                or not isinstance(completion, dict)
                or completion.get("local") != "terminal"
                or completion.get("remote") != "confirmed"
                or completion.get("failures") != []
                or not isinstance(completion.get("committed"), list)
                or not isinstance(completion.get("storage"), list)
                or any(
                    not isinstance(storage, dict) or storage.get("state") != "removed"
                    for storage in completion["storage"]
                )
            ):
                status = "unknown"
    except OSError, ValueError, TypeError, KeyError:
        return {"status": "unknown", "reason": "native owner receipt unavailable", "owners": owners}
    if not owners:
        status = "not_observed"
    return {"status": status, "owners": owners}


def scope_record_identity(value: Mapping[str, Any]) -> str:
    """Identity of a receipt snapshot, not a portable native authority."""
    return hashlib.sha256(
        json.dumps(value, sort_keys=True, separators=(",", ":")).encode()
    ).hexdigest()


def scope_local_owner(directory: Path) -> dict[str, int]:
    """Bind this acknowledgment to its local attachment directory lifetime."""
    from storage_lifecycle import physical

    for path in (directory, directory.parent, directory.parent.parent):
        info = path.stat()
        if info.st_uid != os.getuid() or info.st_mode & 0o022:
            raise ValueError("scope receipt owner directory is not private")
    return {**physical(directory), "uid": directory.stat().st_uid}


SCOPE_LIFECYCLE_FIELDS = ("released", "released_at", "command_cleanup", "recovered_local_cleanup")


def completed_scope_disposition(directory: Path, record: Mapping[str, Any] | None) -> bool:
    """Consume this local owner's completed discard; original native finality stays unknown.

    The maintenance producer acknowledged the dated proofs before publication. This
    reader checks immutable local bytes and correspondence, never old sources or
    current server implementation fingerprints. Same-UID malicious rewrites are
    outside this receipt owner's filesystem trust boundary.
    """
    try:
        if not isinstance(record, dict) or record.get("released") is not True:
            return False
        path = directory / "scope-disposition.json"
        if (
            directory.name != record["id"]
            or directory.parent.name != "attachments"
            or any(p.is_symlink() for p in (path, *path.parents))
        ):
            return False
        fd = os.open(path, os.O_RDONLY | os.O_CLOEXEC | os.O_NOFOLLOW)
        try:
            before = os.fstat(fd)
            if (
                not stat.S_ISREG(before.st_mode)
                or before.st_uid != os.getuid()
                or stat.S_IMODE(before.st_mode) != 0o400
            ):
                return False
            with os.fdopen(fd, "rb", closefd=False) as stream:
                raw = stream.read()
            if service.executable_identity(before) != service.executable_identity(os.fstat(fd)):
                return False
        finally:
            os.close(fd)
        if record.get("scope_disposition") != {
            "path": str(path),
            "sha256": hashlib.sha256(raw).hexdigest(),
        }:
            return False
        receipt = json.loads(raw)
        if receipt["local_owner"] != scope_local_owner(directory):
            return False
        scope, original = receipt["scope"], receipt["original_attachment"]
        hexadecimal = lambda value: (
            isinstance(value, str) and re.fullmatch(r"[0-9a-f]{64}", value) is not None
        )
        generation = lambda value: (
            isinstance(value, list)
            and len(value) == 32
            and all(type(x) is int and 0 <= x <= 255 for x in value)
        )
        databases = scope["databases"]
        if (
            receipt.get("schema") != 1
            or receipt.get("kind") != "explicit-scope-disposition"
            or receipt.get("outcome") != "discarded-and-regenerated"
            or receipt.get("original_native_outcome") != "unknown"
            or scope["installation_id"] != record["fixture"]
            or scope["namespace"] != record["namespace"]
            or not isinstance(scope["endpoint"], str)
            or not scope["endpoint"]
            or not isinstance(databases, list)
            or not databases
            or len(set(databases)) != len(databases)
            or any(_identifier(db) != db for db in databases)
            or record["database"] not in databases
            or original["id"] != record["id"]
            or original["native_drain"] != record["native_drain"]
            or original["native_drain"]["attachment"] != record["id"]
            or original["native_drain"]["directory"] != str(directory / "native-drain")
            or not hexadecimal(scope["original_generation"])
            or scope["original_generation"] != original["native_drain"]["generation"]
            or not generation(scope["generation"])
            or bytes(scope["generation"]).hex() == scope["original_generation"]
            or type(scope["target"]["schema_version"]) is not int
            or scope["target"]["schema_version"] <= 0
            or not hexadecimal(scope["target"]["schema"])
            or record.get("command_cleanup") != {"status": "confirmed", "survivors": []}
        ):
            return False
        reconstructed = {
            key: value
            for key, value in record.items()
            if key not in (*SCOPE_LIFECYCLE_FIELDS, "scope_disposition")
        }
        if (
            set(original["lifecycle"]) - set(SCOPE_LIFECYCLE_FIELDS)
            or original["lifecycle"].get("released") is not False
        ):
            return False
        reconstructed.update(original["lifecycle"])
        if scope_record_identity(reconstructed) != original["record_identity"]:
            return False
        run = receipt["original_run"]
        if (
            not hexadecimal(run["record_identity"])
            or not isinstance(run["id"], str)
            or Path(run["path"]).name != run["id"]
            or run["native_outcome"] != "unknown"
            or run["termination"] not in {"completed", "cancelled", "interrupted"}
            or not isinstance(run["exit"], dict)
            or len(run["exit"]) != 1
            or not (type(run["exit"].get("code")) is int or type(run["exit"].get("signal")) is int)
            or run["cleanup"]["status"] != "confirmed"
            or run["cleanup"]["survivors"] != []
        ):
            return False
        proof = receipt["proofs"]

        def decoded(blob):
            data = base64.b64decode(blob["bytes"], validate=True)
            if hashlib.sha256(data).hexdigest() != blob["sha256"]:
                raise ValueError("scope disposition proof digest changed")
            return json.loads(data)

        joined = proof["joined"]
        state, owner = joined["state"], joined["owner"]
        if (
            not hexadecimal(joined["reference"]["sha256"])
            or not hexadecimal(joined["journal_sha256"])
            or state.get("InvocationID") != owner["invocation"]
            or not isinstance(owner["unit"], str)
            or not owner["unit"]
            or type(owner["process"]["pid"]) is not int
            or owner["process"]["pid"] <= 0
            or any(
                state.get(key) != value
                for key, value in {
                    "MainPID": "0",
                    "ActiveState": "inactive",
                    "Result": "success",
                    "ExecMainCode": "1",
                    "ExecMainStatus": "0",
                }.items()
            )
        ):
            return False
        removals = proof["original_removals"]
        if set(removals) != set(databases) or any(
            decoded(removals[db]) != {"database": db, "removed": True, "join": joined["reference"]}
            for db in databases
        ):
            return False
        checked, complete = decoded(proof["checked"]), decoded(proof["complete"])
        expected = {
            "installation_id": scope["installation_id"],
            "generation": scope["generation"],
            "target": scope["target"],
            "databases": databases,
            "original_native_outcome": "unknown",
            "disposition": "explicitly discarded and regenerated",
            "plan": proof["plan"],
            "joined": joined["reference"],
        }
        if "initialization_revision" in proof:
            return False
        revisions = proof.get("initialization_revisions", [])
        if not isinstance(revisions, list):
            return False
        predecessor, previous_generation = proof["plan"], None
        seen = set()
        for revision in revisions:
            if (
                set(revision)
                != {
                    "reference",
                    "predecessor",
                    "discarded_generation",
                    "generation",
                    "target",
                    "removals",
                }
                or revision["predecessor"] != predecessor
                or set(revision["reference"]) != {"path", "sha256"}
                or not isinstance(revision["reference"]["path"], str)
                or not hexadecimal(revision["reference"]["sha256"])
                or revision["reference"]["sha256"] in seen
                or not generation(revision["discarded_generation"])
                or not generation(revision["generation"])
                or revision["generation"] == revision["discarded_generation"]
                or (
                    previous_generation is not None
                    and revision["discarded_generation"] != previous_generation
                )
                or set(revision["removals"]) != set(databases)
                or any(
                    decoded(revision["removals"][db])
                    != {
                        "database": db,
                        "removed": True,
                        "revision": revision["reference"],
                        "discarded_generation": revision["discarded_generation"],
                    }
                    for db in databases
                )
            ):
                return False
            seen.add(revision["reference"]["sha256"])
            predecessor, previous_generation = revision["reference"], revision["generation"]
        if revisions:
            expected["initialization_revision"] = predecessor
            if (
                revisions[-1]["generation"] != scope["generation"]
                or revisions[-1]["target"] != scope["target"]
            ):
                return False
        return (
            hexadecimal(proof["plan"]["sha256"])
            and checked == expected
            and complete == {**checked, "admission_open": True}
        )
    except OSError, ValueError, KeyError, TypeError, AttributeError:
        return False


CAMPAIGN_RELEASE_FIELDS = ("released", "released_at", "campaign_disposition")


def completed_campaign_disposition(directory: Path, record: Mapping[str, Any] | None) -> bool:
    """Accept the current owned target's discard, without rewriting native finality.

    The service owner produces this local acknowledgement only after authenticated
    whole-executor shutdown and datastore disposal. This reader has no effects.
    """
    try:
        if (
            not isinstance(record, dict)
            or record.get("released") is not True
            or record.get("command_cleanup") != {"status": "confirmed", "survivors": []}
        ):
            return False
        path = directory / "campaign-disposition.json"
        if (
            directory.name != record["id"]
            or directory.parent.name != "attachments"
            or any(p.is_symlink() for p in (path, *path.parents))
        ):
            return False
        fd = os.open(path, os.O_RDONLY | os.O_CLOEXEC | os.O_NOFOLLOW)
        try:
            before = os.fstat(fd)
            if (
                not stat.S_ISREG(before.st_mode)
                or before.st_uid != os.getuid()
                or stat.S_IMODE(before.st_mode) not in (0o400, 0o600)
            ):
                return False
            with os.fdopen(fd, "rb", closefd=False) as stream:
                raw = stream.read()
            if service.executable_identity(before) != service.executable_identity(os.fstat(fd)):
                return False
        finally:
            os.close(fd)
        if record.get("campaign_disposition") != {
            "path": str(path),
            "sha256": hashlib.sha256(raw).hexdigest(),
        }:
            return False
        receipt = json.loads(raw)
        scope, original, disposal = (
            receipt["scope"],
            receipt["original_attachment"],
            receipt["disposal"],
        )
        native = record["native_drain"]
        if (
            receipt.get("schema") != 1
            or receipt.get("kind") != "lifecycle-campaign-attachment-disposition"
            or receipt.get("outcome") != "discarded"
            or receipt.get("local_owner") != scope_local_owner(directory)
            or scope
            != {
                "installation_id": record["fixture"],
                "attachment": record["id"],
                "generation": native["generation"],
            }
            or not isinstance(native["generation"], str)
            or not re.fullmatch("[a-f0-9]{64}", native["generation"])
            or native["attachment"] != record["id"]
            or native["directory"] != str(directory / "native-drain")
            or original["native_drain"] != native
            or disposal.get("schema") != 1
            or disposal.get("kind") != "lifecycle-campaign-disposal"
            or disposal.get("installation_id") != record["fixture"]
            or disposal.get("generation") != native["generation"]
            or disposal.get("outcome") != "passed"
            or disposal.get("steps", {}).get("executor-shutdown") != "passed"
            or disposal.get("steps", {}).get("datastore-disposal") != "passed"
        ):
            return False
        lifecycle = original["lifecycle"]
        if set(lifecycle) - {"released", "released_at"} or lifecycle.get("released") is not False:
            return False
        reconstructed = {
            key: value for key, value in record.items() if key not in CAMPAIGN_RELEASE_FIELDS
        }
        reconstructed.update(lifecycle)
        return scope_record_identity(reconstructed) == original["record_identity"]
    except OSError, ValueError, KeyError, TypeError, AttributeError:
        return False


def acknowledge_campaign_discard(directory: Path, disposal: Mapping[str, Any]) -> dict[str, Any]:
    """Called by the service disposer under its exclusive target admission, after discard.

    Publish one local attachment acknowledgement and release its declared host
    obligation. Native receipts are retained verbatim with their original outcome.
    """
    if (
        not directory.is_absolute()
        or directory.parent.name != "attachments"
        or any(path.is_symlink() for path in (directory, *directory.parents))
    ):
        raise FixtureBlocked("identity", "campaign attachment must have an exact physical path")
    installation = read_json(directory.parent.parent / "installation.json")
    row = read_json(directory / "record.json")
    if (
        not isinstance(installation, dict)
        or not isinstance(row, dict)
        or installation.get("kind") != "campaign"
        or installation.get("purpose") != service.LIFECYCLE_PURPOSE
        or directory.name != row.get("id")
        or installation.get("installation_id") != row.get("fixture")
        or bytes(installation.get("service_generation", [])).hex()
        != row.get("native_drain", {}).get("generation")
    ):
        raise FixtureBlocked(
            "identity", "discard acknowledgement requires the exact lifecycle campaign"
        )
    if owner_alive(row.get("owner"), directory / "owner.lock"):
        raise FixtureBlocked("ownership", "campaign attachment still has a live owner")
    lock = try_hold_lock(directory / "owner.lock")
    if lock is None:
        raise FixtureBlocked("ownership", "campaign attachment acquired a live owner")
    try:
        if read_json(directory / "record.json") != row:
            raise FixtureBlocked("identity", "campaign attachment changed before acknowledgement")
        if completed_campaign_disposition(directory, row):
            _complete_attachment_obligations(
                directory, "confirmed whole campaign discard", owner_lease=lock
            )
            return row
        if (
            row.get("released") is not False
            or row.get("command_cleanup") != {"status": "confirmed", "survivors": []}
            or disposal.get("schema") != 1
            or disposal.get("kind") != "lifecycle-campaign-disposal"
            or disposal.get("installation_id") != row["fixture"]
            or disposal.get("generation") != row["native_drain"]["generation"]
            or disposal.get("outcome") != "passed"
            or disposal.get("steps", {}).get("executor-shutdown") != "passed"
            or disposal.get("steps", {}).get("datastore-disposal") != "passed"
        ):
            raise FixtureBlocked(
                "disposal", "campaign shutdown/discard or local command cleanup is incomplete"
            )
        receipt = {
            "schema": 1,
            "kind": "lifecycle-campaign-attachment-disposition",
            "outcome": "discarded",
            "local_owner": scope_local_owner(directory),
            "scope": {
                "installation_id": row["fixture"],
                "attachment": row["id"],
                "generation": row["native_drain"]["generation"],
            },
            "original_attachment": {
                "record_identity": scope_record_identity(row),
                "lifecycle": {key: row[key] for key in ("released", "released_at") if key in row},
                "native_drain": row["native_drain"],
            },
            "disposal": dict(disposal),
        }
        raw = json.dumps(receipt, sort_keys=True, separators=(",", ":")).encode()
        path = directory / "campaign-disposition.json"
        try:
            fd = os.open(
                path, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_CLOEXEC | os.O_NOFOLLOW, 0o600
            )
        except FileExistsError:
            if path.is_symlink() or path.read_bytes() != raw:
                raise FixtureBlocked(
                    "identity", "campaign attachment disposition already differs"
                ) from None
        else:
            with os.fdopen(fd, "wb") as stream:
                stream.write(raw)
                stream.flush()
                os.fchmod(stream.fileno(), 0o400)
                os.fsync(stream.fileno())
            parent = os.open(directory, os.O_RDONLY | os.O_DIRECTORY | os.O_CLOEXEC)
            try:
                os.fsync(parent)
            finally:
                os.close(parent)
        candidate = {
            **row,
            "released": True,
            "released_at": now(),
            "campaign_disposition": {"path": str(path), "sha256": hashlib.sha256(raw).hexdigest()},
        }
        if not completed_campaign_disposition(directory, candidate):
            raise FixtureBlocked("identity", "campaign attachment disposition validation failed")
        write_json_atomic(directory / "record.json", candidate)
        _complete_attachment_obligations(
            directory, "confirmed whole campaign discard", owner_lease=lock
        )
        return candidate
    finally:
        os.close(lock)


def native_protected(
    directory: Path, record: Mapping[str, Any] | None, native: Mapping[str, Any]
) -> bool:
    if not _native_pending(record, native):
        return False
    if completed_campaign_disposition(directory, record) or completed_scope_disposition(
        directory, record
    ):
        return False
    from surrealdb_pd02_recovery import recovered

    return not recovered(directory, record)


def _native_pending(record: Mapping[str, Any] | None, native: Mapping[str, Any]) -> bool:
    code = (record or {}).get("child_exit_code")
    return native["status"] == "unknown" or (
        native["status"] == "not_observed"
        and (bool((record or {}).get("interrupted")) or (isinstance(code, int) and code < 0))
    )


def _recover_attachment(path: Path, record: dict[str, Any]) -> dict[str, Any]:
    """Explicit selected recovery: revalidate historical group ownership before signalling."""
    observation = _command_observation(record)
    if observation["status"] == "gone":
        cleanup = {"status": "confirmed", "survivors": []}
    elif observation["status"] == "owned":
        cleanup = cleanup_group(int(record["command"]["leader"]["pid"]), grace=STOP_GRACE)
    else:
        cleanup = {
            "status": "unknown",
            "reason": observation.get("reason", "identity unavailable"),
            "members": (record.get("command_cleanup") or {}).get("members", []),
        }
    write_json_atomic(path / "record.json", {**record, "command_cleanup": cleanup})
    return cleanup


def substrate_readiness(
    requirement: str = "native-store",
    env: Mapping[str, str] | None = None,
    *,
    runner=subprocess.run,
) -> Readiness:
    """Observe installed state; never provisions or synchronizes."""
    try:
        installation = Installation.load(env)
        installation.check(allow_maintenance=service.maintenance_owned(installation))
        server = _server_identity(installation)
        detail = f"installed {installation.id} {server['version']} sha256={server['binary_sha256']}"
        if generation := server.get("generation"):
            detail += f" generation={generation}"
        return Readiness(requirement, True, detail)
    except FixtureBlocked as error:
        return Readiness(requirement, False, error.detail, error.repair)


@dataclass
class Server:
    installation: Installation
    report: Callable[[str], None] = field(default=_stderr, repr=False)
    _cleanup_protected: bool = field(default=False, repr=False)

    @classmethod
    def open(
        cls, installation_id: str | None = None, env: Mapping[str, str] | None = None
    ) -> Server:
        installation = Installation.load(env)
        if installation_id and installation_id not in ("installed", installation.id):
            raise FixtureBlocked(
                "identity", "requested installation ID does not match stable service"
            )
        return cls(installation)

    @property
    def directory(self) -> Path:
        return self.installation.directory

    @property
    def record(self) -> dict[str, Any]:
        return self.installation.record

    @property
    def id(self) -> str:
        return self.installation.id

    @property
    def kind(self) -> str:
        return "persistent"

    @property
    def unit(self) -> str:
        return self.installation.unit

    @property
    def endpoint(self) -> str:
        return self.installation.endpoint

    @property
    def grpc_endpoint(self) -> str:
        return self.installation.grpc_endpoint

    @property
    def password(self) -> str:
        return self.installation.runtime()["password"]

    def ready(self) -> None:
        self.installation.check(allow_maintenance=True)

    def exited(self) -> str | None:
        state = unit_properties(self.unit, "ActiveState", "Result")
        if state.get("ActiveState") == "active":
            return None
        return (
            "oom-kill"
            if state.get("Result") == "oom-kill"
            else "result resources"
            if state.get("Result") == "resources"
            else "server inactive"
        )

    def oom(self, when: str) -> FixtureBlocked:
        return FixtureBlocked(
            "oom",
            f"owned service OOM-killed {when}",
            "inspect service/host memory pressure; repair production/test allocation",
        )

    def restart(self, env: Mapping[str, str] | None = None) -> None:
        if not service.maintenance_owned(self.installation, env):
            raise FixtureBlocked(
                "maintenance",
                "restart requires drained explicit maintenance",
                "just service maintenance --restart",
            )
        if systemctl("restart", self.unit).returncode:
            raise FixtureBlocked("launch", "explicit owned-service restart failed")
        deadline = time.monotonic() + service.READY_TIMEOUT
        while True:
            try:
                self.ready()
                return
            except FixtureBlocked:
                if time.monotonic() >= deadline:
                    raise
                time.sleep(0.1)

    def request(
        self, route: str, data: bytes | None, namespace: str | None, database: str | None
    ) -> urllib.request.Request:
        if namespace not in (None, service.NAMESPACE) or database not in (None, "validation"):
            raise FixtureBlocked("scope", "validation credentials cannot target canonical content")
        cfg = self.installation.runtime()
        credentials = f"{cfg['username']}:{cfg['password']}"
        headers = {
            "Authorization": "Basic " + base64.b64encode(credentials.encode()).decode(),
            "Accept": "application/json",
            "Content-Type": "text/plain",
            "Surreal-NS": service.NAMESPACE,
            "Surreal-DB": "validation",
            "Surreal-Auth-NS": service.NAMESPACE,
            "Surreal-Auth-DB": "validation",
        }
        return urllib.request.Request(self.endpoint + route, data=data, headers=headers)

    def query(
        self, sql: str, *, namespace: str | None = None, database: str | None = None
    ) -> list[dict[str, Any]]:
        diagnostic = self.diagnose(sql, namespace=namespace, database=database)
        if diagnostic["outcome"] != "passed":
            raise FixtureQueryError(diagnostic)
        return diagnostic["statements"]

    def diagnose(
        self, sql: str, *, namespace: str | None = None, database: str | None = None
    ) -> dict[str, Any]:
        if not service.maintenance_owned(self.installation):
            raise FixtureBlocked(
                "maintenance",
                "arbitrary SQL requires validation maintenance",
                "just service maintenance -- just fixture --sql FILE",
            )
        return self._diagnose(sql, namespace=namespace, database=database)

    def _diagnose(
        self, sql: str, *, namespace: str | None = None, database: str | None = None
    ) -> dict[str, Any]:
        """Execute explicitly effectful SQL, preserving indexed outcomes, not error text.

        Successful values are intentional query output. Causes/categories never contain the
        query, native messages, response headers, credentials or bindings.
        """
        transport: dict[str, Any] = {"status": "OK", "category": "http", "http_status": 200}
        statements: list[dict[str, Any]] = []
        try:
            request = self.request("/sql", sql.encode(), namespace, database)
            with service.local_urlopen(request, timeout=25) as response:
                transport["http_status"] = response.status
                result = json.load(response)
            if not isinstance(result, list) or not result:
                raise ValueError("missing statement envelopes")
            for index, row in enumerate(result):
                if not isinstance(row, dict) or row.get("status") not in ("OK", "ERR"):
                    raise ValueError("invalid statement envelope")
                ok = row["status"] == "OK"
                statement: dict[str, Any] = {
                    "index": index,
                    "status": row["status"],
                    "category": "success" if ok else "statement_refused",
                }
                if ok:
                    statement["result"] = row.get("result")
                statements.append(statement)
        except urllib.error.HTTPError as error:
            transport = {"status": "ERR", "category": "http_refused", "http_status": error.code}
            error.close()
        except OSError, http.client.HTTPException:
            transport = {"status": "ERR", "category": "connection"}
        except ValueError:
            transport = {"status": "ERR", "category": "invalid_envelope"}
        passed = transport["status"] == "OK" and all(r["status"] == "OK" for r in statements)
        return {
            "outcome": "passed" if passed else "failed",
            "transport": transport,
            "statements": statements,
        }


@dataclass
class Attachment:
    server: Server
    id: str
    directory: Path
    config: dict[str, Any] = field(repr=False)
    extra_env: dict[str, str] = field(default_factory=dict)
    _lock: int | None = field(default=None, repr=False)
    _borrow: Any = field(default=None, repr=False)
    _generation: Any = field(default=None, repr=False)
    _command_cleanup: dict[str, Any] | None = field(default=None, repr=False)
    _unreaped_child: subprocess.Popen | None = field(default=None, repr=False)
    _retaining: Path | None = field(default=None, repr=False)
    _storage_admission: Any = field(default=None, repr=False)
    _storage_id: str | None = field(default=None, repr=False)

    @property
    def scratch(self) -> Path:
        return self.directory / "scratch"

    @property
    def config_path(self) -> Path:
        return self.directory / "runtime.json"

    @property
    def compiler_config_path(self) -> Path:
        return self.directory / "compiler-runtime.json"

    @property
    def endpoint(self) -> str:
        return self.server.endpoint

    @property
    def namespace(self) -> str:
        return service.NAMESPACE

    @classmethod
    def create(
        cls, server: Server, *, maintenance_env: Mapping[str, str] | None = None
    ) -> Attachment:
        # The manager supplies a maintenance proof only after exclusive drainage. No arbitrary
        # environment flag or copied test JSON confers maintenance permission.
        import storage_owners
        from storage_lifecycle import admission

        attachment_id = str(uuid.uuid4())
        directory = server.directory / "attachments" / attachment_id
        storage = admission([directory])
        storage.__enter__()
        ownership = None
        lock = None
        borrowed = False
        try:
            maintenance = service.maintenance_owned(server.installation, maintenance_env)
            ownership = (
                contextlib.nullcontext() if maintenance else service.borrow(server.installation)
            )
            generation = ownership.__enter__()
            borrowed = True
            directory.mkdir(mode=0o700)
            lock = hold_lock(directory / "owner.lock")
            (directory / "scratch").mkdir(mode=0o700)
            (directory / "native-drain").mkdir(mode=0o700)
            (directory / "pd02-recovery").mkdir(mode=0o700)
            cfg = server.installation.runtime()
            config = {
                "scratch": str(directory / "scratch"),
                "fixture": server.id,
                "attachment": attachment_id,
                "attempt_id": attachment_id,
                "endpoint": server.endpoint,
                "grpc_endpoint": server.grpc_endpoint,
                "namespace": service.NAMESPACE,
                "database": "validation",
                "authentication": "database",
                "admin_user": cfg["username"],
                "admin_password": cfg["password"],
                "installation_id": server.id,
                "service_generation": server.record["service_generation"],
                "server": _server_identity(server.installation),
            }
            attachment = cls(
                server,
                attachment_id,
                directory,
                config,
                _lock=lock,
                _borrow=ownership,
                _generation=generation,
            )
            if maintenance_env:
                attachment.extra_env["LCTX_SURREAL_MAINTENANCE_TOKEN"] = maintenance_env[
                    "LCTX_SURREAL_MAINTENANCE_TOKEN"
                ]
            service.private_json(attachment.config_path, config)
            service.private_json(
                attachment.compiler_config_path,
                {**cfg, "selection": str(directory / "scratch/selected.json")},
            )
            attachment._write_record(
                None,
                released=False,
                maintenance=maintenance,
                native_drain={
                    "directory": str(directory / "native-drain"),
                    "attachment": attachment_id,
                    "generation": bytes(server.record["service_generation"]).hex(),
                },
            )
            attachment._storage_admission = storage
            attachment._storage_id = storage_owners.enroll(
                directory,
                "attachment",
                {"kind": "attachment", "path": str(directory.resolve())},
                f"attachment:{attachment_id}",
                temporary_days=0,
            )
            return attachment
        except BaseException:
            if lock is not None:
                os.close(lock)
            if borrowed and ownership is not None:
                ownership.__exit__(*sys.exc_info())
            storage.__exit__(*sys.exc_info())
            raise

    def _write_record(self, command: dict[str, Any] | None, **extra: Any) -> None:
        record = read_json(self.directory / "record.json") or {
            "id": self.id,
            "fixture": self.server.id,
            "namespace": self.namespace,
            "database": "validation",
            "checkout": str(Path.cwd().resolve()),
            "owner": ProcessIdentity.of().to_json(),
            "created": now(),
            "released": False,
        }
        if command is not None:
            record["command"] = command
        record.update(extra)
        write_json_atomic(self.directory / "record.json", record)

    def environment(self, source: Mapping[str, str] | None = None) -> dict[str, str]:
        if self._generation is not None:
            self._generation.validate(self.server.installation)
        env = dict(os.environ if source is None else source)
        env.update(
            {
                "LCTX_SURREAL_TEST_CONFIG": str(self.config_path),
                "LCTX_COMPILER_RUNTIME_CONFIG": str(self.compiler_config_path),
                "LCTX_SURREAL_SERVICE_CONFIG": str(self.server.directory / "installation.json"),
                "LCTX_SURREAL_ATTEMPT_ID": self.id,
                "LCTX_SURREAL_INSTALLATION_ID": self.server.id,
                "LCTX_SURREAL_GENERATION": bytes(self.server.record["service_generation"]).hex(),
                "LCTX_FIXTURE_ID": self.server.id,
                "LCTX_FIXTURE_ATTACHMENT": self.id,
                "LCTX_NATIVE_DRAIN_DIRECTORY": str(self.directory / "native-drain"),
                "LCTX_PD02_RECOVERY_DIRECTORY": str(self.directory / "pd02-recovery"),
                **self.extra_env,
            }
        )
        return env

    def restart(self) -> None:
        """Transition from an ordinary producer to maintenance, then a new consumer.

        No native compiler bypass is granted during maintenance. The old local attempt and
        its persisted outputs remain recorded; only its borrower ownership is released.
        """
        server, extra_env = self.server, dict(self.extra_env)
        self.release()
        if (read_json(self.directory / "record.json") or {}).get("released") is not True:
            raise FixtureFailed("cleanup", "producer ownership did not drain before restart")
        with service.maintenance(server.installation) as maintenance_env:
            server.restart(maintenance_env)
        replacement = Attachment.create(server)
        self.__dict__.update(replacement.__dict__)
        self.extra_env.update(extra_env)

    def retain_serving(self, name: str) -> Path:
        directory = self.server.directory / "serving" / _identifier(name)
        try:
            directory.mkdir(mode=0o700)
        except FileExistsError:
            raise FixtureBlocked(
                "serving", "serving name already owned; choose another name"
            ) from None
        self._retaining = directory
        self._write_record(None, retaining_serving=name)
        path = directory / "viewer.json"
        self.extra_env["LCTX_RETAIN_NATIVE_FIXTURE_CONFIG"] = str(path)
        return path

    def abandon_serving(self) -> None:
        # Failure does not destroy content or another owner's outputs. An uncompleted receipt
        # remains visibly incomplete and cannot be reused.
        self._retaining = None

    def record_serving(self, name: str, command: Sequence[str]) -> dict[str, Any]:
        directory = self.server.directory / "serving" / _identifier(name)
        record = read_json(self.directory / "record.json") or {}
        if record.get("retaining_serving") != name:
            raise FixtureBlocked("ownership", "retained output belongs to another attempt")
        viewer = directory / "viewer.json"
        config = read_json(viewer)
        if not isinstance(config, dict):
            raise FixtureBlocked("serving", "producer did not write its serving configuration")
        selection = Path(config["selection"])
        handle = read_json(selection)
        if not isinstance(handle, dict):
            raise FixtureBlocked("serving", "producer did not write its handle")
        inputs = source_inputs(self.server.installation)
        identity = {
            "name": name,
            "fixture": self.server.id,
            "inputs": {**inputs, "command": list(command)},
            "configuration": {
                "path": str(viewer),
                "sha256": file_sha256(viewer),
                "selection": str(selection),
                "selection_sha256": file_sha256(selection),
                "server": inputs["server"],
            },
            "content": {
                "handle": handle,
                "semantic": handle.get("semantic"),
                "realization": handle.get("realization"),
                "database": handle.get("database"),
            },
            "producing_run": {
                "run_id": os.environ.get("LCTX_RUN_ID"),
                "attachment": self.id,
                "recorded": now(),
            },
        }
        service.private_json(directory / "identity.json", identity)
        self._retaining = None
        return identity

    def use_serving(self, name: str) -> dict[str, Any]:
        identity = serving_available(self.server, name)
        self.extra_env["LCTX_NATIVE_SERVING_CONFIG"] = identity["configuration"]["path"]
        current = source_inputs(self.server.installation)
        reuse = {
            "name": name,
            "content": identity["content"],
            "skipped_obligations": [
                {
                    "producer": identity["inputs"]["command"],
                    "produced_at": identity["inputs"],
                    "producing_run": identity["producing_run"],
                }
            ],
            "current_inputs": current,
            "inputs_changed_since_production": current
            != {key: identity["inputs"].get(key) for key in current},
        }
        self._write_record(None, serving_reuse=reuse)
        return reuse

    def release(self) -> None:
        record = read_json(self.directory / "record.json")
        unresolved = _cleanup_pending(record) or _command_observation(record)["status"] == "owned"
        native = native_completion(self.directory, record)
        self._write_record(None, native_completion=native)
        if _native_pending(record, native):
            unresolved = True
        if self._command_cleanup is not None and self._command_cleanup.get("status") != "confirmed":
            unresolved = True
        if not unresolved:
            self._write_record(None, released=True, released_at=now())
        else:
            self.server.report(f"fixture: attempt {self.id} cleanup unresolved; ownership retained")
        if self._lock is not None:
            os.close(self._lock)
            self._lock = None
        if self._borrow is not None:
            self._borrow.__exit__(None, None, None)
            self._borrow = None
        if self._storage_id is not None and not unresolved:
            import storage_owners

            storage_owners.complete(self._storage_id, f"attachment:{self.id}")
        if self._storage_admission is not None:
            self._storage_admission.__exit__(None, None, None)
            self._storage_admission = None


def serving_available(server: Server, name: str) -> dict[str, Any]:
    if (server.directory / "serving" / _identifier(name) / "retired.json").exists():
        raise FixtureBlocked("serving", "retained output was explicitly retired")
    identity = read_json(server.directory / "serving" / _identifier(name) / "identity.json")
    if not isinstance(identity, dict) or identity.get("fixture") != server.id:
        raise FixtureBlocked("serving", "no completed retained output on this installation")
    for key, digest in (("path", "sha256"), ("selection", "selection_sha256")):
        path = Path(identity["configuration"][key])
        if not path.is_file() or file_sha256(path) != identity["configuration"][digest]:
            raise FixtureBlocked("serving", "retained configuration changed or vanished")
    server.ready()
    # Native opening performs exact manifest/generation/admission checks. Availability is not
    # a cached test verdict, nor authority inferred from the mere presence of a database.
    return identity


def _serving_retirement_bytes(path: Path) -> bytes:
    if any(part.is_symlink() for part in (path, *path.parents)):
        raise FixtureBlocked("identity", "retained retirement path is not physical")
    descriptor = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_CLOEXEC)
    with os.fdopen(descriptor, "rb") as stream:
        info = os.fstat(stream.fileno())
        if not stat.S_ISREG(info.st_mode) or info.st_uid != os.getuid() or info.st_mode & 0o077:
            raise FixtureBlocked("identity", "retained retirement state must be private and owned")
        value = stream.read(1024 * 1024 + 1)
    if len(value) > 1024 * 1024:
        raise FixtureBlocked("identity", "retained retirement metadata exceeds its bound")
    return value


def retire_serving(name: str) -> dict[str, Any]:
    """Retire one named validation publication; host attachment release is separate.

    A durable plan precedes native effects. Interrupted native work keeps the actual
    attachment/maintenance owner; explicit recovery must resolve it before retry.
    Retrying this operation uses the same native job and never nominates a new root.
    """
    import hashlib
    import secrets

    from storage_lifecycle import admission, durable_json, fsync_directory

    installation = Installation.load()
    if installation.record.get("kind") != "campaign":
        raise FixtureBlocked(
            "scope", "retained retirement requires the explicitly selected campaign"
        )
    directory = installation.directory / "serving" / _identifier(name)
    with admission([directory, installation.runtime_path("validation")]):
        if not directory.is_dir() or directory.is_symlink():
            raise FixtureBlocked("serving", "retained output is unavailable")
        preflight = json.loads(_serving_retirement_bytes(directory / "identity.json"))
        if preflight.get("configuration", {}).get("selection") != str(
            (directory / "viewer.json").with_suffix(".selected.json")
        ):
            raise FixtureBlocked(
                "identity", "retained retirement requires the producer's exact selection path"
            )
        lock = try_hold_lock(directory / "retirement.lock")
        if lock is None:
            raise FixtureBlocked("ownership", "retained retirement already has a live owner")
        try:
            plan_path, native_path, retired_path = (
                directory / filename
                for filename in ("retirement-plan.json", "native-retirement.json", "retired.json")
            )
            identity_path = directory / "identity.json"
            identity_bytes = _serving_retirement_bytes(identity_path)
            identity = json.loads(identity_bytes)
            viewer_path = directory / "viewer.json"
            selection = Path(identity["configuration"]["selection"])
            handle = identity["content"]["handle"]
            if (
                identity.get("fixture") != installation.id
                or identity.get("name") != name
                or identity["configuration"]["path"] != str(viewer_path)
                or selection != viewer_path.with_suffix(".selected.json")
                or handle.get("database")
                != {"namespace": service.NAMESPACE, "database": "validation"}
                or handle.get("service_generation")
                != bytes(installation.record["service_generation"]).hex()
            ):
                raise FixtureBlocked(
                    "identity", "retained retirement is outside its exact validation scope"
                )
            runtime_path = directory / "retirement-runtime.json"
            handle_path = directory / "retirement-handle.json"
            runtime = {**installation.runtime("validation"), "selection": str(selection)}
            runtime_bytes = (json.dumps(runtime, sort_keys=True, indent=2) + "\n").encode()
            expected = {
                "schema": 1,
                "fixture": installation.id,
                "name": name,
                "identity_sha256": hashlib.sha256(identity_bytes).hexdigest(),
                "viewer_sha256": identity["configuration"]["sha256"],
                "selection": str(selection),
                "selection_sha256": identity["configuration"]["selection_sha256"],
                "handle": handle,
                "runtime_sha256": hashlib.sha256(runtime_bytes).hexdigest(),
            }
            if plan_path.exists():
                plan = json.loads(_serving_retirement_bytes(plan_path))
                if {key: value for key, value in plan.items() if key != "job"} != expected:
                    raise FixtureBlocked("identity", "retained retirement checkpoint changed")
                if not isinstance(plan.get("job"), str) or not re.fullmatch(
                    "[a-f0-9]{64}", plan["job"]
                ):
                    raise FixtureBlocked("identity", "retained retirement job is unavailable")
            else:
                viewer_bytes = _serving_retirement_bytes(viewer_path)
                viewer = json.loads(viewer_bytes)
                if (
                    hashlib.sha256(viewer_bytes).hexdigest() != expected["viewer_sha256"]
                    or viewer.get("selection") != str(selection)
                    or viewer.get("endpoint") != installation.grpc_endpoint
                    or hashlib.sha256(_serving_retirement_bytes(selection)).hexdigest()
                    != expected["selection_sha256"]
                    or json.loads(_serving_retirement_bytes(selection)) != handle
                ):
                    raise FixtureBlocked(
                        "identity", "retained viewer or exact selected handle changed"
                    )
                plan = {**expected, "job": secrets.token_hex(32)}
                durable_json(plan_path, plan)
            plan_sha256 = file_sha256(plan_path)
            if retired_path.exists():
                retired = json.loads(_serving_retirement_bytes(retired_path))
                if (
                    retired.get("outcome") != "retired"
                    or retired.get("plan_sha256") != plan_sha256
                    or retired.get("native_sha256") != file_sha256(native_path)
                    or viewer_path.exists()
                    or selection.exists()
                    or runtime_path.exists()
                    or handle_path.exists()
                ):
                    raise FixtureBlocked("identity", "completed retained retirement changed")
                if (installation.directory / "maintenance.json").exists():
                    raise FixtureBlocked(
                        "maintenance", "retained retirement needs explicit maintenance recovery"
                    )
                return retired

            with service.maintenance(installation, native_clients=True) as maintenance_env:
                if not native_path.exists():
                    viewer_bytes = _serving_retirement_bytes(viewer_path)
                    if hashlib.sha256(viewer_bytes).hexdigest() != expected["viewer_sha256"]:
                        raise FixtureBlocked(
                            "identity", "retained viewer changed before retirement"
                        )
                    if selection.exists() and (
                        hashlib.sha256(_serving_retirement_bytes(selection)).hexdigest()
                        != expected["selection_sha256"]
                        or json.loads(_serving_retirement_bytes(selection)) != handle
                    ):
                        raise FixtureBlocked(
                            "identity", "retained selected handle changed before retirement"
                        )
                    # Private runtime copies exist only while this exact owner needs them.
                    durable_json(runtime_path, runtime)
                    durable_json(handle_path, handle)
                    attachment = Attachment.create(
                        Server(installation), maintenance_env=maintenance_env
                    )
                    with admission([attachment.directory]):
                        output_path = directory / ("retirement-output-" + attachment.id + ".json")
                        command = [
                            installation.record["installer"],
                            "snapshot",
                            "--runtime-config",
                            str(runtime_path),
                            "retire",
                            str(handle_path),
                            "--readers-stopped",
                            "--clear-selection",
                            "--job",
                            plan["job"],
                        ]
                        durable_json(
                            directory / "retirement-attempt.json",
                            {
                                "plan_sha256": plan_sha256,
                                "attachment": attachment.id,
                                "output": str(output_path),
                                "command": command,
                            },
                        )
                        try:
                            descriptor = os.open(
                                output_path,
                                os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW,
                                0o600,
                            )
                            with os.fdopen(descriptor, "wb") as output:
                                proof = {
                                    key: maintenance_env[key]
                                    for key in (
                                        "LCTX_SURREAL_SERVICE_CONFIG",
                                        "LCTX_SURREAL_MAINTENANCE_TOKEN",
                                        "LCTX_SURREAL_INSTALLER_CONFIG",
                                    )
                                }
                                code = run_attached(
                                    attachment, command, child_env=proof, stdout=output
                                )
                                output.flush()
                                os.fsync(output.fileno())
                        finally:
                            attachment.release()
                        row = read_json(attachment.directory / "record.json")
                        native = native_completion(attachment.directory, row)
                        if (
                            code
                            or row.get("released") is not True
                            or native.get("status") != "confirmed"
                        ):
                            raise FixtureFailed(
                                "retirement",
                                "native retirement or actual owner finality remains unresolved",
                            )
                        progress = json.loads(_serving_retirement_bytes(output_path))
                        if (
                            set(progress)
                            != {"identity", "retired", "examined", "remaining", "retained"}
                            or progress["identity"] != plan["job"]
                            or progress["remaining"] != []
                            or type(progress["retired"]) is not int
                            or progress["retired"] <= 0
                            or type(progress["examined"]) is not int
                            or progress["examined"] < 0
                            or not isinstance(progress["retained"], list)
                            or any(not isinstance(item, str) for item in progress["retained"])
                        ):
                            raise FixtureFailed(
                                "retirement",
                                "native retirement did not acknowledge this completed job",
                            )
                        durable_json(
                            native_path,
                            {
                                "plan_sha256": plan_sha256,
                                "progress": progress,
                                "attachment": attachment.id,
                                "command_cleanup": row["command_cleanup"],
                                "native_completion": native,
                            },
                        )
                native = json.loads(_serving_retirement_bytes(native_path))
                if (
                    native.get("plan_sha256") != plan_sha256
                    or native["progress"]["identity"] != plan["job"]
                    or native["progress"]["remaining"] != []
                    or native["native_completion"].get("status") != "confirmed"
                    or native["command_cleanup"].get("status") != "confirmed"
                    or native["command_cleanup"].get("survivors")
                ):
                    raise FixtureBlocked("identity", "native retained retirement receipt changed")
                if selection.exists():
                    raise FixtureBlocked(
                        "identity", "retained selection reappeared; no other pointer is removed"
                    )
                for path, digest in (
                    (viewer_path, expected["viewer_sha256"]),
                    (runtime_path, expected["runtime_sha256"]),
                ):
                    if path.exists():
                        if hashlib.sha256(_serving_retirement_bytes(path)).hexdigest() != digest:
                            raise FixtureBlocked(
                                "identity", "retained runtime changed before host release"
                            )
                        path.unlink()
                if handle_path.exists():
                    if json.loads(_serving_retirement_bytes(handle_path)) != handle:
                        raise FixtureBlocked(
                            "identity", "retained handle copy changed before host release"
                        )
                    handle_path.unlink()
                fsync_directory(directory)
                producer_id = identity.get("producing_run", {}).get("attachment")
                if isinstance(producer_id, str) and re.fullmatch("[a-f0-9-]{36}", producer_id):
                    producer_directory = installation.directory / "attachments" / producer_id
                    producer = read_json(producer_directory / "record.json")
                    if isinstance(producer, dict):
                        if (
                            producer.get("fixture") != installation.id
                            or producer.get("released") is not True
                            or native_protected(
                                producer_directory,
                                producer,
                                native_completion(producer_directory, producer),
                            )
                        ):
                            raise FixtureBlocked(
                                "ownership", "retained producer local ownership remains unresolved"
                            )
                        _complete_attachment_obligations(
                            producer_directory, "exact retained native publication retired"
                        )
                retired = {
                    "outcome": "retired",
                    "name": name,
                    "fixture": installation.id,
                    "plan_sha256": plan_sha256,
                    "native_sha256": file_sha256(native_path),
                    "job": plan["job"],
                    "publication": handle["publication"],
                    "retired_at": now(),
                }
                durable_json(retired_path, retired)
            return retired
        finally:
            os.close(lock)


def inspection_scope(server: Server, configuration: Path) -> tuple[str, str]:
    path = configuration.resolve()
    if not path.is_relative_to(server.directory / "attachments"):
        raise FixtureBlocked(
            "scope", "diagnostic configuration must be an owned validation attempt"
        )
    row = read_json(path.parent / "record.json")
    config = read_json(path)
    if not row or row.get("released") or not lock_held(path.parent / "owner.lock") or not config:
        raise FixtureBlocked("scope", "selected attempt is not live")
    if (
        config.get("namespace") != service.NAMESPACE
        or config.get("database") != "validation"
        or config.get("fixture") != server.id
    ):
        raise FixtureBlocked(
            "scope", "configuration does not identify this validation installation"
        )
    return service.NAMESPACE, "validation"


MCP_AUTH_ENV = "LCTX_FIXTURE_MCP_AUTH"


def mcp_invocation(
    attachment: Attachment, arguments: Sequence[str], *, non_sensitive: bool = False
) -> tuple[list[str], dict[str, str]]:
    if not non_sensitive or not service.maintenance_owned(attachment.server.installation):
        raise FixtureBlocked(
            "mcp", "native MCP requires synthetic/non-sensitive validation maintenance"
        )
    namespace, database = inspection_scope(attachment.server, attachment.config_path)
    command = [
        "codex",
        "--no-daemon",
        "-c",
        "mcp_servers.lctx_fixture.url=" + json.dumps(attachment.endpoint + "/mcp"),
        "-c",
        "mcp_servers.lctx_fixture.env_http_headers={Authorization="
        + json.dumps(MCP_AUTH_ENV)
        + "}",
        "-c",
        "mcp_servers.lctx_fixture.http_headers={Surreal-NS="
        + json.dumps(namespace)
        + ",Surreal-DB="
        + json.dumps(database)
        + ",Surreal-Auth-NS="
        + json.dumps(namespace)
        + ",Surreal-Auth-DB="
        + json.dumps(database)
        + "}",
        *arguments,
    ]
    cfg = attachment.server.installation.runtime()
    credentials = f"{cfg['username']}:{cfg['password']}"
    return command, {MCP_AUTH_ENV: "Basic " + base64.b64encode(credentials.encode()).decode()}


@contextlib.contextmanager
def fixture(*, report: Callable[[str], None] = _stderr) -> Iterator[Attachment]:
    server = Server.open()
    server.report = report
    attachment = Attachment.create(server)
    try:
        yield attachment
    finally:
        attachment.release()


def inventory(env: Mapping[str, str] | None = None) -> list[dict[str, Any]]:
    env = os.environ if env is None else env
    try:
        installation = Installation.load(env, ready=False)
    except FixtureBlocked:
        if not (service.state_root(env) / "installation.json").exists():
            return []
        raise
    checkout = env.get("LCTX_FIXTURE_CHECKOUT")
    rows = []
    for path in sorted((installation.directory / "attachments").glob("*/record.json")):
        row = read_json(path)
        if row is None:
            rows.append(
                {
                    "id": path.parent.name,
                    "kind": "attachment",
                    "protected": True,
                    "state": "unknown",
                }
            )
            continue
        if checkout and row.get("checkout") != str(Path(checkout).resolve()):
            continue
        observation = _command_observation(row)
        native = native_completion(path.parent, row)
        protected = row.get("released") is not True or native_protected(path.parent, row, native)
        rows.append(
            {
                **{k: row.get(k) for k in ("id", "checkout", "owner", "fixture")},
                "kind": "attachment",
                "state": "owned"
                if lock_held(path.parent / "owner.lock")
                else observation["status"],
                "protected": protected,
                "cleanup": "unresolved" if protected else "confirmed",
                "native_completion": native,
                "host_recovery": row.get("host_recovery"),
                "unit": installation.unit,
                "owner_alive": owner_alive(row.get("owner"), path.parent / "owner.lock"),
            }
        )
    return rows


def _recover_local(installation: Installation, directory: Path) -> None:
    row = read_json(directory / "record.json")
    if not row or row.get("fixture") != installation.id:
        raise FixtureBlocked("identity", "unknown attempt")
    if owner_alive(row.get("owner"), directory / "owner.lock"):
        raise FixtureBlocked("ownership", "attempt still has a live owner; cancel its owned run")
    lock = try_hold_lock(directory / "owner.lock")
    if lock is None:
        raise FixtureBlocked("ownership", "attempt acquired a live owner; recovery refused")
    try:
        current = read_json(directory / "record.json")
        if current != row:
            raise FixtureBlocked("identity", "attempt receipt changed during recovery; reobserve")
        cleanup = _recover_attachment(directory, row)
        if cleanup.get("status") != "confirmed":
            raise FixtureFailed("cleanup", "command cleanup is unresolved")
        native = native_completion(directory, row)
        if native_protected(directory, row, native):
            raise FixtureFailed(
                "native",
                "native completion remains unknown; process cleanup cannot release it; "
                f"explicit service recovery: just service maintenance --recover-attachment {directory.name} "
                f"(PD02) or --recover-native-attachment {directory.name} (Started native store/workspace owners)",
            )
        # Local drainage is separate from native effects. The service manager invokes the
        # native installation drain before disruptive work and before reopening admission.
        write_json_atomic(
            directory / "record.json",
            {
                **(read_json(directory / "record.json") or {}),
                "released": True,
                "released_at": now(),
                "recovered_local_cleanup": True,
            },
        )
    finally:
        os.close(lock)


def recover(attachment_id: str) -> None:
    from storage_lifecycle import admission

    installation = Installation.load()
    try:
        valid = str(uuid.UUID(attachment_id)) == attachment_id
    except ValueError:
        valid = False
    if not valid:
        raise FixtureBlocked("identity", "invalid attempt identity")
    directory = installation.directory / "attachments" / attachment_id
    with admission([directory]):
        _recover_local(installation, directory)
        _complete_attachment_obligations(directory, "confirmed local attachment recovery")


def _complete_attachment_obligations(
    directory: Path, reason: str, *, owner_lease: int | None = None
) -> None:
    # Only declared local attachment obligations; never persisted native content.
    import storage_owners
    from storage_lifecycle import Storage

    for row in Storage().records():
        if row.get("retired_at") or row.get("owner", {}).get("kind") != "attachment":
            continue
        if row["owner"].get("path") != str(directory.resolve()):
            continue
        guard = (
            storage_owners.owner_guard(row, attachment_lease=owner_lease)
            if owner_lease is not None
            else contextlib.nullcontext()
        )
        with guard:
            for consumer, obligation in row["obligations"].items():
                if obligation.get("temporary_days") is not None:
                    if not storage_owners.complete(row["id"], consumer, reason=reason):
                        raise FixtureFailed(
                            "storage",
                            "declared attachment obligation completion remains unresolved",
                        )


class _Interrupted(Exception):
    def __init__(self, signum: int) -> None:
        super().__init__(signum)
        self.signum = signum


def _raise_interrupt(signum: int, _frame: Any) -> None:
    raise _Interrupted(signum)


TERMINATING = (signal.SIGTERM, signal.SIGINT, signal.SIGHUP)


def python_requirements(command: Sequence[str], requested: Sequence[str]) -> list[str]:
    """Which shared Python environment requirements the command uses (explicit or evident)."""
    if requested:
        return [item for item in requested if item != "none"]
    head = Path(command[0]).name if command else ""
    return ["native-python"] if head in ("uv", "python", "python3", "pytest") else []


def run_attached(
    attachment: Attachment,
    command: Sequence[str],
    *,
    requirements: Sequence[str] = (),
    cwd: Path | None = None,
    report: Callable[[str], None] = _stderr,
    child_env: Mapping[str, str] | None = None,
    stdout: Any = None,
) -> int:
    """Run the command in its own process group with this attachment's environment. A signal to
    the launcher stops the command's group; the caller tears the state down."""
    from build_environment import normalized_env

    previous_cleanup = getattr(attachment, "_command_cleanup", None)
    directory = getattr(attachment, "directory", None)
    previous_record = read_json(directory / "record.json") if directory is not None else None
    if (previous_cleanup is not None and previous_cleanup.get("status") != "confirmed") or (
        previous_record
        and (
            _cleanup_pending(previous_record)
            or _native_pending(previous_record, native_completion(directory, previous_record))
            or _command_observation(previous_record)["status"] == "owned"
        )
    ):
        raise FixtureFailed(
            "cleanup",
            "prior command cleanup unresolved; select fixture recovery before another command",
        )
    stack = contextlib.ExitStack()
    with stack:
        env = normalized_env(attachment.environment(), native_inputs=False)
        if requirements:
            import workspace_env

            owned = stack.enter_context(
                workspace_env.ownership(
                    "shared", requirements, command=shlex.join(command), report=report
                )
            )
            env = owned.environment(env)
        if child_env:
            env.update(child_env)
        intent = {"argv": list(command), "leader": None, "started": now()}
        attachment._write_record(
            intent, command_cleanup={"status": "unknown", "reason": "launch pending"}
        )
        try:
            child = spawn_group(
                command, death_signal=signal.SIGTERM, env=env, cwd=cwd, stdout=stdout
            )
        except OSError as error:
            attachment._command_cleanup = {"status": "confirmed", "survivors": []}
            attachment._write_record(
                None, command_cleanup=attachment._command_cleanup, child_exit_code=None
            )
            raise FixtureBlocked("launch", f"cannot start {command[0]!r}: {error}") from error
        guard = SpawnGuard(child, grace=STOP_GRACE / 2)
        attachment._command_cleanup = {"status": "unknown"}
        attachment._unreaped_child = child
        server = getattr(attachment, "server", None)
        if server is not None:
            server._cleanup_protected = True
        command_record = {
            "argv": list(command),
            "leader": {"pid": child.pid, "pgid": child.pid},
            "started": now(),
        }
        code: int | None = None
        try:
            with guard:
                command_record["leader"] = {
                    **ProcessIdentity.of(child.pid).to_json(),
                    "pgid": child.pid,
                }
                attachment._write_record(
                    command_record, command_cleanup=attachment._command_cleanup
                )
                try:
                    while True:
                        code = observe_exit(child)
                        if code is not None:
                            break
                        time.sleep(0.5)
                except _Interrupted as interrupted:
                    attachment._write_record(None, interrupted=interrupted.signum)
                    for sig in TERMINATING:
                        signal.signal(sig, signal.SIG_IGN)
                    raise
        finally:
            attachment._command_cleanup = guard.cleanup
            if guard.cleanup["status"] == "confirmed":
                attachment._unreaped_child = None
            if server is not None:
                server._cleanup_protected = guard.cleanup["status"] != "confirmed"
            updates = {
                "command_cleanup": guard.cleanup,
                "child_exit_code": code if code is not None else guard.returncode,
            }
            if directory is not None:
                updates["native_completion"] = native_completion(
                    directory, read_json(directory / "record.json")
                )
            if (
                guard.cleanup["status"] != "confirmed"
                and "start_ticks" not in command_record["leader"]
            ):
                with contextlib.suppress(OSError):
                    command_record["leader"] = {
                        **ProcessIdentity.of(child.pid).to_json(),
                        "pgid": child.pid,
                    }
            try:
                attachment._write_record(command_record, **updates)
            except Exception as error:
                if guard.cleanup["status"] != "confirmed":
                    # Preserve the initial receipt even when its higher-level publisher failed.
                    # In-memory protection remains if the filesystem also rejects this write.
                    with contextlib.suppress(Exception):
                        path = attachment.directory / "record.json"
                        write_json_atomic(
                            path, {**(read_json(path) or {}), "command": command_record, **updates}
                        )
                report(f"fixture: command cleanup receipt not written: {type(error).__name__}")
        if guard.cleanup["status"] != "confirmed":
            report(f"fixture: command cleanup {guard.cleanup['status']}")
            return code or 1
        if directory is not None and _native_pending(
            read_json(directory / "record.json"), updates["native_completion"]
        ):
            report(
                "fixture: native completion unknown; attachment retained independently of child exit"
            )
            return code or 1
        assert code is not None
        return code


def server_end_outcome(
    server: Server, child_exit_code: int | None, *, cancelled: bool = False
) -> dict[str, Any]:
    """Shared direct/verification policy, keeping the actual child result independently.

    An unexplained end is failed with unknown cause, never inferred OOM or a product defect.
    Only positive substrate evidence is blocked. Cancellation leaves unexecuted work not_run.
    """
    return _classify_server_end(server, server.exited(), child_exit_code, cancelled=cancelled)


def _classify_server_end(
    server: Server, ended: str | None, child_exit_code: int | None, *, cancelled: bool = False
) -> dict[str, Any]:
    """Apply policy to one captured observation, including during readiness."""
    result: dict[str, Any] = {
        "outcome": "not_run"
        if child_exit_code is None
        else "passed"
        if child_exit_code == 0
        else "failed",
        "exit_code": child_exit_code,
        "child_exit_code": child_exit_code,
        "server_end": ended,
        "cause": None,
    }
    if cancelled:
        result.update(outcome="not_run", termination="cancelled")
        return result
    if ended == "oom-kill":
        blocked = server.oom("while the command ran")
        result.update(
            outcome="blocked",
            exit_code=EXIT_BLOCKED,
            cause={"kind": "oom", "detail": blocked.detail, "repair": blocked.repair},
        )
    elif ended == "result resources":
        result.update(
            outcome="blocked",
            exit_code=EXIT_BLOCKED,
            cause={"kind": "infrastructure", "detail": "fixture unit could not allocate resources"},
        )
    elif ended is not None:
        result.update(
            outcome="failed",
            exit_code=child_exit_code or 1,
            cause={
                "kind": "server_end_unknown",
                "detail": "fixture server ended unexpectedly; cause unknown",
            },
        )
    return result


def _finish(server: Server, code: int, report: Callable[[str], None]) -> int:
    result = server_end_outcome(server, code)
    if result["cause"]:
        report(
            f"fixture: {result['outcome']}: {result['cause']['kind']}: "
            f"{result['cause']['detail']}; child exit={code}"
        )
    return result["exit_code"]


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(prog="just fixture", description=__doc__)
    parser.add_argument(
        "--attach", metavar="INSTALLATION", help="check exact installed service identity"
    )
    action = parser.add_mutually_exclusive_group()
    action.add_argument(
        "--list", action="store_true", help="list logical attempt ownership; never sweep"
    )
    action.add_argument(
        "--recover",
        metavar="ATTEMPT",
        help="recover a dead owner's exact command group; never delete DB data",
    )
    action.add_argument(
        "--retire-serving",
        metavar="NAME",
        help="retire only this named campaign validation publication after joined quiescence",
    )
    parser.add_argument("--json", action="store_true")
    parser.add_argument(
        "--requires",
        action="append",
        default=[],
        choices=("native-python", "tools", "native", "none"),
    )
    serving = parser.add_mutually_exclusive_group()
    serving.add_argument("--retain-serving", metavar="NAME")
    serving.add_argument("--serving", metavar="NAME")
    diagnostic = parser.add_mutually_exclusive_group()
    diagnostic.add_argument(
        "--sql", metavar="FILE|-", help="effectful synthetic validation SQL, maintenance only"
    )
    diagnostic.add_argument(
        "--mcp", action="store_true", help="native MCP, validation maintenance only"
    )
    parser.add_argument("--non-sensitive", action="store_true")
    parser.add_argument("command", nargs=argparse.REMAINDER)
    args = parser.parse_args(argv)
    command = args.command[1:] if args.command[:1] == ["--"] else args.command
    try:
        if args.list:
            rows = inventory()
            print(
                json.dumps(rows, indent=2)
                if args.json
                else "\n".join(
                    f"{r['id']} state={r['state']} protected={r['protected']}" for r in rows
                )
            )
            return 0
        if args.recover:
            recover(args.recover)
            return 0
        if args.retire_serving:
            if command or args.serving or args.retain_serving or args.sql or args.mcp:
                parser.error("--retire-serving requires only its exact retained name")
            for sig in TERMINATING:
                signal.signal(sig, _raise_interrupt)
            print(json.dumps(retire_serving(args.retire_serving), indent=2))
            return 0
        if not command and not args.sql:
            parser.error("provide a command after -- or select --sql")
        if args.sql and command:
            parser.error("--sql does not accept a command")
        server = Server.open(args.attach)
        for sig in TERMINATING:
            signal.signal(sig, _raise_interrupt)
        attachment = Attachment.create(server)
        try:
            if args.serving:
                attachment.use_serving(args.serving)
            if args.retain_serving:
                attachment.retain_serving(args.retain_serving)
            if args.sql:
                statement = sys.stdin.read() if args.sql == "-" else Path(args.sql).read_text()
                diagnostic = server.diagnose(
                    statement, namespace=service.NAMESPACE, database="validation"
                )
                print(json.dumps(diagnostic, indent=2))
                code = 0 if diagnostic["outcome"] == "passed" else 1
            else:
                child_env = None
                if args.mcp:
                    command, child_env = mcp_invocation(
                        attachment, command, non_sensitive=args.non_sensitive
                    )
                code = run_attached(
                    attachment,
                    command,
                    requirements=python_requirements(command, args.requires),
                    child_env=child_env,
                )
            code = _finish(server, code, _stderr)
            if args.retain_serving and code == 0:
                attachment.record_serving(args.retain_serving, command)
            return code
        finally:
            attachment.release()
    except FixtureBlocked as error:
        _stderr(error.message())
        return EXIT_BLOCKED
    except FixtureFailed as error:
        _stderr(error.message())
        return 1
    except _Interrupted as error:
        return 128 + error.signum


if __name__ == "__main__":
    raise SystemExit(main())
