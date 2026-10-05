# Graph analytics parity and determinism — 2026-10-05

Consumer: the SurrealDB pivot design review (Phase B, lane W3). This is library-research
evidence and makes no adoption decision. Directives: judge functional intent and contracts, not
like-for-like mechanism. The contracts are determinism, declared universes with isolates and
parallel arcs, recorded parameters and seeds, explicit truncation, provenance, and heuristics
never stated as facts. **No timing or benchmark comparisons are recorded.**
Server `time` fields are dropped from raw outputs.

**Status: complete within the available data.** The synthetic probes, the static parity
analysis and real-data probes on a facts-layer call graph are done.

**Data status.** The FastMCP 4.0.5 behavioral compile (generation schema
`lctx_g786cd6d58dc5dccea686c0a54a8f0dcd`) failed after the facts stage: its own 30 s
idle-in-transaction timeout killed the lifecycle connection, a product defect. Only the facts
layer is populated: 233 tables, about 16.6M rows. The normalized, analysis and catalog
relations are empty (`normalized_call_alternatives`, `occurrence_ownership`, `entity_refs`,
`projection_source_assessments`: 0 rows). Parity against *published* analytics is therefore
**blocked**. The real-data probes use a facts-layer reconstruction, which **approximates the
normalized projection and does not reproduce it** (see "Real data").
The schema was read only as `lctx_superuser` with `default_transaction_read_only = on`; the
generation was not altered.

## Question

