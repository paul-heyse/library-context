"""The query embedder's conformance with the Rust client (DESIGN §11.1, E2), without a GPU."""

from __future__ import annotations

import json
import math
from dataclasses import FrozenInstanceError, replace
from pathlib import Path

import httpx2
import numpy as np
import pytest
from lctx_semantics import embedding_recipe_identities

from lctx_mcp import embedder as embedder_module
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


def test_one_rust_recipe_response_and_decode_per_immutable_spec(monkeypatch) -> None:
    identities = embedder_module.embedding_recipe_identities
    decode = embedder_module.json.loads
    responses = []
    calls = []
    decoded = []

    def count_identities(canonical):
        calls.append(canonical)
        response = identities(canonical)
        responses.append(response)
        return response

    def count_decode(text, *args, **kwargs):
        if text in responses:
            decoded.append(text)
        return decode(text, *args, **kwargs)

    monkeypatch.setattr(embedder_module, "embedding_recipe_identities", count_identities)
    monkeypatch.setattr(embedder_module.json, "loads", count_decode)
    for name in (FAKE, QWEN):
        spec = Spec.packaged(name)
        expected = decode(responses[-1])
        for _ in range(3):
            assert spec.hash == expected["encoder"]
            assert spec.query_hash == expected["query"]
            assert spec.document_hash == expected["document"]
            assert spec.projection_id == expected["projection"]
            assert spec.recipe_version == expected["recipe_version"] == 1
        assert spec._identities.canonical == spec.canonical
    assert len(calls) == len(decoded) == 2
    assert calls[0] != calls[1]


def test_spec_freezes_nested_fields_and_refuses_bypass_construction() -> None:
    spec = Spec.packaged(QWEN)
    expected = embedding_recipe_identities(spec.canonical)
    with pytest.raises(TypeError):
        spec.fields["model"] = "changed"
    with pytest.raises(TypeError):
        spec.fields["admission"]["use_activation"] = False
    with pytest.raises(TypeError):
        spec.fields["admission"]["matryoshka_dimensions"][0] = 1
    with pytest.raises(FrozenInstanceError):
        spec.canonical = "{}"
    with pytest.raises(FrozenInstanceError):
        spec._identities.encoder = "changed"
    assert not hasattr(spec, "__dict__")
    with pytest.raises(TypeError, match=r"Spec\.from_json"):
        Spec(spec.canonical, {"model": "inconsistent"})
    with pytest.raises(TypeError, match=r"Spec\.from_json"):
        Spec()
    with pytest.raises(TypeError, match=r"Spec\.from_json"):
        replace(spec, canonical="{}")
    assert embedding_recipe_identities(spec.canonical) == expected
    assert spec.query_template == spec.fields["query_template"]
    assert spec.query_task == spec.fields["query_task"]
    assert spec.max_query_tokens == spec.fields["max_query_tokens"]


def test_spec_canonicalization_errors_precede_identity_preparation(monkeypatch) -> None:
    calls = []

    def unexpected_identity(canonical):
        calls.append(canonical)
        raise AssertionError("identity preparation before canonicalization")

    monkeypatch.setattr(embedder_module, "embedding_recipe_identities", unexpected_identity)
    with pytest.raises(ValueError, match="invalid canonical embedding specification"):
        Spec.from_json("{}")
    with pytest.raises(ValueError, match="invalid canonical embedding specification"):
        Spec.from_json("not JSON")
    assert calls == []


def test_spec_defensively_owns_every_decoded_field_alias(monkeypatch) -> None:
    canonical = embedder_module.canonical_embedding_spec((SPECS / QWEN).read_text())
    decode = json.loads
    fields = decode(canonical)

    def shared_decode(text, *args, **kwargs):
        return fields if text == canonical else decode(text, *args, **kwargs)

    monkeypatch.setattr(embedder_module.json, "loads", shared_decode)
    spec = Spec.from_json(canonical)
    expected = (spec.model, spec.fields["admission"]["matryoshka_dimensions"], spec.hash)
    fields["model"] = "changed"
    fields["admission"]["matryoshka_dimensions"][0] = 1
    fields["admission"]["use_activation"] = False
    assert (spec.model, spec.fields["admission"]["matryoshka_dimensions"], spec.hash) == expected
    assert spec.canonical == canonical


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
    assert not client.client.is_closed
    await client.aclose()

    def refuse(_request: httpx2.Request) -> httpx2.Response:
        raise httpx2.ConnectError("connection refused")

    down = HttpEmbedder("http://embed.invalid", transport=httpx2.MockTransport(refuse))
    with pytest.raises(EmbedderError, match="no embedding service"):
        await down.embed([text])
    await down.aclose()


@pytest.mark.anyio
async def test_shared_http_lifetime_retains_stream_bytes_and_closes_once() -> None:
    """Exercise the installed HTTPX2 client/transport lifetime without a live service."""
    spec = Spec.packaged(QWEN)
    answered = response(spec, [unit(spec.dimensions)])

    class Transport(httpx2.MockTransport):
        closes = 0

        async def aclose(self):
            self.closes += 1
            await super().aclose()

    transport = Transport(lambda request: httpx2.Response(200, content=answered))
    embedder = HttpEmbedder("http://embed.invalid", transport=transport)
    charges = []
    await embedder.embed(["first"], retain=charges.append)
    await embedder.embed(["second"], retain=charges.append)
    assert transport.closes == 0 and not embedder.client.is_closed
    assert sum(charges) >= len(answered) * 4 + spec.dimensions * 4 * 2
    await embedder.aclose()
    await embedder.aclose()
    assert transport.closes == 1 and embedder.client.is_closed


def test_responses_are_judged_as_the_shared_corpus_says() -> None:
    """The holistic assessment's A7: Python's strict parse accepts or rejects each body of the
    shared corpus as Rust's serde parse does (a boolean is never an index or a component)."""
    corpus = json.loads((SPECS / "responses.json").read_text(encoding="utf-8"))
    fields = {
        **Spec.packaged(QWEN).fields,
        "model": corpus["model"],
        "dimensions": corpus["dimensions"],
        "source_dimensions": corpus["dimensions"],
        "admission": {
            "is_matryoshka": True,
            "matryoshka_dimensions": [corpus["dimensions"]],
            "use_activation": True,
        },
    }
    spec = Spec.from_json(json.dumps(fields))
    for case in corpus["cases"]:
        try:
            parse_embeddings(spec, case["inputs"], case["body"].encode())
            accepted = True
        except EmbedderError:
            accepted = False
        assert accepted == case["accept"], case["name"]
