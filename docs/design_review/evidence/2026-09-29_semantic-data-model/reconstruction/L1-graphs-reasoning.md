# L1: graph and reasoning library capabilities at the pinned versions

Date: 2026-09-29. Scope: library evidence for the target architecture: typed relations → declared
projections (petgraph multigraphs with arc-id weights, shared node dictionaries, adjacency in both
directions, filtered views) → operators (topology; fixed-point transfer with SCC scheduling;
interprocedural composition; rules and patterns; governed heuristics) → derived relations with
AND/OR derivations, with conditions as BDDs. No repository file was edited, and nothing was built in
the product workspace.

## Evidence labels and paths

- **source-read**: exact-version crate source under
  `$REG = ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/<crate>-<version>/`, or repository
  source under `/home/paul/library-context/`.
- **skill-brief**: a reviewed brief or container page in the pinned skills
  `/home/paul/library-context/.claude/skills/{rust-graphs,rust-reasoning}/content/...`.
- **skill-probe**: a probe from a skill's ledger (`content/probes/00-index.md`; matrix probes `M-<Container>`).
- **docs**: upstream README or rustdoc text, or crates.io API metadata. Context7 was queried for
  `cranelift-entity` and `typed-index-collections` and indexes neither, so their evidence comes from
  source-read plus crates.io metadata.
- **review-probe**: a probe I ran for this review on 2026-09-29, in isolated crates in the session
  scratchpad (see "Review probes" at the end). They were built with `--offline` from the registry on
  the default nightly (cargo 1.101.0-nightly 2026-09-25). That is not the repository's pinned
  `nightly-2026-09-29`, but it is the same crate versions.

### Version check (pins versus the versions the skills index)

| Crate | Product pin | Evidence |
|---|---|---|
| petgraph | `=0.8.3` (`Cargo.toml:34`), lock 0.8.3 | rust-graphs indexes 0.8.3: match |
| fixedbitset | `=0.5.7` (`Cargo.toml:39`), lock 0.5.7 | boundary crate of rust-reasoning; source-read |
| roaring | not a direct dependency; lock 0.11.5 (`Cargo.lock:6784`), pulled by `buoyant_kernel` 0.25.1 and `deltalake-core` (`Cargo.lock` ~976, ~2708) | source-read 0.11.5 |
| biodivine-lib-bdd | `=0.6.3` (`Cargo.toml:35`), lock 0.6.3 | rust-reasoning indexes 0.6.3: match |
| ascent | not in the product lock; the evidence probe pins `=0.8.1` | rust-reasoning indexes 0.8.1: match |
| datafrog | not in the product lock; the evidence probe pins `=2.0.1` | rust-reasoning indexes 2.0.1: match |
| rustworkx-core | not in the product lock | rust-graphs indexes 0.18.1; crates.io max stable 0.18.1 (queried 2026-09-29) |
| typed-index-collections | not in the product | crates.io max 3.5.0 (created 2026-01-18); source-read 3.5.0 |
| cranelift-entity | not in the product | crates.io max 0.136.1 (2026-09-24); source-read 0.121.2 (registry) and 0.136.1 (downloaded to scratchpad) |

---

## 1. petgraph =0.8.3

### (a) `Csr`: parallel edges, incoming traversal, blocked algorithms

- **No parallel edges.** source-read `$REG/petgraph-0.8.3/src/csr.rs:67` says "Self loops are
  allowed, no parallel edges."
  - `pub fn from_sorted_edges<Edge>(edges: &[Edge]) -> Result<Self, EdgesNotSorted> where Edge: Clone + IntoWeightedEdge<E, NodeId = NodeIndex<Ix>>, N: Default`
    (csr.rs:192–264) requires targets that rise strictly within a row
    (`if !last_target.map_or(true, |x| m > x) { return Err(EdgesNotSorted{..}) }`). A duplicate
    `(u,v)` is an **error**, not a second edge. The node count is `max id + 1`.
  - `add_edge(a,b,w) -> bool` and `try_add_edge(..) -> Result<bool, CsrError>` (csr.rs:313–397)
    return `false` when the pair exists (`find_edge_pos` returns `Ok` and the result is `Ok(false)`).
    **The new weight is dropped.**
  - review-probe confirmed both: `from_sorted_edges(&[(0,1,7),(0,1,8)]).is_err() == true`; a second
    `add_edge(0,1,8)` gives `false`, with `edge_count == 1` and weights `[7]`.
- **No incoming traversal.** `Csr` stores only outgoing rows (`column`, `edges`, `row`;
  csr.rs:73–84). It implements `IntoNeighbors` and `IntoEdges`, but not `IntoNeighborsDirected` or
  `IntoEdgesDirected` (csr.rs:547–870).
  - `Reversed(&csr)` gives nothing usable, because `impl IntoNeighbors for Reversed<G>` requires
    `G: IntoNeighborsDirected` (`$REG/petgraph-0.8.3/src/visit/reversed.rs:25–27`).
  - Incoming traversal needs a second, transposed `Csr`.
  - `EdgeId = EdgeIndex` is "index into edges vector" (csr.rs:635). It shifts on insertion, and
    `EdgeIndexable` is not implemented.
- **Blocked algorithms.** skill-probe `M-Csr` (proved by `cargo check`,
  `content/containers/Csr.md`) finds 14 of petgraph's 43 generic algorithms blocked:
  - `all_simple_paths`, `all_simple_paths_multi`, `bidirectional_dijkstra`,
    `tred::dag_to_toposorted_adjacency_list`, `dinics`, `ford_fulkerson`
  - `is_isomorphic`, `is_isomorphic_matching`, `is_isomorphic_subgraph`,
    `is_isomorphic_subgraph_matching`, `subgraph_isomorphisms_iter`
  - `kosaraju_scc`, `scc`, `toposort`
  - The missing bounds are mostly `IntoNeighborsDirected` or `IntoEdgesDirected`, sometimes also
    `DataMap` or `EdgeIndexable`.
  - For rustworkx-core, 62 of 91 functions are blocked, including
    `lexicographical_topological_sort`, `ancestors`, `descendants`, `layers` and `longest_path`.
  - Still available on `&Csr`: `tarjan_scc`, `TarjanScc`, `has_path_connecting`,
    `dominators::simple_fast`, `dijkstra` and `page_rank`.
- **Fit for arc-id multigraphs: poor.** The distinct arcs of one `(u,v)` pair cannot be stored. The
  workarounds are:
  - one Csr edge per pair whose weight is a range into an arc-id array, or
  - a hand-rolled CSR per direction: `offsets: Vec<u32>` plus `arcs: Vec<ArcId>` in canonical
    order. That structure has no petgraph visit traits unless you implement them.

### (b) `algo::page_rank` against `crates/lctx-analytics/src/ranking.rs`

**The petgraph function** (source-read `$REG/petgraph-0.8.3/src/algo/page_rank.rs:63–103`):

