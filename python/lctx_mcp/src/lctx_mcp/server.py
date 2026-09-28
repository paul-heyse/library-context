"""The FastMCP capability server over one pinned generation (DESIGN §11.3; ADR-0010).

- The lifespan loads the generation once, checks it against the schemas this server serves and
  the query client's spec, and yields it; tools read it from `ctx.lifespan_context`.
- `search_capabilities` ranks briefs by hybrid retrieval (§11.2). Without a query vector (no
  embedder, no vectors, or the service down) it answers lexically and says so.
- `get_capability` and the `capability://{snapshot_id}/{capability_id}` resource hydrate one
  brief by id, never by a second search.
- The behavioral tools (ADR-0021; §11.3) serve the whole public surface: `get_operation` by
  lookup, `find_operations` exhaustively over materialized facets, `search_operations` as ranked
  discovery (`lctx_mcp.operations`).
- One domain exception, `CapabilityError`, is FastMCP's `ValidationError` (the holistic
  assessment's A7): a tool returns it as an error result, and the resource answers it as invalid
  params (-32602), never as an internal error. Anything else is masked (`mask_error_details=True`).
"""

from __future__ import annotations

import asyncio
import json
import logging
from collections.abc import AsyncIterator
from concurrent.futures import ThreadPoolExecutor
from contextlib import asynccontextmanager
from dataclasses import dataclass
from functools import partial, wraps
from pathlib import Path
from typing import Annotated, Literal

import numpy as np
from fastmcp import Context, FastMCP
from fastmcp.exceptions import ToolError, ValidationError
from fastmcp.tools import ToolResult
from lctx_storage import StorageError, open_repository
from mcp_types import ToolAnnotations
from pydantic import BaseModel, Field, TypeAdapter

from lctx_mcp import operations as ops
from lctx_mcp import value_paths
from lctx_mcp.embedder import Embedder, EmbedderError
from lctx_mcp.generation import Generation, load
from lctx_mcp.retrieval import (
    Lexical,
    RankSource,
    RetrievalMetadata,
    fuse_legs,
    identities,
    lexical_rank,
    vector_legs,
)

INSTRUCTIONS = (
    "A behavioral model of one pinned Python library's whole public surface, compiled from its "
    "code, docs, examples and tests. find_operations answers exhaustively over typed facets "
    "(parameters, types, direct raises, decorators, and what an operation forwards to, delegates "
    "to or hands off to), saying whether the answer is complete; search_operations ranks "
    "operations for a task; get_operation reads one whole, each fate with its verdict and source "
    "line. Capability briefs cover a curated subset: search_capabilities, get_capability. "
    "'established' and 'conditional' admit may-behavior under the stated model; they do not "
    "prove that an execution exists. A negative verdict needs complete may-analysis. "
    "'unknown' and 'not_analyzed' are never 'no'. inspect_value_paths evaluates an exact "
    "primitive against individual cited value paths; its refutation is path-local only. "
    "A relevance score ranks; it is not proof."
)

READ_ONLY = ToolAnnotations(read_only_hint=True, idempotent_hint=True, open_world_hint=False)

NOTE = "Relevance only ranks briefs. Read a brief's statuses and limits before relying on it."


class CapabilityError(ValidationError):
    """A request this generation cannot answer: an unknown library, snapshot or capability. A
    client's bad input, so FastMCP answers it as invalid params in every component."""


class Hit(BaseModel):
    capability_id: str
    title: str
    outcome: str | None
    outcome_status: str
    relevance: float
    rank_source: RankSource
    promoted: bool


class CapabilityUnavailable(BaseModel):
    status: Literal["unavailable"] = "unavailable"
    reason: Literal["not_requested"] = "not_requested"
    capability: str
    snapshot_id: str
    generation: str


def unavailable(served: Served, capability: str) -> CapabilityUnavailable | None:
    if served.generation.manifest["capabilities"][capability]:
        return None
    return CapabilityUnavailable(
        capability=capability,
        snapshot_id=served.generation.snapshot_id,
        generation=served.generation.key,
    )


