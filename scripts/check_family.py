"""Fail if a pinned-family crate resolves to more than one version in a lockfile.

The pinned family is DESIGN §7 / ADR-0118. Two Arrows (or DataFusions) in one
dependency graph compile, but their types are incompatible at every boundary,
which is the failure Initial_plan §7.4 warns about. This reads Cargo.lock
directly so dev-dependencies count too.

Usage: check_family.py [LOCKFILE ...]   (default: ./Cargo.lock)
"""

from __future__ import annotations

import json
import re
import subprocess
import sys
import tomllib
from collections import defaultdict
from itertools import pairwise
from pathlib import Path
from typing import NotRequired, TypedDict, cast


class ExtraFamily(TypedDict):
    name: str
    crates: re.Pattern[str]
    version: str
    versions: dict[str, str]
    source: str
    pinned: dict[str, str]
    dependents: set[str]


class LockPackage(TypedDict):
    name: str
    version: str
    source: NotRequired[str]
    dependencies: NotRequired[list[str]]


class MetadataPackage(TypedDict):
    id: str
    name: str
    version: str
    source: str | None


class DependencyKind(TypedDict):
    kind: str | None


class MetadataDependency(TypedDict):
    pkg: str
    dep_kinds: list[DependencyKind]


class MetadataNode(TypedDict):
    id: str
    deps: list[MetadataDependency]


class MetadataResolve(TypedDict):
    nodes: list[MetadataNode]


class CargoMetadata(TypedDict):
    packages: list[MetadataPackage]
    workspace_members: list[str]
    resolve: MetadataResolve


type PackageIdentity = tuple[str, str, str | None]

FAMILY = re.compile(
    r"^(arrow(-.+)?|parquet|datafusion(-.+)?|object_store|petgraph"
    # Nominal analyzer families are declared below; no native AST crosses their boundary.
    r"|ruff_.+|pyrefly(_.+)?|tsp_types|blake3"
    # The flow provider and its exact salsa family.
    r"|ty_.+|salsa(-.+)?)$"
)

