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


def _owned_server_generation(directory: Path, descriptor: Path) -> dict:
    import surrealdb_server
    try:
        generation = surrealdb_server.validate_descriptor(descriptor)
        surrealdb_server.validate_provenance(generation["provenance"], directory=directory)
        return generation
    except StorageBlocked as error:
        raise FixtureBlocked("binary", "owned server generation cannot be verified: " + str(error)) from None


def _generation_binary(generation: Mapping[str, Any]) -> dict:
    return {"path": generation["path"], "sha256": generation["sha256"],
            "version": generation["http_version"], "generation": dict(generation)}


def _installed_binary(installation: Installation) -> Path:
    binary = installation.record["binary"]
    if generation := binary.get("generation"):
        actual = _owned_server_generation(installation.directory, Path(generation["descriptor_path"]))
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
        if generation := value["binary"].get("generation"):
            valid_binary = value["binary"] == _generation_binary(
                _owned_server_generation(root, Path(generation["descriptor_path"])))
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
        binary = _installed_binary(self)
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
            with local_urlopen(self.endpoint + "/ready", timeout=2) as response:
                if response.status != 200:
                    raise ValueError("not ready")
            with local_urlopen(self.endpoint + "/version", timeout=2) as response:
                if response.read().decode().strip() != self.record["binary"]["version"]:
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
                {"generation": bytes(self.record["service_generation"]).hex(), "schema_version": self.record.get("native_schema_version", 3)}
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


class _NoLocalRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        # An owned daemon response cannot move credentials or administrative SQL
        # to a second endpoint. Let urllib report the original redirect as failure.
        return None


def local_urlopen(request: str | urllib.request.Request, *, timeout: float):
    """Contact the owned loopback HTTP service directly, independent of proxy env."""
    url = request.full_url if isinstance(request, urllib.request.Request) else request
    endpoint = urllib.parse.urlsplit(url)
    if (endpoint.scheme != "http" or endpoint.hostname not in {"127.0.0.1", "localhost", "::1"}
            or endpoint.username is not None or endpoint.password is not None
            or endpoint.port is None):
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
        "Surreal-NS": NAMESPACE,
        "Surreal-DB": cfg["database"],
    }
    if cfg.get("authentication") == "database":
        headers.update({"Surreal-Auth-NS": NAMESPACE, "Surreal-Auth-DB": cfg["database"]})
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
            declared = _upgrade_cli(installation, installer_generation, "main", "schema")
            _upgrade_source(declared)
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
                with local_urlopen(
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
                ("just service maintenance --upgrade-installer " + existing["upgrade"]["candidate"])
                if existing.get("operation") == UPGRADE_OPERATION else "just service maintenance --recover",
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
            or marker[0].get("schema_version") != installation.record.get("native_schema_version", 3)
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


def _bootstrap_owned(installation: Installation) -> None:
    """Bootstrap a dead exact installation under both exclusive maintenance locks.

    Native close/drain cannot run before the daemon exists. A live daemon still follows
    the ordinary pre-stop drainage path; identity failures never authorize replacement.
    """
    try:
        installation.check_daemon()
        return
    except FixtureBlocked:
        state = unit_properties(UNIT, "ActiveState", "MainPID")
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
    if _unit_path().read_text() != unit_text(installation.directory, binary, port):
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
        "service-unit": _unit_path(),
        "surreal": Path(installation.record["binary"]["path"]),
        "lctx": Path(installation.record["installer"]),
    }.items():
        if path.is_symlink() or not path.is_file():
            raise FixtureBlocked(
                "recovery", f"required recovery asset {name} is not a regular file"
            )
        sources["assets/" + name] = path
    if generation := installation.record["binary"].get("generation"):
        actual = _owned_server_generation(installation.directory, Path(generation["descriptor_path"]))
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
    expected_binary = (installation.record["binary"]["sha256"]
                       if installation.record["binary"].get("generation") else BINARY_SHA256)
    if file_sha256(stage / "assets/surreal") != expected_binary:
        raise FixtureBlocked("identity", "backup service binary differs from the pinned release")
    if generation := installation.record["binary"].get("generation"):
        import surrealdb_server
        archived_generation = read_json(stage / "assets/server-generation.json")
        if archived_generation != generation:
            raise FixtureBlocked("identity", "backup server generation provenance differs from installation")
        try:
            # Source/build paths are provenance and warm references, never a cold-restore
            # prerequisite. The archive owns the descriptor and exact executable bytes.
            surrealdb_server.validate_provenance(archived_generation["provenance"], directory=installation.directory)
        except StorageBlocked as error:
            raise FixtureBlocked("identity", "backup server provenance cannot be verified") from error
        if (file_sha256(stage / "assets/server-generation.json") != inventory.get("assets/server-generation.json", {}).get("sha256")
                or (stage / "assets/surreal").stat().st_size != generation["size"]
                or _owned_server_generation(installation.directory, Path(generation["descriptor_path"])) != generation):
            raise FixtureBlocked("identity", "backup server executable or provenance identity differs")
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



UPGRADE_SOURCE_SCHEMA = "464537d364bfabdc43acd2ca3f05037d6f9c29558ff7d72bdfdcea23f5870580"
UPGRADE_SOURCES = {
    4: {"schema_version": 3, "schema": UPGRADE_SOURCE_SCHEMA},
    5: {"schema_version": 4, "schema": "0870158e2d5568650845dd61f61884384c44315071f0b0dc402d02994a4798da"},
}
UPGRADE_OPERATION = "native-schema-upgrade"


def _upgrade_source(target: Mapping[str, Any]) -> dict:
    version = target.get("schema_version")
    if (type(version) is not int or version not in UPGRADE_SOURCES
            or not isinstance(target.get("schema"), str)
            or not re.fullmatch(r"[0-9a-f]{64}", target.get("schema", ""))):
        raise FixtureBlocked("schema", "candidate does not declare a supported exact native transition")
    return dict(UPGRADE_SOURCES[version])


def _upgrade_executable(candidate: Mapping[str, Any]) -> Path:
    executable = Path(candidate["path"])
    if (any(path.is_symlink() for path in (executable, *executable.parents))
            or not executable.is_file() or executable.stat().st_uid != os.getuid()
            or file_sha256(executable) != candidate["sha256"]
            or not os.access(executable, os.X_OK)):
        raise FixtureBlocked("identity", "upgrade executable generation changed")
    return executable


