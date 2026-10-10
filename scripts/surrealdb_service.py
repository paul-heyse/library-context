#!/usr/bin/env python3
"""One durable, explicitly installed library-context SurrealDB user service.

Ordinary callers only observe and borrow the installation. Installation, schema changes,
restart and privileged validation diagnostics are explicit maintenance operations. State,
credentials and borrower records are shared by all checkouts outside the repository.
"""

from __future__ import annotations

import argparse
import base64
import contextlib
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
EXIT_BLOCKED = 75
READY_TIMEOUT = 40.0
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


def _descriptor(root: Path) -> dict[str, Any]:
    value = read_json(root / "installation.json")
    if not isinstance(value, dict):
        raise FixtureBlocked("installation", f"no installed service at {root}", INSTALL_REPAIR)
    expected = {
        "schema": SCHEMA,
        "unit": UNIT,
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
        binary = verify_binary({**os.environ, "LCTX_SURREAL_BIN": self.record["binary"]["path"]})
        state = unit_properties(UNIT, "ActiveState", "Result", "MainPID", "ExecStart")
        if state.get("ActiveState") != "active":
            raise FixtureBlocked(
                "readiness",
                f"owned service is {state.get('ActiveState', 'unknown')}",
                "just service maintenance --restart",
            )
        try:
            exe = Path(f"/proc/{int(state['MainPID'])}/exe").resolve(strict=True)
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
            if expected_store not in argv or expected_bind not in argv:
                raise FixtureBlocked(
                    "identity", "running service storage/endpoint differs from installation"
                )
            with urllib.request.urlopen(self.endpoint + "/ready", timeout=2) as response:
                if response.status != 200:
                    raise ValueError("not ready")
            with urllib.request.urlopen(self.endpoint + "/version", timeout=2) as response:
                if response.read().decode().strip() != VERSION:
                    raise FixtureBlocked("identity", "running server version differs from pin")
        except OSError, ValueError, KeyError:
            raise FixtureBlocked(
                "readiness",
                "owned server did not answer readiness",
                "just service maintenance --check",
            ) from None

    def check(self, *, allow_maintenance: bool = False) -> dict[str, Any]:
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
                {"generation": bytes(self.record["service_generation"]).hex(), "schema_version": 3}
            ]:
                raise FixtureBlocked(
                    "schema",
                    "native installed generation/schema does not match",
                    "just service maintenance --check",
                )
        return {
            "installation_id": self.id,
            "service_generation": self.record["service_generation"],
            "unit": UNIT,
            "endpoint": self.endpoint,
            "namespace": NAMESPACE,
            "databases": list(DATABASES),
            "state_root": str(self.directory),
            "outcome": "passed",
        }


def sql(
    installation: Installation, statement: str, *, cfg: Mapping[str, Any]
) -> list[dict[str, Any]]:
    credentials = f"{cfg['username']}:{cfg['password']}".encode()
    headers = {
        "Authorization": "Basic " + base64.b64encode(credentials).decode(),
        "Accept": "application/json",
        "Content-Type": "text/plain",
        "Surreal-NS": NAMESPACE,
        "Surreal-DB": cfg["database"],
    }
    if cfg.get("authentication") == "database":
        headers.update({"Surreal-Auth-NS": NAMESPACE, "Surreal-Auth-DB": cfg["database"]})
    request = urllib.request.Request(
        installation.endpoint + "/sql", data=statement.encode(), headers=headers
    )
    try:
        with urllib.request.urlopen(request, timeout=25) as response:
            rows = json.load(response)
        if not isinstance(rows, list) or not rows or any(r.get("status") != "OK" for r in rows):
            raise FixtureFailed("query", "native statement failed; SQL and credentials withheld")
        return rows
    except OSError, ValueError:
        raise FixtureBlocked(
            "query", "native request failed; credentials and response withheld"
        ) from None


