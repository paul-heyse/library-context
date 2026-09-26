# Design review: finite summary outcomes

**2026-09-25 · change/conformance · scoped review.** Core 3.0, code-intelligence profile
1.1, library-context binding (`standard.toml`). Subject: ADR-0050 and the W5 finite summary
producer, acquisition, publication, validator and native trace. This is a self-review of the
bounded slice; [plan W5](../../plans/behavioral-model-forward-plan_2026-09-24.md#6-findings-disposition)
owns its current disposition and the unqualified enclosing Stage 3 scope.

## 1–3. Scope, ownership and contract

`cpg-schema` owns the append-only boundary reasons, typed source-origin query and persisted
rows. `cpg-core::summaries` acquires DataFusion rows and condition catalogs. The pure
`lctx-analytics::summaries::finite` producer consumes them, applies the bounded BDD and
callee-first component rules, and returns flows, ordered proofs, typed refusals and coverage
boundaries. `cpg-core::attempt` publishes that result; `validate` reconstructs it once.
The bundle converts codebook values to text, and the native reader validates their names and
serves open boundaries as unknowns. The dependency direction matches §9.9 and §B2/B3; the
new module is an owner within the existing analytics crate, not a new service lifecycle.

| Fact/fidelity | Authority and identity | Coverage and consumer |
|---|---|---|
| Raw return origin | `value_flow_contributions` and `flow_values`, source use/fact/condition | `summary_boundary_candidates` carries every same-callable parameter origin; an origin is not itself a completed transfer |
| SCC order | Attributed call targets, petgraph `tarjan_scc`, sorted condensation | Component membership schedules acyclic composition; candidate/open targets confer no value verdict |
| Condition | Lossless `analysis_conditions`/`condition_nodes`, bounded `Diagram` | One summary-facing `condition_limit` mapping keeps work/node/atom caps unknown |
| Summary | Cited source facts, ordered proof steps and canonical summary id | Delta and native serving use the same typed outcome; absent flows never mean refutation |

The current persisted boundary key is `(snapshot, function, formal, raw fact, condition)`.
Distinct origins can share it. The producer retains a generic open boundary for that group,
but cannot yet serve each origin's distinct cause. This is a declared limit, not a negative
claim. An input containing a unique origin can carry its specific depth or BDD refusal.

## 4–5. Composition and change scenarios

An added finite transfer case needs its seed relation, proof rule and witness check in the
analytics owner; acquisition only supplies the new typed relation. A new cap reason is added
to the append-only codebook and the single kernel-to-summary mapping, then flows through the
same publisher, validator, bundle text and native admission. A second origin that shares a raw
fact requires the planned path-specific persisted identity migration before its individual
reason can be claimed. For recursive composition, the SCC schedule and typed refusal are already
inputs/outputs; a fixed-point engine can be compared without taking over acquisition or
publication. The current local depth budget is deterministic at 8; it withholds paths without
turning a missing proof into false. A BDD work result is not inferred from time elapsed.

## 6–9. Gates, findings and library fit

G1/G2 and CI-G1 hold for the unique-origin path examined: source facts and conditions remain
typed, and a cap stays unknown. G3/G5 hold for that trace: the shared validator reconstructs
the producer result before publication, and the native loader checks the boundary codebook.
G4 holds for the pure transformation, which has no session or store access. G6/G8 hold for this
choice: petgraph owns SCCs and biodivine-lib-bdd owns bounded condition decisions; no generic
fixed-point framework was introduced before W12's comparison. G7 and CI-G2 hold for the open
boundary response examined, with no claim of a completed transfer. CI-G3 is unaffected: gold
references do not enter the producer. These are scoped judgments; the BDD-cap/native trace and
all recursive behavior remain unqualified.

FP-01–FP-06 and applicable DP-01/02/03/08/11/12/13/16/17/18/19/21/23/24 and
CI-01/02/04/06/07/08/11/13 are satisfied for the unique-origin finite case by the explicit
input/output boundary and cited trace. **F01 (scoped limit):** persisted boundary identity
still groups sibling origins, so adding two independently refused paths can require a generic
fallback rather than their individual causes (DP-04, CI-03, A2). The correction is the Stage 3
order-3 origin identity and matching source/serving migration; closure requires a sibling
fixture whose two causes survive publication and native response. [Plan W5/order 3](../../plans/behavioral-model-forward-plan_2026-09-24.md#6-findings-disposition)
owns disposition. The simpler SQL anti-join was rejected in ADR-0050 because it duplicates
summary policy; a new service crate has no second lifecycle consumer. A native SCC worklist,
Ascent and datafrog remain open W12 candidates, not conclusions from this slice.

## 10–12. Verification, authority and judgment

**Tested 2026-09-25:** `cargo test -p lctx-analytics --lib summaries::finite::tests --quiet`
passed direct admission, preceding-call refusal, nine-hop depth cap and condition-work mapping;
`RUST_MIN_STACK=16777216 cargo test -p cpg-core --test compile
nested_returns_need_an_uncontrolled_exit_before_becoming_value_summaries --quiet` and
`pinned_identity_models_require_and_publish_their_real_formals --quiet` passed, preserving a
dynamic sibling's call-transfer boundary. `RUST_MIN_STACK=16777216 cargo test -p cpg-core --test
bundle finite_depth_and_unsupported_refusals_reach_the_native_response --quiet` passed after
`uv sync --locked --reinstall-package lctx-semantics`, observing depth and unsupported causes
in Delta, FORMAT 8 and native output. `INSTA_UPDATE=no cargo test -p cpg-schema --test codebooks
--quiet` passed after reviewing/accepting the append-only snapshot; `cargo check -p cpg-core
--quiet` and `just adr lint` passed. The BDD-cap snapshot/native trace, `just fmt`, integrated
`just test-all`, fresh `just pilot`, and structured evaluation are **not_run**.

ADR-0050 updates §9.9 and this review changes no authority. A1 is satisfied for adding a
finite case without acquiring a session; A2 is satisfied for the unique-origin refusal and
unresolved for grouped siblings (F01); A3 is satisfied for composing the existing SCC and BDD
kernels through typed inputs. **Accept scoped** at Tested strength for unique-origin finite
outcomes. The enclosing Stage 3 architecture is unresolved; W5's BDD control, path identity,
W7's fixed point and W12's engine comparison remain on the plan.
