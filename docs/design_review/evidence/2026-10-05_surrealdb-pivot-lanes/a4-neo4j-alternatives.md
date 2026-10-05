# A4 — Neo4j (Community 2026.09 + GDS/APOC) as an analytical layer, and alternatives

Lane A4, Phase A evidence for the SurrealDB/graph-database design review. Date: 2026-10-05.
Role: library-research (capabilities and contracts; no adoption decision).

## Baseline examined

- Repository (read-only): `docs/design/sections/analytics.md` §9, §9.4, §9.5, §9.7, §9.8;
  `docs/design/DESIGN.md` §B10; `docs/design_review/reviews/design_review_library-leverage_2026-10-04.md`
  line 505 (neo4j skill rejected with cognee/jena etc. under "§B10/§B11"); `Cargo.lock`
  (petgraph 0.8.3 [+0.6.5 transitively], leiden-rs 0.8.1, fixedbitset 0.5.7; no neo4rs,
  surrealdb, rustworkx-core, graphops or graphina locked).
- Skills: `.claude/skills/neo4j-surrealdb/` (NEO4J.md, `content/capabilities/gds-community.md`,
  `content/index/procedures.tsv`, GDS manual corpus at 2026.09.0, `compare/content/matrix.md`,
  SurrealDB behaviours SB041–SB043); `.claude/skills/rust-graphs/` (SKILL.md,
  `content/catalogs/coverage.md`, `content/seams/determinism.md`, `content/index/algorithms.tsv`).
- External: Context7 `/neo4j/docs-operations`, `/neo4j-labs/neo4rs`; neo4j.com operations manual
  (incremental import page, 2026.09); GitHub releases of apache/age, timescale/pgvectorscale,
  paradedb/paradedb, neo4j-labs/neo4rs, pgRouting/pgrouting.
- Local check (read-only, 2026-10-05): `psql -h 127.0.0.1 -U lctx_superuser -d postgres -Atc
  "select version(); select name, default_version from pg_available_extensions where name in
  ('vector','vectorscale','age','pg_search','pgrouting','pg_trgm')"` -> `PostgreSQL 18.6`,
  `pg_trgm 1.6`, `vector 0.8.6`; **age, pg_search, vectorscale, pgrouting not available** on the
  operator cluster. (`docker ps` also shows a `neo4j:2026.09.0-community-trixie` container
  belonging to another project; not touched.)

Evidence labels: **[doc]** upstream documentation at the stated version; **[src]** source/catalog
inspected; **[probe]** skill probe-verified (ID@date); **[issue]** issue/release-note report;
**[vendor]** vendor claim; **[interp]** my interpretation.

---

## 1. Neo4j GDS 2026.09.0 on Community 2026.09.0

### 1.1 Edition limits (what Community actually gets)

| Limit | Fact | Label |
|---|---|---|
| Algorithms | "Community Edition: Includes all algorithms." | [doc] GDS 2026.09 `partials/introduction/enterprise-features.adoc` |
| Concurrency | Max 4; `concurrency: 5` refused with 52N37 "unlicensed GDS and cannot exceed concurrency=4". Default concurrency for most operations is 4. | [probe] GD01@2026-09-27 (driver 5.28.6, same image); [doc] System-requirements.adoc l.93 |
| Model catalog | Capacity 3 models; no model persistence to disk, no publishing. | [doc] enterprise-features.adoc |
| Graph catalog | No graph backup/restore; projections are in-heap only and lost on restart. Projecting an existing name fails until `gds.graph.drop`. | [doc]; [probe] GD02@2026-09-27 |
| Arrow Flight | **Enterprise-only**: Arrow projection (`graph-project-apache-arrow.adoc` is tagged `[.enterprise-edition]`) and Arrow catalog export. | [doc] |
| Defaults/limits config, bit-id-map optimised graph, monitoring | Enterprise-only. | [doc] |
| Databases | Community has exactly one standard database (`neo4j`) plus `system`; no `CREATE DATABASE`. | [doc] Ops manual 2026.09 (Context7 `/neo4j/docs-operations`, database-administration/index.adoc) |
| Import | `neo4j-admin database import full` (CSV or **Parquet**, `--input-type=parquet`) available; requires stopped DB + `--overwrite-destination` to replace. **Incremental import is Enterprise-only.** | [doc] Ops manual 2026.09 |
| Schema | No node-key / existence / property-type constraints (50N11); composite `IS UNIQUE` ignores nodes missing a part. | [probe] SC01/SC02/SC03@2026-09-27 |
| Storage of vectors | Community cannot store VECTOR (or UUID) properties; store embeddings as `LIST<FLOAT>`. | [doc] Cypher manual; skill NEO4J.md (VE01, UU01) |
| Licence | Neo4j Community and GDS: GPL-3.0 (no rejection reason under operator rules). | [doc] skill NOTICE |

