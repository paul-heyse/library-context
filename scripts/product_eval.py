"""PR0 product task protocol, isolated agent runs and immutable receipts.

Evaluation-only. Neither this module nor its task answers are compiler inputs.
Confirmation execution is intentionally a separate PR6 authorization.
"""

from __future__ import annotations

import argparse
import asyncio
import hashlib
import json
import os
import time
from contextlib import AsyncExitStack
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
PROTOCOL = ROOT / "eval/product/fastmcp-4.0.5/protocol.json"
STRATA = {"discovery": 8, "implementation": 8, "choice": 4, "ambiguity": 4}


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def load_tasks(path: Path) -> list[dict]:
    tasks = json.loads(path.read_text())
    if len(tasks) != 24 or len({task["id"] for task in tasks}) != 24:
        raise ValueError("exactly 24 distinct tasks required")
    counts = {name: sum(task["stratum"] == name for task in tasks) for name in STRATA}
    if counts != STRATA:
        raise ValueError("task strata differ from frozen population")
    if (
        sum(task.get("deployment", False) and task["stratum"] == "implementation" for task in tasks)
        < 2
    ):
        raise ValueError("at least two deployment tasks required")
    for task in tasks:
        if not all(task.get(key) for key in ("prompt", "checks", "evidence_requirements")):
            raise ValueError("task lacks prompt, independent checks or evidence prerequisites")
    return tasks


