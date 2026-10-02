"""Actual current serving fixture; invoked explicitly while the Rust disposable DB is live.

The enclosing serving_native control supplies real config/generation identities. This file is
not auto-collected again without that owner. No legacy bundle or mocked storage path is used.
"""

from __future__ import annotations

import asyncio
import json
import os
import sys
import threading
from collections.abc import Sequence
from pathlib import Path

import httpx2
import pytest
from fastmcp import Client
from fastmcp.exceptions import ToolError
from lctx_semantics import wire_resources, wire_tool, wire_tools
from lctx_storage import StorageError
from mcp.shared.exceptions import MCPError
from mcp_types import (
    CLIENT_CAPABILITIES_META_KEY,
    PROTOCOL_VERSION_META_KEY,
    SERVER_INFO_META_KEY,
    CallToolResult,
    Implementation,
    TextResourceContents,
)

from lctx_mcp.generation import open_generation
from lctx_mcp.server import build_server
from lctx_mcp.wire import response_encodings

pytestmark = pytest.mark.anyio


@pytest.fixture
def current_fixture():
    required = [
        "LCTX_SERVING_TEST_CONFIG",
        "LCTX_SERVING_TEST_GENERATION",
        "LCTX_SERVING_TEST_LIBRARY",
        "LCTX_SERVING_TEST_PROFILE",
    ]
    missing = [key for key in required if key not in os.environ]
    if missing:
        pytest.fail("live current Rust fixture required: " + ", ".join(missing))
    return {
        "config": Path(os.environ[required[0]]),
        "generation": os.environ[required[1]],
        "library": os.environ[required[2]],
        "profile": os.environ[required[3]],
        "artifact": json.loads(os.environ["LCTX_SERVING_TEST_ARTIFACT"])
        if "LCTX_SERVING_TEST_ARTIFACT" in os.environ
        else None,
        "capability": os.environ.get("LCTX_SERVING_TEST_CAPABILITY"),
        "native": json.loads(os.environ["LCTX_SERVING_TEST_NATIVE"])
        if "LCTX_SERVING_TEST_NATIVE" in os.environ
        else None,
    }


def server(fixture):
    return build_server(fixture["config"], generation=fixture["generation"])


def generation_bytes(fixture):
    return list(bytes.fromhex(fixture["generation"]))


async def test_real_list_call_original_evidence_and_resource_template(current_fixture):
    fixture = current_fixture
    async with Client(server(fixture)) as client:
        assert {tool.name for tool in await client.list_tools()} == {
            row["name"] for row in json.loads(wire_tools())
        }
        templates = await client.list_resource_templates()
        assert [
            {"uri_template": t.uri_template, "name": t.name, "mime_type": t.mime_type}
            for t in templates
        ] == json.loads(wire_resources())
        found = await client.call_tool("find_operations", {"library": fixture["library"]})
        dto = found.structured_content
        assert dto is not None
        assert dto["generation"] == generation_bytes(fixture)
        candidates = dto["supported"]["items"] + dto["unresolved"]["items"]
        api = next(item for item in candidates if item["name"].endswith("api"))
        result = await client.call_tool(
            "get_operation",
            {
                "library": fixture["library"],
                "operation": {"kind": "member", "member": api["member"]},
            },
        )
        packet = result.structured_content
        assert packet is not None
        assert packet["generation"] == generation_bytes(fixture)
        assert packet["operation"]["resolution"] == "unique"
        assert packet["operation"]["packet"]["core"]["member"] == api["member"]
        assert packet["operation"]["packet"]["core"]["signatures"]
        for extra in [{"ignored": True}, {"page": {"expanded": 1}}, {"page": {"cursor": None}}]:
            with pytest.raises(ToolError):
                await client.call_tool("find_operations", {"library": fixture["library"], **extra})
        ranked = await client.call_tool(
            "search_operations", {"library": fixture["library"], "query": "api"}
        )
        assert ranked.structured_content is not None
        assert ranked.structured_content["generation"] == generation_bytes(fixture)
        assert ranked.structured_content["channels"]["vector"]["status"] == "disabled"
        if fixture["artifact"] is not None:
            evidence = await client.call_tool(
                "get_evidence",
                {
                    "source": {"kind": "artifact", "artifact": fixture["artifact"]},
                    "page": {"expanded": True},
                },
            )
            assert evidence.structured_content is not None
            body = bytes(evidence.structured_content["evidence"]["body"]["bytes"])
            assert b"def api(value):" in body
            assert (
                evidence.structured_content["evidence"]["original"]["artifact"]
                == fixture["artifact"]
            )
        with pytest.raises(MCPError) as invalid_resource:
            await client.read_resource("lctx://capability/not-a-canonical-id")
        assert invalid_resource.value.code == -32602
        if fixture["capability"] is not None:
            identity = list(bytes.fromhex(fixture["capability"]))
            capability = await client.call_tool(
                "get_capability", {"capability": identity, "page": {"expanded": True}}
            )
            contents = await client.read_resource("lctx://capability/" + fixture["capability"])
            assert len(contents) == 1
            assert isinstance(contents[0], TextResourceContents)
            assert contents[0].mime_type == "text/markdown"
            assert capability.structured_content is not None
            packet = capability.structured_content["capability"]
            assert contents[0].text.startswith(packet["rendered"])
            assert "## Assertion evidence" in contents[0].text
            assert packet["assertions"]
            for claim in packet["assertions"]:
                assert json.dumps(claim["assertion"], separators=(",", ":")) in contents[0].text
                assert '"status":' + str(claim["status"]) in contents[0].text
                assert (
                    json.dumps(claim["text"], ensure_ascii=False, separators=(",", ":"))
                    in contents[0].text
                )
                assert claim["supports"]


