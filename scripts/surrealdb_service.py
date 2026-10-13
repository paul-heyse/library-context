#!/usr/bin/env python3
"""Durable, explicitly installed library-context SurrealDB user services.

Ordinary callers only observe and borrow the installation. Installation, schema changes,
restart and privileged validation diagnostics are explicit maintenance operations. State,
credentials and borrower records are shared by all checkouts outside the repository.
"""

from __future__ import annotations

import argparse
import base64
import contextlib
import contextvars
import copy
import fcntl
import hashlib
import io
import json
import os
import re
import secrets
import shutil
import socket
import stat
import subprocess
import sys
import tarfile
import tempfile
import time
import urllib.error
import urllib.parse
import urllib.request
import uuid
from collections.abc import Callable, Iterator, Mapping, Sequence
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from harness import ProcessIdentity, hold_lock, lock_held, read_json, write_json_atomic
from storage_lifecycle import Blocked as StorageBlocked

ROOT = Path(__file__).resolve().parent.parent
UNIT = "library-context-surrealdb.service"
NAMESPACE = "library_context"
DATABASES = ("main", "validation")
SCHEMA = 1
PRODUCT_CAPACITY_BYTES = 8 * 1024**3
EXPORT_BATCH_SIZE = 1
SERVER_SETTINGS = {
    "SURREAL_GRPC_MAX_MESSAGE_SIZE": "128MiB",
    "SURREAL_EXPORT_BATCH_SIZE": str(EXPORT_BATCH_SIZE),
}
LIFECYCLE_PURPOSE = "surrealdb-lifecycle"
# A remote client cannot enlarge this server-side ceiling. No outbound network,
# scripting, guests or experimental feature is enabled by the beta connector.
DEVELOPMENT_CAPABILITIES = [
    "--deny-all",
    "--allow-funcs",
    "--allow-rpc",
    "--allow-http",
    "--allow-arbitrary-query",
    "system,record",
    "--deny-net",
]
EXIT_BLOCKED = 75
READY_TIMEOUT = 40.0
# Best-effort logical cleanup shares the existing host readiness bound. Exact
# executor drainage/discard, rather than unbounded logical retries, settles this
# explicitly disposable target. This is no performance or throughput threshold.
DISPOSAL_LOGICAL_TIMEOUT = READY_TIMEOUT
INSTALL_REPAIR = "just service install --installer target/release/lctx"
VERSION = "surrealdb-3.3.0"
CLI_VERSION = "3.3.0 for linux on x86_64"
BINARY_SHA256 = "58ad479cbdd8b1926a636524d259ef309f029d8d9968fb88f350590f7f2b7bc5"
ARCHIVE_SHA256 = "44aeab565f7e7e39d2d0bf0583c8aae648babc91373c70b2658288d95bbbcd55"
RELEASE_URL = (
    "https://github.com/surrealdb/surrealdb/releases/download/v3.3.0/surreal-v3.3.0.linux-amd64.tgz"
)


class FixtureBlocked(RuntimeError):
    """An infrastructure failure with evidence (OOM, failed readiness, launch error) or a missing
    prerequisite. ``kind`` names the evidence; ``repair`` the route that fixes it, if any."""

    def __init__(self, kind: str, detail: str, repair: str | None = None) -> None:
        super().__init__(detail)
        self.kind = kind
        self.detail = detail
        self.repair = repair

    def message(self) -> str:
        repair = f"; repair: {self.repair}" if self.repair else ""
        return f"fixture: blocked: {self.kind}: {self.detail}{repair}"


class FixtureFailed(RuntimeError):
    """Failed native operation without evidence for infrastructure blocking."""

    def __init__(self, kind: str, detail: str) -> None:
        super().__init__(detail)
        self.kind, self.detail = kind, detail

    def message(self) -> str:
        return f"fixture: failed: {self.kind}: {self.detail}"


BINARY_REPAIR = (
    f"install the pinned binary: download {RELEASE_URL}, check its sha256 is {ARCHIVE_SHA256},"
    " extract `surreal` and set LCTX_SURREAL_BIN to it (binary sha256 "
    f"{BINARY_SHA256})"
)
SYSTEMD_REPAIR = "run from a login session with a user systemd manager (systemctl --user)"


def binary_candidates(env: Mapping[str, str] | None = None) -> list[Path]:
    env = os.environ if env is None else env
    explicit = env.get("LCTX_SURREAL_BIN")
    if explicit:
        return [Path(explicit).expanduser()]
    home = Path(env.get("HOME") or Path.home())
    return [
        home / ".local/share/pse-arrow/tools/surreal/v3.3.0/linux-amd64/surreal",
        home / ".surrealdb/surreal",
    ]


def file_sha256(path: Path) -> str:
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def _owned_server_generation(directory: Path, descriptor: Path) -> dict:
    import surrealdb_server

    try:
        generation = surrealdb_server.validate_descriptor(descriptor)
        surrealdb_server.validate_provenance(generation["provenance"], directory=directory)
        return generation
    except StorageBlocked as error:
        raise FixtureBlocked(
            "binary", "owned server generation cannot be verified: " + str(error)
        ) from None


def _generation_binary(generation: Mapping[str, Any]) -> dict:
    return {
        "path": generation["path"],
        "sha256": generation["sha256"],
        "version": generation["http_version"],
        "generation": dict(generation),
    }


def _server_binary_origin(installation: Installation) -> Path:
    """The executable's original owner remains authoritative when a campaign borrows it."""
    if installation.record.get("binary_origin") is not None:
        if (
            installation.record.get("kind") != "campaign"
            or installation.record.get("purpose") != LIFECYCLE_PURPOSE
        ):
            raise FixtureBlocked("identity", "borrowed executable is not a lifecycle campaign")
        return Path(installation.record["binary_origin"])
    return installation.directory


def _installed_binary(installation: Installation) -> Path:
    binary = installation.record["binary"]
    if generation := binary.get("generation"):
        origin = _server_binary_origin(installation)
        actual = _owned_server_generation(origin, Path(generation["descriptor_path"]))
        if binary != _generation_binary(actual):
            raise FixtureBlocked("identity", "installed server differs from its owned generation")
        return Path(actual["path"])
    if binary.get("sha256") != BINARY_SHA256 or binary.get("version") != VERSION:
        raise FixtureBlocked("binary", "installed server differs from the official pin")
    return verify_binary({**os.environ, "LCTX_SURREAL_BIN": binary["path"]})


Runner = Callable[..., subprocess.CompletedProcess]


def verify_binary(env: Mapping[str, str] | None = None, *, runner: Runner = subprocess.run) -> Path:
    """The first candidate whose sha256 and ``surreal version`` match the pin."""
    problems = []
    for path in binary_candidates(env):
        if not path.is_file():
            problems.append(f"{path}: missing")
            continue
        digest = file_sha256(path)
        if digest != BINARY_SHA256:
            problems.append(
                f"{path}: sha256 {digest[:12]}… is not the pinned {BINARY_SHA256[:12]}…"
            )
            continue
        try:
            result = runner(
                [str(path), "version"], capture_output=True, text=True, timeout=30, check=False
            )
        except OSError as error:
            problems.append(f"{path}: cannot run ({error})")
            continue
        reported = (result.stdout or "").strip()
        if result.returncode or reported != CLI_VERSION:
            problems.append(f"{path}: reports {reported or 'nothing'!r}")
            continue
        return path
    raise FixtureBlocked("binary", "; ".join(problems) or "no candidate", BINARY_REPAIR)


SYSTEMD_RUNTIME_ROOT = Path("/run/user")


def systemd_environment(source: Mapping[str, str] | None = None) -> dict[str, str]:
    """Fill absent login routing from the existing same-user runtime and bus only.

    This is local subprocess routing, not session creation or a change to global/parent state.
    Explicit runtime/bus selections are preserved, including an unavailable selected runtime.
    """
    env = dict(os.environ if source is None else source)
    runtime = (
        Path(env["XDG_RUNTIME_DIR"])
        if "XDG_RUNTIME_DIR" in env
        else SYSTEMD_RUNTIME_ROOT / str(os.getuid())
    )
    try:
        directory, bus = runtime.stat(), (runtime / "bus").stat()
    except OSError:
        return env
    if directory.st_uid != os.getuid() or not stat.S_ISDIR(directory.st_mode):
        return env
    if bus.st_uid != os.getuid() or not stat.S_ISSOCK(bus.st_mode):
        return env
    env.setdefault("XDG_RUNTIME_DIR", str(runtime))
    env.setdefault("DBUS_SESSION_BUS_ADDRESS", f"unix:path={runtime / 'bus'}")
    return env


def systemd_available(
    *, runner: Runner = subprocess.run, env: Mapping[str, str] | None = None
) -> str | None:
    """None when a user systemd manager answers, else why not."""
    try:
        result = runner(
            ["systemctl", "--user", "show-environment"],
            capture_output=True,
            text=True,
            timeout=15,
            check=False,
            env=systemd_environment(env),
        )
    except (OSError, subprocess.TimeoutExpired) as error:
        return f"cannot run systemctl --user ({error})"
    if result.returncode:
        return (result.stderr or "").strip() or f"systemctl --user exited {result.returncode}"
    return None


def state_root(env: Mapping[str, str] | None = None) -> Path:
    env = os.environ if env is None else env
    if env.get("LCTX_SURREAL_SERVICE_CONFIG"):
        return Path(env["LCTX_SURREAL_SERVICE_CONFIG"]).expanduser().resolve().parent
    home = Path(env.get("HOME") or Path.home())
    return Path(env.get("XDG_STATE_HOME") or home / ".local/state") / "library-context/surrealdb"


def private_json(path: Path, value: Any) -> None:
    write_json_atomic(path, value)
    path.chmod(0o600)


def storage_observation() -> dict[str, Any]:
    """Sanitized live dependencies, without database access or runtime credentials."""
    from storage_service import dependencies

    return dependencies(Installation.load(ready=False))


def systemctl(*args: str, runner: Runner = subprocess.run) -> subprocess.CompletedProcess:
    return runner(
        ["systemctl", "--user", *args],
        capture_output=True,
        text=True,
        check=False,
        env=systemd_environment(),
    )


def unit_properties(unit: str = UNIT, *names: str) -> dict[str, str]:
    result = systemctl("show", unit, *(f"--property={name}" for name in names))
    if result.returncode:
        raise FixtureBlocked("systemd", "cannot observe the owned service", SYSTEMD_REPAIR)
    return dict(line.split("=", 1) for line in result.stdout.splitlines() if "=" in line)


def installation_unit(record: Mapping[str, Any]) -> str:
    """Derive the only unit this descriptor may own; never accept an arbitrary override."""
    kind = record.get("kind", "stable")
    if kind == "stable":
        return UNIT
    if kind == "campaign":
        try:
            identity = str(uuid.UUID(record["installation_id"]))
        except KeyError, ValueError, TypeError, AttributeError:
            raise FixtureBlocked("identity", "campaign installation ID is invalid") from None
        return f"library-context-surrealdb-campaign-{identity}.service"
    raise FixtureBlocked("identity", "installation kind is incompatible")


def _descriptor(root: Path) -> dict[str, Any]:
    value = read_json(root / "installation.json")
    if not isinstance(value, dict):
        raise FixtureBlocked("installation", f"no installed service at {root}", INSTALL_REPAIR)
    expected = {
        "schema": SCHEMA,
        "unit": installation_unit(value),
        "namespace": NAMESPACE,
        "databases": list(DATABASES),
        "state_root": str(root.resolve()),
        "export_batch_size": EXPORT_BATCH_SIZE,
    }
    try:
        uuid.UUID(value["installation_id"])
        endpoint = re.fullmatch(r"http://127\.0\.0\.1:(\d+)", value["endpoint"])
        valid_endpoint = endpoint is not None and 0 < int(endpoint[1]) < 65536
        valid_endpoint = valid_endpoint and value["grpc_endpoint"] == value["endpoint"].replace(
            "http://", "grpc://", 1
        )
        valid_binary = (
            value["binary"]["sha256"] == BINARY_SHA256 and value["binary"]["version"] == VERSION
        )
        if generation := value["binary"].get("generation"):
            # Loading is structural observation. Executable proof belongs inside the
            # borrow/maintenance lifetime, after replacement protection is acquired.
            valid_binary = value["binary"] == _generation_binary(generation)
    except KeyError, ValueError, TypeError:
        valid_endpoint = valid_binary = False
    if (
        not valid_endpoint
        or not valid_binary
        or any(value.get(k) != v for k, v in expected.items())
    ):
        raise FixtureBlocked(
            "identity",
            "installation descriptor is incompatible",
            "just service maintenance --check",
        )
    if value.get("server_log_filter", "warn") not in {"warn", "warn,surrealdb::net=info"} or (
        value.get("server_log_filter", "warn") != "warn"
        and value.get("kind", "stable") != "campaign"
    ):
        raise FixtureBlocked("identity", "owned server log filter is incompatible")
    generation = value.get("service_generation")
    if (
        not isinstance(generation, list)
        or len(generation) != 32
        or any(type(x) is not int or not 0 <= x <= 255 for x in generation)
    ):
        raise FixtureBlocked(
            "identity", "invalid schema generation", "just service maintenance --check"
        )
    return value


@dataclass
class Installation:
    directory: Path
    record: dict[str, Any]

    @classmethod
    def load(cls, env: Mapping[str, str] | None = None, *, ready: bool = True) -> Installation:
        root = state_root(env)
        value = _descriptor(root)
        if ready and value.get("phase") != "ready":
            raise FixtureBlocked("installation", "installation is incomplete", INSTALL_REPAIR)
        return cls(root, value)

    @property
    def id(self) -> str:
        return self.record["installation_id"]

    @property
    def unit(self) -> str:
        expected = installation_unit(self.record)
        if self.record.get("unit") != expected:
            raise FixtureBlocked("identity", "installation unit differs from its checked identity")
        return expected

    @property
    def endpoint(self) -> str:
        return self.record["endpoint"]

    @property
    def grpc_endpoint(self) -> str:
        return self.record["grpc_endpoint"]

    def runtime_path(self, database: str = "validation", *, installer: bool = False) -> Path:
        if database not in DATABASES:
            raise FixtureBlocked("scope", "database is not a stable installation scope")
        return self.directory / f"{database}-{'installer' if installer else 'runtime'}.json"

    def runtime(self, database: str = "validation", *, installer: bool = False) -> dict[str, Any]:
        path = self.runtime_path(database, installer=installer)
        try:
            mode = path.stat()
            cfg = json.loads(path.read_text())
        except OSError, ValueError:
            raise FixtureBlocked(
                "configuration", "private runtime configuration is unavailable", INSTALL_REPAIR
            ) from None
        if mode.st_uid != os.getuid() or stat.S_IMODE(mode.st_mode) & 0o077:
            raise FixtureBlocked(
                "configuration",
                "runtime credentials must be private to their owner",
                "chmod 600 the owned runtime configuration",
            )
        if any(
            cfg.get(k) != v
            for k, v in {
                "endpoint": self.grpc_endpoint,
                "namespace": NAMESPACE,
                "database": database,
                "cache_database": database,
                "service_generation": self.record["service_generation"],
                "authentication": "root" if installer else "database",
                "reuse": {
                    "database": database,
                    "capacity_bytes": PRODUCT_CAPACITY_BYTES,
                    "lease_directory": str(self.directory / "product-leases" / database),
                },
            }.items()
        ):
            raise FixtureBlocked(
                "identity", "runtime configuration does not match installation", INSTALL_REPAIR
            )
        return cfg

    def check_daemon(self) -> None:
        """Prove the exact owned daemon before any initial/retried administrative query."""
        if checked := checked_generation(self):
            checked.validate(self)
            return
        try:
            selected_before = executable_identity(Path(self.record["binary"]["path"]).stat())
        except OSError:
            raise FixtureBlocked(
                "binary", "installed executable is unavailable", BINARY_REPAIR
            ) from None
        binary = _installed_binary(self)
        state = unit_properties(self.unit, "ActiveState", "Result", "MainPID", "ExecStart")
        if state.get("ActiveState") != "active":
            raise FixtureBlocked(
                "readiness",
                f"owned service is {state.get('ActiveState', 'unknown')}",
                "just service maintenance --restart",
            )
        try:
            process = ProcessIdentity.of(int(state["MainPID"]))
            exe = Path(f"/proc/{process.pid}/exe").resolve(strict=True)
            if exe != binary.resolve():
                raise FixtureBlocked("identity", "running service binary differs from installation")
            argv = Path(f"/proc/{int(state['MainPID'])}/cmdline").read_bytes().split(b"\0")
            environment = dict(
                item.split(b"=", 1)
                for item in Path(f"/proc/{int(state['MainPID'])}/environ").read_bytes().split(b"\0")
                if b"=" in item
            )
            if any(
                environment.get(key.encode()) != value.encode()
                for key, value in SERVER_SETTINGS.items()
            ):
                raise FixtureBlocked(
                    "identity",
                    "running service native settings differ from installation",
                    INSTALL_REPAIR,
                )
            expected_store = f"rocksdb://{self.directory / 'data/store'}".encode()
            expected_bind = self.endpoint.removeprefix("http://").encode()
            expected_log = ("--log=" + self.record.get("server_log_filter", "warn")).encode()
            if expected_store not in argv or expected_bind not in argv or expected_log not in argv:
                raise FixtureBlocked(
                    "identity", "running service storage/endpoint differs from installation"
                )
            if self.record.get("purpose") == LIFECYCLE_PURPOSE:
                expected = [
                    str(binary),
                    "start",
                    "--no-banner",
                    expected_log.decode(),
                    "--bind",
                    expected_bind.decode(),
                    "--query-timeout",
                    "20s",
                    "--transaction-timeout",
                    "10s",
                    *DEVELOPMENT_CAPABILITIES,
                    "--temporary-directory",
                    str(self.directory / "tmp"),
                    expected_store.decode(),
                ]
                if (
                    self.record.get("capabilities") != DEVELOPMENT_CAPABILITIES
                    or [v.decode() for v in argv if v] != expected
                ):
                    raise FixtureBlocked(
                        "identity", "development server capability ceiling differs"
                    )
            with local_urlopen(self.endpoint + "/ready", timeout=2) as response:
                if response.status != 200:
                    raise ValueError("not ready")
            with local_urlopen(self.endpoint + "/version", timeout=2) as response:
                if response.read().decode().strip() != self.record["binary"]["version"]:
                    raise FixtureBlocked("identity", "running server version differs from pin")
            executable = Path(f"/proc/{process.pid}/exe").stat()
            selected = binary.stat()
            if (
                not process.alive()
                or selected_before != executable_identity(selected)
                or executable_identity(executable) != executable_identity(selected)
            ):
                raise FixtureBlocked("identity", "running executable generation changed")
            self._daemon_identity = (process, executable_identity(executable))
        except OSError, ValueError, KeyError:
            raise FixtureBlocked(
                "readiness",
                "owned server did not answer readiness",
                "just service maintenance --check",
            ) from None

    def check(self, *, allow_maintenance: bool = False) -> dict[str, Any]:
        if checked := checked_generation(self):
            checked.validate(self)
            return copy.deepcopy(checked.readiness)
        if not allow_maintenance and (self.directory / "maintenance.json").exists():
            raise FixtureBlocked(
                "maintenance",
                "borrower admission is closed",
                "just service status; finish or recover the explicit maintenance operation",
            )
        self.check_daemon()
        for db in DATABASES:
            cfg = self.runtime(db)
            try:
                rows = sql(
                    self,
                    "SELECT generation, schema_version FROM native_installation:current;",
                    cfg=cfg,
                )
            except FixtureFailed:
                raise FixtureBlocked(
                    "schema", "installed generation marker is unavailable", INSTALL_REPAIR
                ) from None
            marker = rows[0].get("result")
            if marker != [
                {
                    "generation": bytes(self.record["service_generation"]).hex(),
                    "schema_version": self.record.get("native_schema_version", 3),
                }
            ]:
                raise FixtureBlocked(
                    "schema",
                    "native installed generation/schema does not match",
                    "just service maintenance --check",
                )
        return {
            "installation_id": self.id,
            "service_generation": self.record["service_generation"],
            "unit": self.unit,
            "endpoint": self.endpoint,
            "namespace": NAMESPACE,
            "databases": list(DATABASES),
            "state_root": str(self.directory),
            "outcome": "passed",
        }


