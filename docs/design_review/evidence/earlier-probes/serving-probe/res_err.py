import asyncio, os
os.environ["FASTMCP_CHECK_FOR_UPDATES"] = "off"
from pathlib import Path
from fastmcp import Client
from lctx_mcp.server import build_server
from lctx_mcp.embedder import FakeEmbedder
gen = Path("build/py-fixture") / Path("build/py-fixture/CURRENT").read_text().strip()
async def main():
    for mode in ("auto", "legacy"):
        async with Client(build_server(gen, FakeEmbedder()), mode=mode) as c:
            import json
            snap = json.loads((gen / "MANIFEST.json").read_text())["snapshot_id"]
            try:
                await c.read_resource(f"capability://{snap}/{'00'*16}")
            except Exception as e:
                err = getattr(e, "error", None)
                print(mode, type(e).__name__, getattr(err, "code", None), str(e)[:100])
asyncio.run(main())