async def test_modern_http_emits_exact_admitted_serializer(current_fixture):
    configured = server(current_fixture)
    app = configured.http_app(path="/mcp", json_response=True)
    async with (
        app.lifespan(app),
        httpx2.AsyncClient(
            transport=httpx2.ASGITransport(app=app), base_url="http://localhost"
        ) as client,
    ):
        request_id = "current-λ"
        payload = {
            "jsonrpc": "2.0",
            "id": request_id,
            "method": "tools/call",
            "params": {
                "name": "find_operations",
                "arguments": {"library": current_fixture["library"]},
                "_meta": {
                    PROTOCOL_VERSION_META_KEY: "2026-07-28",
                    CLIENT_CAPABILITIES_META_KEY: {},
                },
            },
        }
        response = await client.post(
            "/mcp",
            json=payload,
            headers={
                "accept": "application/json, text/event-stream",
                "mcp-protocol-version": "2026-07-28",
                "mcp-method": "tools/call",
                "mcp-name": "find_operations",
            },
        )
        assert response.status_code == 200, response.text
        envelope = response.json()
        assert envelope["id"] == request_id
        assert envelope["result"]["structuredContent"]["generation"] == generation_bytes(
            current_fixture
        )
        sdk_version = configured.version
        assert sdk_version is not None
        identity = Implementation(
            name=configured.name,
            version=sdk_version,
            website_url=configured.website_url,
            icons=configured.icons or None,
        ).model_dump(by_alias=True, mode="json", exclude_none=True)
        assert envelope["result"]["_meta"] == {SERVER_INFO_META_KEY: identity}
        result = CallToolResult.model_validate({**envelope["result"], "_meta": None})
        _, expected_http = response_encodings(
            result, request_id, protocol_version="2026-07-28", server_info=identity
        )
        assert response.content == expected_http


