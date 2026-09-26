# Design review: bounded source-reach fixed point

**2026-09-26 · design/target · scoped review.** Standard: core 3.0, code-intelligence
profile 1.1 and library-context binding. Subject: ADR-0051 and W7's `Model::reach`,
`flow_reach_boundaries`, summary boundary input and shared validator. This dated review is
evidence; [plan W7](../../plans/behavioral-model-forward-plan_2026-09-24.md#6-findings-disposition)
owns current disposition.

## 1–3. Responsibility and semantic contract

The `cpg-flow`/ty provider supplies attributed use-def and loop-carried rows. `cpg-core::flow_model`
owns the runtime source lattice: a use maps `(origin, transfer)` to a bounded condition and
capture flag. `cpg-schema` owns the new persisted per-use budget row and its `flow_uses` reference.
`cpg-core::attempt` publishes it; the shared validator reruns the same model and compares the
rows. The summary producer reads the boundary with its raw contribution; a reached cap takes
priority over generic call/control refusal. Field/global negative premises withhold a negative
when any reach mode is incomplete. No analyzer relation is relabelled as a completed call.

| Fact and fidelity | Authority and identity | Coverage / downstream meaning |
|---|---|---|
| Reaching row | ty's attributed use, definition, condition and `loop_carried` | An edge in the stated runtime abstraction, not value provenance by itself |
| Source state | `flow_model` fixed point over origin/transfer/condition | Both identity and weaker call-transfer variants survive; one origin's variants are not collapsed |
| Capped use | `flow_reach_boundaries(snapshot,use)` with `budget_reached` | Missing origins unknown; discovered origins' conditions widen; summary and negative premises must not treat them as complete |

The persisted per-use boundary can name a capped use whose origin was not discovered, but the
current native value-path endpoint indexes by formal and has no direct route from that use to
an affected formal. The absence of a formal path is explicitly unknown in that endpoint's
contract; it is not evidence of no flow. A later operation-wide completeness answer needs a
source-to-formal coverage projection before it can claim exhaustive results.

## 4–5. Composition, scenario and alternatives

The probe's equations `A = p OR Call(B)`, `B = A` exposed the former DFS error: querying A
first memoized only Identity. The sorted reverse-dependency worklist recomputes a parent when
its child state changes; both orders now yield Identity and Call. This semantic state is finite
under the bounded condition kernel, and visited rows/edges/source pairs are capped at one
million; a cap propagates to the pending frontier's ancestors and writes unknown, never false.
A new flow transfer kind changes the lattice and one producer, not the SQL acquisition. A new
consumer of source completeness reads the typed boundary rather than inferring completeness
from an empty contribution table.

ADR-0051 compares DFS patching, SCC-local scheduling and the selected whole-use worklist.
The latter avoids another component representation and keeps a direct local test boundary;
its extra acyclic work and whole-model memory cost remain unmeasured until the fresh pilot.
Petgraph is already used for call-graph SCCs, but an SCC pass is not necessary to express this
least fixed point. This source-use worklist is distinct from W12's recursive summary-engine
comparison. A measured cost may justify an SCC optimization with identical results/caps.

## 6–9. Gates, findings and judgment

G1/G2/CI-G1 pass for the examined cyclic graph: source and transfer fidelity are retained and
budget exhaustion stays unknown. G3/G5 pass for the examined rows: schema, codebook, reference
and source-equality validator reject malformed or forged boundaries before publication. G4
passes for the worklist, which reads fixed inputs and has no external effects. G6 passes for
A-first/B-first/reversed source rows, not yet for shuffled real extraction bytes. G7 passes for
the scoped claim; a production-cap/served formal trace is still untested. G8 passes for the
chosen small fixed-point mechanism pending measured performance. CI-G2/CI-G3 are not changed by
this table: no new positive served claim or gold input was introduced.

FP-01–FP-06 and applicable DP-01/02/03/08/11/12/13/16/18/19/20/21/23/24 and
CI-01/02/04/06/07/08/10 are satisfied for the synthetic fixed-point and real baseline compile.
**F01 (coverage limit):** a cap with no discovered source has a persisted use boundary but no
formal-specific native boundary. A future exhaustive operation answer cannot use empty paths
as absence. The affected owners are the source-to-formal coverage projection and native serving;
closure is a capped zero-source fixture whose formal returns explicit unknown. [Plan W7 and
Stage 3 order 3/7](../../plans/behavioral-model-forward-plan_2026-09-24.md#6-findings-disposition)
own that work. **F02 (unmeasured cost):** whole-use evaluation may recompute more acyclic states
than an SCC schedule; compare actual pilot time/RSS and consider the SCC alternative only if
material. The fresh pilot is the plan's disposition trigger, not a new benchmark gate.

## 10–12. Evidence, authority and bounded decision

**Tested 2026-09-26:** `cargo test -p cpg-core --lib reach_fixed_point_tests --quiet`
passed A-first/B-first/reversed and a forced low work cap; `cargo test -p lctx-analytics --lib
summaries::finite::tests --quiet` passed the budget-priority control;
`RUST_MIN_STACK=16777216 cargo test -p cpg-core --test compile
nested_returns_need_an_uncontrolled_exit_before_becoming_value_summaries --quiet` and the
finite-depth/native bundle test passed with the new table; `INSTA_UPDATE=no cargo test -p
cpg-schema --test contracts --quiet` passed after reviewing and accepting the new contract and
rule snapshots. `just adr lint` passed. `just fmt`, a production-cap snapshot/native trace,
real shuffled-input byte comparison, integrated `just test-all` and fresh `just pilot` are
**not_run**.

ADR-0051 updates §3.9. A1 is satisfied for localized source-transfer changes; A2 is satisfied
for the typed cap row and unresolved for formal-specific zero-source serving (F01); A3 is
satisfied for current fixed-point composition with F02's measured revisit. **Accept scoped**
at Tested strength for the cyclic source-state correction and persisted budget boundary. The
enclosing Stage 3 architecture and W7 acceptance remain open at the stated controls.
