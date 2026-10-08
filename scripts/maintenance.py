"""Scoped end-of-turn maintenance (ADR-0134).

Whole-tree `just turn-end` stays the default. When the tree holds another agent's uncommitted
work, `just turn-end --paths P…` (or `--staged`) formats only those paths and runs a generator
only when its inputs are in scope; every skipped step is reported for a later whole-tree run.
Rust files are formatted with `rustfmt --skip-children`, so named files never pull in child
modules that belong to someone else. Standard library only (plan §5.8).
"""

from __future__ import annotations

import argparse
import subprocess
import sys
import tomllib
from collections.abc import Callable, Iterable, Sequence
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
Runner = Callable[[Sequence[str]], int]

HAKARI_INPUTS = ("Cargo.toml", "Cargo.lock", ".config/hakari.toml")


@dataclass(frozen=True)
class Step:
    name: str
    ran: bool
    reason: str
    code: int = 0


def run(command: Sequence[str]) -> int:
    print("$ " + " ".join(command), flush=True)
    return subprocess.run(list(command), cwd=ROOT, check=False).returncode


def edition(root: Path = ROOT) -> str:
    manifest = tomllib.loads((root / "Cargo.toml").read_text())
    return str(manifest.get("workspace", {}).get("package", {}).get("edition", "2024"))


def staged(root: Path = ROOT) -> list[str]:
    output = subprocess.run(
        ["git", "diff", "--cached", "--name-only", "--diff-filter=ACMR"],
        cwd=root,
        check=True,
        capture_output=True,
        text=True,
    ).stdout
    return [line for line in output.splitlines() if line]


def in_scope(paths: Iterable[str], root: Path = ROOT) -> list[str]:
    """Repository-relative existing files; anything outside the checkout is refused."""
    selected = []
    for raw in paths:
        path = (root / raw).resolve()
        if not path.is_relative_to(root):
            raise SystemExit(f"maintenance: {raw} is outside the checkout")
        if path.is_file():
            selected.append(path.relative_to(root).as_posix())
    return sorted(set(selected))


def fmt(paths: Sequence[str], runner: Runner = run, root: Path = ROOT) -> list[Step]:
    rust = [p for p in paths if p.endswith(".rs")]
    python = [p for p in paths if p.endswith((".py", ".pyi"))]
    steps = []
    if rust:
        code = runner(
            ["rustfmt", "--edition", edition(root), "--unstable-features", "--skip-children", *rust]
        )
        steps.append(Step("rustfmt", True, f"{len(rust)} file(s)", code))
    else:
        steps.append(Step("rustfmt", False, "no Rust files in scope"))
    if python:
        code = runner(["uv", "run", "--no-sync", "ruff", "format", *python])
        code = code or runner(
            ["uv", "run", "--no-sync", "ruff", "check", "--fix", "--quiet", "--exit-zero", *python]
        )
        steps.append(Step("ruff", True, f"{len(python)} file(s)", code))
    else:
        steps.append(Step("ruff", False, "no Python files in scope"))
    return steps


def turn_end(paths: Sequence[str], runner: Runner = run, root: Path = ROOT) -> list[Step]:
    steps = []
    if any(p.startswith("docs/adr/") for p in paths):
        steps.append(Step("adr-index", True, "ADR records in scope", runner(["just", "adr-index"])))
    else:
        steps.append(Step("adr-index", False, "no ADR records in scope"))
    if any(p in HAKARI_INPUTS or p.endswith("/Cargo.toml") for p in paths):
        code = runner(["just", "build-features"])
        steps.append(Step("build-features", True, "Cargo manifests in scope", code))
    else:
        steps.append(Step("build-features", False, "no Cargo manifests in scope"))
    return steps + fmt(paths, runner, root)


def report(name: str, steps: Sequence[Step]) -> int:
    failed = [step.name for step in steps if step.code]
    for step in steps:
        state = ("failed" if step.code else "ran") if step.ran else "skipped"
        print(f"{name}: {step.name} {state} ({step.reason})")
    skipped = [step.name for step in steps if not step.ran]
    if skipped:
        print(
            f"{name}: scoped; not performed: {', '.join(skipped)}. "
            "Run whole-tree `just turn-end` once the tree holds no one else's work."
        )
    if failed:
        print(f"{name}: {len(failed)} failed: {' '.join(failed)}")
        return 1
    return 0


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=("fmt", "turn-end"))
    parser.add_argument("--staged", action="store_true", help="scope to staged paths")
    parser.add_argument("--paths", nargs="*", default=[], help="scope to these paths")
    parser.add_argument("positional", nargs="*", help="paths (same as --paths)")
    args = parser.parse_args(argv)
    requested = [*args.paths, *args.positional, *(staged() if args.staged else [])]
    if not requested:
        parser.error("name paths with --paths or --staged; omit both for whole-tree recipes")
    paths = in_scope(requested)
    if not paths:
        print(f"{args.action}: no existing files in scope")
        return 0
    steps = fmt(paths) if args.action == "fmt" else turn_end(paths)
    return report(args.action, steps)


if __name__ == "__main__":
    sys.exit(main())
