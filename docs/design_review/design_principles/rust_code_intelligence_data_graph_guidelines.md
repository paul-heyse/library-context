# Rust Code Intelligence: Columnar, Relational, and Graph Execution Guidelines

## Scope and interpretation

Agent handoff for a Rust code-intelligence / code-property-graph (CPG) system. Preserve typed, provenance-backed relational facts; use graph representations only where the computation benefits from topology-aware algorithms. Minimize bespoke code without forcing one physical representation or execution strategy onto every operation.

**MUST** is a correctness requirement. **DEFAULT** is a preferred choice, overridable for a documented semantic or workload reason. **MAY** is an optional technique, not implementation scope. The library catalog is an available toolbox, not an instruction to adopt every dependency or expose every algorithm to agents.

Respect the repository's locked Arrow/DataFusion versions and existing storage decisions. Resolve compatible crate versions/features together; do not independently upgrade each library to its latest version. The graph releases examined below fit the requested version families. Capabilities were source/API reviewed on **2026-09-22**; this document does not claim compilation, integration tests, or performance benchmarks.

## 1. Governing division of responsibility

> **Relational facts define the analysis. Columnar/relational execution prepares and combines data. Graph kernels compute topology-dependent results. Results return as typed, provenance-backed relations.**

| Mechanism | DEFAULT responsibility | Prefer it when |
|---|---|---|
| **Arrow arrays / RecordBatch / kernels** | Typed interchange and local, physically specified column operations | Inputs and operations are already known: gather, mask, cast, construct or concatenate batches. |
| **DataFusion** | Set-oriented transformation and relational planning/execution | Scans, joins, semi/anti joins, aggregation, sorting, window expressions, or composable predicates express the problem. |
| **Petgraph and specialist graph crates** | Explicit topology and graph/numerical kernels | Reachability, cycles, dominance, centrality, community structure, graph similarity, or temporal connectivity is the object of computation. |
| **Delta Lake / delta-rs, when warranted** | Durable versioned tables and selected artifacts | Transactional table updates, reproducible snapshots, retention, or supported change feeds justify the storage layer. |
| **Dedicated static-analysis kernel** | Language-specific recursive reasoning | Transfer functions, abstract domains, call/return context, aliasing, or interprocedural summaries are required. A graph library alone is insufficient. |

Arrow exposes array kernels; DataFusion plans and executes Arrow-based computations; graph crates supply algorithm-specific implementations. These are complementary roles.[^arrow][^df-provider][^petgraph]

**DEFAULT:** Choose an algorithm before choosing a crate. Use existing relational/graph functionality before writing an algorithm. A handwritten adapter, frontier worklist, or semantic transfer function is acceptable; a second general query engine or universal graph framework is not a prerequisite.

**Conditionality:** A bounded relationship pattern can remain a join. A large relational job belongs in DataFusion. A small topology problem may warrant a graph. For a one-off selective query, batched frontier lookups over indexed/partition-pruned relations can beat building a whole graph. Conversely, repeated traversal may amortize one adjacency projection. There is no universal row-count or hop-count crossover.

## 2. Keep fact construction and ordinary relationships relational

**DEFAULT:** DataFusion owns fact normalization, schema-compatible unions, identifier resolution, evidence joins, duplicate-policy enforcement, endpoint validation, scope selection, direct-degree aggregation, revision diffs, and result enrichment/ranking. Preserve useful pipelines as composable subplans rather than issuing a query per symbol, edge, or traversal step.

Use direct Arrow kernels for local operations at established boundaries. Do not rebuild joins/grouping in Arrow loops; do not break a useful DataFusion plan merely because an equivalent array kernel exists. Do not collect complete datasets when a streaming scan or narrower projection suffices.[^arrow][^df-dataframe]

**MUST:** Canonical relationships are first-class facts with persistent `edge_id`, typed endpoints, relation kind, evidence, and configuration/snapshot scope. `(src_id, dst_id)` is not generally an edge identity: separate call sites, arguments, conditions, or extraction evidence can share endpoints. Preserve isolates through an explicit node relation, not only an edge-derived vertex set.

**DEFAULT:** Store flat node/edge relations plus typed extension/evidence tables. Small structs/lists are useful for bounded metadata, but recursively nested objects must not replace reference-based composition. Algorithms may use ordinary Rust vectors, maps, queues, or bitsets internally. Arrow schemas are interchange contracts, not a mandate for columnar point lookups.

