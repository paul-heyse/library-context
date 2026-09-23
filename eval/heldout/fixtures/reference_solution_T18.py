"""Reference solution for held-out task T18."""

from fastmcp import Context, FastMCP

mcp = FastMCP("Processor")


@mcp.tool
async def process_items(items: list[str], ctx: Context) -> list[str]:
    """Uppercase each item, reporting progress and logging as it goes."""
    total = len(items)
    results = []
    for done, item in enumerate(items, start=1):
        results.append(item.upper())
        await ctx.info(f"Processed item {item}")
        await ctx.report_progress(progress=done, total=total, message=item)
    return results