class _NoLocalRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        # An owned daemon response cannot move credentials or administrative SQL
        # to a second endpoint. Let urllib report the original redirect as failure.
        return None


def local_urlopen(request: str | urllib.request.Request, *, timeout: float):
    """Contact the owned loopback HTTP service directly, independent of proxy env."""
    url = request.full_url if isinstance(request, urllib.request.Request) else request
    endpoint = urllib.parse.urlsplit(url)
    if (
        endpoint.scheme != "http"
        or endpoint.hostname not in {"127.0.0.1", "localhost", "::1"}
        or endpoint.username is not None
        or endpoint.password is not None
        or endpoint.port is None
    ):
        raise ValueError("owned service requires a local HTTP endpoint")
    opener = urllib.request.build_opener(urllib.request.ProxyHandler({}), _NoLocalRedirect())
    try:
        return opener.open(request, timeout=timeout)
    except urllib.error.HTTPError as error:
        # Callers retain only the status/category; no error body remains borrowed.
        error.close()
        raise


def sql(
    installation: Installation, statement: str, *, cfg: Mapping[str, Any]
) -> list[dict[str, Any]]:
    credentials = f"{cfg['username']}:{cfg['password']}".encode()
    headers = {
        "Authorization": "Basic " + base64.b64encode(credentials).decode(),
        "Accept": "application/json",
        "Content-Type": "text/plain",
        "Surreal-NS": cfg.get("namespace", NAMESPACE),
    }
    if cfg.get("database") is not None:
        headers["Surreal-DB"] = cfg["database"]
    elif cfg.get("authentication") != "root":
        raise FixtureBlocked("identity", "namespace administration requires root authority")
    if cfg.get("authentication") == "database":
        headers.update(
            {"Surreal-Auth-NS": cfg.get("namespace", NAMESPACE), "Surreal-Auth-DB": cfg["database"]}
        )
    request = urllib.request.Request(
        installation.endpoint + "/sql", data=statement.encode(), headers=headers
    )
    try:
        with local_urlopen(request, timeout=25) as response:
            rows = json.load(response)
        if not isinstance(rows, list) or not rows or any(r.get("status") != "OK" for r in rows):
            raise FixtureFailed("query", "native statement failed; SQL and credentials withheld")
        return rows
    except OSError, ValueError:
        raise FixtureBlocked(
            "query", "native request failed; credentials and response withheld"
        ) from None


def unit_text(
    root: Path, binary: Path, port: int, *, log_filter: str = "warn", development: bool = False
) -> str:
    if "\n" in str(root) or "\r" in str(root):
        raise FixtureBlocked("configuration", "service state path cannot contain line separators")

    def quote(value: str | Path) -> str:
        return '"' + str(value).replace("\\", "\\\\").replace('"', '\\"').replace("%", "%%") + '"'

    if log_filter not in {"warn", "warn,surrealdb::net=info"}:
        raise FixtureBlocked("configuration", "unrecognized owned server log filter")
    argv = [
        binary,
        "start",
        "--no-banner",
        f"--log={log_filter}",
        "--bind",
        f"127.0.0.1:{port}",
        "--query-timeout",
        "20s",
        "--transaction-timeout",
        "10s",
        *(DEVELOPMENT_CAPABILITIES if development else []),
        "--temporary-directory",
        root / "tmp",
        f"rocksdb://{root / 'data/store'}",
    ]
    return (
        "[Unit]\nDescription=library-context durable SurrealDB\n[Service]\nType=simple\n"
        # EnvironmentFile consumes one raw absolute filename; unlike ExecStart it does
        # not strip shell-style quotes or escapes. Only systemd specifiers need escaping.
        f"EnvironmentFile={str(root / 'server.env').replace('%', '%%')}\n"
        "ExecStart=" + " ".join(quote(v) for v in argv) + "\n"
        "KillMode=control-group\nRestart=no\nTimeoutStopSec=60\n"
        "[Install]\nWantedBy=default.target\n"
    )


def _available_port() -> int:
    with socket.socket() as listener:
        listener.bind(("127.0.0.1", 0))
        return listener.getsockname()[1]


def _unit_path(unit: str = UNIT) -> Path:
    return (
        Path(os.environ.get("XDG_CONFIG_HOME") or Path.home() / ".config") / "systemd/user" / unit
    )


def _run_installer(installation: Installation, installer: Path, action: str = "init") -> None:
    if not installer.is_file() or not os.access(installer, os.X_OK):
        raise FixtureBlocked(
            "installer",
            f"native installer missing: {installer}",
            "cargo build --release -p lctx, then just service install",
        )
    privileged = action in ("init", "drain", "close-admission", "open-admission")
    # Validate every target before the first effect; otherwise an invalid second
    # scope could leave the first scope partially administered.
    for db in DATABASES:
        installation.runtime(db, installer=privileged)
    for db in DATABASES:
        path = installation.runtime_path(db, installer=privileged)
        args = [str(installer), "store", "--runtime-config", str(path), action]
        environment = {
            **os.environ,
            "LCTX_SURREAL_SERVICE_CONFIG": str(installation.directory / "installation.json"),
        }
        result = subprocess.run(args, env=environment, check=False)
        returncode = result.returncode
        if returncode:
            raise FixtureFailed(
                "schema",
                f"native {action} failed for {db} (exit {returncode}); installation remains closed",
            )


def _stopped_attachment_commands(installation: Installation) -> None:
    # Released attachment receipts prove only local child cleanup. Native reconciliation
    # separately fences remote effects before releasing the explicitly named reader guards.
    for path in (installation.directory / "attachments").glob("*/record.json"):
        row = read_json(path)
        if not isinstance(row, dict) or row.get("released") is not True:
            raise FixtureBlocked("drainage", "reader reconciliation requires released attachments")
        command, cleanup = row.get("command") or {}, row.get("command_cleanup") or {}
        if (command or cleanup) and (
            cleanup.get("status") != "confirmed" or cleanup.get("survivors")
        ):
            raise FixtureBlocked(
                "drainage", "reader reconciliation requires confirmed child cleanup"
            )


def _reconcile_readers(
    installation: Installation, database: str, pins: Sequence[str], backup_holds: Sequence[str]
) -> None:
    installation.runtime(database, installer=True)
    installer = Path(installation.record["installer"])
    command = [
        str(installer),
        "store",
        "--runtime-config",
        str(installation.runtime_path(database, installer=True)),
        "reconcile",
        "--readers-stopped",
    ]
    for identity in pins:
        command.extend(("--pin", identity))
    for identity in backup_holds:
        command.extend(("--backup-hold", identity))
    result = subprocess.run(
        command,
        env={
            **os.environ,
            "LCTX_SURREAL_SERVICE_CONFIG": str(installation.directory / "installation.json"),
        },
        check=False,
    )
    if result.returncode:
        raise FixtureFailed(
            "reconciliation",
            f"named native reader reconciliation failed for {database}; admission stays closed",
        )


def _native_identity_argument(value: str) -> str:
    if not re.fullmatch(r"[0-9a-f]{64}", value):
        raise argparse.ArgumentTypeError("expected a full lowercase 32-byte hexadecimal identity")
    return value


def install(
    installer: Path,
    *,
    env: Mapping[str, str] | None = None,
    campaign: bool = False,
    purpose: str | None = None,
    server_generation: Path | None = None,
) -> Installation:
    env = os.environ if env is None else env
    root = state_root(env).resolve()
    if purpose not in (None, LIFECYCLE_PURPOSE) or (
        (purpose or server_generation) and not campaign
    ):
        raise FixtureBlocked(
            "ownership", "development purpose/server binding requires an owned campaign"
        )
    if server_generation and purpose != LIFECYCLE_PURPOSE:
        raise FixtureBlocked("ownership", "borrowed server generation requires lifecycle purpose")
    if campaign:
        if (
            not env.get("LCTX_SURREAL_SERVICE_CONFIG")
            or root
            == state_root(
                {k: v for k, v in env.items() if k != "LCTX_SURREAL_SERVICE_CONFIG"}
            ).resolve()
        ):
            raise FixtureBlocked(
                "ownership",
                "campaign installation requires an explicit separate service configuration",
            )
    from storage_lifecycle import admission
    from storage_service import tools_root

    with admission([root, tools_root(root), installer]):
        return _install(
            installer,
            env=env,
            campaign=campaign,
            purpose=purpose,
            server_generation=server_generation,
        )


def _install(
    installer: Path,
    *,
    env: Mapping[str, str],
    campaign: bool,
    purpose: str | None = None,
    server_generation: Path | None = None,
) -> Installation:
    root = state_root(env).resolve()
    generation = None
    if server_generation:
        import surrealdb_server

        try:
            generation = surrealdb_server.validate_descriptor(server_generation)
            binary_origin = Path(generation["provenance"]["service_directory"])
            generation = _owned_server_generation(binary_origin, server_generation)
        except (KeyError, StorageBlocked) as error:
            raise FixtureBlocked(
                "binary", "campaign server generation cannot be verified"
            ) from error
        binary = Path(generation["path"])
    else:
        binary = verify_binary(env)
    if missing := systemd_available(env=env):
        raise FixtureBlocked("systemd", missing, SYSTEMD_REPAIR)
    if not installer.is_file() or not os.access(installer, os.X_OK):
        raise FixtureBlocked(
            "installer",
            f"native installer missing: {installer}",
            "cargo build --release -p lctx, then just service install",
        )
    root.mkdir(parents=True, exist_ok=True, mode=0o700)
    root.chmod(0o700)
    with contextlib.ExitStack() as stack:
        gate = hold_lock(root / "admission.lock")
        stack.callback(os.close, gate)
        if (root / "installation.json").exists():
            installation = Installation.load(env, ready=False)
            if (installation.record.get("kind", "stable") == "campaign") != campaign:
                raise FixtureBlocked(
                    "ownership", "install mode differs from the existing installation"
                )
            if installation.record.get("purpose") != purpose:
                raise FixtureBlocked(
                    "ownership", "install purpose differs from the existing installation"
                )
            if generation and installation.record["binary"] != _generation_binary(generation):
                raise FixtureBlocked("identity", "campaign executable generation changed")
            if installation.record["phase"] == "ready":
                installation.check()
                return installation
            if borrowers(installation):
                raise FixtureBlocked(
                    "ownership", "incomplete installation has unresolved borrowers"
                )
        else:
            installation_id = str(uuid.uuid4())
            kind = "campaign" if campaign else "stable"
            unit_name = installation_unit({"kind": kind, "installation_id": installation_id})
            state = unit_properties(unit_name, "LoadState", "ActiveState")
            if state.get("LoadState") != "not-found" or _unit_path(unit_name).exists():
                raise FixtureBlocked(
                    "ownership", "owned unit exists without this installation; refusing adoption"
                )
            service_generation = list(secrets.token_bytes(32))
            port = _available_port()
            from storage_service import prepare

            installer_generation = prepare(root, installer)
            installer = Path(installer_generation["path"])
            record = {
                "schema": SCHEMA,
                "installation_id": installation_id,
                "kind": kind,
                "phase": "installing",
                "service_generation": service_generation,
                "unit": unit_name,
                "state_root": str(root),
                "endpoint": f"http://127.0.0.1:{port}",
                "grpc_endpoint": f"grpc://127.0.0.1:{port}",
                "namespace": NAMESPACE,
                "databases": list(DATABASES),
                "export_batch_size": EXPORT_BATCH_SIZE,
                "binary": {
                    "path": str(binary.resolve()),
                    "sha256": BINARY_SHA256,
                    "version": VERSION,
                },
                "installer": str(installer.resolve()),
                "installer_generation": installer_generation,
            }
            if purpose:
                record["purpose"] = purpose
                record["capabilities"] = list(DEVELOPMENT_CAPABILITIES)
            if generation:
                record["binary"] = _generation_binary(generation)
                record["binary_origin"] = str(Path(generation["provenance"]["service_directory"]))
            installation = Installation(root, record)
            declared = _installer_cli(installation, installer_generation, "main", "schema")
            _current_schema(declared)
            record["native_schema_version"] = declared["schema_version"]
            for name in ("data", "tmp", "attachments", "serving"):
                (root / name).mkdir(mode=0o700)
            root_user, root_pass = "library_context_installer", secrets.token_urlsafe(32)
            server_env = {"SURREAL_USER": root_user, "SURREAL_PASS": root_pass, **SERVER_SETTINGS}
            private_json(root / "installation.json", record)
            environment = root / "server.env"
            environment.write_text("".join(f"{k}={v}\n" for k, v in server_env.items()))
            environment.chmod(0o600)
            for db in DATABASES:
                leases = root / "product-leases" / db
                leases.mkdir(parents=True, mode=0o700)
                cfg = {
                    "reuse": {
                        "database": db,
                        "capacity_bytes": PRODUCT_CAPACITY_BYTES,
                        "lease_directory": str(leases),
                    },
                    "endpoint": record["grpc_endpoint"],
                    "username": f"{db}_writer",
                    "password": secrets.token_urlsafe(32),
                    "viewer_username": f"{db}_viewer",
                    "viewer_password": secrets.token_urlsafe(32),
                    "namespace": NAMESPACE,
                    "database": db,
                    "cache_database": db,
                    "service_generation": service_generation,
                    "authentication": "database",
                    "selection": str(root / f"{db}-selected.json"),
                }
                private_json(installation.runtime_path(db), cfg)
                private_json(
                    installation.runtime_path(db, installer=True),
                    {**cfg, "username": root_user, "password": root_pass, "authentication": "root"},
                )
            unit = _unit_path(installation.unit)
            unit.parent.mkdir(parents=True, exist_ok=True)
            unit.write_text(unit_text(root, binary, port, development=purpose == LIFECYCLE_PURPOSE))
            result = systemctl("daemon-reload")
            if result.returncode:
                raise FixtureBlocked("systemd", "daemon-reload failed", SYSTEMD_REPAIR)
        if campaign:
            # This durable record precedes enable/start, including failed installation.
            from storage_lifecycle import Storage

            consumer = (
                "lifecycle-campaign:" if purpose else "efficiency-campaign:"
            ) + installation.id
            storage = installation.record.setdefault("campaign_storage", {})
            storage["consumer"] = consumer
            storage["content"] = Storage().publish(
                root,
                "native-content",
                {"kind": "native", "path": str(root)},
                consumer,
                requires="immediate-use" if purpose else "raw-replay",
            )
            if generation:
                executable_root = Path(generation["descriptor_path"]).parent
                storage["server"] = Storage().publish(
                    executable_root,
                    "native-content",
                    {"kind": "native", "path": str(executable_root)},
                    consumer,
                )
            private_json(root / "installation.json", installation.record)
        result = systemctl("enable", "--now", installation.unit)
        if result.returncode:
            raise FixtureBlocked("launch", "owned service launch failed", "just service status")
        deadline = time.monotonic() + READY_TIMEOUT
        while True:
            try:
                with local_urlopen(installation.endpoint + "/ready", timeout=1) as response:
                    if response.status == 200:
                        break
            except OSError, ValueError:
                pass
            if time.monotonic() >= deadline:
                raise FixtureBlocked(
                    "readiness", "new owned service did not become ready", "just service status"
                )
            time.sleep(0.1)
        installation.check_daemon()
        for db in DATABASES:
            cfg = installation.runtime(db)
            bootstrap = installation.runtime(db, installer=True)
            sql(
                installation,
                f"DEFINE NAMESPACE IF NOT EXISTS {NAMESPACE}; USE NS {NAMESPACE}; "
                f"DEFINE DATABASE IF NOT EXISTS {db} STRICT; USE DB {db}; "
                f"DEFINE USER OVERWRITE {cfg['username']} ON DATABASE "
                f"PASSWORD {json.dumps(cfg['password'])} ROLES OWNER; "
                f"DEFINE USER OVERWRITE {cfg['viewer_username']} ON DATABASE "
                f"PASSWORD {json.dumps(cfg['viewer_password'])} ROLES VIEWER;",
                cfg=bootstrap,
            )
        _run_installer(installation, installer)
        installation.check(allow_maintenance=True)
        try:
            _run_installer(installation, installer, "open-admission")
        except BaseException:
            with contextlib.suppress(Exception):
                _run_installer(installation, installer, "close-admission")
            raise
        installation.record["phase"] = "ready"
        private_json(root / "installation.json", installation.record)
        return installation


def borrowers(installation: Installation) -> list[dict[str, Any]]:
    # Local process disappearance does not establish remote drainage. Only release confirms it.
    found = []
    for path in sorted((installation.directory / "attachments").glob("*/record.json")):
        row = read_json(path)
        if not isinstance(row, dict):
            found.append({"id": path.parent.name, "state": "unknown", "protected": True})
        else:
            import surrealdb_fixture

            native = surrealdb_fixture.native_completion(path.parent, row)
            if row.get("released") is not True or surrealdb_fixture.native_protected(
                path.parent, row, native
            ):
                found.append({**row, "protected": True})
    return found


