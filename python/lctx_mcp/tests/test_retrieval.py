"""Independent numerical controls; semantic tokenization and fusion live in Rust tests."""

import json
import math

import pytest

from lctx_mcp.retrieval import NumericalScorer


def corpus(documents):
    return {
        "policy": {
            "numerical": {
                "library": "bm25s0311",
                "backend": "numpy",
                "method": "lucene",
                "k1": 1.5,
                "b": 0.75,
            }
        },
        "documents": documents,
    }


def document(n, family, tokens):
    return {
        "id": [n] * 32,
        "family": family,
        "text": "deliberately unrelated to supplied tokens",
        "tokens": tokens,
    }


def test_bm25_scores_are_the_independent_lucene_formula():
    docs = [
        ["register", "a", "tool", "tool"],
        ["expose", "a", "resource"],
        ["prompt", "template", "prompt"],
    ]
    scorer = NumericalScorer()
    scorer.initialize(
        json.dumps(corpus([document(i + 1, "source", tokens) for i, tokens in enumerate(docs)]))
    )
    average = sum(map(len, docs)) / len(docs)
    for terms in [["tool"], ["a", "tool"], ["prompt", "resource"], [], ["unseen"]]:
        expected = []
        for doc in docs:
            value = 0.0
            for term in terms:
                frequency = sum(term in d for d in docs)
                if not frequency:
                    continue
                idf = math.log(1 + (len(docs) - frequency + 0.5) / (frequency + 0.5))
                count = doc.count(term)
                value += idf * count / (count + 1.5 * (1 - 0.75 + 0.75 * len(doc) / average))
            expected.append(value)
        rows = json.loads(scorer(json.dumps({"tokens": [["source", terms]]})))
        assert [row["document"] for row in rows] == [[i] * 32 for i in [1, 2, 3]]
        assert [row["score"] for row in rows] == pytest.approx(expected, rel=1e-6)


def test_family_indexes_share_no_numeric_statistics_and_return_all_documents():
    scorer = NumericalScorer()
    scorer.initialize(
        json.dumps(
            corpus(
                [
                    document(1, "source", ["tool"]),
                    document(2, "source", ["resource"]),
                    document(3, "scenario", ["tool"]),
                ]
            )
        )
    )
    rows = json.loads(
        scorer(json.dumps({"tokens": [["source", ["tool"]], ["scenario", ["tool"]]]}))
    )
    assert {tuple(row["document"]): row["score"] for row in rows} == pytest.approx(
        {
            tuple([1] * 32): math.log(2) / 2.5,
            tuple([2] * 32): 0.0,
            tuple([3] * 32): math.log(4 / 3) / 2.5,
        }
    )


def test_empty_corpus_and_zero_scores_remain_explicit():
    scorer = NumericalScorer()
    scorer.initialize(json.dumps(corpus([])))
    assert json.loads(scorer('{"tokens":[]}')) == []
    scorer.initialize(json.dumps(corpus([document(1, "source", [])])))
    assert json.loads(scorer('{"tokens":[["source",[]]]}')) == [
        {"document": [1] * 32, "score": 0.0}
    ]
    assert json.loads(scorer('{"tokens":[["source",["absent"]]]}')) == [
        {"document": [1] * 32, "score": 0.0}
    ]


def test_uninitialized_numerical_callback_refuses():
    with pytest.raises(RuntimeError, match="not been initialized"):
        NumericalScorer()('{"tokens":[]}')