class SearchResult(BaseModel):
    retrieval: RetrievalMetadata

    library: str
    snapshot_id: str
    generation: str
    mode: Literal["hybrid", "lexical-only"]
    degraded_reason: str | None
    coverage: dict
    note: str
    hits: list[Hit]


class Evidence(BaseModel):
    evidence_id: str
    kind: str
    path: str | None
    start_byte: int | None
    end_byte: int | None
    text: str | None


class FindingWitness(BaseModel):
    path: int
    step: int
    call_site_node_id: str
    callee_node_id: str
    arc_kind: str
    modality: str
    phase: str | None
    source_fact_id: str | None
    source_path: str
    start_byte: int
    end_byte: int


class ConceptAttribute(BaseModel):
    attribute_id: str
    kind: str
    symbol: str | None
    parameter_kind: str | None
    type_term_id: str | None
    class_module: str | None
    class_key: str | None
    target_node_id: str | None
    modality: str | None
    phase: str | None
    producer_modality: str | None
    producer_phase: str | None
    display: str


class AttributeIncidence(BaseModel):
    finding_id: str
    incidence_id: str
    attribute_id: str
    object_node_id: str
    source_fact_id: str
    fact_table: str
    fact_model_id: str
    site_node_id: str | None
    edge_id: str | None
    other_site_node_id: str | None
    other_edge_id: str | None
    consumer_formal_id: str | None
    other_fact_id: str | None
    other_fact_table: str | None
    other_fact_model_id: str | None


class FindingMember(BaseModel):
    role: str
    ordinal: int
    node_id: str | None
    cited_fact_id: str | None
    attribute_id: str | None
    fact_table: str | None
    fact_model_id: str | None
    label: str | None


class FindingSupport(BaseModel):
    finding_id: str
    kind: str
    evidence_status: str
    subject_node_id: str
    related_node_id: str | None
    invocation_id: str
    model_id: str
    method: str
    parameters: str
    completion: str
    stop_reason: str | None
    witnesses_omitted: bool
    witnesses: list[FindingWitness]
    members: list[FindingMember]
    attributes: list[ConceptAttribute]
    attribute_incidences: list[AttributeIncidence]
    source_resolution: Literal["source_span", "fact_only", "unavailable"]


class Support(BaseModel):
    role: str
    finding_id: str | None
    finding_kind: str | None
    evidence_id: str | None
    finding: FindingSupport | None = None


class Assertion(BaseModel):
    assertion_id: str
    kind: str
    section: str
    status: str
    text: str | None
    applicable_case: str | None
    conditions: str | None
    limitations: str | None
    supports: list[Support]


class Capability(BaseModel):
    library: str
    snapshot_id: str
    generation: str
    capability_id: str
    title: str
    access_path: str
    public_paths: list[str]
    documentation_only: bool
    review_state: str
    outcome: str | None
    outcome_status: str
    assertions: list[Assertion]
    evidence: list[Evidence]
    # Slot sections this brief has no statement in: absent, not unresolved and not empty by
    # evidence (increment-1 deep review F4).
    sections_absent: list[str]


class NativeWorkers:
    """Two admitted CPU jobs; cancellation retains the slot until native work actually ends."""

    def __init__(self) -> None:
        self.pool = ThreadPoolExecutor(max_workers=2, thread_name_prefix="lctx-native")
        self.slots = asyncio.Semaphore(2)

    async def run(self, fn, *args):
        try:
            await asyncio.wait_for(self.slots.acquire(), 1.0)
        except TimeoutError as exc:
            raise ToolError("resource_refused: native worker capacity") from exc
        loop = asyncio.get_running_loop()
        try:
            future = loop.run_in_executor(self.pool, partial(fn, *args))
        except BaseException:
            self.slots.release()
            raise

        def completed(done):
            self.slots.release()
            if not done.cancelled():
                done.exception()

        future.add_done_callback(completed)
        try:
            return await asyncio.wait_for(asyncio.shield(future), 30.0)
        except TimeoutError as exc:
            raise ToolError("resource_refused: native request deadline") from exc

    async def close(self) -> None:
        await asyncio.to_thread(self.pool.shutdown, wait=True, cancel_futures=True)


@dataclass
class Served:
    generation: Generation
    lexical: Lexical | None
    operation_lexical: Lexical | None
    embedder: Embedder | None
    workers: NativeWorkers


