"""Hybrid retrieval over one generation (DESIGN §11.2).

1. Lexical: BM25 (bm25s, numpy backend, `get_scores`) over `lexical_text`, with our tokenizer,
   over the query's **discriminating** words only: a word every brief contains cannot tell them
   apart, and a ranking by it alone is BM25's length normalization. With none, the lexical leg
   abstains (ADR-0010 amendment, increment-1 deep review F2).
2. Vector: exact cosine against the instruction-prefixed query vector; a brief's score is its best
   chunk's.
3. Fusion: reciprocal-rank fusion, K = 60, 1-based ranks, ties by brief id.
4. Exact symbols: a query that is a public access path promotes its briefs, recorded as promoted.
A score ranks briefs; it is never evidence that a brief fits the task.
"""

from __future__ import annotations

import re
from typing import Literal

import bm25s
import numpy as np
import pyarrow as pa
import pyarrow.ipc as ipc
from pydantic import BaseModel

K = 60
RankSource = Literal["hybrid", "lexical", "vector", "exact_symbol"]
TOKEN = re.compile(r"[a-z0-9]+")


def tokenize(text: str) -> list[str]:
    """Lower-cased runs of letters and digits: `custom_route` is `custom route`."""
    return TOKEN.findall(text.lower())


class Lexical:
    """A BM25 index of the briefs' lexical texts, in brief order."""

    def __init__(self, texts: list[str]) -> None:
        self.retriever = bm25s.BM25(k1=1.5, b=0.75, method="lucene", backend="numpy")
        tokens = [tokenize(t) for t in texts]
        self.retriever.index(tokens, show_progress=False)
        self.size = len(texts)
        self.df: dict[str, int] = {}
        for doc in tokens:
            for t in set(doc):
                self.df[t] = self.df.get(t, 0) + 1

    def discriminating(self, query: str) -> list[str]:
        """The query's words some briefs contain and others do not, in query order."""
        return [t for t in tokenize(query) if 0 < self.df.get(t, 0) < self.size]

    def scores(self, query: str) -> np.ndarray:
        """One score per brief over the discriminating words; all zero when there are none."""
        tokens = self.discriminating(query)
        if not tokens:
            return np.zeros(self.size, dtype=np.float32)
        return np.asarray(self.retriever.get_scores(tokens), dtype=np.float32)


FUSION_BYTES = 128 * 1024 * 1024
RANK_BYTES = 32 * 1024 * 1024


class RetrievalMetadata(BaseModel):
    profile: str
    requested_route: Literal["exact", "hnsw", "mixed"]
    actual_route: Literal["exact", "hnsw", "lexical-only"]
    fallback: str | None = None
    candidate_depth: int = 0
    approximate: bool = False
    routing_reason: str = "explicit_profile"
    admission: str | None = None


def identities(values: list[str]) -> np.ndarray:
    return np.frombuffer(b"".join(bytes.fromhex(value) for value in values), dtype="V16")


def lexical_rank(ids: np.ndarray, scores: np.ndarray, eligible: np.ndarray) -> np.ndarray:
    keep = (scores > 0) & np.isin(ids, eligible, assume_unique=True)
    ids, scores = ids[keep], scores[keep]
    return ids[np.lexsort((ids, -scores))]


def vector_legs(raw: bytes) -> list[np.ndarray]:
    if len(raw) > RANK_BYTES:
        raise ValueError("resource_refused: rank IPC byte budget")
    table = ipc.open_stream(pa.BufferReader(raw)).read_all()
    expected = pa.schema(
        [
            pa.field("entity_id", pa.binary(16), False),
            pa.field("view", pa.string(), False),
            pa.field("rank", pa.uint32(), False),
            pa.field("score", pa.float64(), False),
        ]
    )
    if table.schema != expected:
        raise ValueError("corrupt: rank schema")
    ids = np.frombuffer(b"".join(table.column("entity_id").to_pylist()), dtype="V16")
    views = table.column("view").to_numpy()
    ranks = table.column("rank").to_numpy()
    result = []
    for view in sorted(set(views)):
        selected = np.flatnonzero(views == view)
        selected = selected[np.argsort(ranks[selected])]
        if not np.array_equal(ranks[selected], np.arange(1, len(selected) + 1)):
            raise ValueError("corrupt: non-contiguous vector ranks")
        result.append(ids[selected])
    return result


def fuse_legs(
    legs: list[np.ndarray], promoted: np.ndarray, limit: int
) -> list[tuple[str, float, str, bool]]:
    """Compact RRF over complete exact ranks or explicitly labelled candidate ranks.

    The conservative ledger covers all arrays and sorting temporaries simultaneously. It is
    checked before concatenation; no full-universe Python score/rank dictionaries are allocated.
    """
    pairs = sum(len(leg) for leg in legs) + len(promoted)
    if pairs * 160 > FUSION_BYTES:
        raise ValueError("resource_refused: aggregate fusion byte budget")
    if not pairs:
        return []
    universe = np.unique(np.concatenate([*legs, promoted]))
    scores = np.zeros(len(universe), dtype=np.float64)
    masks = np.zeros(len(universe), dtype=np.uint8)
    for i, leg in enumerate(legs):
        indices = np.searchsorted(universe, leg)
        scores[indices] += 1.0 / (K + np.arange(1, len(leg) + 1, dtype=np.float64))
        masks[indices] |= 1 << i
    is_promoted = np.isin(universe, promoted, assume_unique=True)
    scores[is_promoted] += 1.0
    order = np.lexsort((universe, -scores))[:limit]
    out = []
    for i in order:
        mask = int(masks[i])
        source = (
            "exact_symbol"
            if is_promoted[i]
            else "hybrid"
            if mask & 1 and mask & ~1
            else "lexical"
            if mask == 1
            else "vector"
        )
        out.append(
            (universe[i].tobytes().hex(), round(float(scores[i]), 6), source, bool(is_promoted[i]))
        )
    return out
