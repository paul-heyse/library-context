"""Bounded inspection of cited value paths for one exact entry-formal input.

The native executor owns semantic evaluation. This adapter validates typed requests and binds
pagination to a snapshot, generation and exact query; it never promotes path-local refutation
to an operation-wide verdict.
"""

from __future__ import annotations

import json
from typing import Protocol

from lctx_semantics import SemanticExecutor

from lctx_mcp.operations import OperationError
from lctx_mcp.wire import Contract, Packet


class NativeGeneration(Protocol):
    """Pure native input boundary; inspection needs no database repository."""

    @property
    def snapshot_id(self) -> str: ...

    @property
    def key(self) -> str: ...

    @property
    def condition_graph(self) -> SemanticExecutor | None: ...


ExactPrimitive = Contract("ExactPrimitive")


ProofStep = Contract("ProofStep")


ValueLinkEvidence = Contract("ValueLinkEvidence")


TheoryWork = Contract("TheoryWork")


ValuePath = Contract("ValuePath")


OpenBoundary = Contract("OpenBoundary")


ValuePathPage = Contract("ValuePathPage")


def inspect(
    gen: NativeGeneration,
    snapshot_id: str,
    operation: str,
    formal: str,
    exact: Packet,
    standard_builtins: bool,
    limit: int,
    cursor: str | None,
) -> Packet:
    """Transport one typed request; Rust owns validation, cursor and page semantics."""
    if snapshot_id != gen.snapshot_id:
        raise OperationError(f"this server serves snapshot {gen.snapshot_id}, not {snapshot_id}")
    native = gen.condition_graph
    if native is None:
        raise OperationError("this generation has no native semantic index")
    request = dict(
        snapshot_id=snapshot_id,
        operation=operation,
        formal=formal,
        exact_input=exact.model_dump(),
        standard_builtins=standard_builtins,
        limit=limit,
        cursor=cursor,
    )
    try:
        return ValuePathPage.model_validate_json(
            native.inspect_value_page(gen.key, json.dumps(request))
        )
    except ValueError as exc:
        raise OperationError(str(exc)) from exc
