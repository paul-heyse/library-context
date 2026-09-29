"""Bounded real stdio MCP journeys, without ranking accuracy or comparative scoring."""
import asyncio
import json
import os
from pathlib import Path
import sys

from fastmcp import Client
from fastmcp.client.transports import StdioTransport
from lctx_semantics import wire_tool
from mcp_types import CallToolResult
import pyarrow.ipc as ipc


def rows(generation, name):
    return ipc.open_file(generation / f"{name}.arrow").read_all().to_pylist()


async def check(generation, config, embedder="none"):
    generation, config = Path(generation), Path(config)
    manifest = json.loads((generation / "MANIFEST.json").read_text())
    transport = StdioTransport(sys.executable, ["-m", "lctx_mcp", "--config", str(config),
        "--library", manifest["library"], "--generation", manifest["projection_generation"],
        "--embedder", embedder], env={**os.environ,"FASTMCP_CHECK_FOR_UPDATES":"off"}, keep_alive=False)
    calls = []
    async with Client(transport, timeout=120) as client:
        for tool in await client.list_tools():
            schema = json.loads(wire_tool(tool.name))
            assert tool.input_schema == schema["parameters"] and tool.output_schema == schema["output_schema"]

        async def call(name, **args):
            result = await client.call_tool(name, args)
            assert result.structured_content is not None
            data = result.structured_content
            actual = CallToolResult(content=result.content, structured_content=data, is_error=False)
            size = len(actual.model_dump_json(by_alias=True, exclude_none=True).encode())
            budget = json.loads(wire_tool(name))["byte_limits"]["expanded" if args.get("expanded", False) else "default"]
            assert size <= budget
            calls.append({"tool":name,"bytes":size})
            return data

        library = manifest["library"]
        outline = await call("browse_library", library=library, limit=5)
        assert outline["members"] == len(rows(generation,"catalog_members"))
        snapshot = outline["snapshot_id"]
        vocabulary = await call("browse_library", library=library, view={"kind":"vocabulary"}, expanded=True)
        assert vocabulary["vocabulary"]["requirement_schema"]["title"] == "Requirement"
        missing = await call("browse_library", library=library, scope={"kind":"module","name":"missing.scope"})
        assert missing["state"] == "unavailable"
        selected = await call("find_operations", library=library, selection={"requirements":[{"predicate":"declares_parameter","name":"transport"}]}, limit=2)
        assert selected["supported"]["total"] > 0
        evidence = await call("search_evidence", library=library, query="deployment server HTTP authentication", limit=3)
        assert evidence["items"]
        assert evidence["retrieval"]["actual_route"] == ("exact" if embedder=="vllm" else "lexical-only")
        winner = evidence["items"][0]
        original = await call("get_evidence", snapshot_id=snapshot, evidence={"kind":"retrieval_unit","id":winner["unit"]["unit_id"]}, expanded=True)
        assert original["unit"]["unit_id"] == winner["unit"]["unit_id"]
        # Release-only deployment discovery requires no API member.
        units = [json.loads(r["detail"]) for r in rows(generation,"retrieval_units")]
        unit = next(u for u in units if u["family"]=="documentation_deployment" and u["subjects"] and all(s["kind"]=="release" for s in u["subjects"]))
        release_evidence = await call("search_evidence", library=library, query="python dependencies environment", families=["documentation_deployment"], subject=unit["subjects"][0], limit=3)
        assert release_evidence["items"]
        compared = await call("compare_operations", library=library, candidates=["fastmcp.FastMCP","fastmcp.cli.cli.run","missing.operation"], selection={"requirements":[{"predicate":"member_kind","kind":"class"}]}, expanded=True)
        assert [r["resolution"] for r in compared["candidates"]] == ["resolved","resolved","not_found"]
        assert compared["candidates"][0]["selection"]["outcome"] == "supported"
        assert compared["candidates"][1]["selection"]["outcome"] == "contradicted"
        for name in ["fastmcp.cli.cli.run","fastmcp.server.auth.JWTVerifier"]:
            core = await call("get_operation", snapshot_id=snapshot, operation=name, expanded=True)
            assert core["catalog"]["signatures"] and len(core["demonstrations"])<=2 and len(core["relationships"])<=5
            assert "docstring" not in core["catalog"]["signatures"][0] and "constructor" not in core
            cursor = None
            for _ in range(50):
                page = await call("get_operation", snapshot_id=snapshot, operation=name, view={"kind":"section","section":"behavior","cursor":cursor}, expanded=True)
                assert "catalog" not in page
                cursor = page["next_cursor"]
                if not manifest["capabilities"]["behavioral_claims"]:
                    assert page["state"] == "not_requested"
                if cursor is None:
                    break
            assert cursor is None, "bounded behavior continuation"
        if not manifest["capabilities"]["briefs"]:
            unavailable = await call("search_capabilities", library=library, query="server")
            assert unavailable["reason"] == "not_requested"
        deployments = rows(generation,"catalog_deployments")
        controls={"declared":"requires-dist","installed":"version","observed":"task_observation"}
        for label, field in controls.items():
            row = next(r for r in deployments if json.loads(r["detail"])["field"]==field)
            opened = await call("get_evidence", snapshot_id=snapshot, evidence={"kind":"deployment","id":row["deployment_id"].hex()}, expanded=True)
            assert opened["deployment"]["field"]==field
            if label=="observed":
                assert opened["deployment"]["task"]["receipt"]["execution"]=="passed"
            if label=="installed":
                assert opened["deployment"]["environment_digest"] is not None
    return {"outcome":"passed","generation":manifest["projection_generation"],"embedder":embedder,
            "calls":calls,"deployment_controls":controls,"accuracy_assessment":"not_run"}


if __name__ == "__main__":
    print(json.dumps(asyncio.run(check(Path(sys.argv[1]),Path(sys.argv[2]),sys.argv[3] if len(sys.argv)>3 else "none")),indent=2))
