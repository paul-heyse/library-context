"""Write the committable aggregate: counts, durations and token figures only.

Reads OUTDIR/{metrics.json, extras.json, events.jsonl.gz} and writes ../metrics.json next to this
directory. Command text, arguments, normalized command shapes, transcript paths and message text are
excluded; labels are classifier categories, recipe names, tool names and skill names.
"""
import collections
import gzip
import json
import os
import re
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import config  # noqa: E402

HERE = os.path.dirname(os.path.abspath(__file__))
DEST = os.path.join(os.path.dirname(HERE), "metrics.json")
SAFE_LABEL = re.compile(r"^[A-Za-z0-9_.:|()<>=+ -]{1,80}$")


def clean_label(k):
    k = re.sub(r"^masked:.*", "masked", k)
    return k if SAFE_LABEL.match(k) else None


def clean(d):
    """Keep nested dicts of counters/summaries with safe labels only."""
    if isinstance(d, dict):
        out = {}
        for k, v in d.items():
            ck = clean_label(str(k))
            if ck is None:
                continue
            cv = clean(v)
            if isinstance(cv, (int, float)) and ck in out and isinstance(out[ck], (int, float)):
                out[ck] += cv
            else:
                out[ck] = cv
        return out
    if isinstance(d, (int, float)) or d is None:
        return d
    if isinstance(d, str):
        return None
    if isinstance(d, list):
        return None
    return None


def main():
    M = json.load(open(os.path.join(config.OUTDIR, "metrics.json")))
    X = json.load(open(os.path.join(config.OUTDIR, "extras.json")))
    rj = json.load(open(os.path.join(config.OUTDIR, "recipes.json")))
    ever = set(rj["ever"])
    strata = collections.defaultdict(lambda: {"sessions": set(), "events": 0, "shell": 0})
    for line in gzip.open(os.path.join(config.OUTDIR, "events.jsonl.gz"), "rt"):
        e = json.loads(line)
        s = strata["%s|%s|%s" % (e["rt"], e["ak"], e.get("per"))]
        s["sessions"].add(e["sid"])
        s["events"] += 1
        s["shell"] += 1 if e.get("shell") else 0
    A = {
        "about": "W1 aggregates for the agent-workspace-effectiveness assessment; counts only. Produced by miner/aggregate.py.",
        "corpus": {k: {"sessions": len(v["sessions"]), "events": v["events"], "shell": v["shell"]} for k, v in sorted(strata.items())},
        "periods": {"bounds_utc": dict(config.PERIOD_BOUNDS), "cutoff_utc": config.CUTOFF, "current_group": list(config.CURRENT_GROUP)},
        "classes": clean(M["classes"]),
        "outcomes": clean(M["outcomes"]),
        "heuristics": clean(M["heuristics"]),
        "recipes_used": {k: {r: n for r, n in v.items() if r in ever} for k, v in M["recipes"]["used"].items()},
        "recipes_current_count": M["recipes"]["current_count"],
        "recipes_never_used_current": [r for r in M["recipes"]["never_used_current"] if r in ever],
        "recipes_used_not_current_count": len([r for r in M["recipes"]["used_not_current"] if r in ever]),
        "lctx": {k: clean(v) for k, v in M["lctx"].items() if k not in ("lctx_cli", "docker")},
        "polling": {"counts": clean(M["polling"]["counts"]), "sleep": clean(M["polling"]["sleep"]), "lockwait": clean(M["polling"]["lockwait"])},
        "retry_without_edit": {"counts": clean(M["retry"]["no_edit"]), "wall": clean(M["retry"]["no_edit_wall"])},
        "truncation_reruns": clean(M["truncation"]["counts"]),
        "redirect_vs_pipe": clean(M["pipes"]["redirect_vs_pipe"]),
        "blocks": clean(M["blocks"]["kinds"]),
        "tools": clean(M["tools"]["by_stratum"]),
        "skills": clean(M["tools"]["skills"]),
        "agent_types": clean(M["tools"]["agent_types"]),
        "wall_by_class": clean(M["wall_by_class"]),
        "extras": {k: clean(v) for k, v in X.items()},
        "classifier_spot_checks": {
            "masked-failure (claude-main)": "10 sampled, 7 genuine pipe-masked failures",
            "eval build_environment --shell": "8 sampled, 8 genuine",
            "inline LCTX_* assignment": "8 sampled, 8 genuine",
            "env:native-adapter": "5 sampled, 4 genuine stale-extension imports, 1 test failure with import noise",
            "env:py312-syntax": "5 sampled, 5 genuine",
            "probe:missing-path(compound)": "10 sampled, 8 explicit missing-path errors",
            "native wheel build seen in output": "8 sampled, 0 caused by the command (log reads); restricted to commands that ran uv",
            "agent flock wrapper": "all 164 inspected by pattern; reclassified from process-monitor to the wrapped command",
            "maintainer correction-like messages": "12 sampled, about 4 genuine corrections; not used for rates",
        },
    }
    json.dump(A, open(DEST, "w"), indent=1, sort_keys=True)
    blob = json.dumps(A)
    assert config.REPO not in blob and "/tmp/" not in blob, "aggregate leaked a path"
    print("wrote", DEST, len(blob), "bytes")


if __name__ == "__main__":
    main()
