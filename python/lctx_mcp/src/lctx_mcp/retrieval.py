"""Numerical BM25S adapter over Rust-supplied documents and query tokens.

Rust owns tokenization, corpus identity, document frequency policy, eligibility, occurrence
joins and fusion. This callback returns complete numerical document scores, including zeros.
Its initialization and calls run under the native service's actual execution grant.
"""
from __future__ import annotations

import json
from importlib.metadata import version

import bm25s
import numpy as np


class NumericalScorer:
    """One library index per nonempty family; no Python semantic or ranking state."""

    def __init__(self) -> None:
        self._families: dict[object, tuple[object, list[list[int]]]] = {}
        self._initialized = False

    def initialize(self, corpus_json: str) -> None:
        corpus = json.loads(corpus_json)
        settings = corpus["policy"]["numerical"]
        if settings["library"] != "bm25s0311" or version("bm25s") != "0.3.11":
            raise ValueError("unsupported numerical library")
        grouped: dict[object, list[dict]] = {}
        for document in corpus["documents"]:
            grouped.setdefault(document["family"], []).append(document)
        families = {}
        for family, documents in grouped.items():
            index = bm25s.BM25(
                k1=settings["k1"], b=settings["b"],
                method=settings["method"], backend=settings["backend"],
            )
            supplied = [document["tokens"] for document in documents]
            # BM25S0.3.11 cannot index an entirely empty vocabulary. There is no numerical
            # term work in that family; retain its document identities and explicit zero scores.
            if any(supplied):
                index.index(supplied, show_progress=False)
            else:
                index = None
            families[family] = index, [document["id"] for document in documents]
        self._families = families
        self._initialized = True

    def __call__(self, query_json: str) -> str:
        if not self._initialized:
            raise RuntimeError("numerical index has not been initialized")
        tokens = dict(json.loads(query_json)["tokens"])
        result = []
        for family, (index, identities) in self._families.items():
            supplied = tokens.get(family, [])
            if index is None:
                scores = np.zeros(len(identities), dtype=np.float32)
            else:
                # get_scores([]) indexes element0 at this pin; its public ID entrypoint
                # performs the same numerical calculation for an empty query safely.
                scores = index.get_scores(supplied) if supplied else index.get_scores_from_ids([])
            result.extend(
                {"document": identity, "score": float(score)}
                for identity, score in zip(identities, scores, strict=True)
            )
        return json.dumps(result, separators=(",", ":"), allow_nan=False)
