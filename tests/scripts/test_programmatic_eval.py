"""Private finite/numeric controls and current Rust/MCP renderer controls; no inference or protected data."""

from __future__ import annotations

import copy
import importlib.util
import json
import os
import subprocess
from pathlib import Path

import pytest
from hypothesis import given, settings
from hypothesis import strategies as st

from programmatic_eval import (
    Worker,
    WorkerError,
    content_digest,
    diagnose_stages,
    frozen_judgments,
    grounded_feedback,
    load_cases,
    load_observations,
    minimize,
    minimize_generated,
    minimize_mcp,
    mutation_outcome,
    packet_ceiling,
    prepare_comparison,
    renderer_inventory_ceiling,
    summary,
)

ROOT = Path(__file__).resolve().parents[2]
BINARY = Path(os.environ.get("LCTX_EVAL_WORKER", ROOT / "target/release/lctx-eval"))


@pytest.fixture(scope="module")
def worker():
    if not BINARY.is_file():
        pytest.fail("missing compiled lctx-eval; run cargo build --release -p lctx-eval")
    with Worker(BINARY) as process:
        yield process


def population():
    return list(load_cases(ROOT / "eval/programmatic/finite-cases.jsonl"))


def public_packet(case):
    return json.loads(case["observation"]["segments"][0])


def replace_packet(case, packet):
    case["observation"]["segments"][0] = json.dumps(packet, ensure_ascii=False)


def test_independent_expected_population_and_explicit_denominators(worker):
    expected = json.loads((ROOT / "eval/programmatic/finite-expectations.json").read_text())
    cases = population()
    rows = list(worker.judge(cases))
    assert {row["task_id"]: row["epistemic"] for row in rows} == expected
    report = summary(rows)
    assert report["total_tasks"] == len(cases)
    assert report["scorable_tasks"] < len(cases)
    assert "explicit-not-applicable" in report["unscored_tasks"]
    assert "empty-worlds" in report["unscored_tasks"]
    assert worker.process.poll() is None


def test_worker_generated_schema_and_private_projection_boundary(worker):
    assert worker.schema["case"]["$defs"]["PublicRequest"]["additionalProperties"] is False
    case = population()[0]
    case["task"]["request"]["expected_anchor"] = "oracle-leak"
    with pytest.raises(WorkerError, match="unknown field"):
        list(worker.judge([case]))
    assert worker.process.poll() is None


def test_schema_valid_mutants_are_judged_not_just_admitted(worker):
    rows = {row["task_id"]: row for row in worker.judge(population())}
    for name in (
        "foreign-variant",
        "foreign-release",
        "wrong-anchor",
        "drop-setup",
        "qualification-id-only",
        "same-substring-foreign-association",
        "correct-ids-hidden-meaning",
        "mislabel-expected-failure",
    ):
        assert mutation_outcome(rows["compatible"], rows[name]) == "caught"
    assert (
        mutation_outcome(rows["compatible"], rows["positive-alternate"], equivalent=True)
        == "equivalent"
    )
    assert mutation_outcome(rows["compatible"], rows["empty-positive"]) == "invalid"
    assert mutation_outcome(rows["compatible"], rows["unsupported"]) == "inconclusive"


def test_shrink_preserves_supported_oracle_and_failure_class(worker):
    case = next(case for case in population() if case["task"]["id"] == "foreign-variant")
    packet = public_packet(case)
    extra = copy.deepcopy(packet["groups"][0])
    extra["evidence"][0].update(role="irrelevant", anchor="unrelated")
    packet["groups"].append(extra)
    replace_packet(case, packet)

    def preserves(trial):
        row = list(worker.judge([trial]))[0]
        return (
            row["epistemic"] == "insufficient"
            and row["reason"] == "individually readable predicates have incompatible contexts"
        )

    minimized = minimize(case, preserves)
    assert minimized["task"] == case["task"]
    assert len(public_packet(minimized)["groups"]) < len(public_packet(case)["groups"])
    assert preserves(minimized)


def test_actual_byte_ceiling_and_missing_inventory(worker):
    case = population()[0]
    assert packet_ceiling(worker, case["task"], [case["observation"]])["status"] == "feasible"
    task = copy.deepcopy(case["task"])
    task["envelope"]["max_bytes"] = 1
    assert (
        packet_ceiling(worker, task, [case["observation"]], inventory_complete=True)["status"]
        == "budget_infeasible"
    )
    assert (
        packet_ceiling(worker, task, [], inventory_complete=True)["status"]
        == "inventory_infeasible"
    )
    assert packet_ceiling(worker, task, [case["observation"]])["status"] == "inconclusive"
    assert packet_ceiling(worker, task, [])["status"] == "inconclusive"
    insufficient = copy.deepcopy(case["observation"])
    packet = json.loads(insufficient["segments"][0])
    packet["groups"].pop()
    insufficient["segments"][0] = json.dumps(packet)
    result = packet_ceiling(
        worker, task, [case["observation"], insufficient], inventory_complete=True
    )
    assert result["status"] == "budget_infeasible"
    assert result["budget_relaxation_diagnostic_only"]


