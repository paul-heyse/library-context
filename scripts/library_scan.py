"""Stage S1 of the library utilization catalog: which files reference which dependency.

A lexical pass over Rust source: it masks comments and string/char literals, finds paths rooted at
a dependency the file's own package declares (`root::...`, `use root;`, `::root::...`, and the
`$crate::__private::root::` macro re-export form), and tags every reference as test code when it
sits in a `#[cfg(test)]` or `#[test]` item. Nothing is parsed or resolved, so it runs the same on a
tree that does not compile. It does not see re-exports
(`lctx_model::domain::__private::RecordBatch` counts for `lctx_model`, an internal crate, not for
arrow) or inferred method calls;
docs/library-utilization.md lists that as the reason for the later semantic stage.
"""

from __future__ import annotations

import re
import subprocess
from collections import defaultdict
from dataclasses import dataclass
from pathlib import Path

HACK_SUFFIX = "workspace-hack"  # cargo-hakari's package: `<repo>-workspace-hack`


def is_hack(name: str) -> bool:
    """True for the workspace-hack package, which unifies features and holds no source."""
    return name.endswith(HACK_SUFFIX)


class SetupError(RuntimeError):
    """The catalog run cannot proceed (a missing tool or skill link, a failed command)."""


SKIP_PARTS = {"target", "third_party", ".sqlx"}

_START = re.compile(r"//|/\*|(?<!\w)b?r(#*)\"|(?<!\w)b\"|\"|'")
_CHAR = re.compile(r"'(?:\\(?:x[0-9a-fA-F]{2}|u\{[0-9a-fA-F_]+\}|.)|[^\\'\n])'")
_BLANK = re.compile(r"[^\n]")
_CFG_ATTR = re.compile(r"#\s*\[\s*cfg\s*\(([^\]]*)\)\s*\]")
_TEST_ATTR = re.compile(r"#\s*\[\s*(?:\w+\s*::\s*)*test\b[^\]]*\]")
_INNER_CFG_TEST = re.compile(r"#\s*!\s*\[\s*cfg\s*\(\s*test\s*\)\s*\]")
_ATTR = re.compile(r"\s*#\s*!?\s*\[[^\]]*\]")
_PATH = re.compile(
    r"(?:(?<![\w$])(?<!\w::)|(?<=__private::))(?:::)?([A-Za-z_]\w*)\s*::(?!:)"
    r"|(?<![\w:$])use\s+(?:pub(?:\([^)]*\))?\s+)?([A-Za-z_]\w*)\s*(?:;|\bas\b)"
    r"|(?<![\w:$])extern\s+crate\s+([A-Za-z_]\w*)"
)


def mask(text: str) -> str:
    """Blank out comments and string/char literal contents, keeping length and newlines."""
    out: list[str] = []
    i, n = 0, len(text)
    while True:
        m = _START.search(text, i)
        if m is None:
            out.append(text[i:])
            return "".join(out)
        out.append(text[i : m.start()])
        token = m.group(0)
        if token == "//":
            end = text.find("\n", m.start())
            end = n if end < 0 else end
        elif token == "/*":
            depth, j = 1, m.end()
            while depth and j < n:
                opened, closed = text.find("/*", j), text.find("*/", j)
                if closed < 0:
                    j = n
                elif 0 <= opened < closed:
                    depth, j = depth + 1, opened + 2
                else:
                    depth, j = depth - 1, closed + 2
            end = j
        elif token == "'":
            char = _CHAR.match(text, m.start())
            if char is None:  # a lifetime
                out.append("'")
                i = m.end()
                continue
            end = char.end()
        elif token.lstrip("b").startswith("r"):
            close = '"' + m.group(1)
            found = text.find(close, m.end())
            end = n if found < 0 else found + len(close)
        else:
            j = m.end()
            while j < n and text[j] != '"':
                j += 2 if text[j] == "\\" else 1
            end = min(j + 1, n)
        out.append(_BLANK.sub(" ", text[m.start() : end]))
        i = end


def _item_end(masked: str, pos: int) -> int:
    """End of the item starting at `pos`: past its matching `}` or its terminating `;`."""
    depth, n = 0, len(masked)
    i = pos
    while i < n:
        c = masked[i]
        if c in "([":
            depth += 1
        elif c in ")]":
            depth -= 1
        elif c == ";" and depth <= 0:
            return i + 1
        elif c == "{" and depth <= 0:
            braces = 0
            while i < n:
                braces += masked[i] == "{"
                braces -= masked[i] == "}"
                i += 1
                if braces == 0:
                    return i
            return n
        i += 1
    return n


def _skip_attrs(masked: str, pos: int) -> int:
    while (m := _ATTR.match(masked, pos)) is not None:
        pos = m.end()
    return pos


def test_regions(masked: str) -> list[tuple[int, int]]:
    """Byte ranges of `#[cfg(test)]` and `#[test]` items; the whole text for `#![cfg(test)]`."""
    if _INNER_CFG_TEST.search(masked):
        return [(0, len(masked))]
    regions: list[tuple[int, int]] = []
    for m in _CFG_ATTR.finditer(masked):
        expr = m.group(1)
        if re.search(r"\btest\b", expr) and not re.search(r"not\s*\(\s*test\s*\)", expr):
            regions.append((m.start(), _item_end(masked, _skip_attrs(masked, m.end()))))
    for m in _TEST_ATTR.finditer(masked):
        regions.append((m.start(), _item_end(masked, _skip_attrs(masked, m.end()))))
    return regions


