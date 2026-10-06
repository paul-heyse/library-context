"""Serve one explicitly selected immutable native snapshot over MCP."""

from __future__ import annotations

import argparse
import asyncio
from contextlib import asynccontextmanager
from pathlib import Path

from fastmcp import FastMCP
from lctx_semantics import NativeSession

from lctx_mcp.embedder import HttpEmbedder
from lctx_mcp.native import NativeExecutor
from lctx_mcp.wire import register


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
        session = await asyncio.to_thread(NativeSession, str(serving_config))
        native = NativeExecutor(session, HttpEmbedder(embedding_url) if embedding_url else None)
        executor.native = native
        try:
            yield {}
        finally:
            executor.native = None
            await asyncio.shield(native.close())

    server = FastMCP("library-context", lifespan=lifespan, cache_ttl=0, mask_error_details=True)
    register(server, executor)
    return server


def main(argv: list[str] | None = None) -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--serving-config", type=Path, default=Path("build/native/selected.serving.json"))
    parser.add_argument("--embedding-url", help="Existing Qwen embedding service; omit to disable vector search")
    options = parser.parse_args(argv)
    create_server(options.serving_config, options.embedding_url).run(transport="stdio")


if __name__ == "__main__":
    main()