def lifecycle_campaign(installation: Installation, identity: str) -> None:
    """An explicit target identity authorizes only this disposable owner boundary."""
    if (
        installation.id != identity
        or installation.record.get("kind") != "campaign"
        or installation.record.get("purpose") != LIFECYCLE_PURPOSE
    ):
        raise FixtureBlocked("ownership", "target is not the exact owned lifecycle campaign")
    if any(path.is_symlink() for path in (installation.directory, *installation.directory.parents)):
        raise FixtureBlocked("ownership", "campaign state must have a physical path")


def development_build_info(value: Any) -> dict[str, Any]:
    """Project the internal compiled response; never expose arbitrary worker output."""
    if value is None:
        return {"status": "missing"}
    try:
        if value["format"] != 1 or value["kind"] != "lctx-surreal-dev-build-info":
            raise ValueError
        dependencies = {}
        for name in ("surrealkit", "surrealdb", "surrealdb-rpc"):
            dependency = value["dependencies"][name]
            version = dependency["version"]
            if not isinstance(version, str) or not re.fullmatch(
                r"[0-9]+\.[0-9]+\.[0-9]+(?:-[a-zA-Z0-9.]+)?", version
            ):
                raise ValueError
            profile = dependency["patch_profile"]
            expected_root = f"third_party/{name}"
            if (
                dependency["source"] != "compiled-resolved-lock-and-workspace-patch"
                or profile["status"] != "declared_build_input"
                or profile["manifest"] != expected_root + "/Cargo.toml"
                or profile["documentation"] != expected_root + "/README.lctx.md"
                or any(
                    not re.fullmatch(r"[0-9a-f]{64}", profile[key])
                    for key in ("manifest_blake3", "documentation_blake3")
                )
            ):
                raise ValueError
            dependencies[name] = {
                "version": version,
                "source": dependency["source"],
                "patch_profile": {
                    key: profile[key]
                    for key in (
                        "status",
                        "manifest",
                        "manifest_blake3",
                        "documentation",
                        "documentation_blake3",
                    )
                },
            }
        schema = value["schema"]
        if (
            schema["source"] != "compiled-embedded-native-schema"
            or schema["authority_correspondence"] != "passed"
            or not re.fullmatch(r"[0-9a-f]{64}", schema["profile_identity"])
            or schema["modules"] != ["control", "canonical", "search", "compiler", "completion"]
        ):
            raise ValueError
        return {
            "status": "recorded",
            "dependencies": dependencies,
            "schema": {
                key: schema[key]
                for key in ("source", "profile_identity", "authority_correspondence", "modules")
            },
            "server": {"status": "not_inspected"},
        }
    except KeyError, TypeError, ValueError:
        return {"status": "unknown"}


def development_workers(installation: Installation) -> list[dict[str, Any]]:
    """Inspect actual run owners; no beta report is needed to find a worker."""
    import runs

    result = []
    for path in sorted((installation.directory / "development").glob("*/record.json")):
        row = read_json(path)
        if (
            not isinstance(row, dict)
            or row.get("installation_id") != installation.id
            or row.get("server_generation")
            != bytes(installation.record["service_generation"]).hex()
            or not isinstance(row.get("run_path"), str)
        ):
            raise FixtureBlocked("ownership", "development worker inventory is incomplete")
        run_path = Path(row["run_path"])
        run = runs.load_record(run_path)
        cleanup = runs.cleanup_of(run)
        pending = run is None or runs.owner_alive(run_path, run) or runs.cleanup_protected(run)

        def axis(value: Any, statuses: set[str]) -> dict[str, str]:
            status = value.get("status") if isinstance(value, dict) else None
            return {
                "status": status if isinstance(status, str) and status in statuses else "unknown"
            }

        provenance = row.get("worker_provenance")
        provenance = provenance if isinstance(provenance, dict) else {}
        worker_sha = row.get("worker_sha256")
        worker_sha = (
            worker_sha
            if isinstance(worker_sha, str) and re.fullmatch(r"[0-9a-f]{64}", worker_sha)
            else None
        )
        build_info = development_build_info(provenance.get("build_info"))
        if provenance.get("worker_sha256") != worker_sha or worker_sha is None:
            build_info = {"status": "unknown"} if provenance else {"status": "missing"}
        run_owner = (run or {}).get("owner")
        ledger_owner = row.get("owner")
        owner_correspondence = (
            "missing"
            if not isinstance(run_owner, dict) or not isinstance(ledger_owner, dict)
            else "matches"
            if run_owner == ledger_owner
            else "differs"
        )
        result.append(
            {
                "id": path.parent.name,
                "run": run_path.name,
                "pending": pending,
                "cleanup": axis(cleanup, {"confirmed", "failed", "unknown"}),
                "report": axis(
                    row.get("report"), {"passed", "failed", "not_created", "not_applicable"}
                ),
                "outcome": row.get("outcome")
                if row.get("outcome") in ("passed", "failed", "unknown")
                else "unknown",
                "operation": row.get("operation")
                if row.get("operation") in ("sync", "test")
                else None,
                "database": row.get("database")
                if isinstance(row.get("database"), str)
                and re.fullmatch(r"lifecycle_[0-9a-f]{32}", row["database"])
                else None,
                "worker_sha256": worker_sha,
                "worker_provenance": {
                    "worker_sha256": worker_sha,
                    "recorded_at": provenance.get("recorded_at"),
                    "build_info": build_info,
                },
                "command_exit": row.get("command_exit")
                if type(row.get("command_exit")) is int
                else None,
                "child_cleanup": axis(row.get("child_cleanup"), {"confirmed", "failed", "unknown"}),
                "credential_cleanup": axis(
                    row.get("credential_cleanup"), {"passed", "failed", "not_applicable", "unknown"}
                ),
                "remote": axis(row.get("remote"), {"passed", "failed", "unknown"}),
                "observation": {
                    "source": "development-ledger-and-managed-run",
                    "observed_at": time.time(),
                    "recorded_at": row.get("recorded_at"),
                    "owner": "surrealdb_development",
                    "run_owner": {
                        key: run_owner[key]
                        for key in ("pid", "start_ticks", "boot_id", "pid_namespace")
                        if key in run_owner
                    }
                    if isinstance(run_owner, dict)
                    else None,
                    "owner_correspondence": owner_correspondence,
                    "run_record": "missing" if run is None else "observed",
                },
                "routes": {
                    "run": ["just", "runs", "status", run_path.name],
                    "native": "docs/surrealdb.md#maintenance-diagnostics",
                    "disposal": ["just", "service", "dispose-campaign", installation.id],
                },
            }
        )
    return result


def _campaign_executor(installation: Installation, *, allow_removed_unit: bool = False) -> bool:
    """Authenticate the current executor independently of a disposal checkpoint."""
    unit = _unit_path(installation.unit)
    state = unit_properties(installation.unit, "ActiveState", "MainPID")
    active = state.get("ActiveState") == "active"
    if not active and (
        state.get("ActiveState") not in {"inactive", "failed"} or state.get("MainPID") != "0"
    ):
        raise FixtureBlocked("identity", "campaign executor state is not authenticated")
    if allow_removed_unit and not unit.exists() and not unit.is_symlink() and not active:
        return False
    port = urllib.parse.urlsplit(installation.endpoint).port
    if port is None:
        raise FixtureBlocked("identity", "campaign endpoint has no explicit port")
    expected = unit_text(
        installation.directory,
        Path(installation.record["binary"]["path"]),
        port,
        log_filter=installation.record.get("server_log_filter", "warn"),
        development=True,
    )
    if (
        unit.is_symlink()
        or installation.record.get("capabilities") != DEVELOPMENT_CAPABILITIES
        or unit.read_text() != expected
    ):
        raise FixtureBlocked("identity", "campaign unit differs from its owned definition")
    if active:
        installation.check_daemon()
    return active


def _campaign_disposal_receipt(installation: Installation, receipt: Any) -> None:
    if not isinstance(receipt, dict):
        raise FixtureBlocked("identity", "campaign disposal receipt is invalid")
    steps = receipt.get("steps")
    if (
        receipt.get("schema") != 1
        or receipt.get("kind") != "lifecycle-campaign-disposal"
        or receipt.get("installation_id") != installation.id
        or receipt.get("generation") != bytes(installation.record["service_generation"]).hex()
        or receipt.get("outcome") not in ("unknown", "passed")
        or not isinstance(receipt.get("workers"), list)
        or not isinstance(steps, dict)
        or not isinstance(receipt.get("logical_commands", []), list)
        or set(steps)
        - {
            "close-admission",
            "drain",
            "executor-shutdown",
            "datastore-disposal",
            "attachment-release",
            "storage-release",
            "credential-removal",
            "unit-removal",
            "installer-removal",
        }
        or any(value not in ("passed", "failed", "not_run") for value in steps.values())
        or (
            steps.get("datastore-disposal") == "passed"
            and steps.get("executor-shutdown") != "passed"
        )
        or (
            receipt.get("outcome") == "passed"
            and any(
                steps.get(key) != "passed" for key in ("executor-shutdown", "datastore-disposal")
            )
        )
    ):
        raise FixtureBlocked("identity", "campaign disposal receipt scope or steps differ")


def _disposal_installer_args(installation: Installation, database: str, action: str) -> list[str]:
    # The existing run owner inherits the environment. Issue the exact campaign
    # selector to this child only, preserving ordinary installer's target binding.
    return [
        "/usr/bin/env",
        "LCTX_SURREAL_SERVICE_CONFIG=" + str(installation.directory / "installation.json"),
        installation.record["installer"],
        "store",
        "--runtime-config",
        str(installation.runtime_path(database, installer=True)),
        action,
    ]


def _disposal_command_record(
    installation: Installation, command: Mapping[str, Any]
) -> tuple[Path, dict]:
    """Authenticate a referenced existing run before observing or recovering it."""
    import runs

    try:
        run_path = Path(command["run_path"])
        database, action = command["database"], command["action"]
        if (
            command.get("installation_id") != installation.id
            or command.get("generation") != bytes(installation.record["service_generation"]).hex()
            or database not in DATABASES
            or action not in {"close-admission", "drain"}
            or not run_path.is_absolute()
            or ".." in run_path.parts
            or any(path.is_symlink() for path in (run_path, *run_path.parents))
            or not run_path.is_dir()
            or run_path.stat().st_uid != os.getuid()
        ):
            raise ValueError
        path = run_path / runs.RECORD
        if path.is_symlink() or path.stat().st_uid != os.getuid() or path.stat().st_mode & 0o077:
            raise ValueError
        record = runs.load_record(run_path)
        expected_args = _disposal_installer_args(installation, database, action)
        if (
            not isinstance(record, dict)
            or record.get("id") != run_path.name
            or record.get("owner") != command.get("owner")
            or record.get("argv") != expected_args
            or record.get("cwd") != str(ROOT)
            or record.get("label") != f"lifecycle-disposal-{action}-{database}"
        ):
            raise ValueError
        ProcessIdentity.from_json(record["owner"])
        reference = command["storage"]
        from storage_lifecycle import Storage

        storage = Storage()
        owned = storage.get(reference["receipt"])
        from storage_lifecycle import physical

        if (
            reference.get("consumer") != f"lifecycle-disposal:{installation.id}:{run_path.name}"
            or owned.get("owner") != {"kind": "run", "path": str(run_path)}
            or owned.get("path") != str(run_path)
            or owned.get("category") != "run-receipt"
            or owned.get("identity") != physical(run_path)
            or reference["consumer"] not in owned.get("obligations", {})
        ):
            raise ValueError
        return run_path, record
    except OSError, KeyError, TypeError, ValueError, StorageBlocked:
        raise FixtureBlocked(
            "ownership", "disposal logical command run identity is unavailable or differs"
        ) from None


def _recover_disposal_commands(installation: Installation, receipt: dict) -> None:
    """Replay only exact existing run owners; unresolved local children forbid discard."""
    import runs

    for command in receipt.get("logical_commands", []):
        run_path, record = _disposal_command_record(installation, command)
        if runs.owner_alive(run_path, record):
            raise FixtureBlocked("drainage", "disposal logical command has a live owner")
        if runs.cleanup_protected(record):
            runs.cancel(run_path)
            record = runs.load_record(run_path)
        if record is None or runs.cleanup_protected(record):
            raise FixtureBlocked(
                "drainage", "disposal logical command local cleanup remains unresolved"
            )
        command["child_cleanup"] = runs.cleanup_of(record)
        if command.get("outcome") != "passed":
            receipt["steps"][command["action"]] = "failed"


def _run_disposal_installer(
    installation: Installation, action: str, receipt: dict
) -> dict[str, Any]:
    """A finite logical attempt owned by ordinary runs, without changing installer semantics."""
    import runs
    from harness import observe_exit
    from storage_lifecycle import Storage, admission

    if action not in {"close-admission", "drain"}:
        raise FixtureBlocked("ownership", "unsupported disposable logical operation")
    installer = Path(installation.record["installer"])
    generation = installation.record.get("installer_generation", {})
    if (
        generation.get("path") != str(installer)
        or any(path.is_symlink() for path in (installer, *installer.parents))
        or not installer.is_file()
        or installer.stat().st_uid != os.getuid()
        or file_sha256(installer) != generation.get("sha256")
        or not os.access(installer, os.X_OK)
    ):
        raise FixtureBlocked("identity", "disposal installer generation differs")
    # Check both scopes before any logical mutation, matching ordinary installer use.
    for database in DATABASES:
        installation.runtime(database, installer=True)
    deadline = time.monotonic() + DISPOSAL_LOGICAL_TIMEOUT
    for database in DATABASES:

        class BoundedOwner(runs.Owner):
            def _own(self, child: subprocess.Popen) -> int:
                while True:
                    if (code := observe_exit(child)) is not None:
                        return code
                    stopped = self.received is not None or (self.dir / runs.CANCEL).exists()
                    if stopped or time.monotonic() >= deadline:
                        self.record.update(
                            phase="stopping",
                            termination="interrupted",
                            disposal_stop="cancelled" if stopped else "deadline",
                        )
                        self.write()
                        # Existing Owner catches this only after its SpawnGuard drains
                        # the still-waitable child group and records actual exit/cleanup.
                        if stopped:
                            raise InterruptedError("disposal logical command cancelled")
                        raise TimeoutError("disposal logical command deadline")
                    if self._fold_progress():
                        self.write()
                    time.sleep(runs.POLL)

        run_path = runs.new_run_dir()
        with admission([run_path]):
            args = _disposal_installer_args(installation, database, action)
            owner = BoundedOwner(
                run_path,
                args,
                label=f"lifecycle-disposal-{action}-{database}",
                cwd=str(ROOT),
                mode="foreground",
                stream=True,
            )
            owner.write()
            consumer = f"lifecycle-disposal:{installation.id}:{run_path.name}"
            content = Storage().publish(
                run_path,
                "run-receipt",
                {"kind": "run", "path": str(run_path)},
                consumer,
                managed=True,
            )
            command = {
                "installation_id": installation.id,
                "generation": bytes(installation.record["service_generation"]).hex(),
                "action": action,
                "database": database,
                "run_path": str(run_path),
                "owner": owner.record["owner"],
                "outcome": "unknown",
                "child_cleanup": {"status": "unknown"},
                "storage": {"receipt": content, "consumer": consumer},
            }
            receipt.setdefault("logical_commands", []).append(command)
            private_json(installation.directory / "disposal.json", receipt)
            code = owner.run()
            record = runs.load_record(run_path)
            cleanup = runs.cleanup_of(record)
            stop = (record or {}).get("disposal_stop")
            # Natural child exit may win the polling race with a signal or cancel
            # request. Preserve its real exit/cleanup while honoring current stop.
            interrupted = (
                owner.received is not None
                or (run_path / runs.CANCEL).exists()
                or stop == "cancelled"
            )
            if interrupted and stop is None:
                stop = "cancelled"
            command.update(
                outcome="passed"
                if code == 0
                and stop is None
                and not interrupted
                and not runs.cleanup_protected(record or {})
                else "failed",
                command_exit=code,
                child_cleanup=cleanup,
                stopped=stop,
                interrupted=interrupted,
            )
            private_json(installation.directory / "disposal.json", receipt)
            if record is None or runs.cleanup_protected(record):
                return {
                    "outcome": "failed",
                    "child_cleanup": cleanup,
                    "local_drained": False,
                    "interrupted": interrupted,
                }
            if command["outcome"] != "passed":
                return {
                    "outcome": "failed",
                    "child_cleanup": cleanup,
                    "local_drained": True,
                    "interrupted": interrupted,
                }
    return {
        "outcome": "passed",
        "child_cleanup": {"status": "confirmed", "survivors": []},
        "local_drained": True,
        "interrupted": False,
    }