- Signature: `pub fn page_rank<G, D>(graph: G, damping_factor: D, nb_iter: usize) -> Vec<D> where G: NodeCount + IntoEdges + NodeIndexable, D: UnitMeasure + Copy`
- It panics unless `0 ≤ d ≤ 1`.
- Start: uniform `1/n`.
- `out_degrees[w]` counts out-edges. Parallel edges count; **edge weights are ignored**.
- Each iteration, for every pair (v, w):
  - if `graph.edges(w).any(|e| e.target()==v)`, add `d·r_w/deg_w`. This is added **once, however
    many parallel edges there are**;
  - else if `deg_w == 0`, add `d·r_w/n`;
  - else add `(1−d)·r_w/n`;
  - then normalize by the sum.
- So the teleport share is withheld from linked pairs, and dangling mass is only `d·r/n` before
  renormalization. **This is not the standard formulation.**
- Iterations are fixed. There is **no tolerance, residual or converged flag**.
- Cost is O(nb_iter·|V|²·deg): the documentation says "O(n|V|²|E|)".
- `parallel_page_rank(graph, d, nb_iter, tol: Option<D>)` (page_rank.rs:134) exists only with the
  `rayon` feature. The repository does not enable it, and on convergence it returns the *previous*
  ranks.

**The repository kernel** (source-read `crates/lctx-analytics/src/ranking.rs:264–310`):

- Signature: `pagerank(n, arcs: &BTreeMap<(u32,u32),u64>, params)`
- Formula: `r' = (1−d)/n + d·(Σ r_i·w_ij/W_i + D/n)`. It is weighted by arc count, and the dangling
  mass is spread uniformly.
- Stop rule: L1 residual `< 1e-10` or 100 iterations. It records iterations, residual and
  `converged`, and `run` maps a non-converged result to `CoverageStatus::Partial`
  (ranking.rs:386–391).
- Cost: O(iter·(V+E)), summed in canonical `BTreeMap` order.
- It already has an oracle, `leiden_rs::infomap::compute_flow` (tests at ranking.rs:592–660).

**Could petgraph's replace it without changing results? No.** review-probe, d = 0.85, 100
iterations:

| Graph | Max \|petgraph − repo\| |
|---|---|
| Control: a 3-cycle (no dangling node, no parallel edges) | 0 |
| Simple graph with one dangling and one isolated node | 9.6e-3 |
| The same graph with one parallel arc pair | 3.2e-2 |

The differences, in order of weight:

1. The teleport and dangling formula.
2. No weights.
3. Parallel edges are counted in the denominator only.
4. No convergence report.
5. O(V²) cost.

rustworkx-core 0.18.1 has no PageRank (skill coverage `pagerank` lists petgraph, graphops, graphina,
rust-igraph and raphtory). **Keep the repository kernel.**

### (c) `algo::dominators::simple_fast`

source-read `$REG/petgraph-0.8.3/src/algo/dominators.rs:24–95` and `178–260`.

- Signature: `pub fn simple_fast<G>(graph: G, root: G::NodeId) -> Dominators<G::NodeId> where G: IntoNeighbors + Visitable, G::NodeId: Eq + Hash`
- Algorithm: Cooper–Harvey–Kennedy, **O(|V|²)**. Post-order comes from an iterative
  `DfsPostOrder` from `root`, and predecessor sets are built on the way.
- Requirements:
  - one root;
  - only nodes reachable from the root are included;
  - multiple entries (or, for post-dominators, multiple exits) need a virtual root node added to
    the graph, because a view cannot add one.
- API on `Dominators<N>` (with `N: Copy+Eq+Hash`):
  - `root()`
  - `immediate_dominator(n) -> Option<N>`: `None` for the root and for unreachable nodes
  - `strict_dominators(n)` and `dominators(n) -> Option<DominatorsIter>`: `None` if n is unreachable
  - `immediately_dominated_by(n) -> DominatedByIter`
- **Order trap:** `immediately_dominated_by` iterates a `hashbrown::HashMap` with the default
  foldhash `RandomState` (petgraph enables `hashbrown` `default-hasher`). The order is not canonical,
  so sort it.
- Accepted inputs: `&Csr`, `&NodeFiltered`, `&EdgeFiltered` and `Reversed(..)`. Post-dominators are
  `simple_fast(Reversed(&view), exit)`. review-probe compiled and ran both on a `NodeFiltered` view.

### (d) Condensation, SCCs, reachability and topological order

- **`condensation`** (source-read `$REG/petgraph-0.8.3/src/algo/mod.rs:477–512`):
  - Signature: `pub fn condensation<N, E, Ty, Ix>(g: Graph<N, E, Ty, Ix>, make_acyclic: bool) -> Graph<Vec<N>, E, Ty, Ix>`.
    It accepts only a **concrete `Graph`, by value**, not a view.
  - `make_acyclic = false` keeps every edge (`add_edge`), including intra-SCC self-loops and
    parallel inter-SCC edges.
  - `make_acyclic = true` drops intra-SCC edges and calls `update_edge`. Parallel inter-SCC edges
    collapse to one, and the **last weight overwrites the earlier ones, so arc ids are silently
    lost**. review-probe: edges with arc ids 0..3 give weights `[0,1,2,3]` with `false` and `[3]`
    with `true` (arc 2 lost).
  - Condensed node order is the `kosaraju_scc` order, reverse topological (skill-probe B002).
- **`TarjanScc` is not iterative.** Both the docs and the code say "This implementation is
  recursive" (source-read `$REG/petgraph-0.8.3/src/algo/scc/tarjan_scc.rs:53`, `visit` recursion at
  98–100). API:
  - `TarjanScc::new()`
  - `run<G,F>(&mut self, g: G, f: F) where G: IntoNodeIdentifiers<NodeId=N> + IntoNeighbors<NodeId=N> + NodeIndexable<NodeId=N>, F: FnMut(&[N]), N: Copy+PartialEq`.
    It is reusable state; the callback runs per SCC in reverse topological order, and member order
    within an SCC is arbitrary.
  - `node_component_index(&self, g, v) -> usize`
  - `tarjan_scc(g) -> Vec<Vec<N>>` wraps it.
  - review-probe: on a path graph, `tarjan_scc` **overflows the 8 MiB main stack at 100,000 nodes**
    (release) and succeeds at 60,000. Threads with a smaller stack (for example the 2 MiB default for
    spawned threads) reach the limit sooner (not measured).
- **`kosaraju_scc`**:
  - Signature: `pub fn kosaraju_scc<G>(g: G) -> Vec<Vec<G::NodeId>> where G: IntoNeighborsDirected + Visitable + IntoNodeIdentifiers`
    (kosaraju_scc.rs:96–140).
  - It is iterative (`DfsPostOrder` over `Reversed(g)`, then `Dfs`). review-probe: fine at 1M nodes.
  - It is blocked on `Csr`. The repository already uses it (`crates/lctx-analytics/src/summaries.rs:60`).
- **`has_path_connecting`**:
  - Signature: `pub fn has_path_connecting<G>(g: G, from: G::NodeId, to: G::NodeId, space: Option<&mut DfsSpace<G::NodeId, G::Map>>) -> bool where G: IntoNeighbors + Visitable`
    (mod.rs:369–383).
  - `DfsSpace::new(g)` (mod.rs:310) reuses the visit map across queries.
  - `from == to` gives `true`. Each query costs O(V+E).
