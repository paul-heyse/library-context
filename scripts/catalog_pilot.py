"""Produce and independently inspect the pinned FastMCP Catalog on an explicit fixture.

This adapter uses the existing CLI, run, acquisition and fixture owners. It neither selects an
operator publication nor synchronizes an analysis environment on its own.
"""

from __future__ import annotations

import argparse
import ast
import contextlib
import hashlib
import json
import os
import signal
import sys
import time
import tomllib
from collections.abc import Iterator
from pathlib import Path
from typing import Any, Protocol

from blake3 import blake3

from harness import SpawnGuard, observe_exit, spawn_group
from runs import current_run, set_progress
from storage_acquisition import Lease
from storage_lifecycle import Storage, admission

ROOT = Path(__file__).resolve().parents[1]
JOURNEY_SECONDS = 3600


class PilotError(RuntimeError):
    """A functional journey failed; partial files and native obligations remain owned."""


class OriginalMismatch(PilotError):
    """A navigation candidate does not point to the independently chosen source."""


class ToolCaller(Protocol):
    def tool(self, name: str, request: dict[str, Any]) -> dict[str, Any]: ...


class EvidenceSearch(ToolCaller, Protocol):
    def pages(
        self,
        name: str,
        request: dict[str, Any],
        section: str,
    ) -> Iterator[dict[str, Any]]: ...


def write_json(path: Path, value: Any) -> None:
    descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    with os.fdopen(descriptor, "w", encoding="utf-8") as stream:
        json.dump(value, stream, indent=2)
        stream.write("\n")


def require(condition: object, message: str) -> None:
    if not condition:
        raise PilotError(message)


class Commands:
    """Each CLI child belongs prospectively to its enclosing managed run."""

    def __init__(
        self,
        lctx: Path,
        output: Path,
        runtime: Path,
        *,
        seconds: float = JOURNEY_SECONDS,
    ):
        self.lctx = lctx
        self.output = output
        self.runtime = runtime
        self.deadline = time.monotonic() + seconds
        self.sequence = 0

    def run(self, arguments: list[str], label: str) -> dict[str, Any]:
        require(time.monotonic() < self.deadline, f"journey deadline before {label}")
        self.sequence += 1
        prefix = self.output / f"{self.sequence:04d}-{label}"
        stdout = prefix.with_suffix(".json")
        stderr = prefix.with_suffix(".stderr.log")
        set_progress(current_command=label)
        failure: BaseException | None = None
        status: int | None = None
        with stdout.open("xb") as out, stderr.open("xb") as err:
            child = spawn_group(
                [str(self.lctx), *arguments],
                cwd=ROOT,
                stdout=out,
                stderr=err,
                death_signal=signal.SIGTERM,
            )
            guard = SpawnGuard(child)
            try:
                with guard:
                    while (status := observe_exit(child)) is None:
                        if time.monotonic() >= self.deadline:
                            with contextlib.suppress(ProcessLookupError):
                                os.kill(child.pid, signal.SIGINT)
                            raise PilotError(f"journey deadline during {label}")
                        time.sleep(0.05)
            except BaseException as error:
                failure = error
            finally:
                write_json(
                    prefix.with_suffix(".child.json"),
                    {
                        "label": label,
                        "exit": guard.returncode,
                        "cleanup": guard.cleanup,
                        "outcome": "failed" if failure or status != 0 else "passed",
                    },
                )
        if guard.cleanup.get("status") != "confirmed":
            cleanup_error = PilotError(f"child cleanup unresolved: {label}")
            if failure is not None:
                raise BaseExceptionGroup(
                    "pilot operation and cleanup failed",
                    [failure, cleanup_error],
                )
            raise cleanup_error
        if failure is not None:
            raise failure
        require(status == 0, f"CLI {label} failed ({status}); see {stderr.name}")
        try:
            value = json.loads(stdout.read_bytes())
        except ValueError as error:
            raise PilotError(f"CLI {label} did not return one JSON result") from error
        require(isinstance(value, dict), f"CLI {label} returned a non-object")
        return value

    def tool(self, name: str, request: dict[str, Any]) -> dict[str, Any]:
        path = self.output / f"request-{self.sequence + 1:04d}.json"
        write_json(path, request)
        result = self.run(
            [
                "tool",
                name,
                "--request",
                str(path),
                "--handle",
                str(self.output / "handle.json"),
                "--runtime-config",
                str(self.runtime),
            ],
            name,
        )
        expected = json.loads((self.output / "handle.json").read_bytes())
        require(result.get("snapshot") == expected, f"{name} changed exact publication handle")
        return result

    def pages(self, name: str, request: dict[str, Any], section: str) -> Iterator[dict[str, Any]]:
        request = {**request, "page": {"size": 64, "expanded": True}}
        seen = set()
        while True:
            result = self.tool(name, request)
            yield result
            cursor = result[section].get("continuation")
            if cursor is None:
                require(
                    not result[section].get("truncated"),
                    f"{name} truncated without continuation",
                )
                return
            token = json.dumps(cursor, sort_keys=True)
            require(token not in seen, f"{name} continuation made no progress")
            seen.add(token)
            request = {**request, "page": {**request["page"], "cursor": cursor}}


