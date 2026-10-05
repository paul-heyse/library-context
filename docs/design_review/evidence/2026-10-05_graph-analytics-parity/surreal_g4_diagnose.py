"""Diagnose G4 reach/within-3 differences: is the start node included when it lies on a cycle?

Reloads database g4 (the memory engine loses it when a risky probe OOM-kills the server).
Usage: same args as surreal_traversal.py."""
import json, random, sys
import networkx as nx, requests
URL, PW, OUT = sys.argv[1:4]
def q(text):
    r = requests.post(f"{URL}/sql", data=f"USE NS p DB g4; {text}".encode(), auth=("root", PW), headers={"Accept": "application/json"}, timeout=600).json()
    return r[-1]["result"] if r[-1]["status"] == "OK" else {"error": r[-1]["result"]}
rng = random.Random(20261005); N = 300
iso = set(range(N - 12, N)); live = [i for i in range(N) if i not in iso]; arcs = []
for m in range(0, len(live), 20):
    block = live[m:m + 20]
    for i, s in enumerate(block):
        for _ in range(2): arcs.append((s, block[rng.randrange(len(block))]))
        if i % 7 == 0: arcs.append((s, block[0]))
for _ in range(80): arcs.append((rng.choice(live), rng.choice(live)))
g = nx.MultiDiGraph(); g.add_nodes_from(range(N)); g.add_edges_from(arcs)
def sql(text):
    return requests.post(f"{URL}/sql", data=text.encode(), auth=("root", PW), headers={"Accept": "application/json"}, timeout=600).json()
sql("DEFINE NAMESPACE IF NOT EXISTS p; USE NS p; REMOVE DATABASE IF EXISTS g4; DEFINE DATABASE g4;")
rel = ", ".join(f"{{in: fn:{a}, out: fn:{b}}}" for a, b in arcs)
nodes = ", ".join(f"{{id: {n}}}" for n in range(N))
res = sql(f"USE NS p DB g4; DEFINE TABLE fn SCHEMALESS; DEFINE TABLE calls TYPE RELATION IN fn OUT fn; INSERT INTO fn [{nodes}]; INSERT RELATION INTO calls [{rel}];")
assert all(x["status"] == "OK" for x in res), res
ids = lambda res: [int(str(x).split(":")[1]) for x in res]
out = {"reach": {"equal_desc": 0, "equal_desc_plus_start_on_cycle": 0, "other": []}, "within3": {"equal": 0, "equal_plus_start_when_cycle_le3": 0, "other": []}}
for s in range(0, N, 15):
    got = set(ids(q(f"fn:{s}.{{..+collect}}->calls->fn;")))
    desc = nx.descendants(g, s)
    on_cycle = any(s in nx.descendants(g, t) for t in g.successors(s)) or g.has_edge(s, s)
    if got == desc: out["reach"]["equal_desc"] += 1
    elif got == desc | {s} and on_cycle: out["reach"]["equal_desc_plus_start_on_cycle"] += 1
    else: out["reach"]["other"].append(s)
    got3 = set(ids(q(f"fn:{s}.{{1..3+collect}}->calls->fn;")))
    ref3 = {n for n, d in nx.single_source_shortest_path_length(g, s, cutoff=3).items() if n != s}
    # walk of length 1..3 returning to s
    back = any(s in {n for n, d in nx.single_source_shortest_path_length(g, t, cutoff=2).items()} for t in g.successors(s))
    if got3 == ref3: out["within3"]["equal"] += 1
    elif got3 == ref3 | {s} and back: out["within3"]["equal_plus_start_when_cycle_le3"] += 1
    else: out["within3"]["other"].append({"start": s, "extra": sorted(got3 - ref3), "missing": sorted(ref3 - got3)})
json.dump(out, open(OUT, "w"), indent=1); print(json.dumps(out))
