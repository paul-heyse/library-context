"""The behavioral tools over one generation's public surface (ADR-0021; DESIGN §11.3).

- `get_operation`: one public operation's record by deterministic lookup.
- `find_operations`: an **exhaustive** conjunction of typed facet terms over the materialized
  `operation_facets`, with `complete` and `unknown` read from the served
  `operation_facet_status` (increment 3's deep review, F3 and F4): no facet class is decided here.
- `search_operations`: **ranked** discovery (BM25 over `operation_text`, cosine per embedded view,
  reciprocal-rank fusion), labelled as such; an exact public spelling is promoted.

Nothing here decides semantics: every fate, facet and verdict was computed at compile time.
"""

from __future__ import annotations

import base64
import difflib
import hashlib
import json
from typing import Annotated, Literal

import numpy as np
from pydantic import BaseModel, Field

from lctx_mcp.generation import Generation
from lctx_mcp.retrieval import K, Lexical, ranks, vector_scores

# The facet names, as the `operation_facet` codebook spells them: `specs/serving/facets.json`
# holds them for both languages (`test_the_facet_names_are_the_codebook's`).
FacetName = Literal[
    "parameter",
    "parameter_type",
    "returns",
    "raises",
    "decorator",
    "async",
    "delegates_to",
    "forwards_to",
    "hands_off_to",
    "takes_from",
    "module",
    "kind",
]
UNKNOWN_CAP = 50
# Rows whose verdict lets them match; an `unknown` row puts its operation in `unknown` instead.
MATCHING = frozenset({"established", "conditional"})
NOTE_FIND = (
    "Exhaustive over the materialized facets of this generation. `complete` is true only when "
    "every operation that does not match has complete rows for every facet asked "
    "(`operation_facet_status`). `unknown` lists the operations that could still match: one whose "
    "rows for a facet are not complete (a class without a public constructor, a scan that met a "
    "boundary, a facet that is never complete such as `raises` or the handoffs), or whose row is "
    "`unknown` (a path through an override-open call)."
)
NOTE_SEARCH = (
    "Ranked discovery, not exhaustive: a score ranks operations, it is never evidence that one "
    "fits the task. Use find_operations for exhaustive answers and get_operation for evidence."
)


class OperationError(ValueError):
    """An invalid request (an unknown operation, snapshot, facet value or cursor)."""


class FacetTerm(BaseModel):
    """One term of a conjunction: an operation has this facet with exactly this value."""

    facet: FacetName
    value: Annotated[str, Field(min_length=1, max_length=500)]


class Where(BaseModel):
    """A conjunction of terms; no negation (Stage 1 makes no negative claims)."""

    facets: list[FacetTerm] = Field(default_factory=list, max_length=10)
    kind: Literal["function", "method", "class"] | None = None
    path_prefix: Annotated[str, Field(max_length=500)] | None = None


class Fate(BaseModel):
    """What one behavior row states, with its verdict and where it is shown."""

    kind: str
    parameter: str | None
    callee: str | None
    target: str | None
    value: str | None
    depth: int
    conditional: bool
    verdict: str
    # Why the verdict is `unknown`: `override_dispatch`, `ambiguous_binding`,
    # `outside_provider_model`; none when established or conditional.
    boundary_reason: str | None
    occurrences: int
    path: str | None
    line: int | None
    site_text: str | None


class ParameterRecord(BaseModel):
    """A parameter's fates; with none, its other channels are not analyzed (never "unused")."""

    name: str
    fates: list[Fate]
    note: str | None


class Operation(BaseModel):
    """One public operation, whole."""

    snapshot_id: str
    generation: str
    operation_id: str
    access_path: str
    own_paths: list[str]
    inherited_paths: list[str]
    kind: str
    is_method: bool
    qualified_name: str
    module: str
    docstring_summary: str | None
    behavior_status: str
    boundary_reason: str | None
    status_reason: str | None
    capability_id: str | None
    facets: dict[str, list[str]]
    # The facets whose rows are not complete for this operation, with why.
    incomplete_facets: dict[str, str]
    parameters: list[ParameterRecord]
    delegates: list[Fate]
    handoffs: list[Fate]
    # A class's controls are its constructor's: the `__init__` record, when one is public.
    constructor: Operation | None = None


