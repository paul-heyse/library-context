"""Drive SurrealDB's HTTP /mcp endpoint (stdlib only).

Credentials come from the environment (MCP_USER, MCP_PASS) and are never
printed. Every call prints a compact JSON line: the step label, HTTP status,
and the JSON-RPC result or error (long values clipped).

Usage: mcp_probe.py BASE_URL SCENARIO
Scenarios: root-legacy, stateless, viewer, host-check, truncation
"""

import base64
import json
import os
import sys
import urllib.error
import urllib.request

BASE = sys.argv[1].rstrip("/") + "/mcp"
SCENARIO = sys.argv[2]
AUTH = "Basic " + base64.b64encode(f"{os.environ['MCP_USER']}:{os.environ['MCP_PASS']}".encode()).decode()


def clip(value, n: int = 700):
    text = json.dumps(value, default=str)
    return text if len(text) <= n else text[:n] + f"...<{len(text)} chars>"


def post(label: str, body: dict, session: str | None = None, extra: dict | None = None, quiet=False):
    headers = {
        "Content-Type": "application/json",
        "Accept": "application/json, text/event-stream",
        "Authorization": AUTH,
        "surreal-ns": "probe",
        "surreal-db": "probe",
    }
    if os.environ.get("MCP_AUTH_LEVEL") == "db":
        # Basic auth for a database user names its level explicitly.
        headers["surreal-auth-ns"] = "probe"
        headers["surreal-auth-db"] = "probe"
    if session:
        headers["mcp-session-id"] = session
    headers.update(extra or {})
    req = urllib.request.Request(BASE, data=json.dumps(body).encode(), headers=headers, method="POST")
    try:
        with urllib.request.urlopen(req, timeout=30) as resp:
            status, raw, rh = resp.status, resp.read().decode(), dict(resp.headers)
    except urllib.error.HTTPError as err:
        status, raw, rh = err.code, err.read().decode(), dict(err.headers)
    payload = None
    for line in raw.splitlines():
        if line.startswith("data:") and line[5:].strip():
            payload = json.loads(line[5:])
    if payload is None and raw.strip().startswith("{"):
        payload = json.loads(raw)
    if not quiet:
        out = {"step": label, "http": status}
        if payload is None:
            out["body"] = raw[:300]
        elif "error" in payload:
            out["error"] = payload["error"]
        else:
            out["result"] = payload.get("result")
        print(clip(out))
    return status, payload, rh


def call(label, name, args, session=None, extra=None):
    return post(label, {"jsonrpc": "2.0", "id": label, "method": "tools/call", "params": {"name": name, "arguments": args}}, session, extra)


def legacy_session(version="2025-11-25") -> str:
    _, payload, rh = post(
        "initialize",
        {
            "jsonrpc": "2.0",
            "id": 0,
            "method": "initialize",
            "params": {"protocolVersion": version, "capabilities": {}, "clientInfo": {"name": "opdiag-probe", "version": "0"}},
        },
        quiet=True,
    )
    sid = {k.lower(): v for k, v in rh.items()}.get("mcp-session-id")
    print(clip({"step": "initialize", "protocolVersion": (payload or {}).get("result", {}).get("protocolVersion"), "session": bool(sid)}))
    post("initialized", {"jsonrpc": "2.0", "method": "notifications/initialized"}, sid, quiet=True)
    return sid


if SCENARIO == "root-legacy":
    sid = legacy_session()
    _, tl, _ = post("tools/list", {"jsonrpc": "2.0", "id": "tl", "method": "tools/list"}, sid, quiet=True)
    tools = [(t["name"], (t.get("annotations") or {}).get("readOnlyHint")) for t in tl["result"]["tools"]]
    print(clip({"step": "tools/list", "tools": tools}, 2000))
    call("query-multi", "query", {"query": "SELECT * FROM prog; SELECT * FROM nope_table; RETURN sleep(20ms);"}, sid)
    call("info-db", "info", {}, sid)
    call("list tables", "list", {"kind": "tables"}, sid)
    _, rl, _ = post("resources/list", {"jsonrpc": "2.0", "id": "rl", "method": "resources/list"}, sid, quiet=True)
    print(clip({"step": "resources/list", "uris": [r["uri"] for r in rl["result"]["resources"]]}))
    for uri in [r["uri"] for r in rl["result"]["resources"] if r["uri"] != "surrealdb://instructions"] + [
        "surrealdb://schema/ns/probe/db/probe",
        "surrealdb://schema/ns/probe/db/probe/table/prog",
    ]:
        post(f"read {uri}", {"jsonrpc": "2.0", "id": uri, "method": "resources/read", "params": {"uri": uri}}, sid)
elif SCENARIO == "stateless":
    hdr = {"mcp-protocol-version": "2026-07-28", "mcp-method": "tools/call", "mcp-name": "query"}
    meta = {"_meta": {"io.modelcontextprotocol/protocolVersion": "2026-07-28", "io.modelcontextprotocol/clientInfo": {"name": "opdiag-probe", "version": "0"}, "io.modelcontextprotocol/clientCapabilities": {}}}
    post("stateless tools/call query", {"jsonrpc": "2.0", "id": "s1", "method": "tools/call", "params": {"name": "query", "arguments": {"query": "SELECT count() FROM prog GROUP ALL;"}, **meta}}, None, hdr)
    post("legacy-without-initialize", {"jsonrpc": "2.0", "id": "s2", "method": "tools/call", "params": {"name": "query", "arguments": {"query": "RETURN 1;"}}}, None, {"mcp-protocol-version": "2025-11-25"})
elif SCENARIO == "viewer":
    sid = legacy_session()
    call("viewer select", "query", {"query": "SELECT * FROM prog;"}, sid)
    call("viewer create", "query", {"query": "CREATE prog:99 SET phase = 'viewer';"}, sid)
    call("viewer update", "query", {"query": "UPDATE prog:1 SET phase = 'changed';"}, sid)
    call("viewer delete", "query", {"query": "DELETE prog:2;"}, sid)
    call("viewer define", "query", {"query": "DEFINE TABLE viewer_made;"}, sid)
    call("viewer create tool", "create", {"target": "prog", "data": {"phase": "tool"}}, sid)
    call("viewer info root", "query", {"query": "INFO FOR ROOT;"}, sid)
    call("viewer select after", "query", {"query": "SELECT * FROM prog;"}, sid)
elif SCENARIO == "host-check":
    post("foreign Host header", {"jsonrpc": "2.0", "id": 1, "method": "ping"}, None, {"Host": "evil.example.com"})
elif SCENARIO == "truncation":
    sid = legacy_session()
    call("large result", "query", {"query": "RETURN string::repeat('x', 300000);"}, sid)
