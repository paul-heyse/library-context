"""The compiled dependency graph, from nightly cargo's unit graph.

`cargo build --unit-graph -Zunstable-options` prints every unit cargo would build (library,
build script, test, bench, example; every profile) with the features it resolved and the units it
depends on. It computes from Cargo.lock and the manifests: no build, no target directory, about
0.3 s here, and it works on a tree that does not compile. That makes it the source for two facts a
manifest-only read gets wrong once a workspace-hack crate unifies features: the features a package
is *compiled* with, and which workspace crates actually link it.

Ids look like `registry+https://...#name@1.2.3`, `git+https://...?rev=..#name@1.2.3` and
`path+file:///.../crates/x#0.1.0` (the last carries no name; it is the directory).
"""

from __future__ import annotations

import json
import re
import subprocess
from collections import defaultdict
from dataclasses import dataclass
from pathlib import Path

import library_scan as scan

COMMAND = [
    "cargo",
    "build",
    "--unit-graph",
    "-Zunstable-options",
    "--locked",
    "--offline",
    "--workspace",
    "--all-targets",
]
TIMEOUT = 30  # seconds; cargo can wait on the package-cache lock while another cargo runs

_NAMED = re.compile(r"#([^@#]+)@(.+)$")
_PATH = re.compile(r"^path\+file://([^#]+)(?:#(.+))?$")


@dataclass(frozen=True)
class Summary:
    """What the compiled graph says about one (package, version)."""

    features: tuple[str, ...]  # union over every unit of that package
    linked_by: tuple[str, ...]  # workspace crates with a unit that depends on it


def parse_package(pkg_id: str) -> tuple[str, str]:
    """(name, version) from a unit-graph package id; a path id without a name is its directory."""
    named = _NAMED.search(pkg_id)
    if named:
        return named.group(1), named.group(2)
    path = _PATH.match(pkg_id)
    if path:
        return Path(path.group(1)).name, path.group(2) or ""
    return pkg_id, ""


def load(root: Path, timeout: int = TIMEOUT) -> tuple[dict | None, str]:
    """The unit graph, or (None, why not): a timeout, a failed command, unreadable output."""
    try:
        proc = subprocess.run(
            COMMAND, cwd=root, capture_output=True, text=True, timeout=timeout, check=False
        )
    except subprocess.TimeoutExpired:
        return None, f"cargo build --unit-graph did not finish within {timeout} s"
    except FileNotFoundError:
        return None, "cargo not found"
    if proc.returncode != 0:
        tail = proc.stderr.strip().splitlines()[-1:] or ["no message"]
        return None, f"cargo build --unit-graph failed: {tail[0][:200]}"
    try:
        return json.loads(proc.stdout), ""
    except json.JSONDecodeError:
        return None, "cargo build --unit-graph printed output that is not JSON"


def members_of(graph: dict, root: Path) -> set[str]:
    """Workspace package names as the graph shows them: path packages under `root`, not vendored.

    Cargo metadata lists members directly; this reads them from the graph itself so a caller that
    has only the graph (a server re-reading it) can classify units the same way.
    """
    prefix = f"path+file://{root.resolve()}/"
    found: set[str] = set()
    for unit in graph["units"]:
        pkg_id = unit["pkg_id"]
        if pkg_id.startswith(prefix) and not pkg_id[len(prefix) :].startswith("third_party/"):
            found.add(parse_package(pkg_id)[0])
    return found


def summarize(graph: dict, members: set[str]) -> dict[tuple[str, str], Summary]:
    """Per external (package, version): compiled features and the workspace crates linking it.

    `members` are the workspace package names (from cargo metadata): a path dependency that is not
    a member, such as a vendored crate, is external. Features are the union over every unit of that
    package (library, build script, test...). `linked_by` counts direct dependency edges from
    workspace units, except the hakari stub's.
    """
    units = graph["units"]
    package = [parse_package(u["pkg_id"]) for u in units]
    features: dict[tuple[str, str], set[str]] = {}
    linked: dict[tuple[str, str], set[str]] = defaultdict(set)
    for unit, (name, version) in zip(units, package, strict=True):
        if name in members:
            if scan.is_hack(name):
                continue
            for edge in unit.get("dependencies", []):
                target = package[edge["index"]]
                if target[0] not in members:
                    linked[target].add(name)
        else:
            features.setdefault((name, version), set()).update(unit.get("features", []))
    return {
        key: Summary(tuple(sorted(found)), tuple(sorted(linked.get(key, ()))))
        for key, found in features.items()
    }
