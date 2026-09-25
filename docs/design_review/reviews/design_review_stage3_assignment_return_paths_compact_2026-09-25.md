# Stage 3 assignment-to-return model paths — compact design review

## 1. Scope, purpose and coverage

| | |
|---|---|
| Subject | `modeled_assignment_return_paths` and its source-model join |
| Standard | Core 2.0, code-intelligence 1.0, library-context binding |
| Tier · purpose | Change · conformance |
| Reviewer · date | Codex · 2026-09-25 |
| Decision | Accept the cited two-step candidate |

The relation joins a pinned whole-assignment model result to one raw identity return through a cited provider reaching definition. It matches the same predecessor use and recomposed condition as the model step, retains the Pysa candidate and model rule separately, and carries the predecessor compatibility decision and approximation/open-target fields. It is a path candidate; no model-call completion or whole-operation verdict follows. On 2026-09-25 the focused Nextest selection (`contracts_snapshot`, `rules_snapshot`, `pinned_identity_models_require_and_publish_their_real_formals`) passed 3/3; targeted release-profile Clippy, `just adr lint` and `git diff --check` passed.

## 6. Gates

| Gate | Verdict | Evidence or scope reason | Required action |
|---|---|---|---|
| G1 Authority | Pass, scoped | Both raw value facts, reaching fact, model call, Pysa candidate and rule are retained. | — |
| G2 Semantic fidelity | Pass, scoped | The successor raw fact is an identity return; model step occupies the whole definition value. | Prove normal completion before promoting to a summary. |
| G3 Validity | Pass, scoped | Arrow contract, references and shared reconstruction cover the relation. | Run integrated publication gate at scope end. |
| G4 Hidden behaviour | Pass | One declared DataFusion relation performs the joins. | — |
| G5 Recovery | Pass, scoped | Output version 53 marks the schema migration. | Qualify the pilot at integrated end. |
| G6 Transformation | Pass | Separate condition roots and approximation flags survive the join. | Compose them through L3. |
| G7 Claims | Pass, scoped | The row is explicitly a may-path candidate, not an established return fate. | — |
| G8 Library leverage | Pass | DataFusion typed joins reuse the existing BDD compatibility result. | — |
| CI-G1 Fidelity | Pass | No sink-span or source-name guess connects the facts. | — |
| CI-G2 Evidence closure | N.a. | No served claim consumes the candidate. | Check FORMAT 7. |
| CI-G3 Evaluation integrity | Pass | Source fixture is independent of gold capability families. | — |

## 7. Findings and principle verdicts

| ID | Finding | Principles · gate | Evidence and consequence | Correction |
|---|---|---|---|---|
| F01, deferred to L3 | Compatibility is not selection or completion. | DP-08, CI-06 · G7 | A true BDD result admits a path under the atoms; model dispatch, normal completion and enclosing exit actions remain open. | Emit a positive summary only after those proofs; otherwise name the boundary. |

DP-01/02/03/04/05/07/08/11/13/14/15/18/19/21/22/23/24 and CI-01/02/03/04/06/07/08/10/12 are satisfied within this candidate-path scope. No SHOULD exception is requested.

## 8. Library-leverage ledger

| Capability | Bespoke code | Built-in feature | Fit and limit | Recommendation |
|---|---|---|---|---|
| Cited path join | Identity-return and same-source requirements | DataFusion equijoins over persisted relations | Exact candidate provenance without reparsing source; BDD result reused. | Retain as an L3 input. |

## 12. Decision

**Accept after focused verification.** Integrated `just test-all`, `just pilot`, structured evaluation and formatting remain `not_run` until full Stage 3 functionality is ready.
