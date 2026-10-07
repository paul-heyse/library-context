"""Observe actual native final MCP maps independently of their semantic judgment."""

from __future__ import annotations

import copy
import hashlib
import json
import os
import sys
from pathlib import Path

import pytest
from fastmcp import Client
from fastmcp.client.transports import StdioTransport
from lctx_semantics import wire_tool_result

from lctx_mcp.__main__ import create_server
from programmatic_eval import Worker, WorkerError, _capture_facts, capture_public_journey, mutation_outcome

ROOT = Path(__file__).resolve().parents[2]


def declared_source_task(call, predicates):
    # Captured fixture input is read, never imported or executed. Expectations are
    # authored independently of production fields and delivery metadata.
    source = (ROOT / "fixtures/python/synthesis_sources/api.py").read_bytes()
    return {"id": "native-connect-declaration", "split": "development", "family": "captured-connect-default",
            "public_call": call, "request": {"question": "Inspect the captured declaration of api.connect timeout",
            "context": {}, "allowed_followups": []}, "envelope": {"max_calls": 0, "max_bytes": 262144},
            "intent": "positive", "oracle": {"kind": "independent_fixture_source",
            "input_digest": hashlib.sha256(source).hexdigest(), "qualification": "declared syntax only, never runtime or effective defaults",
            "supported_domain": "captured api.connect timeout literal in synthesis_sources/api.py", "revision": "1",
            "completeness": "complete", "unknown_limits": ["no runtime execution or override claims"]},
            "predicates": predicates, "witness": {"kind": "all", "children": [{"kind": "leaf", "predicate": p["name"]} for p in predicates]},
            "model": None}


def predicate(name, role, text, anchors):
    return {"name": name, "role": role, "accepted_text": [text], "anchors": anchors,
            "context": {}, "qualifications": [], "candidate_status": "supported"}


async def check_declared_default_and_exact_source(worker, client, library, observation):
    public = json.loads(observation["segments"][0])["structuredContent"]
    core = public["operation"]["packet"]["core"]
    timeout = [row for row in core["interpretation"]["defaults"] if row["subject_name"] == "timeout"]
    assert timeout, "actual captured declared timeout option required"
    # IDs select actual containers. They supply neither expected literal nor meaning.
    task = declared_source_task({"tool": "get_operation", "arguments": {"library": library,
           "operation": {"kind": "public_path", "path": ["api", "connect"]}, "page": {"expanded": True}}},
           [predicate("name", "operation_name", "connect", [bytes(core["member"]).hex()]),
            predicate("parameter", "parameter_name", "timeout", [bytes(row["option"]).hex() for row in timeout]),
            predicate("default", "declared_literal_default", "2", [bytes(row["option"]).hex() for row in timeout])])
    original = list(worker.judge([{"task": task, "observation": observation, "mode": "immediate"}]))[0]
    assert original["epistemic"] == "sufficient", original

    fixture = (ROOT / "fixtures/python/synthesis_sources/api.py").read_bytes()
    authored_prefix = b"timeout: int = "
    expected_start = fixture.index(authored_prefix) + len(authored_prefix)
    assert fixture[expected_start:expected_start + 1] == b"2"
    captured_sources = [row["original"]["original"] for row in timeout if row.get("original") is not None]
    assert captured_sources, "actual source default occurrence required; native defaults alone do not establish it"
    reference = captured_sources[0]
    assert (reference["start"], reference["end"], reference["encoding"]) == (expected_start, expected_start + 1, "raw_bytes")
    call = {"tool": "get_evidence", "arguments": {"library": library, "source": reference["source"],
            "page": {"expanded": True, "evidence_demand": {"facets": [], "context": {
                "analysis": reference["context"], "release": core["release"]["version"]}, "maximum_followups": 0}}}}
    source_task = declared_source_task(call, [predicate("literal_source", "original_source", "2",
        [json.dumps(reference["source"], sort_keys=True, separators=(",", ":"))])])
    actual = await capture_public_journey(worker, client, source_task, lane="native")
    source_case = {"task": source_task, "observation": actual, "mode": "immediate"}
    source_judgment = list(worker.judge([source_case]))[0]
    assert source_judgment["epistemic"] == "sufficient", source_judgment
    source_public = json.loads(actual["segments"][0])["structuredContent"]
    delivered = source_public["evidence"]
    assert (delivered["original"]["start"], delivered["original"]["end"]) == (expected_start, expected_start + 1)
    assert bytes(delivered["body"]["bytes"]) == fixture[expected_start:expected_start + 1]

    # Diagnostic schema-valid mutant of final captured source bytes. The existing
    # map cannot authorize the changed value, and no mutated bytes count as native success.
    changed = copy.deepcopy(source_public)
    changed["evidence"]["body"]["bytes"] = list(b"3")
    mutant = copy.deepcopy(source_case)
    mutant["observation"]["segments"] = [wire_tool_result("get_evidence", json.dumps(changed, ensure_ascii=False), True)]
    judged = list(worker.judge([mutant]))[0]
    assert judged["scorable"] and judged["epistemic"] == "insufficient", judged
    assert mutation_outcome(source_judgment, judged) == "caught"


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
                    {
                        "library": library,
                        "query": "carefully",
                        "families": [],
                        "page": {"size": 1, "expanded": True},
                    },
                ),
            ]:
                result = await client.call_tool_mcp(tool, arguments)
                assert result.is_error is False
                raw = result.model_dump_json(by_alias=True)
                public = json.loads(raw)["structuredContent"]
                assert public["delivery"]["fields"], "actual finalized native map required"
                facts = _capture_facts(declared_source_task({"tool": tool, "arguments": arguments}, []),
                        raw, [{"tool": tool, "arguments": arguments}], "native", 20000)
                facts["serialization"] = "sdk_result_object"
                observation = {
                    "capture": facts,
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
                if tool == "get_operation":
                    await check_declared_default_and_exact_source(worker, client, library, observation)
