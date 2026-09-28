"""PR1 complete catalog packet through real import, PyO3 and FastMCP."""
from pathlib import Path

from fastmcp import Client
import pytest

from lctx_mcp import operations as ops
from support import served_bundle

pytestmark = pytest.mark.anyio


@pytest.fixture(scope="module")
def catalog_serving():
    current = Path("build/pr1-catalog-fixture/CURRENT")
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
        result = await client.call_tool("get_operation", {
            "snapshot_id": snapshot, "operation": "catalogpkg.ordinary", "expanded": True,
        })
        operation = ops.Operation.model_validate(result.structured_content)
        assert operation.catalog is not None and result.content
        formals = operation.catalog.signatures[0].parameters
        assert [p.name for p in formals] == ["first", "optional", "items", "flag", "options"]
        assert [p.kind for p in formals] == ["positional_only", "positional_or_keyword", "var_positional", "keyword_only", "var_keyword"]
        assert formals[1].default_text == '"ok"'
        assert operation.catalog.evidence and operation.catalog.type_observations
        assert operation.catalog.member.brief_status == "not_requested"
        missing = await client.call_tool("search_capabilities", {"library": fixture.library, "query": "ordinary"})
        assert missing.structured_content["reason"] == "not_requested"
        # An alias's old declaration ID must return explicit exposure choices.
        ambiguous = await client.call_tool("get_operation", {"snapshot_id": snapshot, "operation": operation.operation_id})
        assert ambiguous.structured_content["resolution"] == "ambiguous"
        config = await client.call_tool("get_operation", {"snapshot_id": snapshot, "operation": "catalogpkg.Config", "expanded": True})
        config = ops.Operation.model_validate(config.structured_content)
        assert config.catalog.constructors
        assert any(s.role == "provider_constructor" for s in config.catalog.signatures)
