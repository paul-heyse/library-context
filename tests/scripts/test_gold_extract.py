from __future__ import annotations

import gold_extract


def test_spans_are_deduplicated_sorted_and_site_relative() -> None:
    def span(path: str, start: int) -> dict:
        return {
            "kind": "source_span",
            "artifact": f"content/sources/{path}",
            "line_start": start,
            "line_end": start + 3,
            "source_sha256": "s",
            "excerpt_sha256": f"e{start}",
        }

    family = {
        "id": "fm.x",
        "title": "X",
        "authoring_sha256": "a",
        "operations": ["b.op", "a.op"],
        "task_aliases": ["do x"],
        "claims": [
            {"evidence": [span("pkg/b.py", 9), span("pkg/a.py", 5)]},
            {"evidence": [span("pkg/a.py", 5), {"kind": "runtime_observation", "artifact": "o"}]},
        ],
    }
    (out,) = gold_extract.extract([family])["families"]
    assert out["operations"] == ["a.op", "b.op"]
    assert [(s["path"], s["line_start"]) for s in out["source_spans"]] == [
        ("pkg/a.py", 5),
        ("pkg/b.py", 9),
    ]