def dispose_campaign(installation: Installation, identity: str) -> dict[str, Any]:
    """Drain and discard one explicitly owned target, retaining only current disposition."""
    lifecycle_campaign(installation, identity)
    from storage_lifecycle import Storage, admission
    from storage_service import tools_root

    root = installation.directory
    storage = installation.record.get("campaign_storage")
    if not isinstance(storage, dict) or storage.get("consumer") != "lifecycle-campaign:" + identity:
        raise FixtureBlocked("ownership", "campaign prelaunch storage owner is missing")
    from harness import try_hold_lock

    with admission([root, tools_root(root)]), contextlib.ExitStack() as stack:
        for name in ("admission.lock", "borrowers.lock"):
            fd = try_hold_lock(root / name)
            if fd is None:
                raise FixtureBlocked("drainage", "campaign has a live admission/borrower owner")
            stack.callback(os.close, fd)
        # Reload under exclusion; a stale Python object cannot dispose a replacement.
        current = Installation.load(
            {"LCTX_SURREAL_SERVICE_CONFIG": str(root / "installation.json")}, ready=False
        )
        if current.record != installation.record:
            raise FixtureBlocked("identity", "campaign changed before disposal")
        receipt_path = root / "disposal.json"
        receipt = read_json(receipt_path)
        if receipt is not None:
            if receipt_path.is_symlink():
                raise FixtureBlocked("identity", "campaign disposal receipt cannot be a symlink")
            _campaign_disposal_receipt(current, receipt)
        if current.record.get("phase") == "disposed":
            if not receipt or receipt.get("outcome") != "passed":
                raise FixtureBlocked("disposal", "campaign disposition is incomplete")
            return receipt
        previous = read_json(root / "maintenance.json")
        if previous and (
            previous.get("operation") != "dispose-campaign"
            or previous.get("installation_id") != identity
            or receipt is None
        ):
            raise FixtureBlocked("maintenance", "campaign has an unresolved maintenance owner")
        # Both exclusive owner locks are held here. A retained process identity
        # alone cannot make a completed or interrupted operation live again.
        workers = development_workers(current)
        if any(worker["pending"] for worker in workers):
            raise FixtureBlocked(
                "drainage",
                "development worker cleanup remains unresolved; inspect just runs status",
            )
        pending = borrowers(current)
        import surrealdb_fixture

        for row in pending:
            directory = root / "attachments" / str(row.get("id", ""))
            try:
                ProcessIdentity.from_json(row["owner"])
            except KeyError, TypeError, ValueError:
                raise FixtureBlocked(
                    "ownership", "native attachment owner identity is incomplete"
                ) from None
            native = row.get("native_drain")
            if (
                directory.parent != root / "attachments"
                or row.get("fixture") != identity
                or not isinstance(native, dict)
                or native.get("generation") != bytes(current.record["service_generation"]).hex()
                or native.get("attachment") != row.get("id")
                or native.get("directory") != str(directory / "native-drain")
                or row.get("command_cleanup") != {"status": "confirmed", "survivors": []}
                or surrealdb_fixture.owner_alive(row.get("owner"), directory / "owner.lock")
            ):
                raise FixtureBlocked(
                    "drainage",
                    "native attachment local cleanup remains unresolved; "
                    "inspect just fixture --list",
                )
            lease = try_hold_lock(directory / "owner.lock")
            if lease is None:
                raise FixtureBlocked("drainage", "native attachment has a live owner")
            os.close(lease)
        _coordination_identity(current)
        receipt = receipt or {
            "schema": 1,
            "kind": "lifecycle-campaign-disposal",
            "installation_id": identity,
            "generation": bytes(current.record["service_generation"]).hex(),
            "workers": workers,
            "outcome": "unknown",
            "steps": {},
        }
        private_json(receipt_path, receipt)
        marker = {
            "operation": "dispose-campaign",
            "owner": ProcessIdentity.of().to_json(),
            "installation_id": identity,
            "token": secrets.token_urlsafe(32),
        }
        private_json(root / "maintenance.json", marker)
        try:
            _recover_disposal_commands(current, receipt)
            private_json(receipt_path, receipt)
            if (
                current.record.get("phase") == "ready"
                and receipt["steps"].get("executor-shutdown") != "passed"
            ):
                available = _campaign_executor(current)
                if available:
                    cfg = current.runtime("validation", installer=True)
                    root_inventory = sql(current, "INFO FOR ROOT", cfg=cfg)[0]["result"]
                    namespaces = root_inventory.get("namespaces", {})
                    receipt["inventory"] = {"namespace_names": sorted(namespaces), "databases": {}}
                    for namespace in sorted(namespaces):
                        info = sql(
                            current,
                            "INFO FOR NS",
                            cfg={**cfg, "namespace": namespace, "database": None},
                        )[0]["result"]
                        receipt["inventory"]["databases"][namespace] = sorted(
                            info.get("databases", {})
                        )
                    for operation in ("close-admission", "drain"):
                        if receipt["steps"].get(operation) in {"passed", "failed"}:
                            continue
                        try:
                            logical = _run_disposal_installer(current, operation, receipt)
                            receipt["steps"][operation] = logical["outcome"]
                        except (FixtureBlocked, FixtureFailed) as error:
                            # Checked whole-service shutdown settles abandoned native
                            # state, without relabeling a failed drain as successful.
                            receipt["steps"][operation] = "failed"
                            receipt.setdefault("errors", {})[operation] = type(error).__name__
                            logical = {"local_drained": True, "interrupted": False}
                        private_json(receipt_path, receipt)
                        if not logical["local_drained"]:
                            raise FixtureBlocked(
                                "drainage",
                                "disposal logical command local cleanup remains unresolved",
                            )
                        if logical["interrupted"]:
                            raise FixtureBlocked(
                                "interrupted",
                                "disposal logical command cancelled; explicit retry required",
                            )
                else:
                    receipt["inventory"] = {
                        "status": "not_run",
                        "reason": "exact owned executor already inactive",
                    }
                    for operation in ("close-admission", "drain"):
                        receipt["steps"].setdefault(operation, "not_run")
                private_json(receipt_path, receipt)
            # A checkpoint never authorizes stopping a replacement executor.
            _recover_disposal_commands(current, receipt)
            private_json(receipt_path, receipt)
            removed_unit = (
                not _unit_path(current.unit).exists()
                and receipt["steps"].get("executor-shutdown") == "passed"
            )
            _campaign_executor(current, allow_removed_unit=removed_unit)
            if not removed_unit:
                _stop_owned(current)
            receipt["steps"]["executor-shutdown"] = "passed"
            private_json(receipt_path, receipt)
            if _unit_path(current.unit).exists() and systemctl("disable", current.unit).returncode:
                raise FixtureBlocked("disposal", "owned campaign could not be disabled")
            # Only the owned daemon's datastore is disposable; executable generations
            # have independent owners and are released through their own consumers.
            for name in ("data", "tmp", "product-leases", "serving"):
                path = root / name
                if path.is_symlink():
                    raise FixtureBlocked("ownership", "campaign disposable path is a symlink")
                if path.exists():
                    shutil.rmtree(path)
            receipt["steps"]["datastore-disposal"] = "passed"
            # This outcome concerns the whole target. Original request finality is
            # preserved separately in each attachment's immutable acknowledgement.
            receipt["outcome"] = "passed"
            private_json(receipt_path, receipt)
            for row in pending:
                surrealdb_fixture.acknowledge_campaign_discard(
                    root / "attachments" / row["id"], receipt
                )
            receipt["steps"]["attachment-release"] = "passed"
            private_json(receipt_path, receipt)
            # This immutable installer was copied solely for this campaign. Its
            # subprocesses have returned and all campaign executors are excluded.
            generation = current.record.get("installer_generation", {})
            installer = Path(current.record["installer"])
            expected_installer = (
                tools_root(root) / "lctx" / str(generation.get("sha256", "")) / "lctx"
            )
            if generation.get("path") != str(installer) or installer != expected_installer:
                raise FixtureBlocked(
                    "identity", "campaign installer is not its private owned generation"
                )
            if any(path.is_symlink() for path in (installer, *installer.parents)):
                raise FixtureBlocked(
                    "identity", "campaign installer generation must have a physical path"
                )
            if installer.parent.exists():
                if (
                    set(installer.parent.iterdir()) != {installer}
                    or not installer.is_file()
                    or file_sha256(installer) != generation["sha256"]
                ):
                    raise FixtureBlocked(
                        "identity", "campaign installer generation differs before retirement"
                    )
                installer.unlink()
                installer.parent.rmdir()
            receipt["steps"]["installer-removal"] = "passed"
            private_json(receipt_path, receipt)
            unit = _unit_path(current.unit)
            if unit.exists():
                # Reauthenticate immediately before retiring the current unit file.
                if _campaign_executor(current):
                    raise FixtureBlocked(
                        "disposal", "campaign executor restarted before unit retirement"
                    )
                unit.unlink()
            if systemctl("daemon-reload").returncode:
                raise FixtureBlocked("disposal", "campaign unit retirement reload failed")
            receipt["steps"]["unit-removal"] = "passed"
            private_json(receipt_path, receipt)
            for key in ("server", "content"):
                if storage.get(key):
                    Storage().release(
                        storage[key], storage["consumer"], "checked whole campaign disposal"
                    )
            for command in receipt.get("logical_commands", []):
                _disposal_command_record(current, command)
                reference = command["storage"]
                Storage().release(
                    reference["receipt"], reference["consumer"], "checked whole campaign disposal"
                )
            receipt["steps"]["storage-release"] = "passed"
            receipt["outcome"] = "passed"
            private_json(receipt_path, receipt)
            # Credentials have no surviving consumer after confirmed target disposal.
            # Interrupted beta workers may have issued a token into their exact owned
            # input project. Child drainage alone does not erase that local copy.
            from surrealdb_development import restore_campaign_token_inputs

            restore_campaign_token_inputs(current)
            for path in [
                root / "server.env",
                *(
                    root / f"{db}-{kind}.json"
                    for db in DATABASES
                    for kind in ("runtime", "installer")
                ),
            ]:
                path.unlink(missing_ok=True)
            receipt["steps"]["credential-removal"] = "passed"
            private_json(receipt_path, receipt)
            (root / "maintenance.json").unlink()
            # Publish the idempotent terminal phase only after every owned cleanup
            # step. An interruption before this point resumes from the receipt.
            current.record["phase"] = "disposed"
            private_json(root / "installation.json", current.record)
            return receipt
        except BaseException:
            # Keep exact scope, completed steps and closed admission for explicit recovery.
            private_json(receipt_path, receipt)
            raise


def executable_identity(info: os.stat_result) -> tuple[int, ...]:
    return (info.st_dev, info.st_ino, info.st_size, info.st_mtime_ns, info.st_ctime_ns)


_checked_generations = contextvars.ContextVar("checked_service_generations", default=())


def checked_generation(installation: Installation) -> CheckedServiceGeneration | None:
    return next(
        (
            checked
            for checked in reversed(_checked_generations.get())
            if checked.installation.directory == installation.directory
        ),
        None,
    )


@dataclass
class CheckedServiceGeneration:
    """In-process proof carried only while its authenticated borrower lease is live.

    This is deliberately not deserializable authority. A child process borrows/checks for
    itself; exported identity JSON is evidence only. The live daemon owns its loaded inode;
    borrower protection excludes managed replacement/schema changes, while exact identity
    checks below reject process, pathname, descriptor, runtime or admission changes.
    """

    installation: Installation
    readiness: dict[str, Any]
    lease: int
    descriptor: dict[str, Any]
    runtime_bytes: dict[Path, bytes]
    daemon: Any
    owner_pid: int
    active: bool = True

    def validate(self, installation: Installation) -> None:
        try:
            os.fstat(self.lease)
            if (
                not self.active
                or self.owner_pid != os.getpid()
                or installation.record != self.descriptor
                or read_json(installation.directory / "installation.json") != self.descriptor
                or (installation.directory / "maintenance.json").exists()
                or any(path.read_bytes() != value for path, value in self.runtime_bytes.items())
            ):
                raise ValueError("generation or admission changed")
            if self.daemon is not None:
                process, executable = self.daemon
                if (
                    not process.alive()
                    or executable_identity(Path(f"/proc/{process.pid}/exe").stat()) != executable
                    or executable_identity(Path(self.descriptor["binary"]["path"]).stat())
                    != executable
                ):
                    raise ValueError("daemon or executable changed")
        except OSError, ValueError:
            raise FixtureBlocked(
                "identity",
                "checked service generation is no longer protected",
                "just service status; acquire a new fixture borrow",
            ) from None

    def server_identity(self, installation: Installation) -> dict[str, str]:
        self.validate(installation)
        binary = self.descriptor["binary"]
        result = {"version": binary["version"], "binary_sha256": binary["sha256"]}
        if generation := binary.get("generation"):
            result["generation"] = generation["identity"]
        return result


@contextlib.contextmanager
def borrow(installation: Installation) -> Iterator[CheckedServiceGeneration | None]:
    root = installation.directory
    if maintenance_owned(installation):
        yield None
        return
    gate = os.open(root / "admission.lock", os.O_RDWR | os.O_CLOEXEC)
    use = None
    try:
        try:
            fcntl.flock(gate, fcntl.LOCK_SH | fcntl.LOCK_NB)
        except BlockingIOError:
            raise FixtureBlocked(
                "maintenance", "borrower admission is closed", "just service status"
            ) from None
        use = os.open(root / "borrowers.lock", os.O_RDWR | os.O_CREAT | os.O_CLOEXEC, 0o600)
        fcntl.flock(use, fcntl.LOCK_SH)
        # Both admission and replacement protection precede executable/admin proof.
        readiness = installation.check()
        checked = CheckedServiceGeneration(
            installation,
            readiness,
            use,
            copy.deepcopy(installation.record),
            {
                installation.runtime_path(db): installation.runtime_path(db).read_bytes()
                for db in DATABASES
            },
            getattr(installation, "_daemon_identity", None),
            os.getpid(),
        )
        checked.validate(installation)
    except BaseException:
        if use is not None:
            os.close(use)
        raise
    finally:
        os.close(gate)
    token = _checked_generations.set((*_checked_generations.get(), checked))
    try:
        yield checked
    finally:
        checked.active = False
        _checked_generations.reset(token)
        os.close(use)


def maintenance_owned(installation: Installation, env: Mapping[str, str] | None = None) -> bool:
    row = read_json(installation.directory / "maintenance.json")
    token = (os.environ if env is None else env).get("LCTX_SURREAL_MAINTENANCE_TOKEN")
    if not row or not token or token != row.get("token"):
        return False
    return (
        lock_held(installation.directory / "admission.lock")
        and ProcessIdentity.from_json(row["owner"]).alive()
    )


