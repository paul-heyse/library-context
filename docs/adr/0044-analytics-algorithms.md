---
id: ADR-0044
title: Graph and lattice algorithms have named owners — petgraph traversal, leiden-rs communities, our own PageRank and FCA, direct-usage ranking
status: accepted
date: 2026-09-25
supersedes: [ADR-0011]
superseded-by: null
design: [§B4, §5, §9]
evidence: Interface-checked
revisit: leiden-rs gives different partitions for identical normalized input and seed or fails the LFR planted-partition fixtures; our PageRank and its reference oracle disagree beyond tolerance; the plan W12 comparison adopts a recursion engine or W13 decides the SCC routine and ordering owner; or a consumer appears for centralities, articulation points or personalized PageRank beyond the current ones.
---

## Context

The analytics ([§9](../design/sections/analytics.md#section-9)) and behavioral summaries
([§9.9](../design/sections/behavioral-analysis.md#section-9-9)) need traversal, strongly connected
components, community detection, centrality and formal concept analysis over projections whose
identity, parallel arcs and evidence must survive. Each generic library call has to be checked
against those requirements rather than trusted by name. This record replaces ADR-0011, restating
its decision and accepted amendments as they stand today; which techniques run **by default** is
decided separately (ADR-0020, ADR-0021).

Checked facts behind the decision (2026-09-22/23, pinned releases; Tested by probes where noted):
- **petgraph 0.8.3** has no community detection. `Graph` keeps parallel edges; `edges_directed`
  lists the newest edge first; `Bfs` and `Dfs` visit siblings in opposite orders; removing a node
  re-points held indices. Its `page_rank` takes no edge weights, counts parallel edges in the
  out-degree but credits the target once, differs from textbook PageRank on a 3-node graph, is
  O(iterations·V·E) and reports no convergence (Tested). `algo::condensation` merges parallel
  edges, keeping only the last call site (Tested).
- **leiden-rs 0.8.1** offers Leiden with RBER among its quality functions, seeded RNG and a
  sequential build. Its undirected builder does not normalize orientation, so shuffled or flipped
  input changed a partition until edges were put in `(min, max)` + sorted normal form, after
  which every perturbation gave identical output (Tested). It has no converged flag, `seed: None`
  draws OS entropy, `run_multiplex` ignores `layer_weights` after the first level, and
  `resolution_scan`/`resolution_profile` change seeds between points.
- **No usable concept-analysis crate:** odis (NextClosure, FCbO, implication basis) is AGPL and
  not needed at that cost; fcars enumerates concepts only, with no support threshold or
  implications; dci is unmaintained.
- **rustworkx-core 0.18.1** has centralities and connectivity but no community detection or
  PageRank. **graphops 0.5.1** has a converging PageRank, but its `petgraph` feature pulls
  petgraph 0.6.

## Options

1. **Our own Leiden.** Rejected: roughly 500 lines of subtle optimisation code while a seeded,
   tested implementation exists. It is the fallback if leiden-rs fails the revisit checks.
2. **No community detection;** seeds from ranking and public exports only. It is what the
   default now runs (ADR-0020), and the baseline any variant is judged against; the kernel is
   kept so the variants remain testable and reversible.
3. **PageRank from graphops or petgraph.** Rejected: graphops' dependency line, petgraph's
   miscounting and lack of weights.
4. **Chosen:** petgraph for traversal, SCCs and dominators; leiden-rs on a normalized edge list;
   our own weighted PageRank, NextClosure FCA and one-step RCA, each with an independent oracle;
   direct usage as the default ranking.

## Decision

**Traversal and projections** (§5, §9.1; **Tested**). Explicit BFS with parent pointers over an
immutable `Graph<(), u32, Directed, u32>` whose edge weight is the arc's row index. The sorted
domain ids are the dense index; edges are added in canonical arc order; filtered and reversed
views are used, never copies; nothing is removed; SCC members are sorted. `visit::Bfs` (node-only),
`all_simple_paths` and rustworkx-core's BFS (newest-first, collapsing parallel arcs) are not used.

**SCCs and summary order.** Transfer summaries are scheduled bottom-up in petgraph `tarjan_scc`
order, callees first (**Implemented**), each SCC to be iterated to a fixpoint over a finite domain
with widening to `unknown` (accepted target; recursive members are currently withheld). Two
questions are **open** and are not decided here: whether the recursive `tarjan_scc` must give way
to an iterative routine for stack safety on large call graphs, and who owns the SCC/topological
ordering ([plan W13](../plans/behavioral-model-forward-plan_2026-09-24.md#6-findings-disposition));
and which engine carries recursive summary composition — a bounded native worklist, Ascent or
datafrog, compared on one semantic state and refusal contract after explicit summary inputs
land ([plan W12](../plans/behavioral-model-forward-plan_2026-09-24.md#6-findings-disposition)).
No Datalog engine is adopted.

**Condensation** (**Proposed**, deferred until a consumer beyond recursion labelling, DESIGN
§13): built from SCC membership keeping every arc's evidence, never petgraph's `condensation`.

**Flow algorithms** (accepted assignment, not yet implemented; flow reachability currently comes
from ty's index): dominators by petgraph `algo::dominators::simple_fast`; post-dominators by the
same function over `Reversed` from a virtual exit; dominance frontier and control dependence in
our own code.

**Communities** (§9.4; **Tested**; a variant, off by default).
- leiden-rs `=0.8.1`, `default-features = false`, no features, with **RBER** (CPM with γ
  relative to density, so γ is scale-free on unit-normalized layers).
- Each layer is integer counts per `(min, max)` pair, counted in Rust in a `BTreeMap` over
  declared relations, each pair keeping its least contributing site; the layer policies are named
  in `communities::Params`; unit-total normalization, 95th-percentile hub down-weighting and layer
  weighting follow in canonical order.
- **γ = 1** over seeds 0–9; the γ profile runs for its stability record but γ is never chosen
  from it (a grid choice proved to follow the seed block, not the data); a degenerate partition
  reports nothing, with the reason.
- Never `run_multiplex`, `resolution_scan`, `resolution_profile` or `from_petgraph` (which would
  read the §5 graph's arc-row weight as an edge weight). The seed is always set and recorded;
  `track_quality_history` stands in for the converged flag; `rand` is pinned; crate versions go in
  each invocation. Stability by `leiden_rs::metrics::{try_nmi, try_ari}`; a reported community's
  score is its public members' co-assignment across the other seeds.
- Communities may cap seed selection and fill Related. They never define brief boundaries or FCA
  scopes, and they have no tool consumer.

**Ranking and selection** (§9.5; **Tested**).
- The default ranking is **direct usage**: each official-usage call site counted once, split
  among its definite or candidate targets (`ranking::USAGE_POLICY`), published as
  `structurally_observed` counts that order and never state behaviour. The consumers (seed
  selection and the Related order) ask which operations official usage calls; PageRank over
  delegation answers a different question and ranks implementation sinks first.
- Delegation PageRank is our own weighted power iteration over the usage projection in canonical
  order, with uniform teleport and dangling mass, recording iterations, the final L1 residual and a
  converged flag. Its reference oracle is `leiden_rs::compute_flow`, valid while the dangling
  target is uniform. It runs only as the `+pagerank` variant.
- **Selection:** configured seeds first, then eligible public APIs (official usage calls them and
  they have a docstring summary) by rank; when communities run they cap rather than choose (at
  most ⌈budget / 3⌉ seeds each). Every public callable is named by one preferred path, through
  its declaring class first.
- The parameters of communities, PageRank and selection are pre-registered code, recorded in
  every invocation, in the compiler digest and in the gold freeze (ADR-0021 owns the freeze).

**Concept analysis** (§9.6; **Tested**; variants, off by default).
- Our own NextClosure over `fixedbitset`, yielding the frequent concepts and the Duquenne–Guigues
  basis under a support threshold; no stability index (#P-hard). FCbO replaces it only if the
  concept count exceeds the budget. `fcars =0.2.2` is a dev-dependency oracle for concept sets;
  concept-set agreement does not test attribute fidelity. No cover relation is computed.
- FCA runs only over structurally defined scopes keyed by node, so its concepts and implications
  are `structurally_observed` over the stated attributes. A brief states a concept as a shared
  signature under Related, from the seed's own scope; attributes are built from term structure
  (no undetermined type, one attribute per raised class).
- RCA is one relational-scaling step feeding the same FCA. When extended over behavioral contexts
  it runs at the declared least fixed point. It is not extended, and candidate-derived output is
  not claimed as behaviour, until attributes are typed with modality
  ([plan W10](../plans/behavioral-model-forward-plan_2026-09-24.md#6-findings-disposition)).
- FCA over behavioral attributes (candidate facets, Stage 4) is the only planned tool consumer
  among these techniques. None of them writes concept membership.

**Deferred with triggers** (plan §5): rustworkx-core, keeping the bespoke keyed ordering, until a
consumer needs centralities, articulation points or a keyed topological sort beyond the current
ones; graphops, until a consumer needs personalized PageRank and its petgraph feature matches our
line. Ascent and datafrog are compared, not deferred (W12 above).

## Consequences

- One pinned community crate (leiden-rs plus a `rand` pin) and small bespoke kernels (about 300
  lines of FCA, 40 of PageRank), each behind fixtures: LFR planted partitions; shuffled and
  flipped input giving identical output; a hand-computed FCA context with fcars agreeing; a
  hand-computed 3-node PageRank; parallel arcs counted with their weights; a too-small iteration
  budget reported as not converged; shuffled rows giving identical scores.
- Community and PageRank outputs are `statistically_derived` and never state controls, limits or
  behavioural claims (ADR-0005); FCA outputs are `structurally_observed` over a structural scope.
- Recursive relational walks are not graph algorithms: the type-term walks run as DataFusion
  recursive CTEs with `UNION ALL`, relying on extraction's depth cap, and their row identity is an
  open item under W13; Stage 4's registry integrity check adopts `WITH RECURSIVE … UNION`.
- Evidence floor: the flow-algorithm assignment is only **Interface-checked** against the pinned
  API; the other clauses are **Tested** as stated. Open W10, W12 and W13 are not closed by this
  record.
