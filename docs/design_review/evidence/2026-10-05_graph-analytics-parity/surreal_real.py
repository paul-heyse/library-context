"""SurrealDB 3.3.0 traversal correctness on the real facts-layer call graph, against networkx.

Vertices are numbered by sorted semantic key (fn:<index>, key kept as a field); every arc is one
`calls` edge record carrying its arc id (parallel arcs and self-loops preserved).
Checks for seeded sample starts: one-hop multiplicity, bounded `+collect` at depths 1..6 and
unbounded `+collect` (reachability; start-node-on-cycle rule from the synthetic probe), silent
truncation, `+shortest` length and validity, bounded `+path` counts against networkx simple paths
and walks. Deliberate capped probes last: unbounded `+path` from a node of the largest SCC, first
with `TIMEOUT 20s`, then without (container memory-capped). No timings are recorded.

Usage: python surreal_real.py http://127.0.0.1:18000 <pw> <facts-graph dir> <out.json>
"""

import csv
import json
import random
import subprocess
import sys
import time

import networkx as nx
import requests

URL, PW, DIR, OUT = sys.argv[1:5]
AUTH = ("root", PW)


def sql(text, timeout=900):
    r = requests.post(f"{URL}/sql", data=text.encode(), auth=AUTH, headers={"Accept": "application/json"}, timeout=timeout)
    if r.status_code >= 400:
        return [{"status": "ERR", "result": f"HTTP {r.status_code}: {r.text[:300]}"}]
    return r.json()


def q(text, timeout=900):
    res = sql(f"USE NS p DB real; {text}", timeout)
    last = res[-1]
    return last["result"] if last["status"] == "OK" else {"error": last["result"]}


def ids(res):
    return [int(str(x).split(":")[1]) for x in res]


def wait_up():
    for _ in range(120):
        try:
            requests.get(f"{URL}/health", timeout=2)
            return True
        except Exception:
            time.sleep(1)
    return False


def load(keys, arcs, index):
    sql("DEFINE NAMESPACE IF NOT EXISTS p; USE NS p; REMOVE DATABASE IF EXISTS real; DEFINE DATABASE real;")
    sql("USE NS p DB real; DEFINE TABLE fn SCHEMALESS; DEFINE TABLE calls TYPE RELATION IN fn OUT fn;")
    for i in range(0, len(keys), 5000):
        rows = ", ".join(f"{{id: {j}, key: {json.dumps(keys[j])}}}" for j in range(i, min(i + 5000, len(keys))))
        res = sql(f"USE NS p DB real; INSERT INTO fn [{rows}];")
        assert all(x["status"] == "OK" for x in res), res[-1]
    for i in range(0, len(arcs), 5000):
        rows = ", ".join(f"{{in: fn:{index[s]}, out: fn:{index[t]}, arc: '{a}'}}" for a, s, t in arcs[i : i + 5000])
        res = sql(f"USE NS p DB real; INSERT RELATION INTO calls [{rows}];")
        assert all(x["status"] == "OK" for x in res), res[-1]
    return {"fn": q("count(SELECT id FROM fn);"), "calls": q("count(SELECT id FROM calls);")}


STATE = {}


def guarded(text, timeout=600):
    """Run a possibly exhausting query; on server loss record OOM state, restart and reload."""
    try:
        res = q(text, timeout=timeout)
        return res
    except requests.exceptions.ConnectionError as e:
        st = subprocess.run(["docker", "inspect", "gap-surreal", "--format", "{{.State.OOMKilled}} {{.State.ExitCode}}"], capture_output=True, text=True).stdout.strip()
        subprocess.run(["docker", "start", "gap-surreal"], capture_output=True)
        wait_up()
        after = q("count(SELECT id FROM fn);")
        load(*STATE["load_args"])
        return {"server_lost": type(e).__name__, "docker_oomkilled_exitcode": st, "fn_count_after_restart_before_reload": after}


def checkpoint(rep):
    with open(OUT, "w") as f:
        json.dump(rep, f, indent=1, default=str)


