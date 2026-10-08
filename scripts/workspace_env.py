#!/usr/bin/env python3
"""Scoped Python environment preparation, readiness observation and managed ownership (D1).

Standard library only; run as ``uv run --no-project --offline --no-python-downloads python
scripts/workspace_env.py`` (plan §5.8.1), so it works before the environment exists.

- **Identity.** The effective environment is derived from the checkout root and
  ``UV_PROJECT_ENVIRONMENT`` alone (absolute as-is, relative against the root, default
  ``.venv``), never from ``sys.prefix`` or ``VIRTUAL_ENV``. The independently locked vLLM service
  always uses its own project environment (``services/vllm/.venv``).
- **Routes.** ``sync tools`` installs the dev tools only and never builds the extension;
  ``sync native`` is the full sync with the native input key, a no-op while unchanged;
  ``sync vllm`` prepares the service project. Each takes exclusive ownership and reports the live
  holders it waits for.
- **Readiness observes.** ``observe(requirement)`` runs uv's read-only ``--locked --check`` and,
  when blocked, names the route that repairs it. It never synchronizes.
- **Ownership.** Two resources, acquired in this order: the effective environment and the
  checkout's extension directory (maturin builds the extension into the source tree; the
  environment holds only a ``.pth``). Locks are machine-wide ``flock`` files keyed by absolute
  path under ``$XDG_RUNTIME_DIR/library-context/locks`` (fallback
  ``~/.cache/library-context/locks``), on non-inheritable descriptors held by the managing
  process. Children reuse it through ``LCTX_ENV_OWNERSHIP`` while the recorded owner is alive.
  Pure-Rust work names no Python requirement and takes no lock. Unmanaged commands (bare
  importers, an explicit ``uv sync``) remain outside this guarantee.
"""

from __future__ import annotations

import argparse
import fcntl
import hashlib
import json
import os
import shlex
import subprocess
import sys
import time
from collections.abc import Callable, Iterable, Iterator, Mapping
from contextlib import contextmanager
from dataclasses import dataclass, field
from datetime import UTC, datetime
from pathlib import Path
from typing import Literal

from build_environment import ROOT, normalized_env, project_environment
from harness import ProcessIdentity, read_json, write_json_atomic

Mode = Literal["shared", "exclusive"]
Report = Callable[[str], None]

OWNERSHIP_KEY = "LCTX_ENV_OWNERSHIP"
VLLM_PROJECT = Path("services/vllm")
EXTENSION_DIRECTORY = Path("python/lctx_semantics/python/lctx_semantics")
ROUTES = ("tools", "native", "vllm")
# Requirement (verify's vocabulary or a route) -> the preparation route that repairs it.
ROUTE_OF = {"tools": "tools", "native": "native", "native-python": "native", "vllm": "vllm"}
POLL_SECONDS = 0.5


def _stderr(message: str) -> None:
    print(message, file=sys.stderr, flush=True)


# ---------------------------------------------------------------------------------------------
# Identity and resources


@dataclass(frozen=True, order=True)
class Resource:
    """One owned path. Global acquisition order: every environment, then every extension."""

    rank: int
    path: Path
    kind: str = field(compare=False)

    @property
    def name(self) -> str:
        digest = hashlib.sha256(str(self.path).encode()).hexdigest()[:32]
        return f"{self.kind}-{digest}"


def environment_path(root: Path = ROOT, env: Mapping[str, str] | None = None) -> Path:
    return project_environment(root, env)


def vllm_environment_path(root: Path = ROOT) -> Path:
    return root.resolve() / VLLM_PROJECT / ".venv"


def _requirements(requirement: str | Iterable[str]) -> set[str]:
    return {requirement} if isinstance(requirement, str) else set(requirement)


def resources_for(
    requirement: str | Iterable[str], root: Path = ROOT, env: Mapping[str, str] | None = None
) -> tuple[Resource, ...]:
    """The resources a requirement uses; requirements outside the Python routes need none."""
    found: set[Resource] = set()
    for item in _requirements(requirement):
        route = ROUTE_OF.get(item)
        if route in ("tools", "native"):
            found.add(Resource(0, environment_path(root, env), "environment"))
        if route == "native":
            found.add(Resource(1, root.resolve() / EXTENSION_DIRECTORY, "extension"))
        if route == "vllm":
            found.add(Resource(0, vllm_environment_path(root), "environment"))
    return tuple(sorted(found))


def lock_directory(env: Mapping[str, str] | None = None) -> Path:
    env = os.environ if env is None else env
    runtime = env.get("XDG_RUNTIME_DIR", "")
    if runtime and Path(runtime).is_dir():
        return Path(runtime) / "library-context" / "locks"
    return Path.home() / ".cache" / "library-context" / "locks"


