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
    r"|ruff_.+|pyrefly(_.+)?|tsp_types|blake3"
    # The flow provider's line (ADR-0022, ADR-0012 amendment).
    r"|ty_.+|salsa(-.+)?)$"
)

# Declared extra families (ADR-0002 amendment, 2026-09-24): a second version of a family crate is
# allowed only in the declared scope. Each entry: the crates it covers (a pattern), the one extra
# version, the exact versions its own dependencies must hold, and the only packages that may depend
# on it from outside the family itself.
EXTRA_FAMILIES = [
    {
        "name": "the flow provider (ty_python_core 0.0.14; ADR-0022, ADR-0012 amendment)",
        "crates": re.compile(r"^(ruff_.+|ty_.+)$"),
        "version": "0.0.14",
        # 0.28.3 and 0.28.4 break ruff 0.0.14 in a patch release.
        "pinned": {"salsa": "0.28.2", "salsa-macros": "0.28.2", "salsa-macro-rules": "0.28.2"},
        "dependents": {"cpg-flow"},
    },
]


def _extra(name: str, version: str) -> dict | None:
    for extra in EXTRA_FAMILIES:
        if extra["crates"].match(name) and version == extra["version"]:
            return extra
    return None


def duplicates(lockfile: Path) -> dict[str, list[str]]:
    packages = tomllib.loads(lockfile.read_text())["package"]
    versions: dict[str, set[str]] = defaultdict(set)
    for pkg in packages:
        if FAMILY.match(pkg["name"]) and _extra(pkg["name"], pkg["version"]) is None:
            versions[pkg["name"]].add(pkg["version"])
    return {name: sorted(v) for name, v in versions.items() if len(v) > 1}


def extra_scope(lockfile: Path) -> list[str]:
    """A declared extra family's crates reached only from its declared dependents, and its pins."""
    packages = tomllib.loads(lockfile.read_text())["package"]
    have = defaultdict(set)
    for pkg in packages:
        have[pkg["name"]].add(pkg["version"])
    problems = []
    for extra in EXTRA_FAMILIES:
        for name, version in extra["pinned"].items():
            if name in have and have[name] != {version}:
                found = ", ".join(sorted(have[name]))
                problems.append(
                    f"{name} resolves to {found}; {extra['name']} needs exactly {version}"
                )
        for pkg in packages:
            for dep in pkg.get("dependencies", []):
                parts = dep.split(" ")
                dep_name = parts[0]
                versions = [parts[1]] if len(parts) > 1 else sorted(have[dep_name])
                if not any(_extra(dep_name, v) is extra for v in versions):
                    continue
                inside = _extra(pkg["name"], pkg["version"]) is extra
                if not inside and pkg["name"] not in extra["dependents"]:
                    problems.append(
                        f"{pkg['name']} {pkg['version']} depends on {dep_name} "
                        f"{extra['version']}, outside {extra['name']}"
                    )
    return problems


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
        scope = extra_scope(lock)
        for problem in scope:
            print(f"{lock}: {problem}")
        failed |= bool(scope)
        if not dups:
            print(f"{lock}: family ok")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
