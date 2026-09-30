"""Regenerate the mechanical fields of docs/library-utilization.jsonl.

Stages S0 (manifests), S1 (source scan) and S3 (canonical names) of the plan in
docs/library-utilization.md. Nothing here needs a build. Sources: `cargo metadata --no-deps`,
Cargo.lock, each enabled skill's `build/manifests` file, the workspace's `.rs` text
(scripts/library_scan.py) and the skills' `content/index` symbol, alias and method tables
(scripts/library_names.py).

Generated per `library` record: pin, default features, enabled features, dependents (with
dev/build kind), covering skill, skill pin delta, `status` (used, test-only, dormant-only,
declared-unused from the source scan; not-used when there is no direct dependency) and `used_in`
(files per package by role); plus a `not-used` record for each skill-covered library with no
direct dependency. Generated per `capability` record: `items` rewritten to defining paths (the
spellings replaced go to `as_written`). Hand-written fields (wrappers, notes, features, files,
purposes) pass through untouched, and a record gets a new `verified` stamp only when a generated
field changed. Advisory sections report capability files that are missing or no longer reference
their library, items the skills' index does not list, and where the source scan and an
independent ripgrep count disagree.

Usage: library_utilization.py [--write] [--root DIR] [--jsonl FILE] [--metadata FILE]
Without --write it is a dry run: it prints the drift and exits 1 when there is any.
"""

from __future__ import annotations

import argparse
import datetime
import hashlib
import json
import os
import re
import sqlite3
import subprocess
import sys
import tempfile
import tomllib
from collections import defaultdict
from collections.abc import Callable
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

import library_catalog_db as catalog_db
import library_names as names
import library_scan as scan
import library_semantic as semantic
import library_unitgraph as unitgraph

JSONL = Path("docs/library-utilization.jsonl")
CAPABILITY_KEYS = [
    "kind",
    "id",
    "lib",
    "feature",
    "items",
    "as_written",
    "use",
    "files",
    "n_files",
    "wrapper",
    "status",
    "verified",
]
LIBRARY_KEYS = [
    "kind",
    "lib",
    "pin",
    "features",
    "resolved_features",
    "default_features",
    "skill",
    "skill_pin_delta",
    "dependents",
    "linked_by",
    "used_in",
    "crates",
    "status",
    "wrappers",
    "note",
    "adr",
    "verified",
]
# The fields this script owns; a change in any of them restamps `verified`.
GENERATED = (
    "pin",
    "features",
    "resolved_features",
    "default_features",
    "skill",
    "skill_pin_delta",
    "dependents",
    "linked_by",
    "used_in",
    "crates",
    "status",
)


def norm(name: str) -> str:
    return name.replace("_", "-").lower()


@dataclass
class Dep:
    """One direct dependency, keyed by the name code uses (the rename, if any)."""

    key: str
    package: str
    reqs: set[str] = field(default_factory=set)
    sources: set[str] = field(default_factory=set)
    features: set[str] = field(default_factory=set)
    default_flags: set[bool] = field(default_factory=set)
    dependents: set[str] = field(default_factory=set)


@dataclass
class Group:
    """A crate set or library a skill indexes: its crates and the version it indexes."""

    skill: str
    name: str
    label: str
    crates: dict[str, str | None]
    family: bool = False  # a per-library skill manifest: its crates are that library's own family


def load_metadata(root: Path, metadata_file: Path | None) -> dict:
    if metadata_file is not None:
        return json.loads(metadata_file.read_text())
    proc = subprocess.run(
        ["cargo", "metadata", "--format-version", "1", "--no-deps", "--offline"],
        cwd=root,
        capture_output=True,
        text=True,
        check=False,
    )
    if proc.returncode != 0:
        raise scan.SetupError(f"cargo metadata failed: {proc.stderr.strip()[:400]}")
    return json.loads(proc.stdout)


def load_lock(root: Path) -> dict[str, list[str]]:
    lock = tomllib.loads((root / "Cargo.lock").read_text())
    versions: dict[str, set[str]] = defaultdict(set)
    for package in lock.get("package", []):
        versions[package["name"]].add(package["version"])
    return {name: sorted(found) for name, found in versions.items()}


