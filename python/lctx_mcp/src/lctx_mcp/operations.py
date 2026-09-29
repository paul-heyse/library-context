"""Rust-owned public catalog, behavioral evidence and contextual selection contracts."""

from __future__ import annotations

from lctx_mcp.wire import Contract


class OperationError(ValueError):
    """An invalid request (an unknown operation, snapshot, facet value or cursor)."""


FacetTerm = Contract("FacetTerm")


Selection = Contract("Selection")
SelectionResults = Contract("SelectionResults")


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
