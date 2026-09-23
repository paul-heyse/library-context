"""The live leg of the client conformance check (DESIGN §11.1, E2): the Python query client's
vectors for the conformance inputs agree with the Rust client's to cosine >= 0.9995.

Usage: embed_conformance.py RUST_VECTORS.json [--url URL]   Exit 1 on a disagreement.
"""

from __future__ import annotations

import argparse
import asyncio
import json
from pathlib import Path

import numpy as np

from lctx_mcp.embedder import HttpEmbedder

ROOT = Path(__file__).resolve().parent.parent
THRESHOLD = 0.9995


async def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("rust", type=Path)
    parser.add_argument("--url", default="http://127.0.0.1:8000")
    args = parser.parse_args()
    rust = json.loads(args.rust.read_text())
    inputs = json.loads((ROOT / "specs/embedding/conformance_inputs.json").read_text())
    client = HttpEmbedder(args.url, timeout=120.0)
    worst = 1.0
    for item in inputs:
        text = item["text"]
        request = (
            client.spec.query_text(text)
            if item["kind"] == "query"
            else client.spec.document_text(text)
        )
        (py,) = await client.embed([request])
        rs = np.asarray(rust[item["id"]], dtype=np.float64)
        cos = float(np.dot(py.astype(np.float64), rs) / (np.linalg.norm(py) * np.linalg.norm(rs)))
        worst = min(worst, cos)
        print(f"{item['id']}: cosine {cos:.7f}")
    ok = worst >= THRESHOLD
    print(f"embed-conformance: {'passed' if ok else 'failed'} (worst cosine {worst:.7f})")
    return 0 if ok else 1


if __name__ == "__main__":
    raise SystemExit(asyncio.run(main()))