- **`toposort`**:
  - Signature: `pub fn toposort<G>(g: G, space: Option<&mut DfsSpace<..>>) -> Result<Vec<G::NodeId>, Cycle<G::NodeId>> where G: IntoNeighborsDirected + IntoNodeIdentifiers + Visitable`
    (mod.rs:211–276).
  - It is iterative, and a self-loop returns `Err`. review-probe: fine at 1M nodes.
  - The order depends on `node_identifiers` and on neighbour iteration. `Graph` lists neighbours
    **newest-first** (`graph_impl/mod.rs:919–1012`), so the result is not a canonical order.
    `is_cyclic_directed` is recursive (mod.rs:271–291).

### (e) The `NodeFiltered`, `EdgeFiltered` and `Reversed` adaptors over `Graph<(), u32, Directed, u32>`

**`NodeFiltered<G,F>(pub G, pub F)`** (source-read `$REG/petgraph-0.8.3/src/visit/filter.rs:18–355`):

- `F: FilterNode<N>` is implemented for `Fn(N)->bool`, `FixedBitSet`, `&FixedBitSet` and
  `HashSet`/`&HashSet`. `FixedBitSet: VisitMap<NodeIndex<u32>>` holds because
  `NodeIndex<Ix>: IndexType` (`graph_impl/mod.rs:128`, `visit/mod.rs:414`).
- `&NodeFiltered` implements `IntoNeighbors`, `IntoNeighborsDirected`, `IntoNodeIdentifiers`,
  `IntoNodeReferences`, `IntoEdgeReferences`, `IntoEdges`, `IntoEdgesDirected`, `DataMap`, `Data`,
  `NodeIndexable`, `EdgeIndexable`, `GraphProp` and `Visitable`.
- **Missing:** `NodeCount`, `EdgeCount`, `NodeCompactIndexable` and `GetAdjacencyMatrix`
  (skill-brief `graph.adaptors`: "loses NodeCount and EdgeCount").

**`EdgeFiltered<G,F>`** (filter.rs:381–585):

- `F: FilterEdge<G::EdgeRef>`. The filter sees the `EdgeReference`, **including the arc-id weight**,
  so a filter by relation kind is a side-table lookup.
- It has `NodeCount`, `NodeCompactIndexable`, `NodeIndexable`, `EdgeIndexable`, `Visitable`,
  `IntoNodeIdentifiers` and the directed neighbour and edge traits.
- It lacks `EdgeCount` and `DataMap`.

**`Reversed<G>`** (reversed.rs:16–184) forwards nearly everything, including `NodeCount`,
`EdgeCount`, `NodeCompactIndexable`, `EdgeIndexable` and `GetAdjacencyMatrix`. It needs the wrapped
value to be a reference type with the directed traits.

**What composes** (review-probe, compiled and run in `views.rs`):

- `Reversed(&EdgeFiltered(&g,f))` and `Reversed(&NodeFiltered(&g,&mask))` both work.
- On `NodeFiltered`: `tarjan_scc`, `kosaraju_scc`, `toposort`, `has_path_connecting` with a
  `DfsSpace`, `simple_fast` and `dijkstra`.
- On `EdgeFiltered` (by arc kind): `toposort` and `page_rank`.
- On `Reversed(view)`: `has_path_connecting`, `simple_fast` (post-dominators) and `kosaraju_scc`.
- **Negative control:** `page_rank(&NodeFiltered(..))` fails with E0277 (`NodeFiltered<&Graph<(), u32>, &FixedBitSet>: NodeCount` is not satisfied).
- Not accepted by views: `condensation` (it takes a concrete `Graph`), and rustworkx
  `lexicographical_topological_sort` on `NodeFiltered` (it needs `NodeCount`).
- The predicate runs on every visit and is not memoized (skill-brief `graph.adaptors`).
- Visit maps are sized by the base graph's `node_bound()`.

### (f) `acyclic::Acyclic`

source-read `$REG/petgraph-0.8.3/src/acyclic.rs` and `acyclic/order_map.rs`.

- **Features:** the module is unconditional (`pub mod acyclic;`, lib.rs:502). Only the
  `StableDiGraph` impls need `stable_graph`, which is a default feature and on.
- **Algorithm:** Pierce–Kelly dynamic topological order (acyclic.rs:34–52).
- **Construction:** `Acyclic::<G>::new()`, `try_from_graph(G) -> Result<Self, Cycle<N>>`, and
  `TryFrom<DiGraph|StableDiGraph>`.
- **Edges:**
  - `try_add_edge(a, b, w) -> Result<G::EdgeId, AcyclicEdgeError<N>>`. The errors are
    `Cycle(Cycle<N>)`, `SelfLoop` and `InvalidEdge`.
  - `try_update_edge(..)` and `is_valid_edge(a,b) -> bool`.
  - The trait method `Build::update_edge` panics on a cycle, and `Build::add_edge` returns `None`.
- **Order:**
  - `get_position(n) -> TopologicalPosition` (`Ord`) and `at_position(pos) -> Option<N>`
    (order_map.rs:187–197)
  - `nodes_iter()` and `range(..)`, both in topological order
  - `inner()` and `into_inner()`
  - `remove_edge` and `remove_node`, for DiGraph and StableDiGraph only
- **Behaviour:** multi-edges are allowed on `DiGraph` and self-loops are refused. The maintained
  order depends on insertion history, so it is not canonical.
- **Memory:** an extra `BTreeMap<TopologicalPosition,N>`, a `Vec<TopologicalPosition>` and two
  `FixedBitSet`s (acyclic.rs:68–81).
- skill-probe `M-Acyclic`: all 43 algorithms compile against it.

### (g) Memory for `Graph` with u32 indices

- review-probe `size_of`: `Node<(),u32>` is **8 B** (`[EdgeIndex<u32>;2]`) and `Edge<u32,u32>` is
  **20 B** (weight 4 + next 2×4 + endpoints 2×4). This follows from the structs
  (`graph_impl/mod.rs:220–251`), and `Graph` is two `Vec`s (mod.rs:390–394).
- Both directions are included, as intrusive linked lists, plus `Vec` growth slack of up to 2×.
- Example: 1M vertices and 5M arcs ≈ 8 MB + 100 MB before slack.
- `StableGraph` stores `Option`-wrapped slots (`Option<u32>` = 8 B) plus free lists.
- `Csr<(),u32>` costs 8 B per edge and 8 B per node (`usize` row) in one direction; two of them
  cover both directions but still cannot hold parallel edges.
- A hand-rolled CSR of arc ids costs ≈ 4 B per arc per direction plus 4 B per node per direction.

### (h) Features

- The workspace dependency is `petgraph = "=0.8.3"` with default features (`Cargo.toml:34`).
- `cargo tree --frozen -e features -i petgraph@0.8.3` (run 2026-09-29, read-only) shows these
  features resolved: **default, std, graphmap, stable_graph, matrix_graph**.
  - They are requested by `lctx-analytics` and `datafusion-physical-expr` 55.1.0.
  - `cpg-core` uses petgraph only as a **dev-dependency** (`crates/cpg-core/Cargo.toml:40,45`).
