"""Independent F64 replay of protected stored F32 winners and served candidate kernels."""

from __future__ import annotations

import json
import math
import os
import selectors
import signal
import struct
import subprocess
import tempfile
import time
from contextlib import ExitStack, suppress
from pathlib import Path
from typing import Any

from programmatic_eval import Worker, WorkerError, stored_vector_reference
from runs import current_run
from storage_lifecycle import admission

PACKET_BYTES = 32 << 20
TARGET_FRONTIER = 1024


class NativeReference:
    """Freeze a current ViewerConfig's exact handle before starting the replay owner."""

    def __init__(self, binary: Path, viewer: Path):
        self.binary = binary.resolve()
        configured = json.loads(viewer.read_text())
        self.snapshot = json.loads(Path(configured["selection"]).read_text())
        # Admit the real parent before creation. The existing live run owns nested
        # scratch; standalone use has only this lexical temporary lifetime.
        # ExitStack removes scratch after child join and before releasing admission.
        parent = current_run() or Path(tempfile.gettempdir())
        lifetime = ExitStack()
        try:
            lifetime.enter_context(admission([parent]))
            self._scratch = lifetime.enter_context(
                tempfile.TemporaryDirectory(prefix="lctx-private-numeric-", dir=parent)
            )
            root = Path(self._scratch)
            self.handle = root / "handle.json"
            self.viewer = root / "serving.json"
            # Current trusted readers use the existing database pin-writing principal.
            # Keep the minimal ViewerConfig; never invent RuntimeConfig/installer secrets.
            pinned = {key: configured[key] for key in ("endpoint", "username", "password")}
            pinned["selection"] = str(self.handle)
            if "serving_limits" in configured:
                pinned["serving_limits"] = configured["serving_limits"]
            for path, value in ((self.handle, self.snapshot), (self.viewer, pinned)):
                descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
                with os.fdopen(descriptor, "w") as stream:
                    json.dump(value, stream)

        except BaseException:
            lifetime.close()
            raise
        self._lifetime = lifetime

    def observe(self, k: int, cohort: dict[str, Any] | None) -> dict[str, Any]:
        request = json.dumps({"k": k, "cohort": cohort}, allow_nan=False).encode()
        output = bytearray()
        refusal = None
        # Never expose SDK/config diagnostics or force-kill accepted native work.
        # At the existing read deadline/bound, cooperatively cancel then drain
        # discarded output until actual exit. Joining is not another execution deadline.
        with subprocess.Popen(
            [str(self.binary), str(self.viewer)],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
        ) as child:
            assert child.stdin is not None and child.stdout is not None
            try:
                child.stdin.write(request)
                child.stdin.close()
                deadline = time.monotonic() + 30
                with selectors.DefaultSelector() as ready:
                    ready.register(child.stdout, selectors.EVENT_READ)
                    while ready.get_map():
                        remaining = None if refusal else max(0.0, deadline - time.monotonic())
                        events = ready.select(remaining)
                        if not events and refusal is None:
                            refusal = "native reference read timed out"
                            if child.poll() is None:
                                child.send_signal(signal.SIGINT)
                            continue
                        for key, _ in events:
                            chunk = os.read(key.fd, 65536)
                            if not chunk:
                                ready.unregister(key.fileobj)
                                continue
                            if refusal is None and len(output) + len(chunk) > PACKET_BYTES:
                                refusal = "native reference response exceeds bounded replay"
                                if child.poll() is None:
                                    child.send_signal(signal.SIGINT)
                            if refusal is None:
                                output.extend(chunk)
                status = child.wait()
            except BaseException:
                # Caller interruption must join the same accepted owner too.
                # Do not let Popen.__exit__ wait on a child blocked by its stdout pipe.
                if child.poll() is None:
                    # The raced child exit is still joined below.
                    with suppress(ProcessLookupError):
                        child.send_signal(signal.SIGINT)
                with suppress(BrokenPipeError):
                    child.stdin.close()
                while child.stdout.read(65536):
                    pass
                child.wait()
                raise
        if refusal:
            raise WorkerError(refusal)
        if status:
            raise WorkerError("native reference replay failed")
        try:
            observed = json.loads(output)
        except ValueError as error:
            raise WorkerError("native reference response is not JSON") from error
        if (
            not isinstance(observed, dict)
            or observed.get("schema") != 1
            or observed.get("snapshot") != self.snapshot
            or observed.get("inference_started") is not False
        ):
            raise WorkerError("native reference snapshot or observation contract changed")
        return observed

    def close(self) -> None:
        self._lifetime.close()

    def __enter__(self) -> NativeReference:
        return self

    def __exit__(self, *_: object) -> None:
        self.close()


