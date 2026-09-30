"""The catalog server: in-memory in both protocol eras, and over a raw stdio pipe.

The raw-pipe test requires every stdout line to be a JSON-RPC message (the MCP client skips a line
it cannot parse, so a round trip through it cannot see stray output), with a noisy control.
"""

from __future__ import annotations

import asyncio
import contextlib
import json
import os
import sys
from pathlib import Path
from types import SimpleNamespace

import pytest
from fastmcp import Client
from fastmcp.exceptions import ToolError

import library_catalog_db as db
import library_catalog_mcp as server_module
import library_semantic as sem

pytestmark = pytest.mark.anyio
SCRIPT = Path(__file__).resolve().parents[2] / "scripts" / "library_catalog_mcp.py"


def payload(result) -> dict:
    assert result.structured_content is not None
    return result.structured_content


MODES = [("auto", "2026-07-28"), ("legacy", "2025-11-25")]

CATALOG = [
    {"kind": "library", "lib": "sqlx", "skill": "sqlx-postgres", "status": "used", "wrappers": []},
    *[
        {
            "kind": "capability",
            "id": f"sqlx/cap-{n:02d}",
            "lib": "sqlx",
            "feature": f"Feature {n}",
            "use": "Query the database",
            "items": [f"sqlx_core::thing::item_{n}"],
            "files": [{"path": "crates/pg/src/a.rs", "role": "impl"}],
            "status": "used",
        }
        for n in range(40)
    ],
]


@pytest.fixture
def anyio_backend() -> str:
    return "asyncio"


def make_root(tmp_path: Path, with_index: bool = True) -> Path:
    (tmp_path / "docs").mkdir()
    (tmp_path / "docs" / "library-utilization.jsonl").write_text(
        "".join(json.dumps(r) + "\n" for r in CATALOG)
    )
    (tmp_path / "crates" / "pg" / "src").mkdir(parents=True)
    (tmp_path / "crates" / "pg" / "src" / "a.rs").write_text("fn f() {}\n")
    (tmp_path / "Cargo.toml").write_text("[workspace]\n[workspace.dependencies]\nsqlx = '1'\n")
    if with_index:
        hits = [
            sem.Hit(
                "crates/pg/src/a.rs",
                "sqlx-core",
                "0.9.0",
                f"sqlx_core::thing::item_{n}",
                "path",
                (n + 1,),
                "fn",
            )
            for n in range(40)
        ]
        meta = {
            "packages": [{"name": "pg", "manifest_path": str(tmp_path / "crates/pg/Cargo.toml")}]
        }
        run = SimpleNamespace(
            root=tmp_path,
            meta=meta,
            hits=hits,
            packages=[("sqlx", "sqlx", "0.9.0"), ("sqlx", "sqlx-core", None)],
            stamp="2026-09-29@abc1234",
        )
        text = (tmp_path / "docs" / "library-utilization.jsonl").read_text()
        db.write_index(
            tmp_path / "build" / "library-usage.sqlite", run, text, sem.Roles(tmp_path, meta)
        )
    return tmp_path


def server_for(root: Path):
    return server_module.build_server(root, graph_loader=lambda _r: (None, "cargo not found"))


@pytest.mark.parametrize(("mode", "era"), MODES)
async def test_tools_are_read_only_return_objects_and_apply_no_default_limit(
    tmp_path, mode, era
) -> None:
    async with Client(server_for(make_root(tmp_path)), mode=mode) as client:
        tools = {t.name: t for t in await client.list_tools()}
        assert set(tools) == {
            "capabilities_by_library",
            "get_library",
            "library_usage",
            "find_item",
            "find_by_file",
            "search_capabilities",
            "catalog_gaps",
            "catalog_status",
        }
        for tool in tools.values():
            assert tool.annotations is not None
            assert (
                tool.annotations.read_only_hint is True and tool.annotations.idempotent_hint is True
            )
            assert tool.annotations.open_world_hint is False
        assert "limit" in tools["search_capabilities"].input_schema["properties"]
        assert (
            tools["search_capabilities"].input_schema["properties"]["limit"].get("default") is None
        )
        result = await client.call_tool("search_capabilities", {"query": "query database"})
        assert payload(result)["total"] == 40 and len(payload(result)["results"]) == 40
        limited = await client.call_tool("search_capabilities", {"query": "", "limit": 3})
        assert payload(limited)["omitted"] == 37
        library = await client.call_tool("get_library", {"lib": "sqlx"})
        assert payload(library)["state"] == "found"
        assert len(payload(library)["capabilities"]) == 40
        usage = await client.call_tool("library_usage", {"lib": "sqlx"})
        assert payload(usage)["totals"]["rows"] == 40
        assert len(payload(usage)["usage"]) == 40  # nothing cut off