def _upgrade_cli(installation: Installation, candidate: Mapping[str, Any], database: str,
                 action: str, *, config: Path | None = None, operation: str | None = None,
                 native_operation: str | None = None, execution_contract: Path | None = None,
                 stop_after_pages: int | None = None, expected_schema: str = UPGRADE_SOURCE_SCHEMA) -> dict:
    executable = _upgrade_executable(candidate)
    args = [str(executable), "store", "--runtime-config",
            str(config or installation.runtime_path(database, installer=True)), action]
    if action in ("upgrade", "upgrade-contract", "upgrade-qualify"):
        args.extend(["--expected-schema", expected_schema, "--operation", native_operation or operation])
    if action == "upgrade-contract":
        args.extend(["--execution", operation])
    if execution_contract is not None:
        if action not in ("upgrade", "upgrade-qualify"):
            raise FixtureBlocked("identity", "execution contracts apply only to native upgrade execution")
        args.extend(["--execution-contract", str(execution_contract)])
    if stop_after_pages is not None:
        if execution_contract is None or database != "validation" or stop_after_pages <= 0:
            raise FixtureBlocked("identity", "upgrade stepping requires the owned validation execution contract")
        args.extend(["--stop-after-pages", str(stop_after_pages)])
    result = subprocess.run(args, capture_output=True, text=True, check=False,
                            env={**os.environ, "LCTX_SURREAL_SERVICE_CONFIG":
                                 str(installation.directory / "installation.json")})
    if result.returncode:
        diagnostic = None
        if (operation and re.fullmatch(r"[0-9a-f]{64}", operation) and config is not None
                and database in DATABASES):
            directory = installation.directory / "native-upgrades" / operation
            expected = directory / f"{database}-new-installer.json"
            if (config == expected and not any(path.is_symlink() for path in (config, *config.parents))
                    and config.is_file() and config.stat().st_uid == os.getuid()
                    and not config.stat().st_mode & 0o077 and directory.stat().st_uid == os.getuid()
                    and not directory.stat().st_mode & 0o077):
                diagnostic = directory / f"failure-{database}-{action}-{uuid.uuid4().hex}.json"
                _upgrade_write_private(diagnostic, json.dumps({"schema": 1, "operation": operation,
                    "native_operation": native_operation or operation,
                    "database": database, "action": action, "installer_sha256": candidate["sha256"],
                    "returncode": result.returncode, "stdout": result.stdout, "stderr": result.stderr}, indent=2) + "\n")
        detail = f"native {action} failed for {database} (exit {result.returncode}); admission stays closed"
        if diagnostic is not None:
            detail += f"; private diagnostic: {diagnostic}"
        raise FixtureFailed("upgrade", detail)
    try:
        value = json.loads(result.stdout)
    except (ValueError, TypeError):
        raise FixtureFailed("upgrade", "native upgrade response is unavailable; admission stays closed") from None
    if not isinstance(value, dict):
        raise FixtureFailed("upgrade", "native upgrade response is not an object")
    return value


def _upgrade_daemon_start(installation: Installation) -> None:
    """Partial schema migration can only use daemon readiness, never whole-schema readiness."""
    if systemctl("start", UNIT).returncode:
        raise FixtureBlocked("launch", "upgrade daemon start failed; admission stays closed")
    deadline = time.monotonic() + READY_TIMEOUT
    while True:
        try:
            installation.check_daemon()
            return
        except FixtureBlocked:
            if time.monotonic() >= deadline:
                raise
            time.sleep(0.1)


def _upgrade_restart(installation: Installation) -> None:
    # Even after an unknown RPC acknowledgment, confirmed process drainage precedes replay.
    binary = _installed_binary(installation)
    port = int(installation.endpoint.rsplit(":", 1)[1])
    if _unit_path().read_text() != unit_text(installation.directory, binary, port):
        raise FixtureBlocked("identity", "upgrade service unit differs from installation")
    _stopped_attachment_commands(installation)
    _stop_owned(installation)
    _upgrade_daemon_start(installation)


def _upgrade_authenticates(installation: Installation, cfg: Mapping[str, Any]) -> bool:
    """Only an explicit HTTP authentication refusal establishes that a password is rejected."""
    try:
        sql(installation, "RETURN true;", cfg=cfg)
        return True
    except FixtureBlocked as error:
        if isinstance(error.__context__, urllib.error.HTTPError) and error.__context__.code in (401, 403):
            return False
        raise


def _upgrade_user_definition(installation: Installation, cfg: Mapping[str, Any],
                             username: str, scope: str) -> dict:
    if not re.fullmatch(r"[A-Za-z_][A-Za-z_0-9]*", username):
        raise FixtureBlocked("identity", "owned upgrade username is not a simple identifier")
    rows = sql(installation, f"INFO FOR USER {username} ON {scope} STRUCTURE;", cfg=cfg)
    value = rows[0].get("result")
    if not isinstance(value, dict):
        raise FixtureBlocked("identity", "owned user definition is unavailable")
    return value


def _upgrade_rotate(installation: Installation, root_cfg: Mapping[str, Any],
                    old: Mapping[str, Any], new: Mapping[str, Any], *, scope: str,
                    role: str, operation: str) -> None:
    username = new["username"]
    comment = f"lctx-native-upgrade/{operation}/{scope}/{username}"
    def confirmed() -> bool:
        if not _upgrade_authenticates(installation, new):
            return False
        definition = _upgrade_user_definition(installation,
            new if scope == "ROOT" else root_cfg, username, scope)
        if (definition.get("name") != username or definition.get("roles") != [role]
                or definition.get("comment") != comment):
            raise FixtureBlocked("identity", "upgrade credential has an unexpected user definition")
        if _upgrade_authenticates(installation, old):
            raise FixtureBlocked("identity", "previous owned password remains accepted")
        return True
    if confirmed():
        return
    if not _upgrade_authenticates(installation, old):
        raise FixtureBlocked("identity", "neither checkpointed credential authenticates; admission stays closed")
    # SurrealDB3.3 OVERWRITE generates a fresh signing code as well as password material.
    # INFO omits signing secrets; auth + the exact operation comment reconcile lost replies.
    if not re.fullmatch(r"[A-Za-z_][A-Za-z_0-9]*", username):
        raise FixtureBlocked("identity", "owned upgrade username is not a simple identifier")
    try:
        sql(installation, f"DEFINE USER OVERWRITE {username} ON {scope} "
            f"PASSWORD {json.dumps(new['password'])} ROLES {role} COMMENT {json.dumps(comment)};",
            cfg=root_cfg)
    except (FixtureBlocked, FixtureFailed):
        if confirmed():
            return
        raise
    if not confirmed():
        raise FixtureBlocked("identity", "rotated owned credential is not confirmed")


def _upgrade_write_private(path: Path, content: str) -> None:
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


def _upgrade_private_plan(installation: Installation, descriptor: Mapping[str, Any]) -> tuple[Path, dict]:
    operation = descriptor.get("operation", "")
    if not isinstance(operation, str) or not re.fullmatch(r"[0-9a-f]{64}", operation):
        raise FixtureBlocked("identity", "immutable upgrade operation is unavailable")
    expected = installation.directory / "native-upgrades" / operation / "plan.json"
    journal = Path(descriptor.get("journal", ""))
    try:
        invalid = (journal != expected or any(path.is_symlink() for path in (journal, *journal.parents))
                   or not journal.is_file() or journal.stat().st_uid != os.getuid()
                   or journal.stat().st_mode & 0o077
                   or file_sha256(journal) != descriptor.get("journal_sha256"))
        plan = read_json(journal) if not invalid else None
    except OSError:
        plan = None
    if not isinstance(plan, dict):
        raise FixtureBlocked("identity", "immutable upgrade journal changed")
    if (plan.get("operation") != operation or plan.get("installation_id") != installation.id
            or plan.get("service_generation") != installation.record["service_generation"]
            or plan.get("candidate", {}).get("sha256") != descriptor.get("candidate_sha256")
            or plan.get("candidate", {}).get("path") != descriptor.get("candidate")):
        raise FixtureBlocked("identity", "upgrade journal differs from installation")
    native_operation = plan.get("native_operation", operation)
    if (not isinstance(native_operation, str) or not re.fullmatch(r"[0-9a-f]{64}", native_operation)
            or descriptor.get("native_operation", operation) != native_operation):
        raise FixtureBlocked("identity", "upgrade native migration identity differs from its journal")
    completed = plan.get("completed_scopes", [])
    if (not isinstance(completed, list) or any(db not in DATABASES for db in completed)
            or len(set(completed)) != len(completed)):
        raise FixtureBlocked("identity", "upgrade completed scope inventory is unavailable")
    source = _upgrade_source(plan.get("target", {}))
    if (plan.get("source") != {db: source for db in DATABASES}
            or plan.get("previous_installer") != descriptor.get("previous_installer")):
        raise FixtureBlocked("identity", "upgrade journal source or target is unavailable")
    if plan.get("recovery_mode") == "reconcile" or "execution_contracts" in plan:
        contracts = plan.get("execution_contracts")
        if not isinstance(contracts, dict) or set(contracts) != set(DATABASES):
            raise FixtureBlocked("identity", "reconciliation execution contracts are unavailable")
        for db in DATABASES:
            _validate_upgrade_contract(contracts[db], plan, db)
            path = journal.parent / f"{db}-execution.json"
            if (path.is_symlink() or not path.is_file() or path.stat().st_uid != os.getuid()
                    or path.stat().st_mode & 0o077 or read_json(path) != contracts[db]):
                raise FixtureBlocked("identity", "checkpointed upgrade execution contract changed")
    return journal, plan


