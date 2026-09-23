"""The FastMCP capability server over one pinned generation (DESIGN §11.3; ADR-0010).

- The lifespan loads the generation once, checks it against the schemas this server serves and
  the query client's spec, and yields it; tools read it from `ctx.lifespan_context`.
- `search_capabilities` ranks briefs by hybrid retrieval (§11.2). Without a query vector (no
  embedder, no vectors, or the service down) it answers lexically and says so.
- `get_capability` and the `capability://{snapshot_id}/{capability_id}` resource hydrate one
  brief by id, never by a second search.
- One domain exception, `CapabilityError`, becomes `ToolError` in a tool and `ResourceError` in the
  resource; anything else is masked (`mask_error_details=True`).
"""

from __future__ import annotations

from collections.abc import AsyncIterator
from contextlib import asynccontextmanager
from dataclasses import dataclass
from pathlib import Path
from typing import Annotated, Literal

import numpy as np
from fastmcp import Context, FastMCP
from fastmcp.exceptions import ResourceError, ToolError
from mcp_types import ToolAnnotations
from pydantic import BaseModel, Field

from lctx_mcp.embedder import Embedder, EmbedderError
from lctx_mcp.generation import Generation, load
from lctx_mcp.retrieval import Lexical, RankSource, fuse, ranks, vector_scores

INSTRUCTIONS = (
    "Capability briefs of one pinned Python library, compiled from its code, docs, examples and "
    "tests. Search with a coding task (search_capabilities), then read a brief whole "
    "(get_capability). Every statement carries its evidence status; 'unresolved' means the "
    "evidence could not fill it. A relevance score ranks briefs; it is not proof of task fit."
)

READ_ONLY = ToolAnnotations(read_only_hint=True, idempotent_hint=True, open_world_hint=False)

NOTE = "Relevance only ranks briefs. Read a brief's statuses and limits before relying on it."


class CapabilityError(Exception):
    """A request this generation cannot answer: an unknown library, snapshot or capability."""


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
    members: dict[bytes, list[str]] = {}
    for r in generation.tables["brief_members"].to_pylist():
        members.setdefault(r["brief_id"], []).append(r["access_path"])
    return Served(
        generation=generation,
        lexical=Lexical(generation.lexical),
        embedder=embedder,
        assertions=assertions,
        supports=supports,
        evidence={r["evidence_id"]: r for r in generation.tables["evidence"].to_pylist()},
        members=members,
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
        try:
            return await search(ctx.lifespan_context["served"], library, query, limit)
        except CapabilityError as e:
            raise ToolError(str(e)) from e

    @mcp.tool(annotations=READ_ONLY)
    def get_capability(snapshot_id: str, capability_id: str, ctx: Context) -> Capability:
        """One capability brief, whole: every section's statements with their evidence status,
        their supports, and the verbatim evidence they cite."""
        try:
            return hydrate(ctx.lifespan_context["served"], snapshot_id, capability_id)
        except CapabilityError as e:
            raise ToolError(str(e)) from e

    @mcp.resource("capability://{snapshot_id}/{capability_id}", mime_type="text/markdown")
    def capability(snapshot_id: str, capability_id: str, ctx: Context) -> str:
        """A capability brief as Markdown."""
        try:
            return markdown(hydrate(ctx.lifespan_context["served"], snapshot_id, capability_id))
        except CapabilityError as e:
            raise ResourceError(str(e)) from e

    return mcp