Only one GDS behaviour beyond concurrency/catalog naming has been executed at these pins
(PageRank on a 4-node graph). Everything below about algorithm outputs is [doc], not [probe]
("Algorithm results beyond PageRank on a 4-node graph were not probed" — `gds-community.md`).

### 1.2 Algorithm coverage against our analytics

Procedure families present at GDS 2026.09.0 (from `content/index/procedures.tsv`, [src]):
`scc, wcc, leiden, louvain, labelPropagation, sllpa, modularity, modularityOptimization,
conductance, kcore, triangleCount, localClusteringCoefficient, kmeans, hdbscan, k1coloring,
pageRank, articleRank, eigenvector, betweenness, closeness(.harmonic), degree, hits, celf,
nodeSimilarity(.filtered), knn(.filtered), fastRP, node2vec, hashgnn, beta.graphSage,
beta/alpha.pipeline.{linkPrediction,nodeClassification,nodeRegression}, shortestPath.{dijkstra,
astar,yens}, allShortestPaths.{dijkstra,delta}, bellmanFord, bfs, dfs, randomWalk,
dag.topologicalSort, dag.longestPath, spanningTree, kSpanningTree, steinerTree,
prizeSteinerTree, maxFlow, maxFlow.minCost, maxkcut, bridges, articulationPoints,
cliqueCounting, collapsePath, graph.sample.{rwr,cnarw}, splitRelationships`.
Topological link-prediction functions (Adamic-Adar etc.) are documented under
`algorithms/topological-link-prediction` [doc].

Per-analytic contract notes ([doc] unless marked):

- **SCC schedule (callee-first condensation order + SCC-local worklist).** `gds.scc` returns a
  `componentId` per node only. `gds.dag.topologicalSort` is defined for DAGs and **omits nodes in
  cycles and every node reachable from a cycle** (topological-sort.adoc l.28–36). There is no
  condensation/quotient-graph procedure in the family list (`gds.collapsePath` collapses paths
  of given types, not SCCs). A callee-first schedule therefore needs: scc -> build condensation
  (Cypher or client) -> project again -> topologicalSort. The SCC-local summary worklist
  (condition-carrying summaries) has **no GDS counterpart**; it would stay in Rust
  (`gds.pregel` exists only as a Java API extension point, [doc] `pregel-api.adoc`, and requires
  compiling a Java plugin). [interp] GDS covers the decomposition, not the schedule or worklist.
- **Bounded delegation traversal.** `gds.bfs/dfs` support `maxDepth` and target nodes; Cypher 25
  quantified path patterns / `SHORTEST` also available (matrix "Traversal language"). Explicit
  work/witness budgets with named stop inventories are not a GDS concept [interp].
- **WCC.** `gds.wcc` directed/undirected/weighted. **"The actual component ids may differ because
  the order of nodes projected in the in-memory graph is not guaranteed"** (wcc.adoc l.319, 438).
  `consecutiveIds` available (not with seeding).
- **Leiden.** `gds.leiden` traits: `:no-directed: :undirected: :weighted:` — **undirected
  projections only**; docs project `orientation: UNDIRECTED` / `undirectedRelationshipTypes`.
  Config: `randomSeed`, `gamma` (resolution), `theta`, `maxLevels`, `tolerance`,
  `includeIntermediateCommunities`, `seedProperty`, `consecutiveIds`; examples pair
  `randomSeed` with `concurrency: 1`. Explicit direction conversion therefore happens at
  projection time (UNDIRECTED orientation, parallel aggregation), as ours does in the adapter.
  Quality/modularity per level is returned in stats/mutate rows. CPM quality function: not
  found in the Leiden page in this corpus (search scope: `algorithms/leiden.adoc`) — leiden-rs
  has Modularity, CPM, RB [src rust-graphs coverage].
