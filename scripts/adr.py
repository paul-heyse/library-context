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
    evidence: Proposed            # optional, principles §D label
    revisit: $ just deps          # optional; a leading "$ " makes it runnable
    ---

The architectural collection is current truth; an ADR is the why (ADR-0042 lifecycle).
`lint` checks that every `design:` ref resolves to a stable section owner, that governing
references and retained supersession links resolve, that accepted records have not been edited
apart from their status fields and an append-only trailing `## Amendments` section, and that
docs/adr/README.md is current. A `supersedes` entry naming a retired (deleted) record is
historical provenance, recovered from Git. `new` allocates above every number in the tree and in
reachable Git history, so a retired id is never reused. Stdlib only.
"""

from __future__ import annotations

import argparse
import datetime as dt
import re
import subprocess
import sys
from dataclasses import dataclass, field
from pathlib import Path

import design_sections

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
    return set(design_sections.resolve(root))


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


RECOVERY = "../README.md#historical-recovery"


def section_links(root: Path, adr: Adr) -> str:
    """Route each governed section to its owner; an unresolved ref stays plain for lint."""
    try:
        owners = design_sections.resolve(root)
    except ValueError:
        owners = {}
    out = []
    for ref in adr.refs("design"):
        section = owners.get(ref[1:]) if ref.startswith("§") else None
        if section is None:
            out.append(ref)
            continue
        target = section.path.relative_to(adr_dir(root).parent).as_posix()
        out.append(f"[{ref}](../{target}#{design_sections.anchor(section.ident)})")
    return ", ".join(out)


def record_links(ids: list[str], by_id: dict[str, Adr]) -> str:
    """Retained records link; retired ones stay historical identifiers (no broken links)."""
    return ", ".join(f"[{i}]({by_id[i].path.name})" if i in by_id else i for i in ids)


def render_index(root: Path, adrs: list[Adr]) -> str:
    by_id = {a.id: a for a in adrs}
    lines = [
        "# Architecture decision records",
        "",
        "Generated by `just adr index` — do not edit by hand.",
        "The [architecture map](../design/README.md) and its section owners own contracts and "
        "targets; these records own current reasons, rejected alternatives and open choices "
        "(ADR-0042). Read the record that governs the section you are changing.",
    ]
    groups = [
        ("Accepted", "accepted"),
        ("Proposed: open choices, not accepted decisions", "proposed"),
        ("Superseded, still in the tree", "superseded"),
        ("Rejected", "rejected"),
    ]
    retired: set[str] = set()
    for heading, status in groups:
        rows = [a for a in adrs if a.status == status]
        if not rows:
            continue
        lines += [
            "",
            f"## {heading}",
            "",
            "| ADR | Title | Date | Governs | Evidence | Replaces |",
            "|---|---|---|---|---|---|",
        ]
        for a in rows:
            retired.update(i for i in a.refs("supersedes") if i not in by_id)
            lines.append(
                f"| [{a.id}]({a.path.name}) | {a.meta.get('title', '')} "
                f"| {a.meta.get('date', '')} | {section_links(root, a)} "
                f"| {a.meta.get('evidence') or ''} | {record_links(a.refs('supersedes'), by_id)} |"
            )
    if retired:
        lines += [
            "",
            "Unlinked identifiers under *Replaces* are retired records. Their surviving meaning is "
            f"in the records above; recover the text from Git ([historical recovery]({RECOVERY})).",
        ]
    return "\n".join(lines) + "\n"


def lint(root: Path, *, check_git: bool = True) -> list[str]:
    adrs = load_all(root)
    try:
        ids = design_ids(root)
    except ValueError as error:
        return [str(error)]
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
            err(a, f"evidence {evidence!r} is not a principles §D label")
        for ref in a.refs("design"):
            if not ref.startswith("§") or ref[1:] not in ids:
                err(a, f"design ref {ref} does not resolve to a DESIGN.md heading")
        superseded_by = a.refs("superseded-by")
        if (a.status == "superseded") != bool(superseded_by):
            err(a, "status `superseded` and `superseded-by` must be set together")
        for other in superseded_by:
            # A retained superseded record must lead to a retained successor.
            if other not in by_id:
                err(a, f"superseded-by {other} does not exist; retire {a.id} or keep {other}")
            elif a.id not in by_id[other].refs("supersedes"):
                err(a, f"{other} does not list {a.id} in `supersedes`")
        for old in a.refs("supersedes"):
            # A retired predecessor is provenance, recovered from Git (ADR-0042), but only an
            # accepted replacement can hold the decisions it retired.
            if old in by_id and a.id not in by_id[old].refs("superseded-by"):
                err(a, f"{old} does not name {a.id} in `superseded-by`")
            elif old not in by_id and a.status != "accepted":
                err(
                    a,
                    f"supersedes retired {old} but is {a.status}; accept it before retiring {old}",
                )
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
    if adrs and (not index.exists() or index.read_text() != render_index(root, adrs)):
        problems.append("docs/adr/README.md is stale: run `just adr index`")
    return problems


SECTION_RE = re.compile(r"^(#+)\s+§([A-Z]?\d+(?:\.\d+)*)\b")
CITED_RE = re.compile(r"ADR-(\d{4})")
CLAIM_RE = re.compile(r"ADR-(\d{4}),? (?:is |stays )?(accepted|proposed)\b")
REVISION_RE = re.compile(r"^\| \d{4}-\d{2}-\d{2} \|")


def design_decisions(root: Path, adrs: list[Adr]) -> list[str]:
    """DESIGN.md and the ADRs agree on who decides what (ADR-0019 review F2, F5).

    - Every section an active record lists in `design:` ends, itself or through an enclosing
      section, with a `> Decision:` line naming the record.
    - Every `ADR-NNNN` DESIGN.md cites exists, unless the citation says it is "to be written".
    - A sentence outside the revision rows saying "ADR-NNNN accepted" or "proposed" matches it.
    """
    owners = design_sections.resolve(root)
    sources = {
        p: design_sections.prose_lines(p.read_text()) for p in design_sections.source_paths(root)
    }
    problems = []
    for a in adrs:
        if a.status not in ACTIVE:
            continue
        for ref in a.refs("design"):
            found = design_sections.governing_sections(ref[1:], owners)
            if found and not any(
                line.startswith("> Decision:") and a.id in line
                for section in found
                for line in design_sections.own_lines(section, sources[section.path])
            ):
                problems.append(
                    f"DESIGN.md {ref}: no `> Decision:` line names {a.id}, which lists it"
                )
    status = {a.id: a.status for a in adrs}
    for path, lines in sources.items():
        label = "DESIGN.md" if path.name == "DESIGN.md" else path.relative_to(root).as_posix()
        for n, line in enumerate(lines, 1):
            if REVISION_RE.match(line):
                continue
            for m in CLAIM_RE.finditer(line):
                cited = f"ADR-{m.group(1)}"
                if cited in status and status[cited] != m.group(2):
                    problems.append(
                        f"{label}:{n} says {cited} is {m.group(2)}, but it is {status[cited]}"
                    )
        text = "\n".join(lines)
        for m in CITED_RE.finditer(text):
            cited = f"ADR-{m.group(1)}"
            if cited not in status and "to be written" not in text[m.end() : m.end() + 40]:
                problems.append(f"{label} cites {cited}, which does not exist")
    return sorted(set(problems))


HISTORY_RE = re.compile(r"(?:^|/)docs/adr/(\d{4})-[^/]*\.md$")


def git(root: Path, *argv: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        ["git", "-C", str(root), *argv], capture_output=True, text=True, check=False
    )


def historical_numbers(root: Path) -> list[int]:
    """Numbers of every ADR path in reachable history, deleted records included.

    Only record creation reads history, so lint and builds work in a shallow checkout. An
    isolated non-Git directory has no history to reuse. A shallow or unreadable history fails
    rather than silently reusing a retired id.
    """
    inside = git(root, "rev-parse", "--is-inside-work-tree")
    if inside.returncode != 0 or inside.stdout.strip() != "true":
        return []
    if git(root, "rev-parse", "--verify", "--quiet", "HEAD").returncode != 0:
        return []  # no commits yet
    if git(root, "rev-parse", "--is-shallow-repository").stdout.strip() != "false":
        raise SystemExit(
            "ADR history is shallow or unknown; run `git fetch --unshallow` so a new record "
            "cannot reuse a retired ADR number"
        )
    log = git(root, "log", "--all", "--format=", "--name-only", "--", "docs/adr")
    if log.returncode != 0:
        raise SystemExit(f"cannot read ADR history: {log.stderr.strip()}")
    return [int(m[1]) for line in log.stdout.splitlines() if (m := HISTORY_RE.search(line))]


def next_number(root: Path) -> int:
    numbers = [int(p.name[:4]) for p in adr_dir(root).glob("[0-9][0-9][0-9][0-9]-*.md")]
    return max(numbers + historical_numbers(root), default=0) + 1


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
    (adr_dir(args.root) / "README.md").write_text(render_index(args.root, load_all(args.root)))
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