def _validate_upgrade_contract(contract: Mapping[str, Any], plan: Mapping[str, Any], db: str) -> None:
    """Validate non-secret scope identities without inventing compiler-owned digests."""
    identities = {"migration", "execution", "source", "target", "generation", "overlay",
                  "protocol", "preflight", "verifier"}
    additions = {"source_version", "kind"} if isinstance(contract, dict) and contract.get("format") == 2 else set()
    if (not isinstance(contract, dict) or set(contract) != identities | {"format", "namespace", "database"} | additions
            or type(contract.get("format")) is not int
            or (contract.get("format"), plan["target"]["schema_version"]) not in {(1, 4), (2, 5)}):
        raise FixtureBlocked("identity", "native upgrade execution contract shape differs")
    if additions and (type(contract["source_version"]) is not int
            or contract["source_version"] != 4 or contract["kind"] != "history_page_receipts_v5"):
        raise FixtureBlocked("identity", "native upgrade transition contract differs")
    for key in identities:
        value = contract[key]
        if (not isinstance(value, list) or len(value) != 32
                or any(type(byte) is not int or not 0 <= byte <= 255 for byte in value)):
            raise FixtureBlocked("identity", "native upgrade execution contract identity is unavailable")
    expected = {"migration": plan["native_operation"], "execution": plan["operation"],
                "source": plan["source"][db]["schema"], "target": plan["target"]["schema"],
                "generation": bytes(plan["service_generation"]).hex()}
    if (any(bytes(contract[key]).hex() != value for key, value in expected.items())
            or contract["database"] != db or contract["namespace"] != plan["new_root"][db]["namespace"]):
        raise FixtureBlocked("identity", "native upgrade execution contract differs from immutable host scope")


def _upgrade_execution_contracts(installation: Installation, candidate: Mapping[str, Any],
                                plan: Mapping[str, Any]) -> dict:
    contracts = {}
    for db in DATABASES:
        contract = _upgrade_cli(installation, candidate, db, "upgrade-contract",
            operation=plan["operation"], native_operation=plan["native_operation"],
            expected_schema=plan["source"][db]["schema"])
        _validate_upgrade_contract(contract, plan, db)
        contracts[db] = contract
    return contracts


def _upgrade_replacement_preflight(installation: Installation, plans: Sequence[Mapping[str, Any]],
                                   checkpoints: Mapping[str, Any], *, reconcile: bool = False) -> dict:
    """Observe publication and every predecessor intent after confirmed daemon drainage.

    Unpublished scopes permit only absent/intent journals. Published scopes require the
    completed host checkpoint and the same exact native migration's published target.
    """
    current = plans[-1]
    native_operation = current.get("native_operation", current["operation"])
    generation = bytes(current["service_generation"]).hex()
    observed = {plan["operation"]: {} for plan in plans}
    for db in DATABASES:
        cfg = current["new_root"][db]
        published = checkpoints.get("scope-" + db) is True
        expected = {"generation": generation,
                    "schema": current["target"]["schema"] if published else current["source"][db]["schema"],
                    "schema_version": current["target"]["schema_version"] if published else current["source"][db]["schema_version"], "admission_open": False}
        rows = sql(installation,
                   "SELECT generation,schema,schema_version,admission_open FROM native_installation:current;",
                   cfg=cfg)
        actual_marker = rows[0].get("result") if len(rows) == 1 else None
        sealed_uncheckpointed = (reconcile and not published and actual_marker == [{**expected,
            "schema": current["target"]["schema"], "schema_version": current["target"]["schema_version"]}])
        if actual_marker != [expected] and not sealed_uncheckpointed:
            raise FixtureBlocked("schema", f"replacement requires exact closed schema{current['source'][db]['schema_version']} source or checkpointed schema{current['target']['schema_version']} target markers")
        info = sql(installation, "INFO FOR DB STRUCTURE;", cfg=cfg)
        structure = info[0].get("result") if len(info) == 1 else None
        tables = structure.get("tables") if isinstance(structure, dict) else None
        if (not isinstance(tables, list)
                or any(not isinstance(table, dict) or not isinstance(table.get("name"), str) for table in tables)
                or len({table["name"] for table in tables}) != len(tables)):
            raise FixtureBlocked("identity", "replacement native journal inventory is unavailable")
        native_by_migration = {}
        for plan in plans:
            operation = plan["operation"]
            migration = plan.get("native_operation", operation)
            completed_migration = (published or sealed_uncheckpointed) and migration == native_operation
            if migration not in native_by_migration:
                native = []
                if any(table["name"] == "native_upgrade" for table in tables):
                    rows = sql(installation,
                               f"SELECT id,generation,source,target,phase FROM native_upgrade:{migration};", cfg=cfg)
                    native = rows[0].get("result") if len(rows) == 1 else None
                native_by_migration[migration] = native
            native = native_by_migration[migration]
            expected_journal = {"id": "native_upgrade:" + migration, "generation": generation,
                                "source": plan["source"][db]["schema"],
                                "target": plan["target"]["schema"],
                                "phase": "published" if completed_migration else "intent"}
            recognized_partial = (reconcile and migration == native_operation and not completed_migration
                and isinstance(native, list) and len(native) == 1
                and isinstance(native[0], dict)
                and native[0].get("phase") in {"intent", "declarations",
                    "verified" if plan["source"][db]["schema_version"] == 4 else "translated"}
                and native[0] == {**expected_journal, "phase": native[0]["phase"]})
            if ((completed_migration and plan["target"] != current["target"])
                    or (native != [expected_journal] and not recognized_partial
                        and (completed_migration or native != []))):
                raise FixtureBlocked("schema", "replacement refuses changed or advanced native upgrade journals")
            observed[operation][db] = native
    return observed


def _upgrade_predecessor_observations(descriptor: Mapping[str, Any], ancestor: Mapping[str, Any],
                                     current: Mapping[str, Any], observed: Mapping[str, Any], *, reconcile: bool = False) -> None:
    """Historical observations stay immutable; only the retained migration can advance."""
    before = descriptor.get("native_journals")
    if observed == before:
        return
    migration = current.get("native_operation", current["operation"])
    if (ancestor.get("native_operation", ancestor["operation"]) != migration
            or ancestor["target"] != current["target"] or not isinstance(before, dict)
            or set(before) != set(DATABASES)):
        raise FixtureBlocked("identity", "replacement predecessor native intent changed")
    for db in DATABASES:
        previous, actual = before[db], observed[db]
        if previous == actual:
            continue
        if not previous and isinstance(previous, list) and actual:
            continue  # The retained migration may have established its exact intent.
        if (reconcile and isinstance(previous, list) and isinstance(actual, list) and len(previous) == len(actual) == 1
                and isinstance(previous[0], dict) and isinstance(actual[0], dict)):
            phases = {"intent": 0, "declarations": 1,
                "verified" if current["source"][db]["schema_version"] == 4 else "translated": 2,
                "published": 3}
            old, new = previous[0], actual[0]
            if (old.get("phase") in phases and new.get("phase") in phases
                    and phases[old["phase"]] <= phases[new["phase"]]
                    and {**old, "phase": new["phase"]} == new):
                continue
        if (isinstance(previous, list) and len(previous) == 1
                and isinstance(previous[0], dict) and previous[0].get("phase") == "intent"
                and actual == [{**previous[0], "phase": "published"}]):
            continue  # Preflight already proved checkpoint, target marker and generation.
        raise FixtureBlocked("identity", "replacement predecessor native intent changed")


