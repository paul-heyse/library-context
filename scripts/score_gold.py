"""Gold scoring (DESIGN §12, slice 3.3): a served generation against the committed gold extract.

Usage: score_gold.py GENERATION [--embedder vllm|fake|none] [--embed-url URL] [--sources DIR]
                     [--json OUT]

- (a) Per gold family, the best Jaccard between its `operations` and one brief's public paths
  (`brief_members`); the mean over families, and how many families any brief touches.
- (b) As pre-registered in ADR-0010's amendment (deviation log D19): every task alias of every
  family whose operations include a published brief's public path is searched with `limit = 5`;
  a hit is a brief whose public paths meet the family's operations. Hit@1 and hit@5 per family
  and overall. Only live vectors (`--embedder vllm`) make a hybrid result evidence; a lexical-only
  or fake-vector run is labelled as such.
- (c) The recall of the gold's static-evidence spans: a gold span (file, lines) is recalled when a
  served evidence span of that file overlaps it. Over all families, and over those a brief
  touches. The line → byte mapping reads the release's sources (`--sources`, the acquired
  environment's site-packages).

Evaluation only: nothing here feeds the compiler (§1.4). Prints a summary and writes JSON.
"""

from __future__ import annotations

import argparse
import asyncio
import json
from pathlib import Path

from lctx_mcp.embedder import FakeEmbedder, HttpEmbedder
from lctx_mcp.generation import load
from lctx_mcp.server import search, serve

ROOT = Path(__file__).resolve().parent.parent
GOLD = ROOT / "eval" / "gold" / "fastmcp-4.0.5.json"
SOURCES = ROOT / "build" / "envs" / "fastmcp" / "lib" / "python3.14" / "site-packages"


def brief_paths(gen) -> dict[bytes, set[str]]:
    """Each brief's public paths."""
    out: dict[bytes, set[str]] = {}
    for row in gen.tables["brief_members"].to_pylist():
        out.setdefault(row["brief_id"], set()).add(row["access_path"])
    return out


def jaccard(a: set[str], b: set[str]) -> float:
    return len(a & b) / len(a | b) if a | b else 0.0


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
    families = json.loads(GOLD.read_text())["families"]
    paths = brief_paths(gen)
    published = set().union(*paths.values()) if paths else set()

    # (a)
    best: dict[str, float] = {}
    for f in families:
        ops = set(f["operations"])
        best[f["id"]] = max((jaccard(ops, p) for p in paths.values()), default=0.0)
    touched = [fid for fid, j in best.items() if j > 0]
    # Operations outside the release's public root no brief can name (plan 3.3): reported apart.
    unreachable = sorted(
        {o for f in families for o in f["operations"] if o.split(".")[0] != gen.library}
    )

    # (b)
    retrieval: dict[str, dict] = {}
    mode = None
    for f in families:
        ops = set(f["operations"])
        if not ops & published:
            continue
        relevant = {b for b, p in paths.items() if p & ops}
        hits1 = hits5 = 0
        for alias in f["task_aliases"]:
            result = await search(served, gen.library, alias, limit=5)
            mode = result.mode
            order = [bytes.fromhex(h.capability_id) for h in result.hits]
            hits1 += bool(order[:1] and order[0] in relevant)
            hits5 += any(b in relevant for b in order[:5])
        retrieval[f["id"]] = {
            "aliases": len(f["task_aliases"]),
            "hit@1": hits1,
            "hit@5": hits5,
        }
    aliases = sum(r["aliases"] for r in retrieval.values())

    # (c)
    evidence: dict[str, list[tuple[int, int]]] = {}
    for row in gen.tables["evidence"].to_pylist():
        if row["path"] and row["start_byte"] is not None and row["end_byte"] is not None:
            evidence.setdefault(row["path"], []).append((row["start_byte"], row["end_byte"]))
    recall_all = [0, 0]
    recall_touched = [0, 0]
    unmapped = 0
    for f in families:
        for span in f["source_spans"]:
            starts = line_spans(sources, span["path"])
            if starts is None or span["line_end"] >= len(starts):
                unmapped += 1
                continue
            a, z = starts[span["line_start"] - 1], starts[span["line_end"]]
            hit = any(s < z and a < e for s, e in evidence.get(span["path"], []))
            recall_all[0] += hit
            recall_all[1] += 1
            if f["id"] in touched:
                recall_touched[0] += hit
                recall_touched[1] += 1

    label = {
        "vllm": "live vectors",
        "fake": "fake vectors: not evidence",
        "none": "lexical only",
    }[embedder_name]
    return {
        "generation": gen.key,
        "snapshot": gen.snapshot_id,
        "briefs": len(paths),
        "embedder": embedder_name,
        "label": label,
        "mode": mode,
        "a": {
            "families": len(families),
            "touched": len(touched),
            "mean_best_jaccard": round(sum(best.values()) / len(best), 4) if best else 0.0,
            "best_jaccard": {k: round(v, 4) for k, v in sorted(best.items()) if v > 0},
            "operations": sum(len(f["operations"]) for f in families),
            "unreachable_operations": len(unreachable),
            "unreachable_roots": sorted({o.split(".")[0] for o in unreachable}),
        },
        "b": {
            "families": len(retrieval),
            "aliases": aliases,
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
        f"gold scores for generation {out['generation']} ({out['briefs']} briefs; {out['label']})"
    )
    print(
        f"(a) best-match Jaccard: mean {a['mean_best_jaccard']} over {a['families']} families; "
        f"{a['touched']} touched; {a['unreachable_operations']} of {a['operations']} operations "
        f"are outside the release ({', '.join(a['unreachable_roots'])})"
    )
    print(
        f"(b) retrieval ({out['mode']}): hit@1 {b['hit@1']}/{b['aliases']}, "
        f"hit@5 {b['hit@5']}/{b['aliases']} over {b['families']} families"
    )
    print(
        f"(c) span recall: {c['recalled']}/{c['spans']} overall; "
        f"{c['touched_recalled']}/{c['touched_spans']} in touched families"
        + (f"; {c['unmapped']} unmapped" if c["unmapped"] else "")
    )
    if args.json:
        args.json.write_text(json.dumps(out, indent=1, sort_keys=True) + "\n")


if __name__ == "__main__":
    main()
