"""Bounded inspection of cited value paths for one exact entry-formal input.

The native executor owns semantic evaluation. This adapter validates typed requests and binds
pagination to a snapshot, generation and exact query; it never promotes path-local refutation
to an operation-wide verdict.
"""

from __future__ import annotations

import base64
import hashlib
import json
from typing import Annotated, Literal

from pydantic import BaseModel, ConfigDict, Field, model_validator

from lctx_mcp.generation import Generation
from lctx_mcp.operations import OperationError, resolve


class ExactPrimitive(BaseModel):
    """An exact query-supplied builtin primitive, distinct from a Python type hint."""

    model_config = ConfigDict(strict=True, extra="forbid")

    kind: Literal["none", "bool", "int", "str"]
    value: None | bool | int | Annotated[str, Field(max_length=500)]

    @model_validator(mode="after")
    def matching_value(self) -> ExactPrimitive:
        expected = {"none": type(None), "bool": bool, "int": int, "str": str}[self.kind]
        if type(self.value) is not expected:
            raise ValueError(f"{self.kind} requires an exact {expected.__name__} value")
        return self

    def native(self) -> tuple[str, str]:
        if self.kind == "none":
            return self.kind, ""
        if self.kind == "bool":
            return self.kind, "true" if self.value else "false"
        return self.kind, str(self.value)


class ProofStep(BaseModel):
    kind: str
    evidence_id: str
    condition_id: str


class ValueLinkEvidence(BaseModel):
    link_id: str
    path: str | None
    start_byte: int
    end_byte: int


class ValuePath(BaseModel):
    summary_id: str
    source_verdict: str
    condition_id: str
    steps: list[ProofStep]
    exact_input_result: Literal["refuted_under_model", "compatible_under_model", "unknown"]
    value_links: list[ValueLinkEvidence]
    boundary_reason: str | None


class OpenBoundary(BaseModel):
    source_flow_fact_id: str
    condition_id: str
    reason: str


class ValuePathPage(BaseModel):
    snapshot_id: str
    generation: str
    operation: str
    formal: str
    exact_input: ExactPrimitive
    standard_builtins: bool
    paths: list[ValuePath]
    boundaries: list[OpenBoundary]
    total_rows: int
    examined_rows: int
    truncated: bool
    next_cursor: str | None
    note: str = (
        "A refutation applies only to its cited summary path under the exact input model. "
        "Compatibility means only a satisfiable model after checked value links, not a "
        "concrete execution. Unknown and absent paths do not establish absence."
    )


def _query_hash(gen: Generation, operation: str, formal: str, exact: ExactPrimitive,
                standard_builtins: bool) -> str:
    request = {"snapshot": gen.snapshot_id, "generation": gen.key, "operation": operation,
               "formal": formal, "exact": exact.model_dump(),
               "standard_builtins": standard_builtins}
    body = json.dumps(request, sort_keys=True, separators=(",", ":")).encode()
    return hashlib.sha256(body).hexdigest()[:32]


def _offset(gen: Generation, query_hash: str, cursor: str | None) -> int:
    if cursor is None:
        return 0
    try:
        data = json.loads(base64.b64decode(cursor.encode(), altchars=b"-_", validate=True))
        if data["generation"] != gen.key or data["query"] != query_hash:
            raise OperationError("cursor belongs to another generation or query")
        offset = data["offset"]
        if type(offset) is not int or offset < 0:
            raise ValueError("invalid offset")
    except (KeyError, TypeError, ValueError) as e:
        raise OperationError("invalid value-path cursor") from e
    return offset


def inspect(
    gen: Generation, snapshot_id: str, operation: str, formal: str,
    exact: ExactPrimitive, standard_builtins: bool, limit: int, cursor: str | None,
) -> ValuePathPage:
    """One bounded native page, with no operation-wide negative claim."""
    if snapshot_id != gen.snapshot_id:
        raise OperationError(f"this server serves snapshot {gen.snapshot_id}, not {snapshot_id}")
    if not 1 <= limit <= 50:
        raise OperationError("limit must be between 1 and 50")
    node = resolve(gen, operation)
    path = gen.operations[node]["access_path"]
    query_hash = _query_hash(gen, path, formal, exact, standard_builtins)
    offset = _offset(gen, query_hash, cursor)
    kind, value = exact.native()
    native = gen.condition_graph
    if native is None:
        raise OperationError("this generation has no native semantic index")
    try:
        rows, open_rows, total, truncated, work = native.inspect_value_paths(
            path, formal, kind, value, standard_builtins, offset, limit
        )
    except ValueError as e:
        raise OperationError(str(e)) from e
    next_cursor = None
    if truncated:
        body = {"generation": gen.key, "query": query_hash, "offset": offset + work}
        next_cursor = base64.urlsafe_b64encode(
            json.dumps(body, sort_keys=True, separators=(",", ":")).encode()
        ).decode()
    return ValuePathPage(
        snapshot_id=gen.snapshot_id, generation=gen.key, operation=path, formal=formal,
        exact_input=exact, standard_builtins=standard_builtins,
        paths=[ValuePath(
            summary_id=row[0], source_verdict=row[1], condition_id=row[2],
            steps=[ProofStep(kind=s[0], evidence_id=s[1], condition_id=s[2]) for s in row[3]],
            exact_input_result=row[4],
            value_links=[ValueLinkEvidence(link_id=e[0], path=e[1], start_byte=e[2],
                                           end_byte=e[3]) for e in row[5]],
            boundary_reason=row[6],
        ) for row in rows],
        boundaries=[OpenBoundary(source_flow_fact_id=r[0], condition_id=r[1], reason=r[2])
                    for r in open_rows],
        total_rows=total, examined_rows=work, truncated=truncated, next_cursor=next_cursor,
    )
