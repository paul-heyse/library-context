"""ADR tooling: new, supersede, index, lint, revisit.

Records live in docs/adr/NNNN-slug.md with a small frontmatter block:

    ---
    id: ADR-0007
    title: Short decision statement
    status: proposed | accepted | superseded | rejected
    date: 2026-09-22
    supersedes: [ADR-0003]
    superseded-by: null
    design: [§B3, §4.2]
    evidence: Proposed            # optional, charter §D label
    revisit: $ just deps          # optional; a leading "$ " makes it runnable
    ---

DESIGN.md is the current truth; an ADR is the why. `lint` checks that every
`design:` ref resolves to a DESIGN.md heading, that supersession links are
symmetric, that accepted records have not been edited apart from their status
fields and an append-only trailing `## Amendments` section, and that
docs/adr/README.md is current. Stdlib only.
"""

from __future__ import annotations

import argparse
import datetime as dt
import re
import subprocess
import sys
from dataclasses import dataclass, field
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
STATUSES = ("proposed", "accepted", "superseded", "rejected")
ACTIVE = ("proposed", "accepted")
EVIDENCE = (
    "Proposed",
    "Interface-checked",
    "Implemented",
    "Tested",
    "Measured",
    "Formally established",
)
REQUIRED = ("id", "title", "status", "date", "supersedes", "superseded-by", "design")
# Once a record is accepted, only these lines may change, and dated lines may be
# appended under a trailing AMENDMENTS heading (factual corrections, not new decisions).
MUTABLE_WHEN_ACCEPTED = ("status", "superseded-by")
AMENDMENTS = "## Amendments"
FILE_RE = re.compile(r"^(\d{4})-[a-z0-9][a-z0-9-]*\.md$")
HEADING_RE = re.compile(r"^#+\s+§([A-Z]?\d+(?:\.\d+)*)\b", re.M)


@dataclass
class Adr:
    path: Path
    meta: dict[str, str | list[str] | None]
    body: str
    raw: str
    errors: list[str] = field(default_factory=list)

    @property
    def id(self) -> str:
        return str(self.meta.get("id") or "")

    @property
    def status(self) -> str:
        return str(self.meta.get("status") or "")

    def refs(self, key: str) -> list[str]:
        value = self.meta.get(key)
        if value is None:
            return []
        return value if isinstance(value, list) else [value]


def adr_dir(root: Path) -> Path:
    return root / "docs" / "adr"


def parse_value(text: str) -> str | list[str] | None:
    text = text.strip()
    if text in ("", "null", "~"):
        return None
    if text.startswith("[") and text.endswith("]"):
        inner = text[1:-1].strip()
        return [item.strip() for item in inner.split(",") if item.strip()] if inner else []
    return text


def parse(path: Path, raw: str | None = None) -> Adr:
    raw = path.read_text() if raw is None else raw
    adr = Adr(path=path, meta={}, body="", raw=raw)
    if not raw.startswith("---\n"):
        adr.errors.append("missing frontmatter")
        return adr
    end = raw.find("\n---\n", 4)
    if end == -1:
        adr.errors.append("unterminated frontmatter")
        return adr
    for line in raw[4:end].splitlines():
        if not line.strip() or line.lstrip().startswith("#"):
            continue
        key, sep, value = line.partition(":")
        if not sep:
            adr.errors.append(f"frontmatter line is not key: value: {line!r}")
            continue
        adr.meta[key.strip()] = parse_value(value)
    adr.body = raw[end + 5 :]
    return adr


def load_all(root: Path) -> list[Adr]:
    return [parse(p) for p in sorted(adr_dir(root).glob("[0-9][0-9][0-9][0-9]-*.md"))]


def design_ids(root: Path) -> set[str]:
    design = root / "docs" / "design" / "DESIGN.md"
    return set(HEADING_RE.findall(design.read_text())) if design.exists() else set()


def git_head_text(root: Path, path: Path) -> str | None:
    rel = path.relative_to(root).as_posix()
    result = subprocess.run(
        ["git", "-C", str(root), "show", f"HEAD:{rel}"],
        capture_output=True,
        text=True,
        check=False,
    )
    return result.stdout if result.returncode == 0 else None


def split_amendments(raw: str) -> tuple[str, str]:
    """Split off a trailing `## Amendments` section (append-only once accepted)."""
    head, sep, tail = raw.partition(f"\n{AMENDMENTS}\n")
    return (head, tail.strip()) if sep else (raw, "")


def strip_mutable(raw: str) -> str:
    keep = [
        line
        for line in split_amendments(raw)[0].rstrip().splitlines()
        if not any(line.startswith(f"{key}:") for key in MUTABLE_WHEN_ACCEPTED)
    ]
    return "\n".join(keep)


def edited_after_acceptance(head: str, current: str) -> bool:
    """True unless only status fields changed or amendments were appended."""
    if strip_mutable(head) != strip_mutable(current):
        return True
    return not split_amendments(current)[1].startswith(split_amendments(head)[1])


