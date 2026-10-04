"""Qualify a compiled library over actual MCP stdio against independent source bytes.

Run with uv run --no-sync python scripts/qualify_serving.py --config ...
--generation ... --library fastmcp --profile catalog --receipt-dir ... .
Omit --generation to exercise selected-at-startup admission; --expected-generation
can independently pin the expected selection. --source-file overrides the acquired
FastMCP server.py; other libraries require it and an --anchors JSON file containing
[{"path": "demo.api", "qualname": "api"}, ...]. This does not compile, select,
prepare vectors, or execute the analyzed source. Raw protocol receipts contain no
configuration contents or subprocess environment.
"""

from __future__ import annotations

import argparse
import ast
import hashlib
import json
import os
import queue
import re
import subprocess
import threading
import time
from pathlib import Path
from typing import Any

from lctx_semantics import wire_resources, wire_tool, wire_tool_result, wire_tools


def require(condition: object, message: str) -> None:
    if not condition:
        raise AssertionError(message)


def code(schema: dict[str, Any], definition: str, label: str) -> int:
    """Read canonical numeric meanings from Rust's emitted schema, never a Python table."""
    description = schema["$defs"][definition]["description"]
    meanings = {
        name.replace("_", "").lower(): int(number)
        for number, name in (part.strip().split(" = ", 1) for part in description.split(";"))
    }
    normalized = label.replace("_", "").lower()
    require(normalized in meanings, f"missing canonical code {definition}.{label}")
    return meanings[normalized]


def check_channels(dto: dict[str, Any]) -> None:
    require(dto["channels"]["lexical"] is True, "lexical channel disabled")
    require(dto["channels"]["vector"] == {"status": "disabled"}, "vector channel drift")


def check_generation(dto: dict[str, Any], generation: list[int]) -> None:
    require(dto["generation"] == generation, "incorrect generation identity")


def check_original(body: bytes, expected: bytes) -> None:
    require(body == expected, "original evidence differs from independent source bytes")