# ---------------------------------------------------------------------------------------------
# Holder records


def _boot_time() -> float:
    for line in Path("/proc/stat").read_text().splitlines():
        if line.startswith("btime "):
            return float(line.split()[1])
    return 0.0


def _process_start(identity: ProcessIdentity) -> str:
    seconds = _boot_time() + identity.start_ticks / os.sysconf("SC_CLK_TCK")
    return datetime.fromtimestamp(seconds, UTC).isoformat(timespec="seconds")


@dataclass(frozen=True)
class Holder:
    identity: ProcessIdentity
    mode: str
    command: str
    started: str
    resource: str

    def describe(self) -> str:
        return f"pid {self.identity.pid} ({self.mode}, started {self.started}): {self.command}"


def _holders_directory(directory: Path, resource: Resource) -> Path:
    return directory / f"{resource.name}.holders"


def holders(resource: Resource, env: Mapping[str, str] | None = None) -> list[Holder]:
    """Live recorded holders of a resource; records of dead local processes are removed."""
    records = _holders_directory(lock_directory(env), resource)
    found = []
    for path in sorted(records.glob("*.json")) if records.is_dir() else ():
        data = read_json(path)
        if not isinstance(data, dict):
            continue
        identity = ProcessIdentity.from_json(data["identity"])
        if not identity.alive():
            if not identity.foreign():
                path.unlink(missing_ok=True)
            continue
        found.append(
            Holder(identity, data["mode"], data["command"], data["started"], data["resource"])
        )
    return found


# ---------------------------------------------------------------------------------------------
# Ownership


class OwnershipConflict(RuntimeError):
    """A nested managed operation needs a stronger mode than its command tree already holds."""


@dataclass
class _Lock:
    resource: Resource
    descriptor: int
    record: Path


@dataclass(frozen=True)
class Ownership:
    mode: str
    resources: tuple[Resource, ...]
    acquired: tuple[Resource, ...]
    token: str | None

    def environment(self, base: Mapping[str, str]) -> dict[str, str]:
        """``base`` plus the ownership a child command reuses."""
        env = dict(base)
        if self.token is None:
            env.pop(OWNERSHIP_KEY, None)
        else:
            env[OWNERSHIP_KEY] = self.token
        return env


def inherited(env: Mapping[str, str] | None = None) -> dict[str, tuple[str, dict]]:
    """Resources held by a live ancestor: lock name -> (mode, entry)."""
    raw = (os.environ if env is None else env).get(OWNERSHIP_KEY)
    try:
        entries = json.loads(raw) if raw else []
    except ValueError:
        return {}
    held = {}
    for entry in entries if isinstance(entries, list) else ():
        try:
            if ProcessIdentity.from_json(entry["owner"]).alive():
                held[entry["name"]] = (entry["mode"], entry)
        except KeyError, TypeError, ValueError:
            continue
    return held


def _acquire(
    resource: Resource, mode: Mode, command: str, report: Report, env: Mapping[str, str]
) -> _Lock:
    directory = lock_directory(env)
    directory.mkdir(parents=True, exist_ok=True)
    descriptor = os.open(
        directory / f"{resource.name}.lock", os.O_RDWR | os.O_CREAT | os.O_CLOEXEC, 0o600
    )
    operation = fcntl.LOCK_SH if mode == "shared" else fcntl.LOCK_EX
    reported: tuple[int, ...] | None = None
    try:
        while True:
            try:
                fcntl.flock(descriptor, operation | fcntl.LOCK_NB)
                break
            except BlockingIOError:
                live = holders(resource, env)
                seen = tuple(sorted(holder.identity.pid for holder in live))
                if seen != reported:
                    reported = seen
                    lines = [f"waiting for {mode} {resource.kind} ownership of {resource.path}:"]
                    lines += [f"  held by {holder.describe()}" for holder in live] or [
                        "  held by a managed process that has not recorded itself yet"
                    ]
                    report("\n".join(lines))
                time.sleep(POLL_SECONDS)
    except BaseException:
        os.close(descriptor)
        raise
    identity = ProcessIdentity.of()
    record = _holders_directory(directory, resource) / f"{identity.pid}-{identity.start_ticks}.json"
    write_json_atomic(
        record,
        {
            "identity": identity.to_json(),
            "mode": mode,
            "command": command,
            "started": _process_start(identity),
            "acquired": datetime.now(UTC).isoformat(timespec="seconds"),
            "resource": str(resource.path),
            "kind": resource.kind,
        },
    )
    return _Lock(resource, descriptor, record)


