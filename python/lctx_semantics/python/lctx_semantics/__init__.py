"""Canonical Rust-owned schema and wire validation, without an independent semantic executor."""
from ._native import (
    canonical_embedding_spec,
    wire_decode,
    wire_schema,
    wire_tool,
    wire_tools,
    wire_resources,
    wire_tool_result,
    admit_envelope,
)
__all__ = ["canonical_embedding_spec", "wire_decode", "wire_schema", "wire_tool", "wire_tools", "wire_resources", "wire_tool_result", "admit_envelope"]