Can Neo4j GDS (Community 2026.09.0) and SurrealDB 3.3.0 deliver our analytic outcomes with our
contracts? The outcomes are SCC, WCC, communities, PageRank, kNN, bounded traversal,
reachability and shortest path; the contracts are those in
[analytics §9](../../../design/sections/analytics.md#section-9).

## Versions and machine

| Component | Version | How established |
|---|---|---|
| Neo4j | 2026.09.0 community (`dbms.components()`), image `neo4j:2026.09.0-community-trixie` (local id `f6c218c106ad`) | `raw/gds_determinism_*.json` `server` |
| GDS | 2026.09.0, `isLicensed: false` ("No valid GDS license specified.") | `gds.version()`, `gds.license.state()` |
| APOC | installed via `NEO4J_PLUGINS` from the image | container log |
| Python | neo4j 6.3.1, CPython 3.14.7, networkx 3.7, requests 2.34.2, numpy/scipy (networkx PageRank) | scratch venv `build/review-probes/graph-analytics-parity/.venv` |
| SurrealDB | 3.3.0 (`/version` -> `surrealdb-3.3.0`), image `surrealdb/surrealdb:v3.3.0`, memory engine, HTTP `/sql` | `curl /version` |
| neo4rs | 0.9.0-rc.10, tokio 1.53.2, toolchain nightly-2026-09-29, built outside the workspace | `neo4rs_smoke.Cargo.toml`; lock in the build dir |

Machine: Linux 7.0.0-38-generic, 32 CPUs, 188 GiB RAM, Docker. A full real-library compile ran
at the same time, so the probes were kept light (Neo4j heap 2 GiB; cargo `-j4` in a private
build dir).

## Files

- `run_all.sh [schema]`: reproduces every synthetic probe. It starts the two containers
  (SurrealDB memory-capped at 4 GiB), runs the scripts and the neo4rs smoke test, then removes
  the containers and their anonymous volumes. With a generation schema it also runs the
  real-data chain: export, Rust reference, `gds_real.py`, `surreal_real.py`. For the real graph
  both containers are capped at 6 GiB.
- `export_facts_graph.sh`: read-only export of the facts-layer call graph and containment to
  CSV under `build/review-probes/graph-analytics-parity/facts-graph/` (gitignored, about
  20 MB). It uses `COPY (WITH …) TO STDOUT`, because a read-only transaction refuses temp
  objects.
- `petgraph_ref.rs` / `petgraph_ref.Cargo.toml`: in-process reference with petgraph 0.8.3 and
  leiden-rs 0.8.1, following our kernels' stated conventions. It covers SCC, condensation, WCC,
  PageRank with uniform dangling redistribution and a residual, and Leiden with RBER quality,
  resolution 1.0 and seeds 0–9. It is an independent re-implementation, not product code.
- `gds_real.py`: GDS determinism and parity on the real graph against networkx and the Rust
  reference.
- `surreal_real.py`: SurrealDB traversal correctness on the real graph against networkx, plus
  deliberate capped `+path` probes.
- `gds_determinism.py`: P2/P3. It builds one synthetic graph and loads it in two shuffled
  orders (node and relationship order), then runs each algorithm at `concurrency` 1 and 4.
  Each Leiden/Louvain call is repeated 5 times on the same load. Results are re-keyed to
  semantic ids, and partitions are compared as sets of members. The script also compares
  against networkx and an exact brute-force cosine kNN.
  - Mode `modular`: 300 vertices (12 isolates), 703 arcs, 21 parallel pairs, 45 self-loops,
    modules with cycles, and embeddings with deliberate groups of identical vectors.
  - Mode `uniform`: 300 vertices, 900 uniformly random arcs, so community structure is weak.
    The `uniform` embedding generator is degenerate: a reseeded RNG per component makes every
    vector constant, so its kNN is an all-ties case (cosine ±1) and is reported as such.
- `surreal_traversal.py`: P4. It compares SurrealDB recursive idioms with networkx on:
  - G1, an 8-node graph with every hazard (cycle, parallel arcs, self-loop, isolate,
    alternative routes);
  - G2, a 300-node chain (depth limit);
  - G3, diamond chains (2^k paths);
  - G4, the modular 300-node graph.

  The risky unbounded queries run last, guarded, with a container restart after a crash.
  `surreal_g4_diagnose.py` reloads G4, because the memory engine loses it in that crash, and
  explains the G4 differences.
- `neo4rs_smoke.rs` and `neo4rs_smoke.Cargo.toml`: P5.
- `raw/`:
  - `gds_determinism_run1.json`: first run, PageRank `maxIterations: 100`, kNN not rescaled;
  - `gds_determinism_modular.json` and `gds_determinism_uniform.json`: final runs, PageRank
    `maxIterations: 1000`;
  - `surreal_traversal.json`, `surreal_g4_diagnose.json`, `neo4rs_smoke_bolt4.txt`;
  - real data: `facts_graph_summary.csv`, `petgraph_ref_summary.json`, `gds_real.json`,
    `surreal_real.json`.

Commands as run (`run_all.sh` is the reproducible form):

```bash
python gds_determinism.py bolt://127.0.0.1:17687 <pw> raw/gds_determinism_<mode>.json <modular|uniform>
python surreal_traversal.py http://127.0.0.1:18000 <pw> raw/surreal_traversal.json
python surreal_g4_diagnose.py http://127.0.0.1:18000 <pw> raw/surreal_g4_diagnose.json
NEO4J_PROBE_PASSWORD=<pw> build/review-probes/graph-analytics-parity/neo4rs-smoke/target/debug/neo4rs-smoke 127.0.0.1:17687
export_facts_graph.sh lctx_g786cd6d58dc5dccea686c0a54a8f0dcd build/review-probes/graph-analytics-parity/facts-graph
build/review-probes/graph-analytics-parity/petgraph-ref/target/release/petgraph-ref build/review-probes/graph-analytics-parity/facts-graph
python gds_real.py bolt://127.0.0.1:17687 <pw> build/review-probes/graph-analytics-parity/facts-graph raw/gds_real.json
python surreal_real.py http://127.0.0.1:18000 <pw> build/review-probes/graph-analytics-parity/facts-graph raw/surreal_real.json
```

## Results

| Probe | Outcome | Command / raw |
|---|---|---|
| P1 reconstruct projection edges from normalized relations, verify against stored counts | **blocked**: normalized relations and assessments are empty (compile stopped after facts). Facts-layer approximation **passed** (export completed, counts below). | `export_facts_graph.sh`, `raw/facts_graph_summary.csv` |
| P2 GDS parity, synthetic, against networkx / exact references | **passed** (probe completed); findings below | `gds_determinism.py`, both modes |
| P2 GDS parity against the generation's published analytics | **blocked**: no published analysis relations | — |
| P2/P3 GDS parity and determinism on the real facts graph against networkx and the petgraph/leiden-rs reference | **passed** (probe completed); findings under "Real data" | `gds_real.py`, `raw/gds_real.json` |
| Rust reference repeatability | **passed**: rerun produced byte-identical labels and summary | `petgraph-ref` run twice, `cmp` |
| P3 GDS determinism under shuffled load order and concurrency 1/4 | **passed** (probe completed); several algorithms are **not deterministic** | `gds_determinism.py` |
| P4 SurrealDB traversal semantics against networkx, synthetic | **passed** (probe completed); findings below, including a server OOM | `surreal_traversal.py`, `surreal_g4_diagnose.py` |
| P4 SurrealDB traversal on the real facts graph | **passed** (probe completed); bounded `+path` and `TIMEOUT` findings under "Real data" | `surreal_real.py`, `raw/surreal_real.json` |
| P5 neo4rs smoke (connect, UNWIND write, read, GDS project/pageRank/drop) | **passed**, 8/8 steps | `raw/neo4rs_smoke_bolt4.txt` |

### P3 GDS determinism (synthetic, both modes)

"Partition" means membership compared by semantic id; "labels" means GDS's raw numeric ids.
Base = load order 1, concurrency 1.

| Algorithm | Same load, repeated | Concurrency 1 vs 4 (same load) | Shuffled load order | Raw labels |
|---|---|---|---|---|
| SCC | stable | equal | **equal partition** | change with load order |
| WCC | stable | equal | **equal partition** | change with load order |
| Leiden (`randomSeed: 42`) | c1: 1 distinct partition in 5 runs. c4: **2 and 4 distinct (modular), 5 of 5 (uniform)** | modular: equal; uniform: **differs** | modular: equal; uniform: **differs** (c1: 25 vs 24 communities) | change |
| Louvain (no seed parameter) | 1 distinct in 5 runs at c1 and c4 | equal | **differs in both modes** (modular: 27 groups each, different membership; uniform: 26 vs 25) | change |
| PageRank (weighted, d=0.85, tol 1e-10) | bitwise stable | **bitwise equal** | not bitwise: max abs diff 8.9e-16 (modular), 6.7e-16 (uniform); **rank order equal** | n/a |
| kNN (`sampleRate: 1.0, randomSeed: 7`) | stable at c1 | **c4 refused**: 52N37 "Configuration parameter 'randomSeed' may only be set if parameter 'concurrency' is equal to 1" | modular: **pairs and similarity values differ**; uniform (all ties): values equal, pairs differ | n/a |

At concurrency 4 Leiden's separate `stats` calls also reported different modularity for the
same seed (uniform: 0.4576 / 0.4540 / 0.4536 / 0.4578). GDS does not refuse `randomSeed` at
concurrency 4 for Leiden as it does for kNN. The seed alone does not make Leiden repeatable.

### P2 GDS semantic parity (synthetic, independent references)

- **SCC and WCC equal networkx** in both modes, with isolates as singletons. Parallel arcs and
  self-loops do not disturb the result.
- **PageRank is unnormalised.** Scores sum to 289.8 for N=300 (modular), and an isolate scores
  exactly 0.15 = 1 − d, so dangling mass is not redistributed. Rescaled to sum 1, the scores
  agree with networkx (uniform teleport, dangling spread uniformly) to 1.1e-10 (modular) and
  6.1e-12 (uniform), with an identical top 10.
  - Diagnostics are `ranIterations` and `didConverge` only, with no residual.
  - At `maxIterations: 100` the modular graph did **not** converge (`didConverge: false`,
    run1); it is reported, not refused. At 1000 it converged in 146 iterations (uniform: 108).
  - The configuration echo (`pagerank_configuration_echo`) records every parameter plus a
    `jobId`.
- **kNN with `sampleRate: 1.0` is not exact.** On the modular graph only 229 of 300 nodes have
  the exact top-3 similarity values; for the rest GDS missed a true neighbour. Example
  `pkg.mod1.fn031`: GDS's third neighbour has similarity 0.7675, where the exact third is
  0.8132.
  - GDS `COSINE` is reported as (cos+1)/2 (0.8075 ↔ cos 0.6149).
  - Ties are not broken canonically: equal-similarity neighbours change with load order.
- **Communities:** in the undirected projection, Leiden and Louvain put every isolate in a
  singleton community (12 of 12 present in the output).

### P4 SurrealDB 3.3.0 traversal semantics (synthetic)

- **One hop preserves parallel arcs:** `fn:1->calls->fn` returns `[fn:7, fn:2, fn:2]`. Reverse
  arrows work.
- **`+collect` (reachability)** returns reached nodes in discovery order, not a canonical order.
  - On a cycle it **includes the start node**; networkx `descendants` excludes it. With that
    definitional difference, it agreed on 20/20 G4 starts.
  - Bounded `{1..3+collect}` agreed on 10/20 directly and on the other 10/20 once the start
    node is added back where a cycle of length ≤ 3 returns to it.
  - Unbounded `+collect` on a cycle terminates. An isolate returns `[]`.
- **`+shortest`** agrees on length with networkx on 20/20 G4 pairs, and each returned path is
  a real path. "Unreachable" and "beyond the bound" both return `null`, so they cannot be told
  apart. A self-loop target returns `[fn:4]`.
- **Depth bound:**
  - An explicit bound truncates **silently**: on a chain with 299 reachable nodes,
    `{1..10+collect}` returns 10 rows and no truncation flag.
  - Unbounded recursion past 256 errors ("Exceeded the idiom recursion limit of 256.").
  - A bound above 256 is refused ("Found 300 for bound but expected 256 at most.").
  - Upstream's language test `recursion_strategy_limits.surql` pins this contract.
- **`+path`** returns maximal **walks**, not simple paths. On G1 at depth ≤ 4 it returned 13
  walks against 8 simple paths in networkx.
  - Nodes repeat (2-3-1-2, the self-loop 4-4).
  - Parallel arcs yield **duplicate, indistinguishable node lists**, because edge identity is
    not in the path.
  - Walk counts on the 8-node cyclic G1 grow 13, 53, 157, 469, 1229 for depth bounds
    4, 8, 12, 16, 20.
  - On diamond chains the counts are exact (2^k, up to 16,384 at k=14). The G3 `last()`
    sub-query was a harness error, so those fields are void.
- **Unbounded `+path` or plain `{..}` on the 8-node cyclic G1 kills the server** instead of
  raising the documented 256 error.
  - The first, uncapped run: `fn:1.{..+path}->calls->fn` and then `fn:1.{..}->calls->fn` each
    ended with the container OOM-killed (`docker inspect`: `OOMKilled true`, exit 137) after
    consuming host memory.
  - Reruns capped at 4 GiB reproduced both (`raw/surreal_traversal.json`,
    `G1_risky_unbounded_on_cycle`).
  - The memory engine loses all data on such a crash.

  For any query reachable from serving, every recursive idiom therefore needs an explicit
  small bound or a `TIMEOUT`, and that bound truncates silently.

### P5 neo4rs 0.9.0-rc.10

All 8 steps passed against 2026.09.0. The steps were: connect, `dbms.components`, an UNWIND
write of a list of maps, typed row reads, then `gds.graph.project`, `gds.pageRank.stream` and
`gds.graph.drop`. The PageRank values (a=0.3865 b=0.4785 c=0.5566 d=0.3865) equal the skill's
GD01@2026-09-27 result on the same graph.

Source inspection (`neo4rs-0.9.0-rc.10/src/version.rs`) shows the driver **offers only Bolt
4.0–4.4**, whatever `unstable-*` features are enabled, and the 2026.09 server accepted Bolt
4.x. Bolt 5/6-only features are therefore out of reach from Rust: element-id strings,
VECTOR and UUID values, and GQL-status notifications. [interp]

## Static parity against our contracts (analytics §9; `lctx-model::domain::analytics`)

Our retained policy (`analytics/policy.rs` `RETAINED`) is:
- PageRank: damping 0.85, tolerance 1e-10, 100 iterations, directed count weights, uniform
  teleport and dangling redistribution, residual retained (`ranking.rs`);
- Leiden: RBER quality via leiden-rs 0.8.1, resolutions 0.5/1/2/4 × 10 seeds, selected 1.0;
  undirected conversion drops self-loops and non-positive weights; canonical labels by first
  sorted appearance (`communities.rs`);
- kNN: exact cosine, top 3, floor 0.5, canonical tie-breaking (`neighbours.rs`);
- projections: universe `InputEntitiesAndContextTargets`, `PreserveTypedParallelArcs`,
  `ArcId` lineage (`projection.rs`).

| Analytic outcome | GDS functional fit | Semantic consequence if adopted |
|---|---|---|
| SCC / WCC | Delivers the partition, deterministic in membership (P3). The universe and isolates are expressible; parallel arcs are harmless. | Usable once re-keyed to semantic ids; GDS ids must never be persisted. The callee-first schedule still needs a condensation, because GDS `topologicalSort` drops cyclic regions (documented). |
| Communities (Leiden) | Modularity with `gamma` (divided by total weight), not RBER. Undirected only. The seed is honoured only at concurrency 1 **and** a stable load order. | A different quality function is a different heuristic and needs its own declared definition. Determinism requires concurrency 1 and a canonical load order (sorted by semantic id): achievable, but an observation, not a documented contract. Ten-seed profiles map to ten calls. Per-level modularities replace our per-iteration quality history. |
| Louvain | No seed; membership depends on load order. | Not publishable as a heuristic without a canonical load order, and even then only observed-stable. |
| PageRank with diagnostics | Same ranking meaning. Scores are unnormalised with no dangling redistribution. Diagnostics are `didConverge`/`ranIterations`, no residual; non-convergence is reported, not refused. | Scores need normalising and a declared definition. Our residual diagnostic would be lost. Across load orders the scores differ by at most 1e-15 and the rank order is identical. |
| kNN over embeddings | `gds.knn` is approximate even at `sampleRate: 1.0`. It rescales cosine, breaks ties non-canonically and forbids a seed above concurrency 1. | **Does not deliver** our exact-neighbour contract (P2: 71 of 300 nodes wrong). Exact kNN stays in-process, or as a pgvector exact scan. |
| Bounded delegation traversal | GDS `bfs`/`dfs` with `maxDepth`, or Cypher; not probed here. | Explicit budgets and stop inventories stay ours. |
| SurrealDB traversal as a substitute | `+collect`/`+shortest` correct up to the start-node definition. Silent bound truncation; unreachable and over-bound indistinguishable; `+path` returns walks with duplicate parallel-arc paths; unbounded `+path`/`{..}` can OOM the server. | Usable only behind a wrapper that supplies explicit truncation, edge identity and canonical order. The OOM path must be made unreachable (bounded idioms, query `TIMEOUT`, memory caps). |
| FCA/RCA, condition-carrying summaries | Provided by neither engine (static search in A4). | Remain in-process. |

### P1 reconstruction (static, `lctx-model/src/domain/projection.rs`, `projection/normalization.rs`)

Each projection is rebuilt per (input, context, `ProjectionName`, version 3) from normalized
relations by endpoint role (`ProjectionSpec::roles`). Endpoints are `Id<EntityRef>`, and the
arc id is the typed source row (`ArcId`).

| Projection | Role | Arc rule |
|---|---|---|
| `CallableInvocation` | `Invocation` | `normalized_call_alternatives` rows selected by the invocation index: owner → resolved `entity`. Rows not selected, unresolved, ambiguous, outside the universe or override-dispatched become `projection_gaps` with a reason code. |
| `DefinitionContainment` | `Definition` | The same rule, selecting call phase Definition/Decorator. |
| `DefinitionContainment` | `Containment` | `occurrence_ownerships` of the input: entity → occurrence. |
| `DefinitionContainment` | `SourceDefinition` | `occurrence_ownerships` of the input: entity → callable, where the occurrence is a definition. |

`projection_source_assessments` records `vertices`, `arcs`, `gaps` and `availability`. The
petgraph snapshot is stored in `projection_snapshots` / `projection_snapshot_chunks`.
Checking a SQL reconstruction against the stored counts is the real-data step.

## Real data (facts-layer reconstruction)

### Reconstruction (`export_facts_graph.sh`)

This **approximates** the normalized `CallableInvocation` projection and does not reproduce
it. The normalized layer's invocation-index selection, entity universe, override expansion and
gap reasons were never computed for this generation.

- **Vertices** (36,954): every provider callable and every provider function/method/class
  symbol, plus every callee. The key is
  `<context8>|<S|M|C|D>|<module qualified name>|<native_key>`; native keys are opaque provider
  keys. Two analysis contexts are kept apart by the key prefix; there are 0 cross-context arcs.
- **Arcs** (118,595 drawn of 142,039 `call_target_observations`): one arc per observation, so
  parallel arcs are preserved (104 self-loops).
  - The caller comes from `provider_call_sites` for the same (site, origin). That join is 1:1
    in this generation: every observation has exactly one site and caller. Joining on
    `qualification` as well would lose 81,947 rows, because sites and targets carry different
    qualifications.
  - The callee is the destination: Resolved (kind 0), Overrides (kind 2, the dispatch set's
    named member) or Callable (kind 3).
  - 23,444 Unresolved/SyntheticFormatting destinations are gaps: counted, not drawn.
  - All call phases are drawn, including New/Init/PropertyGet/PropertySet; the invocation
    index's phase filter is not applied.
- **Containment** (19,024): module body → member symbols, and class body → class. This replaces
  the empty `occurrence_ownership`. It was exported but not analysed.

Shape according to the Rust reference:
- 36,929 SCCs, of which 6 are non-trivial; the largest has 15 nodes, and its anchor is
  `fastmcp.client.client` `F:46`;
- a condensation of 36,929 nodes and 82,930 edges;
- 16,529 WCCs;
- 20,599 dangling vertices;
- a maximum BFS depth of 13 among the sampled starts.

### GDS on the real graph (`raw/gds_real.json`)

Projections from both load orders had identical counts (`g` 36,954 / 118,595; `u` 166,188
undirected relationships, self-loops excluded). The Cypher aggregation keeps a→b and b→a as two
undirected relationships: 6 mutual pairs, where the reference merges each into one pair with
summed weight.

| Algorithm | Determinism (load order 1/2 × concurrency 1/4, Leiden/Louvain 3 repeats) | Parity |
|---|---|---|
| SCC | identical partition in every run | **equal** to networkx and to petgraph `kosaraju_scc` (36,929 components) |
| WCC | identical partition in every run | **equal** to networkx and petgraph (16,529) |
| PageRank | c1 vs c4 on the same load: **not bitwise** (max diff 1.2e-12) and **rank order differs** (near-ties reorder); shuffled order: max diff 5.4e-13, rank order differs | GDS sums to 7,770.8 (unnormalised; 20,599 dangling vertices get no redistribution). Normalised to sum 1 it matches the Rust reference to 7.2e-13 (same top 100, same top-20 order). The Rust reference matches networkx to 5.3e-10. GDS converged in 15 iterations; the reference took 22 with residual 4.2e-11. |
| Leiden (`randomSeed: 42`, gamma 1.0) | c1 repeats stable; **c4 gave 3 distinct partitions in 3 repeats**. Shuffled order changes membership even at c1 (ARI 0.816 / NMI 0.957 vs base). Community count 16,556–16,561. | Against leiden-rs RBER seed 0: **ARI 0.440, NMI 0.920**. leiden-rs across its own seeds: ARI 0.955–0.968. GDS reports modularity 0.574 against 0.452 for leiden-rs seed 0 under leiden-rs's modularity: different objectives, as expected. |
| Louvain | c1 repeats stable, c4 3 of 3 distinct; shuffled order changes membership (ARI 0.79–0.83) | GDS Leiden vs GDS Louvain: ARI 0.696 |

Semantic consequence on real data:
- GDS SCC and WCC deliver the outcome exactly.
- GDS PageRank delivers the same ranking meaning only after normalisation. Its sub-1e-12
  differences between concurrency levels reorder near-ties, so a published rank order would
  need canonical tie-breaking: by semantic id, after rounding to a declared precision.
- GDS Leiden and Louvain do not give a stable published membership unless concurrency is 1
  **and** load order is canonical. Even then they optimise a different objective from our
  RBER partition. The ARI of 0.44 is far below leiden-rs's own seed-to-seed ARI of about
  0.96, so these are different heuristics, not two implementations of one.
- The Rust reference (petgraph + leiden-rs) was byte-identical on rerun.

### SurrealDB on the real graph (`raw/surreal_real.json`, 6 GiB container)

The import loaded 36,954 `fn` records and 118,595 `calls` edges, matching the export. The
checks used 66 starts: 60 seeded random callers plus the anchor node of each non-trivial SCC.

| Check | Result against networkx |
|---|---|
| One hop with multiplicity | 66/66 equal (parallel arcs preserved) |
| Unbounded `+collect` (reachability) | 66/66 equal under the start-node rule (the start is included when it lies on a cycle) |
| Bounded `{1..d+collect}`, d = 1..6 | 66/66 equal at every depth under the same rule |
| Silent truncation | The bound omitted reachable nodes for 11/7/4/3/3/2 starts at d = 1..6, with no flag in the result |
| `+shortest` to a mid-sample reachable target | 66/66 correct length and valid path |
| Bounded `+path` from 20 starts (d = 2/3/4) | Counts exceed networkx simple paths whenever cycles or parallel arcs are reachable. Examples at d=4: 1,007 vs 443, 4,305 vs 380, 961 vs 9. Tree-like starts are equal (2 vs 2). |

Bounded `+path` from the anchor of the largest SCC (15 nodes):
- with `TIMEOUT 60s`, the counts were 1,007 / 3,962 / 17,831 / 135,820 / 1,822,620 at
  d = 4/6/8/10/12, the same with and without TIMEOUT where both ran;
- at **d = 16 the container was OOM-killed at 6 GiB despite `TIMEOUT 60s`**, and all data was
  lost;
- the deliberate unbounded probes `{..+path}` **with `TIMEOUT 20s`** and without TIMEOUT were
  **both OOM-killed** (exit 137);
- an earlier uncapped-script run, before the guards existed, also OOM-killed the 6 GiB container
  on a bounded ≤16 `+path` from the same node.

Semantic consequence on real data:
- Reachability, bounded reachability and shortest path are correct, given the start-node rule
  and that truncation is silent and must be detected by the caller.
- `+path` enumerates walks. On a real 15-node SCC it grows by roughly 10x per two hops, and at
  depth 16 it exhausts 6 GiB. Query `TIMEOUT` does not prevent this; only a small explicit
  bound does. The memory engine loses the whole database on that crash.
- `+path` is therefore unsuitable for enumerating delegation witnesses on real call graphs
  unless a wrapper caps depth to a small value and still bounds the result count.

### Questions still open (blocked, not run)

1. Does the normalized projection's SQL reconstruction reproduce `projection_source_assessments`
   counts? Blocked until a generation publishes normalized relations.
2. GDS against our published Structural/Analytic outcomes (communities at resolution 1.0, the
   published ranking, published neighbours) and `gds.knn` against published neighbours. Blocked:
   no analysis or catalog relations, and no embeddings (the fake embedder never ran).
3. SurrealDB reachability against published delegation outcomes within our budgets. Blocked
   for the same reason.

## Uncertainties

- The GDS determinism findings are observations on two 300-node synthetic graphs and one real
  facts-layer graph (36,954 vertices). GDS documents determinism only for kNN.
- The real graph is a facts-layer approximation, as described above. It includes all call
  phases and uses opaque native keys. Its Leiden comparison uses our stated conventions through
  an independent re-implementation, not the product kernel.
- The real-data SurrealDB runs used the memory engine. A persistent engine (SurrealKV/RocksDB)
  was not tried; whether it survives the same OOM with data intact was not tested.
- networkx and GDS define PageRank differently; they agree only after rescaling.
- The G3 `last()` sub-queries failed (harness), so the sink-only `+path` counts are void; the
  total counts are valid.
- The negotiated neo4rs version is inferred from what the client offers (4.0–4.4); the
  server's choice was not logged.
- The uncapped OOM consumed host memory while the real compile was running; any effect on
  that compile was not measured.
- Volume cleanup: the anonymous `/data` and `/logs` volumes of the first Neo4j and SurrealDB
  containers and of the re-created SurrealDB container were identified by creation time and
  content, then removed. Other dangling volumes on the host belong to other processes and were
  left alone.

## Progress checkpoints

- 2026-10-05: folder created; harness work on synthetic graphs.
- 2026-10-05: P2/P3 synthetic GDS determinism and parity done.
- 2026-10-05: P4 SurrealDB traversal done (OOM finding; later runs memory-capped).
- 2026-10-05: P5 neo4rs smoke passed.
- 2026-10-05: static parity written; containers and their volumes removed. Real-data parity
  waits for the generation id.
- 2026-10-05: real data: the compile stopped after facts. Facts-layer reconstruction, Rust
  reference, `gds_real.py` and `surreal_real.py` done. The first `surreal_real.py` run lost its
  output to an OOM on bounded `+path`; the rerun with guards completed (`TIMEOUT` did not prevent
  the OOM). Containers and their volumes removed.
