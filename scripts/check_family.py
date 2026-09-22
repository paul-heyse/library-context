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
    r"|buoyant_kernel(_.+)?|delta_kernel(_.+)?|petgraph)$"
)


def duplicates(lockfile: Path) -> dict[str, list[str]]:
    packages = tomllib.loads(lockfile.read_text())["package"]
    versions: dict[str, set[str]] = defaultdict(set)
    for pkg in packages:
        if FAMILY.match(pkg["name"]):
            versions[pkg["name"]].add(pkg["version"])
    return {name: sorted(v) for name, v in versions.items() if len(v) > 1}


def main(argv: list[str]) -> int:
    lockfiles = [Path(a) for a in argv] or [Path("Cargo.lock")]
    failed = False
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
