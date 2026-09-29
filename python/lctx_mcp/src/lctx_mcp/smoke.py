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

DEFAULT_CONFIG = Path.home() / ".config/library-context/postgres-serving.json"


async def smoke(
    generation: Path,
    embedder: str,
    config: Path = DEFAULT_CONFIG,
) -> int:
    from fastmcp import Client
    from fastmcp.client.transports import StdioTransport

    manifest = json.loads((generation / "MANIFEST.json").read_text(encoding="utf-8"))
    transport = StdioTransport(
        command=sys.executable,
        args=[
            "-m",
            "lctx_mcp",
            "--config",
            str(config),
            "--library",
            manifest["library"],
            "--generation",
            manifest["projection_generation"],
            "--embedder",
            embedder,
        ],
        env={**os.environ, "FASTMCP_CHECK_FOR_UPDATES": "off"},
    )
    import pyarrow.ipc as ipc

    briefs = ipc.open_file(str(generation / "briefs.arrow")).read_all().to_pylist()
    operations = ipc.open_file(str(generation / "operations.arrow")).read_all().to_pylist()
    capabilities = manifest["capabilities"]
    async with Client(transport) as client:
        page = await client.call_tool(
            "find_operations", {"library": manifest["library"], "selection": {}, "limit": 3}
        )
        if not page.structured_content:
            raise RuntimeError("operation discovery returned no structured result")
        for operation in sorted(operations, key=lambda row: row["access_path"])[:3]:
            result = await client.call_tool(
                "get_operation",
                {
                    "snapshot_id": manifest["snapshot_id"],
                    "operation": operation["access_path"],
                    "expanded": True,
                },
            )
            record = result.structured_content or {}
            if record.get("capabilities") != capabilities or not record.get("catalog"):
                raise RuntimeError("catalog contract/capabilities missing from operation packet")
            section = await client.call_tool(
                "get_operation",
                {
                    "snapshot_id": manifest["snapshot_id"],
                    "operation": operation["access_path"],
                    "view": {"kind": "section", "section": "evidence"},
                },
            )
            references = (section.structured_content or {})["items"]
            if references:
                evidence = await client.call_tool(
                    "get_evidence",
                    {
                        "snapshot_id": manifest["snapshot_id"],
                        "evidence": references[0]["record"]["evidence"],
                    },
                )
                packet = evidence.structured_content or {}
                if packet.get("result_kind") != "evidence" or not packet.get("content"):
                    raise RuntimeError("original evidence did not hydrate through MCP")
        if not capabilities["briefs"]:
            result = await client.call_tool(
                "search_capabilities", {"library": manifest["library"], "query": "catalog"}
            )
            if (result.structured_content or {}).get("reason") != "not_requested":
                raise RuntimeError("unselected briefs must return not_requested")
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
    print(
        f"smoke: passed ({min(3, len(operations))} catalog packets, {len(briefs)} briefs, "
        f"generation {manifest['generation']})"
    )
    return 0


def main(argv: list[str] | None = None) -> None:
    parser = argparse.ArgumentParser(prog="lctx_mcp.smoke", description=__doc__)
    parser.add_argument("generation", type=Path)
    parser.add_argument(
        "--config", type=Path, default=Path.home() / ".config/library-context/postgres-serving.json"
    )
    parser.add_argument("--embedder", choices=["vllm", "fake", "none"], default="fake")
    args = parser.parse_args(argv)
    sys.exit(asyncio.run(smoke(args.generation.resolve(), args.embedder, args.config)))


if __name__ == "__main__":
    main()
