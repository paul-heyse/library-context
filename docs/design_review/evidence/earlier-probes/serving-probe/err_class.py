import asyncio, os, logging
os.environ["FASTMCP_CHECK_FOR_UPDATES"] = "off"
logging.disable(logging.CRITICAL)
from fastmcp import Client, FastMCP
from fastmcp.exceptions import ValidationError, ToolError

class CapabilityError(ValidationError):
    """unknown library/snapshot/capability"""

mcp = FastMCP("p", mask_error_details=True)

@mcp.tool
def get(cid: str) -> dict:
    raise CapabilityError(f"no capability {cid}")

@mcp.resource("capability://{sid}/{cid}", mime_type="text/markdown")
def cap(sid: str, cid: str) -> str:
    raise CapabilityError(f"no capability {cid} in {sid}")

@mcp.tool
def boom(cid: str) -> dict:
    raise RuntimeError("secret detail")

async def main():
    for mode in ("auto", "legacy"):
        async with Client(mcp, mode=mode) as c:
            r = await c.call_tool("get", {"cid": "x"}, raise_on_error=False)
            print(mode, "tool:", r.is_error, r.content[0].text)
            r = await c.call_tool("boom", {"cid": "x"}, raise_on_error=False)
            print(mode, "masked tool:", r.is_error, r.content[0].text)
            try:
                await c.read_resource("capability://s/x")
            except Exception as e:
                print(mode, "resource:", type(e).__name__, getattr(getattr(e, "error", None), "code", None), str(e))
asyncio.run(main())
