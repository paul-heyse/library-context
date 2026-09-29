"""Stage S2 of the library utilization catalog: merge resolved references into the catalog.

`tools/lu-resolve` loads the workspace into rust-analyzer with no build and prints, for every file,
the library items its paths and method calls resolve to (defining path, package, version, lines).
That is the evidence this module joins to the catalog:

- Library `used_in` and `status` gain the files S1's lexical scan cannot see (references through a
  re-export, `datafusion::arrow::...`, an imported name, an inferred method call).
- A capability's `items` that are not defining paths (crates no skill indexes, macros re-exported
  under another path) are rewritten to the path rust-analyzer resolved, from evidence in the
  capability's own files first and a unique repository-wide match second.
- A capability's `files`, `n_files` and `status` are regenerated from the files where its items
  resolve. Entries that are indirect by nature (a consumer of a wrapper, a macro expansion, a SQL
  string, an uncompiled dormant file) are kept: the resolver cannot see them.
- Resolved library items that match no capability are reported, to triage new usage.
"""

from __future__ import annotations

import json
import subprocess
import sys
from collections import defaultdict
from dataclasses import dataclass
from pathlib import Path

import library_scan as scan

TOOL_DIR = Path("tools/lu-resolve")
TOOL = TOOL_DIR / "target" / "release" / "lu-resolve"
TOOL_SOURCES = ("src/main.rs", "Cargo.toml", "Cargo.lock", "rust-toolchain.toml")
# Roles the resolver cannot evidence, and any non-Rust file (it only sees Rust); entries like these
# are kept as recorded.
INDIRECT_ROLES = {"consumer", "macro-expansion", "sql-string", "dormant"}
LISTED = 5  # files listed per capability, best example first
CATALOG_ROLES = {"src": "consumer", "build": "consumer", "test": "test", "dormant": "dormant"}


def norm(name: str) -> str:
    return name.replace("_", "-").lower()


@dataclass(frozen=True)
class Hit:
    """A workspace file's references to one resolved library item."""

    file: str
    package: str
    version: str
    path: str
    via: str
    lines: tuple[int, ...]


def parse_hits(text: str) -> list[Hit]:
    hits = []
    for line in text.splitlines():
        if line.strip():
            r = json.loads(line)
            hits.append(
                Hit(r["file"], r["package"], r["version"], r["path"], r["via"], tuple(r["lines"]))
            )
    return hits


def ensure_tool(root: Path) -> Path:
    """The resolver binary, built (offline, on its own pinned toolchain) when missing or stale."""
    tool_dir, binary = root / TOOL_DIR, root / TOOL
    newest = max((tool_dir / name).stat().st_mtime for name in TOOL_SOURCES)
    if not binary.exists() or binary.stat().st_mtime < newest:
        print("building tools/lu-resolve (offline) ...", file=sys.stderr)
        proc = subprocess.run(
            ["cargo", "build", "--release", "--offline"],
            cwd=tool_dir,
            capture_output=True,
            text=True,
            check=False,
        )
        if proc.returncode != 0:
            sys.exit(f"lu-resolve build failed:\n{proc.stderr[-1500:]}")
    return binary


def run_tool(root: Path) -> str:
    """Run the resolver over the workspace: no build of the workspace, a couple of minutes."""
    proc = subprocess.run(
        [str(ensure_tool(root)), str(root)], capture_output=True, text=True, check=False
    )
    if proc.returncode != 0:
        sys.exit(f"lu-resolve failed:\n{proc.stderr[-1500:]}")
    print(proc.stderr.strip().splitlines()[-1], file=sys.stderr)
    return proc.stdout


