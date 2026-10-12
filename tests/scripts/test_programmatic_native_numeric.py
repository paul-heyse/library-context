"""Actual retained native values and exact selection versus independent private references."""

import os
import copy
import json
import struct
from pathlib import Path
import pytest

from programmatic_eval import Worker, WorkerError, default_worker_path
from programmatic_native import (NativeReference, native_numeric_reference, _check_exact_order,
                                 _check_served, _full, _project, _cosine)

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
    binary = Path(os.environ.get("LCTX_NATIVE_NUMERIC_REPLAY_BIN", ROOT / "target/debug/examples/native_numeric_replay"))
    worker_binary = default_worker_path()
    assert binary.is_file(), "current private native numeric replay example required"
    with Worker(worker_binary) as worker, NativeReference(binary, Path(configured)) as native:
        result = native_numeric_reference(worker, native, k=2)
        assert result["inference_started"] is False
        assert result["reference"]["eligible_rows"] >= 2
        assert result["reference"]["supplied_population_complete"] is True
        assert result["native_candidates"]
        assert len(result["native_rescored"]) == len(result["native_candidates"])
        assert "not displayed public-tool rank" in result["served_comparison"]
        assert result["reference"]["candidate_union_rescored"]
        assert result["snapshot"] == native.snapshot


def test_numeric_reader_freezes_current_minimal_viewer_config_without_installer(tmp_path):
    handle = tmp_path / "selected.json"
    snapshot = {"database": {"namespace": "owned", "database": "validation"}, "service_generation": "a" * 64}
    handle.write_text(json.dumps(snapshot))
    configured = {"endpoint": "grpc://127.0.0.1:33385", "username": "database-pin-reader",
                  "password": "private-test-secret", "selection": str(handle)}
    path = tmp_path / "serving.json"
    path.write_text(json.dumps(configured))
    with NativeReference(tmp_path / "example", path) as native:
        pinned = json.loads(native.viewer.read_text())
        assert set(pinned) == set(configured)
        assert pinned["username"] == configured["username"]
        assert pinned["selection"] == str(native.handle)
        assert native.viewer.stat().st_mode & 0o777 == 0o600
        handle.write_text("{}")
        assert json.loads(native.handle.read_text()) == snapshot


def observed_candidates():
    def occurrence(unit, window):
        return {"target": {"kind": "unit", "unit": [unit] * 16}, "unit": [unit] * 16,
                "window": [window] * 16, "part": [window] * 16, "binding": None,
                "context": [1] * 16, "anchor": None, "family": 0}
    a, b = occurrence(1, 1), occurrence(2, 2)
    query = [1.0] + [0.0] * 4095
    # Projection ties at1; the second stored full winner is only cosine0.6.
    other = [struct.unpack("<f", struct.pack("<f", .6))[0]] + [0.0] * 4095
    other[1024] = struct.unpack("<f", struct.pack("<f", .8))[0]
    rows = [{"vector_id": "vector:a", "full_key": "a"}, {"vector_id": "vector:b", "full_key": "b"}]
    witnesses = [{"vector_id": "vector:a", "occurrence": a, "occurrence_key": "a"},
                 {"vector_id": "vector:b", "occurrence": b, "occurrence_key": "b"}]
    candidates = [{"occurrence": a, "vector_ids": ["vector:a"], "score": 1.0},
                  {"occurrence": b, "vector_ids": ["vector:b"], "score": 1.0}]
    rescored = copy.deepcopy(candidates)
    rescored[1]["score"] = _cosine(query, other)
    return {"rows": rows, "occurrences": witnesses, "native_candidates": candidates,
            "native_rescored": rescored}, {"a": query, "b": other}


