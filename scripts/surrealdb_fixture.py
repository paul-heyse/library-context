#!/usr/bin/env python3
"""Disposable native SurrealDB fixtures: one boundary that supplies the native fixture variables.

Run it as ``just fixture …`` (``uv run --no-project --offline --no-python-downloads python
scripts/surrealdb_fixture.py …``) or use ``fixture()`` from another harness script.

    -- CMD…                         run-owned server for one command (the default)
    --keep [-- CMD…]                kept server; prints its ID (and runs CMD attached)
    --attach ID [--serving N | --retain-serving N] -- CMD…
    --stop ID                       stop a kept (or dead run-owned) fixture and remove its state
    --list [--json]                 inventory; never sweeps
    --sweep                         sweep only (also automatic at start; ``--no-sweep``)

**Substrate (OD2).** A native ``surreal`` 3.3.0 binary, byte-identical to ``/surreal`` in the
previously pinned image and verified by sha256 and ``surreal version`` before every start
(``LCTX_SURREAL_BIN``; default pse-arrow's tools copy, then ``~/.surrealdb/surreal``). A
run-owned server is a ``PR_SET_PDEATHSIG`` child of the launcher inside a capped
``systemd-run --user --scope`` (``lctx-fixture-<id>.scope``), so it dies with the launcher however
the launcher dies. A kept server is the user unit ``lctx-fixture-<id>.service`` with
``KillMode=control-group`` and a 0600 ``EnvironmentFile``. Both bind a launcher-chosen explicit
loopback port (retried on collision before readiness) that a restart keeps. ``LCTX_FIXTURE_MEMORY``
(default ``1G``) sets ``MemoryMax`` and scales ``SURREAL_MEMORY_THRESHOLD`` to half of it. An OOM
kill, from the wait status or the unit's ``Result=oom-kill``, is an infrastructure failure
(``blocked``, exit 75), also during the readiness wait.

**Lifetimes (plan D2).** The server substrate is run-owned or kept. Each command gets an
*attachment*: its own scratch directory, ``LCTX_SURREAL_TEST_CONFIG`` and
``LCTX_COMPILER_RUNTIME_CONFIG`` (0600), and a namespace ``fixture_<id>_<attachment>`` with
databases ``core`` and ``compiler_cache``. Commands that choose their own namespace or database
names remain responsible for that state; concurrent attachments are not universally isolated.
Retained serving content (``--retain-serving NAME`` on a kept fixture) records its producing
inputs, configuration, immutable content identity and producing run; ``--serving NAME`` checks the
realization is still available and records the obligations its reuse skipped.

**Records.** ``build/fixtures/<id>/`` (``LCTX_FIXTURES_ROOT`` relocates it): ``record.json``,
``owner.lock`` (flocked by a run-owned launcher), ``server.env`` (0600), ``server.log``, ``data/``
and ``attachments/<aid>/``. Liveness is "identity alive or owner lock held" (plan §5.8.3), so a
sibling pid namespace never looks dead; a record from a previous boot is dead. A run handle's
cancel reaches a run-owned fixture through its process group. The sweep removes only state of
dead owners whose owned command tree has no survivors (a run-owned fixture's surviving tree is
terminated first, its server already being gone), and empty leftover scopes of this checkout; it
never touches kept servers, live attachments or same-boot foreign pid namespaces. Standard
library only (plan §5.8.1).
"""

from __future__ import annotations

import argparse
import base64
import contextlib
import hashlib
import json
import os
import random
import re
import secrets
import shlex
import shutil
import signal
import socket
import subprocess
import sys
import time
import urllib.error
import urllib.request
from collections.abc import Callable, Iterator, Mapping, Sequence
from dataclasses import dataclass, field
from datetime import UTC, datetime
from pathlib import Path
from typing import Any

from harness import (
    ProcessIdentity,
    group_members,
    hold_lock,
    lock_held,
    read_json,
    signal_group,
    spawn_group,
    write_json_atomic,
)

ROOT = Path(__file__).resolve().parent.parent

# What `Attachment.environment` supplies to a command (verify's --print names these).
ATTACHMENT_VARIABLES = (
    "LCTX_SURREAL_TEST_CONFIG",
    "LCTX_COMPILER_RUNTIME_CONFIG",
    "LCTX_FIXTURE_ID",
    "LCTX_FIXTURE_ATTACHMENT",
)

# The selected server: 3.3.0 at source 238bfeb11f5725bebed370167656748df8067595, whose
# protocol/DDL/restore parity the native controls were qualified against. The official release
# binary is byte-identical to /surreal in image
# surrealdb/surrealdb@sha256:681c6c22c287421b5c7d99e0fde79b6e0d32c36c1ddeaab2762a1661cb04cd20
# (Tested 2026-10-07). Revisit when qualifying another server release.
VERSION = "surrealdb-3.3.0"
CLI_VERSION = "3.3.0 for linux on x86_64"
BINARY_SHA256 = "58ad479cbdd8b1926a636524d259ef309f029d8d9968fb88f350590f7f2b7bc5"
ARCHIVE_SHA256 = "44aeab565f7e7e39d2d0bf0583c8aae648babc91373c70b2658288d95bbbcd55"
RELEASE_URL = (
    "https://github.com/surrealdb/surrealdb/releases/download/v3.3.0/surreal-v3.3.0.linux-amd64.tgz"
)

SCHEMA = 1
UNIT_PREFIX = "lctx-fixture-"
DEFAULT_MEMORY = "1G"
EXIT_BLOCKED = 75
PORT_RANGE = (20000, 32000)  # below the kernel's ephemeral range, so clients rarely collide
PORT_ATTEMPTS = 8
READY_TIMEOUT = 40.0
STOP_GRACE = 8.0
ADMIN_USER = "fixture_admin"
# Deliberate fixture allocation policy, unchanged from the container fixture. Default durable
# Every sync and background maintenance are kept; the tracked-memory threshold complements the
# cgroup cap. Native rows may travel alone up to 64 MiB with RPC framing; the SDK follows the
# advertised server limit, whose 4 MiB default cannot carry admitted rows.
SERVER_TUNING = {
    "SURREAL_ROCKSDB_BLOCK_CACHE_SIZE": "67108864",
    "SURREAL_ROCKSDB_WRITE_BUFFER_SIZE": "33554432",
    "SURREAL_ROCKSDB_MAX_WRITE_BUFFER_NUMBER": "2",
    "SURREAL_GRPC_MAX_MESSAGE_SIZE": "128MiB",
}
# Variables a run-owned scope needs from the launcher; nothing else reaches the server.
PASSED_THROUGH = ("PATH", "HOME", "USER", "LANG", "XDG_RUNTIME_DIR", "DBUS_SESSION_BUS_ADDRESS")


