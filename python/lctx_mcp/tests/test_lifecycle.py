"""Bounded native jobs and whole-request cancellation keep their actual ownership."""

import asyncio
from threading import Event

import pytest
from fastmcp.exceptions import ToolError

from lctx_mcp import server

pytestmark = pytest.mark.anyio


async def test_cancelled_cpu_job_retains_its_slot_until_work_ends():
    workers = server.NativeWorkers()
    release = Event()
    entered = [Event(), Event()]

    def work(i):
        entered[i].set()
        release.wait(5)
        return i

    pending = [asyncio.create_task(workers.run(work, i)) for i in range(2)]
    try:
        while not all(e.is_set() for e in entered):
            await asyncio.sleep(0.005)
        pending[0].cancel()
        with pytest.raises(asyncio.CancelledError):
            await pending[0]
        with pytest.raises(ToolError, match="worker capacity"):
            await workers.run(lambda: 3)
        release.set()
        assert await pending[1] == 1
        assert await workers.run(lambda: 4) == 4
    finally:
        release.set()
        await workers.close()


async def test_deadline_covers_multiple_individually_short_phases(monkeypatch):
    monkeypatch.setattr(server, "REQUEST_SECONDS", 0.04)
    completed = []

    @server.request_deadline
    async def request():
        for _ in range(3):
            await asyncio.sleep(0.025)
            completed.append(True)

    with pytest.raises(ToolError, match="request deadline"):
        await request()
    assert len(completed) == 1