def parameters(node: ast.FunctionDef | ast.AsyncFunctionDef) -> list[dict[str, Any]]:
    positional = [*node.args.posonlyargs, *node.args.args]
    offset = len(positional) - len(node.args.defaults)
    values = [{"name": arg.arg, "required": index < offset} for index, arg in enumerate(positional)]
    values.extend(
        {"name": arg.arg, "required": default is None}
        for arg, default in zip(node.args.kwonlyargs, node.args.kw_defaults, strict=True)
    )
    return [value for value in values if value["name"] not in {"self", "cls"}]


def capture_oracle(output: Path) -> dict[str, Any]:
    specification = tomllib.loads((ROOT / "libraries/fastmcp/pyproject.toml").read_text())
    require(
        any("==4.0.5" in item for item in specification["project"]["dependencies"]),
        "pilot specification no longer pins FastMCP 4.0.5",
    )
    source = specification["tool"]["lctx"]["source"]
    lease = Lease(
        {
            "library": str(ROOT / "libraries/fastmcp"),
            "environment_root": str(ROOT / "build/envs/fastmcp"),
            "source_root": str(ROOT / "build/sources/fastmcp"),
            "synchronize": False,
            "reinstall": False,
            "source": {key: source[key] for key in ("repository", "commit")},
        }
    )
    try:
        selected = lease.prepare()
        environment = Path(selected["environment"])
        cfg = dict(
            line.split(" = ", 1)
            for line in (environment / "pyvenv.cfg").read_text().splitlines()
            if " = " in line
        )
        version = cfg["version_info"].split(".")
        package = environment / "lib" / f"python{version[0]}.{version[1]}" / "site-packages/fastmcp"
        corpus = Path(selected["source"])
        originals: dict[str, bytes] = {}
        for name in ("__init__.py", "server/server.py"):
            originals[f"package/fastmcp/{name}"] = (package / name).read_bytes()
        init_tree = ast.parse(originals["package/fastmcp/__init__.py"])
        exports = {
            node.names[0].name
            for node in ast.walk(init_tree)
            if isinstance(node, ast.ImportFrom)
            and node.module
            and node.module.startswith("fastmcp")
        }
        require("FastMCP" in exports and "Client" in exports, "pinned top-level exports changed")
        server = ast.parse(originals["package/fastmcp/server/server.py"])
        cls = next(
            node
            for node in server.body
            if isinstance(node, ast.ClassDef) and node.name == "FastMCP"
        )
        methods = {
            node.name: parameters(node)
            for node in cls.body
            if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef))
            and node.name in {"tool", "resource"}
        }
        require(set(methods) == {"tool", "resource"}, "pinned decorator declarations missing")
        examples: dict[str, dict[str, Any]] = {}
        for path in sorted(corpus.glob("examples/**/*.py")):
            raw = path.read_bytes()
            try:
                tree = ast.parse(raw)
            except SyntaxError, UnicodeError:
                continue
            for node in ast.walk(tree):
                if not isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)):
                    continue
                decorators = [ast.unparse(item) for item in node.decorator_list]
                key = None
                if node.name in {"echo", "add"} and any(".tool" in d for d in decorators):
                    key = node.name
                if key is None and any(".resource" in d and "config" in d for d in decorators):
                    key = "config_resource"
                if key is not None and key not in examples:
                    name = f"corpus/{path.relative_to(corpus).as_posix()}"
                    originals[name] = raw
                    lines = raw.splitlines(keepends=True)
                    start = sum(map(len, lines[: node.lineno - 1])) + node.col_offset
                    end_line = node.end_lineno
                    end_column = node.end_col_offset
                    if end_line is None or end_column is None:
                        raise PilotError("parsed example declaration has no source range")
                    end = sum(map(len, lines[: end_line - 1])) + end_column
                    examples[key] = {
                        "file": name,
                        "name": node.name,
                        "parameters": parameters(node),
                        "decorators": decorators,
                        "start": start,
                        "end": end,
                        "operation": "resource" if key == "config_resource" else "tool",
                    }
            if set(examples) == {"echo", "add", "config_resource"}:
                break
        require(
            set(examples) == {"echo", "add", "config_resource"},
            "pinned corpus lacks expected echo/add/config resource declarations",
        )
        files = []
        oracle_dir = output / "oracle"
        oracle_dir.mkdir()
        for index, (name, raw) in enumerate(originals.items()):
            relative = f"oracle/{index:02d}.source"
            (output / relative).write_bytes(raw)
            files.append(
                {
                    "name": name,
                    "saved": relative,
                    "sha256": hashlib.sha256(raw).hexdigest(),
                }
            )
        value = {
            "schema": 1,
            "version": "4.0.5",
            "commit": source["commit"],
            "methods": methods,
            "examples": examples,
            "files": files,
            "qualification": "declared source syntax; not runtime or effective behavior",
        }
        write_json(output / "oracle.json", value)
        return value
    finally:
        lease.close(acknowledged=True)