def serve(generation: Generation, embedder: Embedder | None, workers: NativeWorkers) -> Served:
    state = generation.lexical_state
    return Served(
        generation,
        Lexical(state.brief_text) if state.brief_text else None,
        Lexical(state.operation_text) if state.operation_text else None,
        embedder,
        workers,
    )


REQUEST_SECONDS = 30.0


def request_deadline(fn):
    """One wall-clock budget covers eligibility, embedding, vector work and rendering."""

    @wraps(fn)
    async def wrapped(*args, **kwargs):
        try:
            async with asyncio.timeout(REQUEST_SECONDS):
                return await fn(*args, **kwargs)
        except TimeoutError as exc:
            raise ToolError("resource_refused: request deadline") from exc

    return wrapped


async def decode_model(served: Served, model, raw: str):
    return await served.workers.run(lambda: bounded(model.model_validate_json(raw)))


async def storage(awaitable):
    try:
        return await asyncio.wait_for(awaitable, 30.0)
    except StorageError as exc:
        raise ToolError(f"storage {exc.kind}: {exc}") from exc
    except ValueError as exc:
        raise CapabilityError(str(exc)) from exc
    except TimeoutError as exc:
        raise ToolError("storage unavailable: request deadline") from exc


def bounded(model):
    # Bound the actual Pydantic response including notes and metadata, not only Rust payloads.
    if len(model.model_dump_json().encode()) > 8 * 1024 * 1024:
        raise ToolError("resource_refused: serialized response byte budget")
    return model


@request_deadline
async def search(
    served: Served,
    library: str,
    query: str,
    limit: int,
    where: ops.Where | None = None,
    operations: bool = False,
):
    gen = served.generation
    if library != gen.library:
        raise CapabilityError(f"this server serves {gen.library!r}, not {library!r}")
    where_json = where.model_dump_json() if where else None
    raw_scope = await storage(gen.repository.search_scope(query, operations, where_json))

    def decode_scope():
        scope = json.loads(raw_scope)
        return identities(scope["eligible"]), identities(scope["promoted"])

    eligible, promoted = await served.workers.run(decode_scope)
    lexical = served.operation_lexical if operations else served.lexical
    ids = gen.lexical_state.operation_ids if operations else gen.lexical_state.brief_ids
    legs = [
        await served.workers.run(lambda: lexical_rank(ids, lexical.scores(query), eligible))
        if lexical
        else np.empty(0, dtype="V16")
    ]
    policy = gen.descriptor["policy"]
    metadata = RetrievalMetadata(
        profile=gen.descriptor["profile"],
        requested_route=policy["route"],
        actual_route="lexical-only",
    )
    reason = None
    relation = "operation_vectors" if operations else "vectors"
    if gen.manifest["relations"][relation]["rows"] == 0:
        reason = (
            "this generation has no operation vectors"
            if operations
            else "this generation has no vectors"
        )
    elif served.embedder is None:
        reason = "no query embedder is configured"
    else:
        try:
            query_vector = (await served.embedder.embed([served.embedder.spec.query_text(query)]))[
                0
            ]
        except EmbedderError as exc:
            reason = f"the embedding service failed: {exc}"
        else:
            raw_metadata, raw = await storage(
                gen.repository.vector_ranks(
                    query_vector, served.embedder.spec.hash, operations, where_json, limit
                )
            )
            metadata = RetrievalMetadata.model_validate_json(raw_metadata)
            legs.extend(await served.workers.run(vector_legs, raw))
    ranked = await served.workers.run(fuse_legs, legs, promoted, limit)
    raw_rows = await storage(gen.repository.hit_records([row[0] for row in ranked], operations))

    def render():
        rows = json.loads(raw_rows)
        hits: list[ops.OperationHit | Hit] = []
        for (identity, relevance, source, exact), row in zip(ranked, rows, strict=True):
            if operations:
                hits.append(
                    ops.OperationHit(
                        operation_id=identity,
                        access_path=row["access_path"],
                        kind=row["kind"],
                        docstring_summary=row["docstring_summary"],
                        relevance=relevance,
                        rank_source=source,
                        promoted=exact,
                    )
                )
            else:
                hits.append(
                    Hit(
                        capability_id=identity,
                        title=row["title"],
                        outcome=row["outcome"],
                        outcome_status=row["outcome_status"],
                        relevance=relevance,
                        rank_source=source,
                        promoted=exact,
                    )
                )
        common = dict(
            snapshot_id=gen.snapshot_id,
            generation=gen.key,
            mode="lexical-only" if reason else "hybrid",
            degraded_reason=reason,
            retrieval=metadata,
            hits=hits,
        )
        if operations:
            return bounded(
                ops.OperationHits.model_validate(
                    {**common, "ranked_discovery": True, "note": ops.NOTE_SEARCH}
                )
            )
        return bounded(
            SearchResult.model_validate(
                {**common, "library": gen.library, "coverage": gen.summary, "note": NOTE}
            )
        )

    return await served.workers.run(render)


