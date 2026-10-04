# L3: reasoning and graph libraries against the M1 mechanisms

Lane L3 of the [library-leverage review evidence](README.md). It reports capabilities, contracts,
limits and fit against the [M1 inventory](m1-inventory.md) and the settled choices in
[B0](b0-baseline.md). It makes no adoption decision.

- **Baseline:** `main` at `948b2a88`. Repository code is read-only. No product build or test was run
  (`not_run`).
- **Sources:**
  - Pinned crate source in `~/.cargo/registry/src/index.crates.io-*/`: petgraph 0.8.3,
    rustworkx-core 0.18.1, leiden-rs 0.8.1, graphops 0.5.1, graphina 0.4.0-alpha.6,
    biodivine-lib-bdd 0.6.3, oxidd 0.12.0, ascent/ascent_base/ascent_macro 0.8.1, datafrog 2.0.1,
    fcars 0.2.2, odis 2026.9.1, get-size2 0.11.0, allocative 0.3.6, index_vec 0.1.4,
    typed-index-collections 3.5.0, cranelift-entity 0.121.2, la-arena 0.3.1, arrow-ord 59.3.0 and
    simsimd 6.5.16.
  - The `rust-graphs`, `rust-reasoning` and `fixedbitset` skills: their briefs and probe ledgers.
- **Version applicability:** the workspace `Cargo.lock` resolves petgraph 0.8.3 (plus 0.6.5,
  pulled transitively), leiden-rs 0.8.1, biodivine-lib-bdd 0.6.3, fcars 0.2.2, odis 2026.9.1,
  arrow-ord/arrow-row 59.3.0, get-size2 0.11.0, allocative 0.3.6 (patched, `third_party/allocative`,
  via `starlark_map`) and index_vec 0.1.4 (via `pyrefly_util`). It does **not** resolve
  rustworkx-core, graphops, graphina, ascent, datafrog, oxidd, typed-index-collections,
  cranelift-entity, la-arena or simsimd. Statements about those crates describe the skill-indexed
  or registry copy and have no workspace pin.