def upgrade_installer(installation: Installation, source: Path, *, replace: bool = False,
                      reconcile: bool = False, stop_after_pages: int | None = None) -> dict[str, Any]:
    """Explicit same-generation3→4 migration; immutable private plan, durable owned checkpoints."""
    from storage_lifecycle import admission, durable_json, fsync_directory
    from storage_service import prepare, tools_root
    if replace and reconcile:
        raise FixtureBlocked("identity", "replacement and reconciliation are distinct upgrade routes")
    if stop_after_pages is not None and (type(stop_after_pages) is not int or stop_after_pages <= 0):
        raise FixtureBlocked("identity", "upgrade stepping requires a positive checkpoint count")
    source = source.absolute()
    if (any(path.is_symlink() for path in (source, *source.parents))
            or not source.is_file() or source.stat().st_uid != os.getuid()
            or not os.access(source, os.X_OK)):
        raise FixtureBlocked("identity", "upgrade installer must have a physical regular path")
    # Storage admission precedes native owner locks and lives through every installer child.
    with admission([installation.directory, tools_root(installation.directory), source]):
        gate = os.open(installation.directory / "admission.lock", os.O_RDWR | os.O_CLOEXEC)
        use = None
        marker_path = installation.directory / "maintenance.json"
        try:
            try:
                fcntl.flock(gate, fcntl.LOCK_EX | fcntl.LOCK_NB)
            except BlockingIOError:
                raise FixtureBlocked("ownership", "another maintenance owner holds admission") from None
            marker = read_json(marker_path)
            resumed = bool(marker)
            if marker:
                if marker.get("server_handoff"):
                    raise FixtureBlocked("maintenance", "finish the exact --server-generation handoff before native upgrade resume")
                if marker.get("operation") != UPGRADE_OPERATION:
                    raise FixtureBlocked("maintenance", "another maintenance operation requires its own recovery")
                if ProcessIdentity.from_json(marker["owner"]).alive():
                    raise FixtureBlocked("ownership", "upgrade owner is still alive")
                upgrade = marker["upgrade"]
                if not (replace or reconcile) and file_sha256(source) != upgrade["candidate_sha256"]:
                    raise FixtureBlocked("identity", "resume requires the exact checkpointed upgrade installer")
                journal, plan = _upgrade_private_plan(installation, upgrade)
                if any(upgrade.get("checkpoints", {}).get("scope-" + db) is not True
                       for db in plan.get("completed_scopes", [])):
                    raise FixtureBlocked("identity", "upgrade lost an immutable completed scope checkpoint")
                if upgrade.get("predecessors", []) != plan.get("predecessors", []):
                    raise FixtureBlocked("identity", "upgrade predecessor inventory differs from its journal")
                if stop_after_pages is not None and (not upgrade["checkpoints"].get("scope-main")
                        or upgrade["checkpoints"].get("scope-validation")
                        or (not reconcile and plan.get("recovery_mode") != "reconcile")):
                    raise FixtureBlocked("identity", "upgrade stepping requires reconciliation-owned incomplete validation and completed main")
            else:
                if stop_after_pages is not None:
                    raise FixtureBlocked("identity", "upgrade stepping requires an existing reconciliation-owned upgrade")
                if replace or reconcile:
                    raise FixtureBlocked("maintenance", "replacement requires an incomplete explicit native upgrade")
                if installation.record.get("native_schema_version", 3) not in (3, 4):
                    raise FixtureBlocked("schema", "explicit upgrade requires a supported exact source installation")
                # Validate every private scope before creating any native effect.
                old_runtime = {db: installation.runtime(db) for db in DATABASES}
                old_root = {db: installation.runtime(db, installer=True) for db in DATABASES}
                if len({(cfg["username"], cfg["password"]) for cfg in old_root.values()}) != 1:
                    raise FixtureBlocked("identity", "owned root credentials disagree across upgrade scopes")
                candidate = prepare(installation.directory, source)
                target = _upgrade_cli(installation, candidate, "main", "schema")
                source_identity = _upgrade_source(target)
                if installation.record.get("native_schema_version", 3) != source_identity["schema_version"]:
                    raise FixtureBlocked("schema", "candidate source version differs from the installed generation")
                operation = hashlib.sha256(secrets.token_bytes(32)).hexdigest()
                directory = installation.directory / "native-upgrades" / operation
                directory.mkdir(parents=True, mode=0o700)
                password = secrets.token_urlsafe(48)
                new_runtime = {db: {**cfg, "password": secrets.token_urlsafe(48),
                    "viewer_password": secrets.token_urlsafe(48)} for db, cfg in old_runtime.items()}
                new_root = {db: {**old_root[db], "password": password,
                    "viewer_password": new_runtime[db]["viewer_password"]} for db in DATABASES}
                old_environment = (installation.directory / "server.env").read_text()
                environment = dict(line.split("=", 1) for line in old_environment.splitlines() if line and not line.startswith("#"))
                if environment.get("SURREAL_PASS") != old_root["main"]["password"]:
                    raise FixtureBlocked("identity", "server bootstrap password differs from owned root credentials")
                environment["SURREAL_USER"] = old_root["main"]["username"]
                environment["SURREAL_PASS"] = password
                plan = {"schema": 1, "operation": operation, "installation_id": installation.id,
                    "native_operation": operation,
                    "service_generation": installation.record["service_generation"], "candidate": candidate,
                    "previous_installer": installation.record["installer"],
                    "previous_installer_sha256": file_sha256(Path(installation.record["installer"])),
                    "previous_installer_generation": installation.record.get("installer_generation"),
                    "source": {db: source_identity for db in DATABASES},
                    "target": target, "old_runtime": old_runtime, "old_root": old_root,
                    "new_runtime": new_runtime, "new_root": new_root,
                    "old_environment": old_environment,
                    "new_environment": "".join(f"{key}={value}\n" for key, value in environment.items())}
                plan["execution_contracts"] = _upgrade_execution_contracts(installation, candidate, plan)
                journal = directory / "plan.json"
                durable_json(journal, plan)
                for db in DATABASES:
                    durable_json(directory / f"{db}-new-installer.json", new_root[db])
                    durable_json(directory / f"{db}-execution.json", plan["execution_contracts"][db])
                fsync_directory(directory)
                fsync_directory(directory.parent)
                fsync_directory(installation.directory)
                upgrade = {"operation": operation, "candidate_sha256": candidate["sha256"],
                    "native_operation": operation,
                    "candidate": candidate["path"], "previous_installer": plan["previous_installer"],
                    "journal": str(journal), "journal_sha256": file_sha256(journal),
                    "phase": "prepared", "checkpoints": {}}
                marker = {"operation": UPGRADE_OPERATION, "upgrade": upgrade}
            marker.update(owner=ProcessIdentity.of().to_json(), token=secrets.token_urlsafe(32))
            durable_json(marker_path, marker)
            def checkpoint(name: str) -> None:
                upgrade["checkpoints"][name] = True
                upgrade["phase"] = name
                durable_json(marker_path, marker)
            while pending := borrowers(installation):
                if any(not lock_held(installation.directory / "attachments" / row["id"] / "owner.lock")
                       for row in pending):
                    raise FixtureBlocked("drainage", "upgrade requires resolved host borrowers", "just fixture --recover ID")
                print(f"service: waiting for {len(pending)} existing borrowers", file=sys.stderr, flush=True)
                time.sleep(0.5)
            use = os.open(installation.directory / "borrowers.lock", os.O_RDWR | os.O_CLOEXEC)
            try:
                fcntl.flock(use, fcntl.LOCK_EX | fcntl.LOCK_NB)
            except BlockingIOError:
                raise FixtureBlocked("drainage", "an upgrade borrower lease remains held") from None
            _stopped_attachment_commands(installation)
            candidate = plan["candidate"]
            points = upgrade["checkpoints"]
            if replace and plan.get("recovery_mode") == "reconcile":
                raise FixtureBlocked("identity", "reconciled state requires exact resume or a checked reconciliation successor")
            if replace or reconcile:
                if (installation.record.get("native_schema_version", 3) != plan["source"]["main"]["schema_version"]
                        or any(name.startswith("scope-") and (name not in {"scope-" + db for db in DATABASES}
                               or points[name] is not True) for name in points)
                        or not points.get("authentication-drained")
                        or installation.record["installer"] != plan["previous_installer"]
                        or file_sha256(Path(plan["previous_installer"])) != plan["previous_installer_sha256"]):
                    raise FixtureBlocked("schema", "replacement requires the unchanged legacy installation and known scope checkpoints")
                _upgrade_executable(plan["candidate"])
                if (any(installation.runtime(db) != plan["new_runtime"][db]
                        or installation.runtime(db, installer=True) != plan["new_root"][db] for db in DATABASES)
                        or (installation.directory / "server.env").read_text() != plan["new_environment"]):
                    raise FixtureBlocked("identity", "replacement private authentication assets differ from their journal")
                predecessors = plan.get("predecessors", [])
                if not isinstance(predecessors, list):
                    raise FixtureBlocked("identity", "replacement predecessor chain is unavailable")
                ancestors = []
                for descriptor in predecessors:
                    _, ancestor = _upgrade_private_plan(installation, descriptor)
                    _upgrade_executable(ancestor["candidate"])
                    if descriptor.get("native_journals_sha256") != hashlib.sha256(json.dumps(
                            descriptor.get("native_journals"), sort_keys=True,
                            separators=(",", ":")).encode()).hexdigest():
                        raise FixtureBlocked("identity", "replacement predecessor native intent identity changed")
                    if (ancestor["new_root"] != plan["new_root"]
                            or ancestor["new_runtime"] != plan["new_runtime"]
                            or ancestor["new_environment"] != plan["new_environment"]):
                        raise FixtureBlocked("identity", "replacement predecessor authentication differs")
                    ancestors.append(ancestor)
                if len({entry["operation"] for entry in [*ancestors, plan]}) != len(ancestors) + 1:
                    raise FixtureBlocked("identity", "replacement predecessor chain repeats an operation")
                _upgrade_restart(installation)
                observed = _upgrade_replacement_preflight(installation, [*ancestors, plan], points,
                                                          reconcile=reconcile)
                for descriptor, ancestor in zip(predecessors, ancestors, strict=True):
                    _upgrade_predecessor_observations(descriptor, ancestor, plan,
                                                     observed[descriptor["operation"]], reconcile=reconcile)
                source_sha256 = file_sha256(source)
                candidate = prepare(installation.directory, source)
                if (candidate.get("sha256") != source_sha256 or file_sha256(source) != source_sha256
                        or candidate["sha256"] == plan["candidate"]["sha256"]):
                    raise FixtureBlocked("identity", "replacement candidate generation differs or repeats its predecessor")
                _upgrade_executable(candidate)
                target = _upgrade_cli(installation, candidate, "main", "schema")
                if (_upgrade_source(target) != plan["source"]["main"]
                        or target.get("schema_version") != plan["target"]["schema_version"]):
                    raise FixtureBlocked("schema", "replacement candidate must preserve the exact transition source/version")
                completed_scopes = [db for db in DATABASES if points.get("scope-" + db) is True]
                if (completed_scopes or reconcile) and target != plan["target"]:
                    raise FixtureBlocked("schema", "published scopes require the identical complete schema target")
                predecessor = {"operation": plan["operation"], "journal": str(journal),
                    "native_operation": plan.get("native_operation", plan["operation"]),
                    "journal_sha256": upgrade["journal_sha256"], "candidate": plan["candidate"]["path"],
                    "candidate_sha256": plan["candidate"]["sha256"],
                    "previous_installer": plan["previous_installer"],
                    "native_journals": observed[plan["operation"]],
                    "native_journals_sha256": hashlib.sha256(json.dumps(observed[plan["operation"]],
                        sort_keys=True, separators=(",", ":")).encode()).hexdigest()}
                operation = hashlib.sha256(secrets.token_bytes(32)).hexdigest()
                directory = installation.directory / "native-upgrades" / operation
                directory.mkdir(mode=0o700)
                plan = {**plan, "operation": operation, "candidate": candidate, "target": target,
                    "native_operation": plan.get("native_operation", plan["operation"]) if completed_scopes or reconcile else operation,
                    "completed_scopes": completed_scopes,
                    "credential_operation": plan.get("credential_operation", plan["operation"]),
                    "predecessors": [*predecessors, predecessor]}
                if reconcile:
                    plan["recovery_mode"] = "reconcile"
                plan["execution_contracts"] = _upgrade_execution_contracts(installation, candidate, plan)
                journal = directory / "plan.json"
                durable_json(journal, plan)
                for db in DATABASES:
                    durable_json(directory / f"{db}-new-installer.json", plan["new_root"][db])
                    durable_json(directory / f"{db}-execution.json", plan["execution_contracts"][db])
                fsync_directory(directory)
                fsync_directory(directory.parent)
                fsync_directory(installation.directory)
                upgrade = {"operation": operation, "candidate_sha256": candidate["sha256"],
                    "native_operation": plan["native_operation"],
                    "candidate": candidate["path"], "previous_installer": plan["previous_installer"],
                    "journal": str(journal), "journal_sha256": file_sha256(journal),
                    "predecessors": plan["predecessors"],
                    "phase": "reconciliation-prepared" if reconcile else "replacement-prepared", "checkpoints": {"drained": True,
                        **{"scope-" + db: True for db in completed_scopes}}}
                marker["upgrade"] = upgrade
                durable_json(marker_path, marker)
                points = upgrade["checkpoints"]
            if not points.get("drained"):
                _bootstrap_owned(installation)
                source_markers = _schema_identities(installation)
                if any(row.get("schema") != plan["source"][db]["schema"]
                       or row.get("schema_version") != plan["source"][db]["schema_version"]
                       for db, row in source_markers.items()):
                    raise FixtureBlocked("schema", "installed source differs from the exact native migration source")
                if file_sha256(Path(plan["previous_installer"])) != plan["previous_installer_sha256"]:
                    raise FixtureBlocked("identity", "previous maintenance executable changed")
                _run_installer(installation, Path(plan["previous_installer"]), "close-admission")
                _run_installer(installation, Path(plan["previous_installer"]), "drain")
                _upgrade_restart(installation)
                checkpoint("drained")
            elif resumed and not (replace or reconcile):
                # Caller death can leave unknown server-side DDL/auth effects after its last
                # checkpoint. Terminate that exact incarnation before reconciling/replaying.
                _upgrade_restart(installation)
            # A lost root-rotation reply is reconciled with the privately journaled new auth.
            old_root, new_root = plan["old_root"], plan["new_root"]
            current_root = new_root if _upgrade_authenticates(installation, new_root["main"]) else old_root
            for db in DATABASES:
                for label, role in (("writer", "OWNER"), ("viewer", "VIEWER")):
                    before, after = plan["old_runtime"][db], plan["new_runtime"][db]
                    if label == "viewer":
                        before = {**before, "username": before["viewer_username"], "password": before["viewer_password"]}
                        after = {**after, "username": after["viewer_username"], "password": after["viewer_password"]}
                    _upgrade_rotate(installation, current_root[db], before, after,
                        scope="DATABASE", role=role, operation=plan.get("credential_operation", plan["operation"]))
                    checkpoint(f"rotated-{db}-{label}")
            _upgrade_rotate(installation, current_root["main"], old_root["main"], new_root["main"],
                            scope="ROOT", role="OWNER", operation=plan.get("credential_operation", plan["operation"]))
            checkpoint("rotated-root")
            for db in DATABASES:
                durable_json(installation.runtime_path(db), plan["new_runtime"][db])
                durable_json(installation.runtime_path(db, installer=True), plan["new_root"][db])
            _upgrade_write_private(installation.directory / "server.env", plan["new_environment"])
            checkpoint("configs")
            _upgrade_restart(installation)
            checkpoint("authentication-drained")
            # Closed admission and the immutable candidate keep a verified epoch valid
            # for this operation. Reuse its proof instead of recapturing the same catalog.
            definition_proofs = set()
            if stop_after_pages is not None and (plan.get("recovery_mode") != "reconcile"
                    or not points.get("scope-main") or points.get("scope-validation")):
                raise FixtureBlocked("identity", "upgrade stepping requires reconciliation-owned incomplete validation and completed main")
            if plan.get("recovery_mode") == "reconcile" and resumed and not reconcile:
                current_native = _upgrade_replacement_preflight(installation, [plan], points,
                    reconcile=True)[plan["operation"]]
                for db in DATABASES:
                    if points.get("scope-" + db) and points.get("definitions-" + db):
                        proof = _upgrade_cli(installation, candidate, db, "upgrade-check",
                            config=journal.parent / f"{db}-new-installer.json", operation=plan["operation"],
                            native_operation=plan["native_operation"])
                        if proof.get("definitions_complete") is not True or proof.get("ready") is not False:
                            raise FixtureBlocked("schema", "completed scope executable definitions are unavailable")
                        definition_proofs.add(db)
            elif reconcile:
                current_native = observed[predecessor["operation"]]
            validation_sealed = (plan.get("recovery_mode") == "reconcile"
                and bool(current_native["validation"])
                and current_native["validation"][0]["phase"] == "published")
            # Every new explicit protocol is qualified on closed validation before
            # translating either scope. Frozen legacy plans keep their original CLI.
            if ("execution_contracts" in plan and not points.get("qualification-validation")
                    and not points.get("scope-validation") and not validation_sealed):
                contract_path = journal.parent / "validation-execution.json"
                proof = _upgrade_cli(installation, candidate, "validation", "upgrade-qualify",
                    config=journal.parent / "validation-new-installer.json", operation=plan["operation"],
                    native_operation=plan["native_operation"], execution_contract=contract_path,
                    expected_schema=plan["source"]["validation"]["schema"])
                qualification = proof.get("qualification", {})
                if (proof.get("ready") is not False
                        or any(qualification.get(key) is not True for key in
                            ("rollback", "acknowledgement_reconciliation", "stale_revision_refusal"))):
                    raise FixtureBlocked("schema", "validation upgrade atomic progress qualification is unavailable")
                checkpoint("qualification-validation")
            for db in DATABASES:
                if (not points.get("scope-" + db)
                        or ("execution_contracts" in plan and not points.get("definitions-" + db))):
                    config = journal.parent / f"{db}-new-installer.json"
                    if read_json(config) != plan["new_root"][db] or config.stat().st_mode & 0o077:
                        raise FixtureBlocked("identity", "checkpointed installer credentials changed")
                    execution_args = {}
                    if "execution_contracts" in plan:
                        contract_path = journal.parent / f"{db}-execution.json"
                        if (contract_path.is_symlink() or not contract_path.is_file()
                                or contract_path.stat().st_uid != os.getuid() or contract_path.stat().st_mode & 0o077
                                or read_json(contract_path) != plan["execution_contracts"][db]):
                            raise FixtureBlocked("identity", "checkpointed upgrade execution contract changed")
                        execution_args = {"execution_contract": contract_path,
                                          "stop_after_pages": stop_after_pages if db == "validation" else None}
                    result = _upgrade_cli(installation, candidate, db, "upgrade", config=config,
                                          operation=plan["operation"],
                                          native_operation=plan.get("native_operation", plan["operation"]),
                                          expected_schema=plan["source"][db]["schema"], **execution_args)
                    if result.get("ready") is not False:
                        raise FixtureBlocked("schema", "native upgrade must leave admission closed")
                    if "execution_contracts" in plan:
                        advance = result.get("upgrade", {})
                        if advance.get("published") is not True:
                            if db != "validation" or stop_after_pages is None or type(advance.get("revision")) is not int:
                                raise FixtureBlocked("schema", "native upgrade did not establish publication or a durable step")
                            return {"outcome": "checkpointed", "upgrade_operation": plan["operation"],
                                "native_operation": plan["native_operation"], "database": "validation",
                                "progress": advance, "admission_open": False}
                        if result.get("definitions_complete") is not True:
                            raise FixtureBlocked("schema", "native seal lacks executable-definition completion")
                        # upgrade installs/verifies actual immutable definitions before its
                        # checked session completion. Closed exclusive ownership preserves
                        # that proof here; reentry performs its own actual upgrade-check.
                        definition_proofs.add(db)
                        checkpoint("definitions-" + db)
                    if not points.get("scope-" + db):
                        checkpoint("scope-" + db)
                    if stop_after_pages is not None and db == "validation":
                        return {"outcome": "checkpointed", "upgrade_operation": plan["operation"],
                            "native_operation": plan["native_operation"], "database": "validation",
                            "progress": advance, "admission_open": False}
            # Check both native scopes using the new executable and credentials before handoff.
            for db in DATABASES:
                _upgrade_cli(installation, candidate, db, "check",
                             config=journal.parent / f"{db}-new-installer.json", operation=plan["operation"],
                             native_operation=plan.get("native_operation", plan["operation"]))
                if "execution_contracts" in plan and db not in definition_proofs:
                    proof = _upgrade_cli(installation, candidate, db, "upgrade-check",
                        config=journal.parent / f"{db}-new-installer.json", operation=plan["operation"],
                        native_operation=plan["native_operation"])
                    if proof.get("definitions_complete") is not True or proof.get("ready") is not False:
                        raise FixtureBlocked("schema", "completed scope executable definitions are unavailable")
            for db in DATABASES:
                rows = sql(installation,
                    "SELECT generation, schema, schema_version FROM native_installation:current;",
                    cfg=plan["new_runtime"][db])
                expected = {"generation": bytes(plan["service_generation"]).hex(),
                            "schema": plan["target"]["schema"], "schema_version": plan["target"]["schema_version"]}
                if rows[0].get("result") != [expected]:
                    raise FixtureBlocked("schema", "upgraded target identity differs from checkpointed scope")
            installation.record.update(installer=candidate["path"], installer_generation=candidate,
                native_schema_version=plan["target"]["schema_version"], native_upgrade={
                    "operation": plan["operation"], "journal": str(journal),
                    "native_operation": plan.get("native_operation", plan["operation"]),
                    "journal_sha256": upgrade["journal_sha256"],
                    "previous_installer": plan["previous_installer"],
                    "candidate_sha256": candidate["sha256"], "predecessors": plan.get("predecessors", [])})
            durable_json(installation.directory / "installation.json", installation.record)
            checkpoint("handoff")
            installation.check(allow_maintenance=True)
            try:
                _run_installer(installation, Path(candidate["path"]), "open-admission")
            except BaseException:
                with contextlib.suppress(Exception):
                    _run_installer(installation, Path(candidate["path"]), "close-admission")
                raise
            marker_path.unlink()
            fsync_directory(installation.directory)
            return {"outcome": "passed", "installation_id": installation.id,
                    "service_generation": installation.record["service_generation"],
                    "native_schema_version": plan["target"]["schema_version"], "installer_sha256": candidate["sha256"],
                    "upgrade_operation": plan["operation"]}
        finally:
            if use is not None:
                os.close(use)
            os.close(gate)