def test_protected_input_location_refused(tmp_path):
    path = tmp_path / "heldout.jsonl"
    path.write_text("{}\n")
    with pytest.raises(WorkerError, match="private"):
        list(load_cases(path))
    capture = tmp_path / "heldout.json"
    capture.write_text("{}")
    with pytest.raises(WorkerError, match="private"):
        load_observations(capture)


def test_generated_programs_are_independent_supported_twins():
    # Only our newly generated, bounded programs execute. Repository fixtures are never run.
    for name, expected in (("none-override", "0"), ("truthiness-override", "10")):
        path = ROOT / "eval/programmatic" / (name + ".py")
        result = subprocess.run(
            [
                os.sys.executable,
                "-I",
                "-c",
                "import runpy; print(runpy.run_path(" + repr(str(path)) + ')["connect"](0))',
            ],
            check=True,
            capture_output=True,
            text=True,
            timeout=5,
        )
        assert result.stdout.strip() == expected


def test_generated_population_is_reproducible_and_family_split_is_fixed():
    path = ROOT / "eval/programmatic/generate.py"
    spec = importlib.util.spec_from_file_location("private_eval_generator", path)
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    generated = [case for _, case, _ in module.populations()]
    assert generated == population()
    assert {case["task"]["split"] for case in generated} == {"development"}


def test_freeze_binds_actual_tasks_and_blocks_changed_yardstick(worker):
    realization = {
        "observation_realization": "finite-render-1",
        "source": "hand-source-v1",
        "native": "not_run",
        "encoder": "not_requested",
        "scorer": "finite",
        "settings": {},
    }
    meanings = {
        "judgment": "finite-v1",
        "observation": worker.schema["case"]["$defs"]["Observation"],
        "metrics": "epistemic-counts",
        "numeric_precision_ties": "not_requested",
    }
    cases = population()
    frozen = prepare_comparison(worker, cases, "1", realization, realization, [], meanings)
    assert len(
        frozen_judgments(worker, frozen, cases, frozen["experiment"], lane="baseline")
    ) == len(cases)
    changed = copy.deepcopy(cases)
    changed[0]["task"]["predicates"][0]["accepted_text"] = ["changed expected meaning"]
    with pytest.raises(WorkerError, match="inventory differs"):
        frozen_judgments(worker, frozen, changed, frozen["experiment"], lane="baseline")
    experiment = copy.deepcopy(frozen["experiment"])
    experiment["revision"] = "2"
    with pytest.raises(WorkerError, match="incompatible comparison"):
        frozen_judgments(worker, frozen, cases, experiment, lane="candidate")


def test_timeout_cancels_the_campaign_process(tmp_path):
    executable = tmp_path / "hung-worker"
    executable.write_text(
        "#!"
        + os.sys.executable
        + "\nimport json, sys, time\nfor line in sys.stdin:\n if json.loads(line).get('operation') == 'schema':\n  print(json.dumps({'status':'completed','result':{'protocol_version':3,'case':{'$schema':'https://json-schema.org/draft/2020-12/schema'}}}), flush=True)\n else:\n  time.sleep(30)\n"
    )
    executable.chmod(0o700)
    with Worker(executable, timeout=0.2) as process:
        with pytest.raises(WorkerError, match="timeout"):
            process.request({"operation": "judge", "cases": [population()[0]]})
        assert process.process.poll() is not None


@settings(max_examples=40, derandomize=True, database=None, deadline=None)
@given(
    signature=st.sets(st.sampled_from(["A", "B", "C"])),
    default=st.sets(st.sampled_from(["A", "B", "C"])),
    setup=st.sets(st.sampled_from(["A", "B", "C"])),
)
def test_generated_context_population_uses_one_worker(worker, signature, default, setup):
    case = population()[0]
    packet = public_packet(case)
    prototype = packet["groups"]
    packet["groups"] = []
    for item, variants in zip(prototype, (signature, default, setup), strict=True):
        for variant in sorted(variants):
            group = copy.deepcopy(item)
            group["context"]["variant"] = variant
            packet["groups"].append(group)
    replace_packet(case, packet)
    judgment = list(worker.judge([case]))[0]
    # Independent finite set intersection, not the production/reference join kernel.
    assert (judgment["epistemic"] == "sufficient") == bool(signature & default & setup)


