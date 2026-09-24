"""The serving tokenizer's known answers, shared with Rust (`specs/serving/tokens.json`)."""

from __future__ import annotations

import json
from pathlib import Path

from lctx_mcp.retrieval import tokenize

REPO = Path(__file__).resolve().parents[3]
ANSWERS = json.loads((REPO / "specs/serving/tokens.json").read_text(encoding="utf-8"))


def test_text_tokenizes_as_rust_does() -> None:
    for case in ANSWERS["text"]:
        assert tokenize(case["text"]) == case["tokens"], case["text"]


def test_rusts_name_tokens_are_tokens_once_each() -> None:
    for case in ANSWERS["names"]:
        tokens = case["tokens"]
        assert len(set(tokens)) == len(tokens), case["name"]
        for t in tokens:
            assert tokenize(t) == [t], (case["name"], t)