- **Louvain.** directed/undirected/weighted; no `randomSeed` seen in the page (determinism
  undocumented) — unresolved.
- **PageRank.** directed/undirected/weighted (`relationshipWeightProperty`), personalised via
  `sourceNodes` (with bias), `dampingFactor`, `maxIterations`, `tolerance`, scaler; returns
  `ranIterations`, `didConverge` [doc]. Dangling-node treatment and per-iteration residuals are
  not exposed as parameters/diagnostics in the page (search: page-rank.adoc) — our kernel
  retains both (analytics.md §9.5).
- **kNN / node similarity.** `gds.knn` is an **approximate** sampled kNN (uniform/random-walk
  sampler, `sampleRate`, `deltaThreshold`, `topK`, `similarityCutoff`); deterministic only with
  `concurrency: 1` **and** explicit `randomSeed` (knn.adoc l.424–428). Our analytic kNN is
  **exact**, threshold/top-k with semantic-ID tie-break (§9.7). `gds.nodeSimilarity` is exact
  Jaccard/overlap/cosine over neighbour sets (topology), not embedding vectors. [interp] exact
  dot-product kNN on stored vectors is not GDS's contract; it would need `sampleRate: 1.0`
  equivalence checking (unprobed) or a vector index (approximate HNSW in Neo4j).
- **Embeddings (FastRP, node2vec, GraphSAGE, HashGNN).** Present; GraphSAGE is beta and a trained
  model counts against the 3-model Community cap and is not persisted. FastRP/node2vec pages are
  not in the vendored corpus, so their determinism contract was not inspected here (unresolved).
  Note §B10 currently excludes graph embeddings.
- **Link prediction.** Topological functions (Adamic-Adar, common neighbours, preferential
  attachment, resource allocation, same community, total neighbours) and beta LP pipelines
  (model-catalog bound).
- **Path finding / topological sort.** Dijkstra, A*, Yen, delta-stepping, Bellman-Ford,
  topologicalSort, longestPath, BFS/DFS, max-flow, Steiner — broad.
- **FCA / RCA, condition-carrying summaries (BDD), Merkle/canonical conditions.** **Not
  covered** by GDS or APOC (no concept-lattice or decision-diagram procedures in the 716
  procedure / 473 function catalog; search scope: `procedures.tsv`/`functions.tsv` names for
  `concept|lattice|fca|bdd` — none) [src]. These remain Rust (`fcars`/in-house NextClosure,
  biodivine-lib-bdd per `rust-reasoning`).

### 1.3 Determinism controls (CI-09/CI-11 relevance)

- Seeds: `randomSeed` on Leiden, kNN, FastRP/node2vec/randomWalk-type algorithms [doc].
- Concurrency: documented determinism for kNN requires `concurrency: 1` [doc]; Leiden examples
  use `concurrency: 1` with seed [doc]. Floating-point summation order with concurrency >1 is
  not addressed in the pages read (unresolved).
- Node identity: GDS emits internal `nodeId`s; WCC ids depend on projection order [doc].
  Results must be re-keyed to semantic IDs (via `gds.util.asNode(nodeId).<key>` or streaming a
  node property) and canonicalised client-side — identical to what our adapter does with
  leiden-rs labels today (§9.4 "compare partitions by membership").
- GDS node/relationship properties support only `long`, `double`, `long[]`, `double[]`,
  `float[]` [doc] `graph-creation/index.adoc` l.40ff; string semantic IDs cannot be projected,
  so the semantic key lives in the Neo4j store and is joined back on stream.
- No "shuffled input gives identical output" guarantee is documented for any algorithm.
  [interp] Our CI-09 requirement would have to be met by a Rust-side canonicalisation plus a
  Phase B shuffle probe, as it is for leiden-rs today.

### 1.4 Projection lifecycle, memory, write-back