def _release(lock: _Lock) -> None:
    lock.record.unlink(missing_ok=True)
    try:
        fcntl.flock(lock.descriptor, fcntl.LOCK_UN)
    finally:
        os.close(lock.descriptor)


@contextmanager
def ownership(
    mode: Mode,
    requirement: str | Iterable[str],
    *,
    root: Path = ROOT,
    env: Mapping[str, str] | None = None,
    command: str | None = None,
    report: Report = _stderr,
) -> Iterator[Ownership]:
    """Hold ``mode`` ownership of the resources ``requirement`` uses, once per command tree.

    Resources a live ancestor already holds in the same or a stronger mode are reused, not
    locked again, so nested managed operations never deadlock. A nested exclusive request under
    an ancestor's shared ownership raises ``OwnershipConflict`` instead of waiting on itself.
    While inside, ``os.environ`` carries the token for children; ``Ownership.environment``
    applies it to an explicit child environment.
    """
    if mode not in ("shared", "exclusive"):
        raise ValueError(f"unknown ownership mode: {mode}")
    source = os.environ if env is None else env
    resources = resources_for(requirement, root, source)
    if not resources:
        yield Ownership(mode, (), (), source.get(OWNERSHIP_KEY))
        return
    ancestors = inherited(source)
    needed = []
    for resource in resources:
        prior = ancestors.get(resource.name)
        if prior is None:
            needed.append(resource)
        elif mode == "exclusive" and prior[0] != "exclusive":
            owner = prior[1]["owner"]["pid"]
            raise OwnershipConflict(
                f"{resource.kind} {resource.path} is held shared by this command tree"
                f" (pid {owner}); run the exclusive operation outside managed commands"
            )
    label = command or shlex.join(sys.argv)
    locks: list[_Lock] = []
    previous = os.environ.get(OWNERSHIP_KEY)
    try:
        for resource in needed:
            locks.append(_acquire(resource, mode, label, report, source))
        owner = ProcessIdentity.of().to_json()
        entries = [entry for _, entry in ancestors.values()]
        entries += [
            {
                "name": lock.resource.name,
                "path": str(lock.resource.path),
                "mode": mode,
                "owner": owner,
            }
            for lock in locks
        ]
        token = json.dumps(entries, sort_keys=True, separators=(",", ":"))
        os.environ[OWNERSHIP_KEY] = token
        yield Ownership(mode, resources, tuple(lock.resource for lock in locks), token)
    finally:
        if previous is None:
            os.environ.pop(OWNERSHIP_KEY, None)
        else:
            os.environ[OWNERSHIP_KEY] = previous
        for lock in reversed(locks):
            _release(lock)


# ---------------------------------------------------------------------------------------------
# Readiness and preparation


@dataclass(frozen=True)
class Readiness:
    requirement: str
    route: str
    ready: bool
    command: tuple[str, ...]
    detail: str

    @property
    def outcome(self) -> str:
        return "passed" if self.ready else "blocked"

    @property
    def repair(self) -> str:
        return f"just sync {self.route}"

    def message(self) -> str:
        if self.ready:
            return f"{self.requirement}: ready"
        return f"{self.requirement}: blocked: run {self.repair} ({self.detail})"


def uv_environment(
    route: str, root: Path = ROOT, env: Mapping[str, str] | None = None
) -> dict[str, str]:
    """The environment uv runs with for a route: the native key only for ``native``."""
    result = normalized_env(
        dict(os.environ if env is None else env), root, native_inputs=route == "native"
    )
    # uv ignores a non-matching VIRTUAL_ENV with a warning; UV_NO_SYNC is meaningless for sync.
    for key in ("VIRTUAL_ENV", "UV_NO_SYNC"):
        result.pop(key, None)
    if route == "vllm":
        # The root project's selection must never redirect the service's own environment.
        result.pop("UV_PROJECT_ENVIRONMENT", None)
    return result


def sync_command(route: str, root: Path = ROOT, *, check: bool = False) -> tuple[str, ...]:
    flags = ("--locked", "--check") if check else ("--locked",)
    if route == "tools":
        return ("uv", "sync", *flags, "--inexact", "--only-group", "dev")
    if route == "native":
        return ("uv", "sync", *flags, "--inexact")
    if route == "vllm":
        return ("uv", "sync", "--project", str(root.resolve() / VLLM_PROJECT), *flags)
    raise KeyError(f"unknown preparation route: {route}")


def _last_line(text: str) -> str:
    lines = [line.strip() for line in text.splitlines() if line.strip()]
    errors = [line for line in lines if line.startswith("error:")]
    # uv's own advice ("run `uv sync`") would bypass the scoped route and its ownership.
    return (errors or lines or ["no output"])[-1].split("; run `uv sync`")[0]