- Not enabled: `rayon` (so no `parallel_page_rank`), `serde-1`, `quickcheck`, `dot_parser`,
  `unstable`/`generate`.
- Everything in this section is available without feature changes.
- `lctx-analytics` depends on `fixedbitset.workspace` (`crates/lctx-analytics/Cargo.toml:13`).

---

## 2. rustworkx-core (latest 0.18.1)

**It shares petgraph 0.8.3; there is no second petgraph.**

- `$REG/rustworkx-core-0.18.1/Cargo.toml` asks for `petgraph = "0.8"` with no feature override.
  review-probe (`probe2`, `cargo tree -d`) shows a single `petgraph v0.8.3` shared by the probe and
  rustworkx-core. The duplicates listed are only hashbrown 0.15 and 0.17 and foldhash 0.1 and 0.2,
  which the product lock already carries.
- skill-brief `content/catalogs/releases.md`: 0.17.x and 0.18.x all track "petgraph 0.8".
- The version is current: crates.io max stable is 0.18.1 (2026-07-29).

**Cost against the product lock** (review-probe lock diff):

- It adds 7 crates: `rustworkx-core 0.18.1`, `ndarray 0.17.2`, `matrixmultiply 0.3.11`,
  `rawpointer 0.2.1`, `priority-queue 2.7.0`, `rand_distr 0.6.0` and `rayon-cond 0.4.0`.
- `rayon 1.12`, `rand 0.10.3` and `rand_pcg 0.10.2` are already present.
- `rayon` is **mandatory**, and it enables the `rayon` feature on `hashbrown` and `indexmap`
  through feature unification. The crate has no `[features]` to trim this.
- MSRV 1.85; Apache-2.0.
- It returns hashbrown 0.17 sets (skill-probe C005).

**`dag_algo::lexicographical_topological_sort`** (source-read dag_algo.rs:146–162):

- Signature: `pub fn lexicographical_topological_sort<G, F, K, E>(dag: G, key: F, reverse: bool, initial: Option<&[G::NodeId]>) -> Result<Vec<G::NodeId>, TopologicalSortError<E>>`
  `where G: GraphProp<EdgeType = Directed> + IntoNodeIdentifiers + IntoNeighborsDirected + IntoEdgesDirected + NodeCount, G::NodeId: Hash + Eq + Ord, F: FnMut(G::NodeId) -> Result<K, E>, K: Ord, E: Error`
- `reverse = true` gives the order of the reversed graph. For a caller→callee graph, that is a
  callee-first order.
- It returns an error on a cycle (`TopologicalSortError`).
- skill-brief `graph.dag-analysis`: "the reverse flag sorts the REVERSED graph; it does not reverse
  key order".
- review-probe: on `condensation(g, true)` with key = the component's minimum member id and
  `reverse = true`, the schedule is `[[10,20],[40],[30],[50],[60]]`. That is the order the
  repository's hand-written Kahn loop in `call_components` (`summaries.rs:94–121`, ready set ordered
  by `(members[0], index)`) produces by construction.
- It does **not** compile on `Csr` (M-Csr) or on `NodeFiltered`, because both lack `NodeCount`.

**DFS or BFS visitor with `Control::Prune`** (source-read `traversal/dfs_visit.rs:1–60` and `196–215`):

