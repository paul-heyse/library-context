---
id: ADR-0052
title: Use iterative petgraph SCC discovery for the finite call schedule
status: accepted
date: 2026-09-26
supersedes: []
superseded-by: null
design: [§9.9]
evidence: Tested
revisit: A fresh pilot shows iterative two-pass SCC discovery dominates the finite summary schedule, or a supported petgraph release offers a stack-safe one-pass algorithm with a lower measured cost.
---

## Context

The [forward plan](../plans/behavioral-model-forward-plan_2026-09-24.md#6-findings-disposition)
W13 requires a stack-safe SCC schedule before finite recursive summary composition. The
current `lctx-analytics::summaries::call_components` uses petgraph 0.8.3 `tarjan_scc`, whose
pinned implementation is recursive. A deep attributed caller→callee chain can therefore
exhaust the Rust call stack before any summary work limit applies. The component list feeds
`cpg-core::summaries` and persisted `summary_components`; the order and recursive flags are
part of the finite producer's inputs. [§9.9](../design/sections/behavioral-analysis.md#section-9-9)
owns this schedule.

## Options

1. **Keep recursive Tarjan with an input-size cap.** This is the smallest code change, but a cap
   refuses otherwise ordinary deep acyclic call chains and does not bound stack bytes per edge.
2. **Write an iterative Tarjan implementation.** It could preserve a one-pass algorithm, but
   adds bespoke SCC state, correctness tests and ongoing library-parity burden to an already
   library-owned graph operation.
3. **Use petgraph's iterative `kosaraju_scc`.** Chosen. The pinned implementation uses two
   graph passes and is available for the existing `Graph`. It changes only component discovery;
   the repository's sorted member and condensation schedule still own stable callee-first output.
   The extra pass is a plausible cost, to be checked at the integrated pilot.

## Decision

`lctx-analytics::summaries` uses pinned petgraph `kosaraju_scc` for attributed-call SCC
discovery. It sorts source vertices/edges and each component's members, then schedules the
condensation by canonical member id. A self-edge still marks a singleton recursive. The
published `summary_components` contract and finite summary consumer do not depend on the
library's incidental component order. The graph library owns traversal; the repository owns
identity and tie order.

> Decision: ADR-0052

## Consequences

**Tested 2026-09-26:** `cargo test -p lctx-analytics --lib
deep_call_chain_is_stack_safe_and_callee_first --quiet` passed a 30,000-edge chain;
`cargo test -p lctx-analytics --lib
call_sccs_are_callee_first_and_input_order_independent --quiet` passed the existing
mutual/self-recursion and reversed-input schedule. Pinned petgraph source and the repository's
rust-graphs index identify `tarjan_scc` as recursive and `kosaraju_scc` as iterative.
`RUST_MIN_STACK=16777216 cargo test -p cpg-core --test compile
pinned_identity_models_require_and_publish_their_real_formals --quiet` passed the real
published component-order and modeled-summary checks.

The current component rows are unchanged for tested small graphs. A fresh pilot cost and the
full recursive composition fixtures are still `not_run`; W13's disposition in the
[forward plan](../plans/behavioral-model-forward-plan_2026-09-24.md#6-findings-disposition)
stays open for those, and W12 separately chooses the summary fixed-point engine.