class Stdio:
    """One sequential real subprocess; preserve received bytes before decoding assertions."""

    def __init__(self, args: argparse.Namespace) -> None:
        self.receipts = args.receipt_dir
        self.receipts.mkdir(parents=True, exist_ok=True)
        require(
            not (self.receipts / "receipt.json").exists(), "receipt directory already qualified"
        )
        command = [
            "uv",
            "run",
            "--no-sync",
            "python",
            "-m",
            "lctx_mcp",
            "--config",
            str(args.config.resolve()),
            "--library",
            args.library,
            "--embedder",
            "none",
        ]
        if args.generation:
            command += ["--generation", args.generation]
        self.process = subprocess.Popen(
            command,
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            env=os.environ.copy(),
        )
        assert self.process.stdin is not None
        assert self.process.stdout is not None
        assert self.process.stderr is not None
        self.stdin = self.process.stdin
        self.stdout = self.process.stdout
        self.error_stream = self.process.stderr
        self.lines: queue.Queue[bytes | None] = queue.Queue()
        self.stderr = bytearray()
        self.counter = 0
        self.samples: list[dict[str, Any]] = []
        self.calls: set[str] = set()
        self.generation: list[int] | None = (
            list(bytes.fromhex(args.expected_generation or args.generation))
            if args.expected_generation or args.generation
            else None
        )
        self.timeout = args.timeout
        self.threads = [
            threading.Thread(target=self._read, daemon=True),
            threading.Thread(target=self._read_errors, daemon=True),
        ]
        for thread in self.threads:
            thread.start()

    def _read(self) -> None:
        try:
            with (self.receipts / "protocol.jsonl").open("ab") as stream:
                for line in self.stdout:
                    preserved = re.sub(
                        rb"postgres(?:ql)?://[^\s'\"]+", b"[redacted database URL]", line
                    )
                    stream.write(preserved)
                    stream.flush()
                    self.lines.put(line)
        finally:
            self.lines.put(None)

    def _read_errors(self) -> None:
        for chunk in iter(lambda: self.error_stream.read(4096), b""):
            self.stderr.extend(chunk)

    def request(
        self, method: str, params: dict[str, Any], *, expanded: bool | None = None
    ) -> dict[str, Any]:
        self.counter += 1
        payload = {"jsonrpc": "2.0", "id": self.counter, "method": method, "params": params}
        self.stdin.write(json.dumps(payload, separators=(",", ":")).encode() + b"\n")
        self.stdin.flush()
        start = time.monotonic()
        while True:
            remaining = self.timeout - (time.monotonic() - start)
            require(remaining > 0, "stdio request deadline exceeded")
            raw = self.lines.get(timeout=remaining)
            if raw is None:
                raise AssertionError("stdio subprocess exited before response")
            require(
                b"postgres://" not in raw and b"postgresql://" not in raw,
                "credential URL in protocol response",
            )
            envelope = json.loads(raw)
            if envelope.get("id") == self.counter:
                break
            require("id" not in envelope, "unexpected server request or response ID")
        stem = f"{self.counter:04d}"
        (self.receipts / f"{stem}.request.json").write_text(json.dumps(payload, indent=2) + "\n")
        (self.receipts / f"{stem}.response.jsonl").write_bytes(raw)
        self.samples.append(
            {
                "id": self.counter,
                "method": method,
                "tool": params.get("name"),
                "bytes": len(raw),
                "elapsed_seconds": time.monotonic() - start,
            }
        )
        if expanded is not None:
            limits = json.loads(wire_tool("get_evidence"))["byte_limits"]
            require(
                len(raw) <= limits["expanded" if expanded else "default"],
                "final stdio envelope exceeds Rust byte bound",
            )
        return envelope

    def tool(
        self, name: str, arguments: dict[str, Any], *, failure: bool = False
    ) -> dict[str, Any]:
        self.calls.add(name)
        envelope = self.request(
            "tools/call",
            {"name": name, "arguments": arguments},
            expanded=arguments.get("page", {}).get("expanded", False),
        )
        result = envelope.get("result", {})
        if failure:
            require(
                result.get("isError") is True or "error" in envelope,
                "invalid request unexpectedly succeeded",
            )
            require("structuredContent" not in result, "failure carries successful data")
            return envelope
        require(
            "error" not in envelope and not result.get("isError"),
            f"{name} refused; inspect raw response {self.counter:04d}",
        )
        dto = result["structuredContent"]
        # Rust owns complete finite DTO validation and response byte policy.
        wire_tool_result(
            name,
            json.dumps(dto, ensure_ascii=False, separators=(",", ":")),
            arguments.get("page", {}).get("expanded", False),
        )
        if self.generation is None:
            self.generation = dto["generation"]
        check_generation(dto, self.generation)
        if "channels" in dto:
            check_channels(dto)
        return dto

    def close(self) -> dict[str, Any]:
        self.stdin.close()
        natural = True
        try:
            self.process.wait(timeout=10)
        except subprocess.TimeoutExpired:
            natural = False
            self.process.terminate()
            try:
                self.process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                self.process.kill()
                self.process.wait()
        for thread in self.threads:
            thread.join(timeout=2)
        # Stderr can carry connection errors. Retain bounded, sanitized diagnostics.
        diagnostic = self.stderr.decode(errors="replace")
        diagnostic = re.sub(r"postgres(?:ql)?://[^\s'\"]+", "[redacted database URL]", diagnostic)
        (self.receipts / "stderr.txt").write_text(diagnostic)
        (self.receipts / "samples.json").write_text(json.dumps(self.samples, indent=2) + "\n")
        return {"natural": natural, "returncode": self.process.returncode}


def source_functions(
    source: bytes, declarations: dict[str, list[Any]] | None = None
) -> dict[str, ast.FunctionDef | ast.AsyncFunctionDef]:
    functions: dict[str, ast.FunctionDef | ast.AsyncFunctionDef] = {}

    def visit(nodes: list[ast.stmt], prefix: list[str]) -> None:
        for node in nodes:
            if isinstance(node, ast.ClassDef):
                visit(node.body, prefix + [node.name])
            elif isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)):
                # Later implementation replaces preceding overload declarations.
                name = ".".join(prefix + [node.name])
                functions[name] = node
                if declarations is not None:
                    declarations.setdefault(name, []).append(node)

    visit(ast.parse(source).body, [])
    return functions


