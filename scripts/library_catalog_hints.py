"""Answers and computed hints for the library-utilization catalog server.

Every function takes a `Context` and returns a plain dict, so the server is a thin wrapper and all
behaviour is testable without FastMCP. The rules these functions keep:

1. **Complete.** Every matching record and usage row is returned. An optional caller `limit` is the
   only cap, and then the response counts what it omitted.
2. **Absence is answered from the usage index, with its specific blind spots.** A curated record's
   silence never means a library or item is unused. Without a usage index the answer says that no
   index exists, not that nothing is used.
3. **Staleness is specific or absent.** Nothing is said because a timestamp or hash moved. Two
   checks run, each reported only when it finds a difference: the source files a result cites
   against the mtime and size recorded when the index was built (and, for an absence answer, the
   changed files against the name being asked about), and the compiled features the catalog records
   against the live nightly unit graph.
"""

from __future__ import annotations

import difflib
import json
import re
import tomllib
from collections.abc import Callable
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

import library_catalog_db as db
import library_names as names
import library_scan as scan
import library_unitgraph as unitgraph

READER_FALLBACK = "rg -n '<name>' .claude/skills/{skill}/content/index"
_CARGO_LOCKS = ("Cargo.lock",)


def norm(name: str) -> str:
    return name.replace("_", "-").lower()


# -- live inputs, each cached until the files it reads change --------------------------------


@dataclass
class GraphCache:
    """The parsed unit graph, re-read only when a Cargo.toml or the lock file changes."""

    root: Path
    loader: Callable[[Path], tuple[dict | None, str]] = unitgraph.load
    _key: tuple | None = field(default=None, repr=False)
    _summary: dict | None = field(default=None, repr=False)
    _why: str = field(default="", repr=False)

    def _fingerprint(self) -> tuple:
        files = [self.root / n for n in _CARGO_LOCKS]
        files.append(self.root / "Cargo.toml")
        for base in ("crates", "python"):
            if (self.root / base).is_dir():
                files += [
                    p
                    for p in sorted((self.root / base).rglob("Cargo.toml"))
                    if not scan.SKIP_PARTS & set(p.relative_to(self.root).parts)
                ]
        return tuple((str(f), f.stat().st_mtime_ns) for f in files if f.exists())

    def summary(self) -> tuple[dict | None, str]:
        key = self._fingerprint()
        if key != self._key:
            graph, why = self.loader(self.root)
            self._key, self._why = key, why
            self._summary = (
                unitgraph.summarize(graph, unitgraph.members_of(graph, self.root))
                if graph
                else None
            )
        return self._summary, self._why


def workspace_dependency_names(root: Path) -> set[str]:
    """Dependency keys declared in the root manifest's `[workspace.dependencies]`, normalised."""
    try:
        data = tomllib.loads((root / "Cargo.toml").read_text())
    except OSError, tomllib.TOMLDecodeError:
        return set()
    return {norm(k) for k in data.get("workspace", {}).get("dependencies", {})}


@dataclass
class Context:
    """What the answer builders read: the store, the repo root and the live-check helpers."""

    root: Path
    store: db.Store
    graph: GraphCache
    index: names.Index | None = None
    _index_loaded: bool = False

    def name_index(self) -> names.Index | None:
        if not self._index_loaded:
            try:
                self.index = names.load_index(self.root)
            except OSError, KeyError:
                self.index = None
            self._index_loaded = True
        return self.index


# -- validation (rule 3) -------------------------------------------------------------------------


def _stat(root: Path, rel: str) -> tuple[int, int] | None:
    try:
        s = (root / rel).stat()
    except OSError:
        return None
    return (s.st_mtime_ns, s.st_size)


def validate_files(ctx: Context, files: list[str]) -> list[dict]:
    """Cited files that differ from what the usage index recorded; empty when none does."""
    if not ctx.store.has_usage:
        return []
    states = ctx.store.file_states(sorted(set(files)))
    changed = []
    for rel in sorted(set(files)):
        now = _stat(ctx.root, rel)
        if rel not in states:
            if now is not None and rel.endswith(".rs"):
                changed.append({"file": rel, "change": "not in the usage index"})
        elif now is None:
            changed.append({"file": rel, "change": "deleted since the usage index was built"})
        elif now != states[rel]:
            changed.append({"file": rel, "change": "modified since the usage index was built"})
    return changed


