"""Basic live embedding contract check; no accuracy or ranking assessment."""

import asyncio
import json
import sys
from urllib.request import Request, urlopen

import numpy as np

from lctx_mcp.embedder import HttpEmbedder


async def main() -> None:
    url = sys.argv[1] if len(sys.argv) > 1 else "http://127.0.0.1:8000"
    embedder = HttpEmbedder(url, timeout=60.0)
    spec = embedder.spec
    with urlopen(url + "/health", timeout=10) as response:
        assert response.status == 200
    with urlopen(url + "/v1/models", timeout=10) as response:
        models = json.load(response)
    assert spec.model in {model["id"] for model in models["data"]}
    texts = [
        spec.document_text("Register a Python function as a FastMCP tool."),
        spec.document_text("Configure authentication for the HTTP transport."),
        spec.query_text("How do I register a tool?"),
    ]
    counts = []
    for text in texts:
        request = Request(
            url + "/tokenize",
            data=json.dumps({"model": spec.model, "prompt": text}).encode(),
            headers={"content-type": "application/json"},
        )
        with urlopen(request, timeout=10) as response:
            counts.append(json.load(response)["count"])
    assert all(0 < count <= spec.fields["max_document_tokens"] for count in counts)
    # The production client checks model, count, indices, width, finiteness and L2 norm.
    vectors = await embedder.embed(texts)
    assert vectors.shape == (3, 1024)
    print(json.dumps({
        "outcome": "passed",
        "scope": "startup, model identity, tokenizer and production HTTP vector contract",
        "spec_hash": spec.hash,
        "shape": list(vectors.shape),
        "token_counts": counts,
        "norms": np.linalg.norm(vectors.astype(np.float64), axis=1).tolist(),
        "accuracy_assessment": "not_run: operator instruction",
    }, indent=2))


if __name__ == "__main__":
    asyncio.run(main())