async def hydrate(served: Served, snapshot_id: str, capability_id: str) -> Capability:
    return await decode_model(
        served,
        Capability,
        await storage(served.generation.repository.get_capability(snapshot_id, capability_id)),
    )


def markdown(c: Capability) -> str:
    """A brief as Markdown text, each statement with its status."""
    lines = [
        f"# {c.title}",
        "",
        f"- Library: `{c.library}`; snapshot `{c.snapshot_id}`; generation `{c.generation}`",
        f"- Review: {c.review_state}"
        + ("; documentation only" if c.documentation_only else "; analysis-backed"),
        f"- Public paths: {', '.join(f'`{p}`' for p in c.public_paths)}",
    ]
    section = None
    for a in c.assertions:
        if a.section != section:
            section = a.section
            lines += ["", f"## {section.replace('_', ' ').capitalize()}", ""]
        lines.append(f"- {a.text if a.text is not None else '(unresolved)'} [{a.status}]")
        for support in a.supports:
            if support.finding is not None:
                f = support.finding
                lines.append(
                    f"  - Finding `{f.finding_id}` ({support.role}; {f.kind}, "
                    f"{f.evidence_status}; invocation `{f.invocation_id}`, "
                    f"model `{f.model_id}`, {f.method}, {f.completion}; "
                    f"source {f.source_resolution})"
                )
                lines.append(f"    - Analysis parameters: `{f.parameters}`")
                for witness in f.witnesses:
                    lines.append(
                        f"    - Witness {witness.path}.{witness.step}: "
                        f"`{witness.source_path}:{witness.start_byte}-{witness.end_byte}` "
                        f"({witness.arc_kind}, {witness.modality}; "
                        f"fact `{witness.source_fact_id}`)"
                    )
                for member in f.members:
                    lines.append(
                        f"    - Member {member.role}.{member.ordinal}: "
                        f"node `{member.node_id}`, fact `{member.cited_fact_id}` "
                        f"({member.fact_table or 'source unavailable'}, "
                        f"model `{member.fact_model_id}`)"
                    )
            elif support.evidence_id is not None:
                lines.append(f"  - {support.role} evidence `{support.evidence_id}`")
    if c.sections_absent:
        absent = ", ".join(s.replace("_", " ") for s in c.sections_absent)
        lines += ["", f"No statement yet in: {absent}."]
    if c.evidence:
        lines += ["", "## Evidence", ""]
        for e in c.evidence:
            where = f"{e.path}:{e.start_byte}-{e.end_byte}" if e.path else e.kind
            lines.append(f"- `{e.evidence_id}` ({e.kind}, {where}): {e.text}")
    rendered = "\n".join(lines) + "\n"
    if len(rendered.encode()) > 8 * 1024 * 1024:
        raise ToolError("resource_refused: resource response byte budget")
    return rendered


