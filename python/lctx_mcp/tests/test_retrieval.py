"""Retrieval (DESIGN §11.2): our tokenizer, bm25s against a hand computation, fusion and ties."""

from __future__ import annotations

import math

import pytest

from lctx_mcp.retrieval import K, Lexical, fuse, ranks, tokenize


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
    lexical_ranks = ranks(dict(zip(ids, lexical.scores("register").tolist(), strict=True)), True)
    vector_ranks = ranks({a: 0.2, b: 0.9}, positive_only=False)
    fused = fuse(lexical_ranks, vector_ranks, promoted=set(), limit=2)
    assert [r.brief_id for r in fused] == [b, a]
    assert {r.rank_source for r in fused} == {"vector"}


def test_fusion_ranks_ties_by_id_and_promotes_exact_symbols() -> None:
    a, b, c = b"\x01" * 16, b"\x02" * 16, b"\x03" * 16
    lexical = ranks({a: 2.0, b: 2.0, c: 0.0}, positive_only=True)
    assert lexical == {a: 1, b: 2}
    vector = ranks({a: 0.1, b: 0.9, c: -0.5}, positive_only=False)
    assert vector == {b: 1, a: 2, c: 3}
    fused = fuse(lexical, vector, promoted=set(), limit=10)
    # a and b fuse to equal scores (ranks 1 and 2 each way): the tie goes to the lower id.
    assert [r.brief_id for r in fused] == [a, b, c]
    assert fused[0].relevance == round(1 / (K + 1) + 1 / (K + 2), 6)
    assert {r.rank_source for r in fused} == {"hybrid", "vector"}
    promoted = fuse(lexical, vector, promoted={c}, limit=2)
    assert promoted[0].brief_id == c and promoted[0].promoted
    assert promoted[0].rank_source == "exact_symbol" and len(promoted) == 2
