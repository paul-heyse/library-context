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

from collections.abc import AsyncIterator
from contextlib import asynccontextmanager
from dataclasses import dataclass
from pathlib import Path
from typing import Annotated, Literal

import numpy as np
from fastmcp import Context, FastMCP
from fastmcp.exceptions import ValidationError
from mcp_types import ToolAnnotations
from pydantic import BaseModel, Field

from lctx_mcp import operations as ops
from lctx_mcp import value_paths
from lctx_mcp.embedder import Embedder, EmbedderError
from lctx_mcp.generation import Generation, load
from lctx_mcp.retrieval import Lexical, RankSource, fuse, ranks, vector_scores

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


class SearchResult(BaseModel):
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


class Support(BaseModel):
    role: str
    finding_id: str | None
    finding_kind: str | None
    evidence_id: str | None


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


@dataclass
class Served:
    """What the lifespan yields: the generation, its lexical index, and the query embedder."""

    generation: Generation
    lexical: Lexical
    embedder: Embedder | None
    assertions: dict[bytes, list[dict]]
    supports: dict[bytes, list[dict]]
    evidence: dict[bytes, dict]
    members: dict[bytes, list[str]]
    operations: ops.OperationIndex


def _hex(b: bytes | None) -> str | None:
    return None if b is None else b.hex()


def serve(generation: Generation, embedder: Embedder | None) -> Served:
    """Index a loaded generation for serving."""
    assertions: dict[bytes, list[dict]] = {}
    for r in generation.tables["assertions"].to_pylist():
        assertions.setdefault(r["brief_id"], []).append(r)
    supports: dict[bytes, list[dict]] = {}
    for r in generation.tables["supports"].to_pylist():
        supports.setdefault(r["assertion_id"], []).append(r)
    # A brief is shown by its own public paths; the inherited ones only promote (FORMAT 2).
    members: dict[bytes, list[str]] = {}
    for r in generation.tables["brief_members"].to_pylist():
        if r["own"]:
            members.setdefault(r["brief_id"], []).append(r["access_path"])
    return Served(
        generation=generation,
        lexical=Lexical(generation.lexical),
        embedder=embedder,
        assertions=assertions,
        supports=supports,
        evidence={r["evidence_id"]: r for r in generation.tables["evidence"].to_pylist()},
        members=members,
        operations=ops.OperationIndex(generation),
    )


async def search(served: Served, library: str, query: str, limit: int) -> SearchResult:
    """§11.2: lexical, vector, fused, exact symbols promoted; degraded to lexical when needed."""
    gen = served.generation
    if library != gen.library:
        raise CapabilityError(f"this server serves {gen.library!r}, not {library!r}")
    lexical_scores = dict(zip(gen.brief_ids, served.lexical.scores(query).tolist(), strict=True))
    lexical = ranks(lexical_scores, positive_only=True)
    vector: dict[bytes, int] = {}
    reason: str | None = None
    if gen.vectors is None:
        reason = "this generation has no vectors"
    elif served.embedder is None:
        reason = "no query embedder is configured"
    else:
        try:
            q = (await served.embedder.embed([served.embedder.spec.query_text(query)]))[0]
            vector = ranks(vector_scores(gen.vectors, gen.vector_rows, np.asarray(q)), False)
        except EmbedderError as e:
            reason = f"the embedding service failed: {e}"
    promoted = set(gen.symbols.get(query.strip(), []))
    hits = []
    for r in fuse(lexical, vector, promoted, limit):
        brief = gen.briefs[r.brief_id]
        hits.append(
            Hit(
                capability_id=r.brief_id.hex(),
                title=brief["title"],
                outcome=brief["outcome"],
                outcome_status=brief["outcome_status"],
                relevance=r.relevance,
                rank_source=r.rank_source,
                promoted=r.promoted,
            )
        )
    return SearchResult(
        library=gen.library,
        snapshot_id=gen.snapshot_id,
        generation=gen.key,
        mode="lexical-only" if reason else "hybrid",
        degraded_reason=reason,
        coverage=gen.manifest["summary"],
        note=NOTE,
        hits=hits,
    )


