"""SurrealDB 3.3.0 traversal semantics against networkx as an independent reference.

Covers cycles, parallel arcs, self-loops, isolates, the explicit depth bound (does it say it
truncated?), the 256 recursion limit, +collect / +path / +shortest, and +path growth on
diamond chains. No timings are recorded (operator directive; server `time` fields are dropped).

Usage: python surreal_traversal.py http://127.0.0.1:18000 <root-password> <out.json>
"""

import json
import random
import sys

import networkx as nx
import requests

URL, PW, OUT = sys.argv[1:4]
AUTH = ("root", PW)


def sql(text, ns="p", db="p"):
    h = {"Accept": "application/json"}
    if ns:
        h["surreal-ns"] = ns
    if db:
        h["surreal-db"] = db
    r = requests.post(f"{URL}/sql", data=text.encode(), auth=AUTH, headers=h, timeout=600)
    r.raise_for_status()
    out = []
    for stmt in r.json():
        stmt.pop("time", None)
        out.append(stmt)
    return out


def one(text):
    res = sql(text)
    last = res[-1]
    if last["status"] != "OK":
        return {"error": last["result"]}
    return last["result"]


def reset(db):
    sql(f"DEFINE NAMESPACE IF NOT EXISTS p; USE NS p; REMOVE DATABASE IF EXISTS {db}; DEFINE DATABASE {db};", ns=None, db=None)


def load(nodes, arcs, db):
    """nodes: ints; arcs: (src, dst) with duplicates allowed (parallel arcs)."""
    reset(db)
    stmts = [f"USE NS p DB {db};", "DEFINE TABLE fn SCHEMALESS;", "DEFINE TABLE calls TYPE RELATION IN fn OUT fn;"]
    stmts.append("INSERT INTO fn " + json.dumps([{"id": n} for n in nodes]).replace('"id"', "id") + ";")
    rel = ", ".join(f"{{in: fn:{a}, out: fn:{b}}}" for a, b in arcs)
    stmts.append(f"INSERT RELATION INTO calls [{rel}];")
    res = sql("\n".join(stmts), ns=None, db=None)
    errs = [s for s in res if s["status"] != "OK"]
    assert not errs, errs


def ids(result):
    if isinstance(result, dict):
        return result
    return [int(str(x).split(":")[1]) for x in result]


def q(db, text):
    res = sql(f"USE NS p DB {db}; {text}", ns=None, db=None)
    last = res[-1]
    if last["status"] != "OK":
        return {"error": last["result"]}
    return last["result"]


report = {}