def check_signature(
    core: dict[str, Any], node: ast.FunctionDef | ast.AsyncFunctionDef, schema: dict[str, Any]
) -> list[tuple[dict[str, Any], ast.expr]]:
    positional = node.args.posonlyargs + node.args.args
    default_start = len(positional) - len(node.args.defaults)
    expected: list[tuple[ast.arg, str, ast.expr | None, bool]] = []
    for index, argument in enumerate(positional):
        default = node.args.defaults[index - default_start] if index >= default_start else None
        expected.append(
            (
                argument,
                "positional_only"
                if index < len(node.args.posonlyargs)
                else "positional_or_keyword",
                default,
                default is None,
            )
        )
    if node.args.vararg:
        expected.append((node.args.vararg, "var_positional", None, False))
    expected.extend(
        (arg, "keyword_only", default, default is None)
        for arg, default in zip(node.args.kwonlyargs, node.args.kw_defaults, strict=True)
    )
    if node.args.kwarg:
        expected.append((node.args.kwarg, "var_keyword", None, False))
    # Read actual source-level signature (including self), not adjusted effective parameters.
    signatures = [
        s
        for s in core["signatures"]
        if s["role"] == code(schema, "calls_SignatureRole", "source")
        and [p["name"] for p in s["parameters"]] == [e[0].arg for e in expected]
    ]
    require(signatures, f"no whole source signature for {node.name}")
    literals = {tuple(l["literal"]): l["value"] for l in core["literal_values"]}
    signatures = [
        s
        for s in signatures
        if s["complete"]
        and all(
            p["kind"] == code(schema, "calls_ParameterKind", kind) and p["required"] == required
            for p, (_, kind, _, required) in zip(s["parameters"], expected, strict=True)
        )
    ]
    require(
        signatures,
        f"no complete source signature with correct kinds/required flags for {node.name}",
    )
    source_defaults = []
    # Overloads can share names while changing default presence. Check the implementation
    # variant independently, without assuming each overload is that implementation.
    for signature in signatures[:1]:
        for parameter, (argument, kind, default, required) in zip(
            signature["parameters"], expected, strict=True
        ):
            require(
                parameter["kind"] == code(schema, "calls_ParameterKind", kind),
                f"wrong parameter kind for {argument.arg}",
            )
            require(parameter["required"] == required, f"wrong required flag for {argument.arg}")
            if argument.annotation is not None:
                require(parameter["types"], f"source annotation lost for {argument.arg}")
            if default is not None:
                require(
                    parameter["default"]["kind"] != "absent",
                    f"source-declared default erased for {argument.arg}",
                )
                if parameter["default"]["kind"] in {"expression", "factory"}:
                    source_defaults.append((parameter["default"], default))
                    continue
                try:
                    value = ast.literal_eval(default)
                except ValueError, TypeError:
                    require(
                        parameter["default"]["kind"] in {"expression", "factory", "unknown"},
                        f"expression default misrepresented for {argument.arg}",
                    )
                else:
                    if value is None or isinstance(value, (bool, str, int)):
                        require(
                            parameter["default"]["kind"] == "literal",
                            f"literal default lost for {argument.arg}",
                        )
                        literal = literals[tuple(parameter["default"]["literal"])]
                        wanted = (
                            {"kind": "none"}
                            if value is None
                            else {"kind": "bool", "value": value}
                            if isinstance(value, bool)
                            else {"kind": "integer", "decimal": str(value)}
                            if isinstance(value, int)
                            else {"kind": "string", "value": value}
                        )
                        require(literal == wanted, f"wrong literal default for {argument.arg}")
    return source_defaults


def read_original(client: Stdio, reference: dict[str, Any]) -> tuple[bytes, dict[str, Any]]:
    arguments: dict[str, Any] = {"source": reference, "page": {"expanded": True}}
    body = bytearray()
    original: dict[str, Any] | None = None
    seen: set[str] = set()
    while True:
        evidence = client.tool("get_evidence", arguments)["evidence"]
        if original is None:
            original = evidence["original"]
            require(original["source"] == reference, "incorrect original attribution")
            require(original["encoding"] == "raw_bytes", "original encoding drift")
        require(evidence["original"] == original, "original attribution changed across pages")
        page = evidence["body"]
        require(page["start"] == original["start"] + len(body), "original byte continuation gap")
        body.extend(bytes(page["bytes"]))
        require(
            page["end"] == original["start"] + len(body), "original byte continuation offset drift"
        )
        size = original["end"] - original["start"]
        require(page["omitted"] == size - len(body), "original byte omitted count")
        require(page["truncated"] == (len(body) < size), "original truncation drift")
        cursor = page.get("continuation")
        if cursor is None:
            require(len(body) == size, "original bytes silently truncated")
            break
        require(cursor not in seen, "repeated byte continuation")
        seen.add(cursor)
        arguments["page"]["cursor"] = cursor
    assert original is not None
    return bytes(body), original


