# Stage 3 model application completion context — compact review

## 1. Scope, purpose and coverage

| | |
|---|---|
| Subject | `model_applications` projection of target count and pinned target normal-return assertion |
| Standard | Core 2.0, code-intelligence 1.0, library-context binding |
| Tier · purpose | Change · conformance |
| Reviewer · date | Codex · 2026-09-25 |
| Decision | Accept the source-call context fields; no completed call claim |

The existing model application already retained the source call target, candidate-set completeness and unresolved remainder. `resolutions.target_count` is now retained separately because a complete candidate set can contain several targets. The selected model target's authored normal-return assertion is also carried forward. I inspected the SQL projection, schema, shared source-equality validator and the focused `model_shapes` fixture. Source-call completion and summary use remain unreviewed and unsupported here.

## 6. Gates

| Gate | Verdict | Evidence or scope reason | Required action |
|---|---|---|---|
| G1 Authority | Pass | `resolutions` owns target count; `model_targets` owns the assertion; application rows are derived. | — |
| G2 Fidelity | Pass | Count, candidate closure and target completion are separate fields. | Keep them separate in L3. |
| G3 Validity | Pass | Nonnegative count check and shared reconstruction reject forged rows. | — |
| G4 Hidden behaviour | Pass | One DataFusion join over persisted source facts and catalog targets. | — |
| G5 Consistency | Pass | Compiler output version 57 and reviewed Arrow snapshot mark the migration. | — |
| G6 Transformation | Pass | The projection retains source call, target and model identities. | — |
| G7 Claims | Pass, scoped | The relation says a pinned model applies to a candidate call; no completion is inferred. | — |
| G8 Library leverage | Pass | Existing DataFusion SQL and Arrow contract supply the generic projection. | — |
| CI-G1 Fidelity | Pass | A closed two-target set is not conflated with a sole definite target. | — |
| CI-G2 Evidence closure | Not applicable | No served answer uses this field. | Trace through FORMAT 7 later. |
| CI-G3 Evaluation integrity | Pass | The fixture and model projection do not read gold. | — |

## 7. Findings and principle verdicts

| ID | Finding | Principles · gate | Evidence and consequence | Correction |
|---|---|---|---|---|
| F01, deferred | Neither source target closure nor target normal return proves argument evaluation or enclosing exit fate. | DP-08, CI-06 · CI-G1 | Promoting these two fields directly to a positive `summary_flows` row could claim a path that never reaches the call result. | Require an exact input path, all argument outcomes, a bounded condition and return-exit proof. |

DP-01/02/03/04/05/08/09/11/13/14/15/18/19/21/22/23/24 and CI-01/02/04/06/07/10/12 are satisfied for this bounded projection. Graph, heuristic and serving principles are outside the slice. No SHOULD exception is requested.

## 8. Library-leverage ledger

| Capability | Bespoke scope | Built-in feature | Fit |
|---|---|---|---|
| Source/model context projection | Domain choice of fields | DataFusion join and Arrow schema | No duplicate target-count inference or custom evaluator. |

## 12. Decision

**Accept the attributed call-context fields.** Focused release Nextest `contracts_snapshot` and `pinned_identity_models_require_and_publish_their_real_formals` passed 2/2 on 2026-09-25 after reviewing and accepting the schema snapshot; the latter checks a sole closed typing target and rejects a forged assertion through the shared validator. Targeted Clippy is part of this slice's check. Integrated `just test-all`, `just pilot` and formatting are `not_run` until all Stage 3 functionality is implemented.
