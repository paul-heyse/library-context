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
from dataclasses import dataclass
from typing import Literal

import bm25s
import numpy as np

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


def ranks(scores: dict[bytes, float], positive_only: bool) -> dict[bytes, int]:
    """1-based ranks by descending score, ties by brief id."""
    kept = [b for b, s in scores.items() if s > 0 or not positive_only]
    kept.sort(key=lambda b: (-scores[b], b))
    return {b: r + 1 for r, b in enumerate(kept)}


def vector_scores(matrix: np.ndarray, rows: list[bytes], query: np.ndarray) -> dict[bytes, float]:
    """Each brief's best cosine over its chunks (all vectors are unit vectors)."""
    cosines = matrix.astype(np.float64) @ query.astype(np.float64)
    best: dict[bytes, float] = {}
    for b, c in zip(rows, cosines.tolist(), strict=True):
        best[b] = max(best.get(b, -2.0), c)
    return best


@dataclass(frozen=True)
class Ranked:
    brief_id: bytes
    relevance: float
    rank_source: RankSource
    promoted: bool


def fuse(
    lexical: dict[bytes, int],
    vector: dict[bytes, int],
    promoted: set[bytes],
    limit: int,
) -> list[Ranked]:
    """RRF of the two rankings, promoted briefs first, at most `limit`."""
    fused = {
        b: sum(1.0 / (K + r[b]) for r in (lexical, vector) if b in r)
        for b in set(lexical) | set(vector) | promoted
    }
    for b in promoted:
        fused[b] += 1.0
    order = sorted(fused, key=lambda b: (-fused[b], b))[:limit]
    out = []
    for b in order:
        source: RankSource
        if b in promoted:
            source = "exact_symbol"
        elif b in lexical and b in vector:
            source = "hybrid"
        elif b in vector:
            source = "vector"
        else:
            source = "lexical"
        out.append(Ranked(b, round(fused[b], 6), source, b in promoted))
    return out
