#!/usr/bin/env python3
"""Optional, correctly prepared checkouts (D3): create, carry into and remove worktrees.

Standard library only; run as ``uv run --no-project --offline --no-python-downloads python
scripts/worktree.py``. A stable checkout can be shared by concurrent commands whose effects do not
conflict; a worktree serves an independent revision or conflicting mutable state.

- ``create NAME [--ref R] [--carry PATH ...] [--build-dir shared|own]`` adds
  ``~/library-context-wt/NAME`` on branch ``wt/NAME`` from a printed ref, then runs ``just ready``
  inside it, which selects and prepares that checkout's own environment and extension.
- ``--carry`` copies the named paths' changes from this checkout without touching it: staged
  changes arrive staged, unstaged changes unstaged, untracked files untracked (binary-safe).
  Tracked changes are carried only for files named exactly; a named directory carries its
  untracked files. Nothing is carried implicitly, so other agents' dirty work never travels
  unless named. Environment and credential material (``.venv``, ``.env*``, ``build/``,
  ``target/``, ``.dev/``) is refused.
- ``remove NAME [--force]`` reports dirty files, commits not integrated into main (by merge or
  cherry-pick) and live managed holders, and refuses without ``--force``.

Work returns by branch merge, cherry-pick or an explicit patch.
"""

from __future__ import annotations

import argparse
import json
import os
import re
import shutil
import subprocess
import sys
import time
from collections.abc import Callable, Mapping, Sequence
from dataclasses import dataclass, field
from pathlib import Path

import workspace_env
from build_environment import BUILD_DIR_SELECTION, ROOT

Report = Callable[[str], None]
NAME = re.compile(r"[A-Za-z0-9][A-Za-z0-9._-]{0,63}")
EXCLUDED_TOP = {"build", "target", ".dev"}


def default_base() -> Path:
    return Path(os.environ.get("LCTX_WORKTREE_BASE", "~/library-context-wt")).expanduser()


def _say(message: str) -> None:
    print(message, flush=True)


def git(
    *args: str,
    cwd: Path,
    data: bytes | None = None,
    check: bool = True,
) -> subprocess.CompletedProcess[bytes]:
    """Run git; optional locks are off so reading the source checkout never rewrites its index."""
    env = dict(os.environ, GIT_OPTIONAL_LOCKS="0")
    result = subprocess.run(
        ("git", *args), cwd=cwd, input=data, capture_output=True, env=env, check=False
    )
    if check and result.returncode:
        raise RuntimeError(f"git {' '.join(args)}: {result.stderr.decode().strip()}")
    return result


def branch_exists(root: Path, branch: str) -> bool:
    ref = f"refs/heads/{branch}"
    return bool(git("rev-parse", "--verify", "--quiet", ref, cwd=root, check=False).stdout)


def _names(output: bytes) -> list[str]:
    return [name for name in output.decode().split("\0") if name]


def excluded(path: str) -> str | None:
    parts = Path(path).parts
    if parts and parts[0] in EXCLUDED_TOP:
        return f"{parts[0]}/ is local build or environment state"
    for part in parts:
        if part == ".venv" or part.startswith(".env"):
            return f"{part} is environment or credential material"
    return None


# ---------------------------------------------------------------------------------------------
# Carry


@dataclass
class CarryPlan:
    staged: list[str] = field(default_factory=list)
    unstaged: list[str] = field(default_factory=list)
    untracked: list[str] = field(default_factory=list)
    skipped: list[str] = field(default_factory=list)


def changed_paths(root: Path) -> list[str]:
    """Every changed or untracked path in a checkout (for naming what to carry)."""
    staged = _names(git("diff", "--cached", "--name-only", "--no-renames", "-z", cwd=root).stdout)
    unstaged = _names(git("diff", "--name-only", "--no-renames", "-z", cwd=root).stdout)
    untracked = _names(git("ls-files", "--others", "--exclude-standard", "-z", cwd=root).stdout)
    return sorted({*staged, *unstaged, *untracked})


