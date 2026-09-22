---
id: ADR-0011
title: petgraph traversal and page_rank, leiden-rs communities, own FCA then RCA
status: proposed
date: 2026-09-22
supersedes: []
superseded-by: null
design: [§B4, §5, §9]
evidence: Interface-checked
revisit: leiden-rs gives different partitions for identical input and seed, fails the LFR planted-partition fixtures, or a technique's ablation (DESIGN §9.8) changes no published output after increment 3.
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

## Options

1. **Our own Leiden.** Rejected for now: roughly 500 lines of subtle optimisation code, while a
   seeded, tested implementation exists. It is the fallback if the revisit trigger fires.
2. **The simpler alternative: no community detection;** seeds come from `page_rank` and public
   exports only. Rejected, because the operator wants analytic grouping. It remains the baseline
   that the §9.8 ablation compares against.
3. **Chosen:**
   - petgraph for traversal and `page_rank`;
   - leiden-rs 0.8.1 for communities;
   - our own NextClosure FCA, with a one-step relational-scaling RCA added in increment 3.

## Decision

- **Traversal** (DESIGN §5, §9.1): explicit BFS with parent pointers over an immutable
  `Graph<(), ArcRow, Directed, u32>`. Adjacency is sorted by canonical key, nodes are added in
  canonical order, nothing is ever removed, and SCC members are sorted.
- **Communities** (§9.4):
  - leiden-rs `=0.8.1`, `default-features = false, features = ["petgraph"]`, CPM, with the seed
    recorded;
  - `rand` pinned, and crate versions in the method parameters;
  - layers normalized and hubs down-weighted;
  - seed-consensus stability.
  - Communities select seeds within the brief budget and fill the brief's **Related** field.
    **They do not define brief boundaries or FCA scopes.**
- **Centrality** (§9.5): petgraph `page_rank` combined with usage counts from examples and tests.
- **FCA / RCA** (§9.6): our own NextClosure with a support threshold (no stability index; it is
  #P-hard).
  - It runs only over structurally defined scopes (the subsystem, a module, a class hierarchy), so
    its concepts and implications are `structurally_observed`.
  - The one-step ∃-scaling RCA is added in increment 3, under the §9.8 keep rule.
- **§B4 rewritten:** "Graph algorithms have named owners".

## Consequences

- **New dependency.** One more pinned crate (leiden-rs, plus a `rand` pin) and roughly 300 lines
  of our own FCA code, each behind fixtures:
  - LFR planted partitions;
  - "shuffled input gives identical output";
  - a hand-computed FCA context.
- **Statuses.** Community and page-rank outputs are `statistically_derived`, and by ADR-0005's
  rule they never state controls or limits. FCA outputs are `structurally_observed` over a
  structural scope.
- **Spike before acceptance:** leiden-rs determinism under shuffled input, at our feature set.