@contextlib.contextmanager
def maintenance(
    installation: Installation,
    *,
    native_clients: bool = False,
    recovery: Mapping[str, Any] | None = None,
    restart: bool = False,
) -> Iterator[dict[str, str]]:
    root = installation.directory
    gate = os.open(root / "admission.lock", os.O_RDWR | os.O_CLOEXEC)
    try:
        try:
            fcntl.flock(gate, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError:
            raise FixtureBlocked(
                "maintenance",
                "another admission/maintenance operation is active",
                "just service status",
            ) from None
        existing = read_json(root / "maintenance.json")
        if existing:
            raise FixtureBlocked(
                "maintenance",
                "previous maintenance did not finalize; admission stays closed",
                "just service maintenance --recover",
            )
        token = secrets.token_urlsafe(32)
        marker = {
            "token": token,
            "owner": ProcessIdentity.of().to_json(),
            "operation": "maintenance",
        }
        if recovery:
            marker["recovery"] = dict(recovery)
        private_json(root / "maintenance.json", marker)
        try:
            # Block new borrowers first; existing work continues normally until drained.
            while pending := borrowers(installation):
                if any(
                    not lock_held(root / "attachments" / row["id"] / "owner.lock")
                    for row in pending
                ):
                    raise FixtureBlocked(
                        "drainage",
                        "unreleased borrower needs explicit recovery; service left closed",
                        "just fixture --list --json; just fixture --recover ID",
                    )
                print(
                    f"service: waiting for {len(pending)} existing borrowers",
                    file=sys.stderr,
                    flush=True,
                )
                time.sleep(0.5)
            use = os.open(root / "borrowers.lock", os.O_RDWR | os.O_CREAT | os.O_CLOEXEC, 0o600)
            try:
                fcntl.flock(use, fcntl.LOCK_EX)
                if restart:
                    _bootstrap_owned(installation)
                _run_installer(
                    installation, Path(installation.record["installer"]), "close-admission"
                )
                _run_installer(installation, Path(installation.record["installer"]), "drain")
                if native_clients:
                    # Only the explicit command can borrow under this exclusive host lease.
                    # It uses ordinary native pins/attempts, with no compiler bypass token.
                    _run_installer(
                        installation, Path(installation.record["installer"]), "open-admission"
                    )
                try:
                    yield {
                        **os.environ,
                        "LCTX_SURREAL_SERVICE_CONFIG": str(root / "installation.json"),
                        "LCTX_SURREAL_MAINTENANCE_TOKEN": token,
                        "LCTX_SURREAL_INSTALLER_CONFIG": str(
                            installation.runtime_path("validation", installer=True)
                        ),
                    }
                finally:
                    if native_clients:
                        _run_installer(
                            installation, Path(installation.record["installer"]), "close-admission"
                        )
                        _run_installer(
                            installation, Path(installation.record["installer"]), "drain"
                        )
                if not native_clients:
                    _run_installer(installation, Path(installation.record["installer"]), "drain")
                installation.check(allow_maintenance=True)
                if borrowers(installation):
                    raise FixtureBlocked(
                        "drainage",
                        "maintenance command left unresolved borrowers; admission stays closed",
                    )
                _run_installer(
                    installation, Path(installation.record["installer"]), "open-admission"
                )
                (root / "maintenance.json").unlink()
            finally:
                os.close(use)
        except BaseException:
            # A partially opened/reconciled installation must return to closed admission.
            # Existing owners are not cancelled; native close only rejects new entry.
            with contextlib.suppress(Exception):
                _run_installer(
                    installation, Path(installation.record["installer"]), "close-admission"
                )
            # Preserve the marker: interruption/failure does not prove remote effects ended.
            raise
    finally:
        os.close(gate)


RECOVERY_SCHEMA = 1
# These inodes protect current owners. Never copy an old lock over a live maintenance lock.
LIVE_COORDINATION = frozenset({"admission.lock", "borrowers.lock", "maintenance.json"})
RECOVERY_EXCLUDED = LIVE_COORDINATION | {"tmp"}


def _recovery_identity(installation: Installation) -> dict[str, Any]:
    return {
        key: installation.record[key]
        for key in (
            "installation_id",
            "service_generation",
            "unit",
            "state_root",
            "endpoint",
            "grpc_endpoint",
            "namespace",
            "databases",
            "binary",
            "export_batch_size",
        )
    }


def _schema_identities(installation: Installation) -> dict[str, Any]:
    result = {}
    for database in DATABASES:
        rows = sql(
            installation,
            "SELECT generation, schema, schema_version FROM native_installation:current;",
            cfg=installation.runtime(database),
        )
        marker = rows[0].get("result")
        if (
            not isinstance(marker, list)
            or len(marker) != 1
            or marker[0].get("generation") != bytes(installation.record["service_generation"]).hex()
            or marker[0].get("schema_version")
            != installation.record.get("native_schema_version", 3)
            or not re.fullmatch(r"[0-9a-f]{64}", marker[0].get("schema", ""))
        ):
            raise FixtureBlocked("recovery", "native schema identity is unavailable")
        result[database] = marker[0]
    return result


def _coordination_identity(installation: Installation) -> dict[str, Any]:
    result = {}
    for name in ("admission.lock", "borrowers.lock"):
        info = (installation.directory / name).stat()
        if not stat.S_ISREG(info.st_mode) or info.st_uid != os.getuid():
            raise FixtureBlocked("recovery", "coordination lock identity is not owned")
        result[name] = {"device": info.st_dev, "inode": info.st_ino}
    # Host borrowers and native effects are drained separately; no cache lock can be invented
    # as a restored live owner. Refuse any independently held canonical product lease as well.
    directories = [installation.directory / "product-leases"]
    directories.extend(
        Path(asset["path"])
        for asset in _external_recovery_assets(installation)
        if asset["kind"] == "coordination"
    )
    for database in DATABASES:
        cfg = read_json(installation.runtime_path(database))
        if isinstance(cfg, dict) and cfg.get("selection"):
            directories.append(Path(cfg["selection"]).with_suffix(".selection.lock"))
    for directory in directories:
        candidates = [directory] if directory.is_file() else directory.rglob("*.lock")
        for path in candidates:
            # harness.lock_held probes with a shared lock and intentionally answers only for
            # exclusive owner locks. Product readers hold shared leases, so probe exclusively.
            fd = os.open(path, os.O_RDWR | os.O_CLOEXEC | os.O_NOFOLLOW)
            try:
                try:
                    fcntl.flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
                except BlockingIOError:
                    raise FixtureBlocked(
                        "drainage", "a canonical product lease is still held"
                    ) from None
            finally:
                os.close(fd)
    return result


def _stop_owned(installation: Installation) -> None:
    if systemctl("stop", installation.unit).returncode:
        raise FixtureBlocked("recovery", "owned daemon did not stop; no cold copy is permitted")
    state = unit_properties(installation.unit, "ActiveState", "MainPID", "ControlGroup")
    if state.get("ActiveState") not in {"inactive", "failed"} or state.get("MainPID") != "0":
        raise FixtureBlocked("drainage", "owned daemon stop is not confirmed")
    group = state.get("ControlGroup", "")
    if group:
        directory = Path("/sys/fs/cgroup") / group.lstrip("/")
        for path in directory.rglob("cgroup.procs"):
            if path.read_text().strip():
                raise FixtureBlocked(
                    "drainage", "owned daemon descendants remain; no cold copy is permitted"
                )


def _start_owned(installation: Installation) -> None:
    if systemctl("start", installation.unit).returncode:
        raise FixtureBlocked("launch", "owned daemon restart failed; maintenance remains closed")
    deadline = time.monotonic() + READY_TIMEOUT
    while True:
        try:
            installation.check(allow_maintenance=True)
            return
        except FixtureBlocked:
            if time.monotonic() >= deadline:
                raise
            time.sleep(0.1)


def _bootstrap_owned(installation: Installation) -> None:
    """Bootstrap a dead exact installation under both exclusive maintenance locks.

    Native close/drain cannot run before the daemon exists. A live daemon still follows
    the ordinary pre-stop drainage path; identity failures never authorize replacement.
    """
    try:
        installation.check_daemon()
        return
    except FixtureBlocked:
        state = unit_properties(installation.unit, "ActiveState", "MainPID")
        if state.get("ActiveState") not in {"inactive", "failed"}:
            raise
    if not lock_held(installation.directory / "admission.lock") or not lock_held(
        installation.directory / "borrowers.lock"
    ):
        raise FixtureBlocked("ownership", "dead service bootstrap requires exclusive maintenance")
    for database in DATABASES:
        installation.runtime(database, installer=True)
    binary = _installed_binary(installation)
    port = int(installation.endpoint.rsplit(":", 1)[1])
    if _unit_path(installation.unit).read_text() != unit_text(
        installation.directory,
        binary,
        port,
        log_filter=installation.record.get("server_log_filter", "warn"),
    ):
        raise FixtureBlocked("identity", "dead service unit differs from installation")
    _stop_owned(installation)
    _start_owned(installation)
    installation.check_daemon()


def _external_recovery_assets(installation: Installation) -> list[dict[str, Any]]:
    """The installed configs and installation descriptor own external recovery references.

    Never discover unrelated operator state by scanning the host. Canonical service reuse paths
    are inside the state root; any additional coordination consumer must name its owned path.
    """
    declared = installation.record.get("recovery_assets", [])
    if not isinstance(declared, list):
        raise FixtureBlocked("recovery", "external recovery asset declarations are invalid")
    declared = list(declared)
    root = installation.directory.resolve()
    for database in DATABASES:
        cfg = read_json(installation.runtime_path(database))
        if isinstance(cfg, dict) and cfg.get("selection"):
            selected = Path(cfg["selection"])
            if not selected.is_absolute():
                raise FixtureBlocked(
                    "recovery", "runtime selection must have an absolute recovery path"
                )
            if not selected.is_relative_to(root):
                declared.append({"kind": "selection", "path": str(selected)})
                declared.append(
                    {"kind": "coordination", "path": str(selected.with_suffix(".selection.lock"))}
                )
    result = {}
    for asset in declared:
        if (
            not isinstance(asset, dict)
            or asset.get("kind") not in ("selection", "coordination")
            or not isinstance(asset.get("path"), str)
        ):
            raise FixtureBlocked("recovery", "external recovery asset declarations are invalid")
        path = Path(asset["path"])
        if (
            not path.is_absolute()
            or path.resolve() != path
            or path.is_relative_to(root)
            or root.is_relative_to(path)
        ):
            raise FixtureBlocked(
                "recovery", "external recovery asset path is unsafe or aliases owned state"
            )
        if path.exists():
            info = path.lstat()
            if info.st_uid != os.getuid() or not (
                stat.S_ISREG(info.st_mode) or stat.S_ISDIR(info.st_mode)
            ):
                raise FixtureBlocked(
                    "recovery", "external recovery asset is not owned regular state"
                )
            if asset["kind"] == "selection" and not stat.S_ISREG(info.st_mode):
                raise FixtureBlocked("recovery", "external selection must be an owned regular file")
        elif not path.parent.is_dir() or path.parent.stat().st_uid != os.getuid():
            raise FixtureBlocked(
                "recovery", "external recovery asset parent is unavailable or unowned"
            )
        previous = result.get(str(path))
        if previous and previous["kind"] != asset["kind"]:
            raise FixtureBlocked("recovery", "external recovery asset has conflicting owners")
        result[str(path)] = {
            "kind": asset["kind"],
            "path": str(path),
            "existed": path.exists(),
            "directory": path.is_dir(),
        }
    paths = sorted(result)
    if any(
        Path(child).is_relative_to(Path(parent))
        for i, parent in enumerate(paths)
        for child in paths[i + 1 :]
    ):
        raise FixtureBlocked("recovery", "external recovery asset scopes overlap")
    return [result[path] for path in paths]


def _recovery_sources(installation: Installation) -> dict[str, Path]:
    root = installation.directory
    sources = {}
    for path in sorted(root.rglob("*")):
        relative = path.relative_to(root)
        if relative.parts[0] in RECOVERY_EXCLUDED or relative.parts[0].startswith(".recovery-"):
            continue
        if path.is_symlink() or not (path.is_file() or path.is_dir()):
            raise FixtureBlocked(
                "recovery", "owned state contains a link or unsupported file; no copy made"
            )
        if path.is_file():
            sources["state/" + relative.as_posix()] = path
    for name, path in {
        "service-unit": _unit_path(installation.unit),
        "surreal": Path(installation.record["binary"]["path"]),
        "lctx": Path(installation.record["installer"]),
    }.items():
        if path.is_symlink() or not path.is_file():
            raise FixtureBlocked(
                "recovery", f"required recovery asset {name} is not a regular file"
            )
        sources["assets/" + name] = path
    if generation := installation.record["binary"].get("generation"):
        actual = _owned_server_generation(
            installation.directory, Path(generation["descriptor_path"])
        )
        if actual != generation:
            raise FixtureBlocked("identity", "recovery server provenance differs from installation")
        sources["assets/server-generation.json"] = Path(generation["descriptor_path"])
    required = {
        "state/installation.json",
        "state/server.env",
        "assets/service-unit",
        "assets/surreal",
        "assets/lctx",
    }
    required |= {f"state/{db}-{role}.json" for db in DATABASES for role in ("runtime", "installer")}
    if installation.record["binary"].get("generation"):
        required.add("assets/server-generation.json")
    if not required <= sources.keys() or not (root / "data/store").is_dir():
        raise FixtureBlocked("recovery", "recovery closure is incomplete")
    for index, asset in enumerate(_external_recovery_assets(installation)):
        path = Path(asset["path"])
        if not asset["existed"]:
            continue
        candidates = sorted(path.rglob("*")) if asset["directory"] else [path]
        for candidate in candidates:
            if candidate.is_symlink() or not (candidate.is_file() or candidate.is_dir()):
                raise FixtureBlocked(
                    "recovery", "external recovery state contains links or special files"
                )
            if candidate.is_file():
                relative = candidate.relative_to(path).as_posix() if asset["directory"] else "value"
                sources[f"external/{index}/{relative}"] = candidate
    return sources


def _recovery_pointer(
    installation: Installation, archive: Path, digest: str, metadata: Mapping[str, Any]
) -> dict[str, Any]:
    return {
        "schema": RECOVERY_SCHEMA,
        "archive": str(archive),
        "sha256": digest,
        "installation_id": installation.id,
        "service_generation": installation.record["service_generation"],
        "native_schemas": metadata["native_schemas"],
        "obligations": [
            "explicit maintenance and actual drainage before whole-service restore",
            "reconcile restored native effects, attempts and reader pins before admission",
            "external selection and canonical coordination assets are protected, "
            "never live-client authority",
        ],
    }


class _ArchiveHashWriter:
    """Non-seeking tar sink: hash exactly the bytes the underlying writer accepts."""

    def __init__(self, stream):
        self.stream, self.digest, self.count = stream, hashlib.sha256(), 0

    def write(self, value):
        view = memoryview(value)
        while view:
            count = self.stream.write(view)
            if type(count) is not int or not 0 < count <= len(view):
                raise OSError("archive writer made no valid progress")
            self.digest.update(view[:count])
            self.count += count
            view = view[count:]
        return len(value)

    def flush(self):
        self.stream.flush()

    def seek(self, *args):
        raise io.UnsupportedOperation("archive digest requires non-seeking output")


class _AssetHashReader:
    def __init__(self, stream):
        self.stream, self.digest, self.count = stream, hashlib.sha256(), 0

    def read(self, count=-1):
        value = self.stream.read(count)
        self.digest.update(value)
        self.count += len(value)
        return value


def _asset_identity(info):
    return (
        info.st_dev,
        info.st_ino,
        info.st_size,
        info.st_mtime_ns,
        info.st_ctime_ns,
        info.st_mode,
        info.st_uid,
        info.st_gid,
    )


def _archive_asset(tar, name: str, path: Path) -> tuple[dict, tuple]:
    fd = os.open(path, os.O_RDONLY | os.O_CLOEXEC | os.O_NOFOLLOW)
    with os.fdopen(fd, "rb") as source:
        info = os.fstat(source.fileno())
        identity = _asset_identity(info)
        if not stat.S_ISREG(info.st_mode) or _asset_identity(path.lstat()) != identity:
            raise FixtureBlocked("recovery", "recovery asset changed before copy")
        header = tarfile.TarInfo(name)
        header.size, header.mode = info.st_size, stat.S_IMODE(info.st_mode)
        header.uid, header.gid, header.mtime = info.st_uid, info.st_gid, info.st_mtime
        reader = _AssetHashReader(source)
        tar.addfile(header, reader)
        if (
            reader.count != info.st_size
            or _asset_identity(os.fstat(source.fileno())) != identity
            or _asset_identity(path.lstat()) != identity
        ):
            raise FixtureBlocked("recovery", "recovery asset changed during copy")
        return {
            "sha256": reader.digest.hexdigest(),
            "size": reader.count,
            "mode": stat.S_IMODE(info.st_mode),
        }, identity


def _backup_shape(installation: Installation) -> tuple[list[str], list[dict]]:
    directories = sorted(
        path.relative_to(installation.directory).as_posix()
        for path in installation.directory.rglob("*")
        if path.is_dir()
        and path.relative_to(installation.directory).parts[0] not in RECOVERY_EXCLUDED
        and not path.relative_to(installation.directory).parts[0].startswith(".recovery-")
    )
    external = _external_recovery_assets(installation)
    for asset in external:
        path = Path(asset["path"])
        asset["directories"] = (
            (["."] + sorted(p.relative_to(path).as_posix() for p in path.rglob("*") if p.is_dir()))
            if asset["directory"]
            else []
        )
    return directories, external


def backup_service(installation: Installation, archive: Path) -> dict[str, Any]:
    from storage_lifecycle import admission

    with admission([installation.directory, archive.expanduser().absolute()]):
        return _backup_service(installation, archive)


def _backup_service(installation: Installation, archive: Path) -> dict[str, Any]:
    archive = archive.expanduser().absolute()
    archive = archive.parent.resolve() / archive.name
    if archive.is_relative_to(installation.directory.resolve()):
        raise FixtureBlocked("recovery", "protected archive must be outside owned service state")
    if archive.exists() or archive.is_symlink():
        raise FixtureBlocked("recovery", "archive already exists; refusing overwrite")
    # A pointer in an ordinary content backup identifies this private owner asset, not its secrets.
    with maintenance(installation, recovery={"kind": "backup", "archive": str(archive)}):
        installation.check(allow_maintenance=True)
        _run_installer(installation, Path(installation.record["installer"]), "check")
        schemas = _schema_identities(installation)
        locks = _coordination_identity(installation)
        _stop_owned(installation)
        primary = None
        try:
            sources = _recovery_sources(installation)
            inventory, copied = {}, {}
            directories, external_assets = _backup_shape(installation)
            metadata = {
                "schema": RECOVERY_SCHEMA,
                "identity": _recovery_identity(installation),
                "native_schemas": schemas,
                "coordination": locks,
                "files": inventory,
                "external_assets": external_assets,
                "directories": directories,
            }
            fd = os.open(
                archive, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_CLOEXEC | os.O_NOFOLLOW, 0o600
            )
            with os.fdopen(fd, "wb") as stream:
                writer = _ArchiveHashWriter(stream)
                with tarfile.open(fileobj=writer, mode="w|", dereference=True) as tar:
                    for name, path in sources.items():
                        inventory[name], copied[name] = _archive_asset(tar, name, path)
                    if (
                        _recovery_sources(installation) != sources
                        or _backup_shape(installation) != (directories, external_assets)
                        or any(
                            _asset_identity(path.lstat()) != copied[name]
                            for name, path in sources.items()
                        )
                    ):
                        raise FixtureBlocked("recovery", "recovery closure changed during copy")
                    descriptor = json.dumps(metadata, sort_keys=True).encode()
                    header = tarfile.TarInfo("recovery.json")
                    header.size, header.mode = len(descriptor), 0o600
                    tar.addfile(header, io.BytesIO(descriptor))
                # Tar close includes padding and the final trailer. No digest escapes
                # until finalization, flush, file durability and directory durability.
                writer.flush()
                os.fsync(stream.fileno())
            directory_fd = os.open(archive.parent, os.O_RDONLY | os.O_DIRECTORY)
            try:
                os.fsync(directory_fd)
            finally:
                os.close(directory_fd)
            digest = writer.digest.hexdigest()
            pointer = _recovery_pointer(installation, archive, digest, metadata)
            from storage_lifecycle import durable_json

            successor = {**installation.record, "recovery": pointer}
            durable_json(installation.directory / "installation.json", successor)
            installation.record = successor
        except BaseException as error:
            primary = error
            raise
        finally:
            # Even a failed archive must resume only the exact owned service; maintenance keeps
            # native admission closed until its post-effect drain/check succeeds.
            try:
                _start_owned(installation)
            except BaseException as restart:
                if primary is not None:
                    print(
                        "service: cold backup also failed to restart owned daemon "
                        f"({type(restart).__name__}); primary failure preserved",
                        file=sys.stderr,
                    )
                    raise primary from restart
                raise
    return {"outcome": "passed", "recovery": pointer}


def _safe_recovery_path(name: str, *, state_only: bool = False) -> Path:
    path = Path(name)
    if not name or path.is_absolute() or ".." in path.parts or str(path) != name:
        raise FixtureBlocked("recovery", "archive contains an unsafe path")
    if state_only:
        if path.parts[0] in RECOVERY_EXCLUDED or path.parts[0].startswith(".recovery-"):
            raise FixtureBlocked("recovery", "archive attempts to replace live coordination")
    elif path.parts[0] not in ("state", "assets", "external") or len(path.parts) < 2:
        raise FixtureBlocked("recovery", "archive contains an unknown asset path")
    return path


def _stage_recovery(installation: Installation, archive: Path, stage: Path) -> dict[str, Any]:
    info = archive.lstat()
    if (
        not stat.S_ISREG(info.st_mode)
        or info.st_uid != os.getuid()
        or stat.S_IMODE(info.st_mode) & 0o077
    ):
        raise FixtureBlocked("recovery", "protected archive must be an owned private regular file")
    with tarfile.open(archive, "r:") as tar:
        members = tar.getmembers()
        names = [member.name for member in members]
        if len(set(names)) != len(names) or names.count("recovery.json") != 1:
            raise FixtureBlocked("recovery", "archive has duplicate or missing metadata")
        header = tar.getmember("recovery.json")
        if not header.isfile() or header.size > 1024 * 1024 * 64:
            raise FixtureBlocked("recovery", "archive metadata is unsupported")
        header_source = tar.extractfile(header)
        if header_source is None:
            raise FixtureBlocked("recovery", "archive metadata is unavailable")
        with header_source:
            metadata = json.load(header_source)
        if (
            not isinstance(metadata, dict)
            or metadata.get("schema") != RECOVERY_SCHEMA
            or metadata.get("identity") != _recovery_identity(installation)
        ):
            raise FixtureBlocked(
                "identity", "backup belongs to a different installation, generation or service"
            )
        inventory = metadata.get("files")
        if not isinstance(inventory, dict) or set(names) != set(inventory) | {"recovery.json"}:
            raise FixtureBlocked("recovery", "archive inventory does not match its contents")
        external = metadata.get("external_assets", [])
        expected = _external_recovery_assets(installation)
        if (
            not isinstance(external, list)
            or any(not isinstance(asset, dict) for asset in external)
            or [{k: asset.get(k) for k in ("kind", "path")} for asset in external]
            != [{k: asset[k] for k in ("kind", "path")} for asset in expected]
        ):
            raise FixtureBlocked(
                "identity", "backup external recovery owners differ from installation"
            )
        for index, asset in enumerate(external):
            if (
                type(asset.get("existed")) is not bool
                or type(asset.get("directory")) is not bool
                or not isinstance(asset.get("directories"), list)
            ):
                raise FixtureBlocked("recovery", "backup external recovery metadata is invalid")
            for name in asset["directories"]:
                relative = Path(".") if name == "." else _safe_recovery_path(name, state_only=True)
                (stage / "external" / str(index) / relative).mkdir(
                    parents=True, exist_ok=True, mode=0o700
                )
        for name in metadata.get("directories", []):
            relative = _safe_recovery_path(name, state_only=True)
            (stage / "state" / relative).mkdir(parents=True, exist_ok=True, mode=0o700)
        for member in members:
            if member.name == "recovery.json":
                continue
            relative = _safe_recovery_path(member.name)
            if relative.parts[0] == "state":
                _safe_recovery_path(Path(*relative.parts[1:]).as_posix(), state_only=True)
            if relative.parts[0] == "external":
                if (
                    len(relative.parts) < 3
                    or not relative.parts[1].isdigit()
                    or int(relative.parts[1]) >= len(external)
                ):
                    raise FixtureBlocked(
                        "recovery", "archive contains an undeclared external asset"
                    )
                asset = external[int(relative.parts[1])]
                if not asset["existed"] or (
                    not asset["directory"] and relative.parts[2:] != ("value",)
                ):
                    raise FixtureBlocked(
                        "recovery", "archive external asset shape differs from its owner"
                    )
            if not member.isfile() or member.islnk() or member.issym():
                raise FixtureBlocked("recovery", "archive links and special files are forbidden")
            declared = inventory[member.name]
            if (
                not isinstance(declared, dict)
                or type(declared.get("size")) is not int
                or type(declared.get("mode")) is not int
                or not 0 <= declared["mode"] <= 0o777
                or not isinstance(declared.get("sha256"), str)
                or not re.fullmatch(r"[0-9a-f]{64}", declared["sha256"])
                or member.size != declared["size"]
            ):
                raise FixtureBlocked("recovery", "archive file identity is incompatible")
            target = stage / relative
            target.parent.mkdir(parents=True, exist_ok=True, mode=0o700)
            source = tar.extractfile(member)
            if source is None:
                raise FixtureBlocked("recovery", "archive file bytes are unavailable")
            with source, target.open("xb") as output:
                shutil.copyfileobj(source, output)
            target.chmod(declared["mode"] & 0o700)
            if file_sha256(target) != declared["sha256"]:
                raise FixtureBlocked("recovery", "archive asset digest does not match")
    # Archived executable bytes are recovery assets, never a license to replace another unit.
    current_assets = {
        "service-unit": _unit_path(installation.unit),
        "surreal": Path(installation.record["binary"]["path"]),
        "lctx": Path(installation.record["installer"]),
    }
    for name, current in current_assets.items():
        if not current.is_file() or file_sha256(current) != inventory.get("assets/" + name, {}).get(
            "sha256"
        ):
            raise FixtureBlocked(
                "identity", f"current {name} differs from protected recovery asset"
            )
    expected_binary = (
        installation.record["binary"]["sha256"]
        if installation.record["binary"].get("generation")
        else BINARY_SHA256
    )
    if file_sha256(stage / "assets/surreal") != expected_binary:
        raise FixtureBlocked("identity", "backup service binary differs from the pinned release")
    if generation := installation.record["binary"].get("generation"):
        import surrealdb_server

        archived_generation = read_json(stage / "assets/server-generation.json")
        if archived_generation != generation:
            raise FixtureBlocked(
                "identity", "backup server generation provenance differs from installation"
            )
        try:
            # Source/build paths are provenance and warm references, never a cold-restore
            # prerequisite. The archive owns the descriptor and exact executable bytes.
            surrealdb_server.validate_provenance(
                archived_generation["provenance"], directory=installation.directory
            )
        except StorageBlocked as error:
            raise FixtureBlocked(
                "identity", "backup server provenance cannot be verified"
            ) from error
        if (
            file_sha256(stage / "assets/server-generation.json")
            != inventory.get("assets/server-generation.json", {}).get("sha256")
            or (stage / "assets/surreal").stat().st_size != generation["size"]
            or _owned_server_generation(installation.directory, Path(generation["descriptor_path"]))
            != generation
        ):
            raise FixtureBlocked(
                "identity", "backup server executable or provenance identity differs"
            )
    archived = Installation(
        stage / "state", json.loads((stage / "state/installation.json").read_text())
    )
    if _recovery_identity(archived) != _recovery_identity(installation) or archived.record.get(
        "recovery_assets", []
    ) != installation.record.get("recovery_assets", []):
        raise FixtureBlocked("identity", "archived state descriptor differs from recovery identity")
    for db in DATABASES:
        for role in ("runtime", "installer"):
            name = f"{db}-{role}.json"
            if (stage / "state" / name).read_bytes() != (
                installation.directory / name
            ).read_bytes():
                raise FixtureBlocked(
                    "identity", "backup credentials/configuration differ from installed scope"
                )
    if not (stage / "state/data/store").is_dir():
        raise FixtureBlocked("recovery", "cold database recovery closure is missing")
    return metadata


def restore_service(
    installation: Installation, archive: Path, *, apply: bool = False
) -> dict[str, Any]:
    archive = archive.expanduser().absolute()
    # Validation creates only a private temporary staging area. No daemon, database, credential,
    # selection, admission or coordination state is changed without the explicit --apply.
    with tempfile.TemporaryDirectory(prefix="lctx-recovery-") as temporary:
        stage = Path(temporary)
        metadata = _stage_recovery(installation, archive, stage)
        digest = file_sha256(archive)
        if not apply:
            return {
                "outcome": "passed",
                "applied": False,
                "archive": str(archive),
                "sha256": digest,
                "installation_id": installation.id,
                "native_schemas": metadata["native_schemas"],
            }
        current = read_json(installation.directory / "installation.json")
        if not isinstance(current, dict) or current.get("recovery", {}).get("sha256") != digest:
            raise FixtureBlocked(
                "recovery", "apply requires the installation's current approved protected archive"
            )
        with maintenance(installation, recovery={"kind": "restore", "archive": str(archive)}):
            current = read_json(installation.directory / "installation.json")
            if not isinstance(current, dict) or current.get("recovery", {}).get("sha256") != digest:
                raise FixtureBlocked(
                    "recovery", "current approved protected archive changed before maintenance"
                )
            installation.check(allow_maintenance=True)
            if _schema_identities(installation) != metadata["native_schemas"]:
                raise FixtureBlocked(
                    "identity", "backup native schema differs from the installed epoch"
                )
            if _coordination_identity(installation) != metadata["coordination"]:
                raise FixtureBlocked("identity", "backup canonical coordination identity differs")
            root = installation.directory
            # Same-filesystem staging permits owned atomic renames. The live lock inodes and
            # current maintenance token stay at their original paths throughout the transition.
            prepared = root / (".recovery-staging-" + str(uuid.uuid4()))
            predecessor = root / (".recovery-predecessor-" + str(uuid.uuid4()))
            marker = read_json(root / "maintenance.json")
            if not isinstance(marker, dict) or not isinstance(marker.get("recovery"), dict):
                raise FixtureBlocked(
                    "ownership", "protected recovery maintenance owner disappeared"
                )
            marker["recovery"].update(
                {"predecessors": [str(predecessor)], "staging": [str(prepared)]}
            )
            private_json(root / "maintenance.json", marker)
            shutil.copytree(stage / "state", prepared)
            predecessor.mkdir(mode=0o700)
            _stop_owned(installation)
            external_predecessors = []
            try:
                for path in list(root.iterdir()):
                    if path.name in LIVE_COORDINATION or path.name.startswith(".recovery-"):
                        continue
                    if path.name == "installation.json":
                        # Keep the identity/repair owner readable even if interrupted between
                        # payload renames. The replacement descriptor is installed atomically.
                        shutil.copy2(path, predecessor / path.name)
                        continue
                    path.rename(predecessor / path.name)
                for path in list(prepared.iterdir()):
                    path.rename(root / path.name)
                for index, asset in enumerate(metadata["external_assets"]):
                    target = Path(asset["path"])
                    previous = target.with_name(".recovery-predecessor-" + str(uuid.uuid4()))
                    external_stage = target.with_name(".recovery-staging-" + str(uuid.uuid4()))
                    marker["recovery"]["predecessors"].append(str(previous))
                    marker["recovery"]["staging"].append(str(external_stage))
                    private_json(root / "maintenance.json", marker)
                    if asset["existed"]:
                        source = stage / "external" / str(index)
                        if asset["directory"]:
                            shutil.copytree(source, external_stage)
                        else:
                            shutil.copy2(source / "value", external_stage)
                    if target.exists():
                        target.rename(previous)
                        external_predecessors.append(previous)
                    if asset["existed"]:
                        external_stage.rename(target)
                installation.record = json.loads((root / "installation.json").read_text())
                from storage_service import rebind_restored

                installation.record = rebind_restored(
                    installation.record,
                    current,
                    metadata["files"]["assets/lctx"]["sha256"],
                )
                installation.record["recovery"] = _recovery_pointer(
                    installation, archive, digest, metadata
                )
                private_json(root / "installation.json", installation.record)
                (root / "tmp").mkdir(mode=0o700, exist_ok=True)
                _start_owned(installation)
                _run_installer(installation, Path(installation.record["installer"]), "drain")
                _run_installer(installation, Path(installation.record["installer"]), "check")
            except BaseException:
                with contextlib.suppress(Exception):
                    _stop_owned(installation)
                # Both states retain a current recovery consumer until an explicit repair.
                print(
                    f"service: recovery failed; preserved predecessor {predecessor} "
                    f"and staging {prepared}",
                    file=sys.stderr,
                )
                for path in external_predecessors:
                    print(
                        f"service: protected external predecessor retained: {path}", file=sys.stderr
                    )
                raise
            # The replacement is checked and drained, so the predecessor no longer has a
            # recovery consumer. Only these exact newly owned paths may be removed.
            shutil.rmtree(predecessor)
            prepared.rmdir()
            for path in external_predecessors:
                if path.is_dir():
                    shutil.rmtree(path)
                else:
                    path.unlink()
    return {"outcome": "passed", "applied": True, "recovery": installation.record["recovery"]}


def _retire_recovery_predecessors(installation: Installation, marker: Mapping[str, Any]) -> None:
    """After explicit recovery proves the replacement ready, end the predecessor's consumer."""
    recovery = marker.get("recovery", {})
    if not isinstance(recovery, dict) or recovery.get("kind") != "restore":
        return
    parents = {installation.directory.resolve()}
    parents.update(Path(asset["path"]).parent for asset in _external_recovery_assets(installation))
    targets = []
    for key, prefix in (
        ("predecessors", ".recovery-predecessor-"),
        ("staging", ".recovery-staging-"),
    ):
        paths = recovery.get(key, [])
        if not isinstance(paths, list):
            raise FixtureBlocked("recovery", "protected repair asset record is invalid")
        for raw in paths:
            path = Path(raw)
            try:
                uuid.UUID(path.name.removeprefix(prefix))
            except ValueError:
                raise FixtureBlocked(
                    "recovery", "protected repair asset identity is invalid"
                ) from None
            if not path.name.startswith(prefix) or path.parent not in parents or path.is_symlink():
                raise FixtureBlocked(
                    "recovery", "protected repair asset is outside its exact owned scope"
                )
            if path.exists() and path.stat().st_uid != os.getuid():
                raise FixtureBlocked("recovery", "protected repair asset belongs to another owner")
            targets.append(path)
    for path in targets:
        if path.is_dir():
            shutil.rmtree(path)
        elif path.exists():
            path.unlink()


def _installer_cli(
    installation: Installation,
    candidate: Mapping[str, Any],
    database: str,
    action: str,
    *,
    config: Path | None = None,
    keep_closed: bool = False,
) -> dict:
    executable = Path(candidate["path"])
    if (
        any(path.is_symlink() for path in (executable, *executable.parents))
        or not executable.is_file()
        or executable.stat().st_uid != os.getuid()
        or file_sha256(executable) != candidate["sha256"]
        or not os.access(executable, os.X_OK)
    ):
        raise FixtureBlocked("identity", "installer executable generation changed")
    if database not in DATABASES or action not in {
        "schema",
        "init",
        "check",
        "close-admission",
        "drain",
        "open-admission",
    }:
        raise FixtureBlocked("identity", "unsupported owned installer operation")
    args = [
        str(executable),
        "store",
        "--runtime-config",
        str(config or installation.runtime_path(database, installer=True)),
        action,
    ]
    if keep_closed:
        if action != "init":
            raise FixtureBlocked("identity", "closed initialization flag requires init")
        args.append("--keep-closed")
    result = subprocess.run(
        args,
        env={
            **os.environ,
            "LCTX_SURREAL_SERVICE_CONFIG": str(installation.directory / "installation.json"),
        },
        capture_output=True,
        text=True,
        check=False,
    )
    if result.returncode:
        raise FixtureFailed(
            "installer", f"native {action} failed for {database} (exit {result.returncode})"
        )
    try:
        value = json.loads(result.stdout)
    except ValueError, TypeError:
        raise FixtureBlocked("schema", "installer returned no structured result") from None
    if not isinstance(value, dict):
        raise FixtureBlocked("schema", "installer returned no structured result")
    return value


def _current_schema(value: Mapping[str, Any]) -> None:
    if (
        value.get("schema_version") != 7
        or not isinstance(value.get("schema"), str)
        or not re.fullmatch(r"[0-9a-f]{64}", value["schema"])
    ):
        raise FixtureBlocked("schema", "installer must declare the current native format7 schema")


def _owned_daemon_start(installation: Installation) -> None:
    """Start an owned daemon before checking or constructing native schemas."""
    if systemctl("start", installation.unit).returncode:
        raise FixtureBlocked("launch", "owned daemon start failed; admission stays closed")
    deadline = time.monotonic() + READY_TIMEOUT
    while True:
        try:
            installation.check_daemon()
            return
        except FixtureBlocked:
            if time.monotonic() >= deadline:
                raise
            time.sleep(0.1)


def _write_private(path: Path, content: str) -> None:
    from storage_lifecycle import fsync_directory

    temporary = path.with_name("." + path.name + "." + uuid.uuid4().hex)
    fd = os.open(temporary, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600)
    try:
        with os.fdopen(fd, "w") as stream:
            stream.write(content)
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(temporary, path)
        fsync_directory(path.parent)
    finally:
        temporary.unlink(missing_ok=True)


SERVER_HANDOFF_OPERATION = "server generation handoff"
INSTALLER_HANDOFF_OPERATION = "installer generation handoff"


def _installer_handoff_bytes(path: Path) -> bytes:
    if any(part.is_symlink() for part in (path, *path.parents)):
        raise FixtureBlocked("identity", "installer handoff path is not physical")
    info = path.stat()
    if not stat.S_ISREG(info.st_mode) or info.st_uid != os.getuid() or info.st_mode & 0o077:
        raise FixtureBlocked("identity", "installer handoff state must be private and owned")
    return path.read_bytes()


def _installer_handoff_plan(
    installation: Installation, reference: Mapping[str, Any]
) -> tuple[Path, dict]:
    operation = reference.get("operation", "")
    if not isinstance(operation, str) or not re.fullmatch("[a-f0-9]{64}", operation):
        raise FixtureBlocked("identity", "installer handoff operation is unavailable")
    path = installation.directory / "installer-handoffs" / operation / "plan.json"
    raw = _installer_handoff_bytes(path)
    if reference.get("journal") != str(path) or hashlib.sha256(raw).hexdigest() != reference.get(
        "journal_sha256"
    ):
        raise FixtureBlocked("identity", "immutable installer handoff journal changed")
    plan = json.loads(raw)
    if plan.get("operation") != operation or plan.get("installation_id") != installation.id:
        raise FixtureBlocked("identity", "installer handoff belongs to another installation")
    previous = json.loads(base64.b64decode(plan["previous_bytes"], validate=True))
    successor = json.loads(base64.b64decode(plan["successor_bytes"], validate=True))
    if successor != {
        **previous,
        "installer": plan["candidate"]["path"],
        "installer_generation": plan["candidate"],
    }:
        raise FixtureBlocked("identity", "installer handoff changes more than executable identity")
    return path, plan


def _installer_scope_markers(installation: Installation) -> dict:
    """Observe real admission state; never normalize an open acknowledgement to closed."""
    result = {}
    for database in DATABASES:
        rows = sql(
            installation,
            "SELECT generation,schema,schema_version,admission_open FROM native_installation:current;",
            cfg=installation.runtime(database, installer=True),
        )
        values = rows[0].get("result") if len(rows) == 1 else None
        if not isinstance(values, list) or len(values) != 1 or not isinstance(values[0], dict):
            raise FixtureBlocked("schema", "installer handoff native marker is unavailable")
        marker = values[0]
        if (
            set(marker) != {"generation", "schema", "schema_version", "admission_open"}
            or marker["generation"] != bytes(installation.record["service_generation"]).hex()
            or marker["schema_version"] != installation.record.get("native_schema_version", 3)
            or not isinstance(marker["schema"], str)
            or not re.fullmatch("[a-f0-9]{64}", marker["schema"])
            or type(marker["admission_open"]) is not bool
        ):
            raise FixtureBlocked("schema", "installer handoff native generation or schema differs")
        result[database] = dict(marker)
    return result


def _installer_handoff_cli(
    installation: Installation, candidate: Mapping[str, Any], database: str, action: str, token: str
) -> dict:
    """Execute only the immutable, fully checked candidate inode for this handoff."""
    path = Path(candidate["path"])
    if any(part.is_symlink() for part in (path, *path.parents)):
        raise FixtureBlocked("identity", "installer candidate path changed")
    descriptor = os.open(path, os.O_RDONLY | os.O_CLOEXEC | os.O_NOFOLLOW)
    try:
        info = os.fstat(descriptor)
        identity = executable_identity(info)
        if (
            not stat.S_ISREG(info.st_mode)
            or info.st_uid != os.getuid()
            or stat.S_IMODE(info.st_mode) != 0o500
            or info.st_size != candidate["size"]
        ):
            raise FixtureBlocked(
                "identity", "installer candidate is not an immutable owned executable"
            )
        with os.fdopen(os.dup(descriptor), "rb") as stream:
            digest = hashlib.file_digest(stream, "sha256").hexdigest()
        if digest != candidate["sha256"] or executable_identity(path.stat()) != identity:
            raise FixtureBlocked("identity", "installer candidate bytes changed")
        if database not in DATABASES or action not in {
            "schema",
            "close-admission",
            "drain",
            "check",
            "open-admission",
        }:
            raise FixtureBlocked("identity", "unsupported installer handoff operation")
        result = subprocess.run(
            [
                f"/proc/self/fd/{descriptor}",
                "store",
                "--runtime-config",
                str(installation.runtime_path(database, installer=True)),
                action,
            ],
            pass_fds=(descriptor,),
            capture_output=True,
            check=False,
            env={
                **os.environ,
                "LCTX_SURREAL_SERVICE_CONFIG": str(installation.directory / "installation.json"),
                "LCTX_SURREAL_INSTALLER_CONFIG": str(
                    installation.runtime_path(database, installer=True)
                ),
                "LCTX_SURREAL_MAINTENANCE_TOKEN": token,
            },
        )
        if (
            executable_identity(path.stat()) != identity
            or executable_identity(os.fstat(descriptor)) != identity
            or file_sha256(path) != candidate["sha256"]
        ):
            raise FixtureBlocked("identity", "installer candidate changed during execution")
        if result.returncode:
            raise FixtureFailed(
                "installer", f"candidate {action} failed for {database}; admission stays closed"
            )
        if len(result.stdout) > 4096:
            raise FixtureBlocked("schema", "candidate installer response exceeds bound")
        try:
            value = json.loads(result.stdout)
        except ValueError, TypeError:
            raise FixtureBlocked(
                "schema", "candidate installer response is not structured"
            ) from None
        if action == "schema":
            if not isinstance(value, dict):
                raise FixtureBlocked(
                    "schema", "candidate installer response is not a schema object"
                )
            _current_schema(value)
            if set(value) != {"schema", "schema_version"}:
                raise FixtureBlocked("schema", "candidate schema response differs")
        elif value != {
            "ready": action in {"check", "open-admission"},
            "namespace": NAMESPACE,
            "database": database,
        }:
            raise FixtureBlocked("schema", "candidate installer acknowledgement differs")
        return value
    finally:
        os.close(descriptor)


def handoff_installer(installation: Installation, source: Path) -> dict:
    """Replace only the maintenance executable after both unchanged native scopes agree."""
    from storage_lifecycle import admission, durable_json, fsync_directory, private_directory
    from storage_service import prepare, tools_root

    source = source.absolute()
    root = installation.directory
    if any(
        key in os.environ
        for key in (
            "LCTX_NATIVE_DRAIN_DIRECTORY",
            "LCTX_FIXTURE_ATTACHMENT",
            "LCTX_SURREAL_GENERATION",
            "LCTX_SIGNAL_CONTROL_MODE",
        )
    ):
        raise FixtureBlocked(
            "ownership", "installer handoff refuses inherited native owner bindings"
        )
    with admission([root, source, tools_root(root)]):
        gate = os.open(root / "admission.lock", os.O_RDWR | os.O_CLOEXEC)
        use = None
        marker_path = root / "maintenance.json"
        try:
            try:
                fcntl.flock(gate, fcntl.LOCK_EX | fcntl.LOCK_NB)
            except BlockingIOError:
                raise FixtureBlocked(
                    "ownership", "another owner holds installer handoff admission"
                ) from None
            marker = read_json(marker_path)
            if marker:
                owner = ProcessIdentity.from_json(marker["owner"])
                if (
                    marker.get("operation") != INSTALLER_HANDOFF_OPERATION
                    or owner.foreign()
                    or owner.alive()
                    or not marker.get("installer_handoff")
                ):
                    raise FixtureBlocked(
                        "ownership", "resume only the dead exact installer handoff owner"
                    )
            if borrowers(installation):
                raise FixtureBlocked(
                    "drainage", "installer handoff requires resolved host borrowers"
                )
            use = os.open(root / "borrowers.lock", os.O_RDWR | os.O_CLOEXEC)
            try:
                fcntl.flock(use, fcntl.LOCK_EX | fcntl.LOCK_NB)
            except BlockingIOError:
                raise FixtureBlocked(
                    "drainage", "installer handoff borrower lease remains held"
                ) from None
            _stopped_attachment_commands(installation)
            installation.check_daemon()
            candidate = prepare(root, source)
            token = secrets.token_urlsafe(32)
            descriptor_path = root / "installation.json"
            if json.loads(_installer_handoff_bytes(descriptor_path)) != installation.record:
                raise FixtureBlocked(
                    "identity", "installer handoff descriptor differs from its loaded owner"
                )
            if marker:
                journal, plan = _installer_handoff_plan(installation, marker["installer_handoff"])
                if plan["candidate"] != candidate:
                    raise FixtureBlocked(
                        "identity", "resume requires the exact checkpointed installer candidate"
                    )
                if _installer_handoff_bytes(descriptor_path) not in (
                    base64.b64decode(plan["previous_bytes"], validate=True),
                    base64.b64decode(plan["successor_bytes"], validate=True),
                ):
                    raise FixtureBlocked(
                        "identity", "resume requires the exact previous or successor descriptor"
                    )
            else:
                previous_bytes = _installer_handoff_bytes(descriptor_path)
                if json.loads(previous_bytes) != installation.record:
                    raise FixtureBlocked("identity", "installer descriptor changed before handoff")
                if installation.record.get("installer_generation") == candidate:
                    raise FixtureBlocked("identity", "installer candidate is already installed")
                scopes = _installer_scope_markers(installation)
                declared = _installer_handoff_cli(installation, candidate, "main", "schema", token)
                if any(
                    {key: value[key] for key in ("schema", "schema_version")} != declared
                    for value in scopes.values()
                ):
                    raise FixtureBlocked(
                        "schema", "candidate schema differs from an installed native scope"
                    )
                successor = {
                    **installation.record,
                    "installer": candidate["path"],
                    "installer_generation": candidate,
                }
                successor_bytes = (json.dumps(successor, sort_keys=True, indent=2) + "\n").encode()
                operation = hashlib.sha256(secrets.token_bytes(32)).hexdigest()
                private_directory(root / "installer-handoffs")
                directory = root / "installer-handoffs" / operation
                directory.mkdir(parents=True, mode=0o700)
                plan = {
                    "schema": 1,
                    "operation": operation,
                    "installation_id": installation.id,
                    "source": str(source),
                    "candidate": candidate,
                    "previous_bytes": base64.b64encode(previous_bytes).decode(),
                    "successor_bytes": base64.b64encode(successor_bytes).decode(),
                    "scopes": scopes,
                    "configs": {
                        str(installation.runtime_path(db, installer=privileged)): file_sha256(
                            installation.runtime_path(db, installer=privileged)
                        )
                        for db in DATABASES
                        for privileged in (False, True)
                    },
                    "unit_sha256": file_sha256(_unit_path(installation.unit)),
                }
                journal = directory / "plan.json"
                durable_json(journal, plan)
                journal.chmod(0o400)
                for path in (directory, directory.parent, root):
                    fsync_directory(path)
                marker = {
                    "operation": INSTALLER_HANDOFF_OPERATION,
                    "installer_handoff": {
                        "operation": operation,
                        "journal": str(journal),
                        "journal_sha256": file_sha256(journal),
                    },
                }
            marker.update(owner=ProcessIdentity.of().to_json(), token=token)
            durable_json(marker_path, marker)
            marker_bytes = _installer_handoff_bytes(marker_path)

            def verify(*, closed: bool | None = None) -> dict:
                if (
                    _installer_handoff_bytes(marker_path) != marker_bytes
                    or _installer_handoff_bytes(descriptor_path)
                    not in (
                        base64.b64decode(plan["previous_bytes"], validate=True),
                        base64.b64decode(plan["successor_bytes"], validate=True),
                    )
                    or _installer_handoff_plan(installation, marker["installer_handoff"])[1] != plan
                    or file_sha256(_unit_path(installation.unit)) != plan["unit_sha256"]
                    or any(
                        file_sha256(Path(path)) != digest
                        for path, digest in plan["configs"].items()
                    )
                    or file_sha256(Path(candidate["path"])) != candidate["sha256"]
                ):
                    raise FixtureBlocked(
                        "identity", "installer handoff checkpoint or executable changed"
                    )
                installation.check_daemon()
                for database in DATABASES:
                    installation.runtime(database, installer=True)
                    installation.runtime(database)
                actual = _installer_scope_markers(installation)
                for database, value in actual.items():
                    if {key: value[key] for key in ("generation", "schema", "schema_version")} != {
                        key: plan["scopes"][database][key]
                        for key in ("generation", "schema", "schema_version")
                    } or (closed is not None and value["admission_open"] is closed):
                        raise FixtureBlocked(
                            "schema",
                            "installer handoff native scope or admission acknowledgement differs",
                        )
                return actual

            try:
                verify()
                declared = _installer_handoff_cli(installation, candidate, "main", "schema", token)
                if any(
                    {key: value[key] for key in ("schema", "schema_version")} != declared
                    for value in plan["scopes"].values()
                ):
                    raise FixtureBlocked("schema", "checkpointed candidate schema differs")
                for database in DATABASES:
                    verify()
                    _installer_handoff_cli(
                        installation, candidate, database, "close-admission", token
                    )
                verify(closed=True)
                for database in DATABASES:
                    verify(closed=True)
                    _installer_handoff_cli(installation, candidate, database, "drain", token)
                    verify(closed=True)
                    _installer_handoff_cli(installation, candidate, database, "check", token)
                    verify(closed=True)
                checked = {
                    "candidate": candidate,
                    "scopes": verify(closed=True),
                    "plan_sha256": file_sha256(journal),
                }
                durable_json(journal.parent / "checked.json", checked)
                _write_private(
                    descriptor_path,
                    base64.b64decode(plan["successor_bytes"], validate=True).decode(),
                )
                installation.record = json.loads(
                    base64.b64decode(plan["successor_bytes"], validate=True)
                )
                verify(closed=True)
                for database in DATABASES:
                    verify()
                    _installer_handoff_cli(
                        installation, candidate, database, "open-admission", token
                    )
                final = verify(closed=False)
                installation.check(allow_maintenance=True)
                final = verify(closed=False)
                durable_json(
                    journal.parent / "receipt.json",
                    {
                        "outcome": "passed",
                        "operation": plan["operation"],
                        "plan_sha256": file_sha256(journal),
                        "candidate": candidate,
                        "scopes": final,
                    },
                )
                marker_path.unlink()
                fsync_directory(root)
                return {
                    "outcome": "passed",
                    "installation_id": installation.id,
                    "installer_handoff": plan["operation"],
                    "installer": candidate,
                    "admission_open": True,
                }
            except BaseException:
                # A failed open on either scope must not leave a partially reopened successor.
                for database in DATABASES:
                    with contextlib.suppress(Exception):
                        # Never follow a changed config/descriptor/daemon during failure
                        # cleanup. The host marker itself continues to block borrowers.
                        verify()
                        _installer_handoff_cli(
                            installation, candidate, database, "close-admission", token
                        )
                raise
        finally:
            if use is not None:
                os.close(use)
            os.close(gate)


def _server_handoff_plan(
    installation: Installation, reference: Mapping[str, Any]
) -> tuple[Path, dict]:
    operation = reference.get("operation", "")
    if not isinstance(operation, str) or not re.fullmatch(r"[0-9a-f]{64}", operation):
        raise FixtureBlocked("identity", "server handoff operation is unavailable")
    journal = installation.directory / "server-handoffs" / operation / "plan.json"
    try:
        if (
            reference.get("journal") != str(journal)
            or any(path.is_symlink() for path in (journal, *journal.parents))
            or journal.stat().st_uid != os.getuid()
            or journal.stat().st_mode & 0o077
            or journal.parent.stat().st_uid != os.getuid()
            or journal.parent.stat().st_mode & 0o077
            or file_sha256(journal) != reference.get("journal_sha256")
        ):
            raise FixtureBlocked("identity", "immutable server handoff journal changed")
        plan = read_json(journal)
        if plan.get("operation") != operation or plan.get("installation_id") != installation.id:
            raise FixtureBlocked("identity", "server handoff differs from its installation")
        return journal, plan
    except OSError, TypeError, AttributeError, ValueError:
        raise FixtureBlocked(
            "identity", "immutable server handoff journal is unavailable"
        ) from None


def _server_handoff_dependencies(installation: Installation) -> list[dict]:
    """Storage references cover both generations and every retained immutable handoff."""
    import surrealdb_server

    result = []

    def binary_assets(binary):
        if isinstance(binary, dict) and isinstance(binary.get("path"), str):
            result.append({"path": binary["path"], "role": "service-server-executable"})
            if generation := binary.get("generation"):
                surrealdb_server.validate_provenance(
                    generation["provenance"], directory=_server_binary_origin(installation)
                )
                result.extend(surrealdb_server.dependencies(generation))

    binary_assets(installation.record.get("binary"))
    marker = read_json(installation.directory / "maintenance.json")
    references = [
        (
            installation.record.get("server_handoff"),
            installation.record.get("binary"),
            False,
            frozenset(),
        )
    ]
    if isinstance(marker, dict) and marker.get("server_handoff"):
        if marker.get("operation") != SERVER_HANDOFF_OPERATION:
            raise FixtureBlocked(
                "identity", "active server handoff has a different maintenance owner"
            )
        references.append(
            (marker["server_handoff"], installation.record.get("binary"), True, frozenset())
        )
    seen = set()
    while references:
        reference, expected_binary, active, ancestors = references.pop()
        if not reference:
            continue
        journal, plan = _server_handoff_plan(installation, reference)
        successor = _generation_binary(plan["generation"])
        # Retained executable provenance survives rebuilding native databases. Each
        # link still names the exact binary installed by that handoff, while a live
        # handoff may be on either side of its checkpointed physical replacement.
        matches_binary = expected_binary == successor or (
            active and expected_binary == plan["previous_binary"]
        )
        if not matches_binary:
            raise FixtureBlocked("identity", "server handoff executable chain differs")
        if active and plan.get("service_generation") != installation.record["service_generation"]:
            raise FixtureBlocked("identity", "active server handoff native generation differs")
        if plan["operation"] in ancestors:
            raise FixtureBlocked("identity", "server handoff executable chain is cyclic")
        if plan["operation"] in seen:
            continue
        seen.add(plan["operation"])
        result.append({"path": str(journal.parent), "role": "server-handoff-private-assets"})
        binary_assets(plan["previous_binary"])
        binary_assets(_generation_binary(plan["generation"]))
        references.append(
            (
                plan.get("previous_handoff"),
                plan["previous_binary"],
                False,
                ancestors | {plan["operation"]},
            )
        )
    return result


def _server_scope_markers(
    installation: Installation, expected: Mapping[str, Any] | None = None
) -> dict:
    markers = {}
    generation = bytes(installation.record["service_generation"]).hex()
    for db in DATABASES:
        rows = sql(
            installation,
            "SELECT generation,schema,schema_version,admission_open FROM native_installation:current;",
            cfg=installation.runtime(db, installer=True),
        )
        values = rows[0].get("result") if len(rows) == 1 else None
        if not isinstance(values, list) or len(values) != 1 or not isinstance(values[0], dict):
            raise FixtureBlocked("schema", "server handoff native scope marker is unavailable")
        marker = values[0]
        if (
            marker.get("generation") != generation
            or not isinstance(marker.get("schema"), str)
            or not re.fullmatch(r"[0-9a-f]{64}", marker["schema"])
            or type(marker.get("admission_open")) is not bool
            or (
                expected is None
                and marker.get("schema_version")
                != installation.record.get("native_schema_version", 3)
            )
            or (expected is not None and marker != expected[db])
        ):
            raise FixtureBlocked(
                "schema", "server handoff native scope identity or closed admission differs"
            )
        markers[db] = {**marker, "admission_open": False}
    return markers


def prepare_grpc_recovery(installation: Installation) -> dict:
    from surrealdb_pd02_recovery import prepare_logs

    return prepare_logs(installation)


def handoff_server(installation: Installation, descriptor: Path) -> dict:
    """Explicitly switch one proven server generation without migrating native state."""
    from storage_lifecycle import admission, durable_json, fsync_directory

    descriptor = descriptor.absolute()
    with admission([installation.directory, descriptor.parent]):
        generation = _owned_server_generation(installation.directory, descriptor)
        gate = os.open(installation.directory / "admission.lock", os.O_RDWR | os.O_CLOEXEC)
        use = None
        marker_path = installation.directory / "maintenance.json"
        try:
            try:
                fcntl.flock(gate, fcntl.LOCK_EX | fcntl.LOCK_NB)
            except BlockingIOError:
                raise FixtureBlocked(
                    "ownership", "another owner holds server handoff admission"
                ) from None
            marker = read_json(marker_path)
            if marker and (
                marker.get("operation") != SERVER_HANDOFF_OPERATION
                or ProcessIdentity.from_json(marker["owner"]).alive()
            ):
                raise FixtureBlocked(
                    "ownership", "server handoff requires its dead exact maintenance owner"
                )
            if borrowers(installation):
                raise FixtureBlocked("drainage", "server handoff requires resolved host borrowers")
            use = os.open(installation.directory / "borrowers.lock", os.O_RDWR | os.O_CLOEXEC)
            try:
                fcntl.flock(use, fcntl.LOCK_EX | fcntl.LOCK_NB)
            except BlockingIOError:
                raise FixtureBlocked(
                    "drainage", "a server handoff borrower lease remains held"
                ) from None
            resumed = bool(marker and marker.get("server_handoff"))
            port = int(installation.endpoint.rsplit(":", 1)[1])
            if resumed:
                journal, plan = _server_handoff_plan(installation, marker["server_handoff"])
                if plan.get("service_generation") != installation.record["service_generation"]:
                    raise FixtureBlocked(
                        "identity", "resume requires the exact checkpointed native generation"
                    )
                if generation != plan["generation"]:
                    raise FixtureBlocked(
                        "identity", "resume requires the exact checkpointed server generation"
                    )
            else:
                previous = dict(installation.record["binary"])
                previous_path = _installed_binary(installation)
                if previous == _generation_binary(generation):
                    raise FixtureBlocked("identity", "server generation is already installed")
                expected_unit = unit_text(
                    installation.directory,
                    previous_path,
                    port,
                    log_filter=installation.record.get("server_log_filter", "warn"),
                )
                if _unit_path(installation.unit).read_text() != expected_unit:
                    raise FixtureBlocked(
                        "identity", "owned service unit differs before server handoff"
                    )
                installation.check_daemon()
                scopes = _server_scope_markers(installation)
                operation = hashlib.sha256(secrets.token_bytes(32)).hexdigest()
                directory = installation.directory / "server-handoffs" / operation
                directory.mkdir(parents=True, mode=0o700)
                plan = {
                    "schema": 1,
                    "operation": operation,
                    "installation_id": installation.id,
                    "service_generation": installation.record["service_generation"],
                    "previous_binary": previous,
                    "generation": generation,
                    "previous_unit": expected_unit,
                    "successor_unit": unit_text(
                        installation.directory,
                        Path(generation["path"]),
                        port,
                        log_filter=installation.record.get("server_log_filter", "warn"),
                    ),
                    "previous_handoff": installation.record.get("server_handoff"),
                    "scopes": scopes,
                }
                journal = directory / "plan.json"
                durable_json(journal, plan)
                for path in (directory, directory.parent, installation.directory):
                    fsync_directory(path)
                marker = marker or {"operation": SERVER_HANDOFF_OPERATION}
                marker["server_handoff"] = {
                    "operation": operation,
                    "journal": str(journal),
                    "journal_sha256": file_sha256(journal),
                    "phase": "prepared",
                }
            marker.update(owner=ProcessIdentity.of().to_json(), token=secrets.token_urlsafe(32))
            durable_json(marker_path, marker)
            if installation.record["binary"] not in (
                plan["previous_binary"],
                _generation_binary(generation),
            ):
                raise FixtureBlocked("identity", "server handoff installed generation changed")
            if _unit_path(installation.unit).read_text() not in (
                plan["previous_unit"],
                plan["successor_unit"],
            ):
                raise FixtureBlocked(
                    "identity", "server handoff unit changed outside its exact journal"
                )
            _stopped_attachment_commands(installation)
            if not resumed:
                _run_installer(
                    installation, Path(installation.record["installer"]), "close-admission"
                )
                _run_installer(installation, Path(installation.record["installer"]), "drain")
            # This also confirms a core-dumped daemon has no surviving descendants.
            _stop_owned(installation)
            _write_private(_unit_path(installation.unit), plan["successor_unit"])
            if systemctl("daemon-reload").returncode:
                raise FixtureBlocked(
                    "systemd", "server handoff daemon-reload failed; admission stays closed"
                )
            reference = {
                key: marker["server_handoff"][key]
                for key in ("operation", "journal", "journal_sha256")
            }
            installation.record.update(
                binary=_generation_binary(generation), server_handoff=reference
            )
            durable_json(installation.directory / "installation.json", installation.record)
            marker["server_handoff"]["phase"] = "installed"
            durable_json(marker_path, marker)
            _owned_daemon_start(installation)
            _run_installer(installation, Path(installation.record["installer"]), "close-admission")
            _run_installer(installation, Path(installation.record["installer"]), "drain")
            checked = {"scopes": _server_scope_markers(installation, plan["scopes"])}
            durable_json(journal.parent / "checked.json", checked)
            installation.check(allow_maintenance=True)
            try:
                _run_installer(
                    installation, Path(installation.record["installer"]), "open-admission"
                )
            except BaseException:
                with contextlib.suppress(Exception):
                    _run_installer(
                        installation, Path(installation.record["installer"]), "close-admission"
                    )
                raise
            marker_path.unlink()
            fsync_directory(installation.directory)
            return {
                "outcome": "passed",
                "installation_id": installation.id,
                "service_generation": installation.record["service_generation"],
                "server_generation": generation["identity"],
                "server_handoff": plan["operation"],
                "admission_open": True,
            }
        finally:
            if use is not None:
                os.close(use)
            os.close(gate)


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(prog="just service", description=__doc__)
    commands = parser.add_subparsers(dest="action", required=True)
    install_parser = commands.add_parser(
        "install", help="explicitly install stable service and native schemas"
    )
    install_parser.add_argument(
        "--campaign",
        action="store_true",
        help="install an isolated checked campaign unit at an explicit LCTX_SURREAL_SERVICE_CONFIG",
    )
    install_parser.add_argument("--installer", type=Path, default=ROOT / "target/release/lctx")
    install_parser.add_argument(
        "--purpose",
        choices=[LIFECYCLE_PURPOSE],
        help="register lifecycle purpose/consumer before owned campaign launch",
    )
    install_parser.add_argument(
        "--server-generation",
        type=Path,
        help="borrow this exact immutable patched generation for a lifecycle campaign",
    )
    dispose_parser = commands.add_parser(
        "dispose-campaign", help="checked whole owned lifecycle target disposal"
    )
    dispose_parser.add_argument("installation_id")
    commands.add_parser("check", help="read-only compatibility and native readiness")
    commands.add_parser("status", help="read-only ownership, state and maintenance observation")
    backup_parser = commands.add_parser(
        "backup", help="maintenance-owned cold whole-service protected recovery archive"
    )
    backup_parser.add_argument("archive", type=Path)
    restore_parser = commands.add_parser(
        "restore",
        help="validate protected archive; "
        "--apply explicitly restores this exact owned installation",
    )
    restore_parser.add_argument("archive", type=Path)
    restore_parser.add_argument("--apply", action="store_true")
    maintenance_parser = commands.add_parser(
        "maintenance", help="close admission, drain borrowers and run selected operation"
    )
    operations = maintenance_parser.add_mutually_exclusive_group()
    operations.add_argument(
        "--recover-attachment",
        metavar="ID",
        help="explicit named PD02 unknown recovery with exact joined server invocation",
    )
    operations.add_argument(
        "--recover-native-attachment",
        metavar="ID",
        action="append",
        help="campaign-only Started native owner recovery with source-correlated registered shutdown join",
    )
    maintenance_parser.add_argument(
        "--legacy-attachment-run",
        metavar="ID=RUN",
        action="append",
        default=[],
        help="preserve the explicitly named historical local-release peer's original ended-run evidence",
    )
    operations.add_argument(
        "--prepare-grpc-recovery",
        action="store_true",
        help="explicit campaign-only positive shutdown-log preparation; requires resolved borrowers",
    )
    operations.add_argument("--restart", action="store_true")
    operations.add_argument("--check", action="store_true")
    operations.add_argument(
        "--recover",
        action="store_true",
        help="revalidate a failed operation after borrower/native recovery",
    )
    operations.add_argument(
        "--stabilize-installer",
        action="store_true",
        help="transfer the exact maintenance executable out of its supplying checkout",
    )
    operations.add_argument(
        "--handoff-installer",
        type=Path,
        help="explicitly replace the maintenance executable for unchanged installed native schemas",
    )
    operations.add_argument(
        "--server-generation",
        type=Path,
        help="explicitly hand off the owned server to a reviewed immutable descriptor",
    )
    maintenance_parser.add_argument(
        "--reconcile-database",
        choices=DATABASES,
        help="explicit installed scope for named reader reconciliation during --recover",
    )
    maintenance_parser.add_argument(
        "--reconcile-pin",
        type=_native_identity_argument,
        action="append",
        default=[],
        help="exact abandoned native pin identity, repeatable; "
        "requires --recover and --reconcile-database",
    )
    maintenance_parser.add_argument(
        "--reconcile-backup-hold",
        type=_native_identity_argument,
        action="append",
        default=[],
        help="exact abandoned backup hold identity, repeatable; "
        "requires --recover and --reconcile-database",
    )
    maintenance_parser.add_argument(
        "--native-clients",
        action="store_true",
        help="allow ordinary native clients only inside the explicit command "
        "under exclusive host maintenance",
    )
    maintenance_parser.add_argument("command", nargs=argparse.REMAINDER)
    args = parser.parse_args(argv)
    reconcile = args.action == "maintenance" and bool(
        args.reconcile_pin or args.reconcile_backup_hold
    )
    if args.action == "maintenance":
        if args.legacy_attachment_run and not args.recover_native_attachment:
            parser.error("--legacy-attachment-run requires --recover-native-attachment")
        if (
            (reconcile and (not args.recover or not args.reconcile_database))
            or (args.reconcile_database and not reconcile)
            or (
                (
                    args.recover
                    or args.server_generation
                    or args.handoff_installer
                    or args.recover_attachment
                    or args.recover_native_attachment
                    or args.prepare_grpc_recovery
                )
                and args.command
            )
        ):
            parser.error(
                "named reader reconciliation requires --recover, --reconcile-database "
                "and an explicit inventory, without a command"
            )
        if len(set(args.reconcile_pin)) != len(args.reconcile_pin) or len(
            set(args.reconcile_backup_hold)
        ) != len(args.reconcile_backup_hold):
            parser.error("named reconciliation identities must be distinct")
    if args.action == "maintenance" and args.native_clients:
        command = args.command[1:] if args.command[:1] == ["--"] else args.command
        if (
            not command
            or args.restart
            or args.check
            or args.recover
            or args.stabilize_installer
            or args.handoff_installer
            or args.server_generation
            or args.recover_attachment
            or args.recover_native_attachment
            or args.prepare_grpc_recovery
        ):
            parser.error("--native-clients requires only an explicit command after --")
    try:
        if args.action == "install":
            value = install(
                args.installer.resolve(),
                campaign=args.campaign,
                purpose=args.purpose,
                server_generation=args.server_generation,
            ).check()
        else:
            installation = Installation.load(
                ready=args.action not in {"status", "dispose-campaign"}
            )
            if args.action == "status":
                marker = read_json(installation.directory / "maintenance.json")
                value = {
                    "installation_id": installation.id,
                    "phase": installation.record["phase"],
                    "unit": installation.unit,
                    "state": unit_properties(installation.unit, "ActiveState", "Result"),
                    "maintenance": (installation.directory / "maintenance.json").exists(),
                    "server": {
                        "path": installation.record["binary"]["path"],
                        "sha256": installation.record["binary"]["sha256"],
                        "version": installation.record["binary"]["version"],
                        "generation": installation.record["binary"]
                        .get("generation", {})
                        .get("identity"),
                    },
                    "server_handoff": marker.get("server_handoff")
                    if isinstance(marker, dict)
                    else None,
                    "protected_repair_assets": marker.get("recovery")
                    if isinstance(marker, dict)
                    else None,
                    "borrowers": [
                        {k: r.get(k) for k in ("id", "checkout", "owner", "protected")}
                        for r in borrowers(installation)
                    ],
                }
                from storage_service import dependencies

                value["storage"] = dependencies(installation)
                if installation.record.get("purpose") == LIFECYCLE_PURPOSE:
                    value["purpose"] = LIFECYCLE_PURPOSE
                    value["development_workers"] = development_workers(installation)
                    value["disposal"] = read_json(installation.directory / "disposal.json")
                    value["native_inspection"] = {
                        "status": "not_inspected",
                        "reason": "host status grants no database admission",
                        "route": "docs/surrealdb.md#maintenance-diagnostics",
                    }
                    value["observed_at"] = time.time()
            elif args.action == "dispose-campaign":
                value = dispose_campaign(installation, args.installation_id)
            elif args.action == "check":
                value = installation.check()
            elif args.action == "backup":
                value = backup_service(installation, args.archive)
            elif args.action == "restore":
                value = restore_service(installation, args.archive, apply=args.apply)
            else:
                command = args.command[1:] if args.command[:1] == ["--"] else args.command
                if args.recover_attachment:
                    from surrealdb_pd02_recovery import recover

                    print(json.dumps(recover(installation, args.recover_attachment), indent=2))
                    return 0
                if args.recover_native_attachment:
                    from surrealdb_pd02_recovery import recover_native

                    mappings = {}
                    for value in args.legacy_attachment_run:
                        identity, separator, run = value.partition("=")
                        if not separator or identity in mappings:
                            parser.error(
                                "--legacy-attachment-run requires distinct ID=RUN mappings"
                            )
                        mappings[identity] = run
                    print(
                        json.dumps(
                            recover_native(
                                installation, args.recover_native_attachment, legacy_runs=mappings
                            ),
                            indent=2,
                        )
                    )
                    return 0
                if args.prepare_grpc_recovery:
                    print(json.dumps(prepare_grpc_recovery(installation), indent=2))
                    return 0
                if args.server_generation:
                    print(
                        json.dumps(handoff_server(installation, args.server_generation), indent=2)
                    )
                    return 0
                if args.handoff_installer:
                    print(
                        json.dumps(
                            handoff_installer(installation, args.handoff_installer), indent=2
                        )
                    )
                    return 0
                if args.recover:
                    gate = hold_lock(installation.directory / "admission.lock")
                    use = None
                    native_started = False
                    try:
                        row = read_json(installation.directory / "maintenance.json")
                        if row and row.get("operation") in {
                            "PD02 named attachment recovery",
                            "PD02 campaign log preparation",
                            "native named attachment recovery",
                        }:
                            raise FixtureBlocked(
                                "maintenance", "resume the exact named PD02 maintenance operation"
                            )
                        if row and (
                            row.get("server_handoff")
                            or row.get("operation") == SERVER_HANDOFF_OPERATION
                        ):
                            raise FixtureBlocked(
                                "maintenance",
                                "incomplete server handoff requires the exact --server-generation descriptor",
                            )
                        if row and (
                            row.get("installer_handoff")
                            or row.get("operation") == INSTALLER_HANDOFF_OPERATION
                        ):
                            raise FixtureBlocked(
                                "maintenance",
                                "incomplete installer handoff requires the exact --handoff-installer candidate",
                            )
                        if row and ProcessIdentity.from_json(row["owner"]).alive():
                            raise FixtureBlocked("ownership", "maintenance owner is still alive")
                        if borrowers(installation):
                            raise FixtureBlocked(
                                "drainage",
                                "borrowers remain unresolved",
                                "just fixture --recover ID",
                            )
                        use = os.open(
                            installation.directory / "borrowers.lock", os.O_RDWR | os.O_CLOEXEC
                        )
                        try:
                            fcntl.flock(use, fcntl.LOCK_EX | fcntl.LOCK_NB)
                        except BlockingIOError:
                            raise FixtureBlocked(
                                "drainage", "a borrower lease remains held"
                            ) from None
                        if reconcile:
                            row = row or {}
                            row.update(
                                {
                                    "owner": ProcessIdentity.of().to_json(),
                                    "token": secrets.token_urlsafe(32),
                                    "operation": "reader reconciliation",
                                    "reconciliation": {
                                        "database": args.reconcile_database,
                                        "pins": args.reconcile_pin,
                                        "backup_holds": args.reconcile_backup_hold,
                                    },
                                }
                            )
                            private_json(installation.directory / "maintenance.json", row)
                        for db in DATABASES:
                            installation.runtime(db, installer=True)
                        _bootstrap_owned(installation)
                        native_started = True
                        _run_installer(
                            installation, Path(installation.record["installer"]), "close-admission"
                        )
                        if reconcile:
                            _stopped_attachment_commands(installation)
                            _reconcile_readers(
                                installation,
                                args.reconcile_database,
                                args.reconcile_pin,
                                args.reconcile_backup_hold,
                            )
                        _run_installer(
                            installation, Path(installation.record["installer"]), "drain"
                        )
                        installation.check(allow_maintenance=True)
                        _run_installer(
                            installation, Path(installation.record["installer"]), "check"
                        )
                        if row:
                            _retire_recovery_predecessors(installation, row)
                        try:
                            _run_installer(
                                installation,
                                Path(installation.record["installer"]),
                                "open-admission",
                            )
                        except BaseException:
                            with contextlib.suppress(Exception):
                                _run_installer(
                                    installation,
                                    Path(installation.record["installer"]),
                                    "close-admission",
                                )
                            raise
                        (installation.directory / "maintenance.json").unlink(missing_ok=True)
                    except BaseException:
                        if native_started:
                            with contextlib.suppress(Exception):
                                _run_installer(
                                    installation,
                                    Path(installation.record["installer"]),
                                    "close-admission",
                                )
                        raise
                    finally:
                        if use is not None:
                            os.close(use)
                        os.close(gate)
                else:
                    if args.stabilize_installer:
                        if command:
                            parser.error("--stabilize-installer cannot take a command")
                        from storage_service import stabilize

                        print(json.dumps(stabilize(installation), indent=2))
                        return 0
                    if not (args.restart or args.check or command):
                        parser.error("select --restart, --check, --recover or a command after --")
                    with maintenance(
                        installation, native_clients=args.native_clients, restart=args.restart
                    ) as env:
                        if args.restart:
                            if systemctl("restart", installation.unit).returncode:
                                raise FixtureBlocked("launch", "explicit restart failed")
                            deadline = time.monotonic() + READY_TIMEOUT
                            while True:
                                try:
                                    installation.check(allow_maintenance=True)
                                    break
                                except FixtureBlocked:
                                    if time.monotonic() >= deadline:
                                        raise
                                    time.sleep(0.1)
                        elif command:
                            from surrealdb_fixture import Attachment, Server, run_attached

                            attachment = Attachment.create(
                                Server(installation), maintenance_env=env
                            )
                            try:
                                proof = {
                                    key: env[key]
                                    for key in (
                                        "LCTX_SURREAL_SERVICE_CONFIG",
                                        "LCTX_SURREAL_MAINTENANCE_TOKEN",
                                        "LCTX_SURREAL_INSTALLER_CONFIG",
                                    )
                                }
                                code = run_attached(attachment, command, child_env=proof)
                                if code:
                                    raise FixtureFailed(
                                        "maintenance",
                                        f"selected command failed with exit {code}; "
                                        "admission stays closed",
                                    )
                            finally:
                                attachment.release()
                        _run_installer(
                            installation, Path(installation.record["installer"]), "check"
                        )
                value = installation.check()
        print(json.dumps(value, indent=2))
        return 0
    except StorageBlocked as error:
        print(f"service: blocked: {error}", file=sys.stderr)
        return EXIT_BLOCKED
    except FixtureBlocked as error:
        print("service: " + error.message().removeprefix("fixture: "), file=sys.stderr)
        return EXIT_BLOCKED
    except FixtureFailed as error:
        print("service: " + error.message().removeprefix("fixture: "), file=sys.stderr)
        return 1
    except KeyboardInterrupt:
        return 130
    except (OSError, ValueError, KeyError, TypeError, AttributeError, tarfile.TarError) as error:
        # Recovery/configuration parser failures must not disclose credential-bearing bytes.
        print(
            f"service: failed: owned state operation ({type(error).__name__}); "
            "admission may remain closed",
            file=sys.stderr,
        )
        return 1


if __name__ == "__main__":
    # Helpers import this owner by its canonical name. Keep the CLI's classes
    # and checked-generation context identical to that import, so a transient
    # readiness refusal is caught by the same owner's startup retry loop.
    sys.modules["surrealdb_service"] = sys.modules[__name__]
    raise SystemExit(main())
