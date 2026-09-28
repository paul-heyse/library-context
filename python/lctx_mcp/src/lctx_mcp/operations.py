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

from lctx_mcp.wire import Contract

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


FacetTerm = Contract("FacetTerm")


Where = Contract("Where")


Discharge = Contract("Discharge")


Fate = Contract("Fate")


ParameterRecord = Contract("ParameterRecord")


SettingRead = Contract("SettingRead")


FieldRecord = Contract("FieldRecord")


FacetValue = Contract("FacetValue")


CatalogMember = Contract("CatalogMember")


CatalogBinding = Contract("CatalogBinding")


CatalogParameter = Contract("CatalogParameter")


CatalogSignature = Contract("CatalogSignature")


CatalogEvidence = Contract("CatalogEvidence")


CatalogType = Contract("CatalogType")


CatalogTypeArgument = Contract("CatalogTypeArgument")


CatalogTypeObservation = Contract("CatalogTypeObservation")


CatalogConstructor = Contract("CatalogConstructor")


CatalogRecord = Contract("CatalogRecord")


AmbiguousOperation = Contract("AmbiguousOperation")


Operation = Contract("Operation")


OperationRef = Contract("OperationRef")


OperationSet = Contract("OperationSet")


OperationHit = Contract("OperationHit")


OperationHits = Contract("OperationHits")
