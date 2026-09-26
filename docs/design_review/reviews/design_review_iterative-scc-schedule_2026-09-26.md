# Design review: iterative call SCC schedule

**2026-09-26 · design/target · scoped review.** Core standard 3.0,
code-intelligence profile 1.1 and library-context binding. Subject:
[ADR-0052](../../adr/0052-iterative-scc-schedule.md), the SCC routine inside
`lctx-analytics::summaries::call_components`, and its persisted
`summary_components` consumer. The [forward plan](../../plans/behavioral-model-forward-plan_2026-09-24.md#6-findings-disposition)
owns W13's remaining type-term CTE and pilot-cost work.

## 1–5. Ownership, contract and change scenarios

An attributed call graph can contain a deep acyclic chain, mutual recursion,
self-recursion and duplicate/source-reordered edges. Petgraph owns strong
component discovery. Analytics owns the canonical schedule: source ids and
edges are sorted, component members are sorted, and the condensation is
scheduled callee-first with canonical-id ties. `cpg-core` publishes the rows;
the finite producer consumes component membership/order and recursive flags.
Neither consumer should depend on petgraph's incidental component order.

| Trigger | Owner and contract change | Required observation |
|---|---|---|
| A 30,000-edge chain | Select a stack-safe SCC routine; keep the same `CallComponent` output | Every node is one nonrecursive component, callee first, without recursive stack growth |
| Mutual/self recursion and reordered input | Keep canonical member and condensation ordering outside the library | Equal components/flags/order for reversed inputs; self-edge remains recursive |
| A future larger pilot | Revisit the two-pass cost only if measured material | Same component contract under any replacement, with a cost comparison |

The pinned petgraph 0.8.3 `tarjan_scc` implementation is recursive;
`kosaraju_scc` is iterative and works with the existing `Graph`. Capping
ordinary deep chains would reduce valid analysis, while writing a custom
iterative Tarjan would duplicate a library graph algorithm. The selected
library routine adds a second graph pass but leaves semantic identity and
tie-breaking in one repository owner. No new crate or graph adapter is needed.

## 6–8. Judgments, gates and disposition

A1–A3 and FP-01–FP-06 are **satisfied for this decision**: component ownership
is explicit, the two expected graph variations preserve the consumer contract,
and the library alternative avoids bespoke traversal state. Applicable
DP-01/02/03/08/11/13/15/16/21/23/24 and CI-01/02/04/06/08/11 pass at the
tested boundary. G1–G8 and CI-G1/CI-G2 **pass for SCC discovery and ordering**:
the large-chain and cyclic controls challenge the real production function.
CI-G3 is unchanged; the gold is not an input. Recursive summary composition,
pilot cost, and the independent type-term CTE closure identity are
**unresolved**; this review does not certify Stage 3 or all of W13. No new
finding is needed for the SCC branch; W13 retains the CTE and cost actions.

**Tested 2026-09-26:** `cargo test -p lctx-analytics --lib
deep_call_chain_is_stack_safe_and_callee_first --quiet` passed the 30,000-edge
case. `cargo test -p lctx-analytics --lib
call_sccs_are_callee_first_and_input_order_independent --quiet` passed the
mutual/self and reordered-input case. Pinned petgraph source and the local
rust-graphs index support the recursive/iterative implementation distinction.
`RUST_MIN_STACK=16777216 cargo test -p cpg-core --test compile
pinned_identity_models_require_and_publish_their_real_formals --quiet` passed the
real persisted component-order and modeled-summary checks.
`just fmt`, `just test-all`, fresh `just pilot` and order 6 composition are
`not_run`.

**Decision:** accept ADR-0052 for the SCC routine and canonical schedule;
leave W13's separate type-term and cost checks open in the forward plan.
