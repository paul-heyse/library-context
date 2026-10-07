"""Observe actual native final MCP maps independently of their semantic judgment."""

from __future__ import annotations

import copy
import json
import os
import sys
from pathlib import Path

import pytest
from fastmcp import Client
from fastmcp.client.transports import StdioTransport
from lctx_mcp.__main__ import create_server
from lctx_semantics import wire_tool_result

from programmatic_eval import Worker, WorkerError

ROOT = Path(__file__).resolve().parents[2]


@pytest.fixture
def anyio_backend():
    return "asyncio"


@pytest.mark.anyio
@pytest.mark.parametrize("transport", ["inprocess", "stdio"])
async def test_actual_native_final_maps_and_schema_valid_false_pointer(transport):
    configured = os.environ.get("LCTX_NATIVE_SERVING_CONFIG")
    library = os.environ.get("LCTX_NATIVE_TEST_LIBRARY")
    assert configured and library, "owned published native fixture required"
    target = (
        create_server(Path(configured))
        if transport == "inprocess"
        else StdioTransport(
            command=sys.executable,
            args=["-m", "lctx_mcp", "--serving-config", configured],
            env=dict(os.environ),
            keep_alive=False,
        )
    )
    binary = Path(os.environ.get("LCTX_EVAL_WORKER", ROOT / "target/release/lctx-eval"))
    with Worker(binary) as worker:
        async with Client(target) as client:
            for tool, arguments in [
                (
                    "get_operation",
                    {
                        "library": library,
                        "operation": {"kind": "public_path", "path": ["api", "connect"]},
                        "page": {"expanded": True},
                    },
                ),
                (
                    "search_evidence",
                    {"library": library, "query": "carefully", "families": [], "page": {"size": 1, "expanded": True}},
                ),
            ]:
                result = await client.call_tool_mcp(tool, arguments)
                assert result.is_error is False
                raw = result.model_dump_json(by_alias=True)
                public = json.loads(raw)["structuredContent"]
                assert public["delivery"]["fields"], "actual finalized native map required"
                observation = {
                    "capture": None,
                    "realization": bytes(public["snapshot"]["realization"]).hex(),
                    "segments": [raw],
                    "observer_format": "mcp_tool_result_v1",
                    "expansions": [],
                    "status": "completed",
                    "failure": None,
                }
                accepted = worker.request({"operation": "observe", "observation": observation})
                assert accepted["semantic_snapshot"] == bytes(public["snapshot"]["semantic"]).hex()

                # The current Rust DTO/formatter accepts this shape. Only the independent
                # final-byte check discovers that the retained map invents a field.
                wrong = copy.deepcopy(public)
                wrong["delivery"]["fields"][0]["field"] = "/structuredContent/absent"
                emitted = wire_tool_result(tool, json.dumps(wrong, ensure_ascii=False), True)
                changed = copy.deepcopy(observation)
                changed["segments"] = [emitted]
                with pytest.raises(WorkerError, match="delivery map field pointer is absent"):
                    worker.request({"operation": "observe", "observation": changed})
