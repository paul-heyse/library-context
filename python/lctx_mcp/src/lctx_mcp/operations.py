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

from typing import Annotated, Literal

from pydantic import BaseModel, Field

from lctx_mcp.retrieval import RetrievalMetadata

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
    "reads_setting",
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


class Discharge(BaseModel):
    """One member origin of a call-transfer claim (ADR-0064): the finite summary that proves
    it, or why it stays open. A claim is established or conditional only when every member is
    proved; an open member is never a negative conclusion."""

    origin_id: str
    proof_kind: str
    decision: Literal["proved", "open"]
    summary_id: str | None
    reason: str | None


class Fate(BaseModel):
    """What one behavior row states, with its verdict and where it is shown."""

    kind: str
    transfer: Literal["identity", "derived", "call"] | None
    condition_scope_id: str
    parameter: str | None
    callee: str | None
    target: str | None
    value: str | None
    depth: int
    conditional: bool
    verdict: str
    # Why the verdict is `unknown`: `override_dispatch`, `ambiguous_binding`,
    # `outside_provider_model`, `dynamic_access`; none when established or conditional.
    boundary_reason: str | None
    # The condition in condition_scope_id's places (none: always).
    condition: str | None
    # A callee outside the release, as written.
    callee_text: str | None
    # When a setting is read: `import`, `construction`, `snapshot` or `per_call`.
    phase: str | None
    # The premise a negative claim rests on (`place_claims` / `negative_premises`).
    premise_key: str | None
    occurrences: int
    path: str | None
    line: int | None
    site_text: str | None
    # A call-transfer claim's discharge evidence, one entry per member origin (ADR-0064).
    discharges: list[Discharge] = []


class ParameterRecord(BaseModel):
    """A parameter's fates. With none, it is "never read" only if an `is_read` fate says
    `refuted_under_model`; otherwise its other channels are not analyzed."""

    name: str
    fates: list[Fate]
    note: str | None


class SettingRead(BaseModel):
    """One read of a singleton's field, at its resolved key."""

    reader: str | None
    phase: str
    path: str | None
    line: int
    spelled: str
    condition: str | None


class FieldRecord(BaseModel):
    """A singleton's field: where it is read, and whether "never read" is refuted."""

    name: str
    reads: list[SettingRead]
    # The claim "never read" (none: the field is read, or no claim is made about it).
    never_read: str | None


class FacetValue(BaseModel):
    """A materialized facet value and its own verdict, independent of facet completeness."""

    value: str
    verdict: str


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
    facets: dict[str, list[FacetValue]]
    # The facets whose rows are not complete for this operation, with why.
    incomplete_facets: dict[str, str]
    parameters: list[ParameterRecord]
    delegates: list[Fate]
    handoffs: list[Fate]
    # Settings the operation reads in its own body (Stage 2), each with its phase and condition.
    reads: list[Fate] = Field(default_factory=list)
    # A class's controls are its constructor's: the `__init__` record, when one is public.
    constructor: Operation | None = None
    # A module-global singleton this class backs (`fastmcp.settings`), and its fields.
    singleton_of: str | None = None
    fields: list[FieldRecord] = Field(default_factory=list)


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
    retrieval: RetrievalMetadata

    """`search_operations`' answer."""

    snapshot_id: str
    generation: str
    mode: Literal["hybrid", "lexical-only"]
    degraded_reason: str | None
    ranked_discovery: bool
    hits: list[OperationHit]
    note: str
