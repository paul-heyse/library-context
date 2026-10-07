"""Canonical Rust-owned schema and wire validation, without an independent semantic executor."""

from ._native import (
    NativeFailure,
    NativeSession,
    admit_envelope,
    canonical_embedding_spec,
    embedding_projection,
    embedding_recipe_identities,
    wire_capability_resource,
    wire_decode,
    wire_failure,
    wire_resources,
    wire_schema,
    wire_tool,
    wire_tool_result,
    wire_tools,
)

__all__ = [
    "NativeFailure",
    "NativeSession",
    "admit_envelope",
    "canonical_embedding_spec",
    "embedding_projection",
    "embedding_recipe_identities",
    "wire_capability_resource",
    "wire_decode",
    "wire_failure",
    "wire_resources",
    "wire_schema",
    "wire_tool",
    "wire_tool_result",
    "wire_tools",
]
