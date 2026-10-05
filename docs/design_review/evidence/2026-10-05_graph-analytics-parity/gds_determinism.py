"""GDS determinism and semantic-parity probe on a synthetic call-like graph.

Loads the same graph into Neo4j 2026.09.0 Community + GDS in two shuffled orders, runs each
algorithm at concurrency 1 and 4, re-keys results to semantic ids and compares:
  * across load orders and concurrency (determinism), and
  * against an independent reference (networkx / exact brute force) for meaning.
No timings are recorded (operator directive).

Usage: python gds_determinism.py bolt://127.0.0.1:17687 <password> <out.json>
"""

import hashlib
import json
import math
import random
import sys
import struct

import networkx as nx
from neo4j import GraphDatabase

SEED = 20261005
N = 300
DIM = 16


def build_graph(mode="modular"):
    rng = random.Random(SEED)
    if mode == "uniform":  # weak community structure: sensitive to order and threads
        sids = [f"pkg.mod{i // 20}.fn{i:03d}" for i in range(N)]
        isolates = set(sids[-12:])
        live = [s for s in sids if s not in isolates]
        arcs = [(rng.choice(live), rng.choice(live), 1.0 + rng.randrange(3), f"arc{k:05d}") for k in range(900)]
        emb = {s: [round(random.Random(SEED + i).uniform(-1, 1), 4) for _ in range(DIM)] for i, s in enumerate(sids)}
        return sids, isolates, arcs, emb
    sids = [f"pkg.mod{i // 20}.fn{i:03d}" for i in range(N)]
    isolates = set(sids[-12:])  # declared universe members with no arcs
    live = [s for s in sids if s not in isolates]
    arcs = []  # (src, dst, weight, arc_id) -- parallel arcs keep distinct ids
    # module-local clusters with cycles (SCCs), plus cross-module calls
    for m in range(0, len(live), 20):
        block = live[m : m + 20]
        for i, s in enumerate(block):
            for _ in range(2):
                t = block[rng.randrange(len(block))]
                arcs.append((s, t, 1.0 + rng.randrange(3), None))
            if i % 7 == 0:  # back edge -> cycle
                arcs.append((s, block[0], 1.0, None))
    for _ in range(80):
        s, t = rng.choice(live), rng.choice(live)
        arcs.append((s, t, 1.0, None))
    # explicit parallel arcs and self-loops
    for s, t in [(live[1], live[2]), (live[1], live[2]), (live[5], live[5])]:
        arcs.append((s, t, 1.0, None))
    arcs = [(s, t, w, f"arc{k:05d}") for k, (s, t, w, _) in enumerate(arcs)]
    # embeddings with deliberate exact ties (duplicated vectors)
    emb = {}
    for i, s in enumerate(sids):
        r = random.Random(SEED + (i // 3 if i < 30 else i))  # groups of 3 identical vectors
        emb[s] = [round(r.uniform(-1, 1), 4) for _ in range(DIM)]
    return sids, isolates, arcs, emb


def load(driver, sids, arcs, emb, order_seed):
    rng = random.Random(order_seed)
    nodes = list(sids)
    rels = list(arcs)
    rng.shuffle(nodes)
    rng.shuffle(rels)
    with driver.session() as s:
        s.run("CALL gds.graph.drop('g', false) YIELD graphName RETURN graphName").consume()
        s.run("CALL gds.graph.drop('u', false) YIELD graphName RETURN graphName").consume()
        s.run("MATCH (n) DETACH DELETE n").consume()
        s.run(
            "UNWIND $rows AS r CREATE (:Fn {sid: r.sid, emb: r.emb})",
            rows=[{"sid": x, "emb": emb[x]} for x in nodes],
        ).consume()
        s.run("CREATE INDEX fn_sid IF NOT EXISTS FOR (n:Fn) ON (n.sid)").consume()
        s.run("CALL db.awaitIndexes()").consume()
        s.run(
            "UNWIND $rows AS r MATCH (a:Fn {sid: r.s}), (b:Fn {sid: r.t}) "
            "CREATE (a)-[:CALLS {w: r.w, arc: r.arc}]->(b)",
            rows=[{"s": a, "t": b, "w": w, "arc": k} for a, b, w, k in rels],
        ).consume()
        # directed projection, parallel arcs preserved (GDS default aggregation)
        s.run(
            "CALL gds.graph.project('g', {Fn: {properties: 'emb'}}, "
            "{CALLS: {properties: 'w'}}) YIELD nodeCount, relationshipCount "
            "RETURN nodeCount, relationshipCount"
        ).consume()
        # undirected projection for communities, parallel arcs summed
        s.run(
            "CALL gds.graph.project('u', 'Fn', {CALLS: {orientation: 'UNDIRECTED', "
            "properties: {w: {property: 'w', aggregation: 'SUM'}}}})"
        ).consume()
        rec = s.run(
            "CALL gds.graph.list() YIELD graphName, nodeCount, relationshipCount "
            "RETURN collect([graphName, nodeCount, relationshipCount]) AS g"
        ).single()
    return rec["g"]


def stream(driver, cypher):
    with driver.session() as s:
        return [r.data() for r in s.run(cypher)]


def partition(rows, key):
    groups = {}
    for r in rows:
        groups.setdefault(r[key], []).append(r["sid"])
    return sorted(sorted(g) for g in groups.values())


def digest(obj):
    return hashlib.sha256(json.dumps(obj, sort_keys=True).encode()).hexdigest()[:16]


def run_all(driver, c):
    out = {}
    sid = "gds.util.asNode(nodeId).sid AS sid"
    rows = stream(driver, f"CALL gds.scc.stream('g', {{concurrency: {c}}}) YIELD nodeId, componentId RETURN {sid}, componentId")
    out["scc"] = {"partition": partition(rows, "componentId"), "raw": sorted((r["sid"], r["componentId"]) for r in rows)}
    rows = stream(driver, f"CALL gds.wcc.stream('g', {{concurrency: {c}}}) YIELD nodeId, componentId RETURN {sid}, componentId")
    out["wcc"] = {"partition": partition(rows, "componentId"), "raw": sorted((r["sid"], r["componentId"]) for r in rows)}
    rows = stream(
        driver,
        f"CALL gds.leiden.stream('u', {{concurrency: {c}, randomSeed: 42, gamma: 1.0, theta: 0.01, "
        f"maxLevels: 10, relationshipWeightProperty: 'w'}}) YIELD nodeId, communityId RETURN {sid}, communityId",
    )
    out["leiden"] = {"partition": partition(rows, "communityId"), "raw": sorted((r["sid"], r["communityId"]) for r in rows)}
    with driver.session() as s:
        st = s.run(
            f"CALL gds.leiden.stats('u', {{concurrency: {c}, randomSeed: 42, relationshipWeightProperty: 'w'}}) "
            "YIELD communityCount, modularity, ranLevels, didConverge, modularities "
            "RETURN communityCount, modularity, ranLevels, didConverge, modularities"
        ).single().data()
    out["leiden_stats"] = st
    rows = stream(
        driver,
        f"CALL gds.louvain.stream('u', {{concurrency: {c}, relationshipWeightProperty: 'w'}}) "
        f"YIELD nodeId, communityId RETURN {sid}, communityId",
    )
    out["louvain"] = {"partition": partition(rows, "communityId"), "raw": sorted((r["sid"], r["communityId"]) for r in rows)}
    pr_cfg = f"concurrency: {c}, dampingFactor: 0.85, maxIterations: 1000, tolerance: 1e-10, relationshipWeightProperty: 'w'"
    rows = stream(driver, f"CALL gds.pageRank.stream('g', {{{pr_cfg}}}) YIELD nodeId, score RETURN {sid}, score")
    out["pagerank"] = {r["sid"]: r["score"] for r in rows}
    with driver.session() as s:
        out["pagerank_stats"] = s.run(
            f"CALL gds.pageRank.stats('g', {{{pr_cfg}}}) YIELD ranIterations, didConverge, configuration "
            "RETURN ranIterations, didConverge, configuration"
        ).single().data()
    try:
        rows = stream(
            driver,
            f"CALL gds.knn.stream('g', {{concurrency: {c}, nodeProperties: {{emb: 'COSINE'}}, topK: 3, "
            f"sampleRate: 1.0, deltaThreshold: 0.0, maxIterations: 100, randomSeed: 7, "
            f"randomJoins: 10, initialSampler: 'uniform'}}) YIELD node1, node2, similarity "
            "RETURN gds.util.asNode(node1).sid AS a, gds.util.asNode(node2).sid AS b, similarity",
        )
        out["knn"] = sorted((r["a"], r["b"], r["similarity"]) for r in rows)
    except Exception as e:  # recorded, not hidden
        out["knn"] = {"error": f"{type(e).__name__}: {getattr(e, 'gql_status', '')} {str(e)[:300]}"}
    # repeated executions on the same projection and load (run-to-run variance)
    reps = {}
    for algo, extra in (("leiden", "randomSeed: 42, "), ("louvain", "")):
        digests = []
        for _ in range(5):
            rows = stream(
                driver,
                f"CALL gds.{algo}.stream('u', {{concurrency: {c}, {extra}relationshipWeightProperty: 'w'}}) "
                f"YIELD nodeId, communityId RETURN {sid}, communityId",
            )
            digests.append(digest(partition(rows, "communityId")))
        reps[algo] = digests
    out["repeats"] = reps
    return out


def references(sids, isolates, arcs, emb):
    g = nx.MultiDiGraph()
    g.add_nodes_from(sids)
    for s, t, w, k in arcs:
        g.add_edge(s, t, key=k, weight=w)
    ref = {
        "scc": sorted(sorted(c) for c in nx.strongly_connected_components(g)),
        "wcc": sorted(sorted(c) for c in nx.weakly_connected_components(g)),
    }
    # networkx pagerank on a multigraph sums parallel weights; dangling mass spread uniformly
    ref["pagerank"] = nx.pagerank(g, alpha=0.85, tol=1e-12, max_iter=1000, weight="weight")

    def cos(a, b):
        da = math.sqrt(sum(x * x for x in a))
        db = math.sqrt(sum(x * x for x in b))
        return sum(x * y for x, y in zip(a, b)) / (da * db)

    knn = {}
    for a in sids:
        sims = sorted(((cos(emb[a], emb[b]), b) for b in sids if b != a), key=lambda x: (-x[0], x[1]))
        knn[a] = sims
    ref["knn"] = knn
    return ref


def main():
    uri, pw, out_path = sys.argv[1:4]
    mode = sys.argv[4] if len(sys.argv) > 4 else "modular"
    sids, isolates, arcs, emb = build_graph(mode)
    driver = GraphDatabase.driver(uri, auth=("neo4j", pw))
    runs = {}
    projections = {}
    for order in (1, 2):
        projections[order] = load(driver, sids, arcs, emb, order_seed=order)
        for c in (1, 4):
            runs[(order, c)] = run_all(driver, c)
    with driver.session() as s:
        version = s.run("CALL gds.version() YIELD gdsVersion RETURN gdsVersion").single()["gdsVersion"]
        lic = s.run("CALL gds.license.state() YIELD isLicensed, details RETURN isLicensed, details").single().data()
        server = s.run("CALL dbms.components() YIELD name, versions, edition RETURN name, versions, edition").data()
    driver.close()

    ref = references(sids, isolates, arcs, emb)
    base = runs[(1, 1)]
    report = {
        "mode": mode,
        "graph": {
            "vertices": len(sids),
            "isolates": len(isolates),
            "arcs": len(arcs),
            "parallel_pairs": sum(1 for (s, t) in {(a, b) for a, b, _, _ in arcs} if sum(1 for a, b, _, _ in arcs if (a, b) == (s, t)) > 1),
            "self_loops": sum(1 for a, b, _, _ in arcs if a == b),
        },
        "server": server,
        "gds_version": version,
        "gds_license": lic,
        "projections_per_order": {str(k): v for k, v in projections.items()},
        "determinism": {},
        "parity": {},
    }
    for algo in ("scc", "wcc", "leiden", "louvain"):
        report["determinism"][algo] = {
            f"order{o}_c{c}": {
                "partition_equal_to_base": runs[(o, c)][algo]["partition"] == base[algo]["partition"],
                "raw_labels_equal_to_base": runs[(o, c)][algo]["raw"] == base[algo]["raw"],
                "groups": len(runs[(o, c)][algo]["partition"]),
                "partition_digest": digest(runs[(o, c)][algo]["partition"]),
            }
            for (o, c) in runs
        }
    report["determinism"]["repeats_same_load"] = {f"order{o}_c{c}": {a: len(set(d)) for a, d in r["repeats"].items()} for (o, c), r in runs.items()}
    report["determinism"]["leiden_stats"] = {f"order{o}_c{c}": runs[(o, c)]["leiden_stats"] for (o, c) in runs}

    def bits(x):
        return struct.pack("<d", x).hex()

    pr = {}
    for (o, c), r in runs.items():
        diffs = [abs(r["pagerank"][s] - base["pagerank"][s]) for s in sids]
        pr[f"order{o}_c{c}"] = {
            "bitwise_equal_to_base": all(bits(r["pagerank"][s]) == bits(base["pagerank"][s]) for s in sids),
            "max_abs_diff_to_base": max(diffs),
            "rank_order_equal_to_base": sorted(sids, key=lambda s: (-r["pagerank"][s], s))
            == sorted(sids, key=lambda s: (-base["pagerank"][s], s)),
            "stats": {k: v for k, v in r["pagerank_stats"].items() if k != "configuration"},
        }
    report["determinism"]["pagerank"] = pr
    report["pagerank_configuration_echo"] = base["pagerank_stats"]["configuration"]
    def sims_by_node(rows):
        d = {}
        for a, b, sim in rows:
            d.setdefault(a, []).append(round(sim, 9))
        return {a: sorted(v, reverse=True) for a, v in d.items()}

    report["determinism"]["knn"] = {
        f"order{o}_c{c}": (
            r["knn"]
            if isinstance(r["knn"], dict)
            else {
                "pairs_equal_to_base": r["knn"] == base["knn"],
                "similarity_values_equal_to_base": sims_by_node(r["knn"]) == sims_by_node(base["knn"]),
                "pairs": len(r["knn"]),
                "digest": digest(r["knn"]),
            }
        )
        for (o, c), r in runs.items()
    }

    # semantic parity with independent references
    report["parity"]["scc_equals_networkx"] = base["scc"]["partition"] == ref["scc"]
    report["parity"]["wcc_equals_networkx"] = base["wcc"]["partition"] == ref["wcc"]
    report["parity"]["isolates_present_in_scc_output"] = all([s] in base["scc"]["partition"] for s in isolates)
    report["parity"]["isolates_present_in_leiden_output"] = sum(1 for s in isolates if any(s in g for g in base["leiden"]["partition"]))
    gds_pr_sum = sum(base["pagerank"].values())
    nx_pr = ref["pagerank"]
    scale = {s: base["pagerank"][s] / gds_pr_sum for s in sids}
    report["parity"]["pagerank"] = {
        "gds_score_sum": gds_pr_sum,
        "networkx_score_sum": sum(nx_pr.values()),
        "max_abs_diff_after_normalising_gds_to_sum1": max(abs(scale[s] - nx_pr[s]) for s in sids),
        "top10_gds": sorted(sids, key=lambda s: (-base["pagerank"][s], s))[:10],
        "top10_networkx": sorted(sids, key=lambda s: (-nx_pr[s], s))[:10],
        "isolate_score_gds": base["pagerank"][sorted(isolates)[0]],
        "isolate_score_networkx": nx_pr[sorted(isolates)[0]],
    }
    if not isinstance(base["knn"], dict):
        got = {}
        for a, b, sim in base["knn"]:
            got.setdefault(a, []).append((sim, b))
        exact_sets = 0
        exact_up_to_ties = 0
        mismatches = []
        for a in sids:
            g_list = sorted(got.get(a, []), key=lambda x: (-x[0], x[1]))
            # GDS COSINE similarity is reported as (cos + 1) / 2
            ref3 = [((x + 1) / 2, b) for x, b in ref["knn"][a][:3]]
            if [b for _, b in g_list] == [b for _, b in ref3]:
                exact_sets += 1
            # equal similarity multiset means same answer up to ties
            if [round(s, 9) for s, _ in g_list] == [round(s, 9) for s, _ in ref3]:
                exact_up_to_ties += 1
            elif len(mismatches) < 5:
                mismatches.append({"node": a, "gds": g_list, "exact": ref3})
        report["parity"]["knn_sampleRate1"] = {
            "nodes": len(sids),
            "nodes_with_identical_neighbour_list_semantic_id_tiebreak": exact_sets,
            "nodes_with_identical_similarity_values_after_(cos+1)/2": exact_up_to_ties,
            "sample_mismatches": mismatches,
        }
    with open(out_path, "w") as f:
        json.dump(report, f, indent=1, sort_keys=True, default=str)
    print(json.dumps({"determinism": {k: v for k, v in report["determinism"].items() if k != "leiden_stats"}, "parity": report["parity"]}, indent=1, default=str)[:6000])


if __name__ == "__main__":
    main()