def direct_deps(meta: dict) -> dict[str, Dep]:
    members = {package["name"] for package in meta["packages"]}
    deps: dict[str, Dep] = {}
    for package in meta["packages"]:
        if scan.is_hack(package["name"]):
            continue
        for dep in package["dependencies"]:
            if dep["name"] in members or dep.get("path"):
                continue
            key = dep.get("rename") or dep["name"]
            found = deps.setdefault(key, Dep(key=key, package=dep["name"]))
            found.reqs.add(dep["req"])
            found.sources.add(dep.get("source") or "")
            found.features.update(dep["features"])
            found.default_flags.add(dep["uses_default_features"])
            kind = dep.get("kind")
            found.dependents.add(package["name"] + (f" ({kind})" if kind else ""))
    return deps


def _version_of(entry: dict) -> str | None:
    if entry.get("version"):
        return str(entry["version"])
    match = re.search(r'=\s*"?=?(\d[\w.\-]*)', entry.get("probe_dependency", ""))
    return match.group(1) if match else None


def load_groups(root: Path) -> list[Group]:
    """Every crate set and library of the enabled skills that indexes Rust crates."""
    skills_dir = root / ".claude" / "skills"
    enabled = tomllib.loads((root / ".config" / "library-skills.toml").read_text())["enabled"]
    groups: list[Group] = []
    for skill in enabled:
        if not (skills_dir / skill).exists():
            raise scan.SetupError(
                f"skill {skill!r} is enabled but not linked: run `just skills-sync`"
            )
        manifest = skills_dir / skill / "build" / "manifests" / f"{skill}.json"
        if not manifest.exists():
            continue
        data = json.loads(manifest.read_text())
        for crate_set in data.get("crate_sets", []):
            crates: dict[str, str | None] = {}
            for entry in crate_set.get("crates", []):
                name, version = (
                    (entry, None)
                    if isinstance(entry, str)
                    else (entry["package"], entry.get("version"))
                )
                crates[name] = str(version or crate_set.get("version") or "") or None
            if not crates:
                continue
            single = next(iter(crates)) if len(crates) == 1 else None
            label = (
                single or crate_set.get("facade") or crate_set.get("subject") or crate_set["name"]
            )
            groups.append(Group(skill, crate_set["name"], label, crates))
        for library in data.get("libraries", []):
            path = skills_dir / skill / "build" / "manifests" / "libraries" / f"{library}.json"
            detail: dict[str, Any] = json.loads(path.read_text()) if path.exists() else {}
            package = detail.get("package", library)
            version = _version_of(detail)
            crates: dict[str, str | None] = {package: version}
            for entry in detail.get("crates", []):
                crates.setdefault(entry if isinstance(entry, str) else entry["package"], None)
            groups.append(Group(skill, library, library, crates, family=True))
    return groups


def pin_of(dep: Dep, resolved: list[str]) -> str | None:
    source = sorted(dep.sources)[0]
    if source.startswith("git+"):
        match = re.match(
            r"git\+https?://[^/]+/([^?#]+?)(?:\.git)?(?:\?(\w+)=([^#]+))?(?:#.*)?$", source
        )
        if match:
            repo, kind, ref = match.groups()
            return f"git {repo} {kind or 'rev'} {(ref or '')[:7]}".strip()
        return "git"
    req = sorted(dep.reqs)[0]
    if req == "*":
        return resolved[0] if len(resolved) == 1 else None
    return req


def resolved_versions(dep: Dep, lock: dict[str, list[str]]) -> list[str]:
    exact = [req[1:] for req in dep.reqs if req.startswith("=")]
    return sorted(set(exact)) or lock.get(dep.package, [])


def covering(package: str, groups: list[Group]) -> list[tuple[Group, str | None]]:
    return [
        (group, version)
        for group in groups
        for name, version in group.crates.items()
        if norm(name) == norm(package)
    ]