- **Probes:** one scratch crate, recorded in [§11](#11-probe-record). Labels follow principles §D:
  *Tested* means probed here on the stated fixtures, and *source* means read from the pinned source.

## 1. Capability table

"Keeps" and "loses" refer to the M1 invariants of the consumer named in the M1 column.

| Library · item at pin | Contract and limits | M1 | Fit |
|---|---|---|---|
| biodivine-lib-bdd 0.6.3 · `Bdd::fused_binary_flip_op_with_limit(limit, (f,None), (f,Some(v)), None, or)` (`_impl_boolean_ops.rs:155`) | Computes ∃v.f in one apply over f and its v-flipped copy. `None` when the **result** exceeds `limit` nodes, terminals included. This is exactly what `var_exists` does internally (`_impl_relation_ops.rs:19`), with the limit added. | M1-15 `exists` | Replaces the two `restrict` calls plus the capped `or` per variable (`kernel.rs:723-748`). It removes two intermediate diagrams per variable, which the bespoke loop builds without any cap. Equal to the bespoke form and to `var_exists` on 4,490/4,490 single-variable cases, and to `Bdd::exists` on 100/100 four-variable cases (*Tested*, P1). **Keeps** the node cap and `None` as unknown. Work accounting changes (see the next row). |
| biodivine 0.6.3 · `Bdd::check_fused_binary_flip_op(limit, …)` / `check_binary_op` (`:195`, `:208`) | Dry-run apply that returns `(non_empty, tasks)`, or `None` once more than `limit` low-level tasks are needed. A task count is an upper bound on result size. It builds no diagram. | M1-15 work preflight | A real work bound that could replace the `|a|×|b|` preflight, which B029 shows refuses answers. On P1's fixtures the task counts were 940–3,151 where the pair products were 1e5–7e5 (*Tested*). The kernel already uses `check_binary_op` for `implies`/`compatible` (`kernel.rs:825-845`). **Cost:** the walk runs twice (check, then apply). **Changes** refusal thresholds, and therefore published boundaries, if adopted. |
| biodivine 0.6.3 · `Bdd::substitute`, `exists`, `binary_op_with_exists`, `ternary_op`, `for_all` | **No limit-taking form exists** for any of them (full `pub fn` inventory of `src/_impl_bdd/`; only `binary_op_with_limit` and `fused_binary_flip_op_with_limit` take a limit). `substitute` is single-variable. Its doc says no simultaneous capture-safe substitution exists. | M1-15 `compose` | Bounded single-variable substitution can be written as `iff` → `and` → capped fused-flip exists, all with limits. That form equals `Bdd::substitute` and the cofactor form on 80/80 cases (*Tested*, P1). The **simultaneous** composition in `substitution.rs` (a memoised Shannon walk with a `Budget{work,nodes}` that charges intermediates) has no library counterpart and stays bespoke. |
| OxiDD 0.12.0 (latest 0.13.0, 2026-09-28) · `new_manager(cap, cache, threads)`, `AllocResult` | Capacity is fixed per manager. Running out returns `Err(OutOfMemory)` from the operation, and the manager fails as a whole (skill B010). There is no per-operation node or work limit in 0.12 or 0.13: no `limit`, `budget` or `interrupt` in `oxidd-core-0.13.0/src`. | M1-15 | No change to the 2026-09-25 rejection. Its trigger is pilot evidence that biodivine is too slow or too large. |
| petgraph 0.8.3 · `algo::condensation(g: Graph, make_acyclic)` (`algo/mod.rs:477`) | With `make_acyclic=false` it **keeps every parallel arc**, and arcs inside an SCC become self-loops. With `true` it calls `update_edge`, which merges arcs (the last weight wins) and drops loops. It takes an owned `Graph`, nodes come in `kosaraju_scc` order, and it allocates outside our budget. | M1-04 | ADR-0044 D3 ("condensation merges parallel arcs") holds **only for `make_acyclic=true`** (*Tested*, P3). For an evidence-preserving condensation, the trigger ADR-0044 lists as Proposed, `false` plus self-loop filtering keeps arc multiplicity and weights. |
| petgraph 0.8.3 · `toposort`, `visit::Topo`, `kosaraju_scc`, `tarjan_scc`, `TarjanScc`, `Bfs`, `Dfs`, `DfsPostOrder`, `visit::depth_first_search` (`DfsEvent`, `Control`) | No keyed or lexicographic topological sort. Topo order follows container order. No BFS visitor, only the `Bfs` walker (no parents, no edge events). `depth_first_search` has events plus `Control::Prune/Break`. Neighbour order is the container's (newest edge first for `Graph`; skill D4/B-probes). | M1-04/05/07/09 | Nothing keyed. `toposort` stays as the acyclicity check (`derivation.rs:113`). |
| rustworkx-core 0.18.1 · `dag_algo::lexicographical_topological_sort(dag, key: FnMut(N)->Result<K,E>, reverse, initial)` (`dag_algo.rs:146`) | A Kahn sort with a min-heap on `(key, NodeId)`: the minimum key first, ties broken by the smaller `NodeId`. `reverse=true` gives a callee-first order. It needs `GraphProp<Directed> + IntoNodeIdentifiers + IntoNeighborsDirected + IntoEdgesDirected + NodeCount` and `NodeId: Hash+Ord`. **A cycle does not raise an error: it returns `Ok` with only the acyclic prefix** (`Ok(["a","d"])` for 4 nodes; *Tested*, P2). `CycleOrBadInitialState` is reachable only through `initial`. A self-loop makes its node never ready, so the result is silently truncated. | M1-04, M1-05 | **Order-equivalent to `canonical_order`** on 300/300 random graphs, with `condensation(g, true)` and key = minimum member (*Tested*, P6). With `make_acyclic=false` (self-loops kept), 235/300 results came back `Ok` but truncated. **Loses** cycle refusal unless the caller checks `out.len() == node_count`. Its internal `HashMap` and `BinaryHeap` are not charged. It brings rayon, ndarray, rand 0.10, priority-queue and hashbrown into the production graph (all non-optional in its `Cargo.toml`). |
| rustworkx-core 0.18.1 · `traversal::breadth_first_search(graph, starts, visitor)` with `BfsEvent::{Discover, TreeEdge, NonTreeEdge, Gray/BlackTargetEdge, Finish}` and `Control::{Break, Prune}`; `bfs_predecessors`, `bfs_layers`, `ancestors`, `descendants` (`traversal/bfs_visit.rs:169`, `mod.rs`) | The visitor sees tree edges, so it can build a parent map, depths and caps, and `Break(reason)` can carry a stop value. Neighbour order is `graph.edges(u)`, which is container order. It is uncharged. `bfs_layers` and `ancestors` have no cap. | M1-09 (and M1-07 capped reachability) | Could host the delegation BFS loop, but canonical sibling order `(target, site, arc)` needs either a graph built so that `edges(u)` already yields that order or an adapter that implements `IntoEdges` over sorted adjacency. Arc, vertex and depth caps, boundaries, witness selection and the `Stop` reasons stay in the visitor. **Net:** it moves ~25 lines of queue mechanics, not the logic. ADR-0044 §Traversal ("`visit::Bfs` not used") remains accurate for petgraph. |
| leiden-rs 0.8.1 · `infomap::compute_flow(&GraphData, teleport, tol, max_iter) -> Vec<FlowData>` (`infomap.rs:191`) | Builder: `f64` weights, finite and ≥ 0 (zero allowed). `build_directed_csr` sorts and **sums parallel arcs**. Dangling mass is spread uniformly, at O(n) per dangling node per iteration; teleport is uniform. Stops at L∞ < tol. **Returns no iteration count, residual or converged flag**, and exhausting `max_iter` is silent. It allocates a fresh vector each iteration and takes no work bound. | M1-10 | Same fixed point as ours. The existing oracle agrees to 1e-9 (`crates/lctx-analytics/tests/native_ranking.rs:8-55`), and graphops agrees with it to 4.5e-15 (*Tested*, P5). **Loses** `Stop::{Empty, IterationLimit, WorkLimit}`, the residual, work accounting and zero-weight refusal. A good oracle; it does not fit production. |
| graphops 0.5.1 · `pagerank_weighted_run(&G: WeightedGraph, PageRankConfig) -> PageRankRun{scores, iterations, diff_l1, converged}` (`pagerank.rs:252`) | L1 residual, an iteration count and a converged flag. Dangling and teleport mass are uniform, accumulated per source in index order. `WeightedGraph` is `neighbors(u) -> Vec<usize>` plus `edge_weight(u,v) -> f64`. **A parallel arc listed twice in `neighbors` is double-counted**: an error of 2.25e-2 against the oracle (*Tested*, P5). Negative weights are clamped (the `_checked` variants reject them). Empty input returns `converged=true, iterations=0`. Default features carry **no petgraph** (the `petgraph` feature is the optional 0.6 one); default dependencies are rand 0.9, rand_chacha, rand_distr, ordered-float and thiserror. The `simd` feature swaps the residual kernel. | M1-10 | Covers the convergence report that petgraph `page_rank` lacks. B0's "graphops pulls petgraph 0.6" applies only to an optional feature. **Keeps:** aggregation stays ours (one `neighbors` entry per distinct target), and a work bound maps exactly onto `max_iterations = min(max_iter, max_work / (n+m))` because per-iteration cost is constant. **Loses:** an empty result needs a caller-side `Empty`, allocation is uncharged (per-node `Vec`s), and summation order differs from ours (bitwise results change, not the fixed point). It would replace ~60 of ~140 lines. |
| graphina 0.4.0-alpha.6 · `centrality::pagerank(graph, damping, max_iter, tol, nstart)` | Weighted (`W: Into<f64>`), stops when L1 < `tol·n` (the NetworkX rule), returns only a `NodeMap`, and does not report convergence. Its own `BaseGraph`, which depends on petgraph 0.8.3. | M1-10 | Weaker than graphops. Not a candidate. |
| leiden-rs 0.8.1 · `metrics::{try_ari, try_nmi, try_vi, try_fmi, try_ami, …}`, `resolution::{resolution_scan, resolution_profile}` | Pairwise partition metrics. The contingency table is private (`fn build_contingency_table`, `metrics.rs:241`). `resolution_scan` runs **one** seed per γ (`seed + i`) and parallelises under `rayon` (off here: `default-features = false`). No multi-seed consensus, co-membership, stability aggregate or normalisation. | M1-12 | Already used for ARI/NMI. Mean/sd, degeneracy, percentile normalisation and the per-community agreement loop have no library counterpart. The agreement loop is O(k²) per community; a contingency count would make it O(n), but leiden-rs does not expose one. |
| ascent 0.8.1 · `ascent!` / `ascent_run!`, `lattice` relations, `ascent_base::Lattice`, `BoundedSet<N,T>`, `#![generate_run_timeout]` | Semi-naive, stratified at compile time. A user `Lattice` type works as a per-key column: a Pareto antichain under `(depth, steps)` dominance converged on a cyclic graph, and a fallible side computation can be written as a guard plus a `refused(..)` relation (*Tested*, P4). The only stop hook is **wall-clock** `run_timeout(Duration) -> bool`, checked after each SCC iteration, and it exists only for `ascent!` (not `ascent_run!`). Stopping leaves the derived state but **no public pending/delta set**. Hashing is FxHasher (deterministic); relation order is insertion order. The default `par` feature pulls dashmap and rayon. | M1-01–M1-03, M1-08 | Can express the generic recursion of M1-03 and the dominance merge of M1-01. **Loses:** charging and refusal before mutation, a work bound (only time), a canonical queue order under refusal (cut-off results would depend on engine-internal order), residuals from pending items, and `Refused(limit)` on frontier overflow (`BoundedSet` goes to TOP with no reason). |
| datafrog 2.0.1 · `Iteration`, `Variable{pub stable, pub recent}`, `from_join`, `from_leapjoin`, `from_antijoin` | The caller drives `while it.changed()`, so per-round accounting is possible and `recent` (the next round's input) is readable at a stop. Sorted `Vec` relations are deterministic. Set semantics only: no lattice, no removal from `stable`. Joins key on the first tuple element (C009). | M1-02/03, M1-08 | Per-round charging and residuals from `recent` are expressible. Dominance (M1-01) is not: dominated rows cannot leave `stable`. Per-derivation refusal needs closure side channels. |
| odis 2026.9.1 (dev oracle) · `index_next_closure_concepts` (lazy `Iterator`), `FormalContext::index_next_preclosure(implications, input)` (public **single step**), `index_canonical_basis[_optimised]` (batch `Vec`), `Titanic` (iceberg, batch, `u16` attribute indices) | Lazy NextClosure concepts with no support threshold. One public preclosure step that a caller can drive and charge per step, but whose internal `bit_set::BitSet` allocations are uncharged. The basis and iceberg functions are batch. Its implication closure retires fired implications: a LinClosure-like pass, not LinClosure. `SearchBudget` is milliseconds and applies to drawing only. Its production dependencies include `rust-sugiyama`. | M1-11 | See §4. ADR-0121's "batch contracts" is accurate for the basis and Titanic. It **understates** the lazy concept iterator and the public step function, but neither combines iceberg pruning with the interleaved pseudo-intent enumeration our kernel performs. |
| fcars 0.2.2 (dev oracle) · `FormalContext::new(objs, attrs, Vec<BitVec>)`, `all_concepts_raw`, `num_concepts` | Parallel close-by-one through rayon. Concepts only: no implications, no support, no order guarantee (skill B024). | M1-11 | Oracle only. No change. |
| get-size2 0.11.0 · `trait GetSize`, `#[derive(GetSize)]`, `get_heap_size()` | `Vec`: capacity × stack size plus element heap. `BTreeMap/BTreeSet`: per-entry `get_size` with **no node overhead**. Unchecked `+`. Already compiled through ruff (features include `derive`); pure, no I/O. | M1-16 | A derive would remove most of the 48 hand-written `impl HeapSize for` items (22 are empty inline-only impls) but **changes charged sizes** (our BTree allowance is +16 per entry, with saturating arithmetic), and so changes admission thresholds. Domain newtypes (`Id<T>`, `ContentHash`, `Utf8Text`, `FiniteF64`) still need impls. |
| allocative 0.3.6 (patched, Pyrefly) | A visitor-based memory profiler for flame graphs. Heavy dependency set (anyhow, bumpalo, ctor, dashmap, futures, …). | M1-16 | Measurement, not admission. No fit. `deepsize` is absent from the registry and was not evaluated. |
| typed-index-collections 3.5.0 · `TiVec<K,V>`, `TiSlice::binary_search -> Result<K,K>`, `get -> Option` | Typed dense indices with no required dependencies. A sorted `TiVec` plus `binary_search` is the M1-14 pattern with typed keys. Out-of-range access returns `None`/panics; there is no foreign-instance refusal at run time. | M1-14 | A typed form of the existing three-line pattern. It prevents object/attribute index mixing (two domains in `concepts.rs`). index_vec 0.1.4 (`IndexVec`, `define_index_type!`, typed `binary_search`) is **already compiled** through `pyrefly_util`. cranelift-entity 0.121.2 (`PrimaryMap`, `SecondaryMap`) and la-arena 0.3.1 (`Arena`, `Idx<T>`) are arena-style and not resolved here. |
| arrow-ord 59.3.0 · `sort::lexsort_to_indices(&[SortColumn], limit)`, `partition::partition`, `rank::rank` | `lexsort_to_indices` uses `sort_unstable_by`, so ties are not kept in input order. Its doc recommends arrow-row for multi-column sorts without a limit. Byte-wise UTF-8 order equals `COLLATE "C"`. arrow-ord is resolved in the workspace but is not an `lctx-model` dependency. | M1-17 | No gain over the current arrow-row plus stable sort (§8). |
| simsimd 6.5.16 · ndarray 0.16/0.17 | simsimd selects SIMD kernels at run time per CPU, so f64 sums are reassociated and can differ across machines. ndarray `dot` uses matrixmultiply/BLAS. Neither is resolved in the workspace. | M1-13 | Not worth it (§9). |

## 2. Graph ordering and traversal (M1-04, M1-05, M1-07, M1-09)

- **Keyed topological sort.**
  - petgraph 0.8.3 has none; `toposort` and `Topo` follow container order.
  - rustworkx-core 0.18.1's `lexicographical_topological_sort` is a keyed Kahn sort with the
    tie-break `(key, NodeId)`.
  - **M1-04:** on `condensation(g, true)` with key = minimum member and `reverse = true`, it
    reproduces `canonical_order` exactly (300/300, P6). Minimum members are unique across a
    partition, so the `NodeId` tie-break never applies.
  - **M1-05:** the key would be the stage name, which is unique. The current 15-line O(n²) loop
    needs no graph.
- **Contract hazard (CI-04).** A cycle or self-loop does not raise an error: the call returns
  `Ok` with a truncated prefix (P2, P6). Any adopter must compare the output length with the node
  count. petgraph's `toposort` does return `Err(Cycle)` (P2).
- **W13 re-examined.** W13 kept bespoke keyed ordering because rustworkx-core lacked PageRank and
  communities and seeded its components randomly. Against the actual items:
  - The ordering function exists and fits.
  - Adopting it would add rustworkx-core's non-optional rayon, ndarray, rand 0.10 and
    priority-queue to `lctx-model`'s production graph to replace ~30 lines per site.
  - The library would add no charging and no validated-partition refusal (`summary_schedule.rs:60`).
  - The decision rests on footprint and on the truncation hazard, **not** on a missing capability.
- **ADR-0044 D3** ("`condensation` merges parallel arcs") is true only for `make_acyclic=true`.
  With `false`, every arc survives with its weight and intra-SCC arcs become self-loops (P3). An
  evidence-preserving condensation, still Proposed in ADR-0044, could therefore use petgraph
  directly. Its node order is kosaraju order, so canonical renumbering stays ours.
- **Bounded BFS (M1-09).**
  - petgraph offers only the `Bfs` walker.
  - rustworkx-core's `breadth_first_search` visitor exposes tree-edge events, `Break` with a
    payload and `Prune`. A parent map, depth and arc/vertex caps and a stop reason fit in the
    visitor.
  - Canonical neighbour order is the obstacle: `edges(u)` is container order (newest first for
    petgraph `Graph`). It needs a pre-sorted adjacency, either by inserting arcs in reverse
    canonical order or through an `IntoEdges` adapter. ADR-0044 D4's facts still hold.
  - Boundaries, witness selection and finals stay bespoke.
- **Capped reachability (M1-07).** No pinned graph crate takes a node cap for `ancestors`,
  `descendants`, `bfs_layers` or `has_path_connecting`; the bounded form remains a visitor with
  `Break`.
  - **`local_semantics.rs:800-830`:** that walk asks whether an atom is mentioned, which is
    `Diagram::support()` (`kernel.rs:381`) or biodivine `support_set_contains`. This is reuse of
    an existing kernel, not a library question.
  - **`conditions/kernel.rs:840` `closure`:** this validates persisted node rows (order, atoms,
    caps) before `Bdd::from_nodes`, so the library cannot replace it.

## 3. Ranking (M1-10)

| Property | ours (`ranking.rs:73-213`) | leiden-rs `compute_flow` | graphops `pagerank_weighted_run` | petgraph `page_rank` |
|---|---|---|---|---|
| Weights | `u64`, checked sum; zero refuses | `f64` ≥ 0, zero allowed | `f64`, clamped at 0 (`_checked` rejects) | none (count of arcs) |
| Parallel arcs | summed (checked) | summed by builder | caller must deduplicate `neighbors`, else double-counted (P5) | each arc counted, membership test by `any` |
| Dangling | uniform, one sum per iteration | uniform, O(n) per dangling node | uniform, one sum per iteration | uniform |
| Teleport | uniform | uniform | uniform | uniform |
| Residual | L1 | L∞ (not returned) | L1 (returned) | none |
| Report | `Stop::{Converged, Empty, IterationLimit, WorkLimit}`, iterations, residual, work | none | `iterations`, `diff_l1`, `converged` | none |
| Work/memory | preflighted reservation, work per iteration | none | none (iterations only) | O(it·V²·E) per docs |
| Determinism | sorted arcs, fixed order | CSR order | index order | index order |

- The fixed points agree: ours with leiden-rs to 1e-9 (existing test), and graphops with leiden-rs
  to 4.5e-15 (P5).
- No crate reports a work bound. graphops is the only one with our convergence report, and its
  constant per-iteration cost makes `WorkLimit` derivable from `max_iterations`.
- rustworkx-core 0.18.1 has no PageRank, only eigenvector and Katz centrality
  (`centrality.rs:742,835`). graphina's PageRank is weaker (no report; NetworkX `tol·n` rule).
- ADR-0044/D1 never evaluated leiden-rs `infomap`. The comparison above shows it to be an oracle,
  not a production fit.

## 4. FCA (M1-11)

- **fcars 0.2.2:** concepts only. No iceberg, implications or order; uses rayon.
- **odis 2026.9.1:**
  - lazy `index_next_closure_concepts`, a full lattice with no support threshold;
  - `index_canonical_basis[_optimised]`, batch `Vec`;
  - `Titanic`, iceberg concepts, batch, with `u16` attribute indices and a `HashMap` support store;
  - a **public single-step** `index_next_preclosure(context, implications, input)`;
  - an implication closure that retires fired implications. It is close to, but not, LinClosure,
    and has no per-attribute counters.
- **No crate found enumerates iceberg intents and frequent pseudo-intents together** under one
  per-step budget, which is what `concepts.rs:131-226` does: one lectic NextClosure over
  L•-closure with `min_support` pruning.
- With odis, a caller could drive preclosure steps and stop on a budget, and a prefix of a lectic
  enumeration stays exact. But:
  - the inner allocations (`bit_set::BitSet`) are uncharged;
  - support pruning is not built in;
  - conversion from `fixedbitset` is needed at the boundary;
  - its production dependency set adds `rust-sugiyama`.
- **ADR-0121's production non-fit therefore holds,** with one correction. "Batch contracts" is
  exact for the basis and Titanic. The concept enumerator is lazy, and a step function exists.
- **Remaining uncharged parts:** our `implication_closure` (`concepts.rs:228-247`) and the
  candidate scan are charged only through `max_examined`, as M1 observed.
- **Charged or incremental FCA crate:** none found (§10).

## 5. Communities (M1-12)

- **What leiden-rs supplies:** the algorithm, seeding (`LeidenConfig::seed`), quality history and
  pairwise metrics.
- **What it lacks:**
  - Statistics across seeds (mean and standard deviation of ARI) and co-membership consensus or
    agreement: `resolution_scan` is one seed per γ, and no consensus API exists.
  - Hub or percentile normalisation.
  - A public contingency table. That rules out an O(n) per-community agreement count; the
    bespoke loop is O(Σk²).
- **Caveats, already respected:**
  - `default-features = false` avoids rayon (`Cargo.toml:42`).
  - The builder sums duplicate `(u,v)` keys and validates finite, non-negative weights. It does
    not normalise orientation for undirected input, so ADR-0044 D2's normal form stays ours.
  - `from_petgraph` needs `E: Into<f64>`, reads the arc weight and takes direction from `Ty`.
    D2 and the skill's warning stand.

## 6. BDD kernel (M1-15)

- **Limit-taking operations at the pin.**
  - `binary_op_with_limit` and `fused_binary_flip_op_with_limit` (result-node limit);
    `check_binary_op` and `check_fused_binary_flip_op` (task limit, no result).
  - **No** `*_with_limit` form exists for `exists`, `for_all`, `binary_op_with_exists/for_all`,
    `substitute`, `ternary_op` or `fused_ternary_flip_op`. This is the full `pub fn` inventory of
    `src/_impl_bdd/*.rs`.
- **Bounded `exists`:** `fused_binary_flip_op_with_limit(.., or)` per variable. It is
  semantically identical (P1), avoids two unbounded `restrict` intermediates per variable and
  makes one apply instead of three.
- **Work bound:** pair it with `check_fused_binary_flip_op(work_limit, ..)` to get a work bound
  that does not over-refuse the way the `|low|×|high|` product does (B029). Either change alters
  which conditions refuse, so it is a behaviour change to qualify, not a refactor.
- **`compose`:**
  - Single-variable substitution has a bounded library route: `iff` → `and` → fused-flip exists
    (P1, equal to `Bdd::substitute`).
  - The simultaneous, capture-free composition in `substitution.rs` has none: biodivine's doc
    says no simultaneous-substitution method exists. The memoised Shannon walk with charged
    intermediates remains bespoke.
  - Its three `apply` calls per node could be one `ternary_op` (ITE), but `ternary_op` has no
    limit, so the current form is the bounded one.
- **OxiDD 0.12/0.13:** manager-level capacity only (B010). 0.13 refactors edge ownership
  (`Own`/`Ref`) and moves `restrict` into `BooleanFunction`; it adds no per-operation limit. The
  rejection and its trigger stand.

## 7. Fixpoints (M1-01–M1-03, M1-08)

- **Ascent 0.8.1.**
  - **Expressible** (P4 shows each):
    - SCC-local recursive composition as rules;
    - the Pareto frontier as a user `Lattice` column whose join keeps the minimal elements;
    - external BDD conjunction as a rule guard;
    - refusal as derived data (`refused(..)`).
  - **Not expressible:**
    - admission charging before mutation;
    - a deterministic work bound (only wall-clock `run_timeout`, and only on `ascent!`);
    - frontier-cap refusal with a reason (`BoundedSet` saturates to TOP);
    - residuals from pending tuples (deltas are internal);
    - a canonical order of work under a cap. A cap implemented through a side-effect counter
      would make the cut-off set depend on ascent's internal evaluation order.
  - **Proof identity** per derivation would have to live in the tuple, which turns the lattice
    key into (key, witness) and weakens dominance pruning.
  - **Net:** Ascent could own the M1-03 generic recursion. M1-01's refusal and charging, M1-02
    entirely, and M1-03's residual/boundary publication would stay bespoke or be rebuilt around
    it.
- **datafrog 2.0.1.**
  - The caller owns the round loop, so per-round charging and residuals from the public `recent`
    are possible.
  - Dominance cannot be expressed (a monotone set; `stable` cannot shrink).
  - It fits M1-08's round-robin (a semi-naive replacement with explicit non-convergence when the
    round cap ends while `changed()` is still true) better than M1-03.
- **Generic bounded worklist or antichain crate:** none found. The crates.io searches "pareto
  frontier", "antichain", "worklist", "fixpoint worklist" and "dataflow worklist" (2026-10-04)
  returned `pare` (skyline queries, no admission or refusal) and `antichain` (distributed-progress
  frontiers). Neither fits.
- **W12 and ADR-0053.**
  - P4 adds to the W12 comparison evidence; the 2026-09-26 same-state probe ([`2026-09-26_summary-engine-comparison`](../2026-09-26_summary-engine-comparison/README.md)) came first (coordinator correction). It shows the frontier lattice and
    a fallible guard working in Ascent, and it shows that the time-only stop hook and hidden
    pending state cannot meet the "refusal publishes pending tuples" contract.
  - It is not the full W12 spike: there is no shared all-channel contract, no S4 workload and no
    measurement.
  - ADR-0053's revisit trigger (two or more channels sharing a recursive rule, or SCC composition
    dominating the pilot) is untouched by this lane. M1 found no second recursive rule family.

