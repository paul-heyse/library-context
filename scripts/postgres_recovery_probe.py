"""Serve current recovered generations; report preserved legacy generations explicitly."""

from __future__ import annotations

import asyncio
import json
import sys
from pathlib import Path

from fastmcp import Client
from lctx_storage import open_repository

from lctx_mcp.generation import FORMAT, load
from lctx_mcp.server import NativeWorkers, build_server, serve


async def probe(config: Path, receipt: Path):
    expected = json.loads(receipt.read_text())
    repository = await open_repository(config)
    workers = NativeWorkers()
    results = []
    try:
        await repository.check()
        for generation in expected["inventory"]["generations"]:
            manifest = generation["manifest"]
            if manifest["bundle_format"] != FORMAT:
                if (
                    manifest["format"],
                    manifest["bundle_format"],
                    generation["runtime_admission"],
                ) not in {(2, 12, "legacy_runtime_required"), (3, 13, "legacy_runtime_required")}:
                    raise RuntimeError("unrecognized legacy generation")
                results.append(
                    {
                        "generation": generation["generation"],
                        "serving": "not_run",
                        "reason": "legacy_runtime_required",
                        "preservation": "passed",
                    }
                )
                continue
            pinned = await repository.pin(manifest["context"]["library"], generation["generation"])
            descriptor = json.loads(pinned.descriptor())
            if descriptor["manifest"] != manifest:
                raise RuntimeError("recovered manifest mismatch")
            state = await workers.run(load, pinned, descriptor, await pinned.inputs(), None)
            await workers.run(serve, state, None, workers)
            operations = json.loads(await pinned.find_operations("{}", 1))
            scope = json.loads(await pinned.search_scope("", True))
            if scope["eligible"]:
                answer = json.loads(
                    await pinned.get_operation(manifest["snapshot_id"], scope["eligible"][0])
                )
                if not answer:
                    raise RuntimeError("recovered operation hydration failed")
            async with Client(
                build_server(
                    config,
                    None,
                    library=manifest["context"]["library"],
                    generation=generation["generation"],
                )
            ) as client:
                response = await client.call_tool(
                    "find_operations",
                    {"library": manifest["context"]["library"], "where": {}, "limit": 1},
                )
                if not response.structured_content:
                    raise RuntimeError("recovered MCP lifespan/read failed")
            results.append(
                {
                    "generation": generation["generation"],
                    "native_loaded": state.condition_graph is not None,
                    "capabilities": manifest["capabilities"],
                    "operations": len(scope["eligible"]),
                    "page": bool(operations),
                }
            )
        selections = []
        for selected in expected["selections"]:
            if any(
                g["generation"] == selected["generation"] and g["runtime_admission"] != "current"
                for g in expected["inventory"]["generations"]
            ):
                raise RuntimeError("selected generation requires retained runtime")
            pinned = await repository.pin(selected["library"])
            descriptor = json.loads(pinned.descriptor())
            if (
                descriptor["generation"] != selected["generation"]
                or descriptor["policy"]["route"] != "exact"
            ):
                raise RuntimeError("recovered default selection mismatch")
            selections.append(selected["generation"])
        return {"outcome": "passed", "generations": results, "selected_generations": selections}
    finally:
        await workers.close()
        await repository.close()


if __name__ == "__main__":
    print(json.dumps(asyncio.run(probe(Path(sys.argv[1]), Path(sys.argv[2])))))