async def stdio_exchange(fixture, *, noisy=False):
    args = [
        "-m",
        "lctx_mcp",
        "--config",
        str(fixture["config"]),
        "--generation",
        fixture["generation"],
    ]
    if noisy:
        script = (
            "import runpy,sys;print('noise',flush=True);sys.argv=['lctx_mcp',*"
            + repr(args[2:])
            + "];runpy.run_module('lctx_mcp',run_name='__main__')"
        )
        args = ["-c", script]
    process = await asyncio.create_subprocess_exec(
        sys.executable,
        *args,
        stdin=asyncio.subprocess.PIPE,
        stdout=asyncio.subprocess.PIPE,
        stderr=asyncio.subprocess.DEVNULL,
        limit=1024 * 1024,
        env={**os.environ, "FASTMCP_CHECK_FOR_UPDATES": "off"},
    )
    assert process.stdin is not None and process.stdout is not None
    lines = []
    try:
        messages = [
            {
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {
                    "protocolVersion": "2025-11-25",
                    "capabilities": {},
                    "clientInfo": {"name": "raw", "version": "0"},
                },
            },
            {"jsonrpc": "2.0", "method": "notifications/initialized"},
            {"jsonrpc": "2.0", "id": 2, "method": "tools/list"},
            {
                "jsonrpc": "2.0",
                "id": 3,
                "method": "tools/call",
                "params": {"name": "find_operations", "arguments": {"library": fixture["library"]}},
            },
        ]
        for message in messages:
            process.stdin.write((json.dumps(message) + "\n").encode())
        await process.stdin.drain()
        while True:
            raw = await asyncio.wait_for(process.stdout.readline(), 60)
            assert raw, "current stdio service exited before answering"
            lines.append(raw)
            try:
                if json.loads(raw).get("id") == 3:
                    break
            except ValueError:
                pass
        return lines
    finally:
        process.stdin.close()
        try:
            await asyncio.wait_for(process.wait(), 10)
        except TimeoutError:
            process.kill()
            await process.wait()


async def test_raw_stdio_has_only_protocol_and_exact_response_bytes(current_fixture):
    lines = await stdio_exchange(current_fixture)
    messages = [json.loads(line) for line in lines]
    assert all(message["jsonrpc"] == "2.0" for message in messages)
    answer = next(message for message in messages if message.get("id") == 3)
    assert answer["result"]["isError"] is False
    assert answer["result"]["structuredContent"]["generation"] == generation_bytes(current_fixture)
    expected_stdio, _ = response_encodings(
        CallToolResult.model_validate(answer["result"]), 3, protocol_version="2025-11-25"
    )
    assert lines[-1] == expected_stdio


async def test_raw_stdio_noise_control_detects_a_nonprotocol_line(current_fixture):
    lines = await stdio_exchange(current_fixture, noisy=True)
    assert lines[0] == b"noise\n"
    assert all(json.loads(line)["jsonrpc"] == "2.0" for line in lines[1:])


async def wait_events(events):
    async def wait():
        while not all(event.is_set() for event in events):
            await asyncio.sleep(0.005)

    await asyncio.wait_for(wait(), 10)


@pytest.mark.parametrize("codec", [False, True], ids=["numerical", "envelope"])
async def test_cancelled_python_numerical_worker_keeps_actual_native_grant(current_fixture, codec):
    served = await open_generation(
        current_fixture["config"], None, generation=current_fixture["generation"]
    )
    release = threading.Event()
    entered = [threading.Event(), threading.Event()]
    finished = [threading.Event(), threading.Event()]
    grants = []
    pending = []
    try:
        for index in range(2):
            grant = await served.service.admit()
            grants.append(grant)

            def callback(*args, index=index):
                entered[index].set()
                try:
                    if not release.wait(15):
                        raise RuntimeError("numerical cancellation control timed out")
                    return (b"{}", b"{}") if codec else served.numerical(args[0])
                finally:
                    finished[index].set()

            if codec:
                work = served.service.encode_envelope(grant, callback, False)
            else:
                work = served.service.dispatch(
                    grant,
                    "search_operations",
                    json.dumps({"library": current_fixture["library"], "query": "api"}),
                    numerical_callback=callback,
                )
            pending.append(asyncio.ensure_future(work))
        await wait_events(entered)
        pending[0].cancel()
        with pytest.raises(asyncio.CancelledError):
            await pending[0]
        grants[0].release()
        assert not finished[0].is_set()
        with pytest.raises(StorageError) as refusal:
            await served.service.admit()
        assert refusal.value.kind == "resource_refused"
        assert not any(event.is_set() for event in finished)
        release.set()
        response = await pending[1]
        if codec:
            assert response == (b"{}", b"{}")
        else:
            assert json.loads(response)["generation"] == generation_bytes(current_fixture)
        grants[1].release()
        await wait_events(finished)
        recovered = await served.service.admit()
        try:
            assert (
                json.loads(
                    await served.service.request_info(
                        recovered,
                        "find_operations",
                        json.dumps({"library": current_fixture["library"]}),
                    )
                )["query"]
                is None
            )
        finally:
            recovered.release()
    finally:
        release.set()
        for task in pending:
            if not task.done():
                task.cancel()
        await asyncio.gather(*pending, return_exceptions=True)
        for grant in grants:
            grant.release()
        await served.service.shutdown()