def write_receipt(path: Path, value: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("x") as output:
        output.write(json.dumps(value, indent=2, sort_keys=True) + "\n")


def check_freeze(directory: Path) -> dict:
    protocol = json.loads((directory / "protocol.json").read_text())
    freeze = json.loads((directory / "freeze.json").read_text())
    if not {"protocol.json", "development.json"} <= set(freeze["sha256"]):
        raise ValueError("required frozen inputs missing")
    for name, expected in freeze["sha256"].items():
        if digest(directory / name) != expected:
            raise ValueError(f"frozen input changed: {name}")
    tasks = load_tasks(directory / "development.json")
    return {"outcome": "passed", "tasks": len(tasks), "protocol": protocol["version"]}


def response_usage(data: dict, model: str) -> dict:
    if data.get("model") != model:
        raise ValueError(
            "response model differs from frozen model; aliases require explicit protocol admission"
        )
    usage = data.get("usage", {})
    if not all(
        type(usage.get(k)) is int and usage[k] >= 0 for k in ("input_tokens", "output_tokens")
    ):
        raise ValueError("required usage telemetry missing or invalid")
    return {k: usage[k] for k in ("input_tokens", "output_tokens")}


async def run_trial(args: argparse.Namespace) -> dict:
    """Expose only MCP function schemas to a fresh, stateless Responses conversation.

    No shell, files, web, built-in tools, skills, user config or prior response identifier is
    provided to the model. Original evidence lives behind the allowlisted condition server.
    API function loop: https://developers.openai.com/api/docs/guides/function-calling
    """
    import httpx2 as httpx
    from fastmcp import Client

    check_freeze(PROTOCOL.parent)
    protocol = json.loads(PROTOCOL.read_text())
    tasks = load_tasks(PROTOCOL.parent / "development.json")
    task = next(task for task in tasks if task["id"] == args.task)
    condition = json.loads(args.condition_config.read_text())
    if set(condition) != {"condition", "parity_receipt", "parity_sha256", "servers"}:
        raise ValueError("condition must contain only identity, parity receipt and MCP servers")
    if condition["condition"] != args.condition:
        raise ValueError("condition identity mismatch")
    out = args.out.resolve()
    out.mkdir(parents=True, exist_ok=False)
    base = {
        "task": args.task,
        "condition": args.condition,
        "protocol_sha256": digest(PROTOCOL),
        "runner_sha256": digest(Path(__file__)),
        "condition_sha256": digest(args.condition_config),
        "task_success": "not_scored",
    }
    try:
        parity_path = args.condition_config.parent / condition["parity_receipt"]
        if digest(parity_path) != condition["parity_sha256"]:
            raise ValueError("condition parity receipt changed")
        parity = json.loads(parity_path.read_text())
        if not isinstance(parity, dict):
            raise ValueError("invalid parity receipt")
        reason = None
        if (
            parity.get("outcome") != "passed"
            or parity.get("release") != protocol["release"]
            or not parity.get("independent_reviewer")
            or not parity.get("evidence_inventory")
            or set(parity.get("matched_conditions", [])) != {"A", "B", "C"}
        ):
            reason = "independently reviewed exact-release/material parity unavailable"
        elif not os.environ.get("OPENAI_API_KEY"):
            reason = "OPENAI_API_KEY unavailable; frozen model is never substituted"
    except (OSError, ValueError, TypeError, KeyError) as error:
        receipt = {
            **base,
            "outcome": "failed",
            "reason": "parity preflight: " + type(error).__name__,
        }
        write_receipt(out / "receipt.json", receipt)
        return receipt
    if reason:
        receipt = {**base, "outcome": "blocked", "reason": reason}
        write_receipt(out / "receipt.json", receipt)
        return receipt
    prompt = (
        "Use only the supplied evidence tools for FastMCP 4.0.5. Cite original evidence. "
        "Distinguish unknown support from absence. Return the requested artifact and citations.\n\n"
        + task["prompt"]
    )
    write_receipt(
        out / "input.json",
        {
            **base,
            "prompt": prompt,
            "model": protocol["model"],
            "reasoning": protocol["reasoning"],
            "limits": protocol["limits"],
        },
    )
    limits = protocol["limits"]
    started = time.monotonic()
    calls = 0
    usage = {"input_tokens": 0, "output_tokens": 0}
    requests_sent = 0
    responses_measured = 0
    outcome, reason = "failed", "no completed answer"
    reported_models = set()
    with (out / "events.jsonl").open("x") as events:

        def record(event):
            events.write(json.dumps(event, sort_keys=True) + "\n")
            events.flush()

        try:
            async with asyncio.timeout(limits["seconds"]), AsyncExitStack() as stack:
                api = await stack.enter_async_context(
                    httpx.AsyncClient(trust_env=False, timeout=60)
                )
                functions, routes = [], {}
                for number, server in enumerate(condition["servers"]):
                    if set(server) != {"url", "tools"} or not server["url"].startswith(
                        ("https://", "http://127.0.0.1:")
                    ):
                        raise ValueError("only explicit HTTP MCP endpoints are supported")
                    client = await stack.enter_async_context(Client(server["url"]))
                    available = {t.name: t for t in await client.list_tools()}
                    for name in server["tools"]:
                        tool = available[name]
                        local_name = f"evidence_{number}_{name.replace('-', '_')}"
                        functions.append(
                            {
                                "type": "function",
                                "name": local_name,
                                "description": tool.description or name,
                                "parameters": tool.input_schema,
                                "strict": False,
                            }
                        )
                        routes[local_name] = (client, name)
                if not functions:
                    raise ValueError("empty condition tool inventory")
                write_receipt(out / "effective-tools.json", {"tools": functions})
                messages = [{"role": "user", "content": prompt}]
                while True:
                    if any(usage[k] >= limits[k] for k in usage):
                        reason = "token budget exhausted"
                        break
                    requests_sent += 1
                    response = await api.post(
                        "https://api.openai.com/v1/responses",
                        headers={"Authorization": "Bearer " + os.environ["OPENAI_API_KEY"]},
                        json={
                            "model": protocol["model"],
                            "reasoning": {"effort": protocol["reasoning"]},
                            "store": False,
                            "include": ["reasoning.encrypted_content"],
                            "tools": functions,
                            "input": messages,
                            "parallel_tool_calls": False,
                            "max_output_tokens": limits["output_tokens"] - usage["output_tokens"],
                        },
                    )
                    if response.status_code != 200:
                        reason = f"model API HTTP {response.status_code}; no fallback or retry"
                        outcome = "blocked"
                        break
                    data = response.json()
                    record({"type": "response", "response": data})
                    measured = response_usage(data, protocol["model"])
                    responses_measured += 1
                    reported_models.add(data["model"])
                    for key in usage:
                        usage[key] += measured[key]
                    if any(usage[k] > limits[k] for k in usage):
                        reason = "token budget exceeded"
                        break
                    messages.extend(data["output"])
                    tool_calls = [x for x in data["output"] if x["type"] == "function_call"]
                    if not tool_calls:
                        if data["status"] == "completed":
                            answer = "\n".join(
                                c["text"]
                                for item in data["output"]
                                if item["type"] == "message"
                                for c in item["content"]
                                if c["type"] == "output_text"
                            )
                            (out / "answer.txt").write_text(answer)
                            outcome, reason = "completed", None
                        else:
                            reason = "incomplete model response"
                        break
                    if calls + len(tool_calls) > limits["tool_calls"]:
                        reason = "tool budget exhausted"
                        break
                    for call in tool_calls:
                        client, name = routes[call["name"]]
                        arguments = json.loads(call["arguments"])
                        calls += 1
                        result = await client.call_tool(name, arguments, raise_on_error=False)
                        content = {
                            "content": [x.model_dump(mode="json") for x in result.content],
                            "structured_content": result.structured_content,
                            "is_error": result.is_error,
                        }
                        record(
                            {
                                "type": "tool",
                                "name": call["name"],
                                "arguments": arguments,
                                "result": content,
                            }
                        )
                        messages.append(
                            {
                                "type": "function_call_output",
                                "call_id": call["call_id"],
                                "output": json.dumps(content),
                            }
                        )
        except TimeoutError:
            outcome, reason = "timeout", "wall-clock budget exhausted"
        except Exception as error:
            # Do not persist exception messages that could contain endpoint credentials.
            outcome, reason = "failed", type(error).__name__
    receipt = {
        **base,
        "outcome": outcome,
        "reason": reason,
        "seconds": time.monotonic() - started,
        "tool_calls": calls,
        "usage": usage if requests_sent == responses_measured else None,
        "usage_complete": requests_sent == responses_measured,
        "observed_usage": usage if responses_measured else None,
        "model_requests": requests_sent,
        "measured_responses": responses_measured,
        "reported_models": sorted(reported_models),
    }
    write_receipt(out / "receipt.json", receipt)
    return receipt


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    commands.add_parser("check-freeze")
    run = commands.add_parser("run-development")
    run.add_argument("--task", required=True)
    run.add_argument("--condition", choices=["A", "B", "C"], required=True)
    run.add_argument("--condition-config", type=Path, required=True)
    run.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    result = (
        check_freeze(PROTOCOL.parent)
        if args.command == "check-freeze"
        else asyncio.run(run_trial(args))
    )
    print(json.dumps(result, sort_keys=True))


if __name__ == "__main__":
    main()