def check_signature(core: dict[str, Any], expected: list[dict[str, Any]], label: str) -> None:
    names = {row["name"] for row in expected}
    signatures = core.get("signatures", [])
    matching = [
        sig for sig in signatures if names <= {row.get("name") for row in sig["parameters"]}
    ]
    require(matching, f"{label}: captured source parameter set missing")
    # Effective parameters may differ legitimately. Check defaults only on the preserved source
    # parameter view; Unknown/Unavailable remain explicit and are never relabelled as absence.
    for sig in matching:
        indexed = {row.get("name"): row for row in sig["parameters"]}
        valid = True
        for expected_row in expected:
            actual = indexed[expected_row["name"]]
            default_absent = actual["default"]["kind"] == "absent"
            if (
                actual["required"] != expected_row["required"]
                or default_absent != expected_row["required"]
            ):
                valid = False
                break
        if valid:
            return
    raise PilotError(f"{label}: declared parameter requiredness or default presence changed")


def check_original(
    commands: ToolCaller,
    linked: dict[str, Any],
    originals: list[bytes],
    expected_span: tuple[int, int] | None = None,
) -> dict[str, Any]:
    request = {"source": linked["source"], "page": {"size": 64, "expanded": True}}
    assembled = bytearray()
    initial: dict[str, Any] | None = None
    seen = set()
    while True:
        packet = commands.tool("get_evidence", request)["evidence"]
        original = packet["original"]
        if initial is None:
            initial = original
            require(original["encoding"] == "raw_bytes", "original evidence is not raw bytes")
            for field in ("source", "artifact", "digest", "encoding", "release", "context"):
                require(original[field] == linked[field], f"original changed linked {field}")
            require(
                original["start"] <= linked["start"] and original["end"] >= linked["end"],
                "original does not contain linked evidence range",
            )
        require(original == initial, "original attribution changed across continuation")
        body = packet["body"]
        require(body["start"] == len(assembled), "original continuation offset gap or overlap")
        raw = bytes(body["bytes"])
        require(body["end"] - body["start"] == len(raw), "original body byte count differs")
        assembled.extend(raw)
        cursor = body.get("continuation")
        if cursor is None:
            require(not body["truncated"] and body["omitted"] == 0, "original ended incomplete")
            break
        token = json.dumps(cursor, sort_keys=True)
        require(token not in seen, "original continuation made no progress")
        seen.add(token)
        request = {**request, "page": {**request["page"], "cursor": cursor}}
    require(initial is not None, "missing original range")
    start, end = initial["start"], initial["end"]
    require(len(assembled) == end - start and end > start, "original range was not fully delivered")
    matches = [raw for raw in originals if end <= len(raw) and raw[start:end] == assembled]
    if not matches or (
        expected_span is not None and not (start <= expected_span[0] and end >= expected_span[1])
    ):
        raise OriginalMismatch(
            "served original does not contain the independently chosen declaration"
        )
    matching_artifact = next(
        (raw for raw in matches if list(blake3(raw).digest()) == initial["digest"]),
        None,
    )
    require(
        matching_artifact is not None,
        "original digest differs from independent BLAKE3 of the complete pinned artifact",
    )
    return {
        "artifact": initial["artifact"],
        "native_digest": initial["digest"],
        "start": start,
        "end": end,
        "sha256": hashlib.sha256(assembled).hexdigest(),
        "continuations": len(seen),
        "digest_comparison": "independent BLAKE3 of complete leased artifact",
    }


