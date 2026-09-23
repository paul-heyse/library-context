"""The §1.5 retrieval check: for each task alias of the gold family, does the pre-registered seed's
brief rank first against the distractor briefs? (DESIGN §1.5, §12; evaluation only.)

Usage: ranking_check.py GENERATION [--embedder vllm|fake|none] [--embed-url URL] [--family ID]

Prints each alias with the seed brief's rank and the mode (hybrid or lexical-only), then a summary.
A fake-vector run is labelled as such; only real vectors make a hybrid result evidence.
"""

from __future__ import annotations

import argparse
import asyncio
import json
import tomllib
from pathlib import Path

from lctx_mcp.embedder import FakeEmbedder, HttpEmbedder
from lctx_mcp.generation import load
from lctx_mcp.server import search, serve

ROOT = Path(__file__).resolve().parent.parent
GOLD = ROOT / "eval" / "gold" / "fastmcp-4.0.5.json"
CONFIG = ROOT / "libraries" / "fastmcp" / "analytics.toml"


async def check(generation: Path, embedder_name: str, url: str, family_id: str) -> int:
    embedder = {
        "vllm": lambda: HttpEmbedder(url, timeout=60.0),
        "fake": FakeEmbedder,
        "none": lambda: None,
    }[embedder_name]()
    gen = load(generation, embedder.spec if embedder else None)
    served = serve(gen, embedder)
    seeds = tomllib.loads(CONFIG.read_text())["seeds"]
    target = seeds["primary"][0]
    family = next(f for f in json.loads(GOLD.read_text())["families"] if f["id"] == family_id)
    first = 0
    for alias in family["task_aliases"]:
        result = await search(served, gen.library, alias, limit=10)
        order = [h.title for h in result.hits]
        rank = order.index(target) + 1 if target in order else None
        first += rank == 1
        print(f"{result.mode:12} rank {rank}  {alias!r}  {order}")
        if result.degraded_reason:
            print(f"{'':12} ({result.degraded_reason})")
    label = " (fake vectors: not evidence)" if embedder_name == "fake" else ""
    print(
        f"ranking: {target} first for {first} of {len(family['task_aliases'])} "
        f"{family_id} aliases against {len(gen.briefs) - 1} distractors{label}"
    )
    return 0


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("generation", type=Path)
    parser.add_argument("--embedder", choices=["vllm", "fake", "none"], default="none")
    parser.add_argument("--embed-url", default="http://127.0.0.1:8000")
    parser.add_argument("--family", default="fm.register")
    args = parser.parse_args()
    raise SystemExit(
        asyncio.run(check(args.generation, args.embedder, args.embed_url, args.family))
    )


if __name__ == "__main__":
    main()
