"""Reference solution for held-out task T04."""

from typing import Annotated, Literal

from pydantic import Field

from fastmcp import FastMCP

mcp = FastMCP("Audio", strict_input_validation=True)


@mcp.tool
def set_volume(
    level: Annotated[int, Field(ge=0, le=100, description="Volume level, 0 to 100.")],
    channel: Literal["left", "right", "both"] = "both",
) -> str:
    """Set the output volume for a channel."""
    return f"{channel}:{level}"