## 8. In-memory validator (M1-17), brief

- **Allowed:** `lctx-model` may depend on pure arrow-* crates (it already uses arrow-row and
  arrow-select). arrow-ord 59.3.0 is resolved in the workspace.
- **`lexsort_to_indices`:** unstable for ties (`sort_unstable_by`). The current stable sort over
  `RowConverter` rows already gives `COLLATE "C"` byte order with stable ties, as arrow's own
  documentation recommends for multi-column sorts.
- **`partition::partition`:** finds runs of equal rows in sorted input. It could replace the
  equality scan in `unique`, but conflict detection still needs our comparison.
- **Distinct and anti-join:** pure arrow crates have neither. The `Rows`-keyed hash membership in
  `Keys::references` is the standard construction. Lane L1 covers the DataFusion side.

## 9. Exact kNN (M1-13)

- **simsimd:** run-time SIMD kernel dispatch reassociates f64 sums, so scores can differ bitwise
  across CPUs and break canonical ties.
- **ndarray:** its `dot` adds a dependency and reorders sums through matrixmultiply.
- **Max over windows:** neither crate offers max-over-windows or preflight.
- **Conclusion:** at bounded window counts, the exhaustive loop with work preflight is the exact,
  deterministic form. No pure-Rust exact max-over-windows crate was found. The search covered the
  skills, the local registry and general knowledge; crates.io was not searched for this item.

