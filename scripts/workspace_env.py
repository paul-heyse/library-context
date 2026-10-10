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
  process. Holders are read from ``/proc/locks`` (pid, mode) and ``/proc/<pid>`` (command,
  start), so no holder records exist to go stale. flock has no writer preference, so a pending
  exclusive request holds a per-resource gate that new shared acquirers queue behind.
  Children reuse ownership through ``LCTX_ENV_OWNERSHIP`` while the recorded owner is alive (in
  another pid namespace: while its lock is still held in the recorded mode).
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
from typing import Any, Literal

from build_environment import ROOT, explain, normalized_env, project_environment
from harness import ProcessIdentity

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
# Holders, read from the kernel's lock table (no records to write, sweep or outlive a reboot)


def _boot_time() -> float:
    for line in Path("/proc/stat").read_text().splitlines():
        if line.startswith("btime "):
            return float(line.split()[1])
    return 0.0


def _process_start(pid: int) -> str:
    try:
        ticks = ProcessIdentity.of(pid).start_ticks
    except OSError:
        return "?"
    seconds = _boot_time() + ticks / os.sysconf("SC_CLK_TCK")
    return datetime.fromtimestamp(seconds, UTC).isoformat(timespec="seconds")


def _command(pid: int) -> str:
    try:
        argv = Path(f"/proc/{pid}/cmdline").read_bytes().split(b"\0")
    except OSError:
        return "(exited)"
    return shlex.join(part.decode(errors="replace") for part in argv if part) or "(unknown)"


@dataclass(frozen=True)
class Holder:
    pid: int
    mode: str
    command: str
    started: str

    def describe(self) -> str:
        if self.pid <= 0:
            return f"a process in another pid namespace ({self.mode})"
        return f"pid {self.pid} ({self.mode}, started {self.started}): {self.command}"


def lock_path(
    resource: Resource, env: Mapping[str, str] | None = None, *, gate: bool = False
) -> Path:
    return lock_directory(env) / f"{resource.name}.{'gate' if gate else 'lock'}"


def _flock_holders(path: Path) -> list[tuple[int, str]]:
    """(pid, mode) of every granted flock on ``path``, from /proc/locks."""
    try:
        stat = path.stat()
        table = Path("/proc/locks").read_text()
    except OSError:
        return []
    key = f"{os.major(stat.st_dev):02x}:{os.minor(stat.st_dev):02x}:{stat.st_ino}"
    found = []
    for line in table.splitlines():
        fields = line.split()
        if len(fields) >= 6 and fields[1] == "FLOCK" and fields[5] == key:
            found.append((int(fields[4]), "exclusive" if fields[3] == "WRITE" else "shared"))
    return found


def holders(
    resource: Resource, env: Mapping[str, str] | None = None, *, gate: bool = False
) -> list[Holder]:
    """Live holders of a resource lock (or of its pending-writer gate)."""
    return [
        Holder(pid, mode, _command(pid), _process_start(pid))
        for pid, mode in sorted(set(_flock_holders(lock_path(resource, env, gate=gate))))
    ]


# ---------------------------------------------------------------------------------------------
# Ownership


class OwnershipConflict(RuntimeError):
    """A nested managed operation needs a stronger mode than its command tree already holds."""


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


def _entry_live(entry: Mapping[str, Any], env: Mapping[str, str] | None) -> bool:
    """The recorded owner is alive; in another pid namespace, its lock is still held as recorded."""
    owner = ProcessIdentity.from_json(entry["owner"])
    if owner.alive():
        return True
    if not owner.foreign():
        return False
    held = _flock_holders(lock_directory(env) / f"{entry['name']}.lock")
    return any(mode == entry["mode"] for _, mode in held)


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
            if _entry_live(entry, env):
                held[entry["name"]] = (entry["mode"], entry)
        except KeyError, TypeError, ValueError:
            continue
    return held


def _open(path: Path) -> int:
    path.parent.mkdir(parents=True, exist_ok=True)
    return os.open(path, os.O_RDWR | os.O_CREAT | os.O_CLOEXEC, 0o600)


def _wait_flock(
    descriptor: int,
    operation: int,
    describe: Callable[[], tuple[tuple[int, ...], str]],
    report: Report,
) -> None:
    """Poll a non-blocking flock, reporting whenever the set of blocking processes changes."""
    reported: tuple[int, ...] | None = None
    while True:
        try:
            fcntl.flock(descriptor, operation | fcntl.LOCK_NB)
            return
        except BlockingIOError:
            seen, message = describe()
            if seen != reported:
                reported = seen
                report(message)
            time.sleep(POLL_SECONDS)


