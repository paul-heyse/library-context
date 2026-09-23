"""Reference solution for held-out task T03."""

from fastmcp import FastMCP
from fastmcp.tools import Tool, tool
from fastmcp.tools.tool_transform import ArgTransform

SERVER_KEY = "k-123"


# Stand-in for vendor_search.search, which we may not edit.
def search(q: str, n: int = 10, api_key: str = "") -> list[str]:
    """Search the vendor catalogue."""
    if api_key != "k-123":
        raise PermissionError("invalid api key")
    return [f"{q}:{i}" for i in range(n)]


find_items = Tool.from_tool(
    tool(search),
    name="find_items",
    description="Find items matching a query.",
    transform_args={
        "q": ArgTransform(name="query", description="Search terms"),
        "n": ArgTransform(name="max_results"),
        "api_key": ArgTransform(hide=True, default=SERVER_KEY),
    },
)

mcp = FastMCP("Catalogue")
mcp.add_tool(find_items)