def test_served_candidate_order_and_full_rescore_are_independent_and_reject_corruption():
    observed, full = observed_candidates()
    assert _check_served(observed, full, 2) == ["vector:a", "vector:b"]
    for change in ("order", "projected", "full", "foreign", "missing", "lineage"):
        broken = copy.deepcopy(observed)
        if change == "order":
            broken["native_candidates"].reverse()
        elif change == "projected":
            broken["native_candidates"][1]["score"] = .999
        elif change == "full":
            broken["native_rescored"][1]["score"] = 1.0
        elif change == "foreign":
            broken["native_candidates"][1]["vector_ids"] = ["vector:foreign"]
        elif change == "missing":
            broken["native_rescored"].pop()
        else:
            broken["native_rescored"][0]["occurrence"]["window"] = [3] * 16
        with pytest.raises(WorkerError):
            _check_served(broken, full, 2)


def test_raw_f32_bits_require_full4096_codec_and_keep_tail_information():
    values = [0.0] * 4096
    values[0], values[1024] = .6, .8
    raw = struct.pack("<4096f", *values)
    record = {"dimensions": 4096, "codec": 1, "bytes_hex": raw.hex()}
    decoded = _full(record)
    assert decoded[1024] != 0.0
    assert _project(decoded) == [1.0] + [0.0] * 1023
    for broken in ({**record, "dimensions": 1024}, {**record, "codec": 2},
                   {**record, "bytes_hex": raw[:-1].hex()},
                   {**record, "bytes_hex": (struct.pack("<f", float("nan")) + raw[4:]).hex()}):
        with pytest.raises(WorkerError):
            _full(broken)


def test_numeric_worker_frames_are_bounded_and_global_ties_merge_across_blocks():
    # This host control exercises adapter framing/merge; native acceptance is separate.
    query = [1.0] + [0.0] * 4095
    occurrences = []
    rows = []
    candidates = []
    for index in range(33):
        vector_id = f"vector:{index:03}"
        unit = list(index.to_bytes(16, "big"))
        occurrence = {"target": {"kind": "unit", "unit": unit}, "unit": unit,
                      "window": unit, "part": unit, "binding": None,
                      "context": [1] * 16, "anchor": None, "family": 0}
        rows.append({"vector_id": vector_id, "full_key": "shared", "embedding": query[:1024]})
        occurrences.append({"vector_id": vector_id, "occurrence": occurrence,
                            "occurrence_key": vector_id})
        candidates.append({"occurrence": occurrence, "vector_ids": [vector_id], "score": 1.0})
    observed = {"rows": rows, "occurrences": occurrences,
                "full": [{"semantic_key": "shared", "dimensions": 4096, "codec": 1,
                          "bytes_hex": struct.pack("<4096f", *query).hex()}],
                "native_candidates": candidates, "native_rescored": candidates,
                "cohort": {"host_control": True}}

    class Native:
        snapshot = {"host_control": True}

        def observe(self, k, cohort):
            assert k == 2 and cohort is None
            return observed

    class NumericWorker:
        calls = []

        def request(self, request):
            assert request["operation"] == "numeric"
            supplied = request["input"]
            assert supplied["policy"] == {"full_dimensions": 4096, "projection_dimensions": 1024,
                                         "block_rows": 32, "k": 2}
            vectors = supplied["vectors"]
            assert len(vectors) <= 32
            assert all(len(row["values"]) == 4096 and row["eligible"] for row in vectors)
            assert supplied["nominated_ids"] == [row["id"] for row in vectors]
            self.calls.append([row["id"] for row in vectors])
            hits = [{"id": row["id"], "score": 1.0} for row in vectors[:2]]
            return {"full": hits, "projected": hits, "candidate_union_rescored": hits,
                    "eligible_rows": len(vectors), "supplied_population_complete": True}

    worker = NumericWorker()
    result = native_numeric_reference(worker, Native(), k=2)
    assert [len(block) for block in worker.calls] == [32, 1]
    assert [row for block in worker.calls for row in block] == [row["vector_id"] for row in rows]
    assert result["reference"]["eligible_rows"] == 33
    for field in ("full", "projected", "candidate_union_rescored"):
        assert [row["id"] for row in result["reference"][field]] == ["vector:000", "vector:001"]