def test_generator_shrinks_source_task_context_together(worker):
    def build(factors):
        case = next(case for case in population() if case["task"]["id"] == "foreign-variant")
        source = "def connect(timeout=None): return timeout\n" + "".join(factors["source_noise"])
        packet = public_packet(case)
        for group in packet["groups"]:
            group["context"].update(factors["context_noise"])
        replace_packet(case, packet)
        case["task"]["request"]["context"].update(factors["context_noise"])
        case["task"]["oracle"]["input_digest"] = content_digest(source)
        return {"source": source, "case": case}

    def preserves(generated):
        row = list(worker.judge([generated["case"]]))[0]
        return row["reason"] == "individually readable predicates have incompatible contexts"

    factors = {
        "source_noise": ["# unrelated helper\n", "unused = 42\n"],
        "context_noise": {"deployment_note": "unused"},
    }
    minimized = minimize_generated(factors, build, preserves)
    assert minimized["factors"] == {"source_noise": [], "context_noise": {}}
    assert minimized["case"]["source"] == "def connect(timeout=None): return timeout\n"


def test_generated_wire_schema_is_current(worker):
    assert worker.schema == json.loads((ROOT / "eval/programmatic/wire-schema.json").read_text())


def test_python_grounded_feedback_can_reach_both_owners(worker):
    case = population()[0]
    disposition = grounded_feedback(
        worker,
        case,
        "1",
        "independent source and exact retained observation",
        ["system", "evaluator", "usability"],
        "add precedence task and clearer navigation",
        [case["task"]["id"]],
        "2",
    )
    assert len(disposition["routes"]) == 3
    assert disposition["new_comparison_required"]
    assert disposition["retain_old_semantic_result"]
    with pytest.raises(WorkerError, match="new revision"):
        grounded_feedback(
            worker,
            case,
            "1",
            "independent source",
            ["evaluator"],
            "new expectation",
            [case["task"]["id"]],
        )


def comparison(worker):
    realization = {
        "observation_realization": "finite-render-1",
        "source": "hand-source-v1",
        "native": "not_run",
        "encoder": "not_requested",
        "scorer": "finite",
        "settings": {},
    }
    meanings = {"metrics": "epistemic-counts", "numeric_precision_ties": "not_requested"}
    cases = population()
    frozen = prepare_comparison(worker, cases, "2", realization, realization, [], meanings)
    return cases, frozen


def test_no_overlay_can_relabel_same_actual_foreign_substring(worker):
    case = next(
        case for case in population() if case["task"]["id"] == "same-substring-foreign-association"
    )
    original_bytes = case["observation"]["segments"][0]
    case["observation"]["spans"] = [
        {
            "text": "default timeout is 10",
            "context": {"release": "1", "variant": "A"},
            "anchor": "default",
            "role": "default",
        }
    ]
    with pytest.raises(WorkerError, match="unknown field"):
        list(worker.judge([case]))
    assert case["observation"]["segments"][0] == original_bytes
    del case["observation"]["spans"]
    assert list(worker.judge([case]))[0]["epistemic"] == "insufficient"


def test_retained_qualification_id_cannot_supply_omitted_condition(worker):
    case = population()[0]
    requirement = {
        "id": "setup-condition",
        "accepted_text": ["Applies only with installed transport"],
    }
    case["task"]["predicates"][1]["qualifications"].append(requirement)
    packet = public_packet(case)
    condition = {"id": "setup-condition", "text": "Applies only with installed transport"}
    packet["groups"][1]["evidence"][0]["qualifications"].append(condition)
    replace_packet(case, packet)
    assert list(worker.judge([case]))[0]["epistemic"] == "sufficient"
    condition["text"] = ""
    replace_packet(case, packet)
    assert list(worker.judge([case]))[0]["epistemic"] == "insufficient"


def test_frozen_mode_and_explicit_realization_lane_refuse_mismatch(worker):
    cases, frozen = comparison(worker)
    changed = copy.deepcopy(cases)
    changed[0]["mode"] = "expandable"
    with pytest.raises(WorkerError, match="mode inventory"):
        frozen_judgments(worker, frozen, changed, frozen["experiment"], lane="baseline")
    changed = copy.deepcopy(cases)
    changed[0]["observation"]["realization"] = "foreign"
    with pytest.raises(WorkerError, match="realization differs"):
        frozen_judgments(worker, frozen, changed, frozen["experiment"], lane="candidate")
    with pytest.raises(WorkerError, match="explicit baseline"):
        frozen_judgments(worker, frozen, cases, frozen["experiment"], lane="unspecified")
    # A forged overlay handle cannot hide a different handle in exact emitted bytes.
    packet = public_packet(cases[0])
    packet["realization"] = "foreign"
    replace_packet(cases[0], packet)
    assert list(worker.judge([cases[0]]))[0]["epistemic"] == "inconclusive"


