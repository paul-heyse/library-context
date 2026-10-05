"""GDS determinism and parity on the real facts-layer call graph (FastMCP 4.0.5 generation
g786cd6d58dc5dccea686c0a54a8f0dcd, exported by export_facts_graph.sh).

* Loads vertices/arcs into Neo4j in two shuffled orders; for each: directed projection `g`
  (parallel arcs and self-loops preserved) and undirected Cypher-aggregation projection `u`
  (self-loops dropped, parallel arcs counted into weight w -- the same conversion as the Rust
  reference and our community adapter).
* Runs SCC, WCC, PageRank (+stats), Leiden (randomSeed 42, gamma 1.0), Louvain at concurrency 1
  and 4; repeats Leiden/Louvain 3x on the same load.
* Compares membership by semantic key with the networkx reference and the petgraph/leiden-rs
  reference (ref_labels.csv from petgraph_ref.rs). No timings recorded.

Usage: python gds_real.py bolt://127.0.0.1:17687 <pw> <facts-graph dir> <out.json>
"""

import csv
import hashlib
import json
import math
import random
import sys
from collections import Counter, defaultdict

import networkx as nx
import numpy as np
from neo4j import GraphDatabase

URI, PW, DIR, OUT = sys.argv[1:5]
BATCH = 5000


def read():
    with open(f"{DIR}/vertices.csv") as f:
        keys = [r["key"] for r in csv.DictReader(f)]
    with open(f"{DIR}/arcs.csv") as f:
        arcs = [(r["arc"], r["src"], r["dst"]) for r in csv.DictReader(f)]
    return keys, arcs


def digest(obj):
    return hashlib.sha256(json.dumps(obj, sort_keys=True).encode()).hexdigest()[:16]


def partition_of(labels):
    groups = defaultdict(list)
    for k, v in labels.items():
        groups[v].append(k)
    return sorted(sorted(g) for g in groups.values())


def contingency(a, b, keys):
    ia, ib = {}, {}
    xs = np.array([ia.setdefault(a[k], len(ia)) for k in keys])
    ys = np.array([ib.setdefault(b[k], len(ib)) for k in keys])
    return xs, ys


def ari_nmi(a, b, keys):
    xs, ys = contingency(a, b, keys)
    n = len(keys)
    pair = Counter(zip(xs.tolist(), ys.tolist()))
    ca, cb = Counter(xs.tolist()), Counter(ys.tolist())
    c2 = lambda x: x * (x - 1) / 2
    sij = sum(c2(v) for v in pair.values())
    sa = sum(c2(v) for v in ca.values())
    sb = sum(c2(v) for v in cb.values())
    exp = sa * sb / c2(n)
    mx = (sa + sb) / 2
    ari = 1.0 if mx == exp else (sij - exp) / (mx - exp)
    h = lambda c: -sum(v / n * math.log(v / n) for v in c.values())
    mi = sum(v / n * math.log((v / n) / ((ca[i] / n) * (cb[j] / n))) for (i, j), v in pair.items())
    ha, hb = h(ca), h(cb)
    nmi = 1.0 if ha + hb == 0 else 2 * mi / (ha + hb)
    return round(ari, 6), round(nmi, 6)


def load(driver, keys, arcs, order):
    rng = random.Random(order)
    ks, ar = list(keys), list(arcs)
    rng.shuffle(ks)
    rng.shuffle(ar)
    with driver.session() as s:
        for g in ("g", "u"):
            s.run(f"CALL gds.graph.drop('{g}', false) YIELD graphName RETURN graphName").consume()
        s.run("MATCH (n) CALL (n) { DETACH DELETE n } IN TRANSACTIONS OF 10000 ROWS").consume()
        s.run("CREATE CONSTRAINT fn_key IF NOT EXISTS FOR (n:Fn) REQUIRE n.key IS UNIQUE").consume()
        for i in range(0, len(ks), BATCH):
            s.run("UNWIND $rows AS k CREATE (:Fn {key: k})", rows=ks[i : i + BATCH]).consume()
        for i in range(0, len(ar), BATCH):
            s.run(
                "UNWIND $rows AS r MATCH (a:Fn {key: r[1]}), (b:Fn {key: r[2]}) CREATE (a)-[:CALLS {arc: r[0]}]->(b)",
                rows=[list(x) for x in ar[i : i + BATCH]],
            ).consume()
        counts = s.run("MATCH (n:Fn) WITH count(n) AS n MATCH ()-[r:CALLS]->() RETURN n, count(r) AS r").single().data()
        g = s.run(
            "CALL gds.graph.project('g', 'Fn', 'CALLS') YIELD nodeCount, relationshipCount RETURN nodeCount, relationshipCount"
        ).single().data()
        u = s.run(
            "MATCH (a:Fn) OPTIONAL MATCH (a)-[r:CALLS]->(b:Fn) WHERE a <> b "
            "WITH a, b, count(r) AS w "
            "WITH gds.graph.project('u', a, b, {relationshipProperties: {w: toFloat(w)}}, "
            "{undirectedRelationshipTypes: ['*']}) AS p RETURN p.nodeCount AS nodeCount, p.relationshipCount AS relationshipCount"
        ).single().data()
    return {"stored": counts, "g": g, "u": u}