# ADR-0118: exact nominal sources, not just equal crate version strings.
REGISTRY = "registry+https://github.com/rust-lang/crates.io-index"
RUFF_REVISION = "f7bdff69e1fb94ab0ed5b340e977aac0d26e9301"
RUFF_SOURCE = f"git+https://github.com/paul-heyse/ruff?rev={RUFF_REVISION}#{RUFF_REVISION}"
EXTRA_FAMILIES: list[ExtraFamily] = [
    {
        "name": "latest independent Ruff/ty",
        "crates": re.compile(r"^(ruff_.+|ty_.+)$"),
        "version": "0.0.16",
        "versions": {"ruff_linter": "0.16.10", "ty_ide": "0.0.0", "ty_project": "0.0.0"},
        "source": RUFF_SOURCE,
        "pinned": {
            "salsa": "0.28.5",
            "salsa-macros": "0.28.5",
            "salsa-macro-rules": "0.28.5",
        },
        # Hakari unifies build features only; its stub exports no analyzer types.
        "dependents": {"cpg-extract", "cpg-flow", "lctx-workspace-hack"},
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


def _extra(name: str, version: str, source: str | None) -> ExtraFamily | None:
    for extra in EXTRA_FAMILIES:
        expected = extra["versions"].get(name, extra["version"])
        if extra["crates"].match(name) and version == expected and source == extra["source"]:
            return extra
    return None


def _packages(lockfile: Path) -> list[LockPackage]:
    return cast(list[LockPackage], tomllib.loads(lockfile.read_text())["package"])


def duplicates(
    lockfile: Path, *, development: frozenset[PackageIdentity] = frozenset()
) -> dict[str, list[str]]:
    packages = _packages(lockfile)
    identities: dict[str, set[tuple[str, str | None]]] = defaultdict(set)
    for pkg in packages:
        if (
            FAMILY.match(pkg["name"])
            and _extra(pkg["name"], pkg["version"], pkg.get("source")) is None
            and (pkg["name"], pkg["version"], pkg.get("source")) not in development
        ):
            identities[pkg["name"]].add((pkg["version"], pkg.get("source")))
    result = {}
    for name, values in identities.items():
        if len(values) > 1:
            versions = {v for v, _ in values}
            result[name] = (
                sorted(versions)
                if len(versions) == len(values)
                else sorted(f"{v} ({source or 'workspace'})" for v, source in values)
            )
    return result


def _dependency(dep: str, packages: list[LockPackage]) -> list[LockPackage]:
    parts = dep.split(" ", 2)
    name = parts[0]
    version = parts[1] if len(parts) > 1 else None
    source = parts[2].removeprefix("(").removesuffix(")") if len(parts) > 2 else None
    return [
        p
        for p in packages
        if p["name"] == name
        and (version is None or p["version"] == version)
        and (source is None or p.get("source") == source)
    ]


def extra_scope(lockfile: Path) -> list[str]:
    """Check source identity, unique dependency resolution, family edges and salsa pins."""
    packages = _packages(lockfile)
    problems = []
    have = defaultdict(set)
    for pkg in packages:
        have[pkg["name"]].add(pkg["version"])
        if (
            re.match(r"^(ruff_.+|ty_.+)$", pkg["name"])
            and _extra(pkg["name"], pkg["version"], pkg.get("source")) is None
        ):
            problems.append(
                f"{pkg['name']} {pkg['version']} has undeclared analyzer source "
                f"{pkg.get('source')!r}"
            )
    for name, version in EXTRA_FAMILIES[0]["pinned"].items():
        if name in have and have[name] != {version}:
            problems.append(
                f"{name} resolves to {', '.join(sorted(have[name]))}; "
                f"latest independent Ruff/ty needs exactly {version}"
            )
    for pkg in packages:
        parent = _extra(pkg["name"], pkg["version"], pkg.get("source"))
        for dep in pkg.get("dependencies", []):
            candidates = _dependency(dep, packages)
            if len(candidates) != 1:
                problems.append(
                    f"{pkg['name']} {pkg['version']} has "
                    f"{'ambiguous' if candidates else 'unresolved'} dependency {dep}"
                )
                continue
            child = candidates[0]
            family = _extra(child["name"], child["version"], child.get("source"))
            if family is None:
                continue
            pyrefly = family["name"] == "Pyrefly embedded Ruff" and re.fullmatch(
                r"pyrefly(_.+)?", pkg["name"]
            )
            if parent is not family and pkg["name"] not in family["dependents"] and not pyrefly:
                problems.append(
                    f"{pkg['name']} {pkg['version']} depends on {dep}, outside {family['name']}"
                )
    return problems


# ADR-0121's independent oracle never exchanges graph objects with production.
# A lockfile alone cannot establish that boundary: every exemption needs a fresh
# all-features Cargo graph proving this exact registry chain and dev-only ingress.
ORACLE_CHAIN: tuple[PackageIdentity, ...] = (
    ("odis", "2026.9.1", REGISTRY),
    ("rust-sugiyama", "0.3.0", REGISTRY),
    ("petgraph", "0.6.5", REGISTRY),
)


def oracle_scope(
    packages: list[LockPackage], metadata: CargoMetadata | None
) -> tuple[frozenset[PackageIdentity], list[str]]:
    """Exempt only the proven development oracle's exact old graph package."""
    if not any(p["name"] == "petgraph" and p["version"] == "0.6.5" for p in packages):
        return frozenset(), []
    prefix = "petgraph 0.6.5 development oracle"
    if metadata is None:
        return frozenset(), [f"{prefix} lacks matching Cargo metadata proof"]
    locked = {(p["name"], p["version"], p.get("source")) for p in packages}
    graph_packages = {p["id"]: p for p in metadata["packages"]}
    nodes = {node["id"]: node for node in metadata["resolve"]["nodes"]}
    if any(
        (p["name"], p["version"], p["source"]) not in locked
        for p in graph_packages.values()
    ) or any(
        node["id"] not in graph_packages
        or any(edge["pkg"] not in nodes or not edge["dep_kinds"] for edge in node["deps"])
        for node in nodes.values()
    ):
        return frozenset(), [f"{prefix} has incomplete or mismatched lock/metadata proof"]
    chain = []
    for identity in ORACLE_CHAIN:
        matches = [
            p["id"]
            for p in graph_packages.values()
            if (p["name"], p["version"], p["source"]) == identity
        ]
        if identity not in locked or len(matches) != 1 or matches[0] not in nodes:
            return frozenset(), [f"{prefix} has wrong source/version or missing {identity[0]}"]
        chain.append(matches[0])
    for parent, child in pairwise(ORACLE_CHAIN):
        parent_row = next(
            p for p in packages if (p["name"], p["version"], p.get("source")) == parent
        )
        if not any(
            len(matches := _dependency(dep, packages)) == 1
            and (matches[0]["name"], matches[0]["version"], matches[0].get("source")) == child
            for dep in parent_row.get("dependencies", [])
        ):
            return frozenset(), [f"{prefix} lacks its matching pinned lockfile path"]
    workspace = set(metadata["workspace_members"])
    if not workspace or any(root not in nodes or root not in graph_packages for root in workspace):
        return frozenset(), [f"{prefix} has incomplete workspace graph proof"]
    production: set[str] = set()
    pending = list(workspace)
    while pending:
        current = pending.pop()
        if current in production:
            continue
        if current not in nodes:
            return frozenset(), [f"{prefix} has incomplete dependency graph proof"]
        production.add(current)
        pending.extend(
            edge["pkg"]
            for edge in nodes[current]["deps"]
            if any(kind["kind"] != "dev" for kind in edge["dep_kinds"])
        )
    if any(node in production for node in chain):
        return frozenset(), [f"{prefix} enters a workspace normal/build dependency closure"]
    for index, target in enumerate(chain):
        incoming = [
            (node["id"], edge)
            for node in nodes.values()
            for edge in node["deps"]
            if edge["pkg"] == target
        ]
        if len(incoming) != 1:
            return frozenset(), [f"{prefix} lacks its sole pinned oracle dependency path"]
        for parent, edge in incoming:
            kinds = {kind["kind"] for kind in edge["dep_kinds"]}
            if index == 0:
                allowed = (
                    parent in workspace
                    and graph_packages[parent]["name"] == "lctx-analytics"
                    and kinds == {"dev"}
                )
            else:
                allowed = parent == chain[index - 1] and kinds == {None}
            if not allowed:
                return frozenset(), [f"{prefix} has an edge outside its sole dev-only oracle path"]
    return frozenset({ORACLE_CHAIN[-1]}), []


def _metadata(lockfile: Path) -> CargoMetadata | None:
    manifest = lockfile.resolve().with_name("Cargo.toml")
    if lockfile.name != "Cargo.lock" or not manifest.is_file():
        return None
    try:
        result = subprocess.run(
            [
                "cargo", "metadata", "--locked", "--offline", "--all-features",
                "--format-version", "1", "--manifest-path", str(manifest),
            ],
            check=True,
            capture_output=True,
            text=True,
        )
        return cast(CargoMetadata, json.loads(result.stdout))
    except (OSError, subprocess.CalledProcessError, json.JSONDecodeError):
        return None


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
        packages = _packages(lock)
        needs_proof = any(p["name"] == "petgraph" and p["version"] == "0.6.5" for p in packages)
        development, oracle_problems = oracle_scope(
            packages, _metadata(lock) if needs_proof else None
        )
        dups = duplicates(lock, development=development)
        for name, versions in sorted(dups.items()):
            print(f"{lock}: {name} resolves to {', '.join(versions)}")
        failed |= bool(dups)
        scope = extra_scope(lock) + oracle_problems
        for problem in scope:
            print(f"{lock}: {problem}")
        failed |= bool(scope)
        if not dups:
            print(f"{lock}: family ok")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