- Native projection (`gds.graph.project(name, labels, types)`) or Cypher aggregation
  (`RETURN gds.graph.project(name, s, t, dataConfig)`); legacy `gds.graph.project.cypher` is
  deprecated [src/probe-catalog]. Projections are named, outlive the query, must be dropped
  (`gds.graph.drop(name, false)` in `finally`) [probe GD02].
- **Parallel relationships are preserved by default**; `aggregation: SINGLE|COUNT|SUM|MIN|MAX`
  per relationship projection collapses them [doc] graph-project.adoc l.493ff.
  **Isolates:** native projection includes every node of the projected labels; Cypher
  aggregation projects an unconnected node when `targetNode` is null [doc]
  graph-project-cypher-projection.adoc l.57, 86. Our declared universe (isolates disclosed,
  parallel evidence retained, CI-03/CI-05) is expressible, but ArcId lineage is not carried
  into GDS (properties are numeric only) [interp].
- Memory: projections and algorithm state are on the JVM heap; recommended heap ~90% of RAM for
  purely analytical workloads; `*.estimate` procedures estimate memory [doc]
  System-requirements.adoc.
- Execution modes: `stream` (rows to client), `stats`, `mutate` (into the in-memory projection),
  `write` (back to the Neo4j store; uses transaction-state heap). For us `stream` is the only
  mode consistent with "PostgreSQL generation is the single relational store" — results come
  back to Rust and are written into PG with provenance [interp].

---

## 2. Integration path

### 2.1 Moving one immutable generation into Neo4j

| Route | Community? | Fit | Label |
|---|---|---|---|
| `neo4j-admin database import full --input-type=parquet` (stop DB, `--overwrite-destination`) | yes | Best bulk route: DataFusion (already in the workspace) can write per-generation node/edge Parquet; whole DB replaced per generation. Offline: the server is down during import. | [doc] |
| `neo4j-admin database import incremental` | **no (Enterprise)** | — | [doc] |
| `LOAD CSV` (server-side file URLs) | yes | Online; slower; files must be readable by the server. | [doc] |
| `UNWIND $rows ... MERGE/CREATE` batches over Bolt | yes | Online; one bad row fails the whole statement (22G03); `SET n = $props` wipes properties (use `+=`); `CALL {…} IN TRANSACTIONS` only via `session.run`, not managed tx. | [probe] RQ02/RQ03/TX02@2026-09-27 |
| Arrow Flight projection directly into GDS (skip the store) | **no (Enterprise)** | Would have been the cleanest "compute-only" path. | [doc] |
| `gds.graph.project` via Cypher aggregation from rows passed as parameters (`UNWIND $edges`) | yes | Projection without persisting a store graph is possible in principle (nodes still must exist? — Cypher aggregation takes node values; whether virtual nodes are accepted was not checked) | unresolved |

Generation identity: Community's single database means one generation per server at a time
(replace on rebuild) or a `generation` property/label partition inside one DB. The DB itself is
a **derived, disposable projection**; the authoritative generation id and digests stay in PG and
are stamped onto every streamed result row on the Rust side [interp]. Rebuild cost = export
Parquet + offline import + server start + projection + algorithms; nothing incremental on
Community [doc + interp]. No performance figures were measured (skill disclaims performance).

### 2.2 Client access

- **Python `neo4j` 6.3.1** (skill-pinned, 21 offline probes; server round trips at 6.3.1 queued,
  not evidence). Traps: zoneinfo datetimes break the encoder (TY02), numpy arrays are LIST not
  VECTOR (VE03), retries semantics (TX01/TX04). Placement: the Python adapter is the FastMCP
  serving process, not the compile pipeline [interp].
- **Rust `neo4rs` 0.9.0-rc.10** (published 2026-06-11; still a release candidate; Neo4j Labs, not
  an officially supported driver) [issue/release]. Context7 docs state support for "Neo4j 4.4
  LTS and 5.x latest"; rc.9 added Neo4j 2025.x to integration tests [doc/release notes]. No
  statement of Bolt 6 / 2026.x support found (unresolved; Bolt negotiation probably still works
  at 5.x protocol versions [interp]). Tokio async, pools, transactions, `BoltMap`/`BoltList`
  params for UNWIND batches [doc]. Outside every skill; no probes.