def stream(driver, cy):
    with driver.session() as s:
        return [r.data() for r in s.run(cy)]


KEY = "gds.util.asNode(nodeId).key AS key"


def run(driver, c, repeats):
    out = {}
    for algo, graph, cfg, col in (
        ("scc", "g", "", "componentId"),
        ("wcc", "g", "", "componentId"),
        ("leiden", "u", "randomSeed: 42, gamma: 1.0, relationshipWeightProperty: 'w', ", "communityId"),
        ("louvain", "u", "relationshipWeightProperty: 'w', ", "communityId"),
    ):
        reps = []
        for _ in range(repeats if algo in ("leiden", "louvain") else 1):
            rows = stream(driver, f"CALL gds.{algo}.stream('{graph}', {{{cfg}concurrency: {c}}}) YIELD nodeId, {col} RETURN {KEY}, {col} AS l")
            reps.append({r["key"]: r["l"] for r in rows})
        out[algo] = reps
    with driver.session() as s:
        out["leiden_stats"] = s.run(
            f"CALL gds.leiden.stats('u', {{randomSeed: 42, gamma: 1.0, relationshipWeightProperty: 'w', concurrency: {c}}}) "
            "YIELD communityCount, modularity, ranLevels, didConverge RETURN communityCount, modularity, ranLevels, didConverge"
        ).single().data()
        out["louvain_stats"] = s.run(
            f"CALL gds.louvain.stats('u', {{relationshipWeightProperty: 'w', concurrency: {c}}}) "
            "YIELD communityCount, modularity, ranLevels RETURN communityCount, modularity, ranLevels"
        ).single().data()
        pr_cfg = f"concurrency: {c}, dampingFactor: 0.85, maxIterations: 100, tolerance: 1e-10"
        out["pagerank_stats"] = s.run(
            f"CALL gds.pageRank.stats('g', {{{pr_cfg}}}) YIELD ranIterations, didConverge RETURN ranIterations, didConverge"
        ).single().data()
    rows = stream(driver, f"CALL gds.pageRank.stream('g', {{{pr_cfg}}}) YIELD nodeId, score RETURN {KEY}, score")
    out["pagerank"] = {r["key"]: r["score"] for r in rows}
    return out


