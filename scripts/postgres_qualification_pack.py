"""Freeze a bounded independent f64 cosine reference before ANN measurements.

Requests are JSON objects with query, operations, stratum, and optional Where. The bundle is
validated offline, eligibility comes from the exact serving repository, and only this oracle
loads canonical vectors into Python. Online serving never imports this module.
"""

from __future__ import annotations

import argparse
import asyncio
import hashlib
import json
from pathlib import Path

import numpy as np

from lctx_mcp.embedder import FakeEmbedder, HttpEmbedder
from lctx_mcp.retrieval import identities, lexical_rank
from postgres_session import DEFAULT_CONFIG, session


async def build(args):
    embedder = FakeEmbedder() if args.embedder == "fake" else HttpEmbedder(args.embed_url)
    requests = json.loads(args.requests.read_text())
    if not 1 <= len(requests) <= 64:
        raise ValueError("one through 64 preregistered queries required")
    async with session(args.bundle, embedder, args.config) as (served, reference):
        gen = served.generation
        cases = []
        for request in requests:
            operations = request["operations"]
            where = json.dumps(request["where"]) if request.get("where") else None
            scope = json.loads(
                await gen.repository.search_scope(request["query"], operations, where)
            )
            table = reference.table("operation_vectors" if operations else "vectors")
            vector = np.asarray(
                (await embedder.embed([embedder.spec.query_text(request["query"])]))[0],
                dtype=np.float32,
            )
            matrix = (
                table.column("vector")
                .combine_chunks()
                .flatten()
                .to_numpy()
                .reshape(-1, 1024)
                .astype(np.float64)
            )
            q = vector.astype(np.float64)
            scores = (matrix @ q) / (np.linalg.norm(matrix, axis=1) * np.linalg.norm(q))
            nodes = table.column("node_id" if operations else "brief_id").to_pylist()
            views = (
                table.column("embedding_view").to_pylist() if operations else ["brief"] * len(nodes)
            )
            allowed = set(scope["eligible"])
            expected = {
                view: {} for view in (["signature_doc", "source_body"] if operations else ["brief"])
            }
            for identity, view, score in zip(nodes, views, scores, strict=True):
                identity = identity.hex()
                if identity in allowed:
                    expected[view][identity] = max(expected[view].get(identity, -2.0), float(score))
            expected = {
                view: [
                    {"id": key, "score": values[key]}
                    for key in sorted(values, key=lambda key: (-values[key], key))
                ]
                for view, values in expected.items()
            }
            lexical = served.operation_lexical if operations else served.lexical
            state = gen.lexical_state
            ids = state.operation_ids if operations else state.brief_ids
            ranks = (
                lexical_rank(ids, lexical.scores(request["query"]), identities(scope["eligible"]))
                if lexical
                else []
            )
            cases.append(
                dict(
                    name=request["name"],
                    stratum=request["stratum"],
                    operations=operations,
                    vector=vector.tolist(),
                    eligible=scope["eligible"],
                    promoted=scope["promoted"],
                    lexical=[r.tobytes().hex() for r in ranks],
                    reference=expected,
                )
            )
        pack = dict(
            format=1 if args.phase == "legacy" else 2,
            generation=gen.key,
            spec=gen.spec_hash,
            maximum_ann_p95_ms=args.maximum_ann_p95_ms,
            cases=cases,
        )
        if args.phase != "legacy":
            pack["phase"] = args.phase
            pack["policy"] = None
            pack["calibration_sha256"] = None
            if args.phase == "confirmation":
                if args.calibration is None:
                    raise ValueError("confirmation requires a calibration report")
                calibration = json.loads(args.calibration.read_text())
                if calibration.get("phase") != "calibration" or not calibration.get(
                    "chosen_policy"
                ):
                    raise ValueError("calibration did not admit a candidate policy")
                pack["policy"] = calibration["chosen_policy"]
                pack["calibration_sha256"] = calibration["pack_sha256"]
        raw = json.dumps(pack, separators=(",", ":")).encode()
        if len(raw) > 128 * 1024 * 1024:
            raise ValueError("qualification pack exceeds 128 MiB")
        # Never silently overwrite a frozen reference after measurements.
        with args.out.open("xb") as output:
            output.write(raw)
        print(
            f"Frozen {len(cases)} cases for {gen.key}; {len(raw)} bytes; "
            f"{args.embedder} embeddings; sha256={hashlib.sha256(raw).hexdigest()}"
        )


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("bundle", type=Path)
    parser.add_argument("requests", type=Path)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--config", type=Path, default=DEFAULT_CONFIG)
    parser.add_argument("--embedder", choices=["fake", "vllm"], default="vllm")
    parser.add_argument("--embed-url", default="http://127.0.0.1:8000")
    parser.add_argument("--maximum-ann-p95-ms", type=float, default=250.0)
    parser.add_argument(
        "--phase", choices=["calibration", "confirmation", "legacy"], default="calibration"
    )
    parser.add_argument("--calibration", type=Path)
    asyncio.run(build(parser.parse_args()))


if __name__ == "__main__":
    main()
