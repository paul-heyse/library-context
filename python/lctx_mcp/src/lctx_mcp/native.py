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
        self._pending: set[asyncio.Task[str]] = set()
        self._close_task: asyncio.Task[None] | None = None

    async def execute(self, tool: str, arguments: dict) -> str:
        deadline = asyncio.get_running_loop().time() + 30.0
        # Rust validates the public request before inference, allocation or a store query.
        request = wire_decode(tool, json.dumps(arguments, separators=(",", ":"), ensure_ascii=False))
        if self._closed:
            raise RuntimeError("native session closed")
        try:
            await asyncio.wait_for(self._slots.acquire(), timeout=1.0)
        except TimeoutError as exc:
            raise RuntimeError("resource_refused: native request slots") from exc
        if self._closed:
            self._slots.release()
            raise RuntimeError("native session closed")
        # This task owns its admission slot even when the client cancels or times out.
        worker = asyncio.create_task(self._execute(tool, request, deadline))
        self._pending.add(worker)
        worker.add_done_callback(self._finished)
        try:
            remaining = max(0.0, deadline - asyncio.get_running_loop().time())
            return await asyncio.wait_for(asyncio.shield(worker), timeout=remaining)
        except TimeoutError as exc:
            raise RuntimeError("resource_refused: native request deadline") from exc

    def _finished(self, worker: asyncio.Task[str]) -> None:
        self._pending.discard(worker)
        self._slots.release()
        if not worker.cancelled():
            # Retrieve an abandoned request's exception without logging credentials or payloads.
            worker.exception()

    async def _execute(self, tool: str, request: str, deadline: float) -> str:
        vector_json = None
        if tool in {"search_operations", "search_evidence", "search_capabilities"} and self.embedder:
            query = json.loads(request).get("query")
            if isinstance(query, str):
                text = self.embedder.spec.query_text(query)
                try:
                    remaining = max(0.0, deadline - asyncio.get_running_loop().time())
                    values = await asyncio.wait_for(self.embedder.embed([text]), timeout=remaining)
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
        remaining_ms = int(max(0.0, deadline - asyncio.get_running_loop().time()) * 1000)
        if remaining_ms == 0:
            raise RuntimeError("resource_refused: native request deadline")
        return await asyncio.to_thread(self.session.execute, tool, request, vector_json, remaining_ms)

    async def close(self) -> None:
        self._closed = True
        if self._close_task is None:
            self._close_task = asyncio.create_task(self._drain())
        await asyncio.shield(self._close_task)

    async def _drain(self) -> None:
        await asyncio.gather(*tuple(self._pending), return_exceptions=True)
        await asyncio.to_thread(self.session.close)
