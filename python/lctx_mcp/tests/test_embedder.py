"""The query embedder's conformance with the Rust client (DESIGN §11.1, E2), without a GPU."""

from __future__ import annotations

import json
import math
from pathlib import Path

import httpx2
import numpy as np
import pytest

from lctx_mcp.embedder import (
    FAKE,
    QWEN,
    EmbedderError,
    FakeEmbedder,
    HttpEmbedder,
    Spec,
    fake_vector,
    parse_embeddings,
    request_body,
)

REPO = Path(__file__).resolve().parents[3]

SPECS = REPO / "specs" / "embedding"
INPUTS = json.loads((SPECS / "conformance_inputs.json").read_text(encoding="utf-8"))


def request_text(spec: Spec, item: dict) -> str:
    return (
        spec.query_text(item["text"])
        if item["kind"] == "query"
        else spec.document_text(item["text"])
    )


def test_the_packaged_specs_are_the_committed_ones() -> None:
    for name in (QWEN, FAKE):
        committed = (SPECS / name).read_text(encoding="utf-8")
        spec = Spec.packaged(name)
        assert spec.canonical + "\n" == committed, name


def test_request_bodies_are_byte_identical_to_rusts() -> None:
    spec = Spec.packaged(QWEN)
    bodies = json.loads((SPECS / "request_bodies.json").read_text(encoding="utf-8"))
    for item in INPUTS:
        body = request_body(spec, [request_text(spec, item)])
        assert body == bodies[item["id"]].encode(), item["id"]


def test_the_fake_twin_reproduces_rusts_vectors() -> None:
    fake = FakeEmbedder()
    answers = json.loads((SPECS / "fake_vectors.json").read_text(encoding="utf-8"))
    for item in INPUTS:
        request = request_text(fake.spec, item)
        assert request == answers[item["id"]]["request"]
        head = fake_vector(request, fake.spec.dimensions)[:16]
        assert head.tolist() == [float(np.float32(x)) for x in answers[item["id"]]["head"]]


def response(spec: Spec, vectors: list[list[float]], **overrides: object) -> bytes:
    body: dict[str, object] = {
        "model": spec.model,
        "data": [{"index": i, "embedding": v} for i, v in enumerate(vectors)],
    }
    body.update(overrides)
    return json.dumps(body).encode()


def unit(n: int) -> list[float]:
    return [1.0] + [0.0] * (n - 1)


def test_every_rejection_fires() -> None:
    spec = Spec.packaged(QWEN)
    d = spec.dimensions
    good = response(spec, [unit(d)])
    assert parse_embeddings(spec, 1, good).shape == (1, d)
    cases = {
        "model": response(spec, [unit(d)], model="other"),
        "count": response(spec, [unit(d), unit(d)]),
        "index": json.dumps(
            {"model": spec.model, "data": [{"index": 3, "embedding": unit(d)}]}
        ).encode(),
        "twice": json.dumps(
            {
                "model": spec.model,
                "data": [{"index": 0, "embedding": unit(d)}, {"index": 0, "embedding": unit(d)}],
            }
        ).encode(),
        "length": response(spec, [unit(d - 1)]),
        "finite": response(spec, [[math.inf] + [0.0] * (d - 1)]),
        "norm": response(spec, [[0.5] + [0.0] * (d - 1)]),
    }
    for name, body in cases.items():
        n = 2 if name in ("twice",) else 1
        with pytest.raises(EmbedderError):
            parse_embeddings(spec, n, body)
        del name


@pytest.mark.anyio
async def test_the_http_client_sends_the_exact_bytes_and_reports_a_down_service() -> None:
    spec = Spec.packaged(QWEN)
    seen: list[bytes] = []

    def handler(request: httpx2.Request) -> httpx2.Response:
        seen.append(request.content)
        return httpx2.Response(200, content=response(spec, [unit(spec.dimensions)]))

    client = HttpEmbedder("http://embed.invalid", transport=httpx2.MockTransport(handler))
    text = spec.query_text("résumé naïve café — non-ASCII query 日本語")
    vectors = await client.embed([text])
    assert vectors.shape == (1, spec.dimensions)
    assert seen == [request_body(spec, [text])]

    def refuse(_request: httpx2.Request) -> httpx2.Response:
        raise httpx2.ConnectError("connection refused")

    down = HttpEmbedder("http://embed.invalid", transport=httpx2.MockTransport(refuse))
    with pytest.raises(EmbedderError, match="no embedding service"):
        await down.embed([text])


def test_responses_are_judged_as_the_shared_corpus_says() -> None:
    """The holistic assessment's A7: Python's strict parse accepts or rejects each body of the
    shared corpus as Rust's serde parse does (a boolean is never an index or a component)."""
    corpus = json.loads((SPECS / "responses.json").read_text(encoding="utf-8"))
    fields = {
        **Spec.packaged(QWEN).fields,
        "model": corpus["model"],
        "dimensions": corpus["dimensions"],
    }
    spec = Spec.from_json(json.dumps(fields))
    for case in corpus["cases"]:
        try:
            parse_embeddings(spec, case["inputs"], case["body"].encode())
            accepted = True
        except EmbedderError:
            accepted = False
        assert accepted == case["accept"], case["name"]