## 10. Upgrade deltas (findings only; crates.io checked 2026-10-04)

| Crate | Pin | Latest | Delta for these consumers |
|---|---|---|---|
| petgraph | 0.8.3 | 0.8.3 | none |
| rustworkx-core | (skill 0.18.1) | 0.18.1 | none |
| leiden-rs | 0.8.1 | 0.8.1 | none |
| biodivine-lib-bdd | 0.6.3 | 0.6.3 | none; still no limit forms of quantification or substitution |
| ascent | (skill 0.8.1) | 0.8.1 | none |
| datafrog | (skill 2.0.1) | 2.0.1 (2019) | none |
| OxiDD | (skill 0.12.0) | **0.13.0** (2026-09-28, MSRV 1.91) | edge ownership API refactor (`Own<E>`/`Ref<E>`), `restrict(&self, vars)` documented on `BooleanFunction`; no per-operation limit or budget; the skill's 0.12 snippets need re-checking |
| graphops | — | 0.5.1 | — |
| fcars / odis | 0.2.2 / 2026.9.1 | same | none |
| get-size2 / typed-index-collections | 0.11.0 / — | 0.11.0 / 3.5.0 (2026-09-28) | — |

## 11. Probe record

- **Location:** the coordinator copied the probe from session scratch into [`l3-probe/`](l3-probe/)
  (`Cargo.toml`, `Cargo.lock`, `src/main.rs`; the recorded output is `output.txt`). Rerun it with
  `CARGO_TARGET_DIR=build/evidence-target cargo run --offline --release --manifest-path docs/design_review/evidence/2026-10-04_library-leverage/l3-probe/Cargo.toml`.
  It does not join the product workspace.