def complete_find(client: Stdio, library: str) -> dict[str, Any]:
    first = client.tool(
        "find_operations", {"library": library, "page": {"size": 100, "expanded": True}}
    )
    counts = {}
    members = set()
    for group in ["supported", "unresolved", "conflicting"]:
        section = first[group]
        availability = section["availability"]
        expected = len(section["items"]) + section["omitted"]
        identifiers = set()
        cursors = set()
        while True:
            for candidate in section["items"]:
                identity = tuple(candidate["member"])
                require(
                    identity not in identifiers and identity not in members,
                    "complete discovery repeated or mixed a member",
                )
                identifiers.add(identity)
            require(len(identifiers) + section["omitted"] == expected, "discovery count changed")
            require(section["availability"] == availability, "discovery coverage changed")
            cursor = section.get("continuation")
            if cursor is None:
                require(
                    section["omitted"] == 0 and not section["truncated"],
                    "complete discovery silently truncated",
                )
                break
            require(cursor not in cursors, "discovery continuation loop")
            cursors.add(cursor)
            dto = client.tool(
                "find_operations",
                {"library": library, "page": {"size": 100, "cursor": cursor, "expanded": True}},
            )
            require(dto["extent"] == first["extent"], "discovery extent changed")
            section = dto[group]
        members.update(identifiers)
        counts[group] = {"count": len(identifiers), "availability": availability}
    if first["extent"]["extent"] == "complete_domain":
        require(len(members) == first["extent"]["total"], "complete domain total disagrees")
    return {"groups": counts, "extent": first["extent"], "members": members}


