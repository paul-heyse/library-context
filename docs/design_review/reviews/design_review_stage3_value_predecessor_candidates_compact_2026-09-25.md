# Stage 3 value predecessor candidates — compact design review

## 1. Scope, purpose and coverage

| | |
|---|---|
| Subject | `value_flow_predecessor_candidates` and ADR-0028's inherited-call boundary |
| Standard | Core 2.0, code-intelligence 1.0, library-context binding |
| Tier · purpose | Change · conformance |
| Reviewer · date | Codex · 2026-09-25 |
| Decision | Accept as a cited candidate edge only |

The relation follows a raw successor use through one pinned `flow_reaching` fact and `flow_definitions` value span to an earlier raw value fact with the same parameter origin. It retains the three condition identities, both raw approximation flags, reaching approximation and loop status. A focused `indirect_identity` fixture has the expected one-assignment edge and a missing-row publication tamper is rejected. Final release Nextest passed 3/3 (175 skipped) and targeted release Clippy passed on 2026-09-25; integrated pilot and serving remain `not_run` by operator direction.

## 6. Gates

| Gate | Verdict | Evidence or scope reason | Required action |
|---|---|---|---|
| G1 Authority | Pass | Exact provider raw fact, use, reaching and definition facts are carried. | — |
| G2 Semantic fidelity | Pass, scoped | The edge says candidate predecessor, not feasible/completed transfer. | Check condition compatibility and path coverage in L3. |
| G3 Validity | Pass, scoped | Arrow contract, generated FKs for provider facts and full reconstruction reject dropped rows. | — |
| G4 Hidden behaviour | Pass | DataFusion equijoins persisted facts; analyzed Python is not executed. | — |
| G5 Recovery | Pass, scoped | Version 48 separates this schema and validation runs before publication. | Run integrated publication gate at Stage 3 end. |
| G6 Transformation | Pass, scoped | Source origin, sink callable, raw fact ids and reached definition survive the join. | Retain all candidates through SCC composition. |
| G7 Claims | Pass, scoped | ADR/DESIGN mark condition compatibility, predecessor uniqueness and completion unknown. | Do not serve an edge as a transfer. |
| G8 Library leverage | Pass | Relational join is handled by DataFusion; no custom source walker. | Use existing bounded BDD for subsequent condition checks. |
| CI-G1 Fidelity | Pass, scoped | The edge joins same source parameter and callable without borrowing a sibling raw path. | — |
| CI-G2 Evidence closure | N.a. | No served claim consumes it yet. | Check FORMAT 7 closure. |
| CI-G3 Evaluation integrity | Pass | Fixture is not a compiler-input gold family. | — |

## 7. Findings and principle verdicts

| ID | Finding | Principles · gate | Evidence and consequence | Correction |
|---|---|---|---|---|
| F01, required before L3 | Flow-analysis recomposed condition ids are not necessarily in provider `conditions`. | DP-08, CI-04 · G6 | The raw reaching condition has a provider FK; successor/predecessor composition conditions deliberately do not. | Persist their structural BDD closures and validate them before summary use. |
| F02, required before verdicts | One predecessor edge does not prove compatible conditions, exclusive reaching or call completion. | DP-11, CI-06 · G7 | The row retains independent roots and loop/approximation flags. | Use bounded BDD composition and candidate-set closure, otherwise `unknown`. |

DP-01/02/03/04/05/07/08/11/13/14/15/18/19/21/22/23/24 and CI-01/02/03/04/06/07/08/10/12 are satisfied within the candidate-edge scope. Other principles add no new local obligation. No SHOULD exception is requested.

## 8. Library-leverage ledger

| Capability | Bespoke code | Built-in feature | Fit and limit | Recommendation |
|---|---|---|---|---|
| Reaching predecessor | Domain same-origin requirement | DataFusion equijoins over flow/definition facts | Preserves all candidates without a second Python interpreter. | Keep this relation as evidence input. |
| Condition compatibility | None in this slice | Existing bounded BDD kernel | The provider roots are persisted, but recomposed analysis roots are not yet persisted. | Materialize the analysis root/node catalog before L3. |

## 12. Decision

**Accept the source-preserving candidate edge only.** Final focused release Nextest passed 3/3 and targeted Clippy passed on 2026-09-25. Integrated `just test-all`, `just pilot`, structured evaluation and formatting remain `not_run` until the complete Stage 3 scope is implemented.
