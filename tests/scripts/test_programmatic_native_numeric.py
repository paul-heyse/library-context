"""Actual retained native values and exact selection versus independent private references."""

import os
from pathlib import Path
import pytest

from programmatic_eval import Worker, WorkerError
from programmatic_native import NativeReference, native_numeric_reference, _check_exact_order

ROOT = Path(__file__).resolve().parents[2]


def test_exact_reference_preserves_ties_and_refuses_wrong_order_or_values():
    expected = [{"id": "vector:a", "score": 1.0}, {"id": "vector:b", "score": 1.0}]
    actual = [{"vector_id": row["id"], "score": row["score"]} for row in expected]
    _check_exact_order(actual, expected)
    for corrupt in (actual[::-1], actual[:1], actual + actual[:1],
                    [{"vector_id": "vector:a", "score": True}, actual[1]],
                    [{"vector_id": "vector:a", "score": float("nan")}, actual[1]],
                    [{"vector_id": "vector:a", "score": .999}, actual[1]]):
        with pytest.raises(WorkerError, match="independent scalar reference"):
            _check_exact_order(corrupt, expected)


def test_actual_stored_cohort_full_projection_exact_and_union_without_inference():
    configured = os.environ.get("LCTX_NATIVE_SERVING_CONFIG")
    assert configured, "owned native serving fixture required"
    binary = Path(os.environ.get("LCTX_REMEDIATION_CLI_BIN", ROOT / "target/release/lctx"))
    worker_binary = Path(os.environ.get("LCTX_EVAL_WORKER", ROOT / "target/release/lctx-eval"))
    assert binary.is_file(), "current native CLI required"
    with Worker(worker_binary) as worker, NativeReference(binary, Path(configured)) as native:
        result = native_numeric_reference(worker, native, k=2)
        assert result["inference_started"] is False
        assert result["reference"]["eligible_rows"] >= 2
        assert result["reference"]["supplied_population_complete"] is True
        assert result["reference"]["full"][0]["id"] in {
            row["vector_id"] for row in result["native_exact"]
        }
        assert result["reference"]["candidate_union_rescored"]
        assert result["snapshot"] == native.snapshot