def carry_plan(root: Path, paths: Sequence[str]) -> CarryPlan:
    """Select exactly what the named paths carry; raise when a name is unsafe or carries nothing."""
    root = root.resolve()
    plan = CarryPlan()
    for raw in paths:
        candidate = Path(raw)
        absolute = (candidate if candidate.is_absolute() else Path.cwd() / candidate).resolve()
        if not absolute.is_relative_to(root):
            absolute = (root / candidate).resolve()
        if not absolute.is_relative_to(root):
            raise ValueError(f"{raw} is outside the source checkout")
        name = absolute.relative_to(root).as_posix()
        if name in ("", "."):
            raise ValueError("name files or directories to carry, not the whole checkout")
        reason = excluded(name)
        if reason:
            raise ValueError(f"refusing to carry {name}: {reason}")
        spec = f":(literal){name}"
        staged = _names(
            git(
                "diff", "--cached", "--name-only", "--no-renames", "-z", "--", spec, cwd=root
            ).stdout
        )
        unstaged = _names(
            git("diff", "--name-only", "--no-renames", "-z", "--", spec, cwd=root).stdout
        )
        untracked = _names(
            git("ls-files", "--others", "--exclude-standard", "-z", "--", spec, cwd=root).stdout
        )
        found = False
        for kind, files in (("staged", staged), ("unstaged", unstaged)):
            for file in files:
                if file == name:
                    getattr(plan, kind).append(file)
                    found = True
                else:
                    plan.skipped.append(f"{file} ({kind} tracked change under {name}; name it)")
        for file in untracked:
            reason = excluded(file)
            if reason:
                plan.skipped.append(f"{file} ({reason})")
            else:
                plan.untracked.append(file)
                found = True
        if not found:
            raise ValueError(f"{name} has no carriable changes")
    for kind in ("staged", "unstaged", "untracked"):
        setattr(plan, kind, sorted(set(getattr(plan, kind))))
    plan.skipped = sorted(set(plan.skipped))
    return plan


def apply_carry(root: Path, plan: CarryPlan, target: Path, report: Report = _say) -> list[str]:
    """Copy a plan into ``target``; returns conflicts. The source checkout is only read."""
    conflicts: list[str] = []
    if plan.staged:
        patch = git("diff", "--cached", "--binary", "--no-renames", "--", *plan.staged, cwd=root)
        applied = git("apply", "--index", "--3way", cwd=target, data=patch.stdout, check=False)
        unmerged = _names(git("diff", "--name-only", "--diff-filter=U", "-z", cwd=target).stdout)
        if applied.returncode:
            detail = applied.stderr.decode().strip().splitlines()
            conflicts += [f"{path} (staged change conflicts)" for path in unmerged] or [
                f"staged changes not applied: {detail[-1] if detail else 'git apply failed'}"
            ]
    if plan.unstaged:
        patch = git("diff", "--binary", "--no-renames", "--", *plan.unstaged, cwd=root)
        applied = git("apply", cwd=target, data=patch.stdout, check=False)
        if applied.returncode:
            detail = applied.stderr.decode().strip().splitlines()
            conflicts.append(
                f"unstaged changes to {', '.join(plan.unstaged)} not applied:"
                f" {detail[-1] if detail else 'git apply failed'}"
            )
    for name in plan.untracked:
        source, destination = root / name, target / name
        if destination.exists() or destination.is_symlink():
            conflicts.append(f"{name} (untracked here, present at the worktree's ref)")
            continue
        destination.parent.mkdir(parents=True, exist_ok=True)
        if source.is_symlink():
            destination.symlink_to(os.readlink(source))
        else:
            shutil.copy2(source, destination)
    report(
        "carry: staged changes arrive staged (index and files): "
        + (", ".join(plan.staged) or "none")
    )
    report("carry: unstaged changes arrive unstaged: " + (", ".join(plan.unstaged) or "none"))
    report("carry: untracked files arrive untracked: " + (", ".join(plan.untracked) or "none"))
    for skipped in plan.skipped:
        report(f"carry: not carried: {skipped}")
    for conflict in conflicts:
        report(f"carry: conflict: {conflict}")
    report("carry: the source checkout was only read")
    return conflicts


# ---------------------------------------------------------------------------------------------
# Create and remove


def _cargo_home() -> Path:
    return Path(os.environ.get("CARGO_HOME") or Path.home() / ".cargo")


def own_build_dir(name: str) -> Path:
    return _cargo_home() / "build" / f"library-context-wt-{name}"


def _disk(path: Path) -> str:
    if not path.exists():
        return "absent"
    result = subprocess.run(("du", "-sh", str(path)), capture_output=True, text=True, check=False)
    return result.stdout.split()[0] if result.returncode == 0 else "unknown"