def choose_skill(
    existing: str | None, cover: list[tuple[Group, str | None]], resolved: list[str]
) -> str | None:
    skills = [group.skill for group, _ in cover]
    if existing in skills:
        return existing
    for group, version in cover:
        if version in resolved:
            return group.skill
    return skills[0] if skills else None


def pin_delta(
    existing: str | None, cover: list[tuple[Group, str | None]], resolved: list[str]
) -> str | None:
    off = sorted({f"{g.skill} indexes {v}" for g, v in cover if v and v not in resolved})
    if not off:
        return None
    return existing or "; ".join(off) + f"; we resolve {', '.join(resolved) or '?'}"


def build(
    existing: list[dict],
    meta: dict,
    lock: dict[str, list[str]],
    groups: list[Group],
    refs: dict[str, list[scan.Ref]],
    graph: dict[tuple[str, str], unitgraph.Summary] | None = None,
) -> tuple[list[dict], dict[str, list[str]]]:
    """Return the new library records and report sections (cataloged, unlisted, orphans...).

    `graph` is the compiled dependency graph (`library_unitgraph.summarize`); when it is None the
    compiled-feature fields keep whatever the catalog already records.
    """
    old = {r["lib"]: r for r in existing if r["kind"] == "library"}
    deps = direct_deps(meta)
    out: dict[str, dict] = {}
    unlisted: list[str] = []
    for key, dep in sorted(deps.items()):
        resolved = resolved_versions(dep, lock)
        cover = covering(dep.package, groups)
        prior = old.get(key)
        if not cover and prior is None:
            unlisted.append(f"{key} ({', '.join(sorted(dep.dependents))})")
            continue
        record: dict[str, Any] = dict(prior) if prior else {"kind": "library", "wrappers": []}
        record["lib"] = key
        record["pin"] = pin_of(dep, resolved)
        record["features"] = sorted(dep.features)
        if dep.default_flags == {False}:
            record["default_features"] = False
        else:
            record.pop("default_features", None)
        record["skill"] = choose_skill(prior.get("skill") if prior else None, cover, resolved)
        record["skill_pin_delta"] = pin_delta(
            prior.get("skill_pin_delta") if prior else None, cover, resolved
        )
        record["dependents"] = sorted(dep.dependents)
        if graph is not None:
            found = [graph[(dep.package, v)] for v in resolved if (dep.package, v) in graph]
            record["resolved_features"] = sorted({f for g in found for f in g.features})
            record["linked_by"] = sorted({c for g in found for c in g.linked_by})
        record.pop("crates", None)
        record["used_in"] = scan.used_in(refs.get(key, []))
        record["status"] = scan.status_of(refs.get(key, []))
        if dep.package != key and not record.get("note"):
            record["note"] = (
                f"Workspace key renamed from package {dep.package}; `use` paths carry {key}."
            )
        out[key] = record
    # Keep the pin the hand wrote when it only extends the generated one.
    for key, record in out.items():
        prior = old.get(key)
        if prior and prior.get("pin") and record["pin"] and prior["pin"].startswith(record["pin"]):
            record["pin"] = prior["pin"]
    direct_packages = {norm(dep.package) for dep in deps.values()}
    for group in groups:
        if any(norm(name) in direct_packages for name in group.crates):
            continue
        label = group.label if group.label not in out else f"{group.skill}:{group.name}"
        prior = old.get(label)
        first = next((n for n in group.crates if n in lock), None)
        pin = f"{lock[first][0]} (transitive)" if first else None
        if prior and pin and (prior.get("pin") or "").startswith(pin.split(" ")[0]):
            pin = prior["pin"]
        record = {
            "kind": "library",
            "lib": label,
            "pin": pin,
            "features": [],
            "skill": group.skill,
            "skill_pin_delta": None,
            "dependents": [],
            "status": "not-used",
            "wrappers": [],
            "note": (prior or {}).get("note")
            or "Skill-covered; no direct dependency, so no local precedent."
            + (" Locked transitively." if first else ""),
        }
        if len(group.crates) > 1:
            record["crates"] = sorted(group.crates)
        out[label] = record
    report: dict[str, list[str]] = {"unlisted": unlisted}
    libs = set(out)
    report["orphan capabilities"] = [
        r["id"] for r in existing if r["kind"] == "capability" and r["lib"] not in libs
    ]
    return list(out.values()), report