@pytest.mark.parametrize("change", ["kernel", "schema"])
def test_changed_actual_worker_executable_cannot_use_old_freeze(worker, tmp_path, change):
    cases, frozen = comparison(worker)
    schema = copy.deepcopy(worker.schema)
    if change == "kernel":
        schema["kernel_source_revision"] = "different-executable-source"
    else:
        schema["finite_packet"]["description"] = "different-observation-contract"
    executable = tmp_path / "different-worker"
    executable.write_text(
        "#!"
        + os.sys.executable
        + "\nimport json, sys\nschema = json.loads("
        + repr(json.dumps(schema))
        + ")\nfor line in sys.stdin:\n print(json.dumps({'status':'completed','result':schema}), flush=True)\n"
    )
    executable.chmod(0o700)
    with Worker(executable) as different:
        with pytest.raises(WorkerError, match="actual running kernel/schema"):
            frozen_judgments(different, frozen, cases, frozen["experiment"], lane="baseline")


def renderer_source_case():
    path = ROOT / "eval/programmatic/renderer_cases.py"
    spec = importlib.util.spec_from_file_location("private_renderer_cases", path)
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module.source_case()


def test_numeric_stored4096_projection1024_reference_needs_no_encoder(worker):
    from programmatic_eval import stored_vector_reference

    query = [0.0] * 4096
    query[0] = query[1024] = 1.0
    wrong = query.copy()
    wrong[1024] = -1.0
    result = stored_vector_reference(
        worker,
        {
            "policy": {
                "full_dimensions": 4096,
                "projection_dimensions": 1024,
                "block_rows": 1,
                "k": 1,
            },
            "query": query,
            "vectors": [
                {"id": "z-best", "values": query, "eligible": True},
                {"id": "a-tie", "values": wrong, "eligible": True},
            ],
            "nominated_ids": ["a-tie"],
        },
    )
    assert result["full"][0]["id"] == "z-best"
    assert result["projected"][0]["id"] == "a-tie"
    assert result["missing_from_union"] == ["z-best"]
    assert "no native ANN" in result["nomination_basis"]


def test_actual_rust_renderer_source_observation_and_missing_interpretation(worker):
    from programmatic_eval import capture_renderer

    task, response = renderer_source_case()
    observation = capture_renderer(task, response)
    assert "structuredContent" in json.loads(observation["segments"][0])
    assert "groups" not in json.loads(observation["segments"][0])
    case = {"task": task, "observation": observation, "mode": "immediate"}
    assert list(worker.judge([case]))[0]["epistemic"] == "sufficient"
    # An opaque public qualification ID cannot supply a missing readable condition.
    case["task"]["predicates"][0]["qualifications"] = [
        {"id": "setup", "accepted_text": ["requires installed transport"]}
    ]
    assert list(worker.judge([case]))[0]["epistemic"] == "insufficient"


@pytest.mark.parametrize(
    "mutation",
    [
        "absent_map",
        "empty_map",
        "missing_body",
        "missing_release_dependency",
        "missing_interpretation_dependency",
        "null_original",
        "wrong_role",
    ],
)
def test_actual_rust_renderer_schema_valid_map_mutants_cannot_authorize_truth(worker, mutation):
    from programmatic_eval import capture_renderer

    task, response = renderer_source_case()
    observation = capture_renderer(task, response)
    case = {"task": task, "observation": observation, "mode": "immediate"}
    assert list(worker.judge([case]))[0]["epistemic"] == "sufficient"
    public = json.loads(observation["segments"][0])
    content = copy.deepcopy(public["content"])
    evidence = copy.deepcopy(public["structuredContent"]["evidence"])
    metadata = public["structuredContent"]["delivery"]
    if mutation == "absent_map":
        del public["structuredContent"]["delivery"]
    elif mutation == "empty_map":
        metadata["fields"] = []
    elif mutation == "missing_body":
        metadata["fields"].pop(1)
    elif mutation.startswith("missing_"):
        dependency = "/structuredContent/evidence/" + (
            "release" if mutation == "missing_release_dependency" else "interpretation"
        )
        metadata["fields"][1]["dependencies"].remove(dependency)
    elif mutation == "null_original":
        metadata["fields"][1]["original"] = None
    elif mutation == "wrong_role":
        metadata["fields"][1]["role"] = "synthetic"
    assert public["content"] == content
    assert public["structuredContent"]["evidence"] == evidence
    observation["segments"][0] = json.dumps(public, ensure_ascii=False)
    row = list(worker.judge([case]))[0]
    assert row["epistemic"] == "inconclusive"
    assert not row["scorable"]


@pytest.fixture
def anyio_backend():
    return "asyncio"