SERVER_HANDOFF_OPERATION = "server-generation-handoff"


def _server_handoff_plan(installation: Installation, reference: Mapping[str, Any]) -> tuple[Path, dict]:
    operation = reference.get("operation", "")
    if not isinstance(operation, str) or not re.fullmatch(r"[0-9a-f]{64}", operation):
        raise FixtureBlocked("identity", "server handoff operation is unavailable")
    journal = installation.directory / "server-handoffs" / operation / "plan.json"
    try:
        if (reference.get("journal") != str(journal)
                or any(path.is_symlink() for path in (journal, *journal.parents))
                or journal.stat().st_uid != os.getuid() or journal.stat().st_mode & 0o077
                or journal.parent.stat().st_uid != os.getuid() or journal.parent.stat().st_mode & 0o077
                or file_sha256(journal) != reference.get("journal_sha256")):
            raise FixtureBlocked("identity", "immutable server handoff journal changed")
        plan = read_json(journal)
        if (plan.get("operation") != operation or plan.get("installation_id") != installation.id
                or plan.get("service_generation") != installation.record["service_generation"]):
            raise FixtureBlocked("identity", "server handoff differs from its installation")
        return journal, plan
    except (OSError, TypeError, AttributeError, ValueError):
        raise FixtureBlocked("identity", "immutable server handoff journal is unavailable") from None