- Signature: `pub fn depth_first_search<G, I, F, C>(graph: G, starts: I, visitor: F) -> C where G: IntoEdges + Visitable, I: IntoIterator<Item = G::NodeId>, F: FnMut(DfsEvent<G::NodeId, &G::EdgeWeight>) -> C, C: ControlFlow`
- It is **iterative** (header: "an iterative implementation of the upstream petgraph
  depth_first_search").
- The events carry the **edge weight (arc id)**: `TreeEdge(u,v,&w)`, `BackEdge(u,v,&w)`,
  `CrossForwardEdge(u,v,&w)`, as well as `Discover` and `Finish`.
- `Control::Prune` stops the current node's remaining edges, and `Control::Break(b)` returns.
- By contrast, petgraph's `visit::depth_first_search` (dfsvisit.rs:241, body 261–301) is
  **recursive** and its events carry no edge id.
- review-probe: pruning at node 1 stopped `1→2`, and the events were
  `tree 0->1 arc0`, `tree 4->3 arc4`, `back 3->3 arc3`.
- Also available: `breadth_first_search` (`BfsEvent`), `bfs_layers`, `dijkstra_search`,
  `ancestors`/`descendants` (these need directed traits), and in `dag_algo` also `layers`,
  `longest_path` and `collect_runs`.

**Dominance:** none in rustworkx-core 0.18.1. The only match for "dominat" is a doc comment
(dag_algo.rs:91). Dominators stay petgraph's `simple_fast`.

**Verdict:** optional. rustworkx-core is worth adopting if the operator layer needs an iterative
DFS that is labelled by arc and pruned, or a deterministic key-ordered DAG schedule. Otherwise keep
the 25-line Kahn loop.

---

## 3. Typed dense index domains: typed-index-collections and cranelift-entity

### typed-index-collections 3.5.0 (latest)

source-read `$REG/typed-index-collections-3.5.0/`.

- **Dependencies:** zero at run time (`serde` and `bincode` optional). MIT OR Apache-2.0; MSRV 1.90.
- **Types:** `TiVec<K,V> { pub raw: Vec<V> }` and `TiSlice<K,V> { pub raw: [V] }` (vec.rs:101,
  slice/mod.rs:151–171). `K: From<usize> + Into<usize>`.
- **Methods:**
  - `push_and_get_key(v) -> K` (vec.rs:516)
  - `next_key()`, `keys()`, `iter_enumerated() -> (K,&V)`, `position(..) -> Option<K>`
  - `binary_search(&V) -> Result<K,K>` and `binary_search_by_key` (slice/mod.rs:231–1315)
  - otherwise it mirrors the `Vec` and slice APIs
- **Fit:** the repository's `Projection::dense(id) = ids.binary_search(&id)`
  (`crates/lctx-analytics/src/graph.rs:198`) becomes `TiSlice<VertexIx, Id>::binary_search(&id) -> Result<VertexIx, VertexIx>`,
  which is a typed dictionary in both directions over a sorted id vector. Per-index side tables
  (`kinds`, `modules`, `roles`) become `TiVec<VertexIx, _>`.
- **Gaps:**
  - A `u32` newtype needs hand-written `From<usize>`, with a checked conversion, and `Into<usize>`.
  - There is no bridge to petgraph's `NodeIndex<u32>`, so conversion stays at the graph boundary.
  - It gives no id→index hash dictionary; sorted `binary_search` remains the lookup.

### cranelift-entity (latest 0.136.1; registry has 0.121.2)

source-read `$REG/cranelift-entity-0.121.2/src/` and the downloaded 0.136.1 `src/`.

- **Index type:** `EntityRef` (`new(usize)`, `index()`). `entity_impl!(T)` for `struct T(u32)`:
  `new` only runs `debug_assert!(index < u32::MAX)` and then `index as u32`, so it **truncates
  silently in release** (lib.rs:118–126 in both versions). `u32::MAX` is reserved for `PackedOption`.
- **`PrimaryMap<K,V>`:** `push -> K`, `next_key`, `get`, `keys`, `iter`, and
  `binary_search_values_by_key -> Result<K,K>`.
- **Other containers:**
  - `SecondaryMap<K, V: Clone>`: a dense side table with a default value
  - `EntitySet<K>`: a bitset
  - `SparseMap`
  - `EntityList` with `ListPool`: compact adjacency lists
- **Dependencies at 0.136.1** (crates.io API):
  - `cranelift-bitset =0.136.1`
  - **`wasmtime-internal-core =49.0.1` (exact pin)**, which brings `hashbrown ^0.17`
    (`default-hasher`) and `libm`
  - `serde` optional
- MSRV 1.96; license Apache-2.0 WITH LLVM-exception.
- **Release cadence:** a new 0.x every month, so every upgrade is semver-breaking.

**Fit:** both crates type the u32→value direction. Neither owns canonical `Id`↔`u32` identity; that
stays a sorted id vector plus binary search, or a hash map. **Adopt typed-index-collections if you
want typed domains; reject cranelift-entity** because of its dependency chain and cadence. Keeping
a small in-house newtype macro is also acceptable.

---

## 4. Set-valued analysis state: fixedbitset =0.5.7 and roaring 0.11.5

### fixedbitset 0.5.7

source-read `$REG/fixedbitset-0.5.7/src/lib.rs`.

- **Construction and growth:** `with_capacity`, `grow`, `grow_and_insert`
- **Single bits:** `insert`, `put(bit) -> bool` (returns the previous value), `set`, `remove`,
  `toggle`, `contains`
- **Ranges and counts:** `insert_range`, `count_ones(range)`, `minimum`/`maximum`
- **Iteration:** `ones()`, `into_ones()`, `zeroes()`
- **Lazy set operations:** `union`, `intersection`, `difference`, `symmetric_difference`
  (lib.rs:834–859)
- **In-place set operations:** `union_with`, `intersect_with`, `difference_with`,
  `symmetric_difference_with` (868–930). `union_with` grows `self`; `intersect_with` keeps its
  capacity.
- **Counts and comparisons:** the `*_count` methods, `is_disjoint`, `is_subset`, `is_superset`
  (933–1016), and `BitAnd`/`BitOr`/`BitXor` on `&` plus their `*Assign` forms.

Traps:

- **`Eq`, `Hash` and `Ord` include `length`** (lib.rs:1018–1043). Sets with the same members but
  different capacities compare unequal (review-probe: `eq=false`, `ones_eq=true`). Always allocate
  to the domain size.
- `Ord` compares length and then blocks lexicographically; it is **not** subset order.
- `union_with` returns no "changed" flag. Test `is_subset` first:
  `if !x.is_subset(&s) { s.union_with(&x); changed = true }`.

### roaring 0.11.5

- **Already in the lock** as a transitive dependency (`Cargo.lock:6784`) with its default `std`
  feature (`bytemuck`, `byteorder`). A direct dependency adds **no** crate.
- **`RoaringBitmap`** (u32 values; source-read `$REG/roaring-0.11.5/src/bitmap/`):
  - Construction: `new`, `full`, `from_sorted_iter`, `append`, `push` (sorted append)
  - Membership: `insert(u32) -> bool`, `insert_range -> u64`, `remove`, `contains`, `len -> u64`,
    `min`/`max`, `rank`/`select`
  - Iteration: `iter`, `range`
  - Set operators, by value and by reference: `BitOr`, `BitAnd`, `Sub`, `BitXor` and their
    `*Assign` forms (ops.rs)
  - Cardinality and comparison: `union_len`, `intersection_len` and the other `*_len`;
    `is_subset`, `is_superset`, `is_disjoint` (cmp.rs)
  - `MultiOps::union` and `intersection` over iterators (lib.rs:95)
  - Serialization: `serialize_into` and `deserialize_from` (portable Roaring format)
  - `optimize()` for run containers
  - `RoaringTreemap` covers u64 values.
- **Traits:** `Clone`, `Default`, `Debug`, and `PartialEq`/`Eq` (mod.rs:47). Equality is
  **semantic across container kinds** (store/mod.rs:832–853).
- **No `Hash` and no `Ord`/`PartialOrd`.** Use as an ascent column or a hash key needs a newtype.

**Fit:** roaring suits sparse sets over large dense-id domains, such as per-function reachable arcs
or origins. FixedBitSet suits small domains and dense sets.

---

## 5. biodivine-lib-bdd =0.6.3

source-read paths are under `$REG/biodivine-lib-bdd-0.6.3/src/`.

- **Existential quantification** (`_impl_bdd/_impl_relation_ops.rs:11–90`):
  - `exists(&self, variables: &[BddVariable]) -> Bdd`
  - `var_exists(&self, variable: BddVariable) -> Bdd`
  - `exists_trigger(Fn(BddVariable)->bool)`
  - `project` and `var_project` are `#[deprecated]` aliases.
  - Duals: `for_all`, `var_for_all`, `for_all_trigger`.
  - Relational products: `binary_op_with_exists(left, right, op, vars)` and `binary_op_nested`
    (`_impl_nested_ops.rs`).
  - **None of these has a node limit.**
  - **Bounded single-variable ∃ is available:**
    `Bdd::fused_binary_flip_op_with_limit(limit, (&f, None), (&f, Some(x)), None, op_function::or) -> Option<Bdd>`.
    This is exactly `var_exists`'s body (relation_ops.rs:19–26) with the limited variant
    (boolean_ops.rs:155–176).
- **Substitution:**
  - `substitute(&self, var: BddVariable, function: &Bdd) -> Bdd` (`_impl_util.rs:549–638`) replaces
    **one variable**, with syntactic semantics. It is **unbounded**: it uses
    `binary_op_with_exists` without a limit.
  - When `var ∈ support(function)`, it shifts variables and needs `num_vars+1`, so it panics at
    `u16::MAX`. Its doc says there is currently no simultaneous substitution.
  - Renaming: `unsafe fn rename_variable(old,new)`, `unsafe fn rename_variables(&HashMap<BddVariable,BddVariable>)`
    and `unsafe fn set_num_vars(u16)` (util.rs:32–120).
  - `BddVariableSet::transfer_from(&self, bdd, ctx) -> Option<Bdd>` renames by name and only while
    preserving order (`_impl_bdd_variable_set.rs:355–420`).
- **Restriction:** `restrict(&[(BddVariable,bool)])`, `var_restrict` and `restrict_valuation`.
  `select`/`var_select` conjoin with a literal (relation_ops.rs:161–205).
- **Node-capped apply:**
  - `binary_op_with_limit<T>(limit: usize, left: &Bdd, right: &Bdd, op_function: T) -> Option<Bdd>`
    (boolean_ops.rs:121–133). The limit counts result nodes, terminals included, and `None` means
    "too big" (skill-probe B003).
  - `fused_binary_flip_op_with_limit`.
  - Work-bounded dry run: `check_binary_op<T>(limit, left, right, op) -> Option<(bool /*non-empty*/, usize /*tasks*/)>`
    (boolean_ops.rs:180–205).
- **Witnesses:** `sat_witness(&self) -> Option<BddValuation>` (util.rs:271). Also `first_valuation`,
  `most_positive_valuation`, `random_valuation` and `sat_clauses()` (a lazy path iterator).
- **Serialization** (`_impl_serialisation.rs:9–97`):
  - `to_bytes() -> Vec<u8>` and `from_bytes(&mut &[u8]) -> Bdd`. `from_bytes` **panics** on I/O
    errors and does not validate.
  - `write_as_bytes`/`read_as_bytes`: fallible, but the code notes that truncation can go undetected.
  - `write_as_string`/`read_as_string`/`from_string` and `impl Display` (`to_string`).
  - `serde` is available only through a feature, which is off.
  - Serialization does **not** need the `BddVariableSet`: `num_vars` is stored in the terminal node
    (`num_vars()` reads `self.0[0].var`, util.rs:18–21).
  - It stores library-local variable **indices, not names**, so interpreting a stored diagram needs
    the matching set or your own atom list.
  - `from_nodes(&[BddNode]) -> Result<Bdd,String>` validates. `BddNode` is 12 B (review-probe).
- **Growing the variable set:** a `BddVariableSet` cannot be extended in place; there is no mutating
  method (`_impl_bdd_variable_set.rs:10–370`). The options are:
  - build a new set (`new`, `new_anonymous` or `BddVariableSetBuilder`) and call `transfer_from` for
    each diagram, which fails if shared variables change order; or
  - call `unsafe set_num_vars` when the new variables are appended at the end of the order.
  - The limit is 65,535 variables (a `u16` index).
- **Traits:** `#[derive(Clone, Debug, Eq, Hash, PartialEq)] pub struct Bdd(Vec<BddNode>)`
  (lib.rs:112–114). It is **not `Ord`/`PartialOrd`** and not `Copy`. `Eq`/`Hash` are structural,
  which is semantic only within **one** variable set (skill-probe B001/B002/B005: a 3-variable and
  a 4-variable `true` differ, and `and` across sets panics).
- **Implication:**
  - `imp(&self, &Bdd) -> Bdd` is unbounded.
  - **`Bdd::cmp_implies(a, b) -> Option<Ordering>`** (`_impl_sort.rs:37–58`) calls
    `binary_op_with_limit(2, a, b, imp)` in both directions, so it stops early unless the
    implication is the constant `true`. It returns `Less`, `Greater`, `Equal`, or `None` when the
    functions are incomparable or `num_vars` differ. This is a ready-made `PartialOrd` for an
    implication lattice (review-probe: `cmp_implies(a∧b, a)=Some(Less)`, `cmp_implies(a, b)=None`).
  - The skill's bounded recipe is `and_not` followed by `is_false` (skill-brief
    `reason.bdd-implication`).