@pytest.mark.anyio
async def test_actual_mcp_capture_uses_public_projection_and_final_sdk_object(worker):
    from fastmcp import Client, FastMCP

    from lctx_mcp.wire import register
    from programmatic_eval import capture_public_journey

    task, response = renderer_source_case()
    sent = []

    class ControlledDtoRenderer:
        async def execute(self, tool, arguments):
            sent.append({"tool": tool, "arguments": arguments})
            return json.dumps(response)

    server = FastMCP("private renderer control", dereference_schemas=False)
    register(server, ControlledDtoRenderer())
    async with Client(server) as client:
        observation = await capture_public_journey(worker, client, task, lane="renderer")
    assert sent == [task["public_call"]]
    assert observation["capture"]["lane"] == "renderer"
    assert (
        list(worker.judge([{"task": task, "observation": observation, "mode": "immediate"}]))[0][
            "epistemic"
        ]
        == "sufficient"
    )
    assert (
        json.loads(observation["segments"][0])["structuredContent"]["evidence"]["body"]["bytes"]
        == response["evidence"]["body"]["bytes"]
    )


def test_actual_public_projection_rejects_private_fields_before_effects():
    from lctx_semantics import NativeFailure

    from programmatic_eval import _public_projection

    task, _ = renderer_source_case()
    task["public_call"]["arguments"]["expected_anchor"] = "private-leak"
    with pytest.raises(NativeFailure):
        _public_projection(task)


def test_cursor_replays_actual_public_origin_without_private_expectations():
    from programmatic_eval import _continuation_projection

    origin = {
        "tool": "search_evidence",
        "arguments": {
            "library": "mini",
            "query": "connect",
            "families": ["code"],
            "page": {
                "size": 3,
                "expanded": False,
                "evidence_demand": {"facets": ["defaults"], "maximum_followups": 1},
            },
        },
    }
    visible = {
        "tool": "search_evidence",
        "arguments": {"page": {"cursor": "public-cursor", "expanded": False}},
    }
    replay = _continuation_projection(visible, origin)
    assert replay["arguments"]["query"] == "connect"
    assert (
        replay["arguments"]["page"]["evidence_demand"]
        == origin["arguments"]["page"]["evidence_demand"]
    )
    assert replay["arguments"]["page"]["cursor"] == "public-cursor"
    assert "cursor" not in origin["arguments"]["page"]
    assert set(replay["arguments"]) == {"library", "query", "families", "page"}
    fresh = {
        "tool": "get_evidence",
        "arguments": {
            "source": {"kind": "artifact", "artifact": [9] * 16},
            "page": {"cursor": "another-source"},
        },
    }
    assert _continuation_projection(fresh, origin) == fresh


@pytest.mark.anyio
async def test_python_followup_discovery_keeps_each_actual_public_origin(monkeypatch):
    """Source-only orchestration control; stubs establish no Rust/native acceptance."""
    import sys
    from types import SimpleNamespace

    import programmatic_eval as runner

    initial = {
        "tool": "search_operations",
        "arguments": {"library": "controlled", "query": "connect"},
    }
    following = {
        "tool": "get_operation",
        "arguments": {"library": "controlled", "operation": {"kind": "member", "member": [1] * 16}},
    }
    task = {
        "public_call": initial,
        "request": {"allowed_followups": ["get_operation"]},
        "envelope": {"max_calls": 2, "max_bytes": 32768},
    }
    monkeypatch.setitem(
        sys.modules, "lctx_semantics", SimpleNamespace(wire_decode=lambda *_args: None)
    )
    monkeypatch.setattr(
        runner, "_public_projection", lambda value: copy.deepcopy(value["public_call"])
    )

    def facts(_task, _raw, calls, lane, timeout):
        return {
            "lane": lane,
            "semantic_snapshot": "fixed-source",
            "native_realization": "fixed-native",
            "database_identity": "fixed-database",
            "serialization": "rust_renderer_result",
            "wire_identity": "fixed-wire",
            "calls": calls,
            "timeout_millis": timeout,
            "call_limit": 3,
            "byte_limit": 32768,
            "precision": "not_requested",
        }

    monkeypatch.setattr(runner, "_capture_facts", facts)
    inspected = []

    class RoutingWorker:
        def request(self, request):
            assert request["operation"] == "observe"
            packet = copy.deepcopy(request["observation"])
            inspected.append(packet)
            if len(inspected) == 1:
                assert packet["capture"]["calls"] == [initial]
                return {"public_references": [json.dumps(following)]}
            assert packet["capture"]["calls"] == [following]
            assert packet["expansions"] == []
            assert packet["capture"]["serialization"] == "sdk_result_object"
            assert json.loads(packet["segments"][0])["captured_tool"] == "get_operation"
            return {"public_references": []}

    class Result:
        def __init__(self, tool):
            self.tool = tool

        def model_dump_json(self, **_kwargs):
            return json.dumps({"isError": False, "captured_tool": self.tool})

    class SourceClient:
        async def call_tool_mcp(self, tool, _arguments, **_kwargs):
            return Result(tool)

    observed = await runner.capture_public_journey(
        RoutingWorker(), SourceClient(), task, lane="renderer"
    )
    assert observed["status"] == "completed"
    assert observed["capture"]["calls"] == [initial, following]
    assert len(observed["segments"]) == 2 and len(observed["expansions"]) == 1
    assert len(inspected) == 2