class Roles:
    """The role of a file's references: by location, then by whether they sit in test items."""

    def __init__(self, root: Path, meta: dict) -> None:
        self.root = root
        self.dirs = sorted(
            (
                (Path(p["manifest_path"]).parent.relative_to(root).as_posix(), p["name"])
                for p in meta["packages"]
                if p["name"] != scan.HACK
            ),
            key=lambda d: -len(d[0]),
        )
        self._spans: dict[str, list[tuple[int, int]]] = {}

    def package(self, file: str) -> str | None:
        return next((name for d, name in self.dirs if file.startswith(d + "/")), None)

    def _test_spans(self, file: str) -> list[tuple[int, int]]:
        if file not in self._spans:
            text = (self.root / file).read_text(errors="replace")
            self._spans[file] = [
                (text.count("\n", 0, a) + 1, text.count("\n", 0, b) + 1)
                for a, b in scan.test_regions(scan.mask(text))
            ]
        return self._spans[file]

    def role(self, file: str, lines: tuple[int, ...]) -> str:
        package_dir = next((d for d, _ in self.dirs if file.startswith(d + "/")), "")
        base = scan.path_role(Path(file).relative_to(package_dir))
        spans = self._test_spans(file) if base == "src" else []
        return scan.file_role(base, [any(a <= n <= b for a, b in spans) for n in lines])


def package_keys(deps: dict, resolved: dict) -> dict[tuple[str, str], str]:
    """(package, version) -> the catalog library key, so a renamed 0.0.14 crate is not 0.0.11."""
    return {(dep.package, version): key for key, dep in deps.items() for version in resolved[key]}


def augment_refs(
    refs: dict[str, list[scan.Ref]],
    hits: list[Hit],
    roles: Roles,
    keys: dict[tuple[str, str], str],
) -> dict[str, list[scan.Ref]]:
    """Add the files the lexical scan missed, keyed by the library each hit resolves into."""
    out = {key: list(found) for key, found in refs.items()}
    seen = {(key, r.path) for key, found in out.items() for r in found}
    grouped: dict[tuple[str, str], set[int]] = defaultdict(set)
    for hit in hits:
        key = keys.get((hit.package, hit.version))
        if key is not None and (key, hit.file) not in seen:
            grouped[(key, hit.file)].update(hit.lines)
    for (key, file), lines in sorted(grouped.items()):
        package = roles.package(file)
        if package is not None:
            out.setdefault(key, []).append(
                scan.Ref(package, file, roles.role(file, tuple(sorted(lines))), len(lines))
            )
    return out


def _family(package: str, crate: str) -> bool:
    a, b = norm(package), norm(crate)
    return a.startswith(b) or b.startswith(a)


def evidence_normalize(
    items: list[str],
    files: list[str],
    by_path: dict[str, list[Hit]],
    by_file: dict[str, list[Hit]],
    by_leaf: dict[str, list[Hit]],
) -> tuple[list[str], list[str], list[str]]:
    """Rewrite items no index could place to the path rust-analyzer resolved.

    Candidates share the item's last segment, its crate family and (for `Type::method`) its owner;
    the capability's own files are searched first, then the whole repository. One candidate path
    rewrites the item; none or several leave it and are reported.
    """
    out: list[str] = []
    replaced: list[str] = []
    notes: list[str] = []
    for item in items:
        bang = "!" if item.endswith("!") else ""
        segments = item.removesuffix("!").split("::")
        leaf, crate = segments[-1], segments[0]
        owner = segments[-2] if len(segments) > 2 and segments[-2][:1].isupper() else None

        def pick(
            hits: list[Hit], leaf: str = leaf, crate: str = crate, owner: str | None = owner
        ) -> set[str]:
            return {
                h.path
                for h in hits
                if h.path.rsplit("::", 1)[-1] == leaf
                and _family(h.package, crate)
                and (owner is None or h.path.split("::")[-2:-1] == [owner])
            }

        target = item
        if item.removesuffix("!") not in by_path:
            near = pick([h for f in files for h in by_file.get(f, [])]) or pick(
                by_leaf.get(leaf, [])
            )
            if len(near) == 1:
                target = next(iter(near)) + bang
                replaced.append(item)
            else:
                notes.append(f"{item} ({'ambiguous' if near else 'not resolved'})")
        if target not in out:
            out.append(target)
    return out, replaced, notes