def workspace_sources(root: Path) -> list[str]:
    found = []
    for base in ("crates", "python"):
        for path in sorted((root / base).rglob("*.rs")) if (root / base).is_dir() else []:
            rel = path.relative_to(root)
            if not scan.SKIP_PARTS & set(rel.parts):
                found.append(rel.as_posix())
    return found


def unindexed_mentions(ctx: Context, needle: str) -> list[dict]:
    """Source files that changed after indexing and mention `needle`: the doubt for an absence."""
    if not ctx.store.has_usage or not needle:
        return []
    states = ctx.store.file_states()
    mentions = []
    for rel in workspace_sources(ctx.root):
        if states.get(rel) == _stat(ctx.root, rel):
            continue
        try:
            text = (ctx.root / rel).read_text(errors="replace")
        except OSError:
            continue
        if needle in text:
            change = "new" if rel not in states else "modified"
            mentions.append({"file": rel, "change": f"{change} since the usage index was built"})
    return mentions


def validate_dependency_facts(ctx: Context, record: dict) -> dict | None:
    """The catalog's compiled features and linking crates against the live unit graph."""
    if "resolved_features" not in record:
        return None
    summary, why = ctx.graph.summary()
    if summary is None:
        return {"unavailable": why}
    rows = [
        (p, v)
        for lib, p, v in _library_packages(ctx, record["lib"])
        if lib == record["lib"] and v is not None
    ]
    found = [summary[key] for key in rows if key in summary]
    if not found:
        return None
    live_features = sorted({f for s in found for f in s.features})
    live_linked = sorted({c for s in found for c in s.linked_by})
    out: dict[str, Any] = {}
    if live_features != record["resolved_features"]:
        out["resolved_features"] = {
            "recorded": record["resolved_features"],
            "live": live_features,
            "added": sorted(set(live_features) - set(record["resolved_features"])),
            "removed": sorted(set(record["resolved_features"]) - set(live_features)),
        }
    if live_linked != record.get("linked_by", []):
        out["linked_by"] = {"recorded": record.get("linked_by", []), "live": live_linked}
    return out or None


def _library_packages(ctx: Context, lib: str) -> list[tuple[str, str, str | None]]:
    rows = ctx.store.conn.execute(
        "SELECT lib, package, version FROM library_packages WHERE lib = ?", (lib,)
    ).fetchall()
    return [(r["lib"], r["package"], r["version"]) for r in rows]


# -- skills and readers --------------------------------------------------------------------------


def skill_info(ctx: Context, skill: str | None) -> dict | None:
    """The skill covering a library and how to ask it; facts about this checkout, not advice."""
    if not skill:
        return None
    base = ctx.root / ".claude" / "skills" / skill
    info: dict[str, Any] = {"skill": skill, "linked": base.exists()}
    if not base.exists():
        info["fact"] = f".claude/skills/{skill} is not linked in this checkout"
        return info
    reader = base / "scripts" / "reference.py"
    if reader.exists():
        commands = sorted(
            set(re.findall(r"add_parser\(\s*[\"']([a-z][a-z-]*)[\"']", reader.read_text()))
        )
        info["reader"] = f".claude/skills/{skill}/scripts/reference.py"
        info["subcommands"] = commands
        info["examples"] = [
            f"python .claude/skills/{skill}/scripts/reference.py {c} ..." for c in commands
        ]
    else:
        info["reader"] = None
        info["fallback"] = READER_FALLBACK.format(skill=skill)
    return info


def _pin_delta_hint(record: dict) -> str | None:
    delta = record.get("skill_pin_delta")
    return (
        f"skill pin differs from ours: {delta} (docs/pins.md lists the known deltas)"
        if delta
        else None
    )


_STATUS_MEANING = {
    "test-only": "referenced only from test code",
    "dormant-only": "referenced only from uncompiled (dormant) files",
    "declared-unused": (
        "declared as a dependency but no indexed source references it "
        "(use through a macro or derive would not show)"
    ),
}


# -- lookups -------------------------------------------------------------------------------------


def _did_you_mean(ctx: Context, name: str) -> list[str]:
    known = ctx.store.library_names()
    lowered = {norm(k): k for k in known}
    hits = difflib.get_close_matches(norm(name), list(lowered), n=5, cutoff=0.6)
    return [lowered[h] for h in hits]