def _full(record: dict[str, Any]) -> list[float]:
    if record.get("dimensions") != 4096 or record.get("codec") != 1:
        raise WorkerError("native reference requires canonical full4096 F32 values")
    try:
        raw = bytes.fromhex(record["bytes_hex"])
    except (ValueError, TypeError, KeyError) as error:
        raise WorkerError("native full value has invalid raw bits") from error
    if len(raw) != 4096 * 4:
        raise WorkerError("native full value has incompatible byte length")
    values = list(struct.unpack("<4096f", raw))
    if not all(math.isfinite(value) for value in values):
        raise WorkerError("native full value is nonfinite")
    return values


def _check_exact_order(actual: list[dict[str, Any]], expected: list[dict[str, Any]]) -> None:
    if [row.get("vector_id") for row in actual] != [row["id"] for row in expected] or any(
        type(row.get("score")) not in (int, float)
        or not math.isfinite(row["score"])
        or not math.isclose(row["score"], wanted["score"], rel_tol=1e-12, abs_tol=1e-12)
        for row, wanted in zip(actual, expected, strict=True)
    ):
        raise WorkerError(
            "native exact ordering or scores differ from independent scalar reference"
        )


def _cosine(left: list[float], right: list[float]) -> float:
    dot = left_norm = right_norm = 0.0
    for a, b in zip(left, right, strict=True):
        dot += a * b
        left_norm += a * a
        right_norm += b * b
    denominator = math.sqrt(left_norm) * math.sqrt(right_norm)
    if not denominator or not math.isfinite(denominator):
        raise WorkerError("native reference has zero or nonfinite norm")
    return dot / denominator


def _project(values: list[float]) -> list[float]:
    prefix = values[:1024]
    squared_length = 0.0
    for value in prefix:
        squared_length += value * value
    length = math.sqrt(squared_length)
    if not length or not math.isfinite(length):
        raise WorkerError("native reference has invalid projected values")
    return [struct.unpack("<f", struct.pack("<f", value / length))[0] for value in prefix]


def _identity(occurrence: dict[str, Any]) -> str:
    return json.dumps(occurrence, sort_keys=True, separators=(",", ":"))


def _occurrence_order(row: dict[str, Any]) -> tuple:
    def optional(value):
        return (0, ()) if value is None else (1, tuple(value))

    return (
        tuple(row["target"]["unit"]),
        row["family"],
        tuple(row["unit"]),
        tuple(row["window"]),
        tuple(row["part"]),
        optional(row["binding"]),
        tuple(row["context"]),
        optional(row["anchor"]),
    )


def _expected_candidates(
    occurrences: list[dict[str, Any]], scores: dict[str, float], k: int
) -> list[dict[str, Any]]:
    """Independently rank finite document tiers, then exact (unit,context) witnesses."""
    ordered = sorted(scores, key=lambda key: (-scores[key], key))
    ranks = {key: index for index, key in enumerate(ordered)}
    for tier in (128, 256, 512, 1024):
        best = {}
        for row in occurrences:
            vector = row["vector_id"]
            if vector not in ranks:
                raise WorkerError("foreign native occurrence document")
            if ranks[vector] >= tier:
                continue
            occurrence = row["occurrence"]
            target = (tuple(occurrence["unit"]), tuple(occurrence["context"]))
            rank = (-scores[vector], row["occurrence_key"])
            if target not in best or rank < best[target][0]:
                best[target] = (rank, row)
        winners = [
            row
            for _, row in sorted(
                best.values(),
                key=lambda item: (
                    item[0][0],
                    tuple(item[1]["occurrence"]["unit"]),
                    tuple(item[1]["occurrence"]["context"]),
                ),
            )
        ][:TARGET_FRONTIER]
        if len(winners) >= k:
            break
    return winners