- Operational burden: JVM server (Java 21+ image), a second container beside PG18, heap sizing
  (projections on heap), page cache, a plugin directory for GDS/APOC (image `NEO4J_PLUGINS`),
  credentials, readiness. The skill's image is pinned by digest (`neo4j:2026.09.0-community-trixie`).

### 2.3 Does SurrealDB make Neo4j integration easier than PostgreSQL?

No material difference found; it is export/import either way:

- SurrealDB 3.3 exposes `export()` (SurrealQL dump) and changefeeds (`DEFINE TABLE … CHANGEFEED`,
  `SHOW CHANGES`, live queries) [src SB041–SB043, methods.tsv]; there is **no Neo4j connector,
  no Parquet/CSV/Arrow export** in the indexed SDK [src] (search scope: skill SDK methods/aliases
  for export/backup; no CSV/Parquet). Neo4j has no SurrealDB importer [interp from ops manual
  import formats: CSV/Parquet only].
- Data model: both are property graphs, so a SurrealDB record-edge (`RELATE in/out`) maps 1:1 to
  a Neo4j relationship, whereas from PG the relational rows must be shaped into node/edge files.
  [interp] That shaping is a DataFusion query either way (our relations already own endpoints
  and arcs, §9 "Normalized relations own semantic graph endpoints and arcs"), so the saving is
  small.
- Changefeeds would only matter for incremental sync, which Community import cannot use and
  which our immutable-generation model does not need [interp].
- PostgreSQL side: COPY/DataFusion -> Parquet is already within the stack (sqlx-postgres,
  datafusion skills); SurrealDB side would need a Rust exporter over SDK queries [interp].

---

## 3. Same analytics without a second database (rust-graphs skill pins: petgraph 0.8.3,
rustworkx-core 0.18.1, leiden-rs 0.8.1, graphops 0.5.1, graphina 0.4.0-alpha.6, rust-igraph 0.7.0,
raphtory 0.17.0)

- **SCC + schedule:** petgraph `kosaraju_scc`/`tarjan_scc` return SCCs in reverse topological
  order (probe B002) — i.e. callee-first for call graphs directly; `petgraph::algo::condensation`
  gives the quotient DAG [src algorithms.tsv]. rustworkx-core adds
  `lexicographical_topological_sort` (deterministic tie order) [src coverage]. This is strictly
  richer than GDS for our schedule. Already in use.
- **Leiden:** leiden-rs `Leiden::run` (Modularity, CPM, RB; resolution scans; partition metrics);
  `seed: Option<u64>`: same seed, same partition (probe B011); parallelises local moving above
  2,000 nodes (determinism seam: float-order differences possible) [src]. `from_petgraph` reads
  edge weights (B009). graphops' `leiden` is actually Louvain (B010) — avoid. rust-igraph has
  leiden/louvain/infomap/walktrap (GPL-2.0+, copy).
- **PageRank:** petgraph `page_rank` (unweighted f-type), graphops/graphina/rust-igraph
  pagerank + personalized; our model-owned weighted kernel with dangling/residual diagnostics is
  more explicit than any of these or GDS (§9.5).
- **Similarity / link prediction:** graphops jaccard/cosine; graphina adamic-adar, preferential
  attachment, resource allocation, common neighbours (alpha, `features=["all"]`, C002);
  rust-igraph similarity_jaccard.
- **Embeddings:** graphops node2vec-style biased walks (no trainer); raphtory `fast_rp` (seed
  honoured, B015; but raphtory pins arrow ^56/datafusion ^50 — second type universe — and does
  not compile as published, C001); spectral embeddings in graphina/rust-igraph. "No library here
  trains an embedding" (SKILL.md). GraphSAGE has no Rust equivalent in the ladder.
- **kNN:** our exact dot-product kernel is bespoke and small; no ladder library supplies exact
  vector kNN [src coverage: no `knn` row]. pgvector exact (`ORDER BY <#>` without index) is the
  in-PG alternative.
- **FCA/RCA:** not graph libraries; `rust-reasoning` covers fcars 0.2.2 as oracle.
- Integration cost: zero-copy (rustworkx-core) to copy (graphina/igraph/raphtory); in-process,
  no server, versioned by Cargo.lock, seeds recorded in Rust. Determinism: hash-ordered outputs
  (rustworkx-core hashbrown sets C005, graphina Fx maps) and rayon thread counts must be
  canonicalised — the same obligation GDS imposes, minus the internal-id re-keying.