### How the repository uses it

source-read `crates/cpg-schema/src/condition_kernel.rs`:

- **Binary operations:** `apply` first rejects work above `MAX_PAIR_WORK` (a size-product check),
  then calls `binary_op_with_limit(MAX_NODES, ..)` (273–286). This matches the skill's rule.
- **Implication and compatibility:** `implies` and `compatible` use
  `check_binary_op(MAX_PAIR_WORK, .., and_not|and)` (757–787). This is the library's own work-bounded
  dry run, and it is correct.
- **Other operations:**
  - `restrict_atoms` uses `Bdd::restrict` (662–685).
  - Each value keeps its own minimal `BddVariableSet`, with `transfer_from` into a sorted union for
    binary operations (600–638), and `effective` drops dead support (312–340).
  - `cube_quotient` uses `sat_clauses` and `restrict` (712–729).
- **Not used:** `exists` and `substitute`. Persistence uses the repository's own hashed node catalog
  over atom ids (`root_and_nodes`, `id()`, 552–598) rather than `to_bytes`, as the skill recommends.
- `Diagram` derives only `Clone`: it has no `Eq`, `Hash` or `PartialOrd`.

### G8 check on `condition_kernel/substitution.rs`

The substitution is hand-written (`Composer::visit`, Shannon expansion with a memo; ITE built as
`and`, `not`, `or` through `Budget::apply` → `binary_op_with_limit`, with cumulative work and
retained-node budgets). The library provides `Bdd::substitute`, but that call is:

1. single-variable, and its own documentation says it cannot do simultaneous substitution when
   names clash;
2. unbounded;
3. able to panic at `u16::MAX`.

The repository needs simultaneous, capture-safe substitution, where "a replacement mentioning
another source atom is not substituted again" (condition_kernel.rs:684–692), with bounded work, and with refusal never
reported as false. `ternary_op` and `binary_op_with_exists` have no limited variants either.
**Verdict: the hand-written composer is justified. G8 passes: the alternative was considered and is
insufficient, and the module header already says "not unbounded substitute".**

For interprocedural composition in the target (substitute formals by actuals, then ∃ over locals),
add one bounded ∃ helper over `fused_binary_flip_op_with_limit`, applied per variable.

---

## 6. ascent 0.8.1

**The `Lattice` trait** (source-read `$REG/ascent_base-0.8.1/src/lattice.rs:15–45`):

- `pub trait Lattice: PartialOrd + Sized { fn meet_mut(&mut self, other: Self) -> bool; fn join_mut(&mut self, other: Self) -> bool; fn meet(self, o) -> Self; fn join(self, o) -> Self }`
- `pub trait BoundedLattice: Lattice { fn bottom() -> Self; fn top() -> Self; }`
- It needs **`PartialOrd`, not `Ord`**.
- Provided implementations: `Dual<T>`, `Product`, `set::Set`, `bounded_set`,
  `constant_propagation`, tuples, and ord-based types such as `bool` and the integers.

**Column requirements** (source-read `$REG/ascent-0.8.1/src/internal.rs:221–238`; codegen
`$REG/ascent_macro-0.8.1/src/ascent_codegen.rs:92–120`):

- Every column type must be `Clone + Eq + Hash`.
- A lattice (last) column must be `Clone + Eq + Hash + Lattice`.
- Parallel mode adds `Send + Sync`.

**Lattice insertion** (codegen 1215–1250): a row is keyed by every column except the last. The new
value goes through `join_mut` into the existing row, and the returned bool decides whether the
semi-naive loop sees a change. `meet_mut` is not called on insert.

### Can a BDD implication order be a lattice value?

**Yes, through a newtype.**

- `Bdd` already has `Clone + Eq + Hash`.
- Implement `PartialOrd` with `Bdd::cmp_implies`.
- Implement `join_mut` as a bounded `or`. A refusal becomes a top value `Unknown`, never `false`.
- review-probe executed `lattice reach(u32, Cond)` over guarded edges with a 1→3→1 cycle:
  `reach(1) = reach(3) = a | b`, with **one row per key** (4 rows).
