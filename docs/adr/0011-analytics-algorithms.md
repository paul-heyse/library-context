---
id: ADR-0011
title: petgraph traversal and SCCs, our own weighted PageRank, leiden-rs communities, own FCA then RCA
status: accepted
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

The record was consolidated at acceptance (2026-09-23, the ADR-0011 standard review's F7): the
amendments made while it was proposed are folded in below, and each part carries its own label.
The header's `evidence: Interface-checked` is the record's floor: FCA is still only that.

- **Traversal** (DESIGN §5, §9.1; **Tested**, slice 1.4): explicit BFS with parent pointers over
  an immutable `Graph<(), u32, Directed, u32>` whose edge weight is the arc's row index.
  Construction follows §5's adapter recipe:
  - the sorted domain ids are the dense index;
  - edges are added in canonical arc order;
  - filtered and reversed views are used, never copies;
  - nothing is ever removed;
  - SCC members are sorted.

  `visit::Bfs` is node-only, and `all_simple_paths` and rustworkx-core's BFS walk newest-first and
  collapse parallel arcs, so neither is used.
- **Condensation** (when §13 un-defers it; **Proposed**): built from `kosaraju_scc` membership
  (iterative; `tarjan_scc` recurses), keeping every arc's evidence; never petgraph's
  `condensation`.
- **Communities** (§9.4; **Tested**, slice 2.3 and this review; deviation log D28):
  - leiden-rs `=0.8.1`, `default-features = false` and **no features** (not `petgraph`, not
    rayon), with the **RBER** quality function: CPM with γ relative to the graph's density, so γ is
    scale-free on unit-normalized layers (raw CPM's γ is in edge-weight units);
  - each layer is **integer counts** per `(min, max)` pair, counted in Rust in a `BTreeMap` over
    the declared relations' rows, each pair keeping its least contributing site; what each layer
    counts is a named policy in `communities::Params`; normalization (unit total), hub
    down-weighting (95th-percentile strength) and layer weighting (0.5 each) follow in canonical
    order;
  - **γ = 1**, RBER's own scale, over seeds 0–9; the profile γ ∈ {0.5, 1, 2, 4} runs and its
    stability is recorded, never chosen from (a grid choice was decided by the seed block, not the
    data); a degenerate γ = 1 reports no communities, with the reason in the diagnostics;
  - the parameters are pre-registered code, recorded in every invocation, in the compiler digest
    and in the gold freeze, because `analytics.toml` is frozen (D21);
  - never `run_multiplex` (it ignores `layer_weights` after the first level), never
    `resolution_scan` or `resolution_profile` (they change seeds between points), never
    `from_petgraph` (it would read the §5 graph's `u32` arc-row weight as the edge weight);
  - the seed is always set and recorded; `track_quality_history` stands in for the missing
    converged flag; rand is pinned on its 0.9 line, and crate versions go in each invocation;
  - stability by `leiden_rs::metrics::{try_nmi, try_ari}`; a reported community's score is its
    public members' co-assignment across the other seeds, the grouping the finding publishes.
  - Communities select seeds within the brief budget and fill the brief's **Related** field.
    **They do not define brief boundaries or FCA scopes.**
- **Centrality** (§9.5; the rejection of petgraph's `page_rank` **Tested** by probe; our own
  PageRank **Tested**, slice 2.4): **our own weighted power iteration** over the usage projection
  (the invocation projection restricted and weighted by a named policy) in canonical order.
  - Dangling mass and teleport are uniform.
  - It records iterations, the final L1 residual and a converged flag (guidelines §8).
  - Its reference oracle is `leiden_rs::compute_flow` (a weighted directed PageRank with uniform
    teleport and dangling mass, already a dependency), valid while the dangling target is
    uniform.
- **FCA / RCA** (§9.6; **Interface-checked**): our own NextClosure (Ganter, ICFCA 2010), which
  also yields the Duquenne–Guigues implication basis, over `fixedbitset`, with a support threshold.
  There is no stability index: it is #P-hard.
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

## History while proposed

- 2026-09-23, remaining-scope plan Phase 0 (the rust-graphs skill survey; probes B009–B011): RBER
  instead of raw CPM; our own seeded γ grid instead of `resolution_scan`; never `run_multiplex`
  or `from_petgraph`; integer aggregation; `try_nmi`/`try_ari` stability; the traversal decision
  confirmed; `kosaraju_scc` for SCCs.
- 2026-09-23, slice 2.3, the acceptance spike (D28): the kernel implemented; LFR recovery and
  identical output under shuffled and flipped input Tested; the parameters in code.
- 2026-09-23, the standard review (`design_review_adr0011-slice2.3-communities_2026-09-23.md`,
  Revise small): γ fixed at 1 (F2), named layer policies and the invocation projection in the
  community digest (F3), the parameters in the gold freeze (F4), the public co-assignment score
  (F5), this consolidation (F7). Accepted with those fixes.

## Amendments

- 2026-09-23, the increment-2 compact review (`design_review_inc2-compact_2026-09-23.md`,
  Revise small; deviation log D37, D38):
  - **Centrality's consumer (U1).** Seed selection and the Related order ask which operations
    official usage calls. The default ranking is now **direct usage**: each usage call site
    counted once, split among its definite or candidate targets (`ranking::USAGE_POLICY`), a
    `usage_count` invocation and `direct_usage` findings (`structurally_observed`, ordering
    only). Delegation PageRank, this record's page-rank decision, becomes the `+pagerank`
    analytics variant, kept only if the §9.8 ablation shows it helps: over delegation it ranks
    implementation sinks above what users call (review F3).
  - **Selection.** Configured seeds first; then eligible public APIs (a docstring summary and at
    least one direct usage call) by rank; a community caps rather than chooses (at most
    ⌈budget / 3⌉ seeds each). This replaces slice 2.6's rounds over communities.
  - **FCA's published form (U2).** A concept holding the seed is stated as a shared signature
    under Related, from the seed's **own** scope only (F1: scopes are keyed by node), and the
    Applicable-case slot stays absent until an input-or-mode source exists. Attributes are built
    from term structure: no undetermined (`Unknown`) type, one attribute per raised class (F2).
  - **Names.** One preferred path per public callable, through its declaring class first (F4).
  - **Freeze.** These choices are `selection::Params`, frozen by digest beside the analytics
    parameters (F7).