class OperationRef(BaseModel):
    """An operation as a list shows it."""

    operation_id: str
    access_path: str
    kind: str
    docstring_summary: str | None
    behavior_status: str


class OperationSet(BaseModel):
    """`find_operations`' answer."""

    snapshot_id: str
    generation: str
    matches: list[OperationRef]
    total: int
    complete: bool
    unknown: list[OperationRef]
    unknown_total: int
    unknown_truncated: bool
    truncated: bool
    next_cursor: str | None
    note: str


class OperationHit(BaseModel):
    """One ranked operation."""

    operation_id: str
    access_path: str
    kind: str
    docstring_summary: str | None
    relevance: float
    rank_source: str
    promoted: bool


class OperationHits(BaseModel):
    """`search_operations`' answer."""

    snapshot_id: str
    generation: str
    mode: Literal["hybrid", "lexical-only"]
    degraded_reason: str | None
    ranked_discovery: bool
    hits: list[OperationHit]
    note: str


def _ref(gen: Generation, node: bytes) -> OperationRef:
    o = gen.operations[node]
    return OperationRef(
        operation_id=node.hex(),
        access_path=o["access_path"],
        kind=o["kind"],
        docstring_summary=o["docstring_summary"],
        behavior_status=o["behavior_status"],
    )


def _fate(r: dict) -> Fate:
    return Fate(
        kind=r["kind"],
        parameter=r["parameter_name"],
        callee=r["callee"],
        target=r["target_name"],
        value=r["value"],
        depth=r["depth"],
        conditional=r["conditional"],
        verdict=r["verdict"],
        boundary_reason=r["boundary_reason"],
        occurrences=r["occurrences"],
        path=r["path"],
        line=r["line"],
        site_text=r["site_text"],
    )


def resolve(gen: Generation, operation: str) -> bytes:
    """An operation by any public spelling or by its hex id."""
    node = gen.paths.get(operation.strip())
    if node is None:
        try:
            raw = bytes.fromhex(operation.strip())
        except ValueError:
            raw = b""
        node = raw if raw in gen.operations else None
    if node is None or node not in gen.operations:
        near = difflib.get_close_matches(operation.strip(), list(gen.paths), n=3)
        hint = f"; did you mean {', '.join(near)}?" if near else ""
        raise OperationError(f"no public operation {operation!r} in this generation{hint}")
    return node


def get_operation(gen: Generation, snapshot_id: str, operation: str) -> Operation:
    """§11.3 `get_operation`: one operation's record, by deterministic lookup."""
    if snapshot_id != gen.snapshot_id:
        raise OperationError(f"this server serves snapshot {gen.snapshot_id}, not {snapshot_id}")
    return _record(gen, resolve(gen, operation), operation.strip())


def _record(gen: Generation, node: bytes, spelling: str) -> Operation:
    """One operation's record; a class also carries its constructor's."""
    o = gen.operations[node]
    facets: dict[str, list[str]] = {}
    for facet, value, _ in gen.facets.get(node, []):
        facets.setdefault(facet, []).append(value)
    incomplete = {
        facet: f"{verdict}: {reason}" if reason else verdict
        for facet, (verdict, reason) in sorted(gen.facet_status.get(node, {}).items())
        if verdict != "established"
    }
    rows = gen.behaviors.get(node, [])
    per_parameter: dict[str, list[Fate]] = {
        name: [] for name in facets.get("parameter", []) if not name.startswith("*")
    }
    for r in rows:
        if r["kind"] in ("forwards", "raises_when", "unfollowed") and r["parameter_name"]:
            per_parameter.setdefault(r["parameter_name"], []).append(_fate(r))
    parameters = [
        ParameterRecord(
            name=name,
            fates=fates,
            note=None
            if fates
            else "no fate found: Stage 1 sees reads at call arguments only; stores, returns "
            "and tests are not analyzed (never read this as unused)",
        )
        for name, fates in per_parameter.items()
    ]
    supplies = [_fate(r) for r in rows if r["kind"] == "supplies_literal"]
    return Operation(
        snapshot_id=gen.snapshot_id,
        generation=gen.key,
        operation_id=node.hex(),
        access_path=o["access_path"],
        own_paths=sorted(p for p, own in gen.spellings.get(node, []) if own),
        inherited_paths=sorted(p for p, own in gen.spellings.get(node, []) if not own),
        kind=o["kind"],
        is_method=o["is_method"],
        qualified_name=o["qualified_name"],
        module=o["module"],
        docstring_summary=o["docstring_summary"],
        behavior_status=o["behavior_status"],
        boundary_reason=o["boundary_reason"],
        status_reason=o["status_reason"],
        capability_id=o["brief_id"].hex() if o["brief_id"] else None,
        facets=facets,
        incomplete_facets=incomplete,
        parameters=parameters,
        delegates=[_fate(r) for r in rows if r["kind"] == "delegates"] + supplies,
        handoffs=[_fate(r) for r in rows if r["kind"] in ("hands_off_to", "takes_from")],
        constructor=_constructor(gen, node, spelling),
    )


