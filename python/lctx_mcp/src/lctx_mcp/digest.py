"""The serving schema digest (DESIGN §6.4; ADR-0019).

A SHA-256 over a language-neutral canonical form of a schema, recomputed here from pyarrow with
the standard library. `cpg_schema::bundle` computes the same bytes in Rust, and the known answers
in `specs/serving/schema_digests.json` hold both sides to them.
"""

from __future__ import annotations

import hashlib
import json

import pyarrow as pa


class GrammarError(ValueError):
    """A type outside the declared serving grammar."""


def type_grammar(t: pa.DataType) -> str:
    """A data type in the declared grammar (see `cpg_schema::bundle::type_grammar`)."""
    if pa.types.is_boolean(t):
        return "bool"
    for name, check in (
        ("int16", pa.types.is_int16),
        ("int32", pa.types.is_int32),
        ("int64", pa.types.is_int64),
        ("float32", pa.types.is_float32),
        ("float64", pa.types.is_float64),
    ):
        if check(t):
            return name
    if pa.types.is_string(t):
        return "utf8"
    if pa.types.is_fixed_size_binary(t):
        return f"fixed_size_binary({t.byte_width})"
    if pa.types.is_fixed_size_list(t):
        return f"fixed_size_list({_child(t.value_field)}, {t.list_size})"
    if pa.types.is_list(t):
        return f"list({_child(t.value_field)})"
    raise GrammarError(f"{t} is not in the serving type grammar")


def _child(field: pa.Field) -> str:
    nullability = "null" if field.nullable else "not null"
    return f'{type_grammar(field.type)} {nullability} "{field.name}"'


def canonical_form(schema: pa.Schema) -> str:
    """One object per field, keys sorted, no whitespace, non-ASCII kept."""
    fields = [
        {
            "metadata": sorted([k.decode(), v.decode()] for k, v in (f.metadata or {}).items()),
            "name": f.name,
            "nullable": f.nullable,
            "type": type_grammar(f.type),
        }
        for f in schema
    ]
    return json.dumps(fields, sort_keys=True, separators=(",", ":"), ensure_ascii=False)


def schema_digest(schema: pa.Schema) -> str:
    """SHA-256 of the canonical form, lowercase hex."""
    return hashlib.sha256(canonical_form(schema).encode()).hexdigest()