def _server_handoff_dependencies(installation: Installation) -> list[dict]:
    """Storage references cover both generations and every retained immutable handoff."""
    import surrealdb_server
    result = []
    def binary_assets(binary):
        if isinstance(binary, dict) and isinstance(binary.get("path"), str):
            result.append({"path": binary["path"], "role": "service-server-executable"})
            if generation := binary.get("generation"):
                surrealdb_server.validate_provenance(generation["provenance"], directory=installation.directory)
                result.extend(surrealdb_server.dependencies(generation))
    binary_assets(installation.record.get("binary"))
    marker = read_json(installation.directory / "maintenance.json")
    references = [installation.record.get("server_handoff")]
    if isinstance(marker, dict):
        references.append(marker.get("server_handoff"))
    seen = set()
    while references:
        reference = references.pop()
        if not reference:
            continue
        if reference.get("operation") in seen:
            continue
        journal, plan = _server_handoff_plan(installation, reference)
        seen.add(plan["operation"])
        result.append({"path": str(journal.parent), "role": "server-handoff-private-assets"})
        binary_assets(plan["previous_binary"])
        binary_assets(_generation_binary(plan["generation"]))
        references.append(plan.get("previous_handoff"))
    return result


def _server_upgrade_plans(installation: Installation, upgrade: Mapping[str, Any]) -> list[dict]:
    _, plan = _upgrade_private_plan(installation, upgrade)
    points = upgrade.get("checkpoints", {})
    if (not points.get("drained") or not points.get("authentication-drained")
            or any(name.startswith("scope-") and (name not in {"scope-" + db for db in DATABASES}
                   or points[name] is not True) for name in points)
            or any(points.get("scope-" + db) is not True for db in plan.get("completed_scopes", []))
            or upgrade.get("predecessors", []) != plan.get("predecessors", [])):
        raise FixtureBlocked("identity", "server handoff requires exact drained upgrade checkpoints")
    if (any(installation.runtime(db) != plan["new_runtime"][db]
            or installation.runtime(db, installer=True) != plan["new_root"][db] for db in DATABASES)
            or (installation.directory / "server.env").read_text() != plan["new_environment"]):
        raise FixtureBlocked("identity", "server handoff cannot change checkpointed upgrade credentials")
    ancestors = []
    for reference in plan.get("predecessors", []):
        _, ancestor = _upgrade_private_plan(installation, reference)
        if reference.get("native_journals_sha256") != hashlib.sha256(json.dumps(
                reference.get("native_journals"), sort_keys=True, separators=(",", ":")).encode()).hexdigest():
            raise FixtureBlocked("identity", "server handoff predecessor observation changed")
        ancestors.append(ancestor)
    return [*ancestors, plan]