def unit_text(root: Path, binary: Path, port: int) -> str:
    if "\n" in str(root) or "\r" in str(root):
        raise FixtureBlocked("configuration", "service state path cannot contain line separators")

    def quote(value: str | Path) -> str:
        return '"' + str(value).replace("\\", "\\\\").replace('"', '\\"').replace("%", "%%") + '"'

    argv = [
        binary,
        "start",
        "--no-banner",
        "--log=warn",
        "--bind",
        f"127.0.0.1:{port}",
        "--query-timeout",
        "20s",
        "--transaction-timeout",
        "10s",
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


def _unit_path() -> Path:
    return (
        Path(os.environ.get("XDG_CONFIG_HOME") or Path.home() / ".config") / "systemd/user" / UNIT
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
        result = subprocess.run(
            [str(installer), "store", "--runtime-config", str(path), action],
            env={
                **os.environ,
                "LCTX_SURREAL_SERVICE_CONFIG": str(installation.directory / "installation.json"),
            },
            check=False,
        )
        if result.returncode:
            raise FixtureFailed(
                "schema",
                f"native {action} failed for {db} (exit {result.returncode}); "
                "installation remains closed",
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


def install(installer: Path, *, env: Mapping[str, str] | None = None) -> Installation:
    env = os.environ if env is None else env
    root = state_root(env).resolve()
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
            if installation.record["phase"] == "ready":
                installation.check()
                return installation
            if borrowers(installation):
                raise FixtureBlocked(
                    "ownership", "incomplete installation has unresolved borrowers"
                )
        else:
            state = unit_properties(UNIT, "LoadState", "ActiveState")
            if state.get("LoadState") != "not-found" or _unit_path().exists():
                raise FixtureBlocked(
                    "ownership", "stable unit exists without this installation; refusing adoption"
                )
            for name in ("data", "tmp", "attachments", "serving"):
                (root / name).mkdir(mode=0o700)
            installation_id = str(uuid.uuid4())
            generation = list(secrets.token_bytes(32))
            port = _available_port()
            from storage_service import prepare

            installer_generation = prepare(root, installer)
            installer = Path(installer_generation["path"])
            record = {
                "schema": SCHEMA,
                "installation_id": installation_id,
                "phase": "installing",
                "service_generation": generation,
                "unit": UNIT,
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
            installation = Installation(root, record)
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
                    "service_generation": generation,
                    "authentication": "database",
                    "selection": str(root / f"{db}-selected.json"),
                }
                private_json(installation.runtime_path(db), cfg)
                private_json(
                    installation.runtime_path(db, installer=True),
                    {**cfg, "username": root_user, "password": root_pass, "authentication": "root"},
                )
            unit = _unit_path()
            unit.parent.mkdir(parents=True, exist_ok=True)
            unit.write_text(unit_text(root, binary, port))
            result = systemctl("daemon-reload")
            if result.returncode:
                raise FixtureBlocked("systemd", "daemon-reload failed", SYSTEMD_REPAIR)
        result = systemctl("enable", "--now", UNIT)
        if result.returncode:
            raise FixtureBlocked("launch", "owned service launch failed", "just service status")
        deadline = time.monotonic() + READY_TIMEOUT
        while True:
            try:
                with urllib.request.urlopen(
                    installation.endpoint + "/ready", timeout=1
                ) as response:
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
        elif row.get("released") is not True:
            found.append({**row, "protected": True})
    return found


@contextlib.contextmanager
def borrow(installation: Installation) -> Iterator[int | None]:
    root = installation.directory
    if maintenance_owned(installation):
        yield None
        return
    gate = os.open(root / "admission.lock", os.O_RDWR | os.O_CLOEXEC)
    try:
        try:
            fcntl.flock(gate, fcntl.LOCK_SH | fcntl.LOCK_NB)
        except BlockingIOError:
            raise FixtureBlocked(
                "maintenance", "borrower admission is closed", "just service status"
            ) from None
        installation.check()
        use = os.open(root / "borrowers.lock", os.O_RDWR | os.O_CREAT | os.O_CLOEXEC, 0o600)
        fcntl.flock(use, fcntl.LOCK_SH)
    finally:
        os.close(gate)
    try:
        yield use
    finally:
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
            or marker[0].get("schema_version") != 3
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
    if systemctl("stop", UNIT).returncode:
        raise FixtureBlocked("recovery", "owned daemon did not stop; no cold copy is permitted")
    state = unit_properties(UNIT, "ActiveState", "MainPID", "ControlGroup")
    if state.get("ActiveState") != "inactive" or state.get("MainPID") != "0":
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
    if systemctl("start", UNIT).returncode:
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
        "service-unit": _unit_path(),
        "surreal": Path(installation.record["binary"]["path"]),
        "lctx": Path(installation.record["installer"]),
    }.items():
        if path.is_symlink() or not path.is_file():
            raise FixtureBlocked(
                "recovery", f"required recovery asset {name} is not a regular file"
            )
        sources["assets/" + name] = path
    required = {
        "state/installation.json",
        "state/server.env",
        "assets/service-unit",
        "assets/surreal",
        "assets/lctx",
    }
    required |= {f"state/{db}-{role}.json" for db in DATABASES for role in ("runtime", "installer")}
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


def backup_service(installation: Installation, archive: Path) -> dict[str, Any]:
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
            inventory = {
                name: {
                    "sha256": file_sha256(path),
                    "size": path.stat().st_size,
                    "mode": stat.S_IMODE(path.stat().st_mode),
                }
                for name, path in sources.items()
            }
            metadata = {
                "schema": RECOVERY_SCHEMA,
                "identity": _recovery_identity(installation),
                "native_schemas": schemas,
                "coordination": locks,
                "files": inventory,
                "external_assets": _external_recovery_assets(installation),
                "directories": [
                    p.relative_to(installation.directory).as_posix()
                    for p in installation.directory.rglob("*")
                    if p.is_dir()
                    and p.relative_to(installation.directory).parts[0] not in RECOVERY_EXCLUDED
                    and not p.relative_to(installation.directory).parts[0].startswith(".recovery-")
                ],
            }
            for asset in metadata["external_assets"]:
                path = Path(asset["path"])
                asset["directories"] = (
                    ["."] + [p.relative_to(path).as_posix() for p in path.rglob("*") if p.is_dir()]
                    if asset["directory"]
                    else []
                )
            descriptor = json.dumps(metadata, sort_keys=True).encode()
            fd = os.open(archive, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_CLOEXEC, 0o600)
            with (
                os.fdopen(fd, "wb") as stream,
                tarfile.open(fileobj=stream, mode="w", dereference=True) as tar,
            ):
                header = tarfile.TarInfo("recovery.json")
                header.size, header.mode = len(descriptor), 0o600
                tar.addfile(header, io.BytesIO(descriptor))
                for name, path in sources.items():
                    tar.add(path, arcname=name, recursive=False)
            with archive.open("rb") as durable:
                os.fsync(durable.fileno())
            directory_fd = os.open(archive.parent, os.O_RDONLY | os.O_DIRECTORY)
            try:
                os.fsync(directory_fd)
            finally:
                os.close(directory_fd)
            digest = file_sha256(archive)
            pointer = _recovery_pointer(installation, archive, digest, metadata)
            installation.record["recovery"] = pointer
            private_json(installation.directory / "installation.json", installation.record)
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
        "service-unit": _unit_path(),
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
    if file_sha256(stage / "assets/surreal") != BINARY_SHA256:
        raise FixtureBlocked("identity", "backup service binary differs from the pinned release")
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


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(prog="just service", description=__doc__)
    commands = parser.add_subparsers(dest="action", required=True)
    install_parser = commands.add_parser(
        "install", help="explicitly install stable service and native schemas"
    )
    install_parser.add_argument("--installer", type=Path, default=ROOT / "target/release/lctx")
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
        if (
            (reconcile and (not args.recover or not args.reconcile_database))
            or (args.reconcile_database and not reconcile)
            or (args.recover and args.command)
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
        if not command or args.restart or args.check or args.recover or args.stabilize_installer:
            parser.error("--native-clients requires only an explicit command after --")
    try:
        if args.action == "install":
            value = install(args.installer.resolve()).check()
        else:
            installation = Installation.load(ready=args.action != "status")
            if args.action == "status":
                marker = read_json(installation.directory / "maintenance.json")
                value = {
                    "installation_id": installation.id,
                    "phase": installation.record["phase"],
                    "unit": UNIT,
                    "state": unit_properties(UNIT, "ActiveState", "Result"),
                    "maintenance": (installation.directory / "maintenance.json").exists(),
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
            elif args.action == "check":
                value = installation.check()
            elif args.action == "backup":
                value = backup_service(installation, args.archive)
            elif args.action == "restore":
                value = restore_service(installation, args.archive, apply=args.apply)
            else:
                command = args.command[1:] if args.command[:1] == ["--"] else args.command
                if args.recover:
                    gate = hold_lock(installation.directory / "admission.lock")
                    use = None
                    native_started = False
                    try:
                        row = read_json(installation.directory / "maintenance.json")
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
                        installation.check_daemon()
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
                    with maintenance(installation, native_clients=args.native_clients) as env:
                        if args.restart:
                            if systemctl("restart", UNIT).returncode:
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
    raise SystemExit(main())