def create(
    name: str,
    *,
    ref: str = "HEAD",
    carry: Sequence[str] | None = None,
    build_dir: str = "shared",
    root: Path = ROOT,
    base: Path | None = None,
    prepare: bool = True,
    report: Report = _say,
) -> int:
    if not NAME.fullmatch(name):
        raise ValueError(f"worktree names match {NAME.pattern}")
    root = root.resolve()
    target = (base or default_base()) / name
    branch = f"wt/{name}"
    if target.exists():
        raise ValueError(f"{target} already exists")
    if branch_exists(root, branch):
        raise ValueError(f"branch {branch} already exists")
    commit = git("rev-parse", "--verify", f"{ref}^{{commit}}", cwd=root).stdout.decode().strip()
    plan = None
    if carry is not None:
        if not carry:
            available = "\n  ".join(changed_paths(root)) or "(no changes)"
            raise ValueError(f"--carry needs the paths to carry; changed here:\n  {available}")
        plan = carry_plan(root, carry)
    report(f"worktree: {target} on {branch} from {ref} = {commit}")
    git("worktree", "add", "-b", branch, str(target), commit, cwd=root)
    conflicts = apply_carry(root, plan, target, report) if plan else []
    if build_dir == "own":
        selected = own_build_dir(name)
        (target / BUILD_DIR_SELECTION).parent.mkdir(parents=True, exist_ok=True)
        (target / BUILD_DIR_SELECTION).write_text(f"{selected}\n")
        report(f"build dir: {selected} (own; recipes and `just env --` use it, bare cargo not)")
    else:
        report("build dir: shared (.cargo/config.toml, ADR-0136)")
    code = 0
    if prepare:
        env, notes = ready_environment(target, os.environ)
        for note in notes:
            report(note)
        report(f"$ just ready  (in {target})")
        started = time.monotonic()
        code = subprocess.run(("just", "ready"), cwd=target, env=env, check=False).returncode
        elapsed = time.monotonic() - started
        report(
            f"preparation (information only): {elapsed:.0f} s; worktree {_disk(target)},"
            f" .venv {_disk(target / '.venv')}"
            + (f", own build dir {_disk(own_build_dir(name))}" if build_dir == "own" else "")
        )
        if code:
            report(f"ready failed ({code}); the worktree is kept: fix it there or remove it")
    report(f"remove with: just worktree-remove {name}")
    return 1 if conflicts else code


def ready_environment(target: Path, source: Mapping[str, str]) -> tuple[dict[str, str], list[str]]:
    """The environment for the new checkout's own `just ready`.

    The worktree runs its ref's `ready`, which may predate checkout selection: an inherited
    absolute UV_PROJECT_ENVIRONMENT would then prepare (and repoint) another checkout's
    environment. Select the worktree's own `.venv` here as well, and drop the caller's ownership.
    """
    env = {k: v for k, v in source.items() if k not in ("LCTX_ENV_OWNERSHIP", "VIRTUAL_ENV")}
    selected = str(target / ".venv")
    notes = []
    inherited = source.get("UV_PROJECT_ENVIRONMENT", "").strip()
    if inherited and inherited != selected:
        notes.append(
            f"environment: {selected} (replaced inherited UV_PROJECT_ENVIRONMENT={inherited};"
            " commands you run in the worktree still inherit it: unset it there)"
        )
    env["UV_PROJECT_ENVIRONMENT"] = selected
    return env, notes


LIVE_UNIT_STATES = ("active", "activating", "reloading", "deactivating")
SCRIPTS = ROOT / "scripts"


@dataclass(frozen=True)
class Live:
    """Live state a worktree removal would orphan: a fixture server or a running run."""

    kind: str
    ident: str
    detail: str
    stop: tuple[str, ...] | None  # (script, args…) of the owning harness's stop route

    def describe(self) -> str:
        return f"{self.kind} {self.ident}: {self.detail}"


def _harness(script: str, args: Sequence[str], key: str, directory: Path):
    """Run a harness script over another checkout's state (its root relocated by ``key``)."""
    env = {
        k: v
        for k, v in os.environ.items()
        if k not in ("LCTX_ENV_OWNERSHIP", "LCTX_RUN_DIR", "LCTX_RUN_ID", "UV_NO_SYNC")
    }
    env[key] = str(directory)
    return subprocess.run(
        (sys.executable, str(SCRIPTS / script), *args),
        env=env,
        capture_output=True,
        text=True,
        check=False,
    )