- **Condition 1:** every value in a program must live in **one shared `BddVariableSet`**, or
  `Eq`/`Hash` stop meaning "same function" and cross-set operations panic.
- **Condition 2:** the repository's `Diagram` keeps a minimal variable set per value, so wrapping it
  needs `Eq`/`Hash` through its canonical `id()` and `PartialOrd` through the fallible `implies`
  (an error maps to `None`).
- **Termination:** over a finite atom set the lattice is finite, and the `Unknown` top guarantees a
  stop under the node limit.

### Negation, aggregates, macros and budgets

- **Negation and aggregates:** `!rel(..)` and `agg y = f(x) in rel(..)`. The aggregators are `sum`,
  `min`, `max`, `count`, `mean` and custom ones. Both are stratified and checked at compile time
  (skill-probe C001, B020, B021; skill-brief `reason.ascent-rules`). Lattice relations can be read
  through negation or aggregation only from a lower stratum.
- **`ascent!` versus `ascent_run!`:**
  - `ascent!` generates a struct: one `pub Vec` per relation, `run()`, `scc_times_summary()`, and an
    optional `struct Name<..>;` header for generics.
  - `#![generate_run_timeout]` adds `run_timeout(Duration) -> bool` (codegen 169–205). It is only a
    **wall-clock** budget, and it is `ascent!` only.
  - `ascent_run!` evaluates in place with local variables in scope and returns the struct.
  - `ascent_source!` plus `include_source!` share rule text.
  - **There is no deterministic work budget**, which the repository's refusal model requires (per
    the prior probe).
- **Output order:**
  - Relations are insertion-ordered `Vec`s (skill-brief).
  - Indices use FxHash with a fixed seed (internal.rs:9–19), so sequential runs on the same input
    order repeat exactly.
  - The order is not canonical and changes with input order, so sort before publishing.
- **Parallel feature:** `ascent_par!`/`ascent_run_par!` use rayon and dashmap, and their order is
  nondeterministic. **The default feature `par` is on**, so use `default-features = false`.
- **Provenance: none built in.** A search of ascent, ascent_macro and ascent_base 0.8.1 for
  "provenance" finds nothing, and the README feature list (lattices, parallel, BYODS, `ascent_run`,
  negation and aggregation, macros, misc) has none.

### Recording derivations

The idiomatic way is a multi-head rule that writes an extra derivation relation per rule.
review-probe, executed:

```
reach(y, ..), derived_by(1, y, x) <-- reach(x, ..), edge(x, y, ..);
```

This gave the rows `[(1,1,0),(1,1,3),(1,2,0),(1,3,1),(1,3,2)]`. OR-derivations of a head are its
rows; AND-premises are the columns of one row. The costs:

- one tuple per **distinct rule binding**, each hashed into the full index. For a closure this can
  reach O(V·E);
- a lattice head's row does not record which version of the premise value fired it, so the premise
  condition ids must go into the row;
- `derived_by` can be cyclic (1→3→1). A well-founded proof needs a depth or first-derivation stamp;
  one proposed shape is a `Dual<u32>` depth lattice that emits a derivation only where the premise
  depth is smaller than the head depth.

### The prior probe

`docs/design_review/evidence/2026-09-26_summary-engine-comparison/README.md` and `src/main.rs`:
ascent (with `default-features = false`), datafrog and a petgraph SCC-local worklist all derive
`{(0,101),(1,101),(2,101),(5,105)}`. The result does not change when the inputs are reversed, and
with no bases it is empty.

- Ascent and datafrog compute the relation with little code, "but neither result carries the
  product's ordered source proof, BDD conjunction, per-origin refusal or work accounting".
- The native worklist can place those checks at each transfer and stop before publishing a false
  positive.
- Its conclusion: "a **narrow native choice for the first value channel**, with reevaluation when
  effect, exception and role channels share a genuine multi-relation recursive rule". It makes no
  performance claim.

**Integration burden** (review-probe lock diff with `default-features = false`): 7 new crates:
`ascent`, `ascent_base`, `ascent_macro`, `boxcar 0.1.0`, `derive-syn-parse 0.2.0`, `duplicate 2.0.1`
and `pastey 0.2.3`. `hashbrown 0.14`, `rustc-hash 2` and syn are already present. The macro adds
compile cost and gives macro-shaped error messages.

---

## 7. datafrog =2.0.1

datafrog is a zero-dependency crate (its registry `Cargo.toml` has no `[dependencies]`); the last
release was 2019-01-02. You drive it by hand: create an `Iteration::new()`, declare variables with
`iteration.variable::<T: Ord>(name)` (or `variable_indistinct`), and loop
`while iteration.changed() { .. }`. Each `Variable<T>` is filled by `from_join(&Variable<(K,V1)>, input2, |k,v1,v2| ..)`,
which joins on the **first tuple element** (skill-probe C009), or by `from_antijoin`, `from_map`,
or `from_leapjoin(&source, leapers, logic)`. The leapers are `ExtendWith`, `ExtendAnti`,
`FilterWith`, `FilterAnti`, `PrefixFilter` and `ValueFilter` (source-read
`$REG/datafrog-2.0.1/src/lib.rs:39–489`, `treefrog.rs`). A finished `Relation<T: Ord>` is a sorted,
deduplicated `Vec`, so results are deterministic. Evaluation is semi-naive and single-threaded.
There are **no lattices, aggregates, stratification checking or provenance**; negation is only
against a finished `Relation`, so stratification is the caller's job. Every tuple must be `Ord`,
which rules out raw `Bdd` values. It fits relational joins assembled at run time over set-valued
facts, and it does not fit condition-valued transfer.

---

## Capability table