def library_packages(
    deps: dict[str, Dep], lock: dict[str, list[str]], groups: list[Group]
) -> list[tuple[str, str, str | None]]:
    """(library key, package, version) for the packages whose items count as that library's.

    The library's own package at each resolved version (so a renamed 0.0.14 crate is not 0.0.11),
    plus, for a per-library skill manifest, the sibling crates of its family (sqlx-core for sqlx)
    at any version.
    """
    out: set[tuple[str, str, str | None]] = set()
    for key, dep in deps.items():
        out.update((key, dep.package, v) for v in resolved_versions(dep, lock))
        for group, _ in covering(dep.package, groups):
            if group.family:
                out.update(
                    (key, crate, None) for crate in group.crates if norm(crate) != norm(dep.package)
                )
    return sorted(out, key=lambda r: (r[0], r[1], r[2] or ""))


def cross_check(root: Path, meta: dict, refs: dict[str, list[scan.Ref]]) -> list[str]:
    """Where the source scan and an independent ripgrep file list disagree, per library."""
    lines: list[str] = []
    for key in sorted(direct_deps(meta)):
        other = scan.rg_files(root, meta, key)
        if other is None:
            return ["ripgrep (rg) not found: cross-check not run"]
        ours = {r.path for r in refs.get(key, [])}
        only_rg, only_ours = sorted(other - ours), sorted(ours - other)
        if only_rg or only_ours:
            sample = "; ".join(
                f"{len(v)} {name} e.g. {v[0]}"
                for name, v in (("rg-only", only_rg), ("scan-only", only_ours))
                if v
            )
            lines.append(f"{key}: {sample}")
    return lines


def normalize_capabilities(
    existing: list[dict],
    index: names.Index,
    libraries: list[dict],
    stamp: str,
    resolved: set[str] | frozenset[str] = frozenset(),
) -> tuple[list[dict], list[str], dict[str, list[str]]]:
    """Rewrite each capability's items to defining paths; report what the index cannot place."""
    hints = {r["lib"]: r.get("skill_pin_delta") for r in libraries}
    out: list[dict] = []
    drift: list[str] = []
    missing: list[str] = []
    differs: list[str] = []
    unindexed: dict[str, int] = defaultdict(int)
    for record in existing:
        if record["kind"] != "capability":
            continue
        record = dict(record)
        items, replaced, results = names.normalize_items(record["items"], index, resolved)
        for r in results:
            how = r.index_how if r.how == "resolved" else r.how
            seen = f" (skill lists {', '.join(r.candidates)})" if r.candidates else ""
            if r.how == "resolved" and how in ("absent", "ambiguous", "name"):
                differs.append(f"{record['id']}: {r.item}{seen or f' ({how})'}")
            elif how in ("absent", "ambiguous"):
                hint = f" [{hints[record['lib']]}]" if hints.get(record["lib"]) else ""
                missing.append(f"{record['id']}: {r.item} ({how}){seen}{hint}")
            elif how == "unindexed":
                unindexed[r.item.split("::")[0]] += 1
        prior = record.get("as_written", [])
        written = [w for w in prior + [x for x in replaced if x not in prior] if w not in items]
        if items != record["items"] or written != prior:
            drift.append(f"~ {record['id']}  items: {record['items']} -> {items}")
            record["items"] = items
            if written:
                record["as_written"] = written
            else:
                record.pop("as_written", None)
            record["verified"] = stamp
        out.append(record)
    report = {
        "items not in the skill index": missing,
        "compiler paths the skill index does not list": differs,
        "items in crates no skill indexes": [f"{c}: {n}" for c, n in sorted(unindexed.items())],
    }
    return out, drift, report


def _view(record: dict | None) -> dict:
    return {k: (record or {}).get(k) for k in GENERATED}


