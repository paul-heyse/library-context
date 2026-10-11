"""One pinned native session, with bounded request admission and the retained query embedder."""

from __future__ import annotations

import asyncio
import hashlib
import json
import sys
from typing import TYPE_CHECKING

from lctx_semantics import NativeFailure, NativeSession, wire_decode, wire_failure

from lctx_mcp.embedder import EmbedderError

if TYPE_CHECKING:
    from lctx_mcp.embedder import Embedder


def _failure(kind: str) -> NativeFailure:
    raw = wire_failure(kind)
    error = NativeFailure(json.loads(raw)["message"])
    error.lctx_failure_json = raw
    return error


class NativeExecutor:
    def __init__(self, session: NativeSession, embedder: Embedder | None = None) -> None:
        self.session = session
        self.embedder = embedder
        self._closed = False
        self._pending: dict[asyncio.Task[str], object] = {}
        self._failures: dict[asyncio.Task[str], BaseException] = {}
        self._close_task: asyncio.Task[None] | None = None

    async def execute(self, tool: str, arguments: dict) -> str:
        deadline = asyncio.get_running_loop().time() + 30.0
        # Rust validates the public request before inference, allocation or a store query.
        request = wire_decode(
            tool, json.dumps(arguments, separators=(",", ":"), ensure_ascii=False)
        )
        if self._closed:
            raise _failure("unavailable")
        # The Rust session charges retained preparation before a worker or HTTP
        # wait exists. Native CPU/query demand begins only after embedding.
        owner = self.session.admit(tool, request)
        try:
            owner.retain(sys.getsizeof(request))
        except BaseException:
            owner.release()
            raise
        worker = asyncio.create_task(self._execute(owner, tool, request, deadline))
        self._pending[worker] = owner
        worker.add_done_callback(self._finished)
        try:
            remaining = max(0.0, deadline - asyncio.get_running_loop().time())
            done, _ = await asyncio.wait({worker}, timeout=remaining)
            if not done:
                raise TimeoutError
            return worker.result()
        except TimeoutError as exc:
            owner.cancel()
            raise _failure("resource_refused") from exc
        except asyncio.CancelledError:
            owner.cancel()
            raise
        except BaseException:
            # A delivered error stays with its caller; abandoned worker errors
            # remain owned until close composes them with session/client cleanup.
            self._failures.pop(worker, None)
            raise

    def _finished(self, worker: asyncio.Task[str]) -> None:
        self._pending.pop(worker, None)
        if worker.cancelled():
            self._failures[worker] = RuntimeError("native worker ownership was cancelled")
        elif error := worker.exception():
            self._failures[worker] = error

    async def _execute(self, owner, tool: str, request: str, deadline: float) -> str:
        try:
            return await self._prepare_and_execute(owner, tool, request, deadline)
        finally:
            # execute_async transfers the guard into its independent Rust task;
            # release is then a no-op. Preparation failures release it here.
            owner.release()

    async def _prepare_and_execute(self, owner, tool: str, request: str, deadline: float) -> str:
        owner.check()
        vector_json = None
        if (
            tool in {"search_operations", "search_evidence", "search_capabilities"}
            and self.embedder
            and not json.loads(request).get("page", {}).get("cursor")
        ):
            query = json.loads(request).get("query")
            if isinstance(query, str):
                text = self.embedder.spec.query_text(query)
                owner.retain(sys.getsizeof(text))
                try:
                    remaining = max(0.0, deadline - asyncio.get_running_loop().time())
                    values = await asyncio.wait_for(self.embedder.embed([text], retain=owner.retain), timeout=remaining)
                except EmbedderError:
                    # The backend reports the unavailable vector channel while lexical search runs.
                    values = None
                    vector_json = "unavailable"
                if values is not None:
                    vector = values[0].tolist()
                    owner.retain(sys.getsizeof(vector) + sum(map(sys.getsizeof, vector)))
                    vector_json = json.dumps(
                        {
                            "spec": list(bytes.fromhex(self.embedder.spec.hash)),
                            "input": list(hashlib.sha256(text.encode()).digest()),
                            "recipe": {
                                "template": self.embedder.spec.fields["query_template"],
                                "task": self.embedder.spec.fields["query_task"],
                                "max_tokens": self.embedder.spec.fields["max_query_tokens"],
                            },
                            "projection": list(bytes.fromhex(self.embedder.spec.projection_id)),
                            "vector": vector,
                        },
                        separators=(",", ":"),
                        allow_nan=False,
                    )
                    owner.retain(sys.getsizeof(vector_json))
        owner.check()
        remaining_ms = int(max(0.0, deadline - asyncio.get_running_loop().time()) * 1000)
        if remaining_ms == 0:
            raise _failure("resource_refused")
        return await owner.execute_async(vector_json, remaining_ms)

    async def close(self) -> None:
        if self._close_task is None:
            self._closed = True
            self.session.stop_accepting()
            for owner in tuple(self._pending.values()):
                owner.cancel()
            self._close_task = asyncio.create_task(self._drain())
            # Retrieve an abandoned close error; its cached task still returns
            # the identical exception to a later close caller.
            self._close_task.add_done_callback(lambda task: task.exception() if not task.cancelled() else None)
        await asyncio.wait({self._close_task})
        self._close_task.result()

    async def _drain(self) -> None:
        await asyncio.gather(*tuple(self._pending), return_exceptions=True)
        errors = list(self._failures.values())
        try:
            await self.session.close_async()
        except Exception as error:
            errors.append(error)
        if self.embedder:
            try:
                await self.embedder.aclose()
            except Exception as error:
                errors.append(error)
        if errors:
            raise ExceptionGroup("native request/session/client completion failed", errors)
