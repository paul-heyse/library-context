"""Retrieval (DESIGN §11.2): our tokenizer, bm25s against a hand computation, fusion and ties."""

from __future__ import annotations

import math

import numpy as np
import pytest

from lctx_mcp.retrieval import K, Lexical, fuse_legs, identities, lexical_rank, tokenize


def test_tokens_are_lowercased_runs_of_letters_and_digits() -> None:
    assert tokenize("FastMCP.custom_route(v2)") == ["fastmcp", "custom", "route", "v2"]


def test_bm25_scores_are_the_lucene_formula() -> None:
    texts = ["register a tool tool", "expose a resource", "prompt template prompt"]
    docs = [tokenize(t) for t in texts]
    k1, b = 1.5, 0.75
    n = len(docs)
    avg = sum(len(d) for d in docs) / n

    def expected(query: list[str]) -> list[float]:
        out = []
        for d in docs:
            score = 0.0
            for t in query:
                df = sum(1 for x in docs if t in x)
                if df == 0:
                    continue
                idf = math.log(1 + (n - df + 0.5) / (df + 0.5))
                tf = d.count(t)
                score += idf * tf / (tf + k1 * (1 - b + b * len(d) / avg))
            out.append(score)
        return out

    lexical = Lexical(texts)
    for query in ("tool", "a tool", "prompt resource", "unknown words only", "Tool"):
        # `a` is in two of three briefs, so it still discriminates.
        got = lexical.scores(query).tolist()
        assert got == pytest.approx(expected(tokenize(query)), rel=1e-6), query


def test_a_word_every_brief_contains_does_not_vote() -> None:
    """Increment-1 deep review F2: with only a shared word, the lexical leg abstains, so the
    fused order is the vector leg's, not the briefs' lengths."""
    texts = ["register a tool", "register a resource with a long list of controls and words"]
    lexical = Lexical(texts)
    assert lexical.discriminating("register") == []
    assert lexical.scores("register").tolist() == [0.0, 0.0]
    assert lexical.discriminating("register tool") == ["tool"]
    a, b = b"\x01" * 16, b"\x02" * 16
    ids = [a, b]
    ids = np.array(ids, dtype="V16")
    lexical_ranks = lexical_rank(ids, lexical.scores("register"), ids)
    vector_ranks = np.array([b, a], dtype="V16")
    fused = fuse_legs([lexical_ranks, vector_ranks], promoted=identities([]), limit=2)
    assert [r[0] for r in fused] == [b.hex(), a.hex()]
    assert {r[2] for r in fused} == {"vector"}


def test_fusion_ranks_ties_by_id_and_promotes_exact_symbols() -> None:
    a, b, c = b"\x01" * 16, b"\x02" * 16, b"\x03" * 16
    ids = np.array([a, b, c], dtype="V16")
    lexical = lexical_rank(ids, np.array([2.0, 2.0, 0.0]), ids)
    assert lexical.tolist() == [a, b]
    vector = np.array([b, a, c], dtype="V16")
    fused = fuse_legs([lexical, vector], promoted=identities([]), limit=10)
    assert [r[0] for r in fused] == [a.hex(), b.hex(), c.hex()]
    assert fused[0][1] == round(1 / (K + 1) + 1 / (K + 2), 6)
    assert {r[2] for r in fused} == {"hybrid", "vector"}
    promoted = fuse_legs([lexical, vector], promoted=identities([c.hex()]), limit=2)
    assert promoted[0][0] == c.hex() and promoted[0][3]
    assert promoted[0][2] == "exact_symbol" and len(promoted) == 2
    # Two vector views do not imply a lexical contribution.
    vectors_only = fuse_legs([identities([]), vector, vector], identities([]), 3)
    assert {r[2] for r in vectors_only} == {"vector"}


def test_fusion_refuses_aggregate_budget_before_allocating(monkeypatch):
    import lctx_mcp.retrieval as retrieval

    monkeypatch.setattr(retrieval, "FUSION_BYTES", 100)
    with pytest.raises(ValueError, match="resource_refused"):
        fuse_legs([identities(["01" * 16])], identities([]), 1)