def stamp_changed(new: list[dict], old: dict[str, dict], stamp: str) -> list[str]:
    """Set `verified` on records whose generated fields changed; return the drift lines."""
    drift: list[str] = []
    for record in new:
        prior = old.get(record["lib"])
        if prior is None:
            drift.append(f"+ {record['lib']}  (new, {record['status']})")
        elif _view(prior) != _view(record):
            changed = [
                f"{k}: {json.dumps(prior.get(k))} -> {json.dumps(record.get(k))}"
                for k in GENERATED
                if prior.get(k) != record.get(k)
            ]
            drift.append(f"~ {record['lib']}  " + "; ".join(changed))
        else:
            continue
        record["verified"] = stamp
    kept = {r["lib"] for r in new}
    drift += [f"- {lib}  (no longer generated)" for lib in sorted(set(old) - kept)]
    return drift


def order(record: dict) -> dict:
    keys = LIBRARY_KEYS if record["kind"] == "library" else CAPABILITY_KEYS
    return {k: record[k] for k in keys if k in record} | {
        k: v for k, v in record.items() if k not in keys
    }


def render(records: list[dict]) -> str:
    records = sorted(
        records,
        key=lambda r: (r["lib"].lower(), 0 if r["kind"] == "library" else 1, r.get("id", "")),
    )
    return "".join(
        json.dumps(order(r), separators=(",", ":"), ensure_ascii=False) + "\n" for r in records
    )


def git_stamp(root: Path) -> str:
    sha = subprocess.run(
        ["git", "rev-parse", "--short", "HEAD"],
        cwd=root,
        capture_output=True,
        text=True,
        check=False,
    ).stdout.strip()
    return f"{datetime.date.today().isoformat()}@{sha or 'unknown'}"


EXIT_OK, EXIT_DRIFT, EXIT_SETUP, EXIT_CHANGED = 0, 1, 2, 3


@dataclass
class CatalogRun:
    """Everything one run of the pipeline computed; `main` prints it, tests and servers read it."""

    root: Path
    path: Path
    stamp: str
    existing_hash: str  # of the catalog file as read, to detect a change before writing
    libraries: list[dict]
    capabilities: list[dict]
    drift: list[str]
    report: dict[str, list[str]]
    hits: list[semantic.Hit]
    meta: dict
    packages: list[tuple[str, str, str | None]] = field(default_factory=list)
    notes: list[str] = field(default_factory=list)  # facts about optional inputs (unit graph)

    def records(self) -> list[dict]:
        return self.libraries + self.capabilities


def compute_catalog(
    root: Path,
    path: Path,
    resolved_text: str | None = None,
    metadata_file: Path | None = None,
    graph_loader: Callable[[Path], tuple[dict | None, str]] = unitgraph.load,
) -> CatalogRun:
    """Run every stage and return the result; nothing is written.

    S0 manifests (and the compiled dependency graph), S1 lexical scan, S2 rust-analyzer's resolved
    references (`resolved_text` is a saved resolver output; otherwise the tool runs), S3 canonical
    names. Raises `scan.SetupError` when a required input is missing.
    """
    stamp = git_stamp(root)
    raw = path.read_bytes()
    existing = [json.loads(line) for line in raw.decode().splitlines() if line.strip()]
    meta, lock = load_metadata(root, metadata_file), load_lock(root)
    notes: list[str] = []
    graph_json, why = graph_loader(root)
    summary = None
    if graph_json is None:
        notes.append(
            f"compiled dependency graph not read ({why}); compiled fields left as recorded"
        )
    else:
        summary = unitgraph.summarize(graph_json, {p["name"] for p in meta["packages"]})
    lexical = scan.scan_workspace(root, meta)
    hits = semantic.parse_hits(
        resolved_text if resolved_text is not None else semantic.run_tool(root)
    )
    roles = semantic.Roles(root, meta)
    deps = direct_deps(meta)
    keys = semantic.package_keys(deps, {k: resolved_versions(d, lock) for k, d in deps.items()})
    refs = semantic.augment_refs(lexical, hits, roles, keys)
    groups = load_groups(root)
    libraries, report = build(existing, meta, lock, groups, refs, summary)
    report["scan vs ripgrep"] = cross_check(root, meta, lexical)
    old = {r["lib"]: r for r in existing if r["kind"] == "library"}
    drift = stamp_changed(libraries, old, stamp)
    # S3 (canonical names from the skills' indexes), then S2's evidence for what they cannot place.
    capabilities, cap_drift, cap_report = normalize_capabilities(
        existing,
        names.load_index(root),
        libraries,
        stamp,
        {h.path for h in hits} | {h.path + "!" for h in hits if h.kind == "macro"},
    )
    capabilities, sem_drift, sem_report = semantic.apply(capabilities, hits, roles, stamp)
    report.update(cap_report)
    report.update(sem_report)
    return CatalogRun(
        root=root,
        path=path,
        stamp=stamp,
        existing_hash=hashlib.sha256(raw).hexdigest(),
        libraries=libraries,
        capabilities=capabilities,
        drift=drift + cap_drift + sem_drift,
        report=report,
        hits=hits,
        meta=meta,
        packages=library_packages(deps, lock, groups),
        notes=notes,
    )