Keep extracted, compiler-resolved, analysis-derived, observed, and heuristic/inferred information distinguishable. Preserve unknown/unresolved targets explicitly or record their omission and its effect on completeness.

## 3. Make every graph an explicit, versioned projection

**MUST:** Define a `GraphProjectionSpec` before executing an analysis. This is a semantic contract, not necessarily a new framework or a literal schema to implement wholesale.

```text
GraphProjectionSpec:
  source_snapshot: repository/package revision + exact fact-table/fragment versions
  extraction_context: language/toolchain, target, features/cfg, analysis configuration
  visibility_scope: repository/package scope and any authorization restrictions
  node_selection: node kinds, granularity, isolates, unknown-node policy
  edge_selection: relation kinds, directness, evidence/certainty requirements
  direction: source relation semantics and explicit traversal orientation
  context: call-site, receiver/type, or other context sensitivity when applicable
  transformation: aggregation, symmetrization, parallel-edge, self-loop policies
  weights: meaning, units, domain, normalization, missing-value policy
  time_semantics: snapshot, events, validity intervals, or explicit window
  analysis_scope: full graph, proven independent region, or stated induced subgraph
  source_completeness: known coverage, exclusions, unresolved facts
  projection_schema_version
```

Keep invocation parameters separately identifiable: algorithm/version, roots, objective, damping/resolution, seeds, tolerances, requested depth, and operational budgets. All result-affecting projection and invocation settings participate in artifact validity.

**MUST:** Separate an **output selector** from the **graph universe needed to compute it**. Filtering requested symbols must not remove intermediate dependency nodes. Computing a symbol's global PageRank is not equivalent to PageRank on its selected ego graph. An SCC computed independently within files can miss cross-file cycles.

Choose separate projections for AST containment, control flow, calls, data flow, imports, type relations, and semantic similarity. Do not traverse their untyped union. For a call edge `caller → callee`, downstream dependencies use outgoing traversal; potential callers affected by a callee change use incoming traversal. Other relations need their own declared orientation.

**MUST:** Verify weight semantics against the algorithm. Affinity, frequency, probability, and distance are different quantities. High affinity must not accidentally become high shortest-path cost. Reject or explicitly handle NaN, infinite, negative, zero, missing, or duplicate weights as required by the chosen API.

## 4. Choose graph representations without losing semantics

**DEFAULT:** Use a lightweight Petgraph `Graph`/`DiGraph` for immutable structural projections. Use `StableGraph` when in-place deletion with index stability is useful. Preserve persistent domain IDs separately from graph-local indices; stable indices are not persistent cross-build identities.[^petgraph]

For dense-array kernels, explicitly maintain:

```text
DomainNodeId <-> DenseNodeId in 0..n
ProjectedEdgeId -> contributing canonical EdgeIds / evidence
```

Do not assume `StableGraph` indices are dense after deletions. Validate index bounds and integer-width limits. Do not join outputs to nodes by incidental iteration order.

**Conditionality:** Use a CSR/CSC or borrowed-neighbor representation when repeated scans, memory locality, or a backend contract justify it. Reuse an existing backend representation where possible. Prefer direct construction from prepared edge batches over a chain of full graph copies. Building/remapping adjacency usually allocates; sharing Arrow buffers does not make graph construction zero-copy.

Petgraph `GraphMap` and `Csr` do not preserve parallel edges as a multigraph. Use them only for projections whose explicit simplification policy permits that loss. CSR as a general layout can represent repeated neighbors; this restriction concerns those concrete Petgraph types.[^graphmap][^csr]

**MUST:** Validate any simplification against the question. For reachability, collapsing duplicate edges may preserve the answer; for call-site provenance or multiplicity-weighted metrics it may not. Symmetrization and aggregation require named policies, such as sum/max/binary presence, and lineage back to source facts. Do not double-count an undirected edge merely because a backend stores both orientations.

## 5. Select a small algorithm toolbox, not competing graph authorities

Prefer Petgraph plus selected `rustworkx-core` algorithms as the general structural baseline. Add a specialist only for required functionality or a justified representation/performance advantage. Check exact per-function graph traits, directedness, weight support, normalization, output indexing, and complexity; a crate's headline capability is not a uniform contract for all its functions.

