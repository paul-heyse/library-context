"""Serve one explicitly selected immutable native snapshot over MCP."""

from __future__ import annotations

import argparse
from contextlib import asynccontextmanager
from pathlib import Path
from functools import partial

import anyio
from fastmcp import FastMCP
from lctx_semantics import NativeSession

from lctx_mcp.embedder import HttpEmbedder
from lctx_mcp.native import NativeExecutor
from lctx_mcp.wire import BoundedStdioWriter, EnvelopeAdmission, register


def create_server(serving_config: Path, embedding_url: str | None = None) -> FastMCP:
    class Executor:
        native: NativeExecutor | None = None

        async def execute(self, tool: str, arguments: dict) -> str:
            if self.native is None:
                raise RuntimeError("native session is not running")
            return await self.native.execute(tool, arguments)

    executor = Executor()

    @asynccontextmanager
    async def lifespan(server):
        session = await anyio.to_thread.run_sync(NativeSession, str(serving_config))
        native = NativeExecutor(session, HttpEmbedder(embedding_url) if embedding_url else None)
        executor.native = native
        try:
            yield {}
        finally:
            executor.native = None
            # FastMCP owns its lifespan through AnyIO cancellation scopes. Drain before exit.
            with anyio.CancelScope(shield=True):
                await native.close()

    server = FastMCP("library-context", lifespan=lifespan, cache_ttl=None,
                     mask_error_details=True, dereference_schemas=False)
    server.add_middleware(EnvelopeAdmission(server))
    register(server, executor)
    return server


async def run_stdio(server: FastMCP) -> None:
    """Pinned SDK stdio entry with byte admission immediately before its stdout writer."""
    from fastmcp.server.context import reset_transport, set_transport
    from mcp.server.lowlevel.server import NotificationOptions
    from mcp.server.stdio import stdio_server

    admission = next(m for m in server.middleware if isinstance(m, EnvelopeAdmission))
    token = set_transport("stdio")
    try:
        # FastMCP 4.0.5 owns the lifespan and low-level SDK registry at these seams;
        # no handlers or transport implementations are replaced at runtime.
        async with server._lifespan_manager(), stdio_server() as (read_stream, write_stream):
            await server._mcp_server.run(
                read_stream,
                BoundedStdioWriter(write_stream, admission),
                server._mcp_server.create_initialization_options(
                    notification_options=NotificationOptions(tools_changed=True),
                ),
            )
    finally:
        reset_transport(token)


def main(argv: list[str] | None = None) -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--serving-config", type=Path, default=Path("build/native/selected.serving.json"))
    parser.add_argument("--embedding-url", help="Existing Qwen embedding service; omit to disable vector search")
    options = parser.parse_args(argv)
    server = create_server(options.serving_config, options.embedding_url)
    anyio.run(partial(run_stdio, server))


if __name__ == "__main__":
    main()