def write_atomically(path: Path, text: str) -> None:
    """Replace `path` in one step: a reader sees the old file or the new one, not a partial one."""
    handle, temp = tempfile.mkstemp(dir=path.parent, prefix=f".{path.name}.", suffix=".tmp")
    try:
        with os.fdopen(handle, "w") as out:
            out.write(text)
        os.replace(temp, path)
    except BaseException:
        Path(temp).unlink(missing_ok=True)
        raise


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--write", action="store_true", help="rewrite the JSONL file")
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument("--jsonl", type=Path)
    parser.add_argument("--metadata", type=Path, help="a saved `cargo metadata --no-deps` file")
    parser.add_argument(
        "--resolved", type=Path, help="a saved `lu-resolve` output, instead of running the tool"
    )
    parser.add_argument("--json", action="store_true", help="print one JSON object, not text")
    parser.add_argument(
        "--index",
        type=Path,
        help="where --write puts the usage index (default: build/library-usage.sqlite)",
    )
    args = parser.parse_args(argv)
    root: Path = args.root
    path: Path = args.jsonl or root / JSONL
    try:
        run = compute_catalog(
            root,
            path,
            args.resolved.read_text() if args.resolved else None,
            args.metadata,
        )
    except scan.SetupError as error:
        print(f"library_utilization: {error}", file=sys.stderr)
        return EXIT_SETUP
    code = EXIT_DRIFT if run.drift else EXIT_OK
    wrote = False
    if args.write:
        if hashlib.sha256(path.read_bytes()).hexdigest() != run.existing_hash:
            print(
                f"library_utilization: {path} changed while the pipeline ran; nothing written",
                file=sys.stderr,
            )
            return EXIT_CHANGED
        text = render(run.records())
        write_atomically(path, text)
        wrote, code = True, EXIT_OK
        try:
            catalog_db.write_index(
                args.index or root / catalog_db.INDEX, run, text, semantic.Roles(root, run.meta)
            )
        except (OSError, sqlite3.Error) as error:
            print(
                f"library_utilization: catalog written; usage index failed: {error}",
                file=sys.stderr,
            )
            return EXIT_SETUP
    if args.json:
        print(
            json.dumps(
                {
                    "exit": code,
                    "wrote": wrote,
                    "drift": run.drift,
                    "report": {k: v for k, v in run.report.items() if v},
                    "notes": run.notes,
                    "counts": {
                        "libraries": len(run.libraries),
                        "capabilities": len(run.capabilities),
                        "usage_rows": len(run.hits),
                    },
                }
            )
        )
        return code
    for line in run.drift:
        print(line)
    for title, items in run.report.items():
        if items:
            print(f"\n{title} ({len(items)}):\n  " + "\n  ".join(items))
    for note in run.notes:
        print(f"\nnote: {note}")
    if wrote:
        print(
            f"\nwrote {path} ({len(run.libraries)} libraries, {len(run.capabilities)} capabilities)"
        )
    elif run.drift:
        print("\ndrift: rerun with --write")
    return code


if __name__ == "__main__":
    raise SystemExit(main())