| Library / examined release | Appropriate role | Adoption and interoperability guidance |
|---|---|---|
| **petgraph 0.8.3** | Graph representations, traversal, SCC/condensation, topological and dominance/path analyses | Baseline topology kernel. Pick the concrete graph type for required traits and edge semantics.[^petgraph] |
| **rustworkx-core 0.18.1** | Broader Rust graph analytics: centrality, connectivity/cuts, paths, matching, DAG algorithms | Reuses Petgraph traits/types; import the Rust core, not Python bindings. The tagged workspace uses Petgraph `0.8`. Check each algorithm's restrictions and output-index conventions.[^rustworkx][^rustworkx-manifest] |
| **leiden-rs 0.8.1** | Dedicated community analysis: Leiden objectives, resolution exploration, partition metrics; additional community methods where required | Prefer for a specifically Leiden-based analysis or richer objective controls. Own CSR-backed `GraphData`; conversions are materialization boundaries. Advertises optional Petgraph support with a `^0.8` dependency; verify feature-enabled APIs against the lockfile.[^leiden][^leiden-data] |
| **graphops 0.5.1** | Louvain/connected-Louvain, PageRank/PPR, graph similarity/kernels, walk generation, selected structural embeddings | Useful when several operator families share one projection. Borrowed `GraphRef`/`WeightedGraphRef` interfaces can suit adjacency adapters. Observe the algorithm-name and Petgraph-version cautions below.[^graphops][^graphops-ref] |
| **graphina 0.4.0-alpha.6** | Optional gap-filling: link prediction, approximations, metrics, communities, other exposed modules | Pin the full prerelease and only needed features. Require focused correctness tests and a demonstrated missing capability before adopting its API/representation. Do not infer maturity from catalog breadth.[^graphina] |
| **rust-igraph 0.7.0** | Broad alternative/reference toolbox, including community methods, flow, isomorphism, and structural analysis | This package describes a pure-Rust port, not C-igraph FFI. Its declared license is `GPL-2.0-or-later`; require project license approval before incorporating it. Validate the selected implementation, not just equivalence of API names.[^rust-igraph] |
| **raphtory 0.17.0** | Repeated temporal graph analyses where event ordering, edge validity, or windows are part of the question | A distinct temporal graph model, not a drop-in Petgraph algorithm collection. Documentation declares GPL-3.0; check the exact release/license and required graph-view semantics before adoption. Historical snapshots alone do not justify it.[^raphtory][^raphtory-persistent] |

### Non-obvious compatibility and capability constraints

**graphops algorithm naming:** In 0.5.1, `leiden*` entry points are compatibility names for connectivity-refined Louvain. The source explicitly excludes Leiden's constrained-merge refinement. Record this as `connected_louvain`, not full Leiden; use a dedicated verified Leiden implementation when that algorithm is required. The connected-Louvain contract expects symmetric adjacency for an undirected graph.[^graphops-community]

**graphops dependency boundary:** Its optional Petgraph feature depends on `0.6`, unlike the `0.8` baseline above. Its `PetgraphRef` precomputes neighbor lists rather than borrowing a Petgraph adjacency store without conversion. Prefer an application-owned dense adjacency view implementing the needed graphops trait, or an explicit conversion. Do not cast between incompatible crate types or create a redundant Petgraph 0.6 copy by accident.[^graphops-cargo][^graphops-adapter]

**Embedding boundary:** graphops' `node2vec` module supplies biased-walk generation; this is not by itself a complete learned-vector training pipeline. Select an actual embedding implementation/trainer separately when needed. Do not confuse graph-structural vectors with code/text embedding vectors or assume their coordinates are comparable.[^graphops-walks]

**Optional static-analysis support:** `datafrog` is relevant when recursive relational inference naturally fits static relations plus monotonically growing tuple sets. Consider it for that semantic subsystem, not ordinary traversal, and do not assume it supplies deletion-aware maintenance or complete language semantics.[^datafrog] Bitsets such as `fixedbitset` or `roaring` MAY serve visited/fact sets after checking their exact contracts; they are implementation aids, not additional graph authorities.

## 6. Route code-intelligence tasks by their actual semantics

