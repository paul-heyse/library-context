# Stage 3 summary boundaries — compact design review

## 1. Scope, purpose and coverage

| | |
|---|---|
| Subject | `summary_boundaries` for parameter-origin raw return paths |
| Standard | Core 2.0, code-intelligence 1.0, library-context binding |
| Tier · purpose | Change · conformance |
| Reviewer · date | Codex · 2026-09-25 |
| Decision | Accept explicit unknown coverage |

The direct summary base proves only a narrow subset of raw return paths. This relation retains every other same-callable parameter-origin return fact with its recomposed condition and a reason: crossed calls are `call_transfer`, other unsupported control/execution shapes are `unsupported_control_flow`. Contributions are grouped by fact, parameter and condition so a proved route cannot erase an unrelated unproved route. It is coverage for missing positive summaries, never negative transfer evidence. On 2026-09-25 the focused Nextest selection (`contracts_snapshot`, `rules_snapshot`, `pinned_identity_models_require_and_publish_their_real_formals`) passed 3/3; targeted release-profile Clippy, `just adr lint` and `git diff --check` passed.

## 6. Gates

| Gate | Verdict | Evidence or scope reason | Required action |
|---|---|---|---|
| G1 Authority | Pass | Each row retains the raw value fact, callable, formal and condition id. | — |
| G2 Semantic fidelity | Pass, scoped | Crossed-call uncertainty is distinct from control/execution uncertainty. | Add more specific reasons as L2/L3 proves them. |
| G3 Validity | Pass, scoped | Arrow contract, references and shared reconstruction cover all rows. | Run integrated publication gate at scope end. |
| G4 Hidden behaviour | Pass | One declared DataFusion relation computes the complement of the finite producer. | — |
| G5 Recovery | Pass, scoped | Output version 55 marks the schema migration. | Qualify the pilot at integrated end. |
| G6 Transformation | Pass | Unmerged contribution flags survive as aggregate boundary evidence. | Reclassify only with cited completion proof. |
| G7 Claims | Pass | Unknown coverage cannot be interpreted as a negative flow. | — |
| G8 Library leverage | Pass | DataFusion grouping and anti-coverage use persisted relations. | — |
| CI-G1 Fidelity | Pass | A modeled candidate does not silently close the call. | — |
| CI-G2 Evidence closure | N.a. | No served query consumes these boundaries yet. | Check FORMAT 7. |
| CI-G3 Evaluation integrity | Pass | Source fixture is independent of gold capability families. | — |

## 7. Findings and principle verdicts

| ID | Finding | Principles · gate | Evidence and consequence | Correction |
|---|---|---|---|---|
| F01, deferred to L3 | These boundaries cover only parameter-origin return value facts. | CI-06, CI-07 · G7 | Field origins, effects, handlers and call-target closure need separate summary coverage. | Extend the domain before any exhaustive operation claim. |

DP-01/02/03/04/05/07/08/11/13/14/15/18/19/21/22/23/24 and CI-01/02/03/04/06/07/08/10/12 are satisfied within this coverage scope. No SHOULD exception is requested.

## 8. Library-leverage ledger

| Capability | Bespoke code | Built-in feature | Fit and limit | Recommendation |
|---|---|---|---|---|
| Summary complement | Domain proof predicate and boundary taxonomy | DataFusion grouped aggregate and anti-coverage join | Preserves open paths without an independent source parser. | Keep as L3 coverage input. |

## 12. Decision

**Accept after focused verification.** Integrated `just test-all`, `just pilot`, structured evaluation and formatting remain `not_run` until full Stage 3 functionality is ready.