def qualify(
    client: Stdio, args: argparse.Namespace, source: bytes, anchors: list[dict[str, str]]
) -> dict[str, Any]:
    initialized = client.request(
        "initialize",
        {
            "protocolVersion": "2025-11-25",
            "capabilities": {},
            "clientInfo": {"name": "lctx-serving-qualification", "version": "1"},
        },
    )
    require("result" in initialized, "MCP initialization failed")
    client.stdin.write(b'{"jsonrpc":"2.0","method":"notifications/initialized"}\n')
    client.stdin.flush()
    declarations = json.loads(wire_tools())
    (args.receipt_dir / "rust-wire-inventory.json").write_text(
        json.dumps({"tools": declarations, "resources": json.loads(wire_resources())}, indent=2)
        + "\n"
    )
    tools = client.request("tools/list", {})["result"]["tools"]
    require(
        len(declarations) == 10 and {t["name"] for t in tools} == {d["name"] for d in declarations},
        "tool inventory drift",
    )
    for declaration in declarations:
        actual = next(t for t in tools if t["name"] == declaration["name"])
        require(actual["inputSchema"] == declaration["request_schema"], "tool input schema drift")
        require(
            actual["outputSchema"] == declaration["response_schema"], "tool output schema drift"
        )
        require(actual["description"] == declaration["description"], "tool description drift")
    templates = client.request("resources/templates/list", {})["result"]["resourceTemplates"]
    require(
        [
            {
                "uri_template": t["uriTemplate"],
                "name": t["name"],
                "mime_type": t["mimeType"],
                "description": t["description"],
            }
            for t in templates
        ]
        == json.loads(wire_resources()),
        "resource inventory drift",
    )
    common = {"library": args.library}
    found = client.tool("find_operations", {**common, "page": {"size": 1}})
    groups = ["supported", "unresolved", "conflicting"]
    require(all(g in found for g in groups), "discovery groups lost")
    continuations = 0
    for group in groups:
        first = found[group]
        cursor = first.get("continuation")
        if cursor:
            follow = client.tool(
                "find_operations", {**common, "page": {"size": 1, "cursor": cursor}}
            )
            require(
                not {tuple(c["member"]) for c in first["items"]}
                & {tuple(c["member"]) for c in follow[group]["items"]},
                "continuation repeats row",
            )
            client.tool(
                "find_operations",
                {"library": "foreign-qualification-library", "page": {"size": 1, "cursor": cursor}},
                failure=True,
            )
            continuations += 1
    require(continuations > 0, "fixture did not exercise operation continuation")
    discovery = complete_find(client, args.library)
    discovered_members = discovery.pop("members")
    client.tool("browse_library", common)
    client.tool("browse_library", {**common, "view": "modules"})
    declarations_by_name: dict[str, list[Any]] = {}
    functions = source_functions(source, declarations_by_name)
    signature_observations = []
    packets = []
    evidence_artifacts: dict[tuple[int, ...], dict[str, Any]] = {}
    native_availability = []
    for anchor in anchors:
        path = anchor["path"]
        node = functions[anchor["qualname"]]
        selector = {"kind": "public_path", "path": path.split(".")}
        operation = client.tool(
            "get_operation",
            {
                **common,
                "operation": selector,
                "sections": ["behavior", "scenarios", "briefs"],
                "page": {"expanded": True},
            },
        )["operation"]
        require(operation["resolution"] == "unique", f"{path} did not resolve uniquely")
        packet = operation["packet"]
        require(
            tuple(packet["core"]["member"]) in discovered_members,
            "exact source anchor missing from complete discovery",
        )
        require(packet["core"]["access"]["path"] == path.split("."), "public access path drift")
        defaults = check_signature(
            packet["core"], node, json.loads(wire_tool("get_operation"))["output_schema"]
        )
        declared = declarations_by_name[anchor["qualname"]]
        observations = []
        for declaration in declared:
            names = [a.arg for a in declaration.args.posonlyargs + declaration.args.args]
            if declaration.args.vararg:
                names.append(declaration.args.vararg.arg)
            names.extend(a.arg for a in declaration.args.kwonlyargs)
            if declaration.args.kwarg:
                names.append(declaration.args.kwarg.arg)
            served = [
                s
                for s in packet["core"]["signatures"]
                if [p["name"] for p in s["parameters"]] == names
            ]
            if served:
                require(any(s["complete"] for s in served), "exposed overload signature incomplete")
            observations.append(
                {
                    "source_line": declaration.lineno,
                    "parameters": names,
                    "served_variants_with_same_names": len(served),
                }
            )
        signature_observations.append(
            {
                "path": path,
                "source_declarations": observations,
                "whole_implementation_signature": "passed",
                "declared_defaults": [
                    {"parameter": p["name"], "served": p["default"]}
                    for s in packet["core"]["signatures"]
                    for p in s["parameters"]
                    if not p["required"]
                ],
            }
        )
        for default, expected in defaults:
            body, original = read_original(
                client, {"kind": "occurrence", "occurrence": default["expression"]}
            )
            expected_source = ast.get_source_segment(source.decode(), expected)
            require(expected_source is not None, "independent default source span missing")
            check_original(body, expected_source.encode())
            require(
                source[original["start"] : original["end"]] == body,
                "default source range does not match independent artifact",
            )
            evidence_artifacts.setdefault(tuple(original["artifact"]), original)
        packets.append(packet)
        ranked = client.tool(
            "search_operations", {**common, "query": path, "page": {"expanded": True}}
        )
        require(
            any(c["member"] == packet["core"]["member"] for c in ranked["results"]["items"]),
            f"lexical operation search lost {path}",
        )
        evidence = client.tool(
            "search_evidence",
            {
                **common,
                "query": node.name,
                "families": json.loads(wire_tool("search_evidence"))["parameters"]["$defs"][
                    "retrieval_Family"
                ]["enum"],
                "page": {"expanded": True},
            },
        )
        for hit in evidence["results"]["items"]:
            for original in hit["originals"]:
                key = tuple(original["artifact"])
                evidence_artifacts.setdefault(key, original)
        # Scalar requests qualify only the explicitly named finite model. A defaulted
        # formal can retain DefaultStabilityUnknown; unknown paths are never proof.
        parameter_name = (
            "uri"
            if anchor["qualname"] == "FastMCP.resource"
            else "name_or_fn"
            if anchor["qualname"] == "FastMCP.tool"
            else next((a.arg for a in node.args.args if a.arg not in {"self", "cls"}), None)
        )
        if parameter_name:
            # Scalar inputs address source formals. Effective/synthesized roles can
            # expose NativeSlot identities, which are not source-body inputs.
            schema = json.loads(wire_tool("get_operation"))["output_schema"]
            formals = {
                (tuple(s["analysis"]), tuple(formal))
                for s in packet["core"]["signatures"]
                if s["role"] == code(schema, "calls_SignatureRole", "source") and s["complete"]
                for p in s["parameters"]
                if p["name"] == parameter_name
                for formal in p["formals"]
            }
            require(
                len(formals) == 1,
                f"no unique source formal bridge for scalar exercise {path}.{parameter_name}",
            )
            analysis, source_formal = formals.pop()
            formal = (list(analysis), list(source_formal))
            values = (
                [{"kind": "none"}]
                if parameter_name == "name_or_fn"
                else [
                    {"kind": "string", "value": "qualification://source"},
                    {"kind": "none"},
                    {"kind": "bool", "value": True},
                    {"kind": "integer", "decimal": "1"},
                ]
            )
            for value in values:
                exact = client.tool(
                    "inspect_value_paths",
                    {
                        "member": packet["core"]["member"],
                        "analysis": formal[0],
                        "inputs": [{"formal": formal[1], "value": value}],
                        "assumptions": {"builtin_namespace": "standard_cpython"},
                        "page": {"expanded": True},
                    },
                )
                if args.profile == "catalog":
                    require(
                        exact["paths"]["availability"] == {"status": "not_requested"},
                        "catalog scalar request lost NotRequested",
                    )
                else:
                    require(
                        exact["paths"]["availability"]["status"] != "not_requested",
                        "behavioral scalar request lost requested Flow coverage",
                    )
                for assessment in exact["paths"]["items"]:
                    if assessment["exact"] == "unknown":
                        require(
                            assessment["reason"] is not None
                            or assessment["unexamined"] > 0
                            or exact["paths"]["availability"]["status"] != "available",
                            "native uncertainty lacks declared cause or unexamined evidence",
                        )
                    if assessment["exact"] in {
                        "refuted_path_under_model",
                        "compatible_under_may_model",
                    }:
                        require(
                            assessment["proof"] and assessment["restricted_result"] is not None,
                            "native finite result lost original proof or result identity",
                        )
                native_availability.append(
                    {
                        "path": path,
                        "parameter": parameter_name,
                        "formal": formal[1],
                        "input": value,
                        "analysis": formal[0],
                        "assumptions": {"builtin_namespace": "standard_cpython"},
                        "availability": exact["paths"]["availability"],
                        "paths": exact["paths"]["items"],
                        "claim_scope": "finite paths under the declared model and assumptions",
                    }
                )
    comparison = client.tool(
        "compare_operations",
        {
            **common,
            "operations": [{"kind": "member", "member": p["core"]["member"]} for p in packets[:5]],
        },
    )
    require(
        len(comparison["operations"]) == len(packets[:5]), "comparison lost requested operations"
    )
    for entry, packet in zip(comparison["operations"], packets[:5], strict=True):
        require(
            not entry["ambiguous"]
            and any(c["member"] == packet["core"]["member"] for c in entry["candidates"]),
            "comparison changed source operation identity",
        )
    # Winning original evidence must resolve to the acquired source, rather than a helper's
    # expected bytes. Candidate artifacts are discovered through actual lexical evidence.
    matched = None
    for artifact, original in evidence_artifacts.items():
        body, attribution = read_original(client, {"kind": "artifact", "artifact": list(artifact)})
        if body == source:
            check_original(body, source)
            require(original["digest"] == attribution["digest"], "winning original digest drift")
            require(
                0 <= original["start"] <= original["end"] <= len(source), "original range drift"
            )
            matched = attribution
            (args.receipt_dir / "original-source.bin").write_bytes(body)
            break
    require(
        matched is not None, "ranked original evidence did not resolve to independent source file"
    )
    capabilities = client.tool(
        "search_capabilities",
        {**common, "query": anchors[0]["qualname"], "page": {"expanded": True}},
    )
    items = capabilities["results"]["items"]
    if items:
        identity = items[0]["capability"]
        capability = client.tool(
            "get_capability", {"capability": identity, "page": {"expanded": True}}
        )["capability"]
        resource = client.request(
            "resources/read", {"uri": "lctx://capability/" + bytes(identity).hex()}, expanded=True
        )
        require("error" not in resource, "available capability resource refused")
        contents = resource["result"]["contents"]
        require(
            len(contents) == 1 and contents[0]["mimeType"] == "text/markdown", "resource MIME drift"
        )
        require(
            contents[0]["text"].startswith(capability["rendered"]), "capability rendering drift"
        )
        capability_status = {"status": "available", "capability": identity}
    else:
        # Absence is scoped to the actual search extent; it is never fabricated as support.
        missing = client.tool("get_capability", {"capability": [0] * 16}, failure=True)
        public_failure = missing["result"]["_meta"]["lctx_failure"]
        require(
            public_failure["kind"] in {"unavailable", "incompatible"},
            "optional capability lookup failed for an unrelated reason",
        )
        resource = client.request(
            "resources/read", {"uri": "lctx://capability/" + "00" * 16}, expanded=False
        )
        require("error" in resource, "unknown capability resource unexpectedly exists")
        resource_failure = resource["error"]["data"]
        require(
            resource_failure["kind"] == public_failure["kind"],
            "optional resource and capability failure meaning differ",
        )
        capability_status = {
            "status": "no_search_results",
            "availability": capabilities["results"]["availability"],
            "extent": capabilities["extent"],
            "get_and_resource": "refused",
            "failure": public_failure,
        }
    require(client.calls == {d["name"] for d in declarations}, "not all ten tools invoked")
    assert client.generation is not None
    return {
        "generation": bytes(client.generation).hex(),
        "profile": args.profile,
        "protocol_version": initialized["result"]["protocolVersion"],
        "startup": "explicit" if args.generation else "selected",
        "library": args.library,
        "source_sha256": hashlib.sha256(source).hexdigest(),
        "source_artifact": matched,
        "anchors": anchors,
        "native": native_availability,
        "capability": capability_status,
        "signatures": signature_observations,
        "discovery": discovery,
        "continuation_controls": continuations,
        "tools": sorted(client.calls),
        "vectors": "disabled",
        "live_embedding": "not_run",
        "retrieval_quality": "not_run",
    }