| Task | DEFAULT route | Correctness boundary |
|---|---|---|
| Resolve symbols, signatures, docs, visibility, direct calls; aggregate fan-in/out | DataFusion; Arrow for local output assembly | Define edge multiplicity and distinguish distinct caller counts from call-site counts. |
| Inspect a fixed relationship pattern | DataFusion joins/semi-joins | Use graph execution only if it materially improves the required topology operation. |
| Find delegation chains, reverse callers, or dependency neighborhoods | Filter/prepare facts → directed graph traversal → evidence joins | Explicit edge kinds, contexts, direction, roots, unknowns, and completeness. |
| Find mutually recursive functions / dependency cycles | SCC then condensation on the relevant directed projection | Preserve membership and condensation-edge evidence; do not equate mutual reachability with semantic interchangeability. |
| Analyze dominators / control-dependence prerequisites | Function-scoped CFG → suitable graph algorithm | Define entry/exit treatment, reachable blocks, exceptional edges, and any transformation used for post-dominance. |
| Compute reaching definitions, liveness, taint, points-to, interprocedural summaries | Dedicated worklist/fixed-point analysis over typed CFG/data-flow facts | Requires transfer/join semantics, alias/context handling, and termination. Bare reachability is not automatically sound static analysis. |
| Rank likely relevant symbols | Relational candidate selection plus scoped centrality/PPR or similarity as justified | Candidate pruning changes graph scores unless semantics prove otherwise; scores rank evidence, not prove functionality. |
| Discover likely subsystem groupings | Explicit affinity projection → dedicated community algorithm → relational interpretation | Communities are heuristic analytical outputs, not authoritative module/ownership boundaries. |
| Retrieve structurally similar code | Bounded candidate generation → structural features/kernels/graph comparison | State whether similarity is approximate, structural, semantic, or embedding-based. No automatic behavioral-equivalence claim. |
| Compare revisions or summarize churn | Relational diffs/aggregates; selected per-snapshot graph analysis | Use Raphtory only when the temporal graph semantics, rather than mere historical storage, are required. |

A graph-derived handoff candidate between APIs must still satisfy type/trait, generic, ownership/lifetime, feature/configuration, and documented precondition constraints. Graph proximity, matching type names, or common community membership cannot establish API interoperability.

## 7. Control recursion, cardinality, and execution scope

**DEFAULT:** Store direct facts, compute only requested closures, retain compact witnesses rather than every path, and reuse a prepared projection across compatible analyses. For finite reachability, use visited-node traversal; for changing analysis facts, enqueue newly changed facts until the declared fixed point.

A finite direct graph can contain exponentially many paths. Even ancestor sets for every node of a chain total quadratic size. Therefore, changing a recursive join into graph path enumeration does not solve combinatorial expansion. Do not eagerly materialize all paths, all-pairs distances, dense similarity matrices, maximal cliques, or all transitive edges unless the product explicitly requires the output and its cost is bounded.

**MUST:** Distinguish requested semantics from execution limits. “Within three hops” can be a complete answer to a three-hop query. A memory cap that interrupts an unbounded query is partial, not complete. Preserve partial reasons, uncovered frontier/scope where practical, and prohibit interpreting incomplete non-findings as absence.

**Conditionality:** For oversized graphs, use a proven closed region, batched frontier retrieval, or a genuinely out-of-core/distributed implementation. Do not partition by arbitrary Arrow batches/files and combine local SCCs, PageRanks, or communities as though they were global results. Metadata partitioning is not graph independence.

Estimate end-to-end cost:

```text
scan + relational preparation + remapping/graph construction
+ kernel iterations + result conversion/join + retained memory/publication
```

Account for algorithm-specific worst cases. Exact all-node betweenness or all-pairs similarity can be inappropriate even when linear-time SCC is cheap on the same graph. Prefer source-restricted, sampled, top-k, or approximate variants only when the output contract accepts that change. A top-k output does not prove subquadratic internal work.

**MUST:** Treat invalid cycles, bounded recursion, and monotone fixed points explicitly. SCC decomposition identifies cyclic regions; it does not determine their transfer semantics. Define convergence/iteration limits and error behavior. Never silently drop required facts to force termination.

## 8. Govern heuristic and numerical graph analyses

**MUST:** Preserve the distinction between exact graph results, exact static-analysis results under stated assumptions, and heuristics. “Exact” is relative to the projection and source coverage, not necessarily the full behavior of the program. A conservative may-call graph yields possible reachability; incomplete dynamic observations cannot establish non-reachability.

For communities, record the objective, resolution, direction/symmetrization, edge weighting, initialization/seed, iteration policy, and quality/diagnostic fields actually provided. Do not compare raw objective scores across different graph/objective definitions. Local community labels are run-local IDs; longitudinal identity requires explicit matching, not integer equality.

