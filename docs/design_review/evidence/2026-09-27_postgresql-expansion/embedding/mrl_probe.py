"""Observe prefix-MRL rank changes on immutable live vectors, without quality labels.

No service, database, gold or held-out input is used. Document vectors are pseudoqueries,
so this is not a task retrieval evaluation. The transformation is an explicitly local
Float64 norm and division followed by Float32 conversion, not a vLLM output oracle.
"""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path

import numpy as np
import pyarrow.ipc as ipc


def observe(path: Path, query_cap: int) -> dict:
    rows = ipc.open_file(path).read_all().to_pylist()
    # Duplicate request texts are not independent vectors or useful self-neighbours.
    vectors = {r["input_hash"]: r["vector"] for r in rows}
    keys = sorted(vectors)
    full = np.asarray([vectors[k] for k in keys], dtype=np.float32)
    prefix = full[:, :1024].astype(np.float64)
    norms = np.linalg.norm(prefix, axis=1)
    assert np.all(norms > 0)
    reduced = (prefix / norms[:, None]).astype(np.float32).astype(np.float64)
    full64 = full.astype(np.float64)
    queries = np.unique(np.linspace(0, len(keys) - 1, min(query_cap, len(keys))).astype(int))
    full_scores = full64[queries] @ full64.T
    short_scores = reduced[queries] @ reduced.T
    eligible = np.ones(full_scores.shape, dtype=bool)
    eligible[np.arange(len(queries)), queries] = False
    absolute_error = np.abs(full_scores[eligible] - short_scores[eligible])
    crossings = (full_scores[eligible] >= 0.5) != (short_scores[eligible] >= 0.5)
    full_scores[~eligible] = -np.inf
    short_scores[~eligible] = -np.inf
    # Stable sorting resolves exact ties by the input hash order above.
    full_ranks = np.argsort(-full_scores, axis=1, kind="stable")
    short_ranks = np.argsort(-short_scores, axis=1, kind="stable")
    overlaps = {}
    for k in (1, 3, 10):
        k = min(k, len(keys) - 1)
        counts = [
            len(set(a[:k]) & set(b[:k])) for a, b in zip(full_ranks, short_ranks, strict=True)
        ]
        overlaps[str(k)] = {"retained_neighbours": sum(counts), "possible": len(queries) * k}
    return {
        "file": str(path),
        "sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
        "source_rows": len(rows),
        "distinct_request_vectors": len(keys),
        "pseudoqueries": len(queries),
        "prefix_norm_min": float(norms.min()),
        "prefix_norm_median": float(np.median(norms)),
        "prefix_norm_max": float(norms.max()),
        "output_norm_max_error": float(np.max(np.abs(np.linalg.norm(reduced, axis=1) - 1))),
        "non_self_pair_count": int(eligible.sum()),
        "mean_absolute_similarity_change": float(absolute_error.mean()),
        "max_absolute_similarity_change": float(absolute_error.max()),
        "pairs_crossing_0_5": int(crossings.sum()),
        "nearest_neighbour_overlap": overlaps,
        "full_payload_bytes": int(full.nbytes),
        "reduced_payload_bytes": int(len(keys) * 1024 * 4),
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("generation", type=Path)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    manifest = json.loads((args.generation / "MANIFEST.json").read_text())
    result = {
        "outcome": "passed",
        "date": "2026-09-27",
        "scope": (
            "numerical geometry observation; task quality, actual E0 outputs "
            "and ANN recall not evaluated"
        ),
        "transform": "first1024; Float64 L2 norm/division; Float32 output",
        "source_content_digest": manifest["content_digest"],
        "numpy_version": np.__version__,
        "observations": [
            observe(args.generation / "operation_vectors.arrow", 256),
            observe(args.generation / "vectors.arrow", 256),
        ],
    }
    args.output.write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