---

## 4. PostgreSQL 18 comparators (to separate "graph DB benefit" from "unused PG capability")

| Option | PG18 status (verified 2026-10-05) | What it gives | What it does not | Label |
|---|---|---|---|---|
| Recursive CTEs (`WITH RECURSIVE … SEARCH … CYCLE …`) | core PG18 (local 18.6) | bounded traversal, reachability, cycle marking, depth limits in SQL | no SCC/community/centrality; cost grows with path enumeration | [doc] PG manual |
| Apache AGE (openCypher) | `PG18/v1.8.0-rc0` release published 2026-07-09 (marked non-prerelease; tag says rc0); 1.7.0 for PG18 on 2026-01-21. **Not installed** on the operator cluster. | Cypher MATCH/MERGE (1.8 adds `ON CREATE/ON MATCH SET`), VLE traversal, `shortest_path`/`all_shortest_paths` SRFs, `create_subgraph()`, RLS, composite vertex/edge types | **no algorithm library** (no PageRank/community/SCC); Cypher subset | [issue] GitHub release notes |
| pgRouting | 4.0.2 released 2026-09-05; PG18 build support not confirmed in this pass | Dijkstra/A*/k-shortest, `pgr_strongComponents`, `pgr_connectedComponents`, `pgr_topologicalSort` (Boost Graph) | no community detection/PageRank | [issue] release; function names from training knowledge — unverified at 4.0.2 |
| ParadeDB pg_search (BM25) | v0.26.0 published 2026-10-03; release assets include pg18 packages; upstream states PG15–18; requires pgvector since 0.25 | BM25 full-text, hybrid search inside PG | not graph | [issue]/[vendor]; AGPL-3.0 |
| pgvectorscale (DiskANN) | 0.9.1 published 2026-09-04 with pg14–pg18 amd64/arm64 assets; PG18 added in 0.9.0 | approximate ANN index over pgvector | exact kNN still pgvector sequential | [issue] release |
| pgvector | 0.8.6 available locally | exact and HNSW/IVFFlat vector search | — | local check |

[interp] The only GDS capabilities without a PG-native or in-process Rust equivalent are the
trained embeddings (GraphSAGE/node2vec training, FastRP outside raphtory) and ML pipelines —
both currently excluded by §B10. Everything in the current analytics set (SCC schedule, Leiden,
PageRank, exact kNN, FCA/RCA) is already covered in-process with stronger contracts; traversal
and BM25 gaps are closable inside PG18.

---

## 5. Capability coverage table