def test_served_target_frontier_keeps_best1024_and_replaces_earlier_losing_witness():
    # Two documents can supply more than1024 unique targets in the first128-doc tier.
    # A better witness for the initially losing target arrives last in the stream.
    query = [1.0] + [0.0] * 4095
    low = [struct.unpack("<f", struct.pack("<f", .6))[0],
           struct.unpack("<f", struct.pack("<f", .8))[0]] + [0.0] * 4094
    score = _cosine(query, low)

    def witness(index, document, window):
        unit = list(index.to_bytes(16, "big"))
        occurrence = {"target": {"kind": "unit", "unit": unit}, "unit": unit,
                      "window": [window] * 16, "part": [window] * 16, "binding": None,
                      "context": [1] * 16, "anchor": None, "family": 0}
        return {"vector_id": document, "occurrence": occurrence,
                "occurrence_key": f"{index:04}:{window}"}

    late = witness(1024, "vector:a", 2)
    lows = [witness(index, "vector:b", 1) for index in range(1025)]
    candidates = [{"occurrence": row["occurrence"], "vector_ids": [row["vector_id"]],
                   "score": 1.0 if row is late else score} for row in [late, *lows[:1023]]]
    # Full values use no tail, so independently computed full and projected scores agree.
    rescored = sorted(candidates, key=lambda row: tuple(row["occurrence"]["unit"]))
    observed = {"rows": [{"vector_id": "vector:a", "full_key": "query"},
                          {"vector_id": "vector:b", "full_key": "low"}],
                "occurrences": [*reversed(lows), late],
                "native_candidates": candidates, "native_rescored": rescored}
    full = {"query": query, "low": low}
    assert len(candidates) == 1024
    assert _check_served(observed, full, 2) == ["vector:a", "vector:b"]
    observed["occurrences"].reverse()
    assert _check_served(observed, full, 2) == ["vector:a", "vector:b"]
    unexpected = {"occurrence": lows[1023]["occurrence"], "vector_ids": ["vector:b"], "score": score}
    observed["native_candidates"] = [*candidates, unexpected]
    with pytest.raises(WorkerError, match="canonical witness order"):
        _check_served(observed, full, 2)


def test_private_numeric_storage_admission_precedes_creation_and_outlives_join(tmp_path, monkeypatch):
    from contextlib import contextmanager
    import programmatic_native as module
    import sys

    handle = tmp_path / "selected.json"
    handle.write_text("{}")
    config = tmp_path / "viewer.json"
    config.write_text(json.dumps({"endpoint": "grpc://127.0.0.1:33385", "username": "pin-reader",
                                  "password": "private-test", "selection": str(handle)}))
    events = []
    original = module.tempfile.TemporaryDirectory

    @contextmanager
    def admitted(paths):
        assert paths == [tmp_path]
        assert not list(tmp_path.glob("lctx-private-numeric-*"))
        events.append("admitted")
        yield
        assert not list(tmp_path.glob("lctx-private-numeric-*"))
        events.append("released-after-scratch-cleanup")

    def scratch(*args, **kwargs):
        assert events == ["admitted"]
        return original(*args, **kwargs)

    monkeypatch.setattr(module, "current_run", lambda: tmp_path)
    monkeypatch.setattr(module, "admission", admitted)
    monkeypatch.setattr(module.tempfile, "TemporaryDirectory", scratch)
    binary = tmp_path / "host-observation"
    binary.write_text(f"#!{sys.executable}\n" +
                      "import json, pathlib, sys\n" +
                      "cfg = json.loads(pathlib.Path(sys.argv[1]).read_text())\n" +
                      "snapshot = json.loads(pathlib.Path(cfg['selection']).read_text())\n" +
                      "request = json.load(sys.stdin)\n" +
                      "assert request == {'k': 1, 'cohort': None}\n" +
                      "print(json.dumps({'schema': 1, 'snapshot': snapshot, 'inference_started': False}))\n")
    binary.chmod(0o700)
    with NativeReference(binary, config) as native:
        assert native.viewer.parent.parent == tmp_path
        assert native.observe(1, None)["snapshot"] == {}
        # The host child has actually exited and joined; admission still protects scratch.
        assert native.viewer.is_file()
        assert events == ["admitted"]
    assert events == ["admitted", "released-after-scratch-cleanup"]