| Capability | Library / API | Fit | Gaps | Integration burden | Recommendation | Evidence |
|---|---|---|---|---|---|---|
| Arc-id multigraph projection, both directions | petgraph `Graph<(), u32, Directed, u32>` | Good: parallel edges kept, `edges_directed` both ways, all 43 algorithms | Newest-first adjacency (sort by arc id); 20 B/edge + 8 B/node | None (in use, `graph.rs:23`) | **keep** | source-read graph_impl/mod.rs:220–251, 919–1012; review-probe `sizes`; skill `containers/Graph.md` |
| Compact read-only adjacency | petgraph `Csr` | Poor for multigraphs | No parallel edges (duplicate is `Err` or `false`), no incoming, 14/43 algorithms blocked | None | **reject** for arc-id projections (hand-roll offsets + arc ids if memory matters) | source-read csr.rs:67,192–397; review-probe; skill-probe M-Csr |
| Filtered or reversed views | `NodeFiltered`, `EdgeFiltered`, `Reversed` | Good; they compose | `NodeFiltered` lacks `NodeCount`/`EdgeCount` (no `page_rank` or lexicographic sort); predicate runs per visit; `condensation` refuses views | None | **adopt** | source-read visit/filter.rs, reversed.rs; review-probe `views`/`views_neg`; skill-brief graph.adaptors |
| SCC decomposition | `kosaraju_scc` (iterative); `TarjanScc` | Good (kosaraju) | Tarjan is **recursive**: stack overflow at a 100k-deep path on 8 MiB; kosaraju blocked on Csr | None | **keep** kosaraju; avoid Tarjan on deep graphs | source-read scc/*.rs; review-probe `deep` |
| Condensation that keeps arc ids | `algo::condensation(g, make_acyclic)` | Partial | `true` keeps only the last weight per SCC pair; concrete `Graph` only, by value | None | **adapt**: use `false`, or build from the SCC index (as the repository does) | source-read algo/mod.rs:477–512; review-probe |
| Deterministic callee-first SCC schedule | rustworkx `lexicographical_topological_sort(.., reverse=true, ..)` | Good; zero-copy; same order as the repository's Kahn loop | Needs `NodeCount` and the directed traits | 7 crates; mandatory rayon features | **optional adopt**; otherwise **keep** the 25-line Kahn loop | source-read dag_algo.rs:146; review-probe `probe2`; skill-brief graph.dag-analysis |
| Topological order and DAG invariant | `toposort`; `acyclic::Acyclic` | OK | `toposort` order not canonical; `Acyclic` order depends on history | None (no feature) | **adopt** `Acyclic` only for derived relations that must stay DAGs | source-read algo/mod.rs:211, acyclic.rs |
| Reachability queries | `has_path_connecting` + `DfsSpace` | Good | O(V+E) per query | None | **adopt** | source-read algo/mod.rs:369 |
| Dominators and post-dominators | `dominators::simple_fast` (+ `Reversed`) | Good | One root; O(V²); hash-ordered iterators | None | **adopt** | source-read dominators.rs; review-probe `views` |
| Arc-labelled, prunable DFS | rustworkx `traversal::depth_first_search` | Good: iterative, arc ids in events, `Prune` | petgraph's version is recursive and has no arc ids | Same as rustworkx | **adopt** with rustworkx, or hand-roll | source-read dfs_visit.rs; petgraph dfsvisit.rs:241–301; review-probe |
| Ranking | petgraph `page_rank` | Poor | Non-standard formula; unweighted; parallel edges counted only in the degree; no convergence; O(V²) | None | **keep** the repository kernel | source-read page_rank.rs:63–103; ranking.rs:264–310; review-probe maxdiff up to 3.2e-2 |
| Typed dense index domains | typed-index-collections `TiVec`/`TiSlice` | Good | No id→index dictionary; no petgraph bridge | 1 crate, zero dependencies | **adopt** (optional) | source-read typed-index-collections-3.5.0 |
| Typed dense index domains | cranelift-entity `PrimaryMap`/`SecondaryMap` | Adequate | Silent truncation in release; exact-pinned wasmtime-internal-core; monthly breaking releases | 3+ crates, MSRV 1.96 | **reject** | source-read 0.121.2/0.136.1; crates.io API |
| Dense set states | fixedbitset 0.5.7 | Good | `Eq`/`Hash` include capacity; no changed flag; `Ord` is not subset order | None (in use) | **keep** | source-read lib.rs:834–1043; review-probe |
| Sparse set states | roaring 0.11.5 `RoaringBitmap` | Good for large sparse domains | No `Hash`/`Ord`: newtype needed as a key or ascent column | 0 new crates (already transitive) | **adopt** where the domain is large and sparse | source-read roaring-0.11.5; Cargo.lock:6784 |
| Condition algebra | biodivine `binary_op_with_limit`, `check_binary_op`, `restrict`, `sat_witness` | Good | One variable set per comparison; variable order fixed at creation | None (in use) | **keep** | source-read boolean_ops.rs; condition_kernel.rs:273–787 |
| Existential projection | `Bdd::exists`/`var_exists` (unbounded); `fused_binary_flip_op_with_limit` (bounded, one variable) | Adequate | No bounded multi-variable ∃ | None | **adapt**: bounded per-variable ∃ helper | source-read relation_ops.rs:19–90, boolean_ops.rs:155 |
| Simultaneous substitution | `Bdd::substitute` (one variable, unbounded) | Insufficient | Not simultaneous; unbounded; panics at u16::MAX | — | **keep** the hand-written bounded composer (G8 pass) | source-read util.rs:549–638; substitution.rs |
| Implication order | `Bdd::cmp_implies`; `check_binary_op(.., and_not)` | Good | Same-set requirement | None | **adopt** `cmp_implies` for `PartialOrd` | source-read _impl_sort.rs:37–58; review-probe |
| BDD persistence | `to_bytes`/`from_bytes`/`Display` | Poor for persistence | Stores local indices, not names; `from_bytes` panics and does not validate | None | **reject**; keep the repository's atom-id node catalog | source-read _impl_serialisation.rs; skill-brief reason.bdd-equality |
| Declarative recursive rules with lattices | ascent 0.8.1 | Good for multi-relation rules, negation and aggregates | No provenance; no deterministic work budget; insertion-ordered output; one shared variable set for BDD lattices | 7 crates; proc macro; set `default-features = false` | **adapt later**: use it when channels share a multi-relation recursive rule (prior probe) | source-read ascent_base lattice.rs, ascent internal.rs, codegen; review-probe; evidence README |
| Derivation (AND/OR) recording | ascent multi-head `derived_by(rule, head, premises)` | Partial | Cost per binding; cyclic derivations; lattice version not captured | Rule discipline | **adopt as a pattern** if ascent is adopted (Proposed) | review-probe |
| Leapjoin Datalog | datafrog 2.0.1 | Fair for pure set relations | No lattices or aggregates; joins on the first element; `Ord` tuples (not `Bdd`) | 1 crate, zero dependencies | **reject** for condition-valued analyses | source-read datafrog lib.rs/treefrog.rs; skill-brief reason.datafrog-joins |

---

## Review probes (committed as P3)

They are committed in this evidence folder as `p3-graph-reasoning/` (formerly `probe/`) and `p3-rustworkx/` (formerly `probe2/`).

- `probe/`: petgraph 0.8.3, biodivine-lib-bdd 0.6.3, ascent 0.8.1 (`default-features = false`) and
  fixedbitset 0.5.7.
  - `src/main.rs`: PageRank comparison with its 3-cycle control; Csr duplicates; condensation
    weight loss; BDD lattice in ascent with `derived_by`; FixedBitSet equality.
  - `src/bin/deep.rs`: recursion depth of Tarjan versus Kosaraju and `toposort`.
  - `src/bin/views.rs` and `views_neg.rs`: which algorithms accept each view, with the negative
    control.
  - `src/bin/sizes.rs`: struct sizes.
- `probe2/`: rustworkx-core 0.18.1 with petgraph 0.8.3. It covers the single-petgraph resolve, the
  lexicographic callee-first schedule, and the iterative DFS with `Prune` and arc ids.
- Run command: `CARGO_TARGET_DIR=<dir>/target cargo run --offline [--release] [--bin X]`.

All passed on 2026-09-29, except the intended compile failure in `views_neg` (E0277 `NodeCount`)
and the intended stack overflow of `deep tarjan` at 100k and 1M nodes. The evidence README records the rerun commands and outputs.
