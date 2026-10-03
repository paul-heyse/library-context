"""Fail if a pinned-family crate resolves to more than one version in a lockfile.

The pinned family is DESIGN §7 / ADR-0118. Two Arrows (or DataFusions) in one
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
    r"^(arrow(-.+)?|parquet|datafusion(-.+)?|object_store|petgraph"
    # Nominal analyzer families are declared below; no native AST crosses their boundary.
    r"|ruff_.+|pyrefly(_.+)?|tsp_types|blake3"
    # The flow provider and its exact salsa family.
    r"|ty_.+|salsa(-.+)?)$"
)

# ADR-0118: exact nominal sources, not just equal crate version strings.
REGISTRY = "registry+https://github.com/rust-lang/crates.io-index"
RUFF_REVISION = "8f01d80020921d3867f255ee5f919dd2d329b730"
RUFF_SOURCE = f"git+https://github.com/paul-heyse/ruff?rev={RUFF_REVISION}#{RUFF_REVISION}"
EXTRA_FAMILIES = [
    {
        "name": "latest independent Ruff/ty",
        "crates": re.compile(r"^(ruff_.+|ty_.+)$"),
        "version": "0.0.16",
        "versions": {"ruff_linter": "0.16.10"},
        "source": RUFF_SOURCE,
        "pinned": {"salsa": "0.28.5", "salsa-macros": "0.28.5", "salsa-macro-rules": "0.28.5"},
        "dependents": {"cpg-extract", "cpg-flow"},
    },
    {
        "name": "Pyrefly embedded Ruff",
        "crates": re.compile(r"^ruff_.+$"),
        "version": "0.0.14",
        "versions": {},
        "source": REGISTRY,
        "pinned": {},
        "dependents": {"cpg-extract"},
    },
]


def _extra(name: str, version: str, source: str | None) -> dict | None:
    for extra in EXTRA_FAMILIES:
        expected = extra["versions"].get(name, extra["version"])
        if extra["crates"].match(name) and version == expected and source == extra["source"]:
            return extra
    return None


def duplicates(lockfile: Path) -> dict[str, list[str]]:
    packages = tomllib.loads(lockfile.read_text())["package"]
    identities: dict[str, set[tuple[str, str | None]]] = defaultdict(set)
    for pkg in packages:
        if FAMILY.match(pkg["name"]) and _extra(pkg["name"], pkg["version"], pkg.get("source")) is None:
            identities[pkg["name"]].add((pkg["version"], pkg.get("source")))
    result = {}
    for name, values in identities.items():
        if len(values) > 1:
            versions = {v for v, _ in values}
            result[name] = sorted(versions) if len(versions) == len(values) else sorted(f"{v} ({source or 'workspace'})" for v, source in values)
    return result


def _dependency(dep: str, packages: list[dict]) -> list[dict]:
    parts = dep.split(" ", 2)
    name = parts[0]
    version = parts[1] if len(parts) > 1 else None
    source = parts[2].removeprefix("(").removesuffix(")") if len(parts) > 2 else None
    return [p for p in packages if p["name"] == name and (version is None or p["version"] == version) and (source is None or p.get("source") == source)]


def extra_scope(lockfile: Path) -> list[str]:
    """Check source identity, unique dependency resolution, family edges and salsa pins."""
    packages = tomllib.loads(lockfile.read_text())["package"]
    problems = []
    have = defaultdict(set)
    for pkg in packages:
        have[pkg["name"]].add(pkg["version"])
        if re.match(r"^(ruff_.+|ty_.+)$", pkg["name"]) and _extra(pkg["name"], pkg["version"], pkg.get("source")) is None:
            problems.append(f"{pkg['name']} {pkg['version']} has undeclared analyzer source {pkg.get('source')!r}")
    for name, version in EXTRA_FAMILIES[0]["pinned"].items():
        if name in have and have[name] != {version}:
            problems.append(f"{name} resolves to {', '.join(sorted(have[name]))}; latest independent Ruff/ty needs exactly {version}")
    for pkg in packages:
        parent = _extra(pkg["name"], pkg["version"], pkg.get("source"))
        for dep in pkg.get("dependencies", []):
            candidates = _dependency(dep, packages)
            if len(candidates) != 1:
                problems.append(f"{pkg['name']} {pkg['version']} has {'ambiguous' if candidates else 'unresolved'} dependency {dep}")
                continue
            child = candidates[0]
            family = _extra(child["name"], child["version"], child.get("source"))
            if family is None:
                continue
            pyrefly = family["name"] == "Pyrefly embedded Ruff" and re.fullmatch(r"pyrefly(_.+)?", pkg["name"])
            if parent is not family and pkg["name"] not in family["dependents"] and not pyrefly:
                problems.append(f"{pkg['name']} {pkg['version']} depends on {dep}, outside {family['name']}")
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