def path_role(rel: Path) -> str:
    """The role a file has by its location: dormant, test, build or src."""
    parts = rel.parts
    if "dormant" in parts and "tests" in parts:
        return "dormant"
    if {"tests", "examples", "benches"} & set(parts) or rel.stem == "tests":
        return "test"
    if rel.name == "build.rs":
        return "build"
    return "src"


def scan_text(text: str, idents: dict[str, str]) -> dict[str, list[bool]]:
    """Per dependency key, one flag per reference: True when it sits in test code."""
    masked = mask(text)
    regions = test_regions(masked)
    found: dict[str, list[bool]] = defaultdict(list)
    for m in _PATH.finditer(masked):
        ident = m.group(1) or m.group(2) or m.group(3)
        key = idents.get(ident)
        if key is not None:
            pos = m.start()
            found[key].append(any(a <= pos < b for a, b in regions))
    return found


@dataclass(frozen=True)
class Ref:
    """One file's references to one dependency."""

    package: str
    path: str
    role: str  # src, build, test or dormant
    count: int


def file_role(path_role_: str, flags: list[bool]) -> str:
    """A src file whose references are all in test items counts as test."""
    if path_role_ == "src" and flags and all(flags):
        return "test"
    return path_role_


def package_idents(meta: dict) -> dict[str, dict[str, str]]:
    """Per workspace package, the identifier code uses for each direct dependency -> its key."""
    members = {p["name"] for p in meta["packages"]}
    out: dict[str, dict[str, str]] = {}
    for package in meta["packages"]:
        if is_hack(package["name"]):
            continue
        idents: dict[str, str] = {}
        for dep in package["dependencies"]:
            if dep["name"] in members or dep.get("path"):
                continue
            key = dep.get("rename") or dep["name"]
            idents[key.replace("-", "_")] = key
        out[package["name"]] = idents
    return out


def workspace_files(root: Path, meta: dict) -> list[Path]:
    """Every `.rs` file of every workspace package, repo-relative, minus vendored and build dirs."""
    found: list[Path] = []
    for package in sorted(meta["packages"], key=lambda p: p["name"]):
        if is_hack(package["name"]):
            continue
        for path in sorted(Path(package["manifest_path"]).parent.rglob("*.rs")):
            rel = path.relative_to(root)
            if not SKIP_PARTS & set(rel.parts):
                found.append(rel)
    return found


def scan_workspace(root: Path, meta: dict) -> dict[str, list[Ref]]:
    """References per dependency key over every `.rs` file of every workspace package."""
    idents = package_idents(meta)
    refs: dict[str, list[Ref]] = defaultdict(list)
    for package in sorted(meta["packages"], key=lambda p: p["name"]):
        name = package["name"]
        if is_hack(name):
            continue
        pkg_dir = Path(package["manifest_path"]).parent
        for path in sorted(pkg_dir.rglob("*.rs")):
            rel = path.relative_to(root)
            if SKIP_PARTS & set(rel.parts):
                continue
            found = scan_text(path.read_text(errors="replace"), idents[name])
            for key, flags in found.items():
                role = file_role(path_role(path.relative_to(pkg_dir)), flags)
                refs[key].append(Ref(name, rel.as_posix(), role, len(flags)))
    return dict(refs)


def status_of(refs: list[Ref]) -> str:
    """used (production code), test-only, dormant-only (uncompiled tests) or declared-unused."""
    roles = {r.role for r in refs}
    if roles & {"src", "build"}:
        return "used"
    if "test" in roles:
        return "test-only"
    if "dormant" in roles:
        return "dormant-only"
    return "declared-unused"


def used_in(refs: list[Ref]) -> dict[str, dict[str, int]]:
    """Files per workspace package by role; build counts as src; packages with none omitted."""
    out: dict[str, dict[str, int]] = {}
    for ref in sorted(refs, key=lambda r: (r.package, r.path)):
        role = "src" if ref.role == "build" else ref.role
        bucket = out.setdefault(ref.package, {})
        bucket[role] = bucket.get(role, 0) + 1
    return out


def rg_files(root: Path, meta: dict, key: str) -> set[str] | None:
    """Independent file set for `key` by ripgrep over the workspace packages' directories."""
    ident = key.replace("-", "_")
    dirs = sorted(
        {
            Path(p["manifest_path"]).parent.relative_to(root).as_posix()
            for p in meta["packages"]
            if not is_hack(p["name"])
        }
    )
    pattern = rf"(?<![\w:$])(?:::)?{ident}\s*::|(?<![\w:$])use\s+{ident}\s*;|__private::{ident}\b"
    try:
        proc = subprocess.run(
            ["rg", "-l", "-P", "--glob", "*.rs", pattern, *dirs],
            cwd=root,
            capture_output=True,
            text=True,
            check=False,
        )
    except FileNotFoundError:
        return None
    return {line for line in proc.stdout.splitlines() if not SKIP_PARTS & set(Path(line).parts)}