For iterative centrality/diffusion, record damping/personalization, normalization, stopping criteria, and convergence status where measurable. When a library does not expose a convergence guarantee or diagnostic, report it as unavailable rather than claiming convergence. Canonical node order plus a seed does not guarantee bitwise reproducibility across versions, thread counts, hash iteration, or floating-point reductions.

For embeddings/similarity, store projection, feature/model/training versions, dimensions, normalization, seed, and distance metric. Use sparse bounded candidate generation rather than defaulting to all-pairs similarity. Graph vectors support retrieval/ranking; retain source-backed evidence for interpreted claims.

## 9. Use an explicit DataFusion ↔ graph execution boundary

**DEFAULT pipeline:**

```text
Pinned fact snapshot
  → DataFusion: filter, validate, aggregate/select direct relationships
  → narrow Arrow node/edge batches
  → backend-specific immutable graph projection
  → one or more scoped graph/static-analysis kernels
  → Arrow result relations keyed by domain IDs
  → DataFusion: evidence joins, interpretation features, ranking, serving views
  → optional durable artifact publication
```

If a query only needs relational work, skip graph construction entirely. If several analyses share one valid projection, do not re-extract and rebuild it for each algorithm. Avoid unnecessary graph→rows→graph cycles.

**DEFAULT integration:** For offline/precomputed analytics, call kernels at batch boundaries and expose outputs through `RecordBatch`, `MemTable`, or a suitable existing provider. Do not build custom operators solely to make a library callable.[^df-provider]

**Conditionality:** Use a custom logical node/`ExecutionPlan` when first-class composition, scheduling, profiling, reuse, or bounded query-time execution justifies integration. A graph-wide computation is not an ordinary scalar UDF repeated once per row. A table function/provider or extension operator must still implement the actual global/partitioned semantics.[^df-operators]

**MUST for integrated operators:** Declare required columns, partition/distribution requirements, ordering, boundedness, materialization, cancellation, and result cardinality. Block filter/limit pushdown unless it preserves the analysis universe; pushing `LIMIT` below a graph operation is not generally safe. Reserve/account for adjacency and workspace memory, not just Arrow buffers. DataFusion provides memory-consumer/reservation machinery, but wrapping an allocation in an operator does not make that allocation automatically spillable.[^df-extension][^df-memory]

Avoid nested unbounded Rayon/thread pools or CPU-heavy blocking work on an async executor thread. Use the application's bounded CPU execution mechanism. For third-party kernels without cooperative cancellation, define admission/budget limits and stale-result handling rather than promising immediate cancellation.

## 10. Publish relational results with explicit validity

**DEFAULT:** Keep schemas specific to output meaning rather than putting every result in a generic property bag. Typical relations include:

```text
analysis_runs(run_id, snapshot_id, projection_id, algorithm_id, config_ref,
              source_coverage, execution_status, precision, convergence, metrics_ref)
node_metrics(run_id, node_id, metric_kind, value)
component_memberships(run_id, component_id, node_id)
condensation_edges(run_id, source_component_id, target_component_id)
communities(run_id, community_id, node_id, level)
path_steps(run_id, path_id, ordinal, edge_id, traversal_direction)
analysis_evidence(run_id, result_key, source_fact_id, derivation_kind)
```

These are schema patterns, not a requirement to implement unused tables. A reverse traversal witness retains the original edge identity and records orientation rather than inventing a reversed source fact.

**MUST:** Preserve run/projection lineage, extraction configuration, source scope, algorithm implementation/version, and evidence sufficient to substantiate agent-facing claims. Keep source incompleteness, operational truncation, and heuristic approximation separate. Do not replace an earlier complete artifact with a partial one under the same validity label.

**DEFAULT:** Cache immutable projections/results by relevant content/scope plus projection and algorithm configuration. Include membership/addition/deletion dependencies, not merely the previously visited edge set. Keep full snapshot provenance even when safe reuse is based on narrower unchanged content. Use the existing build/analysis orchestration; no new incremental framework is required.

**Conditionality:** Update cheap direct aggregates locally. Recompute the affected semantic analysis region for global metrics, SCCs, or communities unless a validated incremental algorithm is present. Adding one edge can merge SCCs or affect remote scores; deleting an edge can split a region. Avoid both full-repository rebuilds on every edit and unjustified “only the changed node” updates.

