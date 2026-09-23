"""Reference solution for held-out task T09."""

from fastmcp import FastMCP
from fastmcp.exceptions import ToolError

mcp = FastMCP("Calculator", mask_error_details=True)


@mcp.tool
def divide(a: float, b: float) -> float:
    """Divide a by b."""
    if b == 0:
        raise ToolError("Division by zero is not allowed.")
    return a / b


@mcp.tool
def load_secret() -> str:
    """Load a secret from the database."""
    raise RuntimeError("db password is hunter2")
