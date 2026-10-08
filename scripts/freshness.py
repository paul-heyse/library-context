"""Read-only freshness of enumerated generated outputs (plan P6, AE-11; capability review F08).

Standard library only. `just fresh [output…]` launches it as
`uv run --no-project --offline --no-python-downloads python scripts/freshness.py …`.

Every output reports exactly one state, its regenerate command (never invoked) and a reason:

- `clean`: the owning check ran and found no drift;
- `stale`: the owning check found drift; the command exits 1;
- `heuristic`: a hint that neither proves drift nor freshness;
- `not_run`: freshness was not established (a prerequisite is missing, a check could not run,
  or only a test run can decide).

Nothing here regenerates, formats or synchronizes. Uncommitted work is reported as found.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import subprocess
import sys
from collections.abc import Callable, Sequence
from concurrent.futures import ThreadPoolExecutor
from dataclasses import asdict, dataclass, field
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
STATES = ("clean", "stale", "heuristic", "not_run")
GOLD_SKILL = ROOT / ".claude" / "skills" / "fastmcp" / "build" / "manifests"
INSTA_NOT_RUN = "confirmed only by running the owning tests with INSTA_UPDATE=no"


@dataclass
class Freshness:
    output: str
    state: str
    regenerate: str
    reason: str
    files: list[str] = field(default_factory=list)
    details: list[str] = field(default_factory=list)

    def __post_init__(self) -> None:
        assert self.state in STATES, self.state


@dataclass
class Ran:
    returncode: int
    stdout: str
    stderr: str


Runner = Callable[[Sequence[str]], Ran]


def run(argv: Sequence[str]) -> Ran:
    """Run a read-only check from the checkout root; a missing tool is exit 127."""
    env = dict(os.environ)
    env.update(GIT_OPTIONAL_LOCKS="0", CARGO_TERM_COLOR="never", NO_COLOR="1")
    try:
        done = subprocess.run(
            list(argv),
            cwd=ROOT,
            env=env,
            stdin=subprocess.DEVNULL,
            capture_output=True,
            text=True,
            check=False,
        )
    except FileNotFoundError as error:
        return Ran(127, "", str(error))
    return Ran(done.returncode, done.stdout, done.stderr)


def _failed(output: str, regenerate: str, argv: Sequence[str], ran: Ran) -> Freshness:
    tail = (ran.stderr or ran.stdout).strip().splitlines()[-3:]
    return Freshness(
        output,
        "not_run",
        regenerate,
        f"`{' '.join(argv)}` could not decide (exit {ran.returncode})",
        details=tail,
    )


def _relative(path: str, root: Path) -> str:
    try:
        return str(Path(path).resolve().relative_to(root))
    except ValueError:
        return path


# ---------------------------------------------------------------------------------------------
# Outputs


def hakari(runner: Runner, root: Path = ROOT) -> Freshness:
    regenerate = "just build-features"
    generate = ("cargo", "hakari", "generate", "--diff")
    manage = ("cargo", "hakari", "manage-deps", "--dry-run")
    lock = root / "Cargo.lock"
    before = hashlib.sha256(lock.read_bytes()).hexdigest() if lock.exists() else None
    diff, deps = runner(generate), runner(manage)
    details = []
    if lock.exists() and hashlib.sha256(lock.read_bytes()).hexdigest() != before:
        details.append("cargo metadata rewrote Cargo.lock during the check")
    for argv, ran in ((generate, diff), (manage, deps)):
        if ran.returncode not in (0, 1):
            return _failed("hakari", regenerate, argv, ran)
    if diff.returncode == 0 and deps.returncode == 0:
        return Freshness("hakari", "clean", regenerate, "workspace-hack matches", details=details)
    reasons = []
    if diff.returncode == 1:
        changed = [
            line
            for line in (diff.stdout + diff.stderr).splitlines()
            if line[:1] in "+-" and not line.startswith(("+++", "---")) and "=" in line
        ]
        reasons.append(f"workspace-hack Cargo.toml differs ({len(changed)} dependency lines)")
        details += changed
    if deps.returncode == 1:
        reasons.append("workspace crates' workspace-hack dependencies need changes")
        details += [line for line in (deps.stdout + deps.stderr).splitlines() if line.strip()]
    return Freshness(
        "hakari",
        "stale",
        regenerate,
        "; ".join(reasons),
        files=["crates/lctx-workspace-hack/Cargo.toml"] if diff.returncode == 1 else [],
        details=details,
    )


def adr_index(runner: Runner, root: Path = ROOT) -> Freshness:
    regenerate = "just adr-index"
    try:
        sys.path.insert(0, str(root / "scripts"))
        import adr

        expected = adr.render_index(root, adr.load_all(root))
    except Exception as error:  # an unreadable record set cannot be judged here
        return Freshness("adr-index", "not_run", regenerate, f"could not render: {error}")
    index = adr.adr_dir(root) / "README.md"
    if index.exists() and index.read_text() == expected:
        return Freshness("adr-index", "clean", regenerate, "docs/adr/README.md matches")
    return Freshness(
        "adr-index",
        "stale",
        regenerate,
        "docs/adr/README.md differs from the records",
        files=["docs/adr/README.md"],
    )


def skills(runner: Runner, root: Path = ROOT) -> Freshness:
    regenerate = "just skills-sync"
    argv = (sys.executable, str(root / "scripts" / "library_skills.py"), "--check")
    ran = runner(argv)
    text = (ran.stdout + ran.stderr).strip()
    if "are not installed" in text:
        return Freshness("skills", "not_run", regenerate, "the shared skill store is absent")
    if ran.returncode == 0:
        return Freshness("skills", "clean", regenerate, "selected links match")
    if ran.returncode == 1:
        return Freshness(
            "skills", "stale", regenerate, "skill links differ", details=text.splitlines()
        )
    return _failed("skills", regenerate, argv, ran)


def rust_fmt(runner: Runner, root: Path = ROOT) -> Freshness:
    regenerate = "just fmt"
    argv = ("cargo", "fmt", "--check", "--message-format", "short")
    ran = runner(argv)
    if ran.returncode == 0:
        return Freshness("rust-fmt", "clean", regenerate, "cargo fmt --check passes")
    files = sorted({_relative(line.strip(), root) for line in ran.stdout.splitlines() if line})
    if ran.returncode == 1 and files:
        return Freshness(
            "rust-fmt", "stale", regenerate, f"{len(files)} file(s) unformatted", files=files
        )
    return _failed("rust-fmt", regenerate, argv, ran)


def python_fmt(runner: Runner, root: Path = ROOT) -> Freshness:
    regenerate = "just fmt"
    argv = ("uv", "run", "--no-sync", "ruff", "format", "--check", "--output-format", "json")
    ran = runner(argv)
    if ran.returncode == 0:
        return Freshness("python-fmt", "clean", regenerate, "ruff format --check passes")
    try:
        found = json.loads(ran.stdout)
    except ValueError:
        found = None
    if ran.returncode == 1 and isinstance(found, list) and found:
        files = sorted({_relative(item["filename"], root) for item in found})
        return Freshness(
            "python-fmt", "stale", regenerate, f"{len(files)} file(s) unformatted", files=files
        )
    return _failed("python-fmt", regenerate, argv, ran)


def gold(runner: Runner, root: Path = ROOT, skill: Path = GOLD_SKILL) -> Freshness:
    regenerate = (
        "uv run --no-project --offline --no-python-downloads python scripts/gold_extract.py "
        "for the extract; a pin mismatch needs libraries/fastmcp and the fastmcp skill aligned"
    )
    if not (skill / "fastmcp.json").exists():
        return Freshness("gold", "not_run", regenerate, "the fastmcp skill is not installed")
    argv = (sys.executable, str(root / "scripts" / "check_gold.py"))
    ran = runner(argv)
    lines = [line for line in ran.stdout.splitlines() if line.strip()]
    if "not_run" in ran.stdout:
        return Freshness("gold", "not_run", regenerate, "check_gold.py reported not_run", lines)
    if ran.returncode == 0:
        return Freshness("gold", "clean", regenerate, "gold and libraries/fastmcp agree")
    if ran.returncode == 1:
        return Freshness("gold", "stale", regenerate, "check_gold.py found drift", details=lines)
    return _failed("gold", regenerate, argv, ran)


def _snapshot_source(snapshot: Path) -> str | None:
    try:
        with snapshot.open(encoding="utf-8") as handle:
            if handle.readline().strip() != "---":
                return None
            for line in handle:
                if line.strip() == "---":
                    return None
                if line.startswith("source:"):
                    return line.split(":", 1)[1].strip().strip('"')
    except OSError:
        return None
    return None


def _crate_dir(path: Path, root: Path) -> Path:
    for parent in path.parents:
        if (parent / "Cargo.toml").exists() or parent == root:
            return parent
    return root


def insta(runner: Runner, root: Path = ROOT) -> list[Freshness]:
    """Snapshots: `not_run` (only tests decide) plus a heuristic orphaned-source hint."""
    regenerate = (
        "run the owning tests with INSTA_UPDATE=no; read each .snap.new, then `cargo insta accept`"
    )
    argv = ("git", "ls-files", "-co", "--exclude-standard", "--", "*.snap", "*.snap.new")
    ran = runner(argv)
    if ran.returncode != 0:
        return [_failed("insta", regenerate, argv, ran)]
    paths = [line for line in ran.stdout.splitlines() if line]
    snaps = [p for p in paths if p.endswith(".snap")]
    pending = sorted(p for p in paths if p.endswith(".snap.new"))
    orphans = []
    for relative in snaps:
        source = _snapshot_source(root / relative)
        if source is None:
            continue
        candidates = (root / source, _crate_dir(root / relative, root) / source)
        if not any(candidate.exists() for candidate in candidates):
            orphans.append(f"{relative} (source: {source})")
    pending_note = f"; {len(pending)} pending .snap.new" if pending else ""
    results = [
        Freshness(
            "insta",
            "not_run",
            regenerate,
            f"{len(snaps)} snapshot(s), {INSTA_NOT_RUN}{pending_note}",
            files=pending,
        )
    ]
    reason = (
        f"{len(orphans)} snapshot(s) name a source file that no longer exists"
        if orphans
        else "every snapshot's source file exists (this does not show the content is current)"
    )
    results.append(
        Freshness(
            "insta-sources",
            "heuristic",
            "delete an orphaned snapshot, or rerun its moved test",
            reason,
            files=orphans,
        )
    )
    return results


CHECKS: dict[str, Callable[..., Freshness | list[Freshness]]] = {
    "hakari": hakari,
    "adr-index": adr_index,
    "skills": skills,
    "rust-fmt": rust_fmt,
    "python-fmt": python_fmt,
    "gold": gold,
    "insta": insta,
}
ALIASES = {"fmt": ("rust-fmt", "python-fmt"), "insta-sources": ("insta",)}


def select(names: Sequence[str]) -> list[str]:
    if not names:
        return list(CHECKS)
    chosen: list[str] = []
    for name in names:
        expanded = ALIASES.get(name, (name,))
        for item in expanded:
            if item not in CHECKS:
                raise KeyError(name)
            if item not in chosen:
                chosen.append(item)
    return chosen


def check(names: Sequence[str], runner: Runner = run) -> list[Freshness]:
    chosen = select(names)
    with ThreadPoolExecutor(max_workers=len(chosen)) as pool:
        futures = [pool.submit(CHECKS[name], runner) for name in chosen]
        results: list[Freshness] = []
        for future in futures:
            outcome = future.result()
            results.extend(outcome if isinstance(outcome, list) else [outcome])
    return results


def render(results: Sequence[Freshness], *, limit: int = 20) -> str:
    lines = []
    for result in results:
        lines.append(f"{result.output:<14}{result.state:<10}{result.reason}")
        shown = result.files[:limit]
        lines += [f"{'':<24}{path}" for path in shown]
        if len(result.files) > limit:
            lines.append(f"{'':<24}… {len(result.files) - limit} more (--all or --json)")
        if result.state != "clean":
            details = result.details[:5]
            lines += [f"{'':<24}| {detail}" for detail in details]
            if len(result.details) > 5:
                lines.append(f"{'':<24}| … {len(result.details) - 5} more (--json)")
            lines.append(f"{'':<24}regenerate: {result.regenerate}")
    stale = [r.output for r in results if r.state == "stale"]
    lines.append(f"fresh: {len(stale)} stale" + (f" ({', '.join(stale)})" if stale else ""))
    return "\n".join(lines)


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(prog="fresh", description=__doc__.split("\n\n")[0])
    parser.add_argument("outputs", nargs="*", help=f"any of {', '.join([*CHECKS, *ALIASES])}")
    parser.add_argument("--json", action="store_true")
    parser.add_argument("--all", action="store_true", help="list every file")
    args = parser.parse_args(argv)
    try:
        results = check(args.outputs)
    except KeyError as error:
        print(f"fresh: unknown output {error}; choose from {', '.join(CHECKS)}", file=sys.stderr)
        return 2
    if args.json:
        print(json.dumps([asdict(r) for r in results], indent=2))
    else:
        print(render(results, limit=10**9 if args.all else 20))
    return 1 if any(r.state == "stale" for r in results) else 0


if __name__ == "__main__":
    sys.exit(main())
