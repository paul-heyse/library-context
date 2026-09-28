"""Independent malformed projections reach the shared validator after valid receipt framing."""

from __future__ import annotations

import copy
import hashlib
import json
import struct
from pathlib import Path

import pyarrow as pa
import pyarrow.ipc as ipc
import pytest
from lctx_semantics import validate_projection_ipc

from lctx_mcp.digest import schema_digest


def framed(value: bytes) -> bytes:
    return struct.pack("<Q", len(value)) + value


def receipt(name: str, table: pa.Table) -> dict:
    rows = []
    for row in table.to_pylist():
        body = b"lctx-serving-row-v1"
        for field in table.schema:
            value = row[field.name]
            if value is None:
                body += b"\x00"
                continue
            body += b"\x01"
            if pa.types.is_fixed_size_binary(field.type):
                body += framed(value)
            elif pa.types.is_string(field.type):
                body += framed(value.encode())
            elif pa.types.is_int64(field.type):
                body += struct.pack("<q", value)
            elif pa.types.is_boolean(field.type):
                body += bytes([value])
            elif pa.types.is_fixed_size_list(field.type):
                body += struct.pack(f"<{len(value)}f", *value)
            else:
                raise AssertionError(field.type)
        rows.append(hashlib.sha256(body).digest())
    schema = schema_digest(table.schema)
    body = (
        b"lctx-serving-relation-v1"
        + framed(name.encode())
        + framed(schema.encode())
        + struct.pack("<Q", len(rows))
        + b"".join(sorted(rows))
    )
    return dict(
        schema_digest=schema, rows=len(rows), content_digest=hashlib.sha256(body).hexdigest()
    )


def changed(generation: Path, name: str, table: pa.Table):
    manifest = copy.deepcopy(json.loads((generation / "MANIFEST.json").read_text())["projection"])
    files = {p.stem: p.read_bytes() for p in generation.glob("*.arrow")}
    sink = pa.BufferOutputStream()
    with ipc.new_file(sink, table.schema) as writer:
        writer.write_table(table)
    files[name] = sink.getvalue().to_pybytes()
    manifest["relations"][name] = receipt(name, table)
    artifact = manifest["artifacts"].get(f"{name}.arrow")
    if artifact is not None:
        artifact.update(sha256=hashlib.sha256(files[name]).hexdigest(), bytes=len(files[name]))
    return json.dumps(manifest), list(files.items())


def test_independent_receipts_match_every_fixture_relation(generation: Path):
    manifest = json.loads((generation / "MANIFEST.json").read_text())["projection"]
    for name, expected in manifest["relations"].items():
        assert receipt(name, ipc.open_file(generation / f"{name}.arrow").read_all()) == expected


def test_duplicate_entities_and_missing_support_are_rejected(generation: Path):
    ops = ipc.open_file(generation / "operations.arrow").read_all()
    bad = pa.concat_tables([ops, ops.slice(0, 1)])
    with pytest.raises(ValueError, match="duplicate/null domain key"):
        validate_projection_ipc(*changed(generation, "operations", bad))
    supports = ipc.open_file(generation / "supports.arrow").read_all()
    rows = supports.to_pylist()
    assert rows
    rows[0]["evidence_id"] = b"\xff" * 16
    bad = pa.Table.from_pylist(rows, schema=supports.schema)
    with pytest.raises(ValueError, match="missing generation-local evidence"):
        validate_projection_ipc(*changed(generation, "supports", bad))


def test_unknown_code_and_incompatible_definition_are_rejected(generation: Path):
    ops = ipc.open_file(generation / "operations.arrow").read_all()
    rows = ops.to_pylist()
    rows[0]["behavior_status"] = "silently-complete"
    with pytest.raises(ValueError, match="unknown behavior_status code"):
        validate_projection_ipc(
            *changed(generation, "operations", pa.Table.from_pylist(rows, schema=ops.schema))
        )
    manifest, files = changed(generation, "operations", ops)
    manifest = json.loads(manifest)
    manifest["projection_digest"] = "00" * 32
    with pytest.raises(ValueError, match="incompatible projection definition"):
        validate_projection_ipc(json.dumps(manifest), files)


@pytest.mark.parametrize(
    "field",
    [
        "spec_hash",
        "catalog_digest",
        "snapshot_id",
        "snapshot_digest",
        "compiler_digest",
        "entry_value_effect_digest",
    ],
)
def test_projection_identity_is_bound_to_content_and_envelope(generation: Path, field: str):
    envelope = json.loads((generation / "MANIFEST.json").read_text())
    manifest = copy.deepcopy(envelope["projection"])
    manifest[field] = "ff" * (16 if field == "snapshot_id" else 32)
    files = [(p.stem, p.read_bytes()) for p in generation.glob("*.arrow")]
    with pytest.raises(ValueError, match=r"mismatch|incompatible"):
        validate_projection_ipc(json.dumps(manifest), files, json.dumps(envelope))
    if field == "spec_hash":
        # Also reject it without a file envelope, as a future database reader must.
        with pytest.raises(ValueError, match="embedding specification mismatch"):
            validate_projection_ipc(json.dumps(manifest), files)