## 11. Delta Lake and temporal graphs answer different questions

### Durable storage

Use Delta when its table-level transaction/versioning and supported Rust integration are useful. Keep an existing immutable Parquet/Arrow snapshot plus manifest design when that already meets durability needs. Do not introduce Delta merely to obtain graph adjacency, selective traversal, or query pushdown. Delta's Rust crate exposes table operations and DataFusion integration; exact features must be checked against the chosen release/protocol.[^delta-rs]

**MUST if using Delta:** Read through a pinned table snapshot/provider, not a glob of every Parquet file beneath a Delta directory. Publish a graph-build manifest referencing exact node/edge/evidence/result table versions after writes and validation succeed. A reader must pin that manifest; independently reading each table's latest version is not a consistent multi-table release. Delta documents transactions at table scope, not multi-table atomicity.[^delta-faq]

**DEFAULT:** Persist canonical facts, analysis metadata, compact results, and selected reusable artifacts. Do not commit every visited set/frontier or create a Delta transaction per edge. Batch updates and manage retention for files needed by live snapshots. Graph serializations/CSR caches are disposable derived accelerators, not the canonical interchange format.

**Conditionality:** Use change data feed only where enabled and supported; it identifies changed table facts, not the complete graph-analytic invalidation region. Reconcile additions/removals and global dependencies explicitly.

### Temporal analysis

Use relational revision diffs and ordinary per-snapshot graph kernels for “what changed?” or “how did this score evolve across releases?” Prefer Raphtory when repeated event/window/validity-sensitive graph operations justify a temporal index and its integration cost.[^raphtory]

**MUST:** Distinguish commit/snapshot identity, event time, edge-validity intervals, and ingestion time. Git history is not a single timestamp-ordered lineage; do not mix incompatible branches into a fictitious execution history.

Raphtory 0.17's `PersistentGraph` models edges active from addition until deletion and window membership by activity during the window. Its unwindowed view can include historically added/deleted edges. Choose the exact view deliberately.[^raphtory-persistent]

A union of edges seen during a window does not establish a time-respecting path or that all edges coexisted. Request a temporal algorithm with the needed semantics, or perform explicit interval/event reasoning; never relabel a static window-union traversal as temporal causality.

## 12. Acceptance and implementation discipline

For each implemented analytical operation, record:

```text
Semantic question / agent-facing use:
Authoritative inputs and extraction context:
Projection spec, scope, completeness, and direction:
Algorithm + crate/version/features + license decision:
Adapter/remapping and representation costs:
Graph/weight preconditions and numerical conventions:
Exact / conservative / heuristic / approximate meaning:
Termination, budgets, and partial-result behavior:
Arrow output schema and evidence linkage:
Cache key, invalidation scope, retention/publication:
Expected complexity and measured stage metrics:
Reason for departures from defaults:
```

**MUST:** Test the chosen implementation against semantic invariants and small known-answer fixtures. Include isolates, parallel edges, self-loops, sparse/deleted indices, reconvergent paths, cross-file SCCs, unresolved targets, mixed configurations, and malformed weights. Add temporal add/delete/re-add and non-coexisting-path fixtures only for temporal features. Validate source-edge lineage after projection transformations.

Compare partitions by membership, not raw labels; compare numerical outputs under justified tolerances and matching conventions. Cross-library checks are useful for selected algorithms, but a broad “reference” crate is not a correctness oracle merely because of its name. Approximate/heuristic outputs need appropriate quality/stability checks, not false exact-equality expectations.

Instrument scan/planning, graph construction/remapping, kernel work, result conversion, memory, and cache reuse separately. Do not build competing complete architectures to decide these boundaries. Implement the smallest required backend set, verify its semantics, then optimize the measured expensive stage.

**Final rule:** Keep authoritative meaning relational and provenance-rich; choose physical execution by the operation's structure; never let a graph library's convenient API silently change the question being answered.

---

## Source notes

Release-specific capability notes are grounded in the following primary documentation/manifests. Some documentation URLs are rolling; check the displayed release and the project's locked source before coding. Context7 supplied discovery/context; the rustworkx 0.18.1 tagged manifest and graphops 0.5.1 published source resolve important version-specific details.