def _listing(script: str, args: Sequence[str], key: str, directory: Path) -> list[dict]:
    result = _harness(script, args, key, directory)
    try:
        rows = json.loads(result.stdout) if result.returncode == 0 else None
    except ValueError:
        rows = None
    if not isinstance(rows, list):
        detail = (result.stderr.strip().splitlines() or ["no output"])[-1]
        raise RuntimeError(f"cannot inspect {directory} with {script}: {detail}")
    return rows


def _unit_description(unit: str) -> str:
    from surrealdb_fixture import systemd_environment

    result = subprocess.run(
        ("systemctl", "--user", "show", "-p", "Description", "--value", unit),
        capture_output=True,
        text=True,
        check=False,
        env=systemd_environment(),
    )
    return result.stdout.strip()


def live_state(target: Path) -> list[Live]:
    """Kept or run-owned fixtures and running runs that belong to a checkout.

    Read through the owning harnesses' own listings (``surrealdb_fixture.py --list``,
    ``runs.py list``) with their roots relocated to the checkout; an inspection failure raises,
    since unknown is not absent.
    """
    found: list[Live] = []
    fixtures = target / "build" / "fixtures"
    for row in _listing(
        "surrealdb_fixture.py", ("--list", "--json"), "LCTX_FIXTURES_ROOT", fixtures
    ):
        if row.get("protected"):
            recover = row.get("kind") in {"kept", "run"} and not (
                row.get("kind") == "run" and row.get("owner_alive")
            )
            found.append(
                Live(
                    "unresolved fixture",
                    row["id"],
                    f"{row.get('unit', 'unreadable fixture record')}, "
                    f"cleanup {row.get('cleanup', 'unknown')}",
                    ("surrealdb_fixture.py", "--stop", row["id"], "--force", "--no-sweep")
                    if recover
                    else None,
                )
            )
            continue
        state = str(row.get("state") or "")
        if row.get("kind") == "unrecorded":
            # `systemctl list-units` columns after the unit: LOAD ACTIVE SUB DESCRIPTION.
            columns = state.split()
            active = len(columns) > 1 and columns[1] in LIVE_UNIT_STATES
        else:
            active = state.split("/")[0] in LIVE_UNIT_STATES
        if row.get("kind") == "kept" and active:
            found.append(
                Live(
                    "kept fixture",
                    row["id"],
                    f"{row['unit']} on port {row.get('port')},"
                    f" attachments {row.get('attachments')}",
                    ("surrealdb_fixture.py", "--stop", row["id"], "--force", "--no-sweep"),
                )
            )
        elif row.get("kind") == "run" and row.get("owner_alive"):
            found.append(
                Live(
                    "run-owned fixture",
                    row["id"],
                    f"{row['unit']}, launcher pid {row.get('owner')} (cancel its command)",
                    None,
                )
            )
        elif (
            row.get("kind") == "unrecorded"
            and active
            and f"checkout={target}" in _unit_description(row["unit"])
        ):
            found.append(
                Live(
                    "unrecorded fixture unit",
                    row["unit"],
                    f"its record is gone; stop it with systemctl --user stop {row['unit']}",
                    None,
                )
            )
    runs = target / "build" / "runs"
    for row in _listing("runs.py", ("list", "--json"), "LCTX_RUNS_ROOT", runs):
        if row.get("protected", True) or row.get("owner_live") or row.get("survivors"):
            text = row.get("label") or " ".join(row.get("argv") or [])
            kind = "running run" if row.get("state") == "running" else "unresolved run"
            cleanup = (row.get("cleanup") or {}).get("status", "unknown")
            found.append(
                Live(
                    kind, row["id"], f"{text}; cleanup {cleanup}", ("runs.py", "cancel", row["id"])
                )
            )
    return found


@dataclass
class Removal:
    dirty: list[str]
    unintegrated: list[str]
    holders: list[str]
    live: list[Live]

    def blocked(self) -> bool:
        return bool(self.dirty or self.unintegrated or self.holders or self.live)


