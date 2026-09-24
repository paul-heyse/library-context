"""The query-time embedder (DESIGN §11.1; ADR-0010).

The request bytes are built here exactly as the Rust client (`lctx-embed`) builds them, and both
are held to `specs/embedding/request_bodies.json`. So the body is sent as bytes, never through
httpx2's `json=`, which escapes non-ASCII. The fake twin reproduces the Rust `FakeEmbedder` bit for
bit (`specs/embedding/fake_vectors.json`).
"""

from __future__ import annotations

import hashlib
import json
import math
from dataclasses import dataclass
from importlib import resources
from typing import Protocol

import httpx2
import numpy as np
from pydantic import BaseModel, ConfigDict
from pydantic import ValidationError as PydanticValidationError

MASK = (1 << 64) - 1


class EmbedderError(RuntimeError):
    """The query could not be embedded: the service is down, or its answer was rejected."""


@dataclass(frozen=True)
class Spec:
    """An embedding spec: its canonical JSON (field order as Rust declares it) and fields."""

    canonical: str
    fields: dict

    @classmethod
    def from_json(cls, text: str) -> Spec:
        fields = json.loads(text)
        canonical = json.dumps(fields, separators=(",", ":"), ensure_ascii=False)
        return cls(canonical, fields)

    @classmethod
    def packaged(cls, name: str) -> Spec:
        text = resources.files("lctx_mcp").joinpath("specs", name).read_text(encoding="utf-8")
        return cls.from_json(text)

    @property
    def hash(self) -> str:
        """The spec hash: SHA-256 of the canonical JSON."""
        return hashlib.sha256(self.canonical.encode()).hexdigest()

    @property
    def model(self) -> str:
        return self.fields["model"]

    @property
    def dimensions(self) -> int:
        return int(self.fields["dimensions"])

    def query_text(self, query: str) -> str:
        """`Instruct: {task}\\nQuery:{query}`, substituted in Rust's order."""
        template: str = self.fields["query_template"]
        return template.replace("{task_description}", self.fields["query_task"]).replace(
            "{query}", query
        )

    def document_text(self, text: str) -> str:
        template: str = self.fields["document_template"]
        return template.replace("{text}", text)


QWEN = "qwen3-embedding-8b.json"
FAKE = "lctx-fake-embedder.json"


def request_body(spec: Spec, texts: list[str]) -> bytes:
    """The embeddings request body, byte for byte the Rust client's."""
    body = {"model": spec.model, "input": texts, "encoding_format": "float"}
    return json.dumps(body, separators=(",", ":"), ensure_ascii=False).encode()


def check_vector(v: list[float], dimensions: int) -> None:
    """§11.1's rejections of one vector: length, finiteness, unit norm."""
    if len(v) != dimensions:
        raise EmbedderError(f"{len(v)} dimensions, not {dimensions}")
    if not all(math.isfinite(x) for x in v):
        raise EmbedderError("a non-finite component")
    norm = math.sqrt(math.fsum(float(np.float32(x)) ** 2 for x in v))
    if abs(norm - 1.0) > 1e-3:
        raise EmbedderError(f"norm {norm}, not 1")


class _Datum(BaseModel):
    """One answered input: strict, so a boolean is never an index or a component (A7)."""

    model_config = ConfigDict(strict=True)
    index: int
    embedding: list[float]


class _Embeddings(BaseModel):
    """An embeddings response as Rust's serde reads it: unknown fields ignored, types strict."""

    model_config = ConfigDict(strict=True)
    model: str
    data: list[_Datum]


def parse_embeddings(spec: Spec, n: int, body: bytes) -> np.ndarray:
    """Parse and check an embeddings response for `n` inputs (§11.1's rejections). The shape is a
    pydantic strict model, as Rust's is a serde type, and both are held to one corpus
    (`specs/embedding/responses.json`; the holistic assessment's A7).
    """
    try:
        parsed = _Embeddings.model_validate_json(body)
    except PydanticValidationError as e:
        raise EmbedderError(f"a malformed response: {e.error_count()} errors") from e
    if parsed.model != spec.model:
        raise EmbedderError(f"the service answered with model {parsed.model}")
    if len(parsed.data) != n:
        raise EmbedderError(f"{len(parsed.data)} vectors for {n} inputs")
    out: list[list[float] | None] = [None] * n
    for d in parsed.data:
        if not 0 <= d.index < n:
            raise EmbedderError(f"index {d.index} out of range")
        if out[d.index] is not None:
            raise EmbedderError(f"index {d.index} answered twice")
        check_vector(d.embedding, spec.dimensions)
        out[d.index] = d.embedding
    return np.asarray(out, dtype=np.float32)


class Embedder(Protocol):
    spec: Spec

    async def embed(self, texts: list[str]) -> np.ndarray: ...


class HttpEmbedder:
    """The vLLM service's `/v1/embeddings`, under the committed Qwen spec."""

    def __init__(
        self,
        url: str,
        spec: Spec | None = None,
        timeout: float = 10.0,
        transport: httpx2.AsyncBaseTransport | None = None,
    ) -> None:
        self.url = url.rstrip("/")
        self.spec = spec or Spec.packaged(QWEN)
        self.timeout = timeout
        self.transport = transport

    async def embed(self, texts: list[str]) -> np.ndarray:
        url = f"{self.url}/v1/embeddings"
        try:
            async with httpx2.AsyncClient(timeout=self.timeout, transport=self.transport) as client:
                response = await client.post(
                    url,
                    content=request_body(self.spec, texts),
                    headers={"content-type": "application/json"},
                )
            response.raise_for_status()
        except httpx2.HTTPError as e:
            raise EmbedderError(f"no embedding service at {url}: {e!r}") from e
        return parse_embeddings(self.spec, len(texts), response.content)


def fake_vector(request_text: str, dimensions: int) -> np.ndarray:
    """The Rust fake embedder's vector: splitmix64 seeded by the text's SHA-256, L2-normalized."""
    seed = hashlib.sha256(request_text.encode()).digest()
    state = int.from_bytes(seed[:8], "little")
    raw = []
    for _ in range(dimensions):
        state = (state + 0x9E3779B97F4A7C15) & MASK
        z = state
        z = ((z ^ (z >> 30)) * 0xBF58476D1CE4E5B9) & MASK
        z = ((z ^ (z >> 27)) * 0x94D049BB133111EB) & MASK
        z ^= z >> 31
        raw.append((z >> 11) / float(1 << 53) * 2.0 - 1.0)
    norm = math.sqrt(sum(x * x for x in raw))
    return np.asarray([x / norm for x in raw], dtype=np.float32)


class FakeEmbedder:
    """The deterministic twin of `cpg_core::embed::FakeEmbedder`, for tests and `just check`."""

    def __init__(self) -> None:
        self.spec = Spec.packaged(FAKE)

    async def embed(self, texts: list[str]) -> np.ndarray:
        return np.stack([fake_vector(t, self.spec.dimensions) for t in texts])