@pytest.mark.parametrize(("mode", "era"), MODES)
async def test_absence_missing_index_and_a_missing_catalog_are_answered_honestly(
    tmp_path, mode, era
) -> None:
    async with Client(server_for(make_root(tmp_path)), mode=mode) as client:
        miss = await client.call_tool("find_item", {"item": "nowhere::Widget"})
        assert payload(miss)["state"] == "no_reference_found"
        assert payload(miss)["blind_spots"]
    async with Client(
        server_for(
            make_root(tmp_path / "b", with_index=False)
            if (tmp_path / "b").mkdir() is None
            else tmp_path
        ),
        mode=mode,
    ) as client:
        none = await client.call_tool("find_item", {"item": "nowhere::Widget"})
        assert payload(none)["state"] == "no_usage_index"
    empty = tmp_path / "empty"
    empty.mkdir()
    async with Client(server_for(empty), mode=mode) as client:
        with pytest.raises(ToolError, match="does not exist"):
            await client.call_tool("catalog_status", {})


@pytest.mark.parametrize(("mode", "era"), MODES)
async def test_capabilities_by_library_takes_no_arguments_and_returns_the_grouped_json(
    tmp_path, mode, era
) -> None:
    async with Client(server_for(make_root(tmp_path)), mode=mode) as client:
        tool = next(t for t in await client.list_tools() if t.name == "capabilities_by_library")
        assert tool.input_schema.get("properties", {}) == {}
        body = payload(await client.call_tool("capabilities_by_library", {}))
        assert set(body) == {"libraries"} and body["libraries"]
        for group in body["libraries"].values():
            assert set(group) == {"capabilities"} and group["capabilities"]


async def test_the_schema_resource_and_bad_input(tmp_path) -> None:
    async with Client(server_for(make_root(tmp_path))) as client:
        resources = await client.list_resources()
        assert [str(r.uri) for r in resources] == ["catalog://schema"]
        body = json.loads(getattr((await client.read_resource("catalog://schema"))[0], "text", ""))
        assert "impl" in body["file_roles"] and body["blind_spots"]
        with pytest.raises(ToolError):
            await client.call_tool("catalog_gaps", {"kind": "bogus"})


def test_the_store_is_reloaded_when_the_catalog_file_changes(tmp_path) -> None:
    root = make_root(tmp_path)
    catalog = server_module.Catalog(root, graph_loader=lambda _r: (None, ""))
    first = catalog.context()
    assert catalog.context() is first  # unchanged: the same context
    path = root / "docs" / "library-utilization.jsonl"
    path.write_text(
        path.read_text() + json.dumps({"kind": "library", "lib": "new", "status": "used"}) + "\n"
    )
    os.utime(path, ns=(1, 2))
    assert "new" in catalog.context().store.library_names()


MESSAGES = [
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
        "params": {"name": "catalog_status", "arguments": {}},
    },
]


async def stdout_lines(args: list[str]) -> list[str]:
    process = await asyncio.create_subprocess_exec(
        *args,
        stdin=asyncio.subprocess.PIPE,
        stdout=asyncio.subprocess.PIPE,
        stderr=asyncio.subprocess.DEVNULL,
        limit=1024 * 1024,
        env={**os.environ, "FASTMCP_CHECK_FOR_UPDATES": "off"},
    )
    assert process.stdin is not None and process.stdout is not None
    lines: list[str] = []
    try:
        for message in MESSAGES:
            process.stdin.write((json.dumps(message) + "\n").encode())
        await process.stdin.drain()
        while True:
            raw = await asyncio.wait_for(process.stdout.readline(), timeout=60)
            if not raw:
                break
            line = raw.decode().rstrip("\n")
            lines.append(line)
            try:
                if json.loads(line).get("id") == 3:
                    break
            except ValueError, AttributeError:
                continue
    finally:
        with contextlib.suppress(ProcessLookupError):
            process.kill()
        await process.wait()
    return lines


def not_protocol(lines: list[str]) -> list[str]:
    bad = []
    for line in lines:
        try:
            if json.loads(line).get("jsonrpc") != "2.0":
                bad.append(line)
        except ValueError, AttributeError:
            bad.append(line)
    return bad


async def test_stdout_carries_only_protocol_messages_and_a_noisy_server_fails_the_control(
    tmp_path,
) -> None:
    root = make_root(tmp_path)
    good = await stdout_lines([sys.executable, str(SCRIPT), "--root", str(root)])
    assert good and not not_protocol(good)
    answered = [json.loads(line) for line in good if json.loads(line).get("id") == 3]
    assert answered and answered[0]["result"]["structuredContent"]["libraries"].keys() == {"sqlx"}
    noisy = tmp_path / "noisy.py"
    noisy.write_text(
        "print('not a protocol line', flush=True)\n"
        f"import runpy, sys\nsys.path.insert(0, {str(SCRIPT.parent)!r})\n"
        f"sys.argv = ['x', '--root', {str(root)!r}]\n"
        f"runpy.run_path({str(SCRIPT)!r}, run_name='__main__')\n"
    )
    control = await stdout_lines([sys.executable, str(noisy)])
    assert not_protocol(control) == ["not a protocol line"]
