"""Fail if a pinned-family crate resolves to more than one version in a lockfile.

The pinned family is DESIGN §7 / ADR-0002. Two Arrows (or DataFusions) in one
dependency graph compile, but their types are incompatible at every boundary,
which is the failure Initial_plan §7.4 warns about. This reads Cargo.lock
directly so dev-dependencies count too.

Usage: check_family.py [LOCKFILE ...]   (default: ./Cargo.lock)
"""

from __future__ import annotations

import re
import sys
import tomllib
from collections import defaultdict
from pathlib import Path

FAMILY = re.compile(
    r"^(arrow(-.+)?|parquet|datafusion(-.+)?|object_store|deltalake(-.+)?"
    r"|buoyant_kernel(_.+)?|delta_kernel(_.+)?|petgraph"
    # Analyzers (ADR-0012): two ruff versions would split the AST types Pyrefly shares.
    r"|ruff_.+|pyrefly(_.+)?|tsp_types|blake3)$"
)


def duplicates(lockfile: Path) -> dict[str, list[str]]:
    packages = tomllib.loads(lockfile.read_text())["package"]
    versions: dict[str, set[str]] = defaultdict(set)
    for pkg in packages:
        if FAMILY.match(pkg["name"]):
            versions[pkg["name"]].add(pkg["version"])
    return {name: sorted(v) for name, v in versions.items() if len(v) > 1}


ROOT = Path(__file__).resolve().parent.parent


def unpinned(manifest: Path, pins: Path) -> list[str]:
    """Workspace dependencies without an exact pin (`=x.y.z` or a git `rev`) or a pins.md row.

    A range pin lets `cargo update` move a dependency without a reviewed diff (H1 review F8). A row
    names the crate anywhere in `pins.md`, or matches a first-column glob such as `arrow-*`.
    """
    deps = tomllib.loads(manifest.read_text())["workspace"]["dependencies"]
    text = pins.read_text()
    globs = [
        cell.strip(" `")[:-1]
        for line in text.splitlines()
        if line.startswith("| ")
        for cell in line.split("|")[1].split(",")
        if cell.strip(" `").endswith("*")
    ]
    problems = []
    for name, spec in sorted(deps.items()):
        version = spec if isinstance(spec, str) else spec.get("version")
        exact = (version or "").startswith("=") or (isinstance(spec, dict) and "rev" in spec)
        if not exact:
            problems.append(f"{name}: not pinned exactly ({version!r})")
        if name not in text and not any(name.startswith(g) for g in globs):
            problems.append(f"{name}: no docs/pins.md row")
    return problems


def main(argv: list[str]) -> int:
    lockfiles = [Path(a) for a in argv] or [Path("Cargo.lock")]
    failed = False
    for problem in unpinned(ROOT / "Cargo.toml", ROOT / "docs" / "pins.md"):
        print(f"Cargo.toml: {problem}")
        failed = True
    for lock in lockfiles:
        if not lock.exists():
            print(f"{lock}: not_run (no lockfile)")
            continue
        dups = duplicates(lock)
        for name, versions in sorted(dups.items()):
            print(f"{lock}: {name} resolves to {', '.join(versions)}")
        failed |= bool(dups)
        if not dups:
            print(f"{lock}: family ok")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