def incomplete_file_lists(capabilities: list[dict]) -> list[str]:
    """Capabilities that record more files than they list: use the resolver cannot see."""
    return [
        f"{c['id']} records {c['n_files']} files but lists {len(c.get('files', []))}: the rest use "
        "it through macro or derive output, which the resolver does not see, so a reverse lookup "
        "by file can miss them"
        for c in capabilities
        if c.get("n_files", 0) > len(c.get("files", []))
    ]


def _cited_files(capabilities: list[dict], usage: list[dict]) -> list[str]:
    files = {f["path"] for c in capabilities for f in c.get("files", [])}
    files |= {c["wrapper"]["path"] for c in capabilities if c.get("wrapper")}
    files |= {u["file"] for u in usage}
    return sorted(f for f in files if f.endswith(".rs"))


def get_library(ctx: Context, lib: str) -> dict:
    store = ctx.store
    record = store.library(lib)
    if record is None:
        matches = [n for n in store.library_names() if norm(n) == norm(lib)]
        record = store.library(matches[0]) if matches else None
    if record is None:
        return _unknown_library(ctx, lib)
    caps = store.capabilities(record["lib"])
    usage = store.usage_by_library(record["lib"]) if store.has_usage else []
    status = record.get("status", "")
    state = "not_used" if status == "not-used" else "found"
    hints: list[str] = []
    if state == "not_used":
        hints.append(
            "skill-covered but not a dependency of any workspace crate: there is no local "
            "precedent for this library"
        )
    if status in _STATUS_MEANING:
        hints.append(f"status {status}: {_STATUS_MEANING[status]}")
    hints.extend(incomplete_file_lists(caps))
    if record.get("wrappers"):
        w = record["wrappers"]
        hints.append(
            "reuse the local abstraction: " + ", ".join(f"{x['symbol']} ({x['path']})" for x in w)
        )
    delta = _pin_delta_hint(record)
    if delta:
        hints.append(delta)
    if not caps and state == "found":
        hints.append(
            "no curated capability records this library's use; the usage rows (library_usage) "
            "list every resolved item it is referenced through"
        )
    skill = skill_info(ctx, record.get("skill"))
    if skill and not skill["linked"]:
        hints.append(skill["fact"])
    validation: dict[str, Any] = {}
    files = validate_files(ctx, _cited_files(caps, usage))
    if files:
        validation["files"] = files
    dependency = validate_dependency_facts(ctx, record)
    if dependency:
        validation["dependency_facts"] = dependency
    answer: dict[str, Any] = {
        "state": state,
        "library": record,
        "capabilities": caps,
        "usage": _usage_totals(usage) if store.has_usage else None,
        "skill": skill,
        "hints": hints,
        "next": [{"tool": "library_usage", "args": {"lib": record["lib"]}}]
        if store.has_usage and usage
        else [],
    }
    if validation:
        answer["validation"] = validation
    return answer


def _unknown_library(ctx: Context, lib: str) -> dict:
    declared = norm(lib) in workspace_dependency_names(ctx.root)
    candidates = _did_you_mean(ctx, lib)
    if declared:
        state = "dependency_without_record"
        hints = [
            f"{lib} is declared in the workspace Cargo.toml but the catalog has no record for it; "
            "that is a gap in the catalog, not evidence the library is unused"
        ]
    else:
        state = "unknown"
        hints = [
            f"the catalog has no record for {lib} and the workspace does not declare it; absence "
            "of a record says nothing about whether the library could serve a need"
        ]
        if ctx.store.has_usage:
            rows = ctx.store.conn.execute(
                "SELECT DISTINCT package FROM usage WHERE package = ? OR package = ?",
                (lib, lib.replace("-", "_")),
            ).fetchall()
            if rows:
                hints.append(f"the usage index does show items from package {rows[0]['package']}")
    if candidates:
        hints.append("similar library names: " + ", ".join(candidates))
    return {
        "state": state,
        "library": None,
        "capabilities": [],
        "candidates": candidates,
        "hints": hints,
        "next": [{"tool": "find_item", "args": {"item": lib}}],
    }


def _usage_totals(rows: list[dict]) -> dict:
    return {
        "rows": len(rows),
        "items": len({r["path"] for r in rows}),
        "files": len({r["file"] for r in rows}),
        "packages": sorted({r["package"] for r in rows}),
    }