def check_example_original(
    commands: EvidenceSearch,
    example: dict[str, Any],
    raw: bytes,
    member: Any,
) -> dict[str, Any]:
    for response in commands.pages(
        "search_evidence",
        {
            "library": "fastmcp",
            "query": example["name"],
            "families": [],
        },
        "results",
    ):
        require(
            response["channels"]["vector"]["status"] == "disabled",
            "evidence search requested inference",
        )
        for hit in response["results"]["items"]:
            if member not in hit["associated_members"]:
                continue
            for original in hit["originals"]:
                # Interpreted prose slices are valid navigation results, but only raw
                # originals can satisfy the independent source-file comparison.
                if original["encoding"] != "raw_bytes":
                    continue
                try:
                    return check_original(
                        commands,
                        original,
                        [raw],
                        (example["start"], example["end"]),
                    )
                except OriginalMismatch:
                    continue
    raise PilotError(f"native evidence omitted pinned example {example['name']}")


def check_operation_search(commands: EvidenceSearch, member: Any) -> dict[str, Any]:
    for pages, response in enumerate(
        commands.pages(
            "search_operations",
            {
                "library": "fastmcp",
                "query": "tool",
            },
            "results",
        ),
        start=1,
    ):
        require(
            response["channels"]["vector"]["status"] == "disabled",
            "pilot requested vector inference",
        )
        # SearchOperationsResponse.results contains OperationCandidate packets;
        # navigation rank is not the identity of the independently selected member.
        if any(candidate["member"] == member for candidate in response["results"]["items"]):
            return {"member": member, "pages": pages}
    raise PilotError("native lexical operation search omitted independently selected FastMCP.tool")


