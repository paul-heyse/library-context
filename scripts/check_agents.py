"""Check that agent instructions and skills resolve, for Claude Code and Codex alike.

- `.agents/skills` (Codex discovery) is a symlink to `.claude/skills`.
- Every skill's frontmatter `name` matches its directory.
- In the agent-facing documents, every relative markdown link and every
  backticked repo path exists, and every `just <recipe>` names a real recipe.

Instructions that point at things which do not exist are how the previous
repository's agents were sent after deleted commands; this is the oracle.
"""

from __future__ import annotations

import re
import subprocess
import sys
from pathlib import Path

from repo_paths import local_skill

ROOT = Path(__file__).resolve().parent.parent
PROCESS_SKILLS = ("adr", "design-review", "design-review-code-intelligence", "handoff", "pin-check")
LINK_RE = re.compile(r"\]\(([^)#\s]+)(?:#[^)]*)?\)")
TICK_RE = re.compile(r"`([^`\s]+)`")
JUST_RE = re.compile(r"`just ([a-z][a-z0-9-]*)")
PATH_ROOTS = (
    "docs/",
    "scripts/",
    "crates/",
    "fixtures/",
    "rules/",
    "rule-tests/",
    "tests/",
    ".claude/",
    ".agents/",
)
PLACEHOLDER = re.compile(r"[<>{}*]|NNNN|YYYY|\.\.\.|…")


def documents(root: Path) -> list[Path]:
    docs = [root / "AGENTS.md", root / "CLAUDE.md", root / ".claude" / "skills" / "README.md"]
    docs += [root / ".claude" / "skills" / s / "SKILL.md" for s in PROCESS_SKILLS]
    docs += sorted((root / ".claude" / "agents").glob("*.md"))
    return docs


def recipes(root: Path) -> set[str]:
    out = subprocess.run(
        ["just", "--summary"], cwd=root, capture_output=True, text=True, check=True
    ).stdout
    return set(out.split())


def check(root: Path) -> list[str]:
    problems: list[str] = []
    link = root / ".agents" / "skills"
    if not link.is_symlink() or link.resolve() != (root / ".claude" / "skills").resolve():
        problems.append(".agents/skills must be a symlink to ../.claude/skills")

    for skill_md in sorted((root / ".claude" / "skills").glob("*/SKILL.md")):
        match = re.search(r"^name:\s*(\S+)", skill_md.read_text(), re.M)
        if not match or match.group(1) != skill_md.parent.name:
            problems.append(f"{skill_md.relative_to(root)}: `name` must equal its directory")

    known = recipes(root)
    for doc in documents(root):
        if not doc.exists():
            problems.append(f"{doc.relative_to(root)}: missing")
            continue
        text = doc.read_text()
        rel = doc.relative_to(root)
        for target in LINK_RE.findall(text):
            if "://" in target or PLACEHOLDER.search(target):
                continue
            if not (doc.parent / target).exists() and not local_skill(
                root, (doc.parent / target).resolve()
            ):
                problems.append(f"{rel}: link target does not exist: {target}")
        for token in TICK_RE.findall(text):
            token = token.rstrip(".,:;")
            if not token.startswith(PATH_ROOTS) or PLACEHOLDER.search(token):
                continue
            if not (root / token.split(":")[0]).exists() and not local_skill(
                root, root / token.split(":")[0]
            ):
                problems.append(f"{rel}: path does not exist: {token}")
        for recipe in JUST_RE.findall(text):
            if recipe not in known:
                problems.append(f"{rel}: unknown just recipe: just {recipe}")
    return problems


def main() -> int:
    problems = check(ROOT)
    for p in problems:
        print(p)
    if not problems:
        print("lint-agents: ok")
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())