- **Earlier evidence:** this adds to the W12 same-state probe
  [`2026-09-26_summary-engine-comparison`](../2026-09-26_summary-engine-comparison/README.md),
  which these probes do not replace.
- **Command:** `cargo run --offline --release`.
- **Toolchain and versions:** `rustc 1.101.0-nightly (c1070d693 2026-09-28)`. The scratch lock
  resolved biodivine-lib-bdd 0.6.3, petgraph 0.8.3, rustworkx-core 0.18.1, ascent 0.8.1
  (`default-features = false`), graphops 0.5.1 (default features) and leiden-rs 0.8.1
  (`default-features = false`).
- **Outcome:** `passed` (built and ran). Each line is an observation, not an assertion.

| Probe | Observation (verbatim counts) |
|---|---|
| P1 biodivine | `fused_flip_with_limit == restrict+or == var_exists: 4490/4490`. At limit = result size − 1, refusal agreement was 4490/4490. `check_fused_binary_flip_op` tasks ≤ \|low\|·\|high\| held in 4490/4490; samples were (tasks, product) = (940, 194940), (1149, 688779), (1086, 637260). Samples vary between runs because `support_set` is a `HashSet`; the counts are stable. A task limit of tasks − 1 gave `None` and a limit of tasks gave `Some`. Four-variable exists: 100/100. Substitution in three forms: 80/80. |
| P2 rustworkx lex topo | Name keys gave `["a","b","d","c","e"]`. Equal keys gave NodeIndex order. On a cyclic graph it returned `Ok(["a","d"])` for `node_count 4`; petgraph `toposort` returned `Err`. |
| P3 petgraph condensation | `make_acyclic=false` kept arcs with weights 1, 2 (self-loops) and 3, 4, 5. `true` kept `[5]`. |
| P4 ascent | `finished=true`. `best`: node 1 `[(1,10),(2,2)]`, node 3 `[(2,6)]`. `refused: [(3,1)]`. `run_timeout(ZERO)` returned `finished=false` with partial `best`. |
| P5 graphops | Deduplicated neighbours: 102 iterations, `diff_l1` 7.8e-15, converged, max \|Δ\| against `compute_flow` 4.5e-15. Duplicated neighbours: 2.25e-2. `max_iterations=3`: `converged=false`. Empty input: `converged=true, iterations=0`. |
| P6 schedule | Identical component order in 300/300 cases. With `make_acyclic=false`, 235/300 results were `Ok` but truncated. |