def _stderr(message: str) -> None:
    print(message, file=sys.stderr, flush=True)


def now() -> str:
    return datetime.now(UTC).isoformat(timespec="seconds")


def fixtures_root(env: Mapping[str, str] | None = None) -> Path:
    override = (os.environ if env is None else env).get("LCTX_FIXTURES_ROOT")
    return Path(override) if override else ROOT / "build" / "fixtures"


def _identifier(value: str) -> str:
    if not value or not value.isascii() or not all(c.isalnum() or c == "_" for c in value):
        raise ValueError("fixture database/namespace must be an ASCII identifier")
    return value


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


class PortCollision(RuntimeError):
    pass


# ---------------------------------------------------------------------------------------------
# The pinned binary and substrate readiness

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


def systemd_available(*, runner: Runner = subprocess.run) -> str | None:
    """None when a user systemd manager answers, else why not."""
    try:
        result = runner(
            ["systemctl", "--user", "show-environment"],
            capture_output=True,
            text=True,
            timeout=15,
            check=False,
        )
    except (OSError, subprocess.TimeoutExpired) as error:
        return f"cannot run systemctl --user ({error})"
    if result.returncode:
        return (result.stderr or "").strip() or f"systemctl --user exited {result.returncode}"
    return None


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


def substrate_readiness(
    requirement: str = "native-store",
    env: Mapping[str, str] | None = None,
    *,
    runner: Runner = subprocess.run,
) -> Readiness:
    """Observe the fixture substrate: the pinned binary and a user systemd manager. Never starts
    a server and never repairs anything."""
    try:
        binary = verify_binary(env, runner=runner)
    except FixtureBlocked as blocked:
        return Readiness(requirement, False, blocked.detail, blocked.repair)
    missing = systemd_available(runner=runner)
    if missing:
        return Readiness(requirement, False, missing, SYSTEMD_REPAIR)
    return Readiness(requirement, True, f"{binary} {VERSION}")


# ---------------------------------------------------------------------------------------------
# Memory, liveness, processes and systemd

_UNITS = {"": 1, "K": 1 << 10, "M": 1 << 20, "G": 1 << 30, "T": 1 << 40}


def parse_memory(text: str) -> int:
    match = re.fullmatch(r"\s*(\d+)\s*([KMGT]?)(?:i?B)?\s*", text, re.IGNORECASE)
    if not match:
        raise ValueError(f"memory must look like 512M or 2G, not {text!r}")
    value = int(match.group(1)) * _UNITS[match.group(2).upper()]
    if value < 64 << 20:
        raise ValueError("fixture memory below 64M cannot start a server")
    return value


def memory_text(value: int) -> str:
    for suffix in ("G", "M", "K"):
        if value % _UNITS[suffix] == 0:
            return f"{value // _UNITS[suffix]}{suffix}"
    return str(value)


def server_environment(memory: int, password: str) -> dict[str, str]:
    threshold = memory // 2
    return {
        "SURREAL_USER": ADMIN_USER,
        "SURREAL_PASS": password,
        **SERVER_TUNING,
        "SURREAL_MEMORY_THRESHOLD": f"{threshold >> 20}MiB",
    }


def owner_alive(identity: Mapping[str, Any] | None, lock: Path) -> bool:
    """Plan §5.8.3: identity alive or the owner's lock held (a sibling pid namespace)."""
    if lock_held(lock):
        return True
    return identity is not None and ProcessIdentity.from_json(identity).alive()


def owned_survivors(leader: Mapping[str, Any] | None) -> list[int] | None:
    """Live members of a recorded command group, [] when none, None when unknowable here."""
    if not leader:
        return []
    identity = ProcessIdentity.from_json(leader)
    if identity.previous_boot():
        return []
    if identity.foreign():
        return None
    try:
        current = ProcessIdentity.of(identity.pid)
    except ProcessLookupError:
        current = None
    # A pid is never reused while it still names a live process group, so a different process
    # leading that pid means our group emptied earlier.
    if current is not None and current.start_ticks != identity.start_ticks:
        return []
    return group_members(identity.pid)


def systemctl(*args: str, runner: Runner = subprocess.run) -> subprocess.CompletedProcess:
    return runner(
        ["systemctl", "--user", *args], capture_output=True, text=True, timeout=60, check=False
    )


def unit_properties(unit: str, *names: str) -> dict[str, str]:
    result = systemctl("show", unit, *(f"--property={name}" for name in names))
    values = {}
    for line in result.stdout.splitlines():
        key, _, value = line.partition("=")
        values[key] = value
    return values


def settled_result(unit: str, timeout: float = 5.0) -> dict[str, str]:
    """The unit's state once it has left the active states (or it is gone)."""
    deadline = time.monotonic() + timeout
    while True:
        state = unit_properties(unit, "ActiveState", "Result", "LoadState")
        if state.get("ActiveState") not in ("active", "activating", "deactivating", "reloading"):
            return state
        if time.monotonic() >= deadline:
            return state
        time.sleep(0.1)


def choose_port(taken: set[int] | frozenset[int] = frozenset()) -> int:
    while True:
        port = random.randint(*PORT_RANGE)
        if port in taken:
            continue
        with socket.socket() as probe:
            try:
                probe.bind(("127.0.0.1", port))
            except OSError:
                continue
        return port


def _git(*args: str) -> str:
    result = subprocess.run(
        ["git", "-C", str(ROOT), *args], capture_output=True, timeout=60, check=False
    )
    return result.stdout.decode(errors="replace").strip() if result.returncode == 0 else ""


def source_inputs() -> dict[str, str]:
    """The checkout's producing inputs: HEAD and a digest of the uncommitted difference."""
    diff = subprocess.run(
        ["git", "-C", str(ROOT), "diff", "HEAD", "--binary"],
        capture_output=True,
        timeout=120,
        check=False,
    ).stdout
    untracked = _git("ls-files", "--others", "--exclude-standard")
    digest = hashlib.sha256(diff + b"\0" + untracked.encode()).hexdigest()
    return {"head": _git("rev-parse", "HEAD"), "uncommitted_sha256": digest}


