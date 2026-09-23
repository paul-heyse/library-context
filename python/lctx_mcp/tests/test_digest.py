"""The serving schema digests: Python's recomputation equals Rust's known answers (ADR-0019)."""

from __future__ import annotations

import json
from pathlib import Path

import pyarrow as pa
import pyarrow.ipc as ipc
import pytest

from lctx_mcp.digest import GrammarError, canonical_form, schema_digest, type_grammar
from lctx_mcp.generation import expected_schemas

REPO = Path(__file__).resolve().parents[3]

ANSWERS = json.loads((REPO / "specs/serving/schema_digests.json").read_text(encoding="utf-8"))


def test_expected_schemas_are_rusts_known_answers() -> None:
    schemas = expected_schemas(4096)
    assert list(schemas) == list(ANSWERS)
    for name, schema in schemas.items():
        assert canonical_form(schema) == ANSWERS[name]["form"], name
        assert schema_digest(schema) == ANSWERS[name]["digest"], name


def test_a_generations_files_have_the_digests_their_manifest_records(generation: Path) -> None:
    manifest = json.loads((generation / "MANIFEST.json").read_text(encoding="utf-8"))
    for entry in manifest["files"].values():
        table = ipc.open_file(pa.memory_map(str(generation / entry["file"]))).read_all()
        assert schema_digest(table.schema) == entry["schema_digest"], entry["file"]


def test_the_grammar_refuses_what_it_does_not_declare() -> None:
    assert type_grammar(pa.list_(pa.field("item", pa.float32(), nullable=False), 4)) == (
        'fixed_size_list(float32 not null "item", 4)'
    )
    with pytest.raises(GrammarError):
        type_grammar(pa.large_string())