def check_catalog(commands: Commands, oracle: dict[str, Any]) -> dict[str, Any]:
    for item in oracle["files"]:
        raw = (commands.output / item["saved"]).read_bytes()
        require(
            hashlib.sha256(raw).hexdigest() == item["sha256"],
            "independent oracle bytes changed",
        )
    exported = commands.tool(
        "get_operation",
        {
            "library": "fastmcp",
            "operation": {"kind": "public_path", "path": ["fastmcp", "FastMCP"]},
            "sections": ["access_routes"],
            "page": {"size": 64, "expanded": True},
        },
    )
    require(
        exported["operation"]["resolution"] == "unique",
        "top-level FastMCP export did not resolve",
    )
    require(
        exported["operation"]["packet"]["core"]["release"]["version"] == "4.0.5",
        "FastMCP export has a foreign release",
    )
    client_export = commands.tool(
        "get_operation",
        {
            "library": "fastmcp",
            "operation": {"kind": "public_path", "path": ["fastmcp", "Client"]},
            "page": {"size": 64, "expanded": True},
        },
    )
    require(
        client_export["operation"]["resolution"] == "unique",
        "top-level Client export did not resolve",
    )
    class_member = exported["operation"]["packet"]["core"]["member"]
    members = []
    for page in commands.pages(
        "browse_library",
        {
            "library": "fastmcp",
            "scope": {"kind": "class", "member": class_member},
            "view": "members",
        },
        "entries",
    ):
        members.extend(
            entry["candidate"] for entry in page["entries"]["items"] if entry["kind"] == "member"
        )
    selected = {}
    for method in ("tool", "resource"):
        candidates = [row for row in members if row["name"] == method]
        require(len(candidates) == 1, f"FastMCP.{method} did not browse uniquely")
        result = commands.tool(
            "get_operation",
            {
                "library": "fastmcp",
                "operation": {"kind": "member", "member": candidates[0]["member"]},
                "sections": ["access_routes", "contextual_typing", "briefs", "scenarios"],
                "page": {"size": 64, "expanded": True},
            },
        )
        require(result["operation"]["resolution"] == "unique", f"FastMCP.{method} did not resolve")
        packet = result["operation"]["packet"]
        check_signature(packet["core"], oracle["methods"][method], method)
        require(packet["core"]["options"], f"FastMCP.{method}: declared options missing")
        require(
            packet["access_routes"]["items"],
            f"FastMCP.{method}: linked access analysis missing",
        )
        selected[method] = packet["core"]["member"]
    comparison = commands.tool(
        "compare_operations",
        {
            "library": "fastmcp",
            "operations": [{"kind": "member", "member": member} for member in selected.values()],
            "page": {"size": 64, "expanded": True},
        },
    )
    require(len(comparison["operations"]) == 2, "comparison omitted requested operations")
    for entry, member in zip(comparison["operations"], selected.values(), strict=True):
        require(
            not entry["ambiguous"]
            and entry["candidates"]
            and all(candidate["member"] == member for candidate in entry["candidates"]),
            "comparison changed requested generated operation identities",
        )
    search = check_operation_search(commands, selected["tool"])
    capabilities = commands.tool(
        "search_capabilities",
        {
            "library": "fastmcp",
            "query": "tool",
            "page": {"size": 64, "expanded": True},
        },
    )
    require(
        capabilities["channels"]["vector"]["status"] == "disabled",
        "capability search requested inference",
    )
    require(capabilities["results"]["items"], "native capability search returned no results")
    capability = commands.tool(
        "get_capability",
        {
            "capability": capabilities["results"]["items"][0]["capability"],
            "page": {"size": 64, "expanded": True},
        },
    )
    require(
        capability["capability"]["assertions"],
        "native capability omitted generated assertions",
    )
    evidence = []
    for example in oracle["examples"].values():
        wanted = next(row for row in oracle["files"] if row["name"] == example["file"])
        raw = (commands.output / wanted["saved"]).read_bytes()
        evidence.append(
            check_example_original(commands, example, raw, selected[example["operation"]])
        )
    return {
        "outcome": "passed",
        "methods": selected,
        "operation_search": search,
        "originals": evidence,
        "original_comparison": "independent leased bytes, SHA256 and full-artifact BLAKE3",
        "inference_started": False,
    }


def retain_viewer(runtime: Path, handle: dict[str, Any]) -> Path:
    value = os.environ.get("LCTX_RETAIN_NATIVE_FIXTURE_CONFIG")
    require(value, "produce requires fixture --retain-serving fastmcp-v03")
    path = Path(str(value)).resolve()
    selection = path.with_suffix(".selected.json")
    configured = json.loads(runtime.read_bytes())
    viewer = {key: configured[key] for key in ("endpoint", "username", "password")}
    viewer["selection"] = str(selection)
    if configured.get("serving_limits") is not None:
        viewer["serving_limits"] = configured["serving_limits"]
    with admission([path.parent]):
        write_json(selection, handle)
        write_json(path, viewer)
    return path


