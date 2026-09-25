# Stage 3 `typing.assert_type` model — compact design review

## 1. Scope, purpose and coverage

| | |
|---|---|
| Subject | Pinned `typing.assert_type` catalog rule, exact source argument binding, narrow CrossHair oracle |
| Standard | Core 2.0, code-intelligence 1.0, library-context binding |
| Tier · purpose | Change · conformance |
| Reviewer · date | Codex · 2026-09-25 |
| Decision | Accept as a candidate identity rule, without a summary verdict |

I inspected CPython's documented runtime contract, the committed typed catalog, the compiler's target/formal binding path, the focused `model_shapes` fixture, and the isolated CrossHair control and identity comparison. The focused release Nextest selection passed 1/1 on 2026-09-25. The generic type domain, composed summary, pilot and served claim were not tested.

## 6. Gates

| Gate | Verdict | Evidence or scope reason | Required action |
|---|---|---|---|
| G1 Authority | Pass | One catalog rule names `typing.assert_type`; model target and formal bind to pinned context facts. | — |
| G2 Fidelity | Pass | The model's identity input is typed `Parameter[val]`, not a spelling-matched source use; the application cites an exact argument. | — |
| G3 Validity | Pass | Existing catalog parser, pinned formal checks and shared model/site validators apply; the focused fixture compiled and validated. | — |
| G4 Hidden behaviour | Pass | The authored rule is committed and included in the compiler digest; no ambient import determines the model. | — |
| G5 Consistency | Pass | The catalog is bound before publication and its rows are reconstructed at validation. | — |
| G6 Transformation | Pass, scoped | The site row preserves the call result and input argument facts; summary propagation remains unimplemented. | — |
| G7 Claims | Pass, scoped | CrossHair exhausted the `int` specialization with an opposite-result control; the design does not claim generic equivalence from that probe. | — |
| G8 Library leverage | Pass | Serde's tagged parser, DataFusion's binding joins and CrossHair's differential oracle are reused. | — |
| CI-G1 Fidelity | Pass | A synthetic model assertion remains distinct from observed source behavior and a completed transfer. | — |
| CI-G2 Evidence closure | N.a. | No served claim consumes this rule yet. | Check the FORMAT 7 evidence chain. |
| CI-G3 Evaluation integrity | Pass | The isolated oracle executes a minimal wrapper, not the FastMCP gold reference. | — |

## 7. Findings and principle verdicts

| ID | Finding | Principles · gate | Evidence and consequence | Correction |
|---|---|---|---|---|
| F01, deferred | The identity rule has no composed call-path consumer yet. | DP-08, CI-06 · G6 | `modeled_transfer_sites` identifies a candidate input/result pair only. Serving it as an operation-level transfer would overclaim. | Compose through exact `flow_value_call_links`, conditions and target completeness in L3. |

Within this model-rule scope, DP-01/02/03/04/05/07/08/09/11/13/14/15/18/19/21/22/23/24 and CI-01/02/03/04/06/07/10/12 are satisfied. Recursive summary, serving, graph and heuristic principles add no in-scope claim yet. No SHOULD exception is requested.

## 8. Library-leverage ledger

| Capability | Bespoke code | Library or built-in | Fit and limit | Recommendation |
|---|---|---|---|---|
| Pure identity oracle | None added | CrossHair 0.0.110 `diffbehavior` | Wrong control found a witness; the `int` identity comparison exhausted paths. | Extend representative pure domains, without treating budget silence as proof. |
| Model binding | One new TOML declaration | Existing Serde typed AST and DataFusion pinned-context joins | Focused test confirms exact `val` binding. | Keep model meaning in catalog data. |

## 12. Decision

**Accept the pinned candidate identity model.** The focused release Nextest test passed 1/1 and CrossHair's narrow differential probe exhausted paths on 2026-09-25. L3 summary use, integrated repository and pilot gates, structured evaluation, and formatting remain `not_run` under the operator's end-of-scope policy.