# ---- G1: small graph with every hazard ---------------------------------------------------
# 1->2 twice (parallel), 2->3, 3->1 (cycle 1-2-3), 3->4, 4->4 (self-loop), 4->5, 6 isolate,
# 1->7->5 (alternative equal-length route 1..5 is 1-7-5 (2 hops) vs 1-2-3-4-5 (4 hops)),
# 2->8, 8->5 (another 3-hop route).
G1_NODES = [1, 2, 3, 4, 5, 6, 7, 8]
G1_ARCS = [(1, 2), (1, 2), (2, 3), (3, 1), (3, 4), (4, 4), (4, 5), (1, 7), (7, 5), (2, 8), (8, 5)]
load(G1_NODES, G1_ARCS, "g1")
g1 = nx.MultiDiGraph()
g1.add_nodes_from(G1_NODES)
g1.add_edges_from(G1_ARCS)
r = {}
r["one_hop_from_1_raw"] = q("g1", "fn:1->calls->fn;")
r["one_hop_from_1_reference_multigraph"] = sorted(t for s, t in G1_ARCS if s == 1)
r["collect_unbounded_from_1"] = q("g1", "fn:1.{..+collect}->calls->fn;")
r["reachable_reference_from_1"] = sorted(nx.descendants(g1, 1))
r["collect_inclusive_from_1"] = q("g1", "fn:1.{..+collect+inclusive}->calls->fn;")
r["collect_depth2_from_1"] = q("g1", "fn:1.{1..2+collect}->calls->fn;")
r["within2_reference_from_1"] = sorted(n for n, d in nx.single_source_shortest_path_length(g1, 1, cutoff=2).items() if n != 1)
r["collect_from_isolate_6"] = q("g1", "fn:6.{..+collect}->calls->fn;")
r["path_depth4_from_1"] = q("g1", "fn:1.{1..4+path}->calls->fn;")
r["simple_paths_reference_len_le4_from_1"] = sorted(
    p[1:] for t in G1_NODES for p in nx.all_simple_paths(nx.DiGraph(g1), 1, t, cutoff=4) if t != 1
)
r["shortest_1_to_5"] = q("g1", "fn:1.{..+shortest=fn:5}->calls->fn;")
r["shortest_reference_1_to_5"] = [p[1:] for p in nx.all_shortest_paths(g1, 1, 5)]
r["shortest_1_to_6_unreachable"] = q("g1", "fn:1.{..+shortest=fn:6}->calls->fn;")
r["shortest_1_to_6_bounded8"] = q("g1", "fn:1.{1..8+shortest=fn:6}->calls->fn;")
r["shortest_4_to_4_selfloop"] = q("g1", "fn:4.{..+shortest=fn:4}->calls->fn;")
r["plain_recursion_depth3_from_1"] = q("g1", "fn:1.{3}->calls->fn;")
r["reverse_one_hop_into_5"] = q("g1", "fn:5<-calls<-fn;")
report["G1_hazards"] = r


def guarded(db, text):
    """Run a query that may exhaust the server; record the outcome instead of crashing."""
    try:
        return q(db, text)
    except requests.exceptions.ConnectionError as e:
        import subprocess
        st = subprocess.run(["docker", "inspect", "gap-surreal", "--format", "{{.State.OOMKilled}} {{.State.ExitCode}}"], capture_output=True, text=True).stdout.strip()
        return {"server_lost": type(e).__name__, "docker_oomkilled_exitcode": st}

# ---- G2: chain of 300 (depth limit) ------------------------------------------------------
chain = list(range(1, 301))
load(chain, [(i, i + 1) for i in range(1, 300)], "g2")
r = {}
r["collect_unbounded_count"] = q("g2", "count(fn:1.{..+collect}->calls->fn);")
r["collect_bounded_256_count"] = q("g2", "count(fn:1.{1..256+collect}->calls->fn);")
r["collect_bounded_10_count"] = q("g2", "count(fn:1.{1..10+collect}->calls->fn);")
r["collect_bounded_300_count"] = q("g2", "count(fn:1.{1..300+collect}->calls->fn);")
r["reference_reachable_count"] = 299
report["G2_chain300"] = r

# ---- G3: diamond chains (+path growth): k diamonds -> 2^k source-to-sink paths -----------
r = {}
for k in (4, 8, 12, 14):
    nodes = [0]
    arcs = []
    cur = 0
    nid = 1
    for _ in range(k):
        a, b, j = nid, nid + 1, nid + 2
        nid += 3
        nodes += [a, b, j]
        arcs += [(cur, a), (cur, b), (a, j), (b, j)]
        cur = j
    db = f"g3k{k}"
    load(nodes, arcs, db)
    g = nx.DiGraph()
    g.add_edges_from(arcs)
    res = q(db, f"count(fn:0.{{1..{2 * k}+path}}->calls->fn);")
    sink_paths = q(db, f"array::len(fn:0.{{1..{2 * k}+path}}->calls->fn[WHERE $this.last() = fn:{cur}]);")
    r[f"k{k}"] = {
        "surreal_path_count": res,
        "surreal_paths_ending_at_sink": sink_paths,
        "reference_source_to_sink_paths": sum(1 for _ in nx.all_simple_paths(g, 0, cur)),
        "reference_maximal_paths": 2**k,
    }
report["G3_diamond_path_growth"] = r

