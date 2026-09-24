"""The structured evaluation's packet (ADR-0021; DESIGN §12; the behavioral-model plan's §6).

For each pre-registered question of a stage, one Markdown section: the question, its target items
(positive and negative, with their source citations), and what the served generation answers for
each operation the question names: `get_operation`'s record (each parameter's fates with verdicts
and source lines, delegations, handoffs, the status and its reason) and the brief, if one exists.
The only marks are mechanical ones (does the operation resolve; does a parameter a claim names
have a fate). Rating each item (present / partial / absent / incorrect / misleading) is the
assessor's, in `docs/design_review/reviews/structured_eval_<date>.md`, and the operator reviews it.

Evaluation only: nothing here feeds the compiler.

    uv run python scripts/structured_eval.py GENERATION eval/behavior/fastmcp-4.0.5.toml \
        --stage 1 --out build/structured/stage1.md
"""

from __future__ import annotations

import argparse
import re
import sys
import tomllib
from pathlib import Path

from lctx_mcp import operations as ops
from lctx_mcp.generation import Generation, load

WORD = re.compile(r"[A-Za-z_][A-Za-z0-9_]*")


def _fate_line(f: ops.Fate) -> str:
    where = f"{f.path}:{f.line}" if f.path else "?"
    parts = [f"`{f.kind}`"]
    if f.callee:
        parts.append(f"→ `{f.callee}`")
    elif f.callee_text:
        parts.append(f"→ `{f.callee_text}` (outside the release)")
    if f.target:
        parts.append(f"formal `{f.target}`")
    if f.value:
        parts.append(f"value `{f.value}`")
    parts.append(f"verdict **{f.verdict}**")
    if f.boundary_reason:
        parts.append(f"({f.boundary_reason})")
    if f.condition:
        parts.append(f"when `{f.condition}`")
    if f.phase:
        parts.append(f"phase {f.phase}")
    if f.conditional:
        parts.append("(on some paths)")
    if f.occurrences > 1:
        parts.append(f"x{f.occurrences}")
    text = (f.site_text or "").strip().replace("\n", " ")
    if len(text) > 160:
        text = text[:157] + "..."
    return " ".join(parts) + f" — {where}" + (f" — `{text}`" if text else "")


def _operation(gen: Generation, spelling: str) -> list[str]:
    try:
        op = ops.get_operation(gen, gen.snapshot_id, spelling)
    except ops.OperationError as e:
        return [f"- **`{spelling}`: does not resolve** ({e})"]
    return _render(gen, op, spelling)


def _render(gen: Generation, op: ops.Operation, spelling: str | None = None) -> list[str]:
    spelling = spelling or op.access_path
    out = [
        f"- **`{spelling}`** → `{op.access_path}` ({op.kind}), status **{op.behavior_status}**"
        + (f" ({op.status_reason})" if op.status_reason else "")
    ]
    if op.docstring_summary:
        out.append(f"  - summary: {op.docstring_summary}")
    for p in op.parameters:
        if p.fates:
            out.append(f"  - parameter `{p.name}`:")
            out.extend(f"    - {_fate_line(f)}" for f in p.fates)
        else:
            out.append(f"  - parameter `{p.name}`: no fate ({p.note})")
    if op.delegates:
        out.append(f"  - delegations and literals ({len(op.delegates)}):")
        out.extend(f"    - {_fate_line(f)}" for f in op.delegates[:25])
        if len(op.delegates) > 25:
            out.append(f"    - … {len(op.delegates) - 25} more")
    if op.handoffs:
        out.append(f"  - handoffs ({len(op.handoffs)}):")
        out.extend(f"    - {_fate_line(f)}" for f in op.handoffs)
    if op.reads:
        out.append(f"  - settings read ({len(op.reads)}):")
        out.extend(f"    - `{f.target}` {_fate_line(f)}" for f in op.reads)
    if op.singleton_of:
        out.append(f"  - the singleton `{op.singleton_of}`'s fields:")
        for fr in op.fields:
            reads = "; ".join(
                f"{r.reader or 'module body'} ({r.phase}"
                + (f", when `{r.condition}`" if r.condition else "")
                + f") {r.path}:{r.line}"
                for r in fr.reads
            )
            out.append(
                f"    - `{fr.name}`: "
                + (reads or "no read")
                + (f" — {fr.never_read}" if fr.never_read else "")
            )
    if op.constructor is not None:
        out.append(f"  - constructor `{op.constructor.access_path}`:")
        out.extend("  " + line for line in _render(gen, op.constructor))
    if op.capability_id:
        out.append(f"  - brief `{op.capability_id[:12]}`:")
        brief = bytes.fromhex(op.capability_id)
        for a in gen.tables["assertions"].to_pylist():
            if a["brief_id"] == brief and a["text"]:
                out.append(f"    - [{a['section']}/{a['status']}] {a['text']}")
    return out


