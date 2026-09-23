"""Held-out fixture T18: per-item progress updates and client-visible log messages.

Tests a candidate module `solution` that exposes a module-level `mcp` (a
`fastmcp.FastMCP`). Offline and deterministic: in-memory client only.
"""

import asyncio

from fastmcp import Client, FastMCP

import solution


def _is_subsequence(needle, haystack):
    it = iter(haystack)
    return all(any(item == other for other in it) for item in needle)


def test_server_object():
    assert isinstance(solution.mcp, FastMCP)


def test_result_progress_and_logs():
    progress_events = []
    log_events = []

    async def on_progress(progress, total, message):
        progress_events.append((progress, total, message))

    async def on_log(message):
        log_events.append((message.level, message.data))

    async def run():
        async with Client(
            solution.mcp, progress_handler=on_progress, log_handler=on_log
        ) as client:
            result = await client.call_tool("process_items", {"items": ["a", "b", "c"]})
            assert result.data == ["A", "B", "C"], result.data

    asyncio.run(run())

    expected = [(1, 3, "a"), (2, 3, "b"), (3, 3, "c")]
    assert _is_subsequence(expected, progress_events), progress_events
    values = [p for p, _, _ in progress_events]
    assert values == sorted(values), f"progress went backwards: {progress_events}"
    assert progress_events[-1][:2] == (3, 3), progress_events

    info = [data for level, data in log_events if level == "info"]
    assert len(info) >= 3, log_events
    for item in ("a", "b", "c"):
        assert any(item in str(data) for data in info), (item, log_events)


if __name__ == "__main__":
    for _name, _fn in sorted(globals().items()):
        if _name.startswith("test_") and callable(_fn):
            _fn()
    print("all tests passed")