def merge_files(
    existing: list[dict],
    items: list[str],
    by_path: dict[str, list[Hit]],
    roles: Roles,
) -> tuple[list[dict], list[str]]:
    """Regenerate a capability's file list from where its items resolve.

    `n_files` is then the length of this list: files with evidence plus the indirect entries kept.
    """
    lines: dict[str, set[int]] = defaultdict(set)
    for item in items:
        for hit in by_path.get(item.removesuffix("!"), []):
            lines[hit.file].update(hit.lines)
    resolved = {f: roles.role(f, tuple(sorted(n))) for f, n in lines.items()}
    files: list[dict] = []
    dropped: list[str] = []
    seen: set[str] = set()
    for entry in existing:
        path, role = entry["path"], entry["role"]
        if path in seen:
            continue
        if path in resolved:
            found = resolved[path]
            keep = role if found in ("src", "build") and role not in ("test", "dormant") else None
            files.append({"path": path, "role": keep or CATALOG_ROLES[found]})
        elif (role in INDIRECT_ROLES or not path.endswith(".rs")) and (roles.root / path).exists():
            files.append(entry)
        else:
            dropped.append(f"{path} ({role})")
            continue
        seen.add(path)
    for path in sorted(set(resolved) - seen, key=lambda f: (-len(lines[f]), f)):
        files.append({"path": path, "role": CATALOG_ROLES[resolved[path]]})
    return files, dropped


def capability_status(files: list[dict], old: str) -> str:
    refs = [
        scan.Ref(
            "",
            f["path"],
            {"consumer": "src", "test": "test", "dormant": "dormant"}.get(f["role"], "src"),
            1,
        )
        for f in files
    ]
    status = scan.status_of(refs)
    return old if status == "declared-unused" else status


def apply(
    existing: list[dict], hits: list[Hit], roles: Roles, stamp: str
) -> tuple[list[dict], list[str], dict[str, list[str]]]:
    """Normalize items from evidence and regenerate files, n_files and status per capability."""
    by_path: dict[str, list[Hit]] = defaultdict(list)
    by_file: dict[str, list[Hit]] = defaultdict(list)
    by_leaf: dict[str, list[Hit]] = defaultdict(list)
    for hit in hits:
        by_path[hit.path].append(hit)
        by_file[hit.file].append(hit)
        by_leaf[hit.path.rsplit("::", 1)[-1]].append(hit)
    out: list[dict] = []
    drift: list[str] = []
    unplaced: list[str] = []
    dropped_lines: list[str] = []
    matched: set[str] = set()
    for record in existing:
        if record["kind"] != "capability":
            continue
        new = dict(record)
        items, replaced, notes = evidence_normalize(
            record["items"], [f["path"] for f in record.get("files", [])], by_path, by_file, by_leaf
        )
        unplaced += [f"{record['id']}: {note}" for note in notes]
        prior = record.get("as_written", [])
        written = prior + [x for x in replaced if x not in prior]
        files, dropped = merge_files(record.get("files", []), items, by_path, roles)
        matched.update(i.removesuffix("!") for i in items)
        new["items"] = items
        if written:
            new["as_written"] = written
        if files and any(i.removesuffix("!") in by_path for i in items):
            dropped_lines += [f"{record['id']}: {d}" for d in dropped]
            new["files"] = files[:LISTED]
            new["n_files"] = len(files)
            new["status"] = capability_status(files, record.get("status", "used"))
        else:  # nothing resolved (macro or derive use only): keep the files as recorded
            dropped_lines.append(f"{record['id']}: no item resolved; files kept as recorded")
        changed = [
            k
            for k in ("items", "files", "n_files", "status", "as_written")
            if new.get(k) != record.get(k)
        ]
        if changed:
            drift.append(f"~ {record['id']}  " + ", ".join(changed))
            new["verified"] = stamp
        out.append(new)
    uncatalogued = _uncatalogued(hits, matched)
    return (
        out,
        drift,
        {
            "items not resolved by rust-analyzer": unplaced,
            "capability files dropped (no resolution)": dropped_lines,
            "resolved library items no capability records": uncatalogued,
        },
    )


def _uncatalogued(hits: list[Hit], matched: set[str], per_package: int = 5) -> list[str]:
    files: dict[tuple[str, str], set[str]] = defaultdict(set)
    for hit in hits:
        if hit.path not in matched:
            files[(hit.package, hit.path)].add(hit.file)
    by_package: dict[str, list[tuple[str, int]]] = defaultdict(list)
    for (package, path), found in files.items():
        if len(found) >= 2:
            by_package[package].append((path, len(found)))
    lines = []
    for package in sorted(by_package):
        top = sorted(by_package[package], key=lambda x: (-x[1], x[0]))[:per_package]
        lines.append(f"{package}: " + "; ".join(f"{p} ({n} files)" for p, n in top))
    return lines
