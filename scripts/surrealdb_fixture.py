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
import shlex
import signal
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
    return {"head": _git("rev-parse", "HEAD"), "uncommitted_sha256": digest,
            "server": _server_identity(installation)}


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
        return service.UNIT

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
            ownership.__enter__()
            borrowed = True
            directory.mkdir(mode=0o700)
            lock = hold_lock(directory / "owner.lock")
            (directory / "scratch").mkdir(mode=0o700)
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
                server, attachment_id, directory, config, _lock=lock, _borrow=ownership
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
            attachment._write_record(None, released=False, maintenance=maintenance)
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
        protected = row.get("released") is not True
        rows.append(
            {
                **{k: row.get(k) for k in ("id", "checkout", "owner", "fixture")},
                "kind": "attachment",
                "state": "owned"
                if lock_held(path.parent / "owner.lock")
                else observation["status"],
                "protected": protected,
                "cleanup": "unresolved" if protected else "confirmed",
                "unit": service.UNIT,
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
    import storage_owners
    from storage_lifecycle import Storage, admission

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
        # Recovery activates only already-declared local obligations. It neither adopts an
        # unmanaged legacy attachment nor releases native persisted content.
        for row in Storage().records():
            if row.get("retired_at") or row.get("owner", {}).get("kind") != "attachment":
                continue
            if row["owner"].get("path") != str(directory.resolve()):
                continue
            for consumer, obligation in row["obligations"].items():
                if obligation.get("temporary_days") is not None:
                    storage_owners.complete(
                        row["id"], consumer, reason="confirmed local attachment recovery"
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
            child = spawn_group(command, death_signal=signal.SIGTERM, env=env, cwd=cwd)
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
                except _Interrupted:
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
