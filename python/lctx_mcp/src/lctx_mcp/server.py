"""FastMCP transport over the Rust generation service and closed tool inventory."""

from __future__ import annotations

from collections.abc import AsyncIterator
from contextlib import asynccontextmanager
from pathlib import Path

from fastmcp import FastMCP

from lctx_mcp.embedder import Embedder
from lctx_mcp.generation import open_generation
from lctx_mcp.wire import register

INSTRUCTIONS = (
    "A version-pinned API and evidence catalog. Read the structured result's support, "
    "uncertainty, coverage and original evidence before relying on a discovery result. "
    "Relevance ranks results; it does not prove a requirement. Value-path refutations "
    "apply only to the cited path under the supplied input model. Each request names its "
    "library explicitly and uses the generation pinned when this service starts."
)


def build_server(
    config: Path,
    embedder: Embedder | None = None,
    *,
    generation: str | None = None,
    vectors: bool = False,
    library: str | None = None,
) -> FastMCP:
    @asynccontextmanager
    async def lifespan(server: FastMCP) -> AsyncIterator[dict]:
        served = await open_generation(config, embedder, generation=generation, vectors=vectors)
        try:
            yield {"served": served}
        finally:
            await served.service.shutdown()

    instructions = INSTRUCTIONS
    if library is not None:
        instructions += f" The operator's default library is {library!r}."
    server = FastMCP(
        "library-context",
        instructions=instructions,
        lifespan=lifespan,
        mask_error_details=True,
        dereference_schemas=False,
    )
    register(server)
    return server