async def test_callback_failure_and_invalid_numerical_returns_release_capacity(current_fixture):
    served = await open_generation(
        current_fixture["config"], None, generation=current_fixture["generation"]
    )
    request = json.dumps({"library": current_fixture["library"], "query": "api"})

    def failed(raw):
        raise RuntimeError("intentional numerical callback failure")

    identities = {
        tuple(identity)
        for _, document_ids in served.numerical._families.values()
        for identity in document_ids
    }
    assert identities, "actual current corpus must contain numerical documents"
    foreign = next([byte] * 32 for byte in range(256) if tuple([byte] * 32) not in identities)
    valid = list(next(iter(identities)))
    callbacks = [
        failed,
        lambda raw: '[{"document":' + json.dumps(foreign) + ',"score":1.0}]',
        lambda raw: '[{"document":' + json.dumps(valid) + ',"score":NaN}]',
    ]
    try:
        for callback in callbacks:
            grant = await served.service.admit()
            try:
                with pytest.raises((RuntimeError, ValueError, StorageError)):
                    await served.service.dispatch(
                        grant, "search_operations", request, numerical_callback=callback
                    )
            finally:
                grant.release()
        healthy = await served.service.admit()
        try:
            response = json.loads(
                await served.service.dispatch(
                    healthy, "search_operations", request, numerical_callback=served.numerical
                )
            )
            assert response["generation"] == generation_bytes(current_fixture)
        finally:
            healthy.release()
    finally:
        await served.service.shutdown()


