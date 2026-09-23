---
id: ADR-0011
title: petgraph traversal and SCCs, our own weighted PageRank, leiden-rs communities, own FCA then RCA
status: proposed
date: 2026-09-22
supersedes: []
superseded-by: null
design: [§B4, §5, §9]
evidence: Interface-checked
revisit: leiden-rs gives different partitions for identical normalized input and seed, fails the LFR planted-partition fixtures, or a technique's ablation (DESIGN §9.8) changes no published output after increment 3; or our PageRank's hand-computed fixture and a reference implementation disagree beyond tolerance.
---

## Context

ADR-0005 makes community detection, centrality, concept analysis and embeddings part of v1
synthesis. §B4 as seeded limited petgraph to SCCs, reachability and dominators.

Verified 2026-09-22:
- **petgraph 0.8.3 (skill)** has 46 algorithms, including `page_rank`, and **no community
  detection**.
  - `Graph` keeps parallel edges.
  - `edges_directed` lists the newest edge first.
  - `Bfs` and `Dfs` visit sibling nodes in opposite orders.
  - Removing a node silently re-points held indices.
- **rustworkx-core 0.18.1** has centrality and connectivity, but no community detection.
- **leiden-rs 0.8.1** (MIT/Apache-2.0):
  - a Leiden implementation with modularity, CPM, RBConfiguration and RBER quality functions;
  - multiplex runs;
  - a petgraph 0.8 adapter behind a feature flag;
  - seeded RNG, with upstream determinism tests;
  - rayon is optional, so `default-features = false` runs sequentially.

  Its upstream is on gitcode.com, and it is young.
- **No maintained Rust crate** for formal or relational concept analysis.

**The library-leverage review** (`docs/design_review/reviews/design_review_library-leverage_2026-09-23.md`,
D1–D5, probes run 2026-09-23) changed four parts before acceptance:
- **petgraph `page_rank` cannot serve §9.5** (Tested by probe):
  - It takes no edge weights, so the usage counts cannot enter.
  - It counts parallel edges in the out-degree but credits the target once
    (`page_rank.rs:92`). Two parallel 0→1 arcs gave [0.542, 0.229, 0.229], against
    [0.486, 0.326, 0.188] for PageRank counting each arc.
  - It differs from textbook PageRank by 0.0053 even on a simple 3-node graph.
  - It is O(iterations·V·E): 254 ms against 125 µs for a sparse power iteration at V=2000,
    E=6000.
  - It reports no convergence; `parallel_page_rank` returns the last iterate with no flag.
- **leiden-rs needs normalized input** (Tested by probe):
  - Its undirected builder does not normalize orientation: (0,1)+(1,0) weighs 2, like a
    duplicate edge.
  - On an LFR graph at μ=0.5, shuffled and flipped edges with the same seed changed the
    partition. After a (min,max) + sort normal form, every perturbation gave identical output.
  - Across 10 seeds, pairwise NMI was ≥ 0.9785.
  - `from_petgraph` needs `E: Into<f64>`, which would force a second graph copy.
  - There is no converged flag, and `seed: None` draws OS entropy.
- **`algo::condensation` merges parallel edges** (`update_edge`, `algo/mod.rs:477-512`): three
  call-site arcs became one edge carrying the last call site. The evidence guideline §6 needs is
  lost.
- **Concept-analysis crates** (Interface-checked):
  - odis has NextClosure, FCbO and the implication basis, but is AGPL-3.0;
  - fcars (MIT, PCbO) enumerates concepts only, with no support threshold or implications;
  - dci is unmaintained.

## Options

1. **Our own Leiden.** Rejected for now: roughly 500 lines of subtle optimisation code, while a
   seeded, tested implementation exists. It is the fallback if the revisit trigger fires.
2. **The simpler alternative: no community detection;** seeds come from PageRank and public
   exports only. Rejected, because the operator wants analytic grouping. It remains the baseline
   that the §9.8 ablation compares against.
3. **PageRank from graphops 0.5.1,** which reports iterations and convergence. Rejected for now:
   its `petgraph` feature pulls petgraph 0.6.5 (`just deps` fails), and its trait adapter would be
   ours anyway.
4. **Chosen:**
   - petgraph for traversal and SCCs;
   - our own weighted PageRank;
   - leiden-rs 0.8.1 fed a normalized, sorted edge list;
   - our own NextClosure FCA, with a one-step relational-scaling RCA added in increment 3.

## Decision

- **Traversal** (DESIGN §5, §9.1): explicit BFS with parent pointers over an immutable
  `Graph<(), u32, Directed, u32>` whose edge weight is the arc's row index. Construction follows
  §5's adapter recipe:
  - the sorted domain ids are the dense index;
  - edges are added in canonical arc order;
  - filtered and reversed views are used, never copies;
  - nothing is ever removed;
  - SCC members are sorted.
- **Condensation** (when §13 un-defers it): built from `kosaraju_scc` membership (iterative;
  `tarjan_scc` recurses), keeping every arc's evidence; never petgraph's `condensation`.