def build_server(
    config_path: Path,
    embedder: Embedder | None,
    *,
    library: str,
    generation: str | None = None,
    profile: str | None = None,
) -> FastMCP:
    """Open the read-only repository once; pin selection once for the entire server lifespan."""

    @asynccontextmanager
    async def lifespan(_server: FastMCP) -> AsyncIterator[dict]:
        repository = await open_repository(config_path)
        workers = NativeWorkers()
        try:
            pinned = await repository.pin(library, generation, profile)
            descriptor = json.loads(pinned.descriptor())
            inputs = await pinned.inputs()
            state = await workers.run(
                load, pinned, descriptor, inputs, embedder.spec if embedder else None
            )
            del inputs
            served = await workers.run(serve, state, embedder, workers)
            logging.getLogger(__name__).info(
                "PostgreSQL serving generation=%s profile=%s route=%s",
                descriptor["generation"],
                descriptor["profile"],
                descriptor["policy"]["route"],
            )
            yield {"served": served}
        finally:
            try:
                await workers.close()
            finally:
                await repository.close()

    mcp = FastMCP("lctx", instructions=INSTRUCTIONS, lifespan=lifespan, mask_error_details=True)

    @mcp.tool(
        annotations=READ_ONLY,
        output_schema={
            **TypeAdapter(SearchResult | CapabilityUnavailable).json_schema(),
            "type": "object",
        },
    )
    @request_deadline
    async def search_capabilities(
        library: str,
        query: Annotated[str, Field(min_length=1, max_length=4000)],
        ctx: Context,
        limit: Annotated[int, Field(ge=1, le=10)] = 5,
    ) -> ToolResult:
        """Find capability briefs for a coding task in a library: ranked hits with their outcome
        and its evidence status. Read a hit whole with get_capability."""
        served = ctx.lifespan_context["served"]
        _check(served, library)
        missing = unavailable(served, "briefs")
        result = missing if missing is not None else await search(served, library, query, limit)
        return ToolResult(
            content=result.model_dump_json(), structured_content=result.model_dump(mode="json")
        )

    @mcp.tool(
        annotations=READ_ONLY,
        output_schema={
            **TypeAdapter(Capability | CapabilityUnavailable).json_schema(),
            "type": "object",
        },
    )
    @request_deadline
    async def get_capability(snapshot_id: str, capability_id: str, ctx: Context) -> ToolResult:
        """One capability brief, whole: every section's statements with their evidence status,
        their supports, and the verbatim evidence they cite."""
        served = ctx.lifespan_context["served"]
        if snapshot_id != served.generation.snapshot_id:
            raise CapabilityError("snapshot is not the pinned one")
        missing = unavailable(served, "briefs")
        result = (
            missing if missing is not None else await hydrate(served, snapshot_id, capability_id)
        )
        return ToolResult(
            content=result.model_dump_json(), structured_content=result.model_dump(mode="json")
        )

    def _check(served: Served, library: str) -> None:
        if library != served.generation.library:
            raise CapabilityError(
                f"this server serves {served.generation.library!r}, not {library!r}"
            )

    @mcp.tool(
        annotations=READ_ONLY,
        output_schema={
            **TypeAdapter(ops.Operation | ops.AmbiguousOperation).json_schema(),
            "type": "object",
        },
    )
    @request_deadline
    async def get_operation(
        snapshot_id: str,
        operation: Annotated[str, Field(min_length=1, max_length=500)],
        ctx: Context,
        expanded: bool = False,
    ) -> ToolResult:
        """One public operation, whole, by any public spelling (or its id): its paths, facets,
        each parameter's fates (forwarded, literal, raises-when, unfollowed) with verdicts and
        source lines, its delegations and official-usage handoffs, and its brief if one exists.
        Established and conditional fates are may-behavior admitted by the model, not concrete
        execution witnesses. A negative fate requires complete coverage under the model."""
        served = ctx.lifespan_context["served"]
        encoded = await storage(served.generation.repository.get_operation(snapshot_id, operation))
        model = (
            ops.AmbiguousOperation
            if json.loads(encoded).get("resolution") == "ambiguous"
            else ops.Operation
        )
        result = await decode_model(
            served,
            model,
            encoded,
        )
        payload = result.model_dump(mode="json")
        limit = 256 * 1024 if expanded else 32 * 1024
        if len(json.dumps(payload, ensure_ascii=False).encode()) > limit:
            raise CapabilityError(
                "operation packet exceeds byte budget; request expanded=true"
                if not expanded
                else "expanded operation packet exceeds 256 KiB; no signature was truncated"
            )
        return ToolResult(
            content="Public API contract and evidence; effective behavior may remain unresolved.",
            structured_content=payload,
        )

    @mcp.tool(
        annotations=READ_ONLY,
        output_schema={
            **TypeAdapter(value_paths.ValuePathPage | CapabilityUnavailable).json_schema(),
            "type": "object",
        },
    )
    @request_deadline
    async def inspect_value_paths(
        snapshot_id: str,
        operation: Annotated[str, Field(min_length=1, max_length=500)],
        formal: Annotated[str, Field(min_length=1, max_length=500)],
        exact_input: value_paths.ExactPrimitive,
        ctx: Context,
        standard_builtins: bool = False,
        limit: Annotated[int, Field(ge=1, le=50)] = 20,
        cursor: str | None = None,
    ) -> ToolResult:
        """Inspect cited value-summary paths for one public formal and exact primitive input.
        A refuted path is excluded only under that input model. A compatible path has a
        satisfiable Boolean model after checked value links, not a concrete execution. Other
        paths and open boundaries remain possible; there is no operation-wide negative."""
        served = ctx.lifespan_context["served"]
        if snapshot_id != served.generation.snapshot_id:
            raise CapabilityError("snapshot is not the pinned one")
        missing = unavailable(served, "native_value_paths")
        if missing is not None:
            return ToolResult(
                content=missing.model_dump_json(),
                structured_content=missing.model_dump(mode="json"),
            )
        resolved = json.loads(await storage(served.generation.repository.resolve(operation)))
        try:
            result = await served.workers.run(
                lambda: bounded(
                    value_paths.inspect(
                        served.generation,
                        snapshot_id,
                        resolved["access_path"],
                        formal,
                        exact_input,
                        standard_builtins,
                        limit,
                        cursor,
                    )
                )
            )
            return ToolResult(
                content=result.model_dump_json(), structured_content=result.model_dump(mode="json")
            )
        except ops.OperationError as exc:
            raise CapabilityError(str(exc)) from exc

    @mcp.tool(annotations=READ_ONLY)
    @request_deadline
    async def find_operations(
        library: str,
        where: ops.Where,
        ctx: Context,
        limit: Annotated[int, Field(ge=1, le=50)] = 20,
        cursor: str | None = None,
    ) -> ops.OperationSet:
        """Every public operation matching all the given facet terms (exact values), plus
        optional kind and path prefix. Exhaustive over this generation; `complete` is false when
        some operation that does not match has incomplete rows for a facet you used (a class
        without a public constructor, a behavior scan that met a boundary, a facet that is never
        complete such as `raises`), and those operations are listed in `unknown`. Positive
        behavior facets are may-behavior under the model. No negation."""
        served = ctx.lifespan_context["served"]
        _check(served, library)
        raw = await storage(
            served.generation.repository.find_operations(where.model_dump_json(), limit, cursor)
        )

        def render():
            result = json.loads(raw)
            result["note"] = ops.NOTE_FIND
            return bounded(ops.OperationSet.model_validate(result))

        return await served.workers.run(render)

    @mcp.tool(annotations=READ_ONLY)
    @request_deadline
    async def search_operations(
        library: str,
        query: Annotated[str, Field(min_length=1, max_length=4000)],
        ctx: Context,
        where: ops.Where | None = None,
        limit: Annotated[int, Field(ge=1, le=10)] = 5,
    ) -> ops.OperationHits:
        """Public operations ranked for a coding task (lexical and vector views, fused), within
        an optional facet filter. Ranked discovery, not exhaustive: confirm with get_operation."""
        served = ctx.lifespan_context["served"]
        _check(served, library)
        return await search(served, library, query, limit, where, operations=True)

    @mcp.resource("capability://{snapshot_id}/{capability_id}", mime_type="text/markdown")
    @request_deadline
    async def capability(snapshot_id: str, capability_id: str, ctx: Context) -> str:
        """A capability brief as Markdown."""
        served = ctx.lifespan_context["served"]
        if snapshot_id != served.generation.snapshot_id:
            raise CapabilityError("snapshot differs from the pinned generation")
        if missing := unavailable(served, "briefs"):
            return missing.model_dump_json()
        result = await hydrate(served, snapshot_id, capability_id)
        return await served.workers.run(markdown, result)

    return mcp
