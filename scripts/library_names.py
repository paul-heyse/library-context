"""Stage S3 of the library utilization catalog: canonical item paths from the skills' indexes.

Each enabled Rust skill ships `content/index/{symbols,aliases,methods}.tsv`: the defining path
of every public item, the paths it is reachable by (re-exports, facades) and its methods. A
capability's `items` are meant to be defining paths, so this resolves each recorded item against
that union index:

- `canonical`: already a defining path.
- `alias`: a re-export or facade path; rewritten to the defining path.
- `name`: unknown path, but exactly one item in the same crate has that last segment (a crate-root
  re-export such as `arrow_schema::Schema`); rewritten to it.
- `method`: `Type::method`, with `Type` resolved by the rules above and the method listed in
  `methods.tsv`.
- `ambiguous` / `absent`: the crate is indexed but the item is not (or is one of several): the
  skill's index does not list it (a private or fork-patched item, a name from another version).
- `unindexed`: no skill indexes the crate (sqlx, pyo3, ...); left as recorded.

Nothing here needs a build. Resolution is relative to the version each skill indexes, which
`skill_pin_delta` on the library record says when it differs from ours.
"""

from __future__ import annotations

import tomllib
from collections import defaultdict
from dataclasses import dataclass
from pathlib import Path

NON_ITEMS = {"module", "use"}


@dataclass
class Index:
    symbols: dict[str, str]  # defining path -> kind
    aliases: dict[str, str]  # reachable path -> defining path
    methods: set[tuple[str, str]]  # (defining type path, method name)
    by_name: dict[tuple[str, str], list[str]]  # (crate, last segment) -> defining paths
    crates: set[str]


@dataclass(frozen=True)
class Resolved:
    item: str
    canonical: str | None
    how: str  # canonical, alias, name, method, ambiguous, absent, unindexed


def _rows(path: Path) -> list[list[str]]:
    return [line.split("\t") for line in path.read_text().splitlines() if line]


def load_index(root: Path) -> Index:
    """Union of the enabled skills' symbol, alias and method indexes (crate-prefixed paths)."""
    enabled = tomllib.loads((root / ".config" / "library-skills.toml").read_text())["enabled"]
    symbols: dict[str, str] = {}
    aliases: dict[str, str] = {}
    methods: set[tuple[str, str]] = set()
    for skill in enabled:
        directory = root / ".claude" / "skills" / skill / "content" / "index"
        if (directory / "symbols.tsv").exists():
            for row in _rows(directory / "symbols.tsv"):
                symbols[row[0]] = row[1]
        if (directory / "aliases.tsv").exists():
            for row in _rows(directory / "aliases.tsv"):
                aliases[row[0]] = row[1]
        if (directory / "methods.tsv").exists():
            methods.update((row[0], row[1]) for row in _rows(directory / "methods.tsv"))
    by_name: dict[tuple[str, str], list[str]] = defaultdict(list)
    for path, kind in symbols.items():
        if kind not in NON_ITEMS:
            by_name[(path.split("::")[0], path.rsplit("::", 1)[-1])].append(path)
    crates = {path.split("::")[0] for path in symbols} | {a.split("::")[0] for a in aliases}
    return Index(symbols, aliases, methods, dict(by_name), crates)


def _exact(path: str, index: Index) -> tuple[str | None, str]:
    if path in index.symbols:
        return path, "canonical"
    if path in index.aliases:
        return index.aliases[path], "alias"
    return None, "absent"


def _by_name(path: str, index: Index) -> tuple[str | None, str]:
    """The one item of the same crate with this last segment, if there is exactly one."""
    found = index.by_name.get((path.split("::")[0], path.rsplit("::", 1)[-1]), [])
    if len(found) == 1:
        return found[0], "name"
    return None, "ambiguous" if found else "absent"


def _type(path: str, index: Index) -> tuple[str | None, str]:
    canonical, how = _exact(path, index)
    return (canonical, how) if canonical else _by_name(path, index)


def resolve(item: str, index: Index) -> Resolved:
    """Resolve one recorded item; a trailing `!` (macro) is kept.

    Order: an exact path or alias; `Type::method` with the type resolved and the method listed;
    a unique same-crate name. The method reading comes before the name reading so a method is
    never mistaken for a free function of the same name.
    """
    bang = "!" if item.endswith("!") else ""
    path = item.removesuffix("!")
    if path.split("::")[0] not in index.crates:
        return Resolved(item, None, "unindexed")
    canonical, how = _exact(path, index)
    if canonical:
        return Resolved(item, canonical + bang, how)
    owner, sep, method = path.rpartition("::")
    if sep:
        typ, owner_how = _type(owner, index)
        if typ is not None and (typ, method) in index.methods:
            return Resolved(
                item, f"{typ}::{method}", "method" if owner_how == "canonical" else owner_how
            )
    canonical, how = _by_name(path, index)
    return Resolved(item, canonical + bang if canonical else None, how)


def normalize_items(
    items: list[str], index: Index, keep: frozenset[str] | set[str] = frozenset()
) -> tuple[list[str], list[str], list[Resolved]]:
    """Return the normalized items, the spellings replaced, and every resolution.

    An item in `keep` (a path rust-analyzer resolved exactly) is already a defining path as the
    compiler sees it, so the skills' index does not rewrite it.
    """
    resolved = [
        Resolved(item, None, "resolved") if item.removesuffix("!") in keep else resolve(item, index)
        for item in items
    ]
    out: list[str] = []
    replaced: list[str] = []
    for r in resolved:
        target = r.canonical or r.item
        if target != r.item:
            replaced.append(r.item)
        if target not in out:
            out.append(target)
    return out, replaced, resolved