def _server_scope_markers(installation: Installation, expected: Mapping[str, Any] | None = None) -> dict:
    markers = {}
    generation = bytes(installation.record["service_generation"]).hex()
    for db in DATABASES:
        rows = sql(installation,
            "SELECT generation,schema,schema_version,admission_open FROM native_installation:current;",
            cfg=installation.runtime(db, installer=True))
        values = rows[0].get("result") if len(rows) == 1 else None
        if not isinstance(values, list) or len(values) != 1 or not isinstance(values[0], dict):
            raise FixtureBlocked("schema", "server handoff native scope marker is unavailable")
        marker = values[0]
        if (marker.get("generation") != generation
                or not isinstance(marker.get("schema"), str)
                or not re.fullmatch(r"[0-9a-f]{64}", marker["schema"])
                or type(marker.get("admission_open")) is not bool
                or (expected is None and marker.get("schema_version") != installation.record.get("native_schema_version", 3))
                or (expected is not None and marker != expected[db])):
            raise FixtureBlocked("schema", "server handoff native scope identity or closed admission differs")
        markers[db] = {**marker, "admission_open": False}
    return markers


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
                raise FixtureBlocked("ownership", "another owner holds server handoff admission") from None
            marker = read_json(marker_path)
            if marker and (marker.get("operation") not in {UPGRADE_OPERATION, SERVER_HANDOFF_OPERATION}
                           or ProcessIdentity.from_json(marker["owner"]).alive()):
                raise FixtureBlocked("ownership", "server handoff requires its dead exact maintenance owner")
            if borrowers(installation):
                raise FixtureBlocked("drainage", "server handoff requires resolved host borrowers")
            use = os.open(installation.directory / "borrowers.lock", os.O_RDWR | os.O_CLOEXEC)
            try:
                fcntl.flock(use, fcntl.LOCK_EX | fcntl.LOCK_NB)
            except BlockingIOError:
                raise FixtureBlocked("drainage", "a server handoff borrower lease remains held") from None
            resumed = bool(marker and marker.get("server_handoff"))
            pending_upgrade = marker.get("upgrade") if marker and marker.get("operation") == UPGRADE_OPERATION else None
            plans = _server_upgrade_plans(installation, pending_upgrade) if pending_upgrade else None
            port = int(installation.endpoint.rsplit(":", 1)[1])
            if resumed:
                journal, plan = _server_handoff_plan(installation, marker["server_handoff"])
                if generation != plan["generation"] or pending_upgrade != plan["upgrade"]:
                    raise FixtureBlocked("identity", "resume requires the exact checkpointed server generation and native upgrade")
            else:
                previous = dict(installation.record["binary"])
                previous_path = _installed_binary(installation)
                if previous == _generation_binary(generation):
                    raise FixtureBlocked("identity", "server generation is already installed")
                expected_unit = unit_text(installation.directory, previous_path, port)
                if _unit_path().read_text() != expected_unit:
                    raise FixtureBlocked("identity", "owned service unit differs before server handoff")
                if plans:
                    current = plans[-1]
                    scopes = {db: {"generation": bytes(current["service_generation"]).hex(),
                        "schema": current["target"]["schema"] if pending_upgrade["checkpoints"].get("scope-" + db) else current["source"][db]["schema"],
                        "schema_version": current["target"]["schema_version"] if pending_upgrade["checkpoints"].get("scope-" + db) else current["source"][db]["schema_version"],
                        "admission_open": False} for db in DATABASES}
                else:
                    installation.check_daemon()
                    scopes = _server_scope_markers(installation)
                operation = hashlib.sha256(secrets.token_bytes(32)).hexdigest()
                directory = installation.directory / "server-handoffs" / operation
                directory.mkdir(parents=True, mode=0o700)
                plan = {"schema": 1, "operation": operation, "installation_id": installation.id,
                    "service_generation": installation.record["service_generation"],
                    "previous_binary": previous, "generation": generation,
                    "previous_unit": expected_unit, "successor_unit": unit_text(installation.directory, Path(generation["path"]), port),
                    "previous_handoff": installation.record.get("server_handoff"),
                    "upgrade": pending_upgrade, "scopes": scopes}
                journal = directory / "plan.json"
                durable_json(journal, plan)
                for path in (directory, directory.parent, installation.directory):
                    fsync_directory(path)
                marker = marker or {"operation": SERVER_HANDOFF_OPERATION}
                marker["server_handoff"] = {"operation": operation, "journal": str(journal),
                    "journal_sha256": file_sha256(journal), "phase": "prepared"}
            marker.update(owner=ProcessIdentity.of().to_json(), token=secrets.token_urlsafe(32))
            durable_json(marker_path, marker)
            if installation.record["binary"] not in (plan["previous_binary"], _generation_binary(generation)):
                raise FixtureBlocked("identity", "server handoff installed generation changed")
            if _unit_path().read_text() not in (plan["previous_unit"], plan["successor_unit"]):
                raise FixtureBlocked("identity", "server handoff unit changed outside its exact journal")
            _stopped_attachment_commands(installation)
            if not resumed and not pending_upgrade:
                _run_installer(installation, Path(installation.record["installer"]), "close-admission")
                _run_installer(installation, Path(installation.record["installer"]), "drain")
            # This also confirms a core-dumped daemon has no surviving descendants.
            _stop_owned(installation)
            _upgrade_write_private(_unit_path(), plan["successor_unit"])
            if systemctl("daemon-reload").returncode:
                raise FixtureBlocked("systemd", "server handoff daemon-reload failed; admission stays closed")
            reference = {key: marker["server_handoff"][key] for key in ("operation", "journal", "journal_sha256")}
            installation.record.update(binary=_generation_binary(generation), server_handoff=reference)
            durable_json(installation.directory / "installation.json", installation.record)
            marker["server_handoff"]["phase"] = "installed"
            durable_json(marker_path, marker)
            _upgrade_daemon_start(installation)
            if not pending_upgrade:
                _run_installer(installation, Path(installation.record["installer"]), "close-admission")
                _run_installer(installation, Path(installation.record["installer"]), "drain")
            checked = {"scopes": _server_scope_markers(installation, plan["scopes"])}
            if plans:
                observed = _upgrade_replacement_preflight(installation, plans, pending_upgrade["checkpoints"])
                for prior, ancestor in zip(plans[-1].get("predecessors", []), plans[:-1], strict=True):
                    _upgrade_predecessor_observations(prior, ancestor, plans[-1], observed[prior["operation"]])
                checked["native_journals"] = observed
            durable_json(journal.parent / "checked.json", checked)
            if pending_upgrade:
                del marker["server_handoff"]
                durable_json(marker_path, marker)
            else:
                installation.check(allow_maintenance=True)
                try:
                    _run_installer(installation, Path(installation.record["installer"]), "open-admission")
                except BaseException:
                    with contextlib.suppress(Exception):
                        _run_installer(installation, Path(installation.record["installer"]), "close-admission")
                    raise
                marker_path.unlink()
                fsync_directory(installation.directory)
            return {"outcome": "passed", "installation_id": installation.id,
                "service_generation": installation.record["service_generation"],
                "server_generation": generation["identity"], "server_handoff": plan["operation"],
                "native_upgrade_pending": bool(pending_upgrade), "admission_open": not bool(pending_upgrade)}
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
    operations.add_argument("--upgrade-installer", type=Path,
                            help="explicit checkpointed native installer upgrade through a supported exact transition")
    operations.add_argument("--replace-upgrade-installer", type=Path,
                            help="explicitly supersede an unpublished failed upgrade with a checked successor")
    operations.add_argument("--reconcile-upgrade-installer", type=Path,
                            help="explicit immutable successor for recognized partially translated upgrade state")
    maintenance_parser.add_argument("--upgrade-step-pages", type=int,
                            help="stop at durable validation checkpoints under reconciliation-owned upgrade")
    operations.add_argument("--server-generation", type=Path,
                            help="explicitly hand off the owned server to a reviewed immutable descriptor")
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
            or ((args.recover or args.upgrade_installer or args.replace_upgrade_installer or args.reconcile_upgrade_installer or args.server_generation) and args.command)
        ):
            parser.error(
                "named reader reconciliation requires --recover, --reconcile-database "
                "and an explicit inventory, without a command"
            )
        if args.upgrade_step_pages is not None and (args.upgrade_step_pages <= 0
                or not (args.upgrade_installer or args.reconcile_upgrade_installer)):
            parser.error("--upgrade-step-pages requires a positive count and --upgrade-installer or --reconcile-upgrade-installer")
        if len(set(args.reconcile_pin)) != len(args.reconcile_pin) or len(
            set(args.reconcile_backup_hold)
        ) != len(args.reconcile_backup_hold):
            parser.error("named reconciliation identities must be distinct")
    if args.action == "maintenance" and args.native_clients:
        command = args.command[1:] if args.command[:1] == ["--"] else args.command
        if not command or args.restart or args.check or args.recover or args.stabilize_installer or args.upgrade_installer or args.replace_upgrade_installer or args.reconcile_upgrade_installer or args.server_generation:
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
                    "server": {"path": installation.record["binary"]["path"],
                        "sha256": installation.record["binary"]["sha256"],
                        "version": installation.record["binary"]["version"],
                        "generation": installation.record["binary"].get("generation", {}).get("identity")},
                    "server_handoff": marker.get("server_handoff") if isinstance(marker, dict) else None,
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
                if args.server_generation:
                    print(json.dumps(handoff_server(installation, args.server_generation), indent=2))
                    return 0
                if args.upgrade_installer or args.replace_upgrade_installer or args.reconcile_upgrade_installer:
                    print(json.dumps(upgrade_installer(installation,
                        args.reconcile_upgrade_installer or args.replace_upgrade_installer or args.upgrade_installer,
                        replace=bool(args.replace_upgrade_installer), reconcile=bool(args.reconcile_upgrade_installer),
                        stop_after_pages=args.upgrade_step_pages), indent=2))
                    return 0
                if args.recover:
                    gate = hold_lock(installation.directory / "admission.lock")
                    use = None
                    native_started = False
                    try:
                        row = read_json(installation.directory / "maintenance.json")
                        if row and (row.get("server_handoff") or row.get("operation") == SERVER_HANDOFF_OPERATION):
                            raise FixtureBlocked("maintenance", "incomplete server handoff requires the exact --server-generation descriptor")
                        if row and row.get("operation") == UPGRADE_OPERATION:
                            raise FixtureBlocked("maintenance", "incomplete native upgrade requires the same --upgrade-installer candidate")
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
