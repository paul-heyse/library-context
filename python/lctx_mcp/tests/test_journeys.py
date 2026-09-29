"""Bounded agent journeys through real PostgreSQL, native contracts and FastMCP."""

import json

import pytest
from fastmcp import Client
from fastmcp.exceptions import ToolError

from lctx_mcp.embedder import FakeEmbedder

pytestmark = pytest.mark.anyio


async def test_browse_vocabulary_scopes_and_comparison(pg_serving):
    fixture = pg_serving
    async with Client(fixture.server(None)) as client:

        async def call(name, **args):
            result = await client.call_tool(name, {"library": fixture.library, **args})
            assert result.structured_content is not None
            return result.structured_content

        first = await call("browse_library", limit=1)
        assert first["members"] == fixture.reference.table("catalog_members").num_rows
        assert first["next_cursor"]
        second = await call("browse_library", limit=1, cursor=first["next_cursor"])
        assert first["items"] != second["items"]
        with pytest.raises(ToolError, match="cursor"):
            await call("browse_library", limit=2, cursor=first["next_cursor"])
        vocabulary = await call("browse_library", view={"kind": "vocabulary"}, expanded=True)
        assert "Requirement" in vocabulary["vocabulary"]["requirement_schema"]["title"]
        module = first["items"][0]["name"]
        scope = {"kind": "module", "name": module}
        scoped = await call("browse_library", scope=scope)
        domains = fixture.reference.table("catalog_selection_domains").to_pylist()
        assert scoped["members"] == sum(
            json.loads(r["detail"])["module"] == module for r in domains
        )
        candidates = ["pkg.Catalog.remove", "pkg.Catalog", "missing.operation"]
        compared = await call(
            "compare_operations",
            candidates=candidates,
            selection={"requirements": [{"predicate": "member_kind", "kind": "method"}]},
            expanded=True,
        )
        assert [row["requested"] for row in compared["candidates"]] == candidates
        assert [row["resolution"] for row in compared["candidates"]] == [
            "resolved",
            "resolved",
            "not_found",
        ]
        assert compared["candidates"][0]["selection"]["outcome"] == "supported"
        assert compared["candidates"][1]["selection"]["outcome"] == "contradicted"
        assert compared["candidates"][0]["signatures"]
        unresolved = await call(
            "compare_operations",
            candidates=["pkg.Catalog.remove"],
            selection={
                "requirements": [
                    {"predicate": "facet_membership", "facet": "raises", "value": "MissingError"}
                ]
            },
        )
        assert unresolved["candidates"][0]["selection"]["outcome"] == "unresolved"


async def test_independent_evidence_vector_and_lexical_routes(pg_serving):
    fixture = pg_serving
    for embedder, route in [(None, "lexical-only"), (FakeEmbedder(), "exact")]:
        async with Client(fixture.server(embedder)) as client:
            request = {
                "library": fixture.library,
                "query": "remove component",
                "families": ["api_options", "source"],
                "limit": 1,
            }
            result = await client.call_tool("search_evidence", request)
            data = result.structured_content
            assert data and data["items"]
            assert data["retrieval"]["actual_route"] == route
            hit = data["items"][0]
            assert hit["winners"] and "text" not in hit["unit"]
            original = await client.call_tool(
                "get_evidence",
                {
                    "snapshot_id": data["snapshot_id"],
                    "evidence": {"kind": "retrieval_unit", "id": hit["unit"]["unit_id"]},
                },
            )
            assert original.structured_content is not None
            if data["next_cursor"]:
                page = await client.call_tool(
                    "search_evidence", {**request, "cursor": data["next_cursor"]}
                )
                assert page.structured_content is not None
                assert (
                    page.structured_content["items"][0]["unit"]["unit_id"] != hit["unit"]["unit_id"]
                )
                with pytest.raises(ToolError, match="cursor"):
                    await client.call_tool(
                        "search_evidence",
                        {**request, "query": "load", "cursor": data["next_cursor"]},
                    )