## 12. Uncertainties and absences

- **Search coverage:**
  - full `pub fn` inventories of biodivine `src/_impl_bdd/`, the leiden-rs `src/`, odis
    `src/algorithms` and `src/traits`, and the rustworkx-core `dag_algo` and `traversal` modules;
  - petgraph `algo/mod.rs` exports plus `visit/traversal.rs`;
  - ascent_macro codegen for stop hooks;
  - datafrog `lib.rs`;
  - OxiDD 0.13 `oxidd-core`, `oxidd-rules-bdd` and `oxidd` diffs against 0.12;
  - crates.io keyword searches for antichain and worklist crates;
  - not searched: Context7, and crates.io for exact-kNN or charged-FCA crates beyond the skills and
    the registry.
- **Absence claims are bounded by that coverage.** They are: no limit-taking biodivine
  quantification or substitution, no charged or incremental FCA crate, no generic bounded
  antichain worklist, and no pure-Rust exact max-over-windows crate.
- **Ascent `ascent_par!` and BYODS custom relations** were not examined. A BYODS relation might
  expose pending state; this is unverified.
- **Not measured:** no timing or memory. In particular, the cost of running
  `check_fused_binary_flip_op` and then the apply was not measured against the current preflight.
- **graphops summation-order differences** were not quantified beyond the 4.5e-15 agreement on one
  six-vertex fixture.
- **Count discrepancy:** M1-16's "62 `HeapSize` impls" did not reproduce. `grep 'impl HeapSize
  for'` finds 48 in `lctx-model/src` plus macro-generated ones; the difference is in counting
  method only.
- **Contact header:** the crates.io queries for latest versions sent a User-Agent containing the
  operator's e-mail address in one batch. Later queries used an anonymous agent.
