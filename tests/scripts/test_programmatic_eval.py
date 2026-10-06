"""Pure private evaluator controls. No services, adapters, inference or protected data."""

from __future__ import annotations

import copy
import importlib.util
import json
import os
import subprocess
from pathlib import Path

import pytest
from hypothesis import given, settings, strategies as st

from programmatic_eval import (
    Worker, WorkerError, content_digest, frozen_judgments, grounded_feedback, load_cases, load_observations, minimize, minimize_generated, mutation_outcome, packet_ceiling, prepare_comparison, summary,
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
    for name in ("foreign-variant", "foreign-release", "wrong-anchor", "drop-setup",
                 "corrupt-map", "correct-ids-hidden-meaning", "mislabel-expected-failure"):
        assert mutation_outcome(rows["compatible"], rows[name]) == "caught"
    assert mutation_outcome(rows["compatible"], rows["positive-alternate"], equivalent=True) == "equivalent"
    assert mutation_outcome(rows["compatible"], rows["empty-positive"]) == "invalid"
    assert mutation_outcome(rows["compatible"], rows["unsupported"]) == "inconclusive"


def test_shrink_preserves_supported_oracle_and_failure_class(worker):
    case = next(case for case in population() if case["task"]["id"] == "foreign-variant")
    extra = copy.deepcopy(case["observation"]["spans"][0])
    extra.update(role="irrelevant", anchor="unrelated")
    case["observation"]["spans"].append(extra)
    def preserves(trial):
        row = list(worker.judge([trial]))[0]
        return row["epistemic"] == "insufficient" and row["reason"] == "individually readable predicates have incompatible contexts"
    minimized = minimize(case, preserves)
    assert minimized["task"] == case["task"]
    assert len(minimized["observation"]["spans"]) < len(case["observation"]["spans"])
    assert preserves(minimized)


def test_actual_byte_ceiling_and_missing_inventory(worker):
    case = population()[0]
    assert packet_ceiling(worker, case["task"], [case["observation"]])["status"] == "feasible"
    task = copy.deepcopy(case["task"])
    task["envelope"]["max_bytes"] = 1
    assert packet_ceiling(worker, task, [case["observation"]])["status"] == "budget_infeasible"
    assert packet_ceiling(worker, task, [])["status"] == "inventory_infeasible"
    insufficient = copy.deepcopy(case["observation"])
    insufficient["spans"].pop()
    result = packet_ceiling(worker, task, [case["observation"], insufficient])
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
            [os.sys.executable, "-I", "-c",
             "import runpy; print(runpy.run_path(" + repr(str(path)) + ")[\"connect\"](0))"],
            check=True, capture_output=True, text=True, timeout=5,
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
    realization = {"source": "hand-source-v1", "native": "not_run", "encoder": "not_requested", "scorer": "finite", "settings": {}}
    meanings = {"judgment": "finite-v1", "observation": worker.schema["case"]["$defs"]["Observation"], "metrics": "epistemic-counts", "numeric_precision_ties": "not_requested"}
    cases = population()
    frozen = prepare_comparison(worker, cases, "1", realization, realization, [], meanings)
    assert len(frozen_judgments(worker, frozen, cases, frozen["experiment"])) == len(cases)
    changed = copy.deepcopy(cases)
    changed[0]["task"]["predicates"][0]["accepted_text"] = ["changed expected meaning"]
    with pytest.raises(WorkerError, match="inventory differs"):
        frozen_judgments(worker, frozen, changed, frozen["experiment"])
    experiment = copy.deepcopy(frozen["experiment"])
    experiment["revision"] = "2"
    with pytest.raises(WorkerError, match="incompatible comparison"):
        frozen_judgments(worker, frozen, cases, experiment)


def test_timeout_cancels_the_campaign_process(tmp_path):
    executable = tmp_path / "hung-worker"
    executable.write_text("#!" + os.sys.executable + "\nimport json, sys, time\nfor line in sys.stdin:\n if json.loads(line).get('operation') == 'schema':\n  print(json.dumps({'status':'completed','result':{'case':{'$schema':'https://json-schema.org/draft/2020-12/schema'}}}), flush=True)\n else:\n  time.sleep(30)\n")
    executable.chmod(0o700)
    with Worker(executable, timeout=0.2) as process:
        with pytest.raises(WorkerError, match="timeout"):
            process.request({"operation": "judge", "cases": [population()[0]]})
        assert process.process.poll() is not None


@settings(max_examples=40, derandomize=True, database=None, deadline=None)
@given(signature=st.sets(st.sampled_from(["A", "B", "C"])),
       default=st.sets(st.sampled_from(["A", "B", "C"])),
       setup=st.sets(st.sampled_from(["A", "B", "C"])))
def test_generated_context_population_uses_one_worker(worker, signature, default, setup):
    case = population()[0]
    prototype = case["observation"]["spans"]
    case["observation"]["spans"] = []
    for item, variants in zip(prototype, (signature, default, setup), strict=True):
        for variant in sorted(variants):
            span = copy.deepcopy(item)
            span["context"]["variant"] = variant
            case["observation"]["spans"].append(span)
    judgment = list(worker.judge([case]))[0]
    # Independent finite set intersection, not the production/reference join kernel.
    assert (judgment["epistemic"] == "sufficient") == bool(signature & default & setup)


def test_generator_shrinks_source_task_context_together(worker):
    def build(factors):
        case = next(case for case in population() if case["task"]["id"] == "foreign-variant")
        source = "def connect(timeout=None): return timeout\n" + "".join(factors["source_noise"])
        for span in case["observation"]["spans"]:
            span["context"].update(factors["context_noise"])
        case["task"]["request"]["context"].update(factors["context_noise"])
        case["task"]["oracle"]["input_digest"] = content_digest(source)
        return {"source": source, "case": case}
    def preserves(generated):
        row = list(worker.judge([generated["case"]]))[0]
        return row["reason"] == "individually readable predicates have incompatible contexts"
    factors = {"source_noise": ["# unrelated helper\n", "unused = 42\n"], "context_noise": {"deployment_note": "unused"}}
    minimized = minimize_generated(factors, build, preserves)
    assert minimized["factors"] == {"source_noise": [], "context_noise": {}}
    assert minimized["case"]["source"] == "def connect(timeout=None): return timeout\n"


def test_generated_wire_schema_is_current(worker):
    assert worker.schema == json.loads((ROOT / "eval/programmatic/wire-schema.json").read_text())


def test_python_grounded_feedback_can_reach_both_owners(worker):
    case = population()[0]
    disposition = grounded_feedback(worker, case, "1", "independent source and exact retained observation", ["system", "evaluator", "usability"], "add precedence task and clearer navigation", [case["task"]["id"]], "2")
    assert len(disposition["routes"]) == 3
    assert disposition["new_comparison_required"]
    assert disposition["retain_old_semantic_result"]
    with pytest.raises(WorkerError, match="new revision"):
        grounded_feedback(worker, case, "1", "independent source", ["evaluator"], "new expectation", [case["task"]["id"]])
