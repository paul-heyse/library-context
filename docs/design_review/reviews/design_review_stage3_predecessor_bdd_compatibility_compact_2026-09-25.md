# Stage 3 predecessor BDD compatibility — compact design review

## 1. Scope, purpose and coverage

| | |
|---|---|
| Subject | `lctx_analytics::summaries::predecessor_compatibility` and its persisted relation |
| Standard | Core 2.0, code-intelligence 1.0, library-context binding |
| Tier · purpose | Change · conformance |
| Reviewer · date | Codex · 2026-09-25 |
| Decision | Accept as a bounded candidate-path check only |

The first L3 pass loads both the provider and flow-analysis condition catalogs through `hydrate_catalog`, then uses the existing bounded BDD `and` operation over a predecessor edge's earlier, reaching and successor conditions. A false diagram refutes that edge under the declared evaluation atoms; a satisfiable diagram only admits a may-path. Loop-carried edges, missing roots and kernel caps produce an explicit unknown, with input caps retaining `budget_reached`. The result is reconstructed by the shared publication validator and is not a completed value transfer. Focused release Nextest passed 4/4 (209 skipped) for schema, fixture and contradiction control, then 2/2 (151 skipped) after the budget-reason refinement. Targeted release Clippy passed on 2026-09-25 after fixing a test-only clone lint.

## 6. Gates

| Gate | Verdict | Evidence or scope reason | Required action |
|---|---|---|---|
| G1 Authority | Pass | Every condition id comes from a cited predecessor candidate and one of the two validated catalogs. | — |
| G2 Semantic fidelity | Pass, scoped | `false` is only a propositional path refutation; `true` is may-compatibility. Loop-carried paths are unknown. | Do not upgrade to transfer or negative operation verdict. |
| G3 Validity | Pass, scoped | Declared Arrow contract, FKs and full result reconstruction reject missing/tampered rows. | — |
| G4 Hidden behaviour | Pass | BDD objects are hydrated from persisted roots; no display DNF or Python execution. | — |
| G5 Recovery | Pass, scoped | Version 51 and publication validation precede admission. | Run integrated gate at Stage 3 end. |
| G6 Transformation | Pass, scoped | All three source condition ids remain in-row; bounded `and` is the semantic operation. | Carry the result into later path composition. |
| G7 Claims | Pass, scoped | Relation name and DESIGN state compatibility, not call completion. | — |
| G8 Library leverage | Pass | biodivine BDD's bounded conjunction and the existing shared catalog hydration replace bespoke DNF logic. | — |
| CI-G1 Fidelity | Pass, scoped | The path candidate identity is preserved, and no sibling predecessor is substituted. | — |
| CI-G2 Evidence closure | N.a. | No served verdict consumes the result. | Check FORMAT 7. |
| CI-G3 Evaluation integrity | Pass | Small contradiction control and source fixture are independent of gold. | — |

## 7. Findings and principle verdicts

| ID | Finding | Principles · gate | Evidence and consequence | Correction |
|---|---|---|---|---|
| F01, deferred to summary composition | A satisfiable predecessor edge does not establish exclusive reaching, argument transfer or normal return. | DP-08, CI-06 · G7 | The result retains only input roots and a tri-state compatibility decision. | Apply exact call/model proof and candidate-set closure; else unknown. |
| F02, pilot gate | Combined catalog hydration work and edge counts are unmeasured on the pinned release. | DP-11, CI-08 · G3 | Focused fixtures cannot establish release-scale limits. | Report roots, nodes, checked edges, caps, time and RSS at the end pilot. |

DP-01/02/03/04/05/07/08/11/13/14/15/18/19/21/22/23/24 and CI-01/02/03/04/06/07/08/10/12 are satisfied within this bounded scope. Other principles add no new local obligation. No SHOULD exception is requested.

## 8. Library-leverage ledger

| Capability | Bespoke code | Built-in feature | Fit and limit | Recommendation |
|---|---|---|---|---|
| Three-way condition check | Candidate-to-root routing | Bounded `Diagram::and` and `is_false` | Exact propositional compatibility under attributed atoms; cap is explicit unknown. | Keep BDD kernel as authority. |
| Catalog load | Typed row conversion | Shared `hydrate_catalog` | Rejects malformed roots before checking an edge. | Reuse for summary/native load. |

## 12. Decision

**Accept only a tri-state compatibility fact.** Focused release Nextest passed 4/4 and then 2/2 for the boundary refinement; targeted release Clippy passed on 2026-09-25. It is not `summary_flows` or a call-transfer discharge. Integrated `just test-all`, `just pilot`, structured evaluation and formatting remain `not_run` until full Stage 3 functionality is ready.