# ---------------------------------------------------------------------------------------------
# The server substrate


SETUP_ERRORS = (OSError, urllib.error.URLError, RuntimeError, ValueError, KeyError)


@contextlib.contextmanager
def _setup(what: str) -> Iterator[None]:
    """Fixture setup failures become classifiable ``FixtureBlocked("readiness")`` evidence."""
    try:
        yield
    except FixtureBlocked:
        raise
    except SETUP_ERRORS as error:
        raise FixtureBlocked("readiness", f"{what}: {type(error).__name__}: {error}") from error


def _unit_name(fixture_id: str, kind: str) -> str:
    return f"{UNIT_PREFIX}{fixture_id}.{'service' if kind == 'kept' else 'scope'}"


@dataclass
class Server:
    """One native server: run-owned (a tied scope child of this process) or kept (a user unit)."""

    directory: Path
    record: dict[str, Any]
    password: str
    process: subprocess.Popen | None = field(default=None, repr=False)
    report: Callable[[str], None] = field(default=_stderr, repr=False)
    _lock: int | None = field(default=None, repr=False)
    _ended: str | None = field(default=None, repr=False)

    # -- identity ------------------------------------------------------------------------------
    @property
    def id(self) -> str:
        return str(self.record["id"])

    @property
    def kind(self) -> str:
        return str(self.record["kind"])

    @property
    def unit(self) -> str:
        return str(self.record["unit"])

    @property
    def port(self) -> int:
        return int(self.record["port"])

    @property
    def endpoint(self) -> str:
        return f"http://127.0.0.1:{self.port}"

    @property
    def grpc_endpoint(self) -> str:
        return f"grpc://127.0.0.1:{self.port}"

    @property
    def memory(self) -> int:
        return int(self.record["memory_max"])

    def write(self) -> None:
        write_json_atomic(self.directory / "record.json", self.record)

    # -- creation ------------------------------------------------------------------------------
    @classmethod
    def create(
        cls,
        kind: str,
        *,
        memory: int,
        env: Mapping[str, str] | None = None,
        report: Callable[[str], None] = _stderr,
    ) -> Server:
        env = os.environ if env is None else env
        binary = verify_binary(env)
        missing = systemd_available()
        if missing:
            raise FixtureBlocked("systemd", missing, SYSTEMD_REPAIR)
        root = fixtures_root(env)
        root.mkdir(parents=True, exist_ok=True)
        while True:
            fixture_id = secrets.token_hex(5)
            directory = root / fixture_id
            try:
                directory.mkdir()
                break
            except FileExistsError:
                continue
        directory.chmod(0o700)
        owner = ProcessIdentity.of().to_json()
        lock = hold_lock(directory / "owner.lock") if kind == "run" else None
        password = secrets.token_urlsafe(32)
        environment = directory / "server.env"
        fd = os.open(environment, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
        with os.fdopen(fd, "w") as stream:
            stream.writelines(f"{k}={v}\n" for k, v in server_environment(memory, password).items())
        (directory / "data").mkdir()
        (directory / "tmp").mkdir()
        run_dir = env.get("LCTX_RUN_DIR")
        record = {
            "schema": SCHEMA,
            "id": fixture_id,
            "kind": kind,
            "checkout": str(ROOT),
            "unit": _unit_name(fixture_id, kind),
            "port": 0,
            "memory_max": memory,
            "memory_threshold": server_environment(memory, "")["SURREAL_MEMORY_THRESHOLD"],
            "binary": {"path": str(binary), "sha256": BINARY_SHA256, "version": VERSION},
            "created": now(),
            "owner": owner,
            "run": {"id": env.get("LCTX_RUN_ID"), "dir": run_dir} if run_dir else None,
            "server": None,
        }
        server = cls(directory, record, password, report=report, _lock=lock)
        server.write()
        try:
            server._start_new_port()
        except BaseException:
            server.destroy()
            raise
        return server

    @classmethod
    def open(cls, fixture_id: str, env: Mapping[str, str] | None = None) -> Server:
        directory = fixtures_root(env) / fixture_id
        record = read_json(directory / "record.json")
        if not isinstance(record, dict):
            raise FixtureBlocked("record", f"no fixture {fixture_id} under {directory.parent}")
        return cls(directory, record, _read_env(directory / "server.env")["SURREAL_PASS"])

    # -- launch --------------------------------------------------------------------------------
    def argv(self, binary: Path) -> list[str]:
        return [
            str(binary),
            "start",
            "--no-banner",
            "--log=warn",
            "--bind",
            f"127.0.0.1:{self.port}",
            "--query-timeout",
            "20s",
            "--transaction-timeout",
            "10s",
            "--temporary-directory",
            str(self.directory / "tmp"),
            f"rocksdb://{self.directory / 'data' / 'store'}",
        ]

    def _description(self) -> str:
        return f"lctx fixture {self.id} kind={self.kind} checkout={ROOT}"

    def _start_new_port(self) -> None:
        taken: set[int] = set()
        for _attempt in range(PORT_ATTEMPTS):
            self.record["port"] = choose_port(taken)
            taken.add(self.port)
            self.write()
            try:
                self.start()
                return
            except PortCollision:
                self.report(f"fixture {self.id}: port {self.port} collided; choosing another")
        raise FixtureBlocked("launch", f"no free port after {PORT_ATTEMPTS} attempts")

    def start(self) -> None:
        """Start (or restart) on the recorded port; the binary is verified every time."""
        binary = verify_binary()
        if self.record["binary"]["path"] != str(binary):
            self.record["binary"]["path"] = str(binary)
        log = (self.directory / "server.log").open("ab")
        offset = log.tell()
        try:
            if self.kind == "run":
                self._reset_unit()
                env = {k: os.environ[k] for k in PASSED_THROUGH if k in os.environ}
                env.update(_read_env(self.directory / "server.env"))
                command = [
                    "systemd-run",
                    "--user",
                    "--scope",
                    "--quiet",
                    f"--unit={self.unit}",
                    f"--description={self._description()}",
                    f"--property=MemoryMax={self.memory}",
                    "--property=MemorySwapMax=0",
                    "--property=MemoryZSwapMax=0",
                    "--",
                    *self.argv(binary),
                ]
                try:
                    self.process = spawn_group(
                        command,
                        death_signal=signal.SIGKILL,
                        env=env,
                        cwd=self.directory,
                        stdin=subprocess.DEVNULL,
                        stdout=log,
                        stderr=subprocess.STDOUT,
                    )
                except OSError as error:
                    raise FixtureBlocked("launch", f"cannot start systemd-run: {error}") from error
                self.record["server"] = ProcessIdentity.of(self.process.pid).to_json()
            else:
                self._reset_unit()
                result = subprocess.run(
                    [
                        "systemd-run",
                        "--user",
                        "--quiet",
                        f"--unit={self.unit}",
                        f"--description={self._description()}",
                        "--service-type=exec",
                        "--property=KillMode=control-group",
                        "--property=Restart=no",
                        "--property=TimeoutStopSec=30",
                        f"--property=MemoryMax={self.memory}",
                        "--property=MemorySwapMax=0",
                        "--property=MemoryZSwapMax=0",
                        f"--property=EnvironmentFile={self.directory / 'server.env'}",
                        f"--property=WorkingDirectory={self.directory}",
                        f"--property=StandardOutput=append:{self.directory / 'server.log'}",
                        "--property=StandardError=inherit",
                        "--",
                        *self.argv(binary),
                    ],
                    capture_output=True,
                    text=True,
                    timeout=60,
                    check=False,
                )
                if result.returncode:
                    raise FixtureBlocked("launch", f"systemd-run failed: {result.stderr.strip()}")
                self.record["server"] = None
            self.record["started"] = now()
            self.write()
        finally:
            log.close()
        self.ready(log_offset=offset)

    def _reset_unit(self) -> None:
        state = unit_properties(self.unit, "ActiveState", "LoadState")
        if state.get("ActiveState") == "failed":
            systemctl("reset-failed", self.unit)

    def _log_tail(self, offset: int = 0) -> str:
        try:
            with (self.directory / "server.log").open("rb") as stream:
                stream.seek(offset)
                text = stream.read().decode(errors="replace")
        except OSError:
            return ""
        lines = [line for line in text.splitlines() if line.strip()]
        return " | ".join(lines[-3:])

    def _scope_oom(self) -> bool:
        """OOM evidence for a run-owned scope: the cgroup's ``memory.events`` ``oom_kill`` count,
        read while the scope still exists (an emptied scope stays loaded on systemd 255 and its
        ``Result`` stays ``success``); the unit's ``Result`` is the fallback."""
        state = unit_properties(self.unit, "ControlGroup", "Result")
        group = state.get("ControlGroup", "")
        if group.startswith("/"):
            try:
                events = (Path("/sys/fs/cgroup") / group.lstrip("/") / "memory.events").read_text()
            except OSError:
                events = ""
            for line in events.splitlines():
                key, _, value = line.partition(" ")
                if key == "oom_kill" and value.strip().isdigit() and int(value) > 0:
                    return True
        return state.get("Result") == "oom-kill"

    def _close_scope(self) -> None:
        """Stop the (empty) run-owned scope so its name is free again and no unit leaks."""
        systemctl("stop", self.unit)
        systemctl("reset-failed", self.unit)

    def exited(self) -> str | None:
        """None while the server runs; otherwise a description of how it ended."""
        if self.kind == "run":
            if self._ended is not None:
                return self._ended
            if self.process is None or self.process.poll() is None:
                return None
            code = self.process.returncode
            oom = self._scope_oom()
            self._close_scope()
            self._ended = "oom-kill" if oom else f"signal {-code}" if code < 0 else f"exit {code}"
            return self._ended
        state = unit_properties(self.unit, "ActiveState", "Result")
        if state.get("ActiveState") in ("active", "activating", "deactivating", "reloading"):
            return None
        result = state.get("Result", "")
        if result == "oom-kill":
            return "oom-kill"
        return "exit 0" if result in ("success", "") else f"result {result}"

    def oom(self, when: str) -> FixtureBlocked:
        return FixtureBlocked(
            "oom",
            f"server {self.id} OOM-killed {when} at MemoryMax={memory_text(self.memory)}",
            "raise LCTX_FIXTURE_MEMORY (or --memory)",
        )

    def ready(self, timeout: float = READY_TIMEOUT, *, log_offset: int = 0) -> None:
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            ended = self.exited()
            if ended is not None:
                tail = self._log_tail(log_offset)
                if ended == "oom-kill":
                    raise self.oom("during readiness")
                if "Address already in use" in tail:
                    raise PortCollision(tail)
                raise FixtureBlocked("readiness", f"server ended ({ended}) before ready: {tail}")
            try:
                with urllib.request.urlopen(self.endpoint + "/ready", timeout=1) as response:
                    healthy = response.status == 200
                if healthy:
                    with urllib.request.urlopen(self.endpoint + "/version", timeout=1) as response:
                        version = response.read().decode().strip()
                    if version != VERSION:
                        raise FixtureBlocked("readiness", f"server reports {version}")
                    return
            except urllib.error.URLError, TimeoutError, ConnectionError:
                pass
            time.sleep(0.1)
        raise FixtureBlocked("readiness", f"server {self.id} not ready after {timeout:.0f}s")

    # -- lifecycle -----------------------------------------------------------------------------
    def stop_process(self, grace: float = 30.0) -> str | None:
        """Stop the server; returns how it ended when that was not a clean stop (e.g. oom-kill)."""
        if self.kind == "kept":
            before = self.exited()
            systemctl("stop", self.unit)
            settled_result(self.unit)
            systemctl("reset-failed", self.unit)
            return None if before in (None, "exit 0") else before
        if self.process is None:
            return None
        ended = self.exited()
        if ended is None:
            self.process.terminate()
            try:
                self.process.wait(grace)
            except subprocess.TimeoutExpired:
                self.process.kill()
                self.process.wait()
            stopped = self.exited()  # reads OOM evidence, then closes the scope
            ended = "oom-kill" if stopped == "oom-kill" else None
        elif ended == "exit 0":
            ended = None
        self.process = None
        self._ended = None
        return ended

    def restart(self) -> None:
        """Restart on the same port (port stability is the point; a collision is an error)."""
        if self.kind == "kept":
            result = systemctl("restart", self.unit)
            if result.returncode:
                raise FixtureBlocked("launch", f"restart failed: {result.stderr.strip()}")
            self.ready()
            return
        self.stop_process()
        try:
            self.start()
        except PortCollision as collision:
            raise FixtureBlocked(
                "launch", f"port {self.port} taken on restart: {collision}"
            ) from collision

    def destroy(self) -> str | None:
        """Stop the server and remove all of its state. Returns a non-clean ending, if any."""
        ended = None
        try:
            ended = self.stop_process(grace=STOP_GRACE)
        finally:
            shutil.rmtree(self.directory, ignore_errors=True)
            if self._lock is not None:
                os.close(self._lock)
                self._lock = None
        return ended

    # -- queries -------------------------------------------------------------------------------
    def request(
        self, route: str, data: bytes | None, namespace: str | None, database: str | None
    ) -> urllib.request.Request:
        credentials = f"{ADMIN_USER}:{self.password}"
        headers = {
            "Authorization": "Basic " + base64.b64encode(credentials.encode()).decode(),
            "Accept": "application/json",
            "Content-Type": "text/plain",
        }
        if namespace:
            headers["Surreal-NS"] = namespace
        if database:
            headers["Surreal-DB"] = database
        return urllib.request.Request(self.endpoint + route, data=data, headers=headers)

    def query(
        self, sql: str, *, namespace: str | None = None, database: str | None = None
    ) -> list[dict[str, Any]]:
        request = self.request("/sql", sql.encode(), namespace, database)
        with urllib.request.urlopen(request, timeout=25) as response:
            result = json.load(response)
        if not isinstance(result, list):
            raise RuntimeError("fixture query returned no statement results")
        failures = [row for row in result if row.get("status") != "OK"]
        if failures:
            # SQL/errors can contain bound secrets; retain the shape, not the body.
            kinds = ", ".join(str(row.get("kind", "statement")) for row in failures)
            raise RuntimeError(f"fixture query failed: {kinds}")
        return result


def _read_env(path: Path) -> dict[str, str]:
    values = {}
    for line in path.read_text().splitlines():
        key, sep, value = line.partition("=")
        if sep:
            values[key] = value
    return values


# ---------------------------------------------------------------------------------------------
# Attachments: per-command mutable state


@dataclass
class Attachment:
    """One command's owned state on a server."""

    server: Server
    id: str
    directory: Path
    config: dict[str, Any] = field(repr=False)
    extra_env: dict[str, str] = field(default_factory=dict)
    _lock: int | None = field(default=None, repr=False)
    _retaining: Path | None = field(default=None, repr=False)

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
        return str(self.config["namespace"])

    @classmethod
    def create(cls, server: Server) -> Attachment:
        while True:
            attachment_id = secrets.token_hex(4)
            directory = server.directory / "attachments" / attachment_id
            try:
                directory.mkdir(parents=True)
                break
            except FileExistsError:
                continue
        lock = hold_lock(directory / "owner.lock")
        (directory / "scratch").mkdir()
        namespace = _identifier(f"fixture_{server.id}_{attachment_id}")
        config = {
            "scratch": str(directory / "scratch"),
            "fixture": server.id,
            "attachment": attachment_id,
            "endpoint": server.endpoint,
            "grpc_endpoint": server.grpc_endpoint,
            "namespace": namespace,
            "database": "core",
            "admin_user": ADMIN_USER,
            "admin_password": server.password,
            "server": {"version": VERSION, "binary_sha256": BINARY_SHA256},
        }
        attachment = cls(server, attachment_id, directory, config, _lock=lock)
        attachment._write_record(None)
        _write_private(attachment.config_path, config)
        _write_private(
            attachment.compiler_config_path,
            {
                "endpoint": server.grpc_endpoint,
                "username": ADMIN_USER,
                "password": server.password,
                "viewer_username": f"fixture_viewer_{attachment_id}",
                "viewer_password": server.password + "_viewer_" + attachment_id,
                "namespace": namespace,
                "cache_database": "compiler_cache",
                "selection": str(directory / "scratch" / "selected.json"),
            },
        )
        try:
            with _setup(f"defining attachment namespace {namespace}"):
                server.query(
                    f"DEFINE NAMESPACE {namespace};"
                    f" USE NS {namespace}; DEFINE DATABASE core STRICT;"
                    " DEFINE DATABASE compiler_cache STRICT;"
                )
        except FixtureBlocked:
            attachment.release()
            raise
        return attachment

    def _write_record(self, command: dict[str, Any] | None, **extra: Any) -> None:
        record = read_json(self.directory / "record.json") or {
            "id": self.id,
            "fixture": self.server.id,
            "namespace": self.namespace,
            "databases": ["core", "compiler_cache"],
            "owner": ProcessIdentity.of().to_json(),
            "created": now(),
        }
        if command is not None:
            record["command"] = command
        record.update(extra)
        write_json_atomic(self.directory / "record.json", record)

    def environment(self, source: Mapping[str, str] | None = None) -> dict[str, str]:
        env = dict(os.environ if source is None else source)
        env["LCTX_SURREAL_TEST_CONFIG"] = str(self.config_path)
        env["LCTX_COMPILER_RUNTIME_CONFIG"] = str(self.compiler_config_path)
        env["LCTX_FIXTURE_ID"] = self.server.id
        env["LCTX_FIXTURE_ATTACHMENT"] = self.id
        env.update(self.extra_env)
        return env

    def restart(self) -> None:
        self.server.restart()

    # -- retained serving content (kept fixtures) ----------------------------------------------
    def retain_serving(self, name: str) -> Path:
        """Where the producing command writes the viewer configuration it retains."""
        directory = self.server.directory / "serving" / _identifier(name)
        if (directory / "identity.json").exists():
            raise FixtureBlocked("serving", f"serving content {name!r} already retained")
        # An earlier retention that never recorded its identity failed: it is replaced.
        shutil.rmtree(directory, ignore_errors=True)
        directory.mkdir(parents=True)
        self._retaining = directory
        path = directory / "viewer.json"
        self.extra_env["LCTX_RETAIN_NATIVE_FIXTURE_CONFIG"] = str(path)
        return path

    def abandon_serving(self) -> None:
        """Remove a retention this attachment started but never recorded (the producer failed).
        ``release`` calls it, so every caller gets the same cleanup."""
        if self._retaining is not None and not (self._retaining / "identity.json").exists():
            shutil.rmtree(self._retaining, ignore_errors=True)
        self._retaining = None

    def record_serving(self, name: str, command: Sequence[str]) -> dict[str, Any]:
        """After a successful producer: record the retained content's identity."""
        directory = self.server.directory / "serving" / _identifier(name)
        viewer = directory / "viewer.json"
        if not viewer.is_file():
            self.abandon_serving()
            raise FixtureBlocked("serving", f"producer did not write {viewer}")
        try:
            with _setup(f"reading retained serving {name!r}"):
                config = json.loads(viewer.read_text())
                selection = Path(config["selection"])
                handle = json.loads(selection.read_text())
        except FixtureBlocked:
            self.abandon_serving()
            raise
        identity = {
            "name": name,
            "fixture": self.server.id,
            "inputs": {**source_inputs(), "command": list(command)},
            "configuration": {
                "path": str(viewer),
                "sha256": file_sha256(viewer),
                "selection": str(selection),
                "selection_sha256": file_sha256(selection),
                "server": {"version": VERSION, "binary_sha256": BINARY_SHA256},
            },
            "content": {
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
        write_json_atomic(directory / "identity.json", identity)
        self._retaining = None
        return identity

    def use_serving(self, name: str) -> dict[str, Any]:
        """Check the retained realization is still available, expose it, record skipped work."""
        with _setup(f"checking retained serving {name!r}"):
            identity = serving_available(self.server, name)
        self.extra_env["LCTX_NATIVE_SERVING_CONFIG"] = identity["configuration"]["path"]
        current = source_inputs()
        produced = {key: identity["inputs"].get(key) for key in current}
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
            # The availability check decides reuse; changed sources are recorded, not refused.
            "inputs_changed_since_production": current != produced,
        }
        self._write_record(None, serving_reuse=reuse)
        return reuse

    # -- teardown ------------------------------------------------------------------------------
    def release(self) -> None:
        """Remove this attachment's namespace and files; retained serving content is kept."""
        self.abandon_serving()
        try:
            if self.server.exited() is None and self.namespace not in retained_namespaces(
                self.server
            ):
                self.server.query(f"REMOVE NAMESPACE IF EXISTS {self.namespace};")
        except OSError, RuntimeError, urllib.error.URLError:
            pass
        shutil.rmtree(self.directory, ignore_errors=True)
        if self._lock is not None:
            os.close(self._lock)
            self._lock = None


def _write_private(path: Path, data: Any) -> None:
    fd = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_TRUNC, 0o600)
    with os.fdopen(fd, "w") as stream:
        json.dump(data, stream)


def retained_identities(server: Server) -> list[dict[str, Any]]:
    root = server.directory / "serving"
    found = []
    for path in sorted(root.glob("*/identity.json")) if root.is_dir() else ():
        data = read_json(path)
        if isinstance(data, dict):
            found.append(data)
    return found


def retained_namespaces(server: Server) -> set[str]:
    names = set()
    for identity in retained_identities(server):
        database = identity.get("content", {}).get("database") or {}
        if database.get("namespace"):
            names.add(database["namespace"])
    return names


def serving_available(server: Server, name: str) -> dict[str, Any]:
    """The identity of retained content whose realization the server still holds."""
    identity = read_json(server.directory / "serving" / _identifier(name) / "identity.json")
    if not isinstance(identity, dict):
        raise FixtureBlocked("serving", f"no retained serving content {name!r} on {server.id}")
    configuration = identity["configuration"]
    for key, digest in (("path", "sha256"), ("selection", "selection_sha256")):
        path = Path(configuration[key])
        if not path.is_file() or file_sha256(path) != configuration[digest]:
            raise FixtureBlocked("serving", f"retained {key} of {name!r} changed or vanished")
    if configuration["server"]["binary_sha256"] != server.record["binary"]["sha256"]:
        raise FixtureBlocked("serving", f"{name!r} was produced on another server binary")
    server.ready()
    database = identity["content"]["database"]
    namespace = _identifier(database["namespace"])
    info = server.query("INFO FOR NAMESPACE;", namespace=namespace)[0]["result"]
    if database["database"] not in info.get("databases", {}):
        raise FixtureBlocked(
            "serving", f"{name!r}: database {namespace}.{database['database']} is gone"
        )
    return identity


# ---------------------------------------------------------------------------------------------
# Library entry point


@contextlib.contextmanager
def fixture(
    *, memory: int | None = None, sweep_first: bool = True, report: Callable[[str], None] = _stderr
) -> Iterator[Attachment]:
    """A run-owned server with one attachment, removed on exit. The server also dies with this
    process if it is killed (PDEATHSIG), so call it from the main thread."""
    with _setup("starting the run-owned fixture"):
        if sweep_first:
            sweep(report=report)
        server = Server.create("run", memory=memory or configured_memory(), report=report)
    ended: str | None = None
    try:
        with _setup("attaching to the run-owned fixture"):
            attachment = Attachment.create(server)
        yield attachment
    finally:
        ended = server.destroy()
    if ended == "oom-kill":
        raise server.oom("while in use")


def configured_memory(explicit: str | None = None) -> int:
    return parse_memory(explicit or os.environ.get("LCTX_FIXTURE_MEMORY") or DEFAULT_MEMORY)


# ---------------------------------------------------------------------------------------------
# Inventory and sweep


def records(env: Mapping[str, str] | None = None) -> list[tuple[Path, dict[str, Any] | None]]:
    root = fixtures_root(env)
    if not root.is_dir():
        return []
    return [
        (path, read_json(path / "record.json")) for path in sorted(root.iterdir()) if path.is_dir()
    ]


def _attachments(directory: Path) -> list[tuple[Path, dict[str, Any] | None]]:
    root = directory / "attachments"
    if not root.is_dir():
        return []
    return [
        (path, read_json(path / "record.json")) for path in sorted(root.iterdir()) if path.is_dir()
    ]


def _attachment_live(path: Path, record: dict[str, Any] | None) -> bool:
    return owner_alive((record or {}).get("owner"), path / "owner.lock")


def inventory(env: Mapping[str, str] | None = None) -> list[dict[str, Any]]:
    """Every recorded fixture with its unit state, plus unrecorded lctx-fixture units."""
    rows: list[dict[str, Any]] = []
    seen_units = set()
    for directory, record in records(env):
        if record is None:
            rows.append({"id": directory.name, "kind": "incomplete", "directory": str(directory)})
            continue
        unit = record["unit"]
        seen_units.add(unit)
        state = unit_properties(unit, "LoadState", "ActiveState", "Result", "MemoryCurrent")
        attachments = _attachments(directory)
        live = sum(1 for path, data in attachments if _attachment_live(path, data))
        owner = record.get("owner")
        rows.append(
            {
                "id": record["id"],
                "kind": record["kind"],
                "unit": unit,
                "state": "gone"
                if state.get("LoadState") == "not-found"
                else f"{state.get('ActiveState', '?')}/{state.get('Result', '?')}",
                "owner": owner.get("pid") if owner else None,
                "owner_alive": record["kind"] == "kept"
                or owner_alive(owner, directory / "owner.lock"),
                "attachments": f"{live}/{len(attachments)}",
                "port": record["port"],
                "memory_max": record["memory_max"],
                "memory_current": state.get("MemoryCurrent"),
                "created": record["created"],
                "serving": [i["name"] for i in retained_identities(Server(directory, record, ""))],
            }
        )
    listed = systemctl("list-units", "--all", "--plain", "--no-legend", f"{UNIT_PREFIX}*")
    for line in listed.stdout.splitlines():
        unit = line.split()[0] if line.split() else ""
        if unit and unit not in seen_units:
            rows.append(
                {"id": None, "kind": "unrecorded", "unit": unit, "state": line[len(unit) :].strip()}
            )
    return rows


def _sweep_empty_scopes(recorded: set[str]) -> list[str]:
    """Stop unrecorded run-owned scopes of this checkout whose cgroup holds no process (leaked by
    a launcher that died between its scope emptying and closing it)."""
    removed = []
    listed = systemctl("list-units", "--all", "--plain", "--no-legend", f"{UNIT_PREFIX}*.scope")
    for line in listed.stdout.splitlines():
        unit = line.split()[0] if line.split() else ""
        fixture_id = unit.removeprefix(UNIT_PREFIX).removesuffix(".scope")
        if not unit or fixture_id in recorded:
            continue
        state = unit_properties(unit, "Description", "ControlGroup")
        if f"checkout={ROOT}" not in state.get("Description", "").split():
            continue
        group = state.get("ControlGroup", "")
        try:
            procs = (Path("/sys/fs/cgroup") / group.lstrip("/") / "cgroup.procs").read_text()
        except OSError:
            procs = ""
        if procs.strip():
            continue
        systemctl("stop", unit)
        systemctl("reset-failed", unit)
        removed.append(f"{unit} (empty scope)")
    return removed


def sweep(
    env: Mapping[str, str] | None = None, *, report: Callable[[str], None] = _stderr
) -> list[str]:
    """Remove state of dead owners only. Returns what was removed."""
    removed = []
    for directory, record in records(env):
        if record is None:
            # A creation that died before its first record: no lock holder, at least a minute old.
            if (
                not lock_held(directory / "owner.lock")
                and time.time() - directory.stat().st_mtime > 60
            ):
                shutil.rmtree(directory, ignore_errors=True)
                removed.append(f"{directory.name} (incomplete)")
            continue
        if record.get("checkout") != str(ROOT):
            continue
        attachments = _attachments(directory)
        if record["kind"] == "kept":
            server = Server(
                directory, record, _read_env(directory / "server.env").get("SURREAL_PASS", "")
            )
            for path, data in attachments:
                if _attachment_live(path, data):
                    continue
                survivors = owned_survivors((data or {}).get("command", {}).get("leader"))
                if survivors:  # live consumers of a dead launcher: preserved
                    continue
                if survivors is None:  # foreign pid namespace: never touched
                    continue
                namespace = (data or {}).get("namespace")
                with contextlib.suppress(OSError, RuntimeError, urllib.error.URLError):
                    if (
                        namespace
                        and server.exited() is None
                        and namespace not in retained_namespaces(server)
                    ):
                        server.query(f"REMOVE NAMESPACE IF EXISTS {_identifier(namespace)};")
                shutil.rmtree(path, ignore_errors=True)
                removed.append(f"{record['id']}/{path.name}")
            continue
        owner = record.get("owner")
        if owner_alive(owner, directory / "owner.lock"):
            continue
        if owner and ProcessIdentity.from_json(owner).foreign():
            continue
        leaders = [(data or {}).get("command", {}).get("leader") for _, data in attachments]
        survivors = [owned_survivors(leader) for leader in leaders]
        if any(s is None for s in survivors):
            continue
        for leader, members in zip(leaders, survivors, strict=True):
            if members and leader:
                report(f"fixture {record['id']}: terminating surviving command tree {members}")
                signal_group(int(leader["pid"]), grace=STOP_GRACE)
        server_identity = record.get("server")
        if server_identity and ProcessIdentity.from_json(server_identity).alive():
            with contextlib.suppress(ProcessLookupError):
                os.kill(int(server_identity["pid"]), signal.SIGKILL)
        systemctl("stop", record["unit"])
        systemctl("reset-failed", record["unit"])
        shutil.rmtree(directory, ignore_errors=True)
        removed.append(record["id"])
    removed += _sweep_empty_scopes({d.name for d, _ in records(env)})
    for item in removed:
        report(f"fixture sweep: removed {item}")
    return removed


# ---------------------------------------------------------------------------------------------
# Running a command attached


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
) -> int:
    """Run the command in its own process group with this attachment's environment. A signal to
    the launcher stops the command's group; the caller tears the state down."""
    from build_environment import normalized_env

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
        try:
            child = spawn_group(command, death_signal=signal.SIGTERM, env=env, cwd=cwd)
        except OSError as error:
            raise FixtureBlocked("launch", f"cannot start {command[0]!r}: {error}") from error
        leader = ProcessIdentity.of(child.pid).to_json()
        attachment._write_record({"argv": list(command), "leader": leader, "started": now()})
        try:
            while True:
                try:
                    return child.wait(0.5)
                except subprocess.TimeoutExpired:
                    continue
        except _Interrupted:
            for sig in TERMINATING:
                signal.signal(sig, signal.SIG_IGN)
            signal_group(child.pid, grace=STOP_GRACE / 2)
            raise
        finally:
            if child.poll() is not None and group_members(child.pid):
                signal_group(child.pid, grace=STOP_GRACE / 2)