- **Communities** (§9.4):
  - leiden-rs `=0.8.1`, `default-features = false` and **no features** (not `petgraph`), with
    the **RBER** quality function: CPM with γ taken relative to the graph's density, so γ is
    scale-free on unit-normalized layers (amended 2026-09-23, below);
  - built with `GraphDataBuilder` from the dense index: DataFusion aggregates each (min,max) pair
    as **integer counts** under a named weight policy, then sorts; normalization, hub
    down-weighting and layer weighting happen in Rust in canonical order;
  - γ comes from our own fixed-seed grid, pre-registered in the analytics config;
  - never `run_multiplex` (it ignores `layer_weights` after the first level) and never
    `from_petgraph` (it would read the §5 graph's `u32` arc-row weight as the edge weight);
  - the seed is always set and recorded;
  - `track_quality_history` stands in for the missing converged flag;
  - `rand` is pinned, and crate versions go in the method parameters;
  - layers are normalized and hubs down-weighted;
  - seed-consensus stability.
  - Communities select seeds within the brief budget and fill the brief's **Related** field.
    **They do not define brief boundaries or FCA scopes.**
- **Centrality** (§9.5): **our own weighted power iteration**, about 40 lines, over the usage
  projection in canonical order.
  - It uses a named weight policy (the usage counts from examples and tests) and
    dangling-mass redistribution.
  - It records iterations, the final L1 residual and a converged flag (guidelines §8).
  - Its reference oracle is `leiden_rs::compute_flow` (a weighted directed PageRank with uniform
    teleport and dangling mass, already a dependency), valid while the dangling target is
    uniform.
- **FCA / RCA** (§9.6): our own NextClosure (Ganter, ICFCA 2010), which also yields the
  Duquenne–Guigues implication basis, over `fixedbitset`, with a support threshold. There is no
  stability index: it is #P-hard.
  - FCbO (Outrata & Vychodil 2012) replaces it only if the concept count exceeds the budget.
  - `fcars =0.2.2` is a **dev-dependency oracle** for concept sets, and Python `concepts` 0.9.2
    for the cover relation.
  - It runs only over structurally defined scopes (the subsystem, a module, a class hierarchy),
    so its concepts and implications are `structurally_observed`.
  - The one-step ∃-scaling RCA (a DataFusion join adding attribute columns, feeding the same FCA)
    is added in increment 3, under the §9.8 keep rule.
- **Deferred with triggers:**
  - rustworkx-core 0.18.1 (no PageRank or communities; `connected_components` returns a randomly
    seeded set): a named consumer of eigenvector/Katz centrality, articulation points or a keyed
    topological sort;
  - datafrog 2.0.1: a Pass B Datalog-style fixed point;
  - graphops: option 3's objection resolved.
- **§B4 rewritten:** "Graph algorithms have named owners".

## Consequences

- **New dependency.** One more pinned crate (leiden-rs, plus a `rand` pin) and roughly 300 lines
  of our own FCA code and 40 of PageRank, each behind fixtures:
  - LFR planted partitions;
  - "shuffled and flipped input gives identical output";
  - a hand-computed FCA context, with fcars agreeing;
  - a hand-computed 3-node PageRank;
  - two parallel arcs counted with their weights;
  - a budget too small to converge, reported as not converged;
  - shuffled rows giving identical scores.
- **Statuses.** Community and page-rank outputs are `statistically_derived`, and by ADR-0005's
  rule they never state controls or limits. FCA outputs are `structurally_observed` over a
  structural scope.
- **Spike before acceptance:** leiden-rs determinism under shuffled input at our feature set.
  The probe settled it for normalized input, and it is re-run at acceptance with the fixture.

## Amendments (in place; the record is still proposed)

- 2026-09-23: remaining-scope plan, Phase 0, from the rust-graphs skill survey (source-read at
  the pinned leiden-rs 0.8.1 and petgraph 0.8.3; probes B009–B011):
  - **RBER, not raw CPM.** CPM's γ is in edge-weight units. With each layer normalized to unit
    total weight, CPM's useful γ range moves with the node and layer counts. RBER is the same
    objective with γ scaled by density (`quality.rs:239-309`).
  - **γ from our own fixed-seed grid.** `resolution_scan` changes the seed at each point, and
    `resolution_profile` runs an interval's ends with different seeds. Both hard-code the default
    configuration.
  - **Never `run_multiplex`**, which ignores `layer_weights` after the first local-moving level
    (`multiplex.rs:183-230`). Our weighted sum of layers is the multiplex objective.
  - **Never `from_petgraph` on the §5 graph** (a B009-class silent failure: its `E: Into<f64>`
    bound accepts the arc-row `u32`).
  - **Integer aggregation.** A multi-partition f64 `SUM` in DataFusion is not bit-stable, and a
    last-bit difference can flip a Leiden move.
  - **Stability** uses `leiden_rs::metrics::{try_nmi, try_ari}`. Per-community agreement is our
    own max-Jaccard matching.
  - **The traversal decision** (§5's adapter, a hand-written BFS) is confirmed. `visit::Bfs` is
    node-only, and `all_simple_paths` and rustworkx-core's BFS walk newest-first and collapse
    parallel arcs. Pass A implements it at increment 1 slice 1.4. The record is accepted at the
    increment-2 spike, with the Leiden fixture.
  - **SCCs, when a consumer appears,** use `kosaraju_scc` (iterative) rather than the recursive
    `tarjan_scc`.

