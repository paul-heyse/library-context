"""Stable section ownership for the authoritative architectural collection (stdlib only)."""

from __future__ import annotations

import re
from dataclasses import dataclass
from pathlib import Path

HEADING = re.compile(r"^ {0,3}(#{1,6})\s+(.+?)(?:\s+#+)?\s*$")
IDENT = re.compile(r"§([A-Z]?\d+(?:\.\d+)*)\b")
RELOCATED = "<!-- relocated-section -->"


def prose_lines(text: str) -> list[str]:
    """Blank fenced code while retaining original line positions."""
    lines = text.splitlines()
    fence = ""
    size = 0
    out = []
    for line in lines:
        match = re.match(r"^ {0,3}(`{3,}|~{3,})(.*)$", line)
        if fence:
            if match and match[1][0] == fence and len(match[1]) >= size and not match[2].strip():
                fence = ""
            out.append("")
        elif match:
            fence, size = match[1][0], len(match[1])
            out.append("")
        else:
            out.append(line)
    return out


def headings(text: str) -> list[tuple[int, int, str]]:
    return [
        (i, len(m[1]), m[2])
        for i, line in enumerate(prose_lines(text))
        if (m := HEADING.match(line))
    ]


def source_paths(root: Path) -> list[Path]:
    base = root / "docs/design"
    return [
        p for p in [base / "DESIGN.md", *sorted((base / "sections").rglob("*.md"))] if p.is_file()
    ]


@dataclass(frozen=True)
class Section:
    ident: str
    path: Path
    start: int  # zero-based source line, inclusive
    end: int  # exclusive
    title: str
    parents: tuple[str, ...]


def resolve(root: Path) -> dict[str, Section]:
    owners: dict[str, Section] = {}
    for path in source_paths(root):
        text = path.read_text()
        lines = prose_lines(text)
        heads = headings(text)
        stack: list[tuple[int, str]] = []
        for k, (start, level, title) in enumerate(heads):
            while stack and stack[-1][0] >= level:
                stack.pop()
            match = IDENT.match(title)
            if not match:
                continue
            ident = match[1]
            end = next((i for i, lv, _ in heads[k + 1 :] if lv <= level), len(lines))
            following = next((line.strip() for line in lines[start + 1 : end] if line.strip()), "")
            if following == RELOCATED:
                continue
            parents = tuple(s for _, s in reversed(stack))
            if not parents and "." in ident:
                parents = tuple(ident.rsplit(".", n)[0] for n in range(1, ident.count(".") + 1))
            elif not parents and ident.startswith("B"):
                parents = ("2",)
            if ident in owners:
                old = owners[ident]
                raise ValueError(
                    f"duplicate section §{ident}: {old.path}:{old.start + 1} and {path}:{start + 1}"
                )
            owners[ident] = Section(ident, path, start, end, title, parents)
            stack.append((level, ident))
    return owners


def governing_sections(ident: str, owners: dict[str, Section]) -> list[Section]:
    pending = [ident]
    seen = set()
    result = []
    while pending:
        key = pending.pop(0)
        if key in seen or key not in owners:
            continue
        seen.add(key)
        section = owners[key]
        result.append(section)
        pending.extend(section.parents)
    return result


def anchor(ident: str) -> str:
    return "section-" + ident.lower().replace(".", "-")


def own_lines(section: Section, lines: list[str]) -> list[str]:
    """Governing prose belongs to this section, never a sibling/child section."""
    end = next(
        (
            i
            for i in range(section.start + 1, section.end)
            if (heading := HEADING.match(lines[i])) and IDENT.match(heading[2])
        ),
        section.end,
    )
    return lines[section.start : end]
