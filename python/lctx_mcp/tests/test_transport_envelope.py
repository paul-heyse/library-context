"""Public MCP protocol serialization controls independent of stored domain fixtures."""

import json

import pytest
from fastmcp.tools import ToolResult
from mcp_types import CallToolResult, ReadResourceResult

from lctx_mcp.wire import response_encodings


@pytest.mark.parametrize("request_id", [17, "17", "résultat"])
def test_actual_rpc_id_and_complete_tool_result_survive_both_writers(request_id):
    result = CallToolResult.model_validate(
        {
            "content": [{"type": "text", "text": "Original evidence: λ"}],
            "structuredContent": {"snapshot": {"semantic": [3] * 32, "realization": [4] * 32, "database": {"namespace": "fixture", "database": "snapshot"}}, "original": "λ\n雪", "score": 0.0},
            "isError": False,
            "_meta": {"trace": "transport metadata"},
        }
    )
    wrapped = ToolResult.from_mcp_result(result)
    assert wrapped.to_mcp_result() is result
    stdio, http = response_encodings(result, request_id)
    assert stdio.endswith(b"\n")
    assert not http.endswith(b"\n")
    expected = {
        "jsonrpc": "2.0",
        "id": request_id,
        "result": {
            "content": [{"type": "text", "text": "Original evidence: λ"}],
            "structuredContent": {"snapshot": {"semantic": [3] * 32, "realization": [4] * 32, "database": {"namespace": "fixture", "database": "snapshot"}}, "original": "λ\n雪", "score": 0.0},
            "isError": False,
            "_meta": {"trace": "transport metadata"},
            "resultType": "complete",
        },
    }
    assert json.loads(stdio) == expected
    assert json.loads(http) == expected
    assert b"\\u03bb" in http
    assert "λ".encode() in stdio
    assert len(http) > len(stdio)


def test_resource_rpc_bytes_preserve_uri_mime_and_unicode_body():
    result = ReadResourceResult.model_validate(
        {
            "contents": [
                {
                    "uri": "lctx://capability/" + "05" * 16,
                    "mimeType": "text/markdown",
                    "text": "# λ\n雪",
                }
            ]
        }
    )
    stdio, http = response_encodings(result, "resource-17")
    expected = {
        "jsonrpc": "2.0",
        "id": "resource-17",
        "result": {
            "contents": [
                {
                    "uri": "lctx://capability/" + "05" * 16,
                    "mimeType": "text/markdown",
                    "text": "# λ\n雪",
                }
            ],
            "resultType": "complete",
            "ttlMs": 0,
            "cacheScope": "private",
        },
    }
    assert json.loads(stdio) == expected
    assert json.loads(http) == expected
    assert len(http) > len(stdio)
