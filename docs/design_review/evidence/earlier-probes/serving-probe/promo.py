import asyncio, sys
from pathlib import Path
from lctx_mcp.generation import load
from lctx_mcp.server import search, serve
g = load(Path("build/generations/438801c473c4d6d6"), None)
s = serve(g, None)
async def main():
    for q in ["fastmcp.FastMCP.http_app", "fastmcp.server.mixins.TransportMixin.http_app", "fastmcp.FastMCP.run"]:
        r = await search(s, "fastmcp", q, 3)
        print(q, r.mode, [(h.title, h.rank_source, h.promoted) for h in r.hits])
asyncio.run(main())
