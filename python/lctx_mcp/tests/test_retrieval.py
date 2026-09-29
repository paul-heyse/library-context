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


def test_family_fusion_has_equal_family_weight_and_keeps_both_actual_units():
    from lctx_mcp.retrieval import fuse_families

    a, b, c = (f"{i:02x}" * 16 for i in (1, 2, 3))

    def row(member, family, channel, rank, unit):
        return dict(
            member_id=member,
            family=family,
            channel=channel,
            rank=rank,
            score=0.5,
            unit_id=unit,
            fragment_id=unit,
        )

    rows = [
        row(a, "api_options", "lexical", 1, a),
        row(a, "api_options", "vector", 2, c),
        row(b, "api_options", "lexical", 2, b),
        row(b, "api_options", "vector", 1, b),
        row(b, "source", "vector", 1, c),
    ]
    result = fuse_families(rows, set())
    assert [r["member_id"] for r in result] == [b, a]
    assert result[0]["score"] == pytest.approx(1 / 62 + 1 / 61)
    assert result[1]["score"] == pytest.approx(1 / 61)
    assert {w["unit_id"] for w in result[1]["winners"]} == {a, c}
    assert fuse_families(list(reversed(rows)), set()) == result
    assert fuse_families(rows, {c})[0] == dict(member_id=c, score=0.0, promoted=True, winners=[])


def test_duplicate_documents_leave_corpus_scores_stable_and_keep_member_owned_occurrence():
    import pyarrow as pa

    from lctx_mcp.retrieval import UnitLexical

    a, b, c = (bytes([i]) * 16 for i in (1, 2, 3))

    def index(duplicate):
        fragments = [
            dict(unit_id=a, fragment_id=a, family="scenario", text="register tool"),
            dict(unit_id=b, fragment_id=b, family="scenario", text="read resource"),
        ]
        subjects = [dict(unit_id=a, member_id=a), dict(unit_id=b, member_id=b)]
        if duplicate:
            fragments.append(
                dict(unit_id=c, fragment_id=c, family="scenario", text="register tool")
            )
            subjects.append(dict(unit_id=c, member_id=c))
        return UnitLexical(pa.Table.from_pylist(fragments), pa.Table.from_pylist(subjects))

    original = index(False).winners("tool", {a.hex(), b.hex()})
    duplicated = index(True).winners("tool", {a.hex(), b.hex()})
    assert original == duplicated
    alias = index(True).winners("tool", {c.hex()})
    assert alias[0]["member_id"] == c.hex() and alias[0]["unit_id"] == c.hex()
    assert alias[0]["score"] == original[0]["score"]


def test_independent_units_include_release_only_and_unassociated_text():
    import pyarrow as pa

    from lctx_mcp.retrieval import UnitLexical

    fragments = [
        {
            "unit_id": bytes([i]) * 16,
            "fragment_id": bytes([i]) * 16,
            "family": "documentation_deployment",
            "text": "install deployment server",
        }
        for i in [1, 2, 3]
    ]
    subjects = [
        {"unit_id": bytes([1]) * 16, "member_id": bytes([7]) * 16},
        {"unit_id": bytes([2]) * 16, "member_id": None},
    ]
    index = UnitLexical(pa.Table.from_pylist(fragments), pa.Table.from_pylist(subjects))
    winners = index.unit_winners("deployment", ["documentation_deployment"])
    assert {w["unit_id"] for w in winners} == {bytes([i]).hex() * 16 for i in [1, 2, 3]}
    assert len({w["score"] for w in winners}) == 1
    repeated = UnitLexical(pa.Table.from_pylist(fragments * 5), pa.Table.from_pylist(subjects))
    assert repeated.unit_winners("deployment", ["documentation_deployment"]) == winners