def _constructor(gen: Generation, node: bytes, spelling: str) -> Operation | None:
    """A class's `__init__` record, by the spelling asked for, else by any of its paths."""
    if gen.operations[node]["kind"] != "class":
        return None
    for path in [spelling] + [p for p, _ in gen.spellings.get(node, [])]:
        init = gen.paths.get(f"{path}.__init__")
        if init is not None and init in gen.operations:
            return _record(gen, init, f"{path}.__init__")
    return None


Operation.model_rebuild()


def _universe(gen: Generation, where: Where) -> list[bytes]:
    out = []
    for node in gen.op_ids:
        if where.kind is not None:
            kind = next((v for f, v, _ in gen.facets.get(node, []) if f == "kind"), None)
            if kind != where.kind:
                continue
        if where.path_prefix is not None and not any(
            p.startswith(where.path_prefix) for p, _ in gen.spellings.get(node, [])
        ):
            continue
        out.append(node)
    return out


def _request_hash(where: Where) -> str:
    canonical = json.dumps(where.model_dump(), sort_keys=True, separators=(",", ":"))
    return hashlib.sha256(canonical.encode()).hexdigest()[:16]


def _cursor(gen: Generation, where: Where, offset: int) -> str:
    raw = json.dumps({"g": gen.key, "h": _request_hash(where), "o": offset}).encode()
    return base64.urlsafe_b64encode(raw).decode()


def _offset(gen: Generation, where: Where, cursor: str | None) -> int:
    if cursor is None:
        return 0
    try:
        data = json.loads(base64.urlsafe_b64decode(cursor.encode()))
        ok = data["g"] == gen.key and data["h"] == _request_hash(where)
        offset = int(data["o"])
    except (ValueError, KeyError, TypeError) as e:
        raise OperationError("the cursor is not one this server issued") from e
    if not ok or offset < 0:
        raise OperationError("the cursor belongs to another generation or request")
    return offset


def _check_terms(gen: Generation, where: Where) -> None:
    """Refuse a term no operation has, but only where every operation's rows for its facet are
    complete; elsewhere absence is not known (increment 3's deep review, F3)."""
    for term in where.facets:
        if (term.facet, term.value) in gen.by_facet:
            continue
        if all(
            gen.facet_status.get(n, {}).get(term.facet, ("unknown", None))[0] == "established"
            for n in gen.op_ids
        ):
            known = sorted({v for f, v in gen.by_facet if f == term.facet})
            near = difflib.get_close_matches(term.value, known, n=5)
            hint = f"; close values: {', '.join(near)}" if near else ""
            raise OperationError(
                f"no operation has {term.facet} = {term.value!r} in this generation{hint}"
            )


def _term(gen: Generation, node: bytes, term: FacetTerm) -> str:
    """How one operation stands on one term: `match`, `no` (complete and absent) or `open`."""
    verdict = gen.by_facet.get((term.facet, term.value), {}).get(node)
    if verdict in MATCHING:
        return "match"
    status = gen.facet_status.get(node, {}).get(term.facet, ("unknown", None))[0]
    if verdict is None and status == "established":
        return "no"
    return "open"