def _acquire(resource: Resource, mode: Mode, report: Report, env: Mapping[str, str]) -> int:
    """Take one resource lock behind its pending-writer gate; returns the held descriptor.

    flock has no writer preference, so an exclusive waiter first holds the gate exclusively and
    keeps it until it owns the resource; shared acquirers pass the gate (briefly, shared) before
    the resource and therefore queue behind a pending sync instead of starving it. Existing
    holders finish normally.
    """
    gate = _open(lock_path(resource, env, gate=True))
    descriptor = _open(lock_path(resource, env))
    try:

        def pending() -> tuple[tuple[int, ...], str]:
            writers = [h for h in holders(resource, env, gate=True) if h.mode == "exclusive"]
            pids = ", ".join(str(h.pid) for h in writers) or "unknown"
            return (
                tuple(h.pid for h in writers),
                f"waiting for pending sync (pid {pids}) of {resource.kind} {resource.path}",
            )

        def blocking() -> tuple[tuple[int, ...], str]:
            live = holders(resource, env)
            lines = [f"waiting for {mode} {resource.kind} ownership of {resource.path}:"]
            lines += [f"  held by {holder.describe()}" for holder in live] or [
                "  held by a process that has just released it"
            ]
            return tuple(h.pid for h in live), "\n".join(lines)

        gate_mode = fcntl.LOCK_EX if mode == "exclusive" else fcntl.LOCK_SH
        _wait_flock(gate, gate_mode, pending, report)
        operation = fcntl.LOCK_SH if mode == "shared" else fcntl.LOCK_EX
        _wait_flock(descriptor, operation, blocking, report)
    except BaseException:
        os.close(descriptor)
        raise
    finally:
        os.close(gate)
    return descriptor