def _marks(gen: Generation, question: dict, item: dict) -> str:
    named = set(WORD.findall(item["claim"]))
    hits = []
    for spelling in question["operations"]:
        try:
            op = ops.get_operation(gen, gen.snapshot_id, spelling)
        except ops.OperationError:
            continue
        for p in op.parameters:
            if p.name in named:
                hits.append(f"`{p.name}` {'has fates' if p.fates else 'has no fate'}")
    return "; ".join(sorted(set(hits))) or "no parameter of the operations is named"


def _request(gen: Generation, request: dict, embedder=None) -> list[str]:
    """A pre-registered `find_operations` or `search_operations` request, answered."""
    if request["tool"] == "find_operations":
        found = ops.find_operations(
            gen, ops.Where.model_validate(request["where"]), limit=50, cursor=None
        )
        lines = [
            f"- `find_operations({request['where']})`: {found.total} match(es), "
            f"complete **{found.complete}**, {found.unknown_total} could still match"
        ]
        lines += [f"  - match `{m.access_path}` ({m.behavior_status})" for m in found.matches]
        lines += [f"  - could match `{u.access_path}`" for u in found.unknown[:10]]
        return lines
    import asyncio

    hits = asyncio.run(
        ops.search_operations(
            gen, ops.OperationIndex(gen), embedder, request["query"], None, limit=10
        )
    )
    lines = [f"- `search_operations({request['query']!r})`, mode {hits.mode}:"]
    lines += [
        f"  {i + 1}. `{h.access_path}` ({h.rank_source}, {h.relevance})"
        for i, h in enumerate(hits.hits)
    ]
    return lines


def packet(
    gen: Generation, questions: list[dict], stage: int | None, exits: list[dict], embedder=None
) -> str:
    lines = [
        f"# Structured evaluation packet — generation `{gen.key}`",
        "",
        f"Snapshot `{gen.snapshot_id}`, library `{gen.library}`. Stage filter: {stage or 'all'}.",
        "Generated by `scripts/structured_eval.py`; the targets are the pre-registered set.",
        "",
    ]
    for q in questions:
        if stage is not None and q["stage"] != stage:
            continue
        lines += [f"## {q['id']} ({q['kind']}, stage {q['stage']})", "", q["text"], ""]
        lines.append("**Targets**")
        lines.append("")
        for it in q["item"]:
            sources = ", ".join(it.get("source", []))
            lines.append(
                f"- `{it['id']}` ({it['polarity']}) {it['claim']} — {sources} "
                f"— *marks: {_marks(gen, q, it)}*"
            )
        lines += ["", "**Served answer**", ""]
        if "request" in q:
            lines += _request(gen, q["request"], embedder)
        for spelling in q["operations"]:
            lines += _operation(gen, spelling)
        lines.append("")
    for e in exits:
        if stage is None or e["stage"] == stage:
            lines += [f"## Exit rule, stage {e['stage']}", "", e["rule"], ""]
    return "\n".join(lines) + "\n"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("generation", type=Path)
    parser.add_argument("questions", type=Path)
    parser.add_argument("--stage", type=int)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument(
        "--embed-url", help="the vLLM service for search items' query vectors (else lexical)"
    )
    args = parser.parse_args()
    embedder = None
    if args.embed_url:
        from lctx_mcp.embedder import HttpEmbedder

        embedder = HttpEmbedder(args.embed_url)
    gen = load(args.generation, embedder.spec if embedder else None)
    spec = tomllib.loads(args.questions.read_text(encoding="utf-8"))
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(
        packet(gen, spec["question"], args.stage, spec.get("exit", []), embedder),
        encoding="utf-8",
    )
    print(f"structured-eval: wrote {args.out} (generation {gen.key})")
    return 0


if __name__ == "__main__":
    sys.exit(main())
