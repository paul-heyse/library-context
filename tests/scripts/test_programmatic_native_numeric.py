"""Actual retained native values and HNSW versus independent private references."""

import os
from pathlib import Path

from programmatic_eval import Worker
from programmatic_native import NativeReference, native_numeric_reference

ROOT = Path(__file__).resolve().parents[2]


def test_actual_stored_cohort_full_projection_ann_and_union_without_inference():
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
            row["vector_id"] for row in result["native_ann"]
        }
        assert result["reference"]["candidate_union_rescored"]
        assert result["snapshot"] == native.snapshot
