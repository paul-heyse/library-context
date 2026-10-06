"""One pinned native session, with bounded request admission and the retained query embedder."""

from __future__ import annotations

import asyncio
import hashlib
import json
from typing import TYPE_CHECKING

from lctx_semantics import NativeSession, wire_decode

from lctx_mcp.embedder import EmbedderError

if TYPE_CHECKING:
    from lctx_mcp.embedder import Embedder


class NativeExecutor:
    def __init__(self, session: NativeSession, embedder: Embedder | None = None) -> None:
        self.session = session
        self.embedder = embedder
        self._slots = asyncio.Semaphore(2)
        self._closed = False

    async def execute(self, tool: str, arguments: dict) -> str:
        # Rust validates the public request before inference, allocation or a store query.
        request = wire_decode(tool, json.dumps(arguments, separators=(",", ":"), ensure_ascii=False))
        if self._closed:
            raise RuntimeError("native session closed")
        try:
            await asyncio.wait_for(self._slots.acquire(), timeout=1.0)
        except TimeoutError as exc:
            raise RuntimeError("resource_refused: native request slots") from exc
        try:
            if self._closed:
                raise RuntimeError("native session closed")
            vector_json = None
            if tool in {"search_operations", "search_evidence", "search_capabilities"} and self.embedder:
                query = json.loads(request).get("query")
                if isinstance(query, str):
                    text = self.embedder.spec.query_text(query)
                    try:
                        values = await self.embedder.embed([text])
                    except EmbedderError:
                        # The backend reports the unavailable vector channel while lexical search runs.
                        values = None
                        vector_json = "unavailable"
                    if values is not None:
                        vector_json = json.dumps(
                            {
                                "spec": list(bytes.fromhex(self.embedder.spec.hash)),
                                "input": list(hashlib.sha256(text.encode()).digest()),
                                "vector": values[0].tolist(),
                            },
                            separators=(",", ":"),
                            allow_nan=False,
                        )
            return await asyncio.to_thread(self.session.execute, tool, request, vector_json)

        finally:
            self._slots.release()

    async def close(self) -> None:
        self._closed = True
        await asyncio.to_thread(self.session.close)