async def test_original_grant_final_envelope_preflight_and_both_writer_bounds(current_fixture):
    served = await open_generation(
        current_fixture["config"], None, generation=current_fixture["generation"]
    )
    grant = None
    try:
        grant = await served.service.admit()
        bounds = json.loads(wire_tool("find_operations"))["byte_limits"]
        # Non-JSON bytes one byte above the bound must fail byte admission before parsing.
        for expanded, bound in [(False, bounds["default"]), (True, bounds["expanded"])]:
            with pytest.raises(ValueError, match="resource_refused: final MCP envelope bytes"):
                await served.service.admit_envelope(grant, b"\x00" * (bound + 1), expanded)
        # This is a protocol codec control, not a fabricated canonical domain response.
        result = CallToolResult.model_validate(
            {
                "content": [{"type": "text", "text": "🦀" * (bounds["default"] // 8)}],
                "isError": False,
            }
        )
        stdio, http = response_encodings(result, 0)
        assert len(stdio) <= bounds["default"] < len(http) <= bounds["expanded"]
        await served.service.admit_envelope(grant, stdio, False)
        with pytest.raises(ValueError, match="resource_refused: final MCP envelope bytes"):
            await served.service.admit_envelope(grant, http, False)
        await served.service.admit_envelope(grant, http, True)
        response = json.loads(
            await served.service.dispatch(
                grant, "find_operations", json.dumps({"library": current_fixture["library"]})
            )
        )
        assert response["generation"] == generation_bytes(current_fixture)
    finally:
        if grant is not None:
            grant.release()
        await served.service.shutdown()


async def test_native_argument_admission_precedes_decode_vector_copy_and_callback(current_fixture):
    served = await open_generation(
        current_fixture["config"], None, generation=current_fixture["generation"]
    )
    grant = None
    accesses = []

    class OversizedVector(Sequence):
        def __len__(self):
            return 1025

        def __getitem__(self, index):
            accesses.append(index)
            raise AssertionError("oversized query vector component was accessed")

    length_calls = []

    class MisreportedString(str):
        def __len__(self):
            length_calls.append(True)
            return 0

    callbacks = []

    def numerical(raw):
        callbacks.append(raw)
        return served.numerical(raw)

    try:
        grant = await served.service.admit()
        limit = json.loads(wire_tool("find_operations"))["byte_limits"]["expanded"]
        oversized = "\x00" * (limit + 1)
        encoded_calls = []

        def encode(arguments):
            encoded_calls.append(True)
            return json.dumps(arguments, ensure_ascii=False, separators=(",", ":"), allow_nan=False)

        cycle = {}
        cycle["self"] = cycle
        for arguments, kind in [
            ({"value": oversized}, "resource_refused"),
            ({"value": 1 << (limit * 4)}, "resource_refused"),
            ({"value": float("nan")}, "corrupt"),
            (cycle, "corrupt"),
            ({"value": MisreportedString("valid")}, "corrupt"),
        ]:
            with pytest.raises(StorageError) as refusal:
                await served.service.encode_request(grant, arguments, encode)
            assert refusal.value.kind == kind
        assert encoded_calls == []
        unicode_arguments = {"value": "é"}
        assert (
            json.loads(await served.service.encode_request(grant, unicode_arguments, encode))
            == unicode_arguments
        )
        with pytest.raises(StorageError) as refusal:
            await served.service.encode_request(grant, {"value": "é" * (limit // 2)}, encode)
        assert refusal.value.kind == "resource_refused"
        assert len(encoded_calls) == 2
        for invoke in [served.service.request_info, served.service.dispatch]:
            with pytest.raises(StorageError) as refusal:
                await invoke(grant, "find_operations", oversized)
            assert refusal.value.kind == "resource_refused"
            with pytest.raises(ValueError, match="exact builtin strings required"):
                await invoke(grant, "find_operations", MisreportedString(oversized))
            with pytest.raises(ValueError, match="exact builtin strings required"):
                await invoke(
                    grant,
                    MisreportedString("find_operations"),
                    json.dumps({"library": current_fixture["library"]}),
                )
        with pytest.raises(ValueError, match="invalid canonical capability id"):
            await served.service.capability_resource(grant, oversized)
        with pytest.raises(ValueError, match="exact builtin strings required"):
            await served.service.capability_resource(
                grant, MisreportedString(current_fixture["capability"])
            )
        assert length_calls == []
        request = json.dumps({"library": current_fixture["library"], "query": "api"})
        for vector in [OversizedVector(), [True] + [0.0] * 1023, [float("nan")] + [0.0] * 1023]:
            with pytest.raises(StorageError) as refusal:
                await served.service.dispatch(
                    grant,
                    "search_operations",
                    request,
                    query_vector=vector,
                    numerical_callback=numerical,
                )
            assert refusal.value.kind == "corrupt"
        assert accesses == []
        assert callbacks == []
        result = json.loads(
            await served.service.dispatch(
                grant, "find_operations", json.dumps({"library": current_fixture["library"]})
            )
        )
        assert result["generation"] == generation_bytes(current_fixture)
    finally:
        if grant is not None:
            grant.release()
        await served.service.shutdown()


async def test_live_native_assessment_keeps_original_proofs_and_path_local_outcomes(
    current_fixture,
):
    native = current_fixture["native"]
    assert native is not None, "actual published member/analysis/formal fixture identities required"
    request = {
        "member": native["member"],
        "analysis": native["analysis"],
        "inputs": [{"formal": native["formal"], "value": {"kind": "string", "value": "ready"}}],
        "assumptions": {"builtin_namespace": "unknown"},
    }
    async with Client(server(current_fixture)) as client:
        response = await client.call_tool("inspect_value_paths", request)
        dto = response.structured_content
        assert dto is not None
        assert dto["generation"] == generation_bytes(current_fixture)
        assert dto["member"] == native["member"]
        paths = dto["paths"]
        if current_fixture["profile"] == "catalog":
            assert paths["availability"]["status"] == "not_requested"
            assert paths["items"] == []
        else:
            assert paths["items"]
            # Verdict is the append-only canonical DomainCode2; ExactOutcome has native
            # snake_case tags. A refuted path must coexist with a compatible sibling path.
            assert any(
                path["verdict"] == 2 and path["exact"] == "refuted_path_under_model"
                for path in paths["items"]
            )
            assert any(path["exact"] == "compatible_under_may_model" for path in paths["items"])
            for path in paths["items"]:
                assert len(path["original_condition"]) == 16
                assert path["proof"]
                assert len(path["path"]["row"]) == 16
                assert all(proof["relation"] and len(proof["row"]) == 16 for proof in path["proof"])
        invalid = {
            **request,
            "inputs": [{"formal": native["formal"], "value": {"kind": "integer", "decimal": "-0"}}],
        }
        with pytest.raises(ToolError):
            await client.call_tool("inspect_value_paths", invalid)