def test_actual_capture_freeze_binds_source_native_wire_budget_precision(worker):
    from programmatic_eval import capture_renderer

    task, response = renderer_source_case()
    observation = capture_renderer(task, response)
    capture = observation["capture"]
    case = {"task": task, "observation": observation, "mode": "immediate"}
    realization = {
        "observation_realization": observation["realization"],
        "source": capture["semantic_snapshot"],
        "native": capture["native_realization"],
        "encoder": "not_requested",
        "scorer": "source-renderer",
        "settings": {
            "wire_identity": capture["wire_identity"],
            "database_identity": capture["database_identity"],
            "lane": capture["lane"],
            "serialization": capture["serialization"],
            "timeout_millis": capture["timeout_millis"],
        },
    }
    frozen = prepare_comparison(
        worker,
        [case],
        "renderer-1",
        realization,
        realization,
        [],
        {"metrics": "epistemic", "numeric_precision_ties": "not_requested"},
    )
    assert frozen_judgments(worker, frozen, [case], frozen["experiment"], lane="baseline")[0][
        "scorable"
    ]
    for field, value in [
        ("semantic_snapshot", "foreign"),
        ("native_realization", "foreign"),
        ("wire_identity", "foreign"),
        ("database_identity", "foreign"),
        ("byte_limit", 1),
        ("precision", "unknown"),
        ("lane", "native"),
        ("timeout_millis", 1),
        ("serialization", "foreign"),
    ]:
        changed = copy.deepcopy(case)
        changed["observation"]["capture"][field] = value
        with pytest.raises(WorkerError):
            frozen_judgments(worker, frozen, [changed], frozen["experiment"], lane="baseline")


@pytest.mark.anyio
async def test_actual_capture_timeout_is_explicit_not_empty_success(worker):
    import asyncio

    from programmatic_eval import capture_public_journey

    task, _ = renderer_source_case()

    class SlowClient:
        async def call_tool_mcp(self, *_args, **_kwargs):
            await asyncio.sleep(1)

    observation = await capture_public_journey(
        worker, SlowClient(), task, lane="renderer", timeout_seconds=0.005
    )
    row = list(worker.judge([{"task": task, "observation": observation, "mode": "expandable"}]))[0]
    assert row["execution"] == "failed"
    assert row["epistemic"] == "inconclusive"
    assert not row["scorable"]


@pytest.mark.anyio
async def test_actual_mcp_continuation_is_visible_bounded_and_source_scoped(worker):
    from fastmcp import Client, FastMCP

    from lctx_mcp.wire import register
    from programmatic_eval import capture_public_journey

    task, response = renderer_source_case()
    initial = copy.deepcopy(response)
    body = response["evidence"]["body"]["bytes"]
    cut = 12
    # Same ordering-bearing hex JSON as the Rust renderer control. This controlled
    # renderer does not assert native snapshot/request binding or cursor admission.
    # BLAKE3("serving-key-order/v2:default"), computed with locked blake3 1.8.5.
    ordering = bytes.fromhex("6ade0dd88d6cb7bc982071c22a05cca2b25c1db2268561ad0d3fe9691b8aceb2")
    cursor = json.dumps({"binding": {"ordering": list(ordering)}}).encode().hex()
    initial["evidence"]["body"].update(
        {
            "bytes": body[:cut],
            "end": cut,
            "continuation": cursor,
            "truncated": True,
            "omitted": len(body) - cut,
        }
    )
    initial["delivery"]["fields"][1]["original"]["end"] = cut
    initial["delivery"]["omissions"].append(
        {
            "field": "/structuredContent/evidence/body",
            "availability": {"status": "partial", "reason": "original_body_page"},
            "expand": {
                "tool": "get_evidence",
                "arguments": {
                    "source": copy.deepcopy(task["public_call"]["arguments"]["source"]),
                    "page": {"cursor": cursor, "expanded": False},
                },
            },
        }
    )
    final = copy.deepcopy(response)
    final["evidence"]["body"].update({"bytes": body[cut:], "start": cut})
    final["delivery"]["fields"][1]["original"]["start"] = cut
    # Exact each-page meaning; the independent oracle never authorizes concatenating arbitrary spans.
    task["predicates"][0]["accepted_text"] = [bytes(body[cut:]).decode()]
    sent = []

    class ControlledPages:
        async def execute(self, tool, arguments):
            sent.append({"tool": tool, "arguments": arguments})
            return json.dumps(final if "cursor" in arguments.get("page", {}) else initial)

    server = FastMCP("private current pagination control", dereference_schemas=False)
    register(server, ControlledPages())
    async with Client(server) as client:
        observation = await capture_public_journey(worker, client, task, lane="renderer")
    assert len(sent) == 2
    assert sent[1]["arguments"]["page"]["cursor"] == cursor
    assert observation["capture"]["calls"] == sent
    case = {"task": task, "observation": observation, "mode": "expandable"}
    assert list(worker.judge([case]))[0]["epistemic"] == "sufficient"
    case["mode"] = "immediate"
    assert list(worker.judge([case]))[0]["epistemic"] == "insufficient"
    case["mode"] = "expandable"
    wrong_expansion = copy.deepcopy(case)
    public = json.loads(wrong_expansion["observation"]["segments"][0])
    delivered = copy.deepcopy(public["structuredContent"]["evidence"])
    body_omission = next(
        item
        for item in public["structuredContent"]["delivery"]["omissions"]
        if item["field"] == "/structuredContent/evidence/body"
    )
    body_omission["expand"]["arguments"]["page"]["evidence_demand"] = {
        "facets": ["originals"],
        "context": {"analysis": [8] * 16},
        "maximum_followups": 0,
    }
    assert public["structuredContent"]["evidence"] == delivered
    wrong_expansion["observation"]["segments"][0] = json.dumps(public, ensure_ascii=False)
    assert list(worker.judge([wrong_expansion]))[0]["epistemic"] == "inconclusive"
    case["observation"]["capture"]["calls"][1]["arguments"]["source"]["artifact"] = [8] * 16
    assert list(worker.judge([case]))[0]["epistemic"] == "inconclusive"
    final["snapshot"]["semantic"] = [7] * 32
    async with Client(server) as client:
        stale = await capture_public_journey(worker, client, task, lane="renderer")
    row = list(worker.judge([{"task": task, "observation": stale, "mode": "expandable"}]))[0]
    assert row["execution"] == "stale"
    assert not row["scorable"]