| Our analytic | Neo4j GDS 2026.09 (Community) | Rust libs (pinned skill) | SurrealDB 3.3 | PostgreSQL 18 option |
|---|---|---|---|---|
| SCC decomposition | `gds.scc` (ids only) [doc] | petgraph kosaraju/tarjan (in use) [probe B002] | none (app code) [src matrix] | pgRouting strongComponents (unverified PG18) |
| Callee-first SCC schedule / condensation | not provided; topoSort drops cyclic + downstream nodes [doc] | petgraph `condensation` + rev-topo SCC order; rustworkx lexicographic toposort [src] | none | none native |
| SCC-local summary worklist with conditions | no (Pregel = Java plugin) | in-house + biodivine-lib-bdd | no | no |
| Bounded delegation traversal | bfs/dfs `maxDepth`, Cypher QPP/SHORTEST [doc] | petgraph visitors with own budgets | recursive idioms `.{1..n}`, `+shortest` [probe SB025] | recursive CTE SEARCH/CYCLE; AGE VLE |
| WCC | `gds.wcc` (ids order-dependent) [doc] | petgraph `connected_components` (weak on directed, B005) | none | pgRouting connectedComponents (unverified) |
| Leiden communities | `gds.leiden` undirected only, randomSeed+concurrency 1 [doc] | leiden-rs 0.8.1 seeded (B011), Modularity/CPM/RB | none | none |
| Louvain / LPA | yes (seed control undocumented for Louvain) | graphops/graphina/igraph | none | none |
| Weighted PageRank + diagnostics | yes, `didConverge`/`ranIterations`; no dangling/residual knobs seen [doc] | model-owned kernel (in use); petgraph/graphops/igraph | none | none |
| Exact kNN over embeddings | no (gds.knn approximate unless proven; vector index approximate) [doc] | in-house exact kernel (in use) | HNSW `<|k,ef|>` approximate (SB038) | pgvector exact scan / HNSW; vectorscale DiskANN |
| Node similarity (topology) | `gds.nodeSimilarity` exact Jaccard/overlap/cosine | graphops jaccard/cosine; igraph | none | SQL joins |
| Graph embeddings (FastRP/node2vec/GraphSAGE) | yes; GraphSAGE beta, 3-model cap, no persistence | raphtory fast_rp (arrow 56 seam), graphops walks only | none | none |
| Link prediction | topological functions + beta pipelines | graphina (alpha) | none | SQL |
| Path finding | Dijkstra/A*/Yen/delta/Bellman-Ford | petgraph/rustworkx/igraph | `+shortest`, GQL ANY SHORTEST | pgRouting; AGE shortest_path SRF |
| Topological sort | DAG only, drops cyclic region | petgraph toposort; rustworkx lexicographic | none | pgRouting topologicalSort (unverified) |
| FCA / RCA | no | fcars (oracle) / in-house | no | no |
| BM25 retrieval (adjacent) | full-text index (Lucene; no stemming default, FT01) | — | full-text search (not covered here) | ParadeDB pg_search 0.26.0 |

---

## 6. Unresolved items

1. GDS determinism under shuffled input order (CI-09) for Leiden/WCC/PageRank at concurrency 1
   and 4: undocumented beyond kNN; Louvain seed control absent from the page.
2. Whether `gds.knn` with `sampleRate: 1.0, concurrency: 1` is exhaustive (exact) — not stated.
3. FastRP/node2vec/GraphSAGE determinism pages not in the vendored corpus.
4. neo4rs 0.9.0-rc.10 against Neo4j 2026.09 (Bolt 6.x) — no compatibility statement.
5. Projection from parameters only (no store write) via Cypher aggregation — feasibility not
   checked.
6. pgRouting 4.0.2 PG18 packaging and exact function set — not verified.
7. AGE PG18 1.8.0 tag is named `rc0` but published as a release; ASF vote status unclear.
8. All 34 Neo4j server probes for driver 6.3.1 remain queued (skill); server facts rest on the
   2026-09-27 run through driver 5.28.6.

## 7. Candidate Phase B probes (all scratch or a dated evidence folder, if authorised)

- **P-N1 GDS shuffle determinism:** same graph loaded in two node/edge orders; run leiden
  (seed, concurrency 1 and 4), wcc, pageRank, scc; compare canonicalised memberships/scores
  byte-for-byte; control: unseeded leiden differs.
- **P-N2 GDS vs in-process parity:** on a real generation's call graph, compare GDS
  leiden/pageRank/scc to leiden-rs / model-owned PageRank / kosaraju after semantic-ID
  re-keying (agreement tolerance declared beforehand).
- **P-N3 Generation round trip:** DataFusion -> Parquet -> `neo4j-admin database import full
  --input-type=parquet --overwrite-destination` -> project -> stream -> PG with generation stamp;
  record wall time and heap (measurement, not a gate).
- **P-N4 Exact kNN in GDS:** `gds.knn` with `sampleRate 1.0, concurrency 1, randomSeed` vs our
  exact kernel on the same vectors (control: default sampleRate differs).
- **P-N5 neo4rs smoke:** connect neo4rs 0.9.0-rc.10 to the pinned 2026.09 image; UNWIND batch,
  `gds.pageRank.stream` call, type round trip.
- **P-PG1 PG18 extension availability:** install AGE 1.8.0 (PG18) and pgRouting 4.0.2 into a
  disposable PG18 container; run strongComponents/topologicalSort and an AGE VLE query against
  the same edge table; confirms the PG-native comparator.
- **P-PG2 pg_search BM25 on PG18** in a disposable container (only if retrieval is in scope).
