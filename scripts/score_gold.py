"""Gold scoring (DESIGN §12, slice 3.3): a served generation against the committed gold extract.

Usage: score_gold.py GENERATION [--embedder vllm|fake|none] [--embed-url URL] [--sources DIR]
                     [--json OUT]

The matcher is version 2 (`gold_match`, pre-registered in ADR-0010's 2026-09-24 amendment):
identity is the declaration node, a gold operation resolving by exact path against the served
`public_paths`; unresolved operations are listed apart and stay in the Jaccard union as strings.
Every metric is over all gold units (22 families, 44 aliases, 157 spans).

- (a) Per gold family, the best node Jaccard over briefs, each contributing its seed; the mean over
  families, and how many any brief touches.
- (b) Every task alias of every family is searched with `limit = 5`; a hit is a returned brief
  whose seed is in the family's node set. Hit@1 and hit@5 per family and overall. Each alias
  records its mode, any degraded reason and its ranked hits. Only live vectors
  (`--embedder vllm`) make a hybrid result evidence: an alias degraded to lexical-only under them
  makes the run `blocked` (exit 2); a lexical-only or fake-vector run is labelled as such.
- (c) The recall of the gold's static-evidence spans: a gold span (file, lines) is recalled when a
  served evidence span of that file overlaps it. Over all families, and over those a brief
  touches. The line → byte mapping reads the release's sources (`--sources`, the acquired
  environment's site-packages).

Evaluation only: nothing here feeds the compiler (§1.4). Prints a summary and writes JSON.
"""

from __future__ import annotations

import argparse
import asyncio
import hashlib
import json
from pathlib import Path

from gold_match import MATCHER_VERSION, class_nodes, hits, jaccard, resolve, status
from lctx_mcp.embedder import FakeEmbedder, HttpEmbedder
from lctx_mcp.generation import load
from lctx_mcp.server import search, serve

ROOT = Path(__file__).resolve().parent.parent
GOLD = ROOT / "eval" / "gold" / "fastmcp-4.0.5.json"
SOURCES = ROOT / "build" / "envs" / "fastmcp" / "lib" / "python3.14" / "site-packages"


def span_recall(
    gold: list[dict],
    evidence: dict[str, list[tuple[int, int]]],
    sources: Path,
    touched: set[str],
) -> tuple[list[int], list[int], int]:
    """(c): recalled and total spans, overall and in touched families, and how many the sources
    could not map. An unmapped span is a miss, never out of the denominator (R2 F2)."""
    recall_all = [0, 0]
    recall_touched = [0, 0]
    unmapped = 0
    for f in gold:
        for span in f["source_spans"]:
            starts = line_spans(sources, span["path"])
            hit = False
            if starts is None or span["line_end"] >= len(starts):
                unmapped += 1
            else:
                a, z = starts[span["line_start"] - 1], starts[span["line_end"]]
                hit = any(s < z and a < e for s, e in evidence.get(span["path"], []))
            recall_all[0] += hit
            recall_all[1] += 1
            if f["id"] in touched:
                recall_touched[0] += hit
                recall_touched[1] += 1
    return recall_all, recall_touched, unmapped


def line_spans(sources: Path, path: str) -> list[int] | None:
    """The byte offset of each line start of a release file (and its end)."""
    file = sources / path
    if not file.exists():
        return None
    data = file.read_bytes()
    starts = [0]
    for i, b in enumerate(data):
        if b == 0x0A:
            starts.append(i + 1)
    starts.append(len(data))
    return starts