def observe(
    requirement: str, *, root: Path = ROOT, env: Mapping[str, str] | None = None
) -> Readiness:
    """Read-only readiness of one requirement, with the route that repairs it."""
    route = ROUTE_OF.get(requirement)
    if route is None:
        raise KeyError(f"{requirement} is not a Python environment requirement")
    command = sync_command(route, root, check=True)
    try:
        result = subprocess.run(
            command,
            cwd=root,
            env=uv_environment(route, root, env),
            capture_output=True,
            text=True,
            check=False,
        )
    except OSError as error:
        return Readiness(requirement, route, False, command, f"cannot launch uv: {error}")
    detail = _last_line(result.stderr + result.stdout)
    return Readiness(requirement, route, result.returncode == 0, command, detail)


def sync(
    route: str,
    *,
    root: Path = ROOT,
    env: Mapping[str, str] | None = None,
    report: Report = _stderr,
) -> int:
    """Prepare one route under exclusive ownership; a current environment is left untouched."""
    if route not in ROUTES:
        raise KeyError(f"unknown preparation route: {route}")
    before = observe(route, root=root, env=env)
    if before.ready:
        report(f"sync {route}: already current")
        return 0
    report(f"sync {route}: {before.detail}")
    with ownership("exclusive", route, root=root, env=env, report=report):
        command = sync_command(route, root)
        report("$ " + shlex.join(command))
        try:
            code = subprocess.run(
                command, cwd=root, env=uv_environment(route, root, env), check=False
            ).returncode
        except OSError as error:
            report(f"sync {route}: blocked: cannot launch uv ({error})")
            return 127
    if code:
        report(f"sync {route}: failed (uv exited {code})")
        return code
    after = observe(route, root=root, env=env)
    report(f"sync {route}: {'passed' if after.ready else 'failed: ' + after.detail}")
    return 0 if after.ready else 1


# ---------------------------------------------------------------------------------------------
# CLI


def _all_holders(env: Mapping[str, str] | None = None) -> list[str]:
    lines = []
    directory = lock_directory(env)
    for records in sorted(directory.glob("*.holders")) if directory.is_dir() else ():
        for path in sorted(records.glob("*.json")):
            data = read_json(path)
            if not isinstance(data, dict):
                continue
            identity = ProcessIdentity.from_json(data["identity"])
            if identity.alive():
                lines.append(
                    f"{data['kind']} {data['resource']}: pid {identity.pid} {data['mode']}"
                    f" since {data['acquired']}: {data['command']}"
                )
    return lines


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    commands = parser.add_subparsers(dest="action", required=True)
    commands.add_parser("sync", help="prepare one route").add_argument("route", choices=ROUTES)
    observed = commands.add_parser("observe", help="read-only readiness")
    observed.add_argument("requirements", nargs="+", choices=sorted(ROUTE_OF))
    observed.add_argument("--json", action="store_true")
    hold = commands.add_parser("hold", help="run a command under managed ownership")
    hold.add_argument("mode", choices=("shared", "exclusive"))
    hold.add_argument("requirement", choices=sorted(ROUTE_OF))
    hold.add_argument("command", nargs=argparse.REMAINDER)
    commands.add_parser("holders", help="list live managed holders")
    commands.add_parser("identity", help="print the effective environment and lock paths")
    args = parser.parse_args(argv)

    if args.action == "sync":
        return sync(args.route)
    if args.action == "observe":
        results = [observe(requirement) for requirement in args.requirements]
        if args.json:
            print(
                json.dumps(
                    [
                        {
                            "requirement": r.requirement,
                            "outcome": r.outcome,
                            "repair": None if r.ready else r.repair,
                            "detail": r.detail,
                        }
                        for r in results
                    ],
                    indent=2,
                )
            )
        else:
            print("\n".join(result.message() for result in results))
        return 0 if all(result.ready for result in results) else 1
    if args.action == "hold":
        command = args.command[1:] if args.command[:1] == ["--"] else args.command
        if not command:
            parser.error("hold needs -- COMMAND")
        with ownership(args.mode, args.requirement, command=shlex.join(command)) as owned:
            child = owned.environment(os.environ)
            return subprocess.run(command, env=child, check=False).returncode
    if args.action == "holders":
        print("\n".join(_all_holders()) or "no live managed holders")
        return 0
    print(f"environment: {environment_path()}")
    print(f"vllm environment: {vllm_environment_path()}")
    print(f"locks: {lock_directory()}")
    for resource in resources_for(("native", "vllm")):
        print(f"{resource.kind} {resource.path}: {resource.name}.lock")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