def render_index(adrs: list[Adr]) -> str:
    lines = [
        "# Architecture decision records",
        "",
        "Generated by `just adr index` — do not edit by hand.",
        "`docs/design/DESIGN.md` is the current truth; these records are the why.",
        "",
        "## Active",
        "",
        "| ADR | Title | Status | Date | Design | Evidence |",
        "|---|---|---|---|---|---|",
    ]
    for a in adrs:
        if a.status in ACTIVE:
            lines.append(
                f"| [{a.id}]({a.path.name}) | {a.meta.get('title', '')} | {a.status} "
                f"| {a.meta.get('date', '')} | {', '.join(a.refs('design'))} "
                f"| {a.meta.get('evidence') or ''} |"
            )
    retired = [a for a in adrs if a.status not in ACTIVE]
    lines += ["", "## Superseded and rejected", ""]
    if not retired:
        lines.append("None yet.")
    else:
        lines += ["| ADR | Title | Status | Replaced by |", "|---|---|---|---|"]
        for a in retired:
            lines.append(
                f"| [{a.id}]({a.path.name}) | {a.meta.get('title', '')} | {a.status} "
                f"| {', '.join(a.refs('superseded-by'))} |"
            )
    return "\n".join(lines) + "\n"


def lint(root: Path, *, check_git: bool = True) -> list[str]:
    adrs = load_all(root)
    ids = design_ids(root)
    by_id = {a.id: a for a in adrs}
    problems: list[str] = []

    def err(adr: Adr, message: str) -> None:
        problems.append(f"{adr.path.relative_to(root)}: {message}")

    for a in adrs:
        for e in a.errors:
            err(a, e)
        match = FILE_RE.match(a.path.name)
        for key in REQUIRED:
            if key not in a.meta:
                err(a, f"missing field `{key}`")
        if match and a.id != f"ADR-{match.group(1)}":
            err(a, f"id {a.id!r} does not match filename")
        if a.status not in STATUSES:
            err(a, f"status {a.status!r} not in {STATUSES}")
        try:
            dt.date.fromisoformat(str(a.meta.get("date")))
        except ValueError:
            err(a, f"date {a.meta.get('date')!r} is not YYYY-MM-DD")
        evidence = a.meta.get("evidence")
        if evidence is not None and evidence not in EVIDENCE:
            err(a, f"evidence {evidence!r} is not a charter §D label")
        for ref in a.refs("design"):
            if not ref.startswith("§") or ref[1:] not in ids:
                err(a, f"design ref {ref} does not resolve to a DESIGN.md heading")
        superseded_by = a.refs("superseded-by")
        if (a.status == "superseded") != bool(superseded_by):
            err(a, "status `superseded` and `superseded-by` must be set together")
        for other in superseded_by:
            if other not in by_id:
                err(a, f"superseded-by {other} does not exist")
            elif a.id not in by_id[other].refs("supersedes"):
                err(a, f"{other} does not list {a.id} in `supersedes`")
        for old in a.refs("supersedes"):
            if old not in by_id:
                err(a, f"supersedes {old} which does not exist")
            elif a.id not in by_id[old].refs("superseded-by"):
                err(a, f"{old} does not name {a.id} in `superseded-by`")
        if check_git:
            head = git_head_text(root, a.path)
            accepted_at_head = head is not None and parse(a.path, head).status == "accepted"
            if accepted_at_head and edited_after_acceptance(head or "", a.raw):
                err(
                    a,
                    "accepted record edited beyond status/superseded-by or appended amendments; "
                    "write a superseding ADR instead (`just adr supersede`)",
                )

    problems.extend(design_decisions(root, adrs))

    index = adr_dir(root) / "README.md"
    if adrs and (not index.exists() or index.read_text() != render_index(adrs)):
        problems.append("docs/adr/README.md is stale: run `just adr index`")
    return problems


SECTION_RE = re.compile(r"^(#+)\s+§([A-Z]?\d+(?:\.\d+)*)\b")
CITED_RE = re.compile(r"ADR-(\d{4})")


def design_decisions(root: Path, adrs: list[Adr]) -> list[str]:
    """DESIGN.md and the ADRs agree on who decides what (ADR-0019 review F2, F5).

    - Every section an active record lists in `design:` ends, itself or through an enclosing
      section, with a `> Decision:` line naming the record.
    - Every `ADR-NNNN` DESIGN.md cites exists, unless the citation says it is "to be written".
    """
    design = root / "docs" / "design" / "DESIGN.md"
    if not design.exists():
        return []
    lines = design.read_text().splitlines()
    heads = [
        (i, len(m.group(1)), m.group(2))
        for i, line in enumerate(lines)
        if (m := SECTION_RE.match(line))
    ]

    def end(k: int) -> int:
        level = heads[k][1]
        return next((j for j, lv, _ in heads[k + 1 :] if lv <= level), len(lines))

    def spans(ref: str) -> list[tuple[int, int]]:
        for k, (i, level, sid) in enumerate(heads):
            if sid != ref:
                continue
            out = [(i, end(k))]
            for kk in range(k - 1, -1, -1):
                if heads[kk][1] < level:
                    out.append((heads[kk][0], end(kk)))
                    level = heads[kk][1]
            return out
        return []

    problems = []
    for a in adrs:
        if a.status not in ACTIVE:
            continue
        for ref in a.refs("design"):
            found = spans(ref[1:])
            if found and not any(
                line.startswith("> Decision:") and a.id in line
                for s, e in found
                for line in lines[s:e]
            ):
                problems.append(
                    f"DESIGN.md {ref}: no `> Decision:` line names {a.id}, which lists it"
                )
    known = {a.id for a in adrs}
    text = "\n".join(lines)
    for m in CITED_RE.finditer(text):
        cited = f"ADR-{m.group(1)}"
        if cited not in known and "to be written" not in text[m.end() : m.end() + 40]:
            problems.append(f"DESIGN.md cites {cited}, which does not exist")
    return sorted(set(problems))