def test_unbounded_journey_envelopes_refuse_before_any_public_effect(worker):
    from programmatic_eval import _public_projection

    case = population()[0]
    case["task"]["envelope"]["max_calls"] = 257
    assert list(worker.judge([case]))[0]["applicability"] == "invalid_task"
    with pytest.raises(WorkerError, match="bounded public journey"):
        _public_projection(case["task"])


def test_stage_diagnosis_localizes_only_observed_intervals(worker):
    cases = {case["task"]["id"]: case for case in population()}
    task = cases["compatible"]["task"]
    report = diagnose_stages(
        worker,
        task,
        [
            {
                "stage": "expansion",
                "basis": "controlled_injection",
                "observation": cases["compatible"]["observation"],
            },
            {
                "stage": "serialized_delivery",
                "basis": "controlled_injection",
                "observation": cases["drop-setup"]["observation"],
            },
        ],
    )
    assert report["diagnostic_only"] is True
    assert report["first_observed_loss"]["after"] == "expansion"
    assert report["first_observed_loss"]["unobserved_between"] == ["fusion_rescore", "packing"]
    assert report["rows"][1]["judgment"]["epistemic"] == "insufficient"
    with pytest.raises(WorkerError, match="capture facts"):
        diagnose_stages(
            worker,
            task,
            [
                {
                    "stage": "native_nomination",
                    "basis": "actual_capture",
                    "observation": cases["compatible"]["observation"],
                }
            ],
        )
    with pytest.raises(WorkerError, match="ordered"):
        diagnose_stages(
            worker,
            task,
            [
                {
                    "stage": "packing",
                    "basis": "controlled_injection",
                    "observation": cases["compatible"]["observation"],
                },
                {
                    "stage": "expansion",
                    "basis": "controlled_injection",
                    "observation": cases["compatible"]["observation"],
                },
            ],
        )


def test_stage_diagnosis_preserves_unsupported_oracle_and_budget_outcomes(worker):
    cases = {case["task"]["id"]: case for case in population()}
    unsupported = cases["unsupported"]
    report = diagnose_stages(
        worker,
        unsupported["task"],
        [
            {
                "stage": "serialized_delivery",
                "basis": "controlled_injection",
                "observation": unsupported["observation"],
            }
        ],
    )
    assert report["first_observed_loss"] is None
    assert report["rows"][0]["judgment"]["applicability"] == "unsupported"
    task = copy.deepcopy(cases["compatible"]["task"])
    task["envelope"]["max_bytes"] = 1
    report = diagnose_stages(
        worker,
        task,
        [
            {
                "stage": "packing",
                "basis": "controlled_injection",
                "observation": cases["compatible"]["observation"],
            }
        ],
    )
    assert report["first_observed_loss"] is None
    assert report["rows"][0]["judgment"]["epistemic"] == "budget_infeasible"