def _group_by_item(rows: list[dict]) -> list[dict]:
    grouped: dict[tuple[str, str], dict] = {}
    for r in rows:
        entry = grouped.setdefault(
            (r["path"], r["kind"]),
            {
                "path": r["path"],
                "kind": r["kind"],
                "package": r["package"],
                "version": r["version"],
                "files": [],
            },
        )
        entry["files"].append(
            {
                "file": r["file"],
                "lines": r["lines"],
                "via": r["via"],
                "role": r["role"],
                "workspace_package": r["workspace_package"],
            }
        )
    return sorted(grouped.values(), key=lambda e: (e["path"], e["kind"]))


def _group_by_file(rows: list[dict]) -> list[dict]:
    grouped: dict[str, dict] = {}
    for r in rows:
        entry = grouped.setdefault(
            r["file"],
            {
                "file": r["file"],
                "role": r["role"],
                "workspace_package": r["workspace_package"],
                "items": [],
            },
        )
        entry["items"].append(
            {
                "path": r["path"],
                "kind": r["kind"],
                "lines": r["lines"],
                "via": r["via"],
                "package": r["package"],
            }
        )
    return sorted(grouped.values(), key=lambda e: e["file"])


def _no_index_answer(what: str) -> dict:
    return {
        "state": "no_usage_index",
        "hints": [
            f"no usage index has been built, so {what} could not be checked against the source; "
            "the curated catalog was searched and its silence does not show anything is unused. "
            "The index is written by a full scripts/library_utilization.py --write run."
        ],
    }


def library_usage(ctx: Context, lib: str, group_by: str = "item") -> dict:
    record = ctx.store.library(lib) or next(
        (ctx.store.library(n) for n in ctx.store.library_names() if norm(n) == norm(lib)), None
    )
    if record is None:
        return {**_unknown_library(ctx, lib), "usage": []}
    if not ctx.store.has_usage:
        return {
            **_no_index_answer(f"where {record['lib']} is used"),
            "library": record["lib"],
            "usage": [],
        }
    rows = ctx.store.usage_by_library(record["lib"])
    answer: dict[str, Any] = {
        "state": "found" if rows else "no_reference_found",
        "library": record["lib"],
        "group_by": group_by,
        "totals": _usage_totals(rows),
        "usage": _group_by_file(rows) if group_by == "file" else _group_by_item(rows),
        "hints": [],
    }
    if not rows:
        answer["hints"].append(
            f"no indexed source references an item of {record['lib']}; the blind spots below "
            "could still hide use"
        )
        answer["blind_spots"] = blind_spots(ctx)
        mentions = unindexed_mentions(ctx, record["lib"].replace("-", "_"))
        if mentions:
            answer["validation"] = {"changed_files_mentioning": mentions}
    else:
        changed = validate_files(ctx, sorted({r["file"] for r in rows}))
        if changed:
            answer["validation"] = {"files": changed}
    return answer


def blind_spots(ctx: Context) -> list[str]:
    raw = ctx.store.meta.get("blind_spots")
    return json.loads(raw) if raw else list(db.BLIND_SPOTS)


