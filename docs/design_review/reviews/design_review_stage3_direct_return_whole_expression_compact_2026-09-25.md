# Stage 3 whole-expression model return — compact design review

## 1. Scope, purpose and coverage

| | |
|---|---|
| Subject | `modeled_direct_return_transfers` call/sink span equality |
| Standard | Core 2.0, code-intelligence 1.0, library-context binding |
| Tier · purpose | Change · conformance |
| Reviewer · date | Codex · 2026-09-25 |
| Decision | Accept the tightened direct bridge |

The prior one-call rule matched an exact modeled call and argument but did not require the call to be the whole returned expression. A single call nested under `or`, an arithmetic operator or other outer expression could acquire a modeled identity candidate for the entire result. The corrected DataFusion join requires the call span to equal the raw return sink span. The focused fixture adds `computed_identity`, which is withheld while the direct `cast` and `assert_type` cases remain. Focused release Nextest passed 1/1 (117 skipped) and targeted release Clippy passed on 2026-09-25.

## 6. Gates

| Gate | Verdict | Evidence or scope reason | Required action |
|---|---|---|---|
| G1 Authority | Pass | Both spans come from provider raw value and call-step facts. | — |
| G2 Semantic fidelity | Pass, scoped | An outer computation can no longer borrow the inner call's return identity. | Continue withholding assigned/nested paths. |
| G3 Validity | Pass, scoped | Existing shared reconstruction uses the amended relation. | — |
| G4 Hidden behaviour | Pass | No source text interpretation; exact persisted coordinates only. | — |
| G5 Recovery | Pass | Output version 50 separates the rule change. | Run integrated gate at Stage 3 end. |
| G6 Transformation | Pass | The stronger equality relates the same raw fact and source call. | — |
| G7 Claims | Pass, scoped | Still a candidate path, not completed flow. | — |
| G8 Library leverage | Pass | DataFusion equijoin predicate, no bespoke syntax walker. | — |
| CI-G1 Fidelity | Pass | Source expression nesting is respected. | — |
| CI-G2 Evidence closure | N.a. | No served claim yet. | Verify in FORMAT 7. |
| CI-G3 Evaluation integrity | Pass | Focused source fixture remains independent of gold. | — |

## 7. Findings and principle verdicts

| ID | Finding | Principles · gate | Evidence and consequence | Correction |
|---|---|---|---|---|
| F01, deferred to L3 | An exact whole-expression call still may not complete, and target/model modality remains open. | DP-08, CI-06 · G7 | The row preserves both modalities and unresolved remainder. | Aggregate exits and dispatch before a verdict. |

DP-01/02/03/04/05/07/08/11/13/14/15/18/19/21/22/23/24 and CI-01/02/03/04/06/07/08/10/12 are satisfied within this direct-candidate scope. No SHOULD exception is requested.

## 8. Library-leverage ledger

| Capability | Bespoke code | Built-in feature | Fit and limit | Recommendation |
|---|---|---|---|---|
| Full-result check | None | DataFusion equality over raw value/call spans | Exact enough for the source-local direct shape; cannot prove runtime completion. | Keep as a positive admission predicate only. |

## 12. Decision

**Accept the tightened candidate bridge.** Focused release Nextest passed 1/1 and targeted Clippy passed on 2026-09-25. Integrated `just test-all`, `just pilot`, structured evaluation and formatting remain `not_run` until all Stage 3 functional work is implemented.