def test_renderer_inventory_enumeration_keeps_dependency_and_context_bundles(monkeypatch):
    import programmatic_eval as runner

    case = population()[0]
    observation = copy.deepcopy(case["observation"])
    observation["capture"] = {"byte_limit": 32768}
    monkeypatch.setattr(
        runner, "capture_renderer", lambda task, response: copy.deepcopy(observation)
    )

    class StubWorker(Worker):
        def __init__(self):
            pass

        def judge(self, cases):
            for row in cases:
                yield {
                    "scorable": True,
                    "epistemic": "budget_infeasible"
                    if row["task"]["envelope"]["max_bytes"] == 1
                    else "sufficient",
                }

    worker = StubWorker()
    inventory = {
        "complete": False,
        "scope": "declared tiny bundles only",
        "basis": "hand-authored independent inventory",
        "bundles": [
            {"id": "a", "context": {"variant": "a"}},
            {"id": "setup", "context": {"variant": "a"}, "requires": ["a"]},
            {"id": "b", "context": {"variant": "b"}},
        ],
    }
    result = renderer_inventory_ceiling(
        worker, case["task"], inventory, lambda selected: {}, [1, 32768]
    )
    assert result["selections"] == [[], ["a"], ["a", "setup"], ["b"]]
    assert result["inventory_complete"] is False
    assert result["profiles"][0]["status"] == "inconclusive"
    assert result["profiles"][1]["status"] == "feasible"
    inventory["bundles"] *= 3
    with pytest.raises(WorkerError, match="bounded"):
        renderer_inventory_ceiling(worker, case["task"], inventory, lambda selected: {}, [32768])


def test_actual_renderer_ceiling_uses_independent_source_inventory(worker):
    task, response = renderer_source_case()
    raw = bytes(response["evidence"]["body"]["bytes"])
    cut = len(raw) // 2
    inventory = {
        "complete": True,
        "scope": "all contiguous prefix alternatives of this controlled source only",
        "basis": "hand-authored source bytes with exact contiguous offsets",
        "bundles": [
            {"id": "prefix", "context": {"analysis": "03" * 16}, "bytes": list(raw[:cut])},
            {
                "id": "suffix",
                "context": {"analysis": "03" * 16},
                "requires": ["prefix"],
                "bytes": list(raw[cut:]),
            },
        ],
    }

    def compose(selected):
        packet = copy.deepcopy(response)
        delivered = [byte for bundle in selected for byte in bundle["bytes"]]
        packet["evidence"]["body"].update(bytes=delivered, end=len(delivered))
        packet["delivery"]["fields"][1]["original"]["end"] = len(delivered)
        return packet

    result = renderer_inventory_ceiling(worker, task, inventory, compose, [1, 32768])
    assert result["inventory_complete"] is True
    assert result["selections"] == [[], ["prefix"], ["prefix", "suffix"]]
    assert [row["status"] for row in result["profiles"]] == ["budget_infeasible", "feasible"]
    assert result["profiles"][1]["alternative"] == 2
    assert result["emitted_bytes"] == sorted(result["emitted_bytes"])


def test_actual_mcp_shrink_retains_task_oracle_supported_failure_and_coordinates(worker):
    from programmatic_eval import capture_renderer

    task, response = renderer_source_case()
    # The independently authored task requires the entire captured source. Start
    # with a truthful contiguous partial candidate, then preserve that same loss.
    partial = copy.deepcopy(response)
    cut = len(partial["evidence"]["body"]["bytes"]) // 2
    partial["evidence"]["body"].update(bytes=partial["evidence"]["body"]["bytes"][:cut], end=cut)
    partial["delivery"]["fields"][1]["original"]["end"] = cut
    case = {"task": task, "observation": capture_renderer(task, partial), "mode": "immediate"}
    initial = list(worker.judge([case]))[0]
    assert initial["scorable"] and initial["epistemic"] == "insufficient"
    result = minimize_mcp(worker, case, max_trials=16)
    reduced = result["case"]
    assert result["diagnostic_only"] and result["production_success"] is False
    assert reduced["task"] == task
    assert len(reduced["observation"]["segments"][0].encode()) < len(
        case["observation"]["segments"][0].encode()
    )
    public = json.loads(reduced["observation"]["segments"][0])["structuredContent"]
    body = public["evidence"]["body"]
    assert (
        bytes(body["bytes"])
        == bytes(response["evidence"]["body"]["bytes"])[body["start"] : body["end"]]
    )
    assert list(worker.judge([reduced]))[0]["reason"] == initial["reason"]
    assert result["trials"] <= 16
    with pytest.raises(WorkerError, match="bounded supported"):
        minimize_mcp(worker, case, max_trials=0)