def find_item(ctx: Context, item: str) -> dict:
    """Everything the catalog and the usage index say about an item, however it is spelled."""
    store = ctx.store
    text = item.strip()
    bare = text.removesuffix("!")
    leaf = bare.rsplit("::", 1)[-1]
    candidates = {bare}
    normalized: dict[str, Any] = {"requested": text}
    index = ctx.name_index()
    if index is not None and "::" in bare:
        resolved = names.resolve(text, index)
        normalized["how"] = resolved.how
        if resolved.canonical:
            candidates.add(resolved.canonical.removesuffix("!"))
            normalized["canonical"] = resolved.canonical
        candidates.update(c.removesuffix("!") for c in resolved.candidates)
        if resolved.candidates:
            normalized["candidates"] = list(resolved.candidates)
    exact: list[dict] = []
    for path in sorted(candidates):
        exact += store.usage_by_path(path)
    by_leaf = store.usage_by_leaf(leaf) if store.has_usage else []
    usage = exact or by_leaf
    caps = store.capabilities_with_items(sorted(candidates), leaf if "::" not in bare else None)
    # A path nobody listed falls back to capabilities sharing the leaf, labelled as such.
    by_name = store.capabilities_with_items([], leaf) if not caps and "::" in bare else []
    answer: dict[str, Any] = {
        "requested": text,
        "normalized": normalized,
        "capabilities": caps,
        "hints": [],
        "next": [],
    }
    if by_name:
        answer["capabilities_sharing_the_name"] = by_name
    if usage:
        answer["match"] = "path" if exact else "leaf"
        answer["usage"] = _group_by_item(usage)
        answer["totals"] = _usage_totals(usage)
        if not exact:
            answer["hints"].append(
                f"no resolved item has the path {bare}; these are the {len(_group_by_item(usage))} "
                f"paths that end in {leaf}"
            )
        changed = validate_files(ctx, sorted({r["file"] for r in usage}))
        if changed:
            answer["validation"] = {"files": changed}
    if usage or caps or by_name:
        answer["state"] = "found"
        answer["hints"].extend(incomplete_file_lists(caps + by_name))
        if caps and not usage and store.has_usage:
            answer["hints"].append(
                "a curated capability lists this item but no resolved reference matches it now; "
                "macro or derive use, or a file the index does not cover, would explain that"
            )
        return answer
    if not store.has_usage:
        answer.update(_no_index_answer(f"whether {text} is used"))
        return answer
    answer["state"] = "no_reference_found"
    answer["searched"] = {
        "usage_rows": int(store.meta.get("usage_rows", 0)),
        "files": int(store.meta.get("files_scanned", 0)),
        "matched_on": [bare, f"any item ending in {leaf}"],
    }
    answer["blind_spots"] = blind_spots(ctx)
    answer["hints"].append(
        "no resolved reference or curated capability matches; this is a statement about what the "
        "usage index can see, so use inside the blind spots below is not excluded"
    )
    mentions = unindexed_mentions(ctx, leaf)
    if mentions:
        answer["validation"] = {"changed_files_mentioning": mentions}
    return answer


def _relative(ctx: Context, path: str) -> str:
    p = path.strip()
    root = str(ctx.root.resolve()) + "/"
    if p.startswith(root):
        p = p[len(root) :]
    return p.removeprefix("./")


def find_by_file(ctx: Context, path: str) -> dict:
    store = ctx.store
    rel = _relative(ctx, path)
    caps = store.capabilities_for_path(rel)
    rows = store.usage_by_file(rel)
    answer: dict[str, Any] = {
        "requested": rel,
        "capabilities": caps,
        "files": _group_by_file(rows),
        "hints": [],
    }
    exists = (ctx.root / rel).exists()
    if caps or rows:
        answer["state"] = "found"
        answer["hints"].extend(incomplete_file_lists(caps))
        changed = validate_files(
            ctx, sorted({r["file"] for r in rows} | ({rel} if rel.endswith(".rs") else set()))
        )
        if changed:
            answer["validation"] = {"files": changed}
        return answer
    if not store.has_usage:
        answer.update(_no_index_answer(f"what {rel} uses"))
        return answer
    states = store.file_states([rel])
    if rel in states:
        answer["state"] = "indexed_no_library_items"
        answer["hints"].append(
            f"{rel} was indexed and no library item resolves in it; macro or derive use inside "
            "the file would not show"
        )
    elif exists:
        answer["state"] = "not_in_usage_index"
        answer["hints"].append(
            f"{rel} exists but is not in the usage index (a file added since, or outside the "
            "module tree, for example a tests subdirectory no target includes)"
        )
    else:
        answer["state"] = "no_such_file"
        answer["hints"].append(f"{rel} does not exist in this checkout")
    answer["blind_spots"] = blind_spots(ctx)
    return answer


def search_capabilities(
    ctx: Context,
    query: str,
    lib: str | None = None,
    skill: str | None = None,
    role: str | None = None,
    status: str | None = None,
    limit: int | None = None,
) -> dict:
    libs = {r["lib"]: r for r in ctx.store.libraries()}
    matched: list[tuple[dict, float]] = []
    for cap, score in ctx.store.search(query):
        if lib and norm(cap["lib"]) != norm(lib):
            continue
        if skill and (libs.get(cap["lib"], {}).get("skill") or "") != skill:
            continue
        if role and role not in {f["role"] for f in cap.get("files", [])}:
            continue
        if status and cap.get("status") != status:
            continue
        matched.append((cap, score))
    total = len(matched)
    if limit is not None:
        matched = matched[: max(limit, 0)]
    results = [{"capability": cap, "score": round(score, 4)} for cap, score in matched]
    answer: dict[str, Any] = {
        "state": "found" if total else "no_curated_match",
        "query": query,
        "total": total,
        "results": results,
        "hints": [],
    }
    if limit is not None and total > len(results):
        answer["omitted"] = total - len(results)
    answer["hints"].extend(incomplete_file_lists([cap for cap, _ in matched]))
    if not total:
        answer["hints"].append(
            "no curated capability matches; capabilities record chosen approaches, so this does "
            "not show the workspace lacks a use of the library. find_item and library_usage "
            "answer from every resolved reference"
        )
        answer["next"] = [{"tool": "find_item", "args": {"item": query}}]
    return answer