def _check_served(observed: dict[str, Any], full: dict[str, list[float]], k: int) -> list[str]:
    rows = observed["rows"]
    values = {row["vector_id"]: full[row["full_key"]] for row in rows}
    query = values[rows[0]["vector_id"]]
    projected_query = _project(query)
    projected = {key: _cosine(projected_query, _project(value)) for key, value in values.items()}
    full_scores = {key: _cosine(query, value) for key, value in values.items()}
    expected = _expected_candidates(observed["occurrences"], projected, k)
    actual = observed["native_candidates"]
    if [_identity(row["occurrence"]) for row in actual] != [
        _identity(row["occurrence"]) for row in expected
    ]:
        raise WorkerError("served candidate identity or canonical witness order differs")
    nominated = set()
    for row, wanted in zip(actual, expected, strict=True):
        matching = sorted(
            {
                w["vector_id"]
                for w in observed["occurrences"]
                if w["occurrence"] == wanted["occurrence"]
            }
        )
        if row["vector_ids"] != matching:
            raise WorkerError("served candidate has foreign or missing selected documents")
        _check_exact_order(
            [{"vector_id": wanted["vector_id"], "score": row["score"]}],
            [{"id": wanted["vector_id"], "score": projected[wanted["vector_id"]]}],
        )
        nominated.update(matching)
    rescored = observed["native_rescored"]
    expected_rescore = sorted(actual, key=lambda row: _occurrence_order(row["occurrence"]))
    if [_identity(row["occurrence"]) for row in rescored] != [
        _identity(row["occurrence"]) for row in expected_rescore
    ]:
        raise WorkerError("served rescore union changed exact nominated occurrences")
    for row, wanted in zip(rescored, expected_rescore, strict=True):
        if row["vector_ids"] != wanted["vector_ids"]:
            raise WorkerError("served rescore changed selected document lineage")
        scores = [full_scores[key] for key in row["vector_ids"]]
        if not scores or any(score != scores[0] for score in scores):
            raise WorkerError("ambiguous native full winner for one occurrence")
        _check_exact_order(
            [{"vector_id": "actual occurrence", "score": row["score"]}],
            [{"id": "actual occurrence", "score": scores[0]}],
        )
    if not nominated:
        raise WorkerError("served candidate kernel returned no nominated documents")
    return sorted(nominated)


def native_numeric_reference(
    worker: Worker, native: NativeReference, *, k: int = 4, cohort: dict[str, Any] | None = None
) -> dict[str, Any]:
    """Compare actual candidate/rescore kernels over one complete <=256-row cohort."""
    if not 1 <= k <= 64:
        raise WorkerError("unsupported native reference k")
    observed = native.observe(k, cohort)
    rows = observed["rows"]
    if not rows or len(rows) > 256 or len(observed["occurrences"]) > 4096:
        raise WorkerError("native reference cohort empty or exceeds exhaustive bound256")
    ids = [row["vector_id"] for row in rows]
    if ids != sorted(set(ids)):
        raise WorkerError("duplicate or unordered native cohort row")
    full = {}
    for record in observed["full"]:
        key = record["semantic_key"]
        if key in full:
            raise WorkerError("duplicate canonical full row")
        full[key] = _full(record)
    if set(full) != {row["full_key"] for row in rows}:
        raise WorkerError("native reference has missing or foreign canonical winning values")
    for row in rows:
        if row["embedding"] != _project(full[row["full_key"]]):
            raise WorkerError("native indexed projection differs from canonical winning bytes")
    nominated = _check_served(observed, full, k)
    # Every worker frame stays within the existing32-row numerical boundary.
    blocks = []
    for start in range(0, len(rows), 32):
        block = rows[start : start + 32]
        blocks.append(
            stored_vector_reference(
                worker,
                {
                    "policy": {
                        "full_dimensions": 4096,
                        "projection_dimensions": 1024,
                        "block_rows": 32,
                        "k": k,
                    },
                    "query": full[rows[0]["full_key"]],
                    "vectors": [
                        {"id": row["vector_id"], "values": full[row["full_key"]], "eligible": True}
                        for row in block
                    ],
                    "nominated_ids": [
                        row["vector_id"] for row in block if row["vector_id"] in nominated
                    ],
                },
            )
        )
    reference = dict(blocks[0])
    for name in ("full", "projected", "candidate_union_rescored"):
        reference[name] = sorted(
            (hit for block in blocks for hit in block[name]),
            key=lambda hit: (-hit["score"], hit["id"]),
        )[:k]
    reference["eligible_rows"] = sum(block["eligible_rows"] for block in blocks)
    reference["missing_after_projection"] = [
        hit["id"]
        for hit in reference["full"]
        if hit["id"] not in {row["id"] for row in reference["projected"]}
    ]
    reference["missing_from_union"] = [
        hit["id"] for hit in reference["full"] if hit["id"] not in nominated
    ]
    reference["supplied_population_complete"] = all(
        block["supplied_population_complete"] for block in blocks
    )
    return {
        "snapshot": native.snapshot,
        "cohort": observed["cohort"],
        "population_scope": "complete eligible selected rows in this declared native cohort",
        "query_basis": "retained document full4096 winner, numerical control only",
        "inference_started": False,
        "native_candidates": observed["native_candidates"],
        "native_rescored": observed["native_rescored"],
        "served_comparison": (
            "exact candidate witness order and full4096 rescore of its "
            "nominated occurrence union; not displayed public-tool rank"
        ),
        "reference": reference,
    }