[^arrow]: Apache Arrow Rust, computation kernels: https://arrow.apache.org/rust/arrow/compute/index.html
[^df-dataframe]: Apache DataFusion, DataFrame execution/streaming/cache: https://datafusion.apache.org/library-user-guide/using-the-dataframe-api.html
[^df-provider]: Apache DataFusion, custom providers and in-memory batch integration: https://datafusion.apache.org/library-user-guide/custom-table-providers.html
[^df-operators]: Apache DataFusion, extending operators: https://datafusion.apache.org/library-user-guide/extending-operators.html
[^df-extension]: Apache DataFusion, logical extension-node contracts, including conservative predicate/limit pushdown: https://github.com/apache/datafusion/blob/main/datafusion/expr/src/logical_plan/extension.rs
[^df-memory]: DataFusion memory-consumer API: https://docs.rs/datafusion/latest/datafusion/execution/memory_pool/struct.MemoryConsumer.html
[^petgraph]: Petgraph 0.8.3 representations and algorithm index: https://docs.rs/petgraph/0.8.3/petgraph/ and https://docs.rs/petgraph/0.8.3/petgraph/algo/index.html
[^graphmap]: Petgraph 0.8.3 GraphMap: https://docs.rs/petgraph/0.8.3/petgraph/graphmap/struct.GraphMap.html
[^csr]: Petgraph 0.8.3 Csr: https://docs.rs/petgraph/0.8.3/petgraph/csr/struct.Csr.html
[^rustworkx]: rustworkx-core Rust API, displayed release 0.18.1 when checked: https://docs.rs/rustworkx-core/latest/rustworkx_core/ ; tagged core manifest: https://github.com/Qiskit/rustworkx/blob/0.18.1/rustworkx-core/Cargo.toml
[^rustworkx-manifest]: rustworkx 0.18.1 workspace manifest, including Petgraph 0.8: https://github.com/Qiskit/rustworkx/blob/0.18.1/Cargo.toml
[^leiden]: leiden-rs Rust API, displayed release 0.8.1 when checked; objectives, methods, optional dependencies: https://docs.rs/leiden-rs/latest/leiden_rs/
[^leiden-data]: leiden-rs GraphData CSR representation, displayed release 0.8.1: https://docs.rs/leiden-rs/latest/leiden_rs/graph/data/struct.GraphData.html
[^graphops]: graphops 0.5.1 API: https://docs.rs/graphops/0.5.1/graphops/
[^graphops-ref]: graphops 0.5.1 borrowed-neighbor GraphRef: https://docs.rs/graphops/0.5.1/graphops/graph/trait.GraphRef.html
[^graphops-community]: graphops 0.5.1 documents connectivity-refined Louvain rather than Leiden refinement: https://docs.rs/graphops/0.5.1/graphops/leiden/index.html ; repository source inspected: https://github.com/arclabs561/graphops/blob/main/src/leiden.rs
[^graphops-cargo]: Published graphops 0.5.1 manifest, optional Petgraph 0.6 dependency: https://docs.rs/crate/graphops/0.5.1/source/Cargo.toml
[^graphops-adapter]: graphops 0.5.1 materializing PetgraphRef adapter: https://docs.rs/graphops/0.5.1/graphops/graph/struct.PetgraphRef.html
[^graphops-walks]: graphops 0.5.1 Node2Vec/Node2Vec+ walk-generation module: https://docs.rs/graphops/0.5.1/graphops/node2vec/index.html
[^graphina]: Graphina API, displayed release 0.4.0-alpha.6 when checked, feature-gated modules: https://docs.rs/graphina/latest/graphina/
[^rust-igraph]: rust-igraph 0.7.0 capability and license declarations: https://docs.rs/rust-igraph/0.7.0/rust_igraph/
[^raphtory]: Raphtory 0.17.0 Rust API and license declaration: https://docs.rs/raphtory/0.17.0/raphtory/
[^raphtory-persistent]: Raphtory 0.17.0 PersistentGraph time/deletion/window semantics: https://docs.rs/raphtory/0.17.0/raphtory/db/graph/views/deletion_graph/struct.PersistentGraph.html
[^datafrog]: Datafrog recursive relational inference model: https://docs.rs/datafrog/latest/datafrog/
[^delta-rs]: delta-rs Rust table/operations/DataFusion integration API: https://docs.rs/deltalake/latest/deltalake/
[^delta-faq]: Delta Lake FAQ, table-level transactions and versioned storage: https://docs.delta.io/delta-faq/
