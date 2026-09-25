"""Recognize optional local capability references from the existing skill directory."""

import re
import subprocess
from pathlib import Path


def local_skill(root: Path, path: Path) -> bool:
    try:
        relative = path.relative_to(root)
    except ValueError:
        return False
    if relative.parts[:2] != (".claude", "skills") or len(relative.parts) < 3:
        return False
    index = root / ".claude/skills/README.md"
    names = (
        set(re.findall(r"\]\(([a-z0-9-]+)/SKILL\.md\)", index.read_text()))
        if index.exists()
        else set()
    )
    return (
        relative.parts[2] in names
        and subprocess.run(["git", "check-ignore", "-q", "--", str(relative)], cwd=root).returncode
        == 0
    )