def hydrate(served: Served, snapshot_id: str, capability_id: str) -> Capability:
    """One brief, whole, by deterministic lookup."""
    gen = served.generation
    if snapshot_id != gen.snapshot_id:
        raise CapabilityError(
            f"snapshot {snapshot_id} is not the served one ({gen.snapshot_id}); search again"
        )
    try:
        brief_id = bytes.fromhex(capability_id)
    except ValueError as e:
        raise CapabilityError(f"{capability_id!r} is not a capability id") from e
    brief = gen.briefs.get(brief_id)
    if brief is None:
        raise CapabilityError(f"no capability {capability_id} in snapshot {snapshot_id}")
    assertions = []
    cited: list[bytes] = []
    for a in served.assertions.get(brief_id, []):
        rows = served.supports.get(a["assertion_id"], [])
        for s in rows:
            if s["evidence_id"] is not None and s["evidence_id"] not in cited:
                cited.append(s["evidence_id"])
        assertions.append(
            Assertion(
                assertion_id=a["assertion_id"].hex(),
                kind=a["kind"],
                section=a["section"],
                status=a["status"],
                text=a["text"],
                applicable_case=a["applicable_case"],
                conditions=a["conditions"],
                limitations=a["limitations"],
                supports=[
                    Support(
                        role=s["role"],
                        finding_id=_hex(s["finding_id"]),
                        finding_kind=s["finding_kind"],
                        evidence_id=_hex(s["evidence_id"]),
                    )
                    for s in rows
                ],
            )
        )
    evidence = [
        Evidence(
            evidence_id=e["evidence_id"].hex(),
            kind=e["kind"],
            path=e["path"],
            start_byte=e["start_byte"],
            end_byte=e["end_byte"],
            text=e["text"],
        )
        for e in (served.evidence[i] for i in cited if i in served.evidence)
    ]
    present = {a.section for a in assertions}
    absent = [s for s in gen.manifest["summary"]["slot_sections"] if s not in present]
    return Capability(
        library=gen.library,
        snapshot_id=gen.snapshot_id,
        generation=gen.key,
        capability_id=capability_id,
        title=brief["title"],
        access_path=brief["access_path"],
        public_paths=served.members.get(brief_id, []),
        documentation_only=brief["documentation_only"],
        review_state=brief["review_state"],
        outcome=brief["outcome"],
        outcome_status=brief["outcome_status"],
        assertions=assertions,
        evidence=evidence,
        sections_absent=absent,
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
    if c.sections_absent:
        absent = ", ".join(s.replace("_", " ") for s in c.sections_absent)
        lines += ["", f"No statement yet in: {absent}."]
    if c.evidence:
        lines += ["", "## Evidence", ""]
        for e in c.evidence:
            where = f"{e.path}:{e.start_byte}-{e.end_byte}" if e.path else e.kind
            lines.append(f"- `{e.evidence_id}` ({e.kind}, {where}): {e.text}")
    return "\n".join(lines) + "\n"


def build_server(generation_dir: Path, embedder: Embedder | None) -> FastMCP:
    """The server over the generation at `generation_dir`, embedding queries with `embedder`."""

    @asynccontextmanager
    async def lifespan(_server: FastMCP) -> AsyncIterator[dict]:
        spec = embedder.spec if embedder is not None else None
        yield {"served": serve(load(generation_dir, spec), embedder)}

    mcp = FastMCP("lctx", instructions=INSTRUCTIONS, lifespan=lifespan, mask_error_details=True)

    @mcp.tool(annotations=READ_ONLY)
    async def search_capabilities(
        library: str,
        query: Annotated[str, Field(min_length=1, max_length=4000)],
        ctx: Context,
        limit: Annotated[int, Field(ge=1, le=10)] = 5,
    ) -> SearchResult:
        """Find capability briefs for a coding task in a library: ranked hits with their outcome
        and its evidence status. Read a hit whole with get_capability."""
        return await search(ctx.lifespan_context["served"], library, query, limit)

    @mcp.tool(annotations=READ_ONLY)
    def get_capability(snapshot_id: str, capability_id: str, ctx: Context) -> Capability:
        """One capability brief, whole: every section's statements with their evidence status,
        their supports, and the verbatim evidence they cite."""
        return hydrate(ctx.lifespan_context["served"], snapshot_id, capability_id)

    def _check(served: Served, library: str) -> None:
        if library != served.generation.library:
            raise CapabilityError(
                f"this server serves {served.generation.library!r}, not {library!r}"
            )

    @mcp.tool(annotations=READ_ONLY)
    def get_operation(
        snapshot_id: str,
        operation: Annotated[str, Field(min_length=1, max_length=500)],
        ctx: Context,
    ) -> ops.Operation:
        """One public operation, whole, by any public spelling (or its id): its paths, facets,
        each parameter's fates (forwarded, literal, raises-when, unfollowed) with verdicts and
        source lines, its delegations and official-usage handoffs, and its brief if one exists.
        Established and conditional fates are may-behavior admitted by the model, not concrete
        execution witnesses. A negative fate requires complete coverage under the model."""
        try:
            return ops.get_operation(
                ctx.lifespan_context["served"].generation, snapshot_id, operation
            )
        except ops.OperationError as e:
            raise CapabilityError(str(e)) from e

    @mcp.tool(annotations=READ_ONLY)
    def inspect_value_paths(
        snapshot_id: str,
        operation: Annotated[str, Field(min_length=1, max_length=500)],
        formal: Annotated[str, Field(min_length=1, max_length=500)],
        exact_input: value_paths.ExactPrimitive,
        ctx: Context,
        standard_builtins: bool = False,
        limit: Annotated[int, Field(ge=1, le=50)] = 20,
        cursor: str | None = None,
    ) -> value_paths.ValuePathPage:
        """Inspect cited value-summary paths for one public formal and exact primitive input.
        A refuted path is excluded only under that input model. A compatible path has a
        satisfiable Boolean model after checked value links, not a concrete execution. Other
        paths and open boundaries remain possible; there is no operation-wide negative."""
        try:
            return value_paths.inspect(
                ctx.lifespan_context["served"].generation, snapshot_id, operation, formal,
                exact_input, standard_builtins, limit, cursor,
            )
        except ops.OperationError as e:
            raise CapabilityError(str(e)) from e

    @mcp.tool(annotations=READ_ONLY)
    def find_operations(
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
        try:
            return ops.find_operations(served.generation, where, limit, cursor)
        except ops.OperationError as e:
            raise CapabilityError(str(e)) from e

    @mcp.tool(annotations=READ_ONLY)
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
        return await ops.search_operations(
            served.generation, served.operations, served.embedder, query, where, limit
        )

    @mcp.resource("capability://{snapshot_id}/{capability_id}", mime_type="text/markdown")
    def capability(snapshot_id: str, capability_id: str, ctx: Context) -> str:
        """A capability brief as Markdown."""
        return markdown(hydrate(ctx.lifespan_context["served"], snapshot_id, capability_id))

    return mcp
