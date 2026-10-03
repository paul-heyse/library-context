"""Canonical Rust-owned schema and wire validation, without an independent semantic executor."""

from ._native import (
    admit_envelope,
    canonical_embedding_spec,
    wire_decode,
    wire_failure,
    wire_resources,
    wire_schema,
    wire_tool,
    wire_tool_result,
    wire_tools,
)

__all__ = [
    "admit_envelope",
    "canonical_embedding_spec",
    "wire_decode",
    "wire_failure",
    "wire_resources",
    "wire_schema",
    "wire_tool",
    "wire_tool_result",
    "wire_tools",
]
