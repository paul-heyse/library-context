"""Extract the gold families the evaluation scores against (DESIGN §1.4, §12; ADR-0013).

Reads the `fastmcp` skill's capability catalog (the gold reference: evaluation only, never a
compiler input) and writes a small committed extract: per family, its id, title,
`authoring_sha256`, operations, task aliases and static-evidence source spans (site-relative
path, lines, hashes). `check_gold.py` fails when the committed extract is stale.

Usage: gold_extract.py [--check]   Writes eval/gold/fastmcp-4.0.5.json, or with --check exits 1
when it differs from what the skill gives.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CATALOG = ROOT / ".claude" / "skills" / "fastmcp" / "content" / "capabilities" / "catalog.json"
EXTRACT = ROOT / "eval" / "gold" / "fastmcp-4.0.5.json"
SOURCES = "content/sources/"


def spans(family: dict) -> list[dict]:
    """The family's static-evidence spans, each once, sorted."""
    found: dict[tuple, dict] = {}
    for claim in family.get("claims", []):
        for e in claim.get("evidence", []):
            if e.get("kind") != "source_span" or not e["artifact"].startswith(SOURCES):
                continue
            span = {
                "path": e["artifact"][len(SOURCES) :],
                "line_start": e["line_start"],
                "line_end": e["line_end"],
                "source_sha256": e["source_sha256"],
                "excerpt_sha256": e["excerpt_sha256"],
            }
            found[(span["path"], span["line_start"], span["line_end"])] = span
    return [found[k] for k in sorted(found)]


def family(entry: dict) -> dict:
    """A catalog entry with its claims: the family's own record holds them in full."""
    record = CATALOG.parent / f"{entry['id']}.json"
    return json.loads(record.read_text(encoding="utf-8")) if record.exists() else entry


def extract(catalog: list[dict]) -> dict:
    families = [
        {
            "id": f["id"],
            "title": f["title"],
            "authoring_sha256": f["authoring_sha256"],
            "operations": sorted(f["operations"]),
            "task_aliases": list(f["task_aliases"]),
            "source_spans": spans(f),
        }
        for f in sorted((family(e) for e in catalog), key=lambda f: f["id"])
    ]
    return {"library": "fastmcp", "release": "fastmcp==4.0.5", "families": families}


def render(value: dict) -> str:
    return json.dumps(value, indent=1, sort_keys=True, ensure_ascii=False) + "\n"


def main(argv: list[str]) -> int:
    text = render(extract(json.loads(CATALOG.read_text(encoding="utf-8"))))
    if "--check" in argv:
        if not EXTRACT.exists() or EXTRACT.read_text(encoding="utf-8") != text:
            print(f"gold: {EXTRACT.relative_to(ROOT)} is stale: run scripts/gold_extract.py")
            return 1
        return 0
    EXTRACT.parent.mkdir(parents=True, exist_ok=True)
    EXTRACT.write_text(text, encoding="utf-8")
    print(f"wrote {EXTRACT.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