def inspect(root: Path, target: Path, branch: str, into: str) -> Removal:
    dirty = []
    if target.exists():
        status = git("status", "--porcelain=v1", "--untracked-files=all", cwd=target).stdout
        dirty = [line for line in status.decode().splitlines() if line]
    unintegrated = []
    if branch_exists(root, branch):
        # `git cherry` treats a cherry-picked (patch-equivalent) commit as integrated.
        cherry = git("cherry", "-v", into, branch, cwd=root).stdout.decode().splitlines()
        unintegrated = [line[2:] for line in cherry if line.startswith("+ ")]
    holders = []
    if target.exists():
        for resource in workspace_env.resources_for("native", target, {}):
            holders += [holder.describe() for holder in workspace_env.holders(resource)]
    return Removal(dirty, unintegrated, holders, live_state(target))


def remove(
    name: str,
    *,
    force: bool = False,
    into: str = "main",
    root: Path = ROOT,
    base: Path | None = None,
    report: Report = _say,
) -> int:
    if not NAME.fullmatch(name):
        raise ValueError(f"worktree names match {NAME.pattern}")
    root = root.resolve()
    target = (base or default_base()) / name
    branch = f"wt/{name}"
    found = inspect(root, target, branch, into)
    for line in found.dirty:
        report(f"dirty: {line}")
    for line in found.unintegrated:
        report(f"not integrated into {into}: {line}")
    for line in found.holders:
        report(f"in use: {line}")
    for item in found.live:
        report(f"live: {item.describe()}")
    if found.blocked() and not force:
        report(f"worktree-remove: refused ({target} kept); --force discards the work listed above")
        return 1
    if found.live:
        # Stop through the owners' routes (runs first: a run's fixtures die with its launcher);
        # never by deleting their records.
        for item in sorted(found.live, key=lambda live: live.kind != "running run"):
            if item.stop is None:
                continue
            script, *args = item.stop
            directory = target / "build" / ("runs" if script == "runs.py" else "fixtures")
            key = "LCTX_RUNS_ROOT" if script == "runs.py" else "LCTX_FIXTURES_ROOT"
            result = _harness(script, args, key, directory)
            outcome = (result.stdout.strip() or result.stderr.strip()).splitlines()
            report(
                f"stopped {item.kind} {item.ident}: {outcome[-1] if outcome else result.returncode}"
            )
        remaining = live_state(target)
        if remaining:
            for item in remaining:
                report(f"still live: {item.describe()}")
            report(f"worktree-remove: refused ({target} kept); stop the state listed above first")
            return 1
    selection = target / BUILD_DIR_SELECTION
    own = Path(selection.read_text().strip()) if selection.is_file() else None
    if target.exists():
        git("worktree", "remove", *(["--force"] * 2 if force else []), str(target), cwd=root)
        report(f"removed worktree {target}")
    else:
        git("worktree", "prune", cwd=root)
    if branch_exists(root, branch):
        tip = git("rev-parse", "--short", branch, cwd=root).stdout.decode().strip()
        git("branch", "-D", branch, cwd=root)
        report(f"deleted branch {branch} (was {tip})")
    if own is not None:
        if own == own_build_dir(name) and own.is_dir():
            size = _disk(own)
            shutil.rmtree(own)
            report(f"removed own build dir {own} ({size})")
        elif own.exists():
            report(f"kept build dir {own} (not this worktree's own path)")
    return 0


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    commands = parser.add_subparsers(dest="action", required=True)
    made = commands.add_parser("create", help="create and prepare a worktree")
    made.add_argument("name")
    made.add_argument("--ref", default="HEAD")
    made.add_argument("--carry", nargs="*", metavar="PATH")
    made.add_argument("--build-dir", choices=("shared", "own"), default="shared")
    made.add_argument("--no-ready", action="store_true", help="skip `just ready` (tests)")
    removed = commands.add_parser("remove", help="remove a clean, integrated worktree")
    removed.add_argument("name")
    removed.add_argument("--force", action="store_true")
    removed.add_argument("--into", default="main", help="integration branch (default main)")
    args = parser.parse_args(argv)
    try:
        if args.action == "create":
            return create(
                args.name,
                ref=args.ref,
                carry=args.carry,
                build_dir=args.build_dir,
                prepare=not args.no_ready,
            )
        return remove(args.name, force=args.force, into=args.into)
    except (ValueError, RuntimeError) as error:
        print(f"worktree: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
