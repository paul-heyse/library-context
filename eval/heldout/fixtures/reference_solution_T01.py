"""Reference solution for held-out task T01."""

from mcp.types import ToolAnnotations

from fastmcp import FastMCP
from fastmcp.exceptions import ToolError

ORDERS = {"A-100": {"id": "A-100", "status": "shipped", "items": ["widget", "gadget"]}}

mcp = FastMCP("Orders")


@mcp.tool(
    title="Look up order",
    annotations=ToolAnnotations(readOnlyHint=True, idempotentHint=True, openWorldHint=False),
)
def lookup_order(order_id: str, include_items: bool = False) -> dict:
    """Look up a customer order by its ID.

    Args:
        order_id: The order identifier, e.g. A-100.
        include_items: Whether to include line items in the result.
    """
    if order_id not in ORDERS:
        raise ToolError(f"Order {order_id} not found")
    order = dict(ORDERS[order_id])
    if not include_items:
        order.pop("items", None)
    return order