# ---- G4: modular synthetic call graph (same generator family as gds_determinism.py) -------
rng = random.Random(20261005)
N = 300
iso = set(range(N - 12, N))
live = [i for i in range(N) if i not in iso]
arcs = []
for m in range(0, len(live), 20):
    block = live[m : m + 20]
    for i, s in enumerate(block):
        for _ in range(2):
            arcs.append((s, block[rng.randrange(len(block))]))
        if i % 7 == 0:
            arcs.append((s, block[0]))
for _ in range(80):
    arcs.append((rng.choice(live), rng.choice(live)))
load(list(range(N)), arcs, "g4")
g4 = nx.MultiDiGraph()
g4.add_nodes_from(range(N))
g4.add_edges_from(arcs)
agree = {"reach": 0, "within3": 0, "shortest_len": 0, "shortest_is_a_shortest_path": 0}
mism = []
starts = list(range(0, N, 15))
for s in starts:
    got = q("g4", f"fn:{s}.{{..+collect}}->calls->fn;")
    if not isinstance(got, dict) and sorted(set(ids(got))) == sorted(nx.descendants(g4, s)) and len(got) == len(set(ids(got))):
        agree["reach"] += 1
    else:
        mism.append({"start": s, "kind": "reach", "got": got if isinstance(got, dict) else len(got), "ref": len(nx.descendants(g4, s))})
    got3 = q("g4", f"fn:{s}.{{1..3+collect}}->calls->fn;")
    ref3 = sorted(n for n, d in nx.single_source_shortest_path_length(g4, s, cutoff=3).items() if n != s)
    if not isinstance(got3, dict) and sorted(set(ids(got3))) == ref3:
        agree["within3"] += 1
    else:
        mism.append({"start": s, "kind": "within3"})
    targets = sorted(nx.descendants(g4, s))
    if targets:
        t = targets[len(targets) // 2]
        sp = q("g4", f"fn:{s}.{{..+shortest=fn:{t}}}->calls->fn;")
        ref_len = nx.shortest_path_length(g4, s, t)
        if not isinstance(sp, dict) and len(sp) == ref_len:
            agree["shortest_len"] += 1
            path = [s] + ids(sp)
            if all(g4.has_edge(a, b) for a, b in zip(path, path[1:])):
                agree["shortest_is_a_shortest_path"] += 1
        else:
            mism.append({"start": s, "kind": "shortest", "got": sp, "ref_len": ref_len})
report["G4_modular"] = {"starts": len(starts), "agree": agree, "mismatches": mism[:10], "arcs": len(arcs)}

# ---- last: unbounded +path on G1 (cycle + parallel arcs); container is memory-capped -------
load(G1_NODES, G1_ARCS, "g1")
report["G1_path_bounded_on_cycle"] = {
    f"depth{d}_count": q("g1", f"count(fn:1.{{1..{d}+path}}->calls->fn);") for d in (4, 8, 12, 16, 20)
}
risky = {}
for name, text in (
    ("path_unbounded_from_1_on_cycle", "fn:1.{..+path}->calls->fn;"),
    ("plain_recursion_unbounded_from_1_on_cycle", "fn:1.{..}->calls->fn;"),
    ("collect_unbounded_from_1_on_cycle_repeat", "fn:1.{..+collect}->calls->fn;"),
):
    risky[name] = guarded("g1", text)
    if isinstance(risky[name], dict) and "server_lost" in risky[name]:
        import subprocess, time as _t
        subprocess.run(["docker", "start", "gap-surreal"], capture_output=True)
        for _ in range(60):
            try:
                requests.get(f"{URL}/health", timeout=2)
                break
            except Exception:
                _t.sleep(1)
        load(G1_NODES, G1_ARCS, "g1")
report["G1_risky_unbounded_on_cycle"] = risky

with open(OUT, "w") as f:
    json.dump(report, f, indent=1, default=str)
print(json.dumps(report, default=str)[:12000])