@contextmanager
def _resource_ownership(
    mode: Mode,
    requirement: str | Iterable[str],
    *,
    root: Path = ROOT,
    env: Mapping[str, str] | None = None,
    command: str | None = None,  # accepted for callers; holders are reported from /proc
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
    held: list[tuple[Resource, int]] = []
    previous = os.environ.get(OWNERSHIP_KEY)
    try:
        for resource in needed:
            held.append((resource, _acquire(resource, mode, report, source)))
        owner = ProcessIdentity.of().to_json()
        entries = [entry for _, entry in ancestors.values()]
        entries += [
            {"name": resource.name, "path": str(resource.path), "mode": mode, "owner": owner}
            for resource, _ in held
        ]
        token = json.dumps(entries, sort_keys=True, separators=(",", ":"))
        os.environ[OWNERSHIP_KEY] = token
        yield Ownership(mode, resources, tuple(resource for resource, _ in held), token)
    finally:
        if previous is None:
            os.environ.pop(OWNERSHIP_KEY, None)
        else:
            os.environ[OWNERSHIP_KEY] = previous
        for _, descriptor in reversed(held):
            os.close(descriptor)  # closing the only descriptor releases the flock


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
    """Storage admission precedes existing environment/extension ownership."""
    import storage_owners
    from storage_lifecycle import admission

    resources = resources_for(requirement, root, env)
    with (
        admission([resource.path for resource in resources]),
        _resource_ownership(
            mode, requirement, root=root, env=env, command=command, report=report
        ) as owned,
    ):
        existing = set()
        for resource in resources:
            if resource.path.exists():
                storage_owners.enroll(
                    resource.path,
                    "environment",
                    {
                        "kind": "environment",
                        "path": str(root.resolve()),
                        "resource_kind": resource.kind,
                        "resource_rank": resource.rank,
                    },
                    f"checkout:{root.resolve()}",
                )
                existing.add(resource.path)
        try:
            yield owned
        finally:
            # Preparation can create a previously absent environment/extension. Publish
            # while both admissions still cover it, including interrupted preparation.
            for resource in resources:
                if resource.path not in existing and resource.path.exists():
                    storage_owners.enroll(
                        resource.path,
                        "environment",
                        {
                            "kind": "environment",
                            "path": str(root.resolve()),
                            "resource_kind": resource.kind,
                            "resource_rank": resource.rank,
                        },
                        f"checkout:{root.resolve()}",
                    )


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
    with ownership("exclusive", route, root=root, env=env, report=report) as owned:
        selected = owned.environment(os.environ if env is None else env)
        before = observe(route, root=root, env=selected)
        if before.ready:
            report(f"sync {route}: already current")
            return 0
        report(f"sync {route}: {before.detail}")
        command = sync_command(route, root)
        report("$ " + shlex.join(command))
        try:
            code = subprocess.run(
                command, cwd=root, env=uv_environment(route, root, selected), check=False
            ).returncode
        except OSError as error:
            report(f"sync {route}: blocked: cannot launch uv ({error})")
            return 127
        if code:
            report(f"sync {route}: failed (uv exited {code})")
            return code
        after = observe(route, root=root, env=selected)
        report(f"sync {route}: {'passed' if after.ready else 'failed: ' + after.detail}")
        return 0 if after.ready else 1


# ---------------------------------------------------------------------------------------------
# Checkout selection (`just ready` in any checkout, D3)


def selection(
    root: Path = ROOT, env: Mapping[str, str] | None = None
) -> tuple[dict[str, str | None], list[str]]:
    """Variables that select this checkout's own environment (None unsets), and what they replace.

    An inherited absolute ``UV_PROJECT_ENVIRONMENT`` (for example main's ``.venv`` carried into a
    worktree session) would make uv prepare and import another checkout's environment; checking
    ``VIRTUAL_ENV`` alone does not catch it.
    """
    source = os.environ if env is None else env
    selected = root.resolve() / ".venv"
    changes: dict[str, str | None] = {"UV_PROJECT_ENVIRONMENT": str(selected)}
    notes = []
    inherited = source.get("UV_PROJECT_ENVIRONMENT", "").strip()
    if inherited and Path(os.path.abspath(root.resolve() / inherited)) != selected:
        notes.append(f"replaced inherited UV_PROJECT_ENVIRONMENT={inherited} with {selected}")
    active = source.get("VIRTUAL_ENV", "").strip()
    if active and Path(os.path.abspath(active)) != selected:
        changes["VIRTUAL_ENV"] = None
        notes.append(f"unset inherited VIRTUAL_ENV={active} (another environment)")
    return changes, notes or [f"selected environment {selected}"]


def checkout_report(
    root: Path = ROOT, env: Mapping[str, str] | None = None
) -> tuple[list[str], bool]:
    """Interpreter, extension import origin, build directory and lock identity of a checkout."""
    source = dict(os.environ if env is None else env)
    venv = environment_path(root, source)
    probe = (
        "import sys; print(sys.executable); import lctx_semantics; print(lctx_semantics.__file__)"
    )
    child = normalized_env(source, root)
    for key in ("VIRTUAL_ENV", "UV_NO_SYNC"):
        child.pop(key, None)
    try:
        result = subprocess.run(
            ("uv", "run", "--no-sync", "python", "-c", probe),
            cwd=root,
            env=child,
            capture_output=True,
            text=True,
            check=False,
        )
        output = result.stdout.split()
    except OSError as error:
        result, output = None, [f"(cannot launch uv: {error})"]
    interpreter = output[0] if output else _last_line(result.stderr if result else "")
    origin = output[1] if len(output) > 1 else (_last_line(result.stderr) if result else "")
    ok = (
        result is not None
        and result.returncode == 0
        and Path(interpreter).is_relative_to(venv)
        and Path(origin).resolve().is_relative_to(root.resolve())
    )
    build = next(
        (line.split(":", 1)[1].strip() for line in explain(source, root) if "build dir" in line),
        "(unknown)",
    )
    lines = [
        f"checkout:         {root.resolve()}",
        f"environment:      {venv}",
        f"interpreter:      {interpreter}",
        f"extension origin: {origin}",
        f"build dir:        {build}",
    ]
    lines += [
        f"{resource.kind + ' lock:':<17} {resource.name}.lock ({resource.path})"
        for resource in resources_for("native", root, source)
    ]
    if not ok:
        lines.append("blocked: interpreter or extension does not follow this checkout")
    return lines, ok


# ---------------------------------------------------------------------------------------------
# CLI


def _all_holders(env: Mapping[str, str] | None = None) -> list[str]:
    directory = lock_directory(env)
    lines = []
    for path in sorted(directory.glob("*.lock")) if directory.is_dir() else ():
        for pid, mode in _flock_holders(path):
            lines.append(
                f"{path.stem}: pid {pid} {mode}, started {_process_start(pid)}: {_command(pid)}"
            )
    return lines


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    commands = parser.add_subparsers(dest="action", required=True)
    commands.add_parser("sync", help="prepare one route").add_argument("route", choices=ROUTES)
    hold = commands.add_parser("hold", help="run a command under managed ownership")
    hold.add_argument("mode", choices=("shared", "exclusive"))
    hold.add_argument("requirement", choices=sorted(ROUTE_OF))
    hold.add_argument("command", nargs=argparse.REMAINDER)
    commands.add_parser("holders", help="list live managed holders")
    commands.add_parser("select", help="shell lines selecting this checkout's own environment")
    commands.add_parser("report", help="interpreter, import origin, build dir and locks")
    args = parser.parse_args(argv)

    if args.action == "sync":
        return sync(args.route)
    if args.action == "hold":
        command = args.command[1:] if args.command[:1] == ["--"] else args.command
        if not command:
            parser.error("hold needs -- COMMAND")
        with ownership(args.mode, args.requirement) as owned:
            child = owned.environment(os.environ)
            return subprocess.run(command, env=child, check=False).returncode
    if args.action == "select":
        changes, notes = selection()
        for note in notes:
            _stderr(f"ready: {note}")
        for key, value in changes.items():
            print(f"unset {key}" if value is None else f"export {key}={shlex.quote(value)}")
        return 0
    if args.action == "report":
        lines, ok = checkout_report()
        print("\n".join(lines))
        return 0 if ok else 1
    print("\n".join(_all_holders()) or "no live managed holders")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