def next_number(root: Path) -> int:
    numbers = [int(p.name[:4]) for p in adr_dir(root).glob("[0-9][0-9][0-9][0-9]-*.md")]
    return max(numbers, default=0) + 1


def create(root: Path, slug: str, title: str, supersedes: list[str]) -> Path:
    if not re.fullmatch(r"[a-z0-9][a-z0-9-]*", slug):
        raise SystemExit(f"slug must be kebab-case: {slug!r}")
    number = next_number(root)
    template = (adr_dir(root) / "TEMPLATE.md").read_text()
    text = (
        template.replace("ADR-NNNN", f"ADR-{number:04d}")
        .replace("title: TITLE", f"title: {title}")
        .replace("date: YYYY-MM-DD", f"date: {dt.date.today().isoformat()}")
        .replace("supersedes: []", f"supersedes: [{', '.join(supersedes)}]")
    )
    path = adr_dir(root) / f"{number:04d}-{slug}.md"
    path.write_text(text)
    return path


def set_field(path: Path, key: str, value: str) -> None:
    lines = path.read_text().splitlines(keepends=True)
    for i, line in enumerate(lines):
        if line.startswith(f"{key}:"):
            lines[i] = f"{key}: {value}\n"
            break
    else:
        raise SystemExit(f"{path}: no `{key}` field")
    path.write_text("".join(lines))


def cmd_new(args: argparse.Namespace) -> int:
    path = create(args.root, args.slug, args.title or args.slug.replace("-", " "), [])
    print(path.relative_to(args.root))
    return 0


def cmd_supersede(args: argparse.Namespace) -> int:
    by_id = {a.id: a for a in load_all(args.root)}
    old = by_id.get(args.old)
    if old is None:
        raise SystemExit(f"no such ADR: {args.old}")
    path = create(args.root, args.slug, args.title or args.slug.replace("-", " "), [old.id])
    new_id = f"ADR-{path.name[:4]}"
    set_field(old.path, "status", "superseded")
    set_field(old.path, "superseded-by", new_id)
    print(f"{path.relative_to(args.root)} supersedes {old.id}")
    return 0


def cmd_index(args: argparse.Namespace) -> int:
    (adr_dir(args.root) / "README.md").write_text(render_index(load_all(args.root)))
    return 0


def cmd_lint(args: argparse.Namespace) -> int:
    problems = lint(args.root, check_git=not args.no_git)
    for p in problems:
        print(p)
    if not problems:
        print(f"adr lint: ok ({len(load_all(args.root))} records)")
    return 1 if problems else 0


def cmd_revisit(args: argparse.Namespace) -> int:
    failed = False
    for a in load_all(args.root):
        trigger = a.meta.get("revisit")
        if a.status not in ACTIVE or not isinstance(trigger, str):
            continue
        if trigger.startswith("$ "):
            result = subprocess.run(
                trigger[2:], shell=True, cwd=args.root, capture_output=True, check=False
            )
            state = "passed" if result.returncode == 0 else "failed"
            failed |= result.returncode != 0
            print(f"{a.id}: {state}: {trigger}")
        else:
            print(f"{a.id}: manual: {trigger}")
    return 1 if failed else 0


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="ADR tooling: new, supersede, index, lint, revisit."
    )
    parser.add_argument("--root", type=Path, default=ROOT, help=argparse.SUPPRESS)
    sub = parser.add_subparsers(dest="command", required=True)
    p = sub.add_parser("new", help="create docs/adr/NNNN-<slug>.md")
    p.add_argument("slug")
    p.add_argument("--title")
    p.set_defaults(func=cmd_new)
    p = sub.add_parser("supersede", help="new ADR that supersedes OLD")
    p.add_argument("old", help="e.g. ADR-0003")
    p.add_argument("slug")
    p.add_argument("--title")
    p.set_defaults(func=cmd_supersede)
    sub.add_parser("index", help="regenerate docs/adr/README.md").set_defaults(func=cmd_index)
    p = sub.add_parser("lint", help="validate records, refs, links, immutability, index")
    p.add_argument("--no-git", action="store_true", help="skip the accepted-record diff")
    p.set_defaults(func=cmd_lint)
    sub.add_parser("revisit", help="list revisit triggers; run `$ ` ones").set_defaults(
        func=cmd_revisit
    )
    args = parser.parse_args(argv)
    return args.func(args)


if __name__ == "__main__":
    sys.exit(main())