def _finish(server: Server, code: int, report: Callable[[str], None]) -> int:
    ended = server.exited()
    if ended == "oom-kill":
        report(server.oom("while the command ran").message())
        return EXIT_BLOCKED
    if ended is not None:
        report(
            f"fixture: blocked: readiness: server {server.id} ended ({ended}) during the command"
        )
        return EXIT_BLOCKED
    return code


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        prog="just fixture", description=(__doc__ or "").split("\n\n")[0]
    )
    action = parser.add_mutually_exclusive_group()
    action.add_argument("--keep", action="store_true", help="create a kept fixture; print its ID")
    action.add_argument("--attach", metavar="ID", help="run the command attached to a kept fixture")
    action.add_argument("--stop", metavar="ID", help="stop a fixture and remove its state")
    action.add_argument("--restart", metavar="ID", help="restart a kept fixture on its port")
    action.add_argument("--list", action="store_true", help="inventory (never sweeps)")
    action.add_argument("--sweep", action="store_true", help="sweep dead owners' state only")
    parser.add_argument("--json", action="store_true", help="--list as JSON")
    parser.add_argument("--force", action="store_true", help="--stop despite live attachments")
    parser.add_argument("--no-sweep", action="store_true", help="skip the automatic start sweep")
    parser.add_argument(
        "--memory", help=f"MemoryMax (LCTX_FIXTURE_MEMORY, default {DEFAULT_MEMORY})"
    )
    parser.add_argument(
        "--requires",
        action="append",
        default=[],
        choices=("native-python", "tools", "native", "none"),
        help="shared Python environment ownership for the command (default: evident from argv)",
    )
    serving = parser.add_mutually_exclusive_group()
    serving.add_argument("--retain-serving", metavar="NAME", help="record retained serving content")
    serving.add_argument("--serving", metavar="NAME", help="reuse retained serving content")
    parser.add_argument("command", nargs=argparse.REMAINDER)
    args = parser.parse_args(argv)
    command = args.command[1:] if args.command[:1] == ["--"] else args.command

    if args.list:
        rows = inventory()
        if args.json:
            print(json.dumps(rows, indent=2))
        elif not rows:
            print("no fixtures")
        else:
            for row in rows:
                print(" ".join(f"{k}={v}" for k, v in row.items() if v not in (None, [], "")))
        return 0
    if args.sweep:
        sweep()
        return 0
    if args.stop:
        server = Server.open(args.stop)
        if server.kind == "run" and owner_alive(
            server.record.get("owner"), server.directory / "owner.lock"
        ):
            _stderr(f"fixture {args.stop} is run-owned by a live launcher; cancel that command")
            return 1
        attached = [
            p.name for p, data in _attachments(server.directory) if _attachment_live(p, data)
        ]
        if attached and not args.force:
            _stderr(f"fixture {args.stop} has live attachments {attached}; pass --force to stop")
            return 1
        ended = server.destroy()
        print(f"fixture {args.stop}: stopped{f' (had ended: {ended})' if ended else ''}")
        return 0
    if args.restart:
        server = Server.open(args.restart)
        if server.kind != "kept":
            parser.error("--restart applies to kept fixtures")
        server.restart()
        print(f"fixture {server.id}: restarted on {server.endpoint}")
        return 0
    if not command and not args.keep:
        parser.error("provide a command after --")
    if (args.retain_serving or args.serving) and not args.attach and not args.keep:
        parser.error("--retain-serving/--serving need a kept fixture (--keep or --attach)")

    for sig in TERMINATING:
        signal.signal(sig, _raise_interrupt)
    server: Server | None = None
    attachment: Attachment | None = None
    code = 0
    try:
        if not args.no_sweep:
            sweep()
        if args.attach:
            server = Server.open(args.attach)
            if server.kind != "kept":
                parser.error(f"{args.attach} is not a kept fixture")
            ended = server.exited()
            if ended is not None:
                raise (
                    server.oom("before attaching")
                    if ended == "oom-kill"
                    else FixtureBlocked(
                        "readiness", f"kept fixture {server.id} is not running ({ended})"
                    )
                )
            server.ready()
        else:
            server = Server.create(
                "kept" if args.keep else "run", memory=configured_memory(args.memory)
            )
            _stderr(
                f"fixture {server.id} ({server.kind}) ready: {server.endpoint}"
                f" MemoryMax={memory_text(server.memory)}"
                f" SURREAL_MEMORY_THRESHOLD={server.record['memory_threshold']}"
            )
            if args.keep:
                print(server.id, flush=True)
        if not command:
            return 0
        attachment = Attachment.create(server)
        if args.retain_serving:
            attachment.retain_serving(args.retain_serving)
        if args.serving:
            reuse = attachment.use_serving(args.serving)
            _stderr(
                f"fixture {server.id}: reusing serving {args.serving!r}; skipped producer "
                f"{shlex.join(reuse['skipped_obligations'][0]['producer'])}"
            )
        code = run_attached(
            attachment,
            command,
            requirements=python_requirements(command, args.requires),
            cwd=Path.cwd(),
        )
        code = _finish(server, code, _stderr)
        if args.retain_serving:
            if code == 0:
                identity = attachment.record_serving(args.retain_serving, command)
                _stderr(
                    f"fixture {server.id}: retained serving {args.retain_serving!r}:"
                    f" {identity['content']}"
                )
            else:
                shutil.rmtree(
                    server.directory / "serving" / args.retain_serving, ignore_errors=True
                )
        return code
    except FixtureBlocked as blocked:
        _stderr(blocked.message())
        code = EXIT_BLOCKED
        return code
    except _Interrupted as interrupted:
        code = 128 + interrupted.signum
        return code
    finally:
        for sig in TERMINATING:
            signal.signal(sig, signal.SIG_IGN)
        if attachment is not None:
            attachment.release()
        if server is not None and server.kind == "run":
            ended = server.destroy()
            if ended == "oom-kill" and code != EXIT_BLOCKED:
                _stderr(server.oom("while the command ran").message())


if __name__ == "__main__":
    raise SystemExit(main())
