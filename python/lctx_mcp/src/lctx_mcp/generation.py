"""Lifespan state for the Rust generation service and numerical callback."""
from __future__ import annotations

import asyncio
from dataclasses import dataclass
from pathlib import Path
from typing import TYPE_CHECKING

from lctx_mcp.embedder import Embedder, EmbedderError, Spec
from lctx_mcp.retrieval import NumericalScorer

if TYPE_CHECKING:
    from lctx_storage import RequestGrant, Service


class GenerationError(RuntimeError):
    """The configured query client cannot serve the selected vector artifact."""


@dataclass(frozen=True)
class Generation:
    service: Service
    numerical: NumericalScorer
    embedder: Embedder | None

    async def query_vector(self, grant: RequestGrant, query: str | None) -> tuple[list[float] | None, str | None]:
        if query is None:
            return None, None
        raw_spec = await self.service.embedding_spec(grant)
        if raw_spec is None:
            return None, None
        if self.embedder is None:
            return None, "query_embedder_unconfigured"
        actual = Spec.from_json(raw_spec)
        if actual.canonical != self.embedder.spec.canonical:
            raise GenerationError("query embedding spec differs from the selected artifact")
        try:
            async with asyncio.timeout(grant.remaining_seconds()):
                vectors = await self.embedder.embed([actual.query_text(query)])
        except (EmbedderError, TimeoutError):
            return None, "query_embedding_unavailable"
        return vectors[0].tolist(), None


async def open_generation(config: Path, embedder: Embedder | None, *, generation: str | None = None, vectors: bool = False) -> Generation:
    from lctx_storage import open_service

    service = await open_service(str(config), generation=generation, vectors=vectors)
    try:
        numerical = NumericalScorer()
        grant = await service.admit()
        try:
            raw_spec = await service.embedding_spec(grant)
            if raw_spec is not None and embedder is not None:
                if Spec.from_json(raw_spec).canonical != embedder.spec.canonical:
                    raise GenerationError("query embedding spec differs from the selected artifact")
            await service.initialize_numerical(grant, numerical.initialize)
        finally:
            grant.release()
        return Generation(service, numerical, embedder)
    except BaseException:
        await service.shutdown()
        raise
