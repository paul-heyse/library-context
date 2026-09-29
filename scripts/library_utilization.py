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
import json
import re
import subprocess
import sys
import tomllib
from collections import defaultdict
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

import library_names as names
import library_scan as scan

HACK = scan.HACK  # cargo-hakari feature-unification stub, never real use
INDIRECT_ROLES = {"consumer", "macro-expansion", "sql-string"}
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
    "default_features",
    "skill",
    "skill_pin_delta",
    "dependents",
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
    "default_features",
    "skill",
    "skill_pin_delta",
    "dependents",
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
        sys.exit(f"cargo metadata failed: {proc.stderr.strip()[:400]}")
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
        if package["name"] == HACK:
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
            sys.exit(f"skill {skill!r} is enabled but not linked: run `just skills-sync`")
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
            groups.append(Group(skill, library, library, crates))
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
) -> tuple[list[dict], dict[str, list[str]]]:
    """Return the new library records and report sections (cataloged, unlisted, orphans...)."""
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


def capability_drift(
    existing: list[dict], refs: dict[str, list[scan.Ref]], root: Path
) -> list[str]:
    """Capability files that are gone or no longer reference their library (advisory).

    Consumer, macro-expansion and sql-string files reach a library through a wrapper, a macro or
    a query file, so they are only checked for existence.
    """
    by_path: dict[str, set[str]] = defaultdict(set)
    for key, found in refs.items():
        for ref in found:
            by_path[ref.path].add(key)
    lines: list[str] = []
    for record in existing:
        if record["kind"] != "capability":
            continue
        for entry in record.get("files", []):
            path, role = entry["path"], entry["role"]
            if not (root / path).exists():
                lines.append(f"{record['id']}: {path} is missing")
            elif role not in INDIRECT_ROLES and record["lib"] not in by_path[path]:
                lines.append(f"{record['id']}: {path} ({role}) has no reference to {record['lib']}")
    return lines


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
    existing: list[dict], index: names.Index, libraries: list[dict], stamp: str
) -> tuple[list[dict], list[str], dict[str, list[str]]]:
    """Rewrite each capability's items to defining paths; report what the index cannot place."""
    hints = {r["lib"]: r.get("skill_pin_delta") for r in libraries}
    out: list[dict] = []
    drift: list[str] = []
    missing: list[str] = []
    unindexed: dict[str, int] = defaultdict(int)
    for record in existing:
        if record["kind"] != "capability":
            continue
        record = dict(record)
        items, replaced, resolved = names.normalize_items(record["items"], index)
        for r in resolved:
            if r.how in ("absent", "ambiguous"):
                hint = f" [{hints[record['lib']]}]" if hints.get(record["lib"]) else ""
                missing.append(f"{record['id']}: {r.item} ({r.how}){hint}")
            elif r.how == "unindexed":
                unindexed[r.item.split("::")[0]] += 1
        prior = record.get("as_written", [])
        written = prior + [x for x in replaced if x not in prior]
        if items != record["items"] or written != prior:
            drift.append(f"~ {record['id']}  items: {record['items']} -> {items}")
            record["items"] = items
            if written:
                record["as_written"] = written
            record["verified"] = stamp
        out.append(record)
    report = {
        "items not in the skill index": missing,
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


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--write", action="store_true", help="rewrite the JSONL file")
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument("--jsonl", type=Path)
    parser.add_argument("--metadata", type=Path, help="a saved `cargo metadata --no-deps` file")
    args = parser.parse_args(argv)
    root: Path = args.root
    path: Path = args.jsonl or root / JSONL
    existing = [json.loads(line) for line in path.read_text().splitlines() if line.strip()]
    meta = load_metadata(root, args.metadata)
    refs = scan.scan_workspace(root, meta)
    libraries, report = build(existing, meta, load_lock(root), load_groups(root), refs)
    report["capability files"] = capability_drift(existing, refs, root)
    report["scan vs ripgrep"] = cross_check(root, meta, refs)
    old = {r["lib"]: r for r in existing if r["kind"] == "library"}
    drift = stamp_changed(libraries, old, git_stamp(root))
    capabilities, cap_drift, cap_report = normalize_capabilities(
        existing, names.load_index(root), libraries, git_stamp(root)
    )
    drift += cap_drift
    report.update(cap_report)
    for line in drift:
        print(line)
    for title, items in report.items():
        if items:
            print(f"\n{title} ({len(items)}):\n  " + "\n  ".join(items))
    if args.write:
        path.write_text(render(libraries + capabilities))
        print(f"\nwrote {path} ({len(libraries)} libraries, {len(capabilities)} capabilities)")
        return 0
    if drift:
        print("\ndrift: rerun with --write")
    return 1 if drift else 0


if __name__ == "__main__":
    raise SystemExit(main())