def capabilities_by_library(ctx: Context) -> dict:
    """Every curated capability, grouped under its library; no filters, nothing omitted."""
    grouped: dict[str, list[dict]] = {}
    for record in ctx.store.capabilities():
        body = {k: v for k, v in record.items() if k not in ("kind", "lib")}
        grouped.setdefault(record["lib"], []).append(body)
    return {"libraries": {lib: {"capabilities": caps} for lib, caps in sorted(grouped.items())}}


GAP_KINDS = ("not_used", "declared_unused", "pin_delta", "no_wrapper", "dependency_without_record")


def catalog_gaps(ctx: Context, kind: str) -> dict:
    store = ctx.store
    libs = store.libraries()
    if kind == "not_used":
        rows = [
            {"lib": r["lib"], "skill": r.get("skill"), "note": r.get("note")}
            for r in libs
            if r.get("status") == "not-used"
        ]
    elif kind == "declared_unused":
        rows = [
            {"lib": r["lib"], "pin": r.get("pin")}
            for r in libs
            if r.get("status") == "declared-unused"
        ]
    elif kind == "pin_delta":
        rows = [
            {"lib": r["lib"], "skill": r.get("skill"), "delta": r["skill_pin_delta"]}
            for r in libs
            if r.get("skill_pin_delta")
        ]
    elif kind == "no_wrapper":
        rows = [
            {"id": c["id"], "lib": c["lib"], "feature": c["feature"]}
            for c in store.capabilities()
            if not c.get("wrapper")
        ]
    elif kind == "dependency_without_record":
        have = {norm(r["lib"]) for r in libs}
        rows = [{"dependency": d} for d in sorted(workspace_dependency_names(ctx.root) - have)]
    else:
        return {
            "state": "unknown_kind",
            "kinds": list(GAP_KINDS),
            "hints": [f"kind must be one of {list(GAP_KINDS)}"],
        }
    answer: dict[str, Any] = {
        "state": "found" if rows else "none",
        "kind": kind,
        "total": len(rows),
        "rows": rows,
        "hints": [],
    }
    meaning = {
        "not_used": "skill-covered libraries no workspace crate depends on: no local precedent",
        "declared_unused": "declared dependencies no indexed source references",
        "pin_delta": "libraries whose covering skill indexes a version other than ours",
        "no_wrapper": "capabilities with no recorded local abstraction to reuse",
        "dependency_without_record": (
            "workspace dependencies with no catalog record (utility crates such as serde are "
            "expected here; a missing record is not evidence of non-use)"
        ),
    }
    answer["hints"].append(meaning[kind])
    return answer


def catalog_status(ctx: Context) -> dict:
    """The catalog's contents as a listing: libraries with their status and capabilities."""
    store = ctx.store
    libs = store.libraries()
    caps = store.capabilities()
    listing: dict[str, dict[str, Any]] = {
        r["lib"]: {"status": r.get("status", ""), "capabilities": []} for r in libs
    }
    for c in caps:
        listing.setdefault(c["lib"], {"status": "", "capabilities": []})["capabilities"].append(
            {"id": c["id"], "feature": c["feature"]}
        )
    stamps = sorted({r["verified"] for r in libs + caps if r.get("verified")})
    answer: dict[str, Any] = {
        "state": "found",
        "libraries": dict(sorted(listing.items())),
        "skills": {k or "none": v for k, v in store.skills().items()},
        "verified_stamps": stamps,
        "usage_index": None,
    }
    if store.has_usage:
        answer["usage_index"] = {
            "run": store.meta.get("run_stamp"),
            "built_at": store.meta.get("built_at"),
            "packages": store.usage_packages(),
            "blind_spots": blind_spots(ctx),
        }
    else:
        answer["hints"] = ["no usage index has been built"]
    return answer
