"""Print sampled raw commands for one heuristic/outcome/class value (spot checks; output stays local).

Usage: python -I peek.py <value> [n] [key] [stratum]   e.g. peek.py masked-failure 10 out claude-main
"""
import gzip
import json
import os
import random
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import config  # noqa: E402

val = sys.argv[1]
n = int(sys.argv[2]) if len(sys.argv) > 2 else 6
key = sys.argv[3] if len(sys.argv) > 3 else "heur"
strat = sys.argv[4] if len(sys.argv) > 4 else None
seed = int(os.environ.get("SEED", "7"))
ids = {}
for line in gzip.open(os.path.join(config.OUTDIR, "events.jsonl.gz"), "rt"):
    e = json.loads(line)
    v = str(e.get(key))
    if strat and "%s-%s" % (e["rt"], e["ak"]) != strat:
        continue
    if v == val or (val.endswith("*") and v.startswith(val[:-1])):
        ids[e["id"]] = e
random.seed(seed)
pick = set(random.sample(sorted(ids), min(n, len(ids))))
print("total", len(ids))
for line in gzip.open(os.path.join(config.OUTDIR, "raw_cmds.jsonl.gz"), "rt"):
    r = json.loads(line)
    if r["id"] in pick:
        e = ids[r["id"]]
        print("=== %s-%s %s %s cls=%s sub=%s exit=%s out=%s heur=%s fam=%s feats=%s" % (
            e["rt"], e["ak"], e["per"], e.get("role"), e.get("cls"), e.get("sub"), e.get("exit"), e.get("out"),
            e.get("heur"), e.get("fam"), e.get("feats")))
        print("CMD:", r["cmd"][:300].replace("\n", "\\n"))
        print("OUT:", r["otail"][-300:].replace("\n", "|"))
