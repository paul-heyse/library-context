"""PR1 complete catalog packet through real import, PyO3 and FastMCP."""

from pathlib import Path

import pytest
from fastmcp import Client
from support import served_bundle

from lctx_mcp import operations as ops

pytestmark = pytest.mark.anyio


@pytest.fixture(scope="module")
def catalog_serving():
    current = Path("build/catalog-fixture/CURRENT")
    if not current.exists():
        pytest.skip("targeted catalog fixture not built")
    with served_bundle(Path(current.read_text().strip())) as fixture:
        yield fixture


async def test_catalog_contract_without_native_or_briefs(catalog_serving):
    fixture = catalog_serving
    state = fixture.load()
    assert state.condition_graph is None
    async with Client(fixture.server(None)) as client:
        snapshot = state.snapshot_id
        result = await client.call_tool(
            "get_operation",
            {
                "snapshot_id": snapshot,
                "operation": "catalogpkg.ordinary",
                "expanded": True,
            },
        )
        operation = ops.Operation.model_validate(result.structured_content)
        assert operation.catalog is not None and result.content
        formals = operation.catalog.signatures[0].parameters
        assert [p.name for p in formals] == ["first", "optional", "items", "flag", "options"]
        assert [p.kind for p in formals] == [
            "positional_only",
            "positional_or_keyword",
            "var_positional",
            "keyword_only",
            "var_keyword",
        ]
        assert formals[1].default_text == '"ok"'
        assert operation.catalog.evidence_page["items"] and operation.catalog.type_observations
        assert operation.catalog.member.brief_status == "not_requested"
        missing = await client.call_tool(
            "search_capabilities", {"library": fixture.library, "query": "ordinary"}
        )
        assert missing.structured_content is not None
        assert missing.structured_content["reason"] == "not_requested"
        # The same declaration exposed through a base and child must return choices by ID.
        method = await client.call_tool(
            "get_operation", {"snapshot_id": snapshot, "operation": "catalogpkg.Child.method"}
        )
        assert method.structured_content is not None
        ambiguous = await client.call_tool(
            "get_operation",
            {"snapshot_id": snapshot, "operation": method.structured_content["operation_id"]},
        )
        assert ambiguous.structured_content is not None
        assert ambiguous.structured_content["resolution"] == "ambiguous"
        assert {m["access_path"] for m in ambiguous.structured_content["choices"]} >= {
            "catalogpkg.Base.method",
            "catalogpkg.Child.method",
        }
        alias = await client.call_tool(
            "get_operation", {"snapshot_id": snapshot, "operation": "catalogpkg.alias"}
        )
        assert alias.structured_content is not None
        assert alias.structured_content["resolution"] == "unresolved"
        config = await client.call_tool(
            "get_operation",
            {"snapshot_id": snapshot, "operation": "catalogpkg.Config", "expanded": True},
        )
        config = ops.Operation.model_validate(config.structured_content)
        assert config.catalog is not None
        assert config.catalog.constructors
        assert any(s.role == "provider_constructor" for s in config.catalog.signatures)
        inherited = await client.call_tool(
            "get_operation",
            {"snapshot_id": snapshot, "operation": "catalogpkg.PublicConfig", "expanded": True},
        )
        inherited = ops.Operation.model_validate(inherited.structured_content)
        assert inherited.catalog is not None
        assert any(not c.own and c.ancestry_fact_id for c in inherited.catalog.constructors)
        originals = []
        for item in inherited.catalog.evidence_page["items"]:
            opened = await client.call_tool(
                "get_evidence",
                {
                    "snapshot_id": snapshot,
                    "evidence": item.evidence.model_dump(mode="json"),
                    "expanded": True,
                },
            )
            assert opened.structured_content is not None
            originals.extend(c.get("text") or "" for c in opened.structured_content["content"])
        assert any("class _PrivateConfig" in text for text in originals)
