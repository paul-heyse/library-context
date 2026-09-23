"""`python -m lctx_mcp.smoke GENERATION [--embedder fake|vllm|none]`: serve a generation over
stdio in a subprocess and check it (DESIGN §6.4's smoke query, §11.3).

For every brief: its own access path finds it first, promoted, and it hydrates whole through
`get_capability` and the resource, and every usage pattern it serves parses (§10.4). Prints one
line per brief; exits 1 on the first failure.
"""

from __future__ import annotations

import argparse
import ast
import asyncio
import json
import os
import sys
from pathlib import Path


async def smoke(generation: Path, embedder: str) -> int:
    from fastmcp import Client
    from fastmcp.client.transports import StdioTransport

    manifest = json.loads((generation / "MANIFEST.json").read_text(encoding="utf-8"))
    transport = StdioTransport(
        command=sys.executable,
        args=["-m", "lctx_mcp", "--generation", str(generation), "--embedder", embedder],
        env={**os.environ, "FASTMCP_CHECK_FOR_UPDATES": "off"},
    )
    import pyarrow.ipc as ipc

    briefs = ipc.open_file(str(generation / "briefs.arrow")).read_all().to_pylist()
    async with Client(transport) as client:
        for brief in briefs:
            found = await client.call_tool(
                "search_capabilities",
                {"library": manifest["library"], "query": brief["access_path"], "limit": 1},
            )
            result = found.structured_content or {}
            hits = result.get("hits", [])
            if not hits or hits[0]["capability_id"] != brief["brief_id"].hex():
                print(f"smoke: {brief['access_path']} does not find its own brief", file=sys.stderr)
                return 1
            full = await client.call_tool(
                "get_capability",
                {"snapshot_id": result["snapshot_id"], "capability_id": hits[0]["capability_id"]},
            )
            card = full.structured_content or {}
            uri = f"capability://{result['snapshot_id']}/{hits[0]['capability_id']}"
            (text,) = await client.read_resource(uri)
            if not card.get("assertions") or not getattr(text, "text", "").startswith("# "):
                print(f"smoke: {brief['access_path']} does not hydrate", file=sys.stderr)
                return 1
            for a in card["assertions"]:
                if a["kind"] == "usage_pattern" and a["text"]:
                    code = a["text"].split("```python\n", 1)[-1].rsplit("\n```", 1)[0]
                    try:
                        ast.parse(code)
                    except SyntaxError as e:
                        print(f"smoke: {brief['access_path']}'s pattern: {e}", file=sys.stderr)
                        return 1
            print(
                f"smoke: {brief['access_path']}: found first ({hits[0]['rank_source']}, "
                f"{result['mode']}), {len(card['assertions'])} assertions"
            )
    print(f"smoke: passed ({len(briefs)} briefs, generation {manifest['generation']})")
    return 0


def main(argv: list[str] | None = None) -> None:
    parser = argparse.ArgumentParser(prog="lctx_mcp.smoke", description=__doc__)
    parser.add_argument("generation", type=Path)
    parser.add_argument("--embedder", choices=["vllm", "fake", "none"], default="fake")
    args = parser.parse_args(argv)
    sys.exit(asyncio.run(smoke(args.generation.resolve(), args.embedder)))


if __name__ == "__main__":
    main()