async def score(generation: Path, embedder_name: str, url: str, sources: Path) -> dict:
    embedder = {
        "vllm": lambda: HttpEmbedder(url, timeout=60.0),
        "fake": FakeEmbedder,
        "none": lambda: None,
    }[embedder_name]()
    gen = load(generation, embedder.spec if embedder else None)
    served = serve(gen, embedder)
    gold = json.loads(GOLD.read_text())["families"]
    public = gen.tables["public_paths"].to_pylist()
    families = resolve(gold, public)
    classes = class_nodes(public)
    seeds = {b: r["seed_node_id"] for b, r in gen.briefs.items()}

    # (a)
    best: dict[str, float] = {
        f.id: max((jaccard(f, s) for s in seeds.values()), default=0.0) for f in families
    }
    touched = [fid for fid, j in best.items() if j > 0]
    under = sorted({op for f in families for op in f.unresolved_under_root})
    outside = sorted({op for f in families for op in f.outside_root})

    # (b), over every alias of every family
    retrieval: dict[str, dict] = {}
    degraded = 0
    mode = None
    for f, g in zip(families, gold, strict=True):
        hits1 = hits5 = 0
        aliases = []
        for alias in g["task_aliases"]:
            result = await search(served, gen.library, alias, limit=5)
            mode = result.mode
            ranked = [(h.title, seeds[bytes.fromhex(h.capability_id)]) for h in result.hits]
            first = bool(ranked[:1]) and hits(f, ranked[0][1])
            any5 = any(hits(f, s) for _, s in ranked[:5])
            hits1 += first
            hits5 += any5
            degraded += result.mode != "hybrid"
            aliases.append(
                {
                    "alias": alias,
                    "mode": result.mode,
                    "degraded_reason": result.degraded_reason,
                    "hit@1": first,
                    "hit@5": any5,
                    "ranked": [title for title, _ in ranked],
                }
            )
        retrieval[f.id] = {
            "aliases": len(g["task_aliases"]),
            "hit@1": hits1,
            "hit@5": hits5,
            "per_alias": aliases,
        }
    aliases_total = sum(r["aliases"] for r in retrieval.values())
    run_status = status(embedder_name, degraded)

    # (c)
    evidence: dict[str, list[tuple[int, int]]] = {}
    for row in gen.tables["evidence"].to_pylist():
        if row["path"] and row["start_byte"] is not None and row["end_byte"] is not None:
            evidence.setdefault(row["path"], []).append((row["start_byte"], row["end_byte"]))
    recall_all, recall_touched, unmapped = span_recall(gold, evidence, sources, set(touched))

    label = {
        "vllm": "live vectors",
        "fake": "fake vectors: not evidence",
        "none": "lexical only",
    }[embedder_name]
    return {
        "generation": gen.key,
        "snapshot": gen.snapshot_id,
        "matcher_version": MATCHER_VERSION,
        "gold_sha256": hashlib.sha256(GOLD.read_bytes()).hexdigest(),
        "bundle_format": gen.manifest.get("format"),
        "units": {"families": len(families), "aliases": aliases_total, "spans": recall_all[1]},
        # The class ceiling (R2 O1): a class operation matches only a brief seeded by that class.
        "class_ceiling": {
            "class_operations": sum(1 for f in families for n in f.nodes if n in classes),
            "class_only_families": [f.id for f in families if f.nodes and f.nodes <= classes],
            "class_only_aliases": sum(
                len(g["task_aliases"])
                for f, g in zip(families, gold, strict=True)
                if f.nodes and f.nodes <= classes
            ),
        },
        "briefs": len(seeds),
        "embedder": embedder_name,
        "label": label,
        "status": run_status,
        "mode": mode,
        "degraded_aliases": degraded,
        "a": {
            "families": len(families),
            "touched": len(touched),
            "mean_best_jaccard": round(sum(best.values()) / len(best), 4) if best else 0.0,
            "best_jaccard": {k: round(v, 4) for k, v in sorted(best.items()) if v > 0},
            "operations": sum(len(g["operations"]) for g in gold),
            "unresolved_under_root": under,
            "outside_root": outside,
        },
        "b": {
            "families": len(retrieval),
            "aliases": aliases_total,
            "hit@1": sum(r["hit@1"] for r in retrieval.values()),
            "hit@5": sum(r["hit@5"] for r in retrieval.values()),
            "per_family": retrieval,
        },
        "c": {
            "spans": recall_all[1],
            "recalled": recall_all[0],
            "touched_spans": recall_touched[1],
            "touched_recalled": recall_touched[0],
            "unmapped": unmapped,
        },
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("generation", type=Path)
    parser.add_argument("--embedder", choices=["vllm", "fake", "none"], default="none")
    parser.add_argument("--embed-url", default="http://127.0.0.1:8000")
    parser.add_argument("--sources", type=Path, default=SOURCES)
    parser.add_argument("--json", type=Path)
    args = parser.parse_args()
    out = asyncio.run(score(args.generation, args.embedder, args.embed_url, args.sources))
    a, b, c = out["a"], out["b"], out["c"]
    print(
        f"gold scores for generation {out['generation']} ({out['briefs']} briefs; {out['label']}; "
        f"matcher {out['matcher_version']}; {out['status']})"
    )
    print(
        f"(a) best-match node Jaccard: mean {a['mean_best_jaccard']} over {a['families']} "
        f"families; {a['touched']} touched; of {a['operations']} operations, "
        f"{len(a['unresolved_under_root'])} under the root resolve to nothing and "
        f"{len(a['outside_root'])} are outside it"
    )
    print(
        f"(b) retrieval ({out['mode']}): hit@1 {b['hit@1']}/{b['aliases']}, "
        f"hit@5 {b['hit@5']}/{b['aliases']} over {b['families']} families"
        + (f"; {out['degraded_aliases']} aliases degraded" if out["degraded_aliases"] else "")
    )
    print(
        f"(c) span recall: {c['recalled']}/{c['spans']} overall; "
        f"{c['touched_recalled']}/{c['touched_spans']} in touched families"
        + (f"; {c['unmapped']} unmapped" if c["unmapped"] else "")
    )
    if args.json:
        args.json.write_text(json.dumps(out, indent=1, sort_keys=True) + "\n")
    if out["status"] == "blocked":
        raise SystemExit(2)


if __name__ == "__main__":
    main()