def execute(mode: str, lctx: Path, output: Path) -> dict[str, Any]:
    require(
        os.environ.get("LCTX_COMPILER_RUNTIME_CONFIG"),
        "explicit fixture runtime configuration required",
    )
    runtime = Path(os.environ["LCTX_COMPILER_RUNTIME_CONFIG"]).resolve()
    run = current_run()
    require(run is not None, "catalog pilot must run under just run")
    require(lctx.is_file(), "build the current lctx executable before the pilot")
    output = output.resolve()
    with admission([output]):
        if mode == "produce":
            output.mkdir(parents=True, exist_ok=False)
            storage = Storage()
            identity = storage.publish(
                output,
                "run-receipt",
                {"kind": "run", "path": str(run)},
                "catalog-pilot:fastmcp-v03",
                managed=True,
            )
            write_json(
                output / "owner.json",
                {
                    "storage": identity,
                    "consumer": "catalog-pilot:fastmcp-v03",
                },
            )
        else:
            require(output.is_dir(), "pilot output missing")
        # A check attaches through a new fixture and leaves original production logs immutable.
        command_output = output if mode == "produce" else output / f"check-{Path(str(run)).name}"
        if mode == "check":
            command_output.mkdir(exist_ok=False)
        commands = Commands(lctx.resolve(), command_output, runtime)
        try:
            if mode == "produce":
                compiled = commands.run(
                    [
                        "compile",
                        "fastmcp",
                        "--profile",
                        "catalog",
                        "--through",
                        "catalog",
                        "--artifact-only",
                        "--output",
                        str(output / "artifact"),
                        "--runtime-config",
                        str(runtime),
                    ],
                    "compile",
                )
                require(
                    compiled.get("published") is False
                    and compiled.get("frontier") == "catalog"
                    and compiled.get("profile") == "catalog"
                    and isinstance(compiled.get("receipt"), dict),
                    "compile did not return the expected Catalog export receipt",
                )
                write_json(output / "receipt.json", compiled["receipt"])
                handle = commands.run(
                    [
                        "publish-artifact",
                        str(output / "artifact"),
                        "--expect-receipt",
                        str(output / "receipt.json"),
                        "--runtime-config",
                        str(runtime),
                    ],
                    "publish",
                )
                write_json(output / "handle.json", handle)
                oracle = capture_oracle(output)
            else:
                handle = json.loads((output / "handle.json").read_bytes())
                write_json(command_output / "handle.json", handle)
                oracle = json.loads((output / "oracle.json").read_bytes())
                # Oracle filenames stay relative to the retained producer root.
                oracle = {
                    **oracle,
                    "files": [
                        {**row, "saved": str(output / row["saved"])} for row in oracle["files"]
                    ],
                }
            result = check_catalog(commands, oracle)
            if mode == "produce":
                result["viewer"] = str(retain_viewer(runtime, handle))
            write_json(command_output / "result.json", result)
            return result
        except BaseException as error:
            write_json(
                command_output / "failure.json",
                {
                    "outcome": "failed",
                    "error": type(error).__name__,
                    "message": str(error),
                },
            )
            raise
        finally:
            set_progress(current_command=None)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=("produce", "check"))
    parser.add_argument("--lctx", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()

    def interrupted(signum: int, _frame: object) -> None:
        raise InterruptedError(f"pilot interrupted by signal {signum}")

    for signum in (signal.SIGTERM, signal.SIGHUP):
        signal.signal(signum, interrupted)
    try:
        print(json.dumps(execute(args.mode, args.lctx, args.output)))
    except (PilotError, OSError, ValueError, KeyboardInterrupt) as error:
        print(f"catalog pilot: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