def main():
    keys, arcs = read()
    driver = GraphDatabase.driver(URI, auth=("neo4j", PW))
    runs, loads = {}, {}
    for order in (1, 2):
        loads[order] = load(driver, keys, arcs, order)
        for c in (1, 4):
            runs[(order, c)] = run(driver, c, 3)
    with driver.session() as s:
        server = s.run("CALL dbms.components() YIELD name, versions, edition RETURN name, versions, edition").data()
        gds_version = s.run("CALL gds.version() YIELD gdsVersion RETURN gdsVersion").single()["gdsVersion"]
    driver.close()

    # references
    with open(f"{DIR}/ref_labels.csv") as f:
        ref = list(csv.DictReader(f))
    refl = {col: {r["key"]: r[col] for r in ref} for col in ref[0] if col != "key"}
    with open(f"{DIR}/ref_summary.json") as f:
        ref_summary = json.load(f)
    g = nx.MultiDiGraph()
    g.add_nodes_from(keys)
    g.add_edges_from((s, t) for _, s, t in arcs)
    nx_scc = {k: i for i, comp in enumerate(nx.strongly_connected_components(g)) for k in comp}
    nx_wcc = {k: i for i, comp in enumerate(nx.weakly_connected_components(g)) for k in comp}
    nx_pr = nx.pagerank(g, alpha=0.85, tol=1e-12, max_iter=1000)

    base = runs[(1, 1)]
    rep = {
        "data": "facts-layer reconstruction (approximates, does not reproduce, the normalized projection)",
        "server": server,
        "gds_version": gds_version,
        "loads": {str(k): v for k, v in loads.items()},
        "reference_summary": ref_summary,
        "determinism": {},
        "parity": {},
    }
    for algo in ("scc", "wcc", "leiden", "louvain"):
        bp = partition_of(base[algo][0])
        rep["determinism"][algo] = {
            f"order{o}_c{c}": {
                "partition_equal_to_base": [partition_of(x) == bp for x in r[algo]],
                "distinct_partitions_in_repeats": len({digest(partition_of(x)) for x in r[algo]}),
                "raw_labels_equal_to_base": r[algo][0] == base[algo][0],
                "groups": len(set(r[algo][0].values())),
                "ari_nmi_vs_base": ari_nmi(base[algo][0], r[algo][0], keys),
            }
            for (o, c), r in runs.items()
        }
    rep["determinism"]["leiden_stats"] = {f"order{o}_c{c}": r["leiden_stats"] for (o, c), r in runs.items()}
    rep["determinism"]["louvain_stats"] = {f"order{o}_c{c}": r["louvain_stats"] for (o, c), r in runs.items()}
    rep["determinism"]["pagerank"] = {
        f"order{o}_c{c}": {
            "bitwise_equal_to_base": all(r["pagerank"][k] == base["pagerank"][k] for k in keys),
            "max_abs_diff_to_base": max(abs(r["pagerank"][k] - base["pagerank"][k]) for k in keys),
            "rank_order_equal_to_base": sorted(keys, key=lambda k: (-r["pagerank"][k], k))
            == sorted(keys, key=lambda k: (-base["pagerank"][k], k)),
            "stats": r["pagerank_stats"],
        }
        for (o, c), r in runs.items()
    }

    # parity
    p = rep["parity"]
    p["scc"] = {
        "gds_equals_networkx": partition_of(base["scc"][0]) == partition_of(nx_scc),
        "gds_equals_petgraph": partition_of(base["scc"][0]) == partition_of(refl["scc"]),
        "networkx_equals_petgraph": partition_of(nx_scc) == partition_of(refl["scc"]),
        "count": len(set(base["scc"][0].values())),
    }
    p["wcc"] = {
        "gds_equals_networkx": partition_of(base["wcc"][0]) == partition_of(nx_wcc),
        "gds_equals_petgraph": partition_of(base["wcc"][0]) == partition_of(refl["wcc"]),
        "count": len(set(base["wcc"][0].values())),
    }
    gsum = sum(base["pagerank"].values())
    gnorm = {k: v / gsum for k, v in base["pagerank"].items()}
    refpr = {k: float(v) for k, v in refl["pagerank"].items()}
    top = lambda d, n: [k for k in sorted(keys, key=lambda k: (-d[k], k))[:n]]
    p["pagerank"] = {
        "gds_sum": gsum,
        "rust_ref_vs_networkx_max_abs": max(abs(refpr[k] - nx_pr[k]) for k in keys),
        "gds_normalised_vs_rust_ref_max_abs": max(abs(gnorm[k] - refpr[k]) for k in keys),
        "top20_overlap_gds_vs_ref": len(set(top(gnorm, 20)) & set(top(refpr, 20))),
        "top100_overlap_gds_vs_ref": len(set(top(gnorm, 100)) & set(top(refpr, 100))),
        "top20_order_equal": top(gnorm, 20) == top(refpr, 20),
        "dangling_vertices": sum(1 for k in keys if g.out_degree(k) == 0),
        "top5_gds": top(gnorm, 5),
        "top5_ref": top(refpr, 5),
    }
    leiden_seeds = [refl[f"leiden_s{s}"] for s in range(10)]
    p["communities"] = {
        "gds_leiden_vs_leidenrs_seed0_ari_nmi": ari_nmi(base["leiden"][0], leiden_seeds[0], keys),
        "gds_louvain_vs_leidenrs_seed0_ari_nmi": ari_nmi(base["louvain"][0], leiden_seeds[0], keys),
        "gds_leiden_vs_gds_louvain_ari_nmi": ari_nmi(base["leiden"][0], base["louvain"][0], keys),
        "leidenrs_seed0_vs_seeds1to9_ari_nmi": [ari_nmi(leiden_seeds[0], x, keys) for x in leiden_seeds[1:]],
        "gds_leiden_communities": len(set(base["leiden"][0].values())),
        "gds_leiden_modularity": base["leiden_stats"]["modularity"],
        "leidenrs_seed0_communities": ref_summary["leiden"][0]["communities"],
        "leidenrs_seed0_modularity": ref_summary["leiden"][0]["modularity"],
    }
    with open(OUT, "w") as f:
        json.dump(rep, f, indent=1, default=str)
    print(json.dumps({"determinism": {k: v for k, v in rep["determinism"].items()}, "parity": p}, default=str)[:9000])


if __name__ == "__main__":
    main()
