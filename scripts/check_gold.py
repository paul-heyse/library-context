"""Keep the gold reference and the analysis on one FastMCP (ADR-0013, DESIGN §1.4, §12).

The `fastmcp` skill (the gold reference, evaluation only) and `libraries/fastmcp` (what the
compiler analyzes) must name the same install line and resolve the release distributions to the
same versions. Otherwise gold scores compare different code.

Usage: check_gold.py   Exit 0 when they agree, 1 on a mismatch, 0 with `not_run` when the skill is
absent (it is local, not in the repo).
"""

from __future__ import annotations

import json
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
LIBRARY = ROOT / "libraries" / "fastmcp"
SKILL = ROOT / ".claude" / "skills" / "fastmcp" / "build" / "manifests"


def library_pins(library: Path) -> tuple[str, list[str], dict[str, str]]:
    """The requirement, the release distributions, and every locked version."""
    project = tomllib.loads((library / "pyproject.toml").read_text())
    lock = tomllib.loads((library / "uv.lock").read_text())
    versions = {p["name"]: p["version"] for p in lock.get("package", []) if "version" in p}
    return (
        project["project"]["dependencies"][0],
        project["tool"]["lctx"]["release"],
        versions,
    )


def skill_pins(manifests: Path) -> tuple[str, dict[str, str]]:
    """The skill's install line and its resolved versions."""
    manifest = json.loads((manifests / "fastmcp.json").read_text())
    resolved = json.loads((manifests / "resolved.json").read_text())
    return manifest["environment"]["install"][0], resolved


def problems(library: Path, manifests: Path) -> list[str]:
    requirement, release, locked = library_pins(library)
    install, resolved = skill_pins(manifests)
    out = []
    if install != requirement:
        out.append(f"skill installs {install!r}, libraries/fastmcp requires {requirement!r}")
    for dist in release:
        if resolved.get(dist) != locked.get(dist):
            out.append(f"{dist}: skill {resolved.get(dist)}, uv.lock {locked.get(dist)}")
    return out


def main() -> int:
    if not (SKILL / "fastmcp.json").exists():
        print("gold: not_run (the fastmcp skill is not installed)")
        return 0
    found = problems(LIBRARY, SKILL)
    # The committed extract the evaluation scores against is the skill's (DESIGN §12).
    import gold_extract

    catalog = json.loads(gold_extract.CATALOG.read_text(encoding="utf-8"))
    text = gold_extract.render(gold_extract.extract(catalog))
    if not gold_extract.EXTRACT.exists() or gold_extract.EXTRACT.read_text("utf-8") != text:
        found.append(f"{gold_extract.EXTRACT.relative_to(ROOT)} is stale: run gold_extract.py")
    for p in found:
        print(f"gold: {p}")
    if not found:
        print("gold: ok (the fastmcp skill and libraries/fastmcp name one FastMCP)")
    return 1 if found else 0


if __name__ == "__main__":
    sys.exit(main())