def main(argv: list[str] | None = None) -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--config", type=Path, required=True)
    parser.add_argument("--generation")
    parser.add_argument("--expected-generation")
    parser.add_argument("--library", required=True)
    parser.add_argument("--profile", choices=["catalog", "behavioral"], required=True)
    parser.add_argument("--receipt-dir", type=Path, required=True)
    parser.add_argument("--source-file", type=Path)
    parser.add_argument("--anchors", type=Path)
    parser.add_argument("--timeout", type=float, default=180)
    args = parser.parse_args(argv)
    require(
        args.library == "fastmcp" or args.anchors is not None,
        "other libraries require independent --anchors",
    )
    for generation in [args.generation, args.expected_generation]:
        if generation:
            require(
                len(generation) == 32 and len(bytes.fromhex(generation)) == 16,
                "generation must be 32 hexadecimal characters",
            )
    if args.source_file is None:
        require(args.library == "fastmcp", "other libraries require --source-file and --anchors")
        args.source_file = Path(
            "build/envs/fastmcp/lib/python3.14/site-packages/fastmcp/server/server.py"
        )
    anchors = (
        json.loads(args.anchors.read_text())
        if args.anchors
        else [
            {"path": f"fastmcp.FastMCP.{name}", "qualname": f"FastMCP.{name}"}
            for name in ["tool", "resource", "mount"]
        ]
    )
    require(anchors and len(anchors) <= 5, "one to five source-grounded anchors required")
    source = args.source_file.read_bytes()
    client = Stdio(args)
    receipt = {"outcome": "failed", "profile": args.profile, "library": args.library}
    failure = None
    try:
        receipt = {"outcome": "passed", **qualify(client, args, source, anchors)}
    except Exception as error:
        receipt["failure"] = type(error).__name__ + ": " + str(error)
        failure = error
    finally:
        shutdown = client.close()
        receipt["shutdown"] = shutdown
        if not shutdown["natural"] or shutdown["returncode"] != 0:
            receipt["outcome"] = "failed"
            receipt["shutdown_failure"] = "stdio did not exit naturally and successfully after EOF"
            if failure is None:
                failure = AssertionError(receipt["shutdown_failure"])
        if client.generation is not None:
            receipt["generation"] = bytes(client.generation).hex()
        (args.receipt_dir / "receipt.json").write_text(json.dumps(receipt, indent=2) + "\n")
    if failure is not None:
        raise failure
    print(json.dumps(receipt))


if __name__ == "__main__":
    main()
