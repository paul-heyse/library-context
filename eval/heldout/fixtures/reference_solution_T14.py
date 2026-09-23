"""Reference solution for held-out task T14."""

from fastmcp import FastMCP
from fastmcp.server.middleware import Middleware, MiddlewareContext

AUDIT_LOG: list[dict] = []


class AuditMiddleware(Middleware):
    """Append one redacted record per tool call to AUDIT_LOG."""

    async def on_call_tool(self, context: MiddlewareContext, call_next):
        arguments = context.message.arguments or {}
        record = {
            "tool": context.message.name,
            "arguments": {key: "<redacted>" for key in arguments},
        }
        try:
            result = await call_next(context)
        except Exception as exc:
            record["status"] = "failed"
            record["error"] = type(exc).__name__
            AUDIT_LOG.append(record)
            raise
        record["status"] = "failed" if result.is_error else "completed"
        AUDIT_LOG.append(record)
        return result


mcp = FastMCP("Audited")
mcp.add_middleware(AuditMiddleware())


@mcp.tool
def echo(text: str) -> str:
    """Return the text unchanged."""
    return text


@mcp.tool
def explode() -> str:
    """Always fail."""
    raise RuntimeError("boom")
