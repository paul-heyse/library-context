---
id: ADR-0051
title: Compute source reach as a bounded fixed point over uses
status: accepted
date: 2026-09-26
supersedes: []
superseded-by: null
design: [§3.9]
evidence: Tested
revisit: The fresh pilot shows the whole-use worklist materially exceeds the flow-model budget, or an incomplete frontier cannot be represented truthfully by per-use boundaries and unknown source conditions.
---

## Context

The previous `Model::reach` cut a recursive path at a repeated use, then memoized the cycle
head's first result. For `A = p OR Call(B)` and `B = A`, querying A first lost the loop-carried
`Call` variant; querying B first retained it. The affected source-origin rows feed finite
summary seeds and unknown coverage. The [forward plan](../plans/behavioral-model-forward-plan_2026-09-24.md#6-findings-disposition)
W7 requires the least fixed point, deterministic work bound and an explicit unknown before
incomplete sources can be treated as complete. [§3.9](../design/sections/behavior-model.md#section-3-9)
owns this runtime abstraction.

## Options

1. **Patch DFS memoization at cycle heads.** This can fix the two-node example but still needs
   repeated propagation, a termination rule and an explicit cap. It keeps query order in the
   semantic algorithm.
2. **Build and schedule SCCs first.** This localizes cyclic work, but adds a second graph and
   condensation policy for a source-use relation whose finite transfer already has a small,
   monotone state. It is useful if the whole-use worklist is measured too costly.
3. **Use a sorted reverse-dependency worklist over all uses.** Chosen. Each changed child
   reschedules parents; recomputation joins `(origin, transfer, condition)` states until the
   least fixed point. A deterministic row/source-work budget withholds the pending frontier
   and its transitive dependents. This is the simplest fixed-point mechanism for the current
   relation and has no separate engine dependency.

## Decision

`cpg-core::flow_model::Model` solves source reach once per receiver mode by a sorted worklist.
It recomputes a use from current child states and memoizes only after convergence or a named
budget cut. The transfer lattice retains both strong and weak variants; loop-carried edges keep
their declared use-side condition approximation. Equal state compares semantic condition IDs,
not allocation or query order.

The cap is one million visited reaching rows, definition-value edges and child-source pairs.
On exhaustion, every unfinished use and transitive parent gets a `flow_reach_boundaries` row
with `budget_reached`; already discovered origins at those uses receive an unknown bounded
condition. `summary_boundary_candidates` gives reach-budget refusal priority over generic
call/control fallbacks, and field/global negative premises are withheld when any reach mode is
incomplete. The table has a shared source-equality validator and a `flow_uses` reference.
The bound is not a negative assertion about undiscovered origins.

This uses a whole-use worklist rather than the W7 plan's SCC-local schedule. It avoids a second
component representation now; the revisit trigger above allows an SCC optimization against
this semantic result, with shuffled-input and cap parity required. This decision is separate
from W12's choice of an engine for recursive interprocedural summaries.

> Decision: ADR-0051

## Consequences

**Tested 2026-09-26:** focused unit tests give the same Identity and Call variants for A-first,
B-first and reversed input rows; a low-budget control marks both members and widens a known
source to unknown. A real `nested_returns_need_an_uncontrolled_exit_before_becoming_value_summaries`
compile and the finite-depth/native bundle test passed with the new producer and table; reviewed
schema/rule snapshots were accepted. The production one-million-work cap has not been reached in
a published fixture, and the fresh pilot's cost/unknown distribution is `not_run`. Those,
path-specific refusal identity, integrated tests and Stage 3 questions remain in the
[forward plan](../plans/behavioral-model-forward-plan_2026-09-24.md#6-findings-disposition).