def main():
    with open(f"{DIR}/vertices.csv") as f:
        keys = sorted(r["key"] for r in csv.DictReader(f))
    with open(f"{DIR}/arcs.csv") as f:
        arcs = [(r["arc"], r["src"], r["dst"]) for r in csv.DictReader(f)]
    index = {k: i for i, k in enumerate(keys)}
    g = nx.MultiDiGraph()
    g.add_nodes_from(range(len(keys)))
    g.add_edges_from((index[s], index[t]) for _, s, t in arcs)
    sg = nx.DiGraph(g)
    STATE["load_args"] = (keys, arcs, index)
    rep = {"data": "facts-layer reconstruction (approximates the normalized projection)", "loaded": load(keys, arcs, index)}

    rng = random.Random(20261005)
    callers = sorted(n for n in g.nodes if g.out_degree(n) > 0)
    sccs = sorted((sorted(c) for c in nx.strongly_connected_components(g) if len(c) > 1), key=len, reverse=True)
    starts = sorted(set(rng.sample(callers, 60) + [c[0] for c in sccs]))
    on_cycle = lambda s: any(s in nx.descendants(g, t) or t == s for t in g.successors(s))
    agg = {
        "starts": len(starts),
        "one_hop_multiplicity_equal": 0,
        "collect_unbounded_equal_with_start_rule": 0,
        "collect_unbounded_other": [],
        "collect_bounded_equal_with_start_rule": {d: 0 for d in range(1, 7)},
        "collect_bounded_other": {d: [] for d in range(1, 7)},
        "silently_truncated_at_depth": {d: 0 for d in range(1, 7)},
        "max_reach_depth": 0,
        "shortest_len_equal": 0,
        "shortest_valid_path": 0,
        "shortest_pairs": 0,
        "shortest_other": [],
    }
    for s in starts:
        one = q(f"fn:{s}->calls->fn;")
        if not isinstance(one, dict) and sorted(ids(one)) == sorted(t for _, t in g.out_edges(s)):
            agg["one_hop_multiplicity_equal"] += 1
        lengths = nx.single_source_shortest_path_length(g, s)
        desc = set(lengths) - {s}
        agg["max_reach_depth"] = max(agg["max_reach_depth"], max(lengths.values()))
        cyc = on_cycle(s)
        full = q(f"fn:{s}.{{..+collect}}->calls->fn;")
        got = set(ids(full)) if not isinstance(full, dict) else None
        if got is not None and (got == desc or (cyc and got == desc | {s})):
            agg["collect_unbounded_equal_with_start_rule"] += 1
        else:
            agg["collect_unbounded_other"].append({"start": keys[s], "got": full if got is None else len(got), "ref": len(desc)})
        for d in range(1, 7):
            res = q(f"fn:{s}.{{1..{d}+collect}}->calls->fn;")
            gotd = set(ids(res)) if not isinstance(res, dict) else None
            refd = {n for n, l in lengths.items() if 0 < l <= d}
            back = s in {n for t in g.successors(s) for n, l in nx.single_source_shortest_path_length(g, t, cutoff=d - 1).items()}
            if gotd is not None and (gotd == refd or (back and gotd == refd | {s})):
                agg["collect_bounded_equal_with_start_rule"][d] += 1
            elif len(agg["collect_bounded_other"][d]) < 5:
                agg["collect_bounded_other"][d].append({"start": keys[s], "extra": len(gotd - refd) if gotd else None, "missing": len(refd - gotd) if gotd else None})
            if len(refd) < len(desc):
                agg["silently_truncated_at_depth"][d] += 1  # the bounded answer omits reachable nodes, no flag
        if desc:
            t = sorted(desc)[len(desc) // 2]
            agg["shortest_pairs"] += 1
            sp = q(f"fn:{s}.{{..+shortest=fn:{t}}}->calls->fn;")
            if not isinstance(sp, dict) and sp is not None:
                path = [s] + ids(sp)
                if len(path) - 1 == lengths[t]:
                    agg["shortest_len_equal"] += 1
                if path[-1] == t and all(g.has_edge(a, b) for a, b in zip(path, path[1:])):
                    agg["shortest_valid_path"] += 1
            else:
                agg["shortest_other"].append({"start": keys[s], "target": keys[t], "got": sp})
    rep["traversal"] = agg
    checkpoint(rep)

    # +path: counts vs networkx, bounded, on the sample and on the largest SCC
    paths = {}
    for s in starts[:20] + [sccs[0][0]]:
        row = {"key": keys[s]}
        for d in (2, 3, 4):
            res = guarded(f"SELECT VALUE count($this.{{1..{d}+path}}->calls->fn) FROM ONLY fn:{s} TIMEOUT 60s;")
            row[f"surreal_paths_d{d}"] = res
            row[f"nx_simple_paths_le{d}"] = sum(1 for t in sg.nodes if t != s for _ in nx.all_simple_paths(sg, s, t, cutoff=d))
        paths[keys[s]] = row
        rep["path_counts"] = paths
        checkpoint(rep)
    big = sccs[0][0]
    rep["largest_scc"] = {"size": len(sccs[0]), "start": keys[big]}
    bpc = rep["largest_scc"]["bounded_path_counts"] = {}
    for d in (4, 6, 8, 10, 12, 16):
        bpc[d] = guarded(f"SELECT VALUE count($this.{{1..{d}+path}}->calls->fn) FROM ONLY fn:{big} TIMEOUT 60s;")
        bpc[f"{d}_without_timeout"] = guarded(f"count(fn:{big}.{{1..{d}+path}}->calls->fn);") if d <= 8 else "not attempted"
        checkpoint(rep)
        if isinstance(bpc[d], dict) and "server_lost" in bpc[d]:
            break

    # deliberate capped probes
    risky = {}
    for name, text in (
        ("path_unbounded_with_timeout_20s", f"SELECT VALUE count($this.{{..+path}}->calls->fn) FROM ONLY fn:{big} TIMEOUT 20s;"),
        ("path_unbounded_no_timeout", f"count(fn:{big}.{{..+path}}->calls->fn);"),
    ):
        risky[name] = guarded(text)
        rep["risky"] = risky
        checkpoint(rep)
    rep["risky"] = risky
    checkpoint(rep)
    print(json.dumps(rep, default=str)[:8000])


if __name__ == "__main__":
    main()
