"""Bounded PG17 stdio workloads, two pinned servers and selected-generation rollover.

The 1x/10x cases repeat the same hydration fixture (transport cost, not corpus scalability).
Run concurrently with the separately recorded compile/import/report processes to measure contention.
Only the selection pointer is changed, then restored. Both supplied bundles must already be ready.
"""

from __future__ import annotations

import argparse
import asyncio
import json
import math
import os
import sys
import time
from contextlib import AsyncExitStack
from pathlib import Path

from fastmcp import Client
from fastmcp.client.transports import StdioTransport
from lctx_storage import open_repository

ROOT = Path(__file__).resolve().parents[1]


def process_memory():
    # Aggregate Linux RSS of this harness and its owned descendants; excludes PG/vLLM.
    pending, seen, rss = [os.getpid()], set(), 0
    while pending:
        pid = pending.pop()
        if pid in seen:
            continue
        seen.add(pid)
        try:
            fields = (Path("/proc") / str(pid) / "status").read_text().splitlines()
            rss += next(int(x.split()[1]) * 1024 for x in fields if x.startswith("VmRSS:"))
            for task in (Path("/proc") / str(pid) / "task").iterdir():
                pending.extend(map(int, (task / "children").read_text().split()))
        except OSError, StopIteration:
            pass
    return rss


async def execute(args):
    manifests = [json.loads((p / "MANIFEST.json").read_text()) for p in args.bundles]
    library = manifests[0]["library"]
    if (
        len(manifests) != 2
        or any(m["library"] != library for m in manifests)
        or manifests[0]["projection_generation"] == manifests[1]["projection_generation"]
    ):
        raise ValueError("two distinct ready generations of one library are required")
    generations = [m["projection_generation"] for m in manifests]
    requests = json.loads(args.requests.read_text())
    if not 1 <= len(requests) <= 64:
        raise ValueError("one through 64 frozen requests required")
    repository = await open_repository(args.config)
    initial = json.loads((await repository.pin(library)).descriptor())

    async def select(generation, profile=None):
        command = [
            str(ROOT / "target/release/lctx"),
            "serving",
            "--importer-config",
            str(args.importer),
            "select",
            "--library",
            library,
            "--generation",
            generation,
        ]
        if profile:
            command += ["--profile", profile]
        process = await asyncio.create_subprocess_exec(
            *command, stdout=asyncio.subprocess.PIPE, stderr=asyncio.subprocess.PIPE
        )
        _, _error = await process.communicate()
        if process.returncode:
            raise RuntimeError("selection command failed")

    def client(embedder):
        return Client(
            StdioTransport(
                command=sys.executable,
                args=[
                    "-m",
                    "lctx_mcp",
                    "--config",
                    str(args.config),
                    "--library",
                    library,
                    "--embedder",
                    embedder,
                ],
                env={**os.environ, "FASTMCP_CHECK_FOR_UPDATES": "off"},
                keep_alive=False,
            )
        )

    result = {
        "format": 1,
        "outcome": "passed",
        "generations": generations,
        "embedder": args.embedder,
        "rss_scope": "harness and owned MCP processes; excludes PostgreSQL and vLLM",
        "workloads": [],
    }
    peak = process_memory()
    stop = asyncio.Event()

    async def sample():
        nonlocal peak
        while not stop.is_set():
            peak = max(peak, process_memory())
            await asyncio.sleep(0.1)

    sampler = asyncio.create_task(sample())
    started = time.monotonic()
    try:
        async with AsyncExitStack() as stack:
            await select(generations[0])
            a = await stack.enter_async_context(client("none"))
            first = (
                await a.call_tool(
                    "find_operations", {"library": library, "selection": {}, "limit": 1}
                )
            ).structured_content
            if not first or not first["supported"].get("next_cursor"):
                raise RuntimeError("rollover fixture needs a continuation cursor")
            cursor = first["supported"]["next_cursor"]
            continuation = (
                await a.call_tool(
                    "find_operations",
                    {"library": library, "selection": {}, "limit": 1, "cursor": cursor},
                )
            ).structured_content
            await select(generations[1])
            b = await stack.enter_async_context(client(args.embedder))
            second = (
                await b.call_tool(
                    "find_operations", {"library": library, "selection": {}, "limit": 1}
                )
            ).structured_content
            repeated = (
                await a.call_tool(
                    "find_operations",
                    {"library": library, "selection": {}, "limit": 1, "cursor": cursor},
                )
            ).structured_content
            if (
                not second
                or first["generation"] != generations[0]
                or second["generation"] != generations[1]
                or repeated != continuation
            ):
                raise RuntimeError("selected rollover changed an existing pin/cursor")
            rejected = await b.call_tool(
                "find_operations",
                {"library": library, "selection": {}, "limit": 1, "cursor": cursor},
                raise_on_error=False,
            )
            if not rejected.is_error:
                raise RuntimeError("a cursor crossed generations")
            result["startup_two_servers_seconds"] = time.monotonic() - started
            result["rollover"] = "passed"
            for concurrency in (1, 4):
                sem = asyncio.Semaphore(concurrency)
                samples = []

                async def search_one(i, sem=sem, samples=samples):
                    request = requests[i % len(requests)]
                    params = {"library": library, "query": request["query"], "limit": 10}
                    if request.get("selection"):
                        params["selection"] = request["selection"]
                    async with sem:
                        start = time.monotonic()
                        answer = await b.call_tool(
                            "search_operations" if request["operations"] else "search_capabilities",
                            params,
                        )
                        samples.append((time.monotonic() - start) * 1000)
                        if not answer.structured_content:
                            raise RuntimeError("empty MCP response")

                await asyncio.gather(*(search_one(i) for i in range(len(requests) * 3)))
                ordered = sorted(samples)
                result["workloads"].append(
                    {
                        "kind": "MCP search including embedding",
                        "concurrency": concurrency,
                        "samples": len(samples),
                        "p95_ms": ordered[math.ceil(0.95 * len(ordered)) - 1],
                    }
                )
            scope = json.loads(
                (await (await repository.pin(library, generations[1])).prepare_selection()).scope()
            )
            entity = scope["eligible"][0]
            for multiplier in (1, 10):
                start = time.monotonic()
                size = 0
                for _ in range(10 * multiplier):
                    answer = await b.call_tool(
                        "get_operation",
                        {"snapshot_id": manifests[1]["snapshot_id"], "operation": entity},
                    )
                    size += len(json.dumps(answer.structured_content).encode())
                result["workloads"].append(
                    {
                        "kind": "repeated hydration transport fixture",
                        "multiplier": multiplier,
                        "calls": 10 * multiplier,
                        "bytes": size,
                        "seconds": time.monotonic() - start,
                    }
                )
            result["diagnostics"] = json.loads(
                await (await repository.pin(library, generations[1])).diagnostics()
            )
    finally:
        stop.set()
        await sampler
        try:
            await select(initial["generation"], initial["profile"])
        finally:
            await repository.close()
    result["peak_aggregate_rss_bytes"] = peak
    result["total_seconds"] = time.monotonic() - started
    with args.out.open("x") as output:
        json.dump(result, output, indent=2)
    print(json.dumps(result))


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("bundles", nargs=2, type=Path)
    p.add_argument("--config", type=Path, required=True)
    p.add_argument("--importer", type=Path, required=True)
    p.add_argument("--requests", type=Path, required=True)
    p.add_argument("--embedder", choices=["none", "fake", "vllm"], default="none")
    p.add_argument("--out", type=Path, required=True)
    asyncio.run(execute(p.parse_args()))


if __name__ == "__main__":
    main()