def find_operations(gen: Generation, where: Where, limit: int, cursor: str | None) -> OperationSet:
    """§11.3 `find_operations`: exhaustive over the materialized facets."""
    _check_terms(gen, where)
    universe = _universe(gen, where)
    matched: list[bytes] = []
    hiding: list[bytes] = []
    for n in universe:
        standing = [_term(gen, n, t) for t in where.facets]
        if all(s == "match" for s in standing):
            matched.append(n)
        elif "no" not in standing:
            # Every term is matched or could be: the operation could still match.
            hiding.append(n)
    matched.sort(key=lambda n: gen.operations[n]["access_path"])
    unknown = sorted(hiding, key=lambda n: gen.operations[n]["access_path"])
    offset = _offset(gen, where, cursor)
    page = matched[offset : offset + limit]
    more = offset + limit < len(matched)
    return OperationSet(
        snapshot_id=gen.snapshot_id,
        generation=gen.key,
        matches=[_ref(gen, n) for n in page],
        total=len(matched),
        complete=not unknown,
        unknown=[_ref(gen, n) for n in unknown[:UNKNOWN_CAP]],
        unknown_total=len(unknown),
        unknown_truncated=len(unknown) > UNKNOWN_CAP,
        truncated=more,
        next_cursor=_cursor(gen, where, offset + limit) if more else None,
        note=NOTE_FIND,
    )


def fuse_legs(
    legs: list[dict[bytes, int]], promoted: set[bytes], limit: int
) -> list[tuple[bytes, float, str, bool]]:
    """Reciprocal-rank fusion over any number of rankings, promoted operations first."""
    fused = {
        n: sum(1.0 / (K + leg[n]) for leg in legs if n in leg) for n in set().union(*legs, promoted)
    }
    for n in promoted:
        fused[n] += 1.0
    order = sorted(fused, key=lambda n: (-fused[n], n))[:limit]
    out = []
    for n in order:
        present = [i for i, leg in enumerate(legs) if n in leg]
        if n in promoted:
            source = "exact_symbol"
        elif len(present) > 1:
            source = "hybrid"
        elif present == [0]:
            source = "lexical"
        else:
            source = "vector"
        out.append((n, round(fused[n], 6), source, n in promoted))
    return out


class OperationIndex:
    """The lexical index over the operations' texts, built once per generation."""

    def __init__(self, gen: Generation) -> None:
        self.lexical = Lexical(gen.op_text) if gen.op_text else None


async def search_operations(
    gen: Generation,
    index: OperationIndex,
    embedder,
    query: str,
    where: Where | None,
    limit: int,
) -> OperationHits:
    """§11.3 `search_operations`: ranked discovery over operations."""
    allowed = set(_universe(gen, where)) if where is not None else set(gen.op_ids)
    if where is not None:
        _check_terms(gen, where)
        allowed = {n for n in allowed if all(_term(gen, n, t) == "match" for t in where.facets)}
    legs: list[dict[bytes, int]] = []
    if index.lexical is not None:
        scores = dict(zip(gen.op_ids, index.lexical.scores(query).tolist(), strict=True))
        legs.append(ranks({n: s for n, s in scores.items() if n in allowed}, positive_only=True))
    reason: str | None = None
    if not gen.op_vectors:
        reason = "this generation has no operation vectors"
    elif embedder is None:
        reason = "no query embedder is configured"
    else:
        from lctx_mcp.embedder import EmbedderError

        try:
            q = np.asarray((await embedder.embed([embedder.spec.query_text(query)]))[0])
            for view in sorted(gen.op_vectors):
                matrix, rows = gen.op_vectors[view]
                best = vector_scores(matrix, rows, q)
                legs.append(ranks({n: s for n, s in best.items() if n in allowed}, False))
        except EmbedderError as e:
            reason = f"the embedding service failed: {e}"
    promoted = {gen.paths[query.strip()]} & allowed if query.strip() in gen.paths else set()
    hits = []
    for node, relevance, source, is_promoted in fuse_legs(legs, promoted, limit):
        o = gen.operations[node]
        hits.append(
            OperationHit(
                operation_id=node.hex(),
                access_path=o["access_path"],
                kind=o["kind"],
                docstring_summary=o["docstring_summary"],
                relevance=relevance,
                rank_source=source,
                promoted=is_promoted,
            )
        )
    return OperationHits(
        snapshot_id=gen.snapshot_id,
        generation=gen.key,
        mode="lexical-only" if reason else "hybrid",
        degraded_reason=reason,
        ranked_discovery=True,
        hits=hits,
        note=NOTE_SEARCH,
    )
