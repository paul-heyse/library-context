# Stage 3 bounded handler return path — compact design review

## 1. Scope, purpose and coverage

| | |
|---|---|
| Subject | `modeled_exception_return_none_paths`, ADR-0031, shared publication validation |
| Standard | Core 2.0, code-intelligence 1.0, library-context binding |
| Tier · purpose | Change · conformance |
| Reviewer · date | Codex · 2026-09-25 |
| Decision | Accept the conditional, candidate-local path only |

The producer composes a modeled potential raise with one first provable clause and a sole direct `return None`. It retains the source, model and handler fact IDs, both modalities, open dispatch and the handler region's approximation. Focused release Nextest passed 3/3 (175 skipped), including positive, withholding and missing-row validation cases; targeted release Clippy passed on 2026-09-25. Integrated tests, fresh pilot and serving were `not_run` by the operator's end-of-scope policy.

## 6. Gates

| Gate | Verdict | Evidence or scope reason | Required action |
|---|---|---|---|
| G1 Authority | Pass | The relation reads persisted modeled exceptions, handler candidate/walk, return site and syntax ancestry; it creates no new source authority. | — |
| G2 Semantic fidelity | Pass, scoped | First-match is explicitly conditional on a raise reaching the frame; nested `try`/`with` and `finally` withhold the path. | Do not promote this to a completed catch. |
| G3 Validity | Pass | Declared Arrow contract, FKs and shared full reconstruction reject a missing path row. | — |
| G4 Hidden behaviour | Pass | DataFusion composes existing facts without executing analyzed Python. | — |
| G5 Recovery | Pass, scoped | Version 46 migration and publication validation precede admission. | Run integrated publication gate at Stage 3 end. |
| G6 Transformation | Pass, scoped | Complete ancestor walk and direct-body shape preserve the bounded local derivation. | Compose general frames only with additional exit evidence. |
| G7 Claims | Pass, scoped | DESIGN and ADR-0031 label the row candidate-local and conditional, with open modalities. | Preserve that scope in L3 and serving. |
| G8 Library leverage | Pass | Existing DataFusion recursive CTE, Arrow schema and validator machinery suffice; no bespoke graph interpreter is added. | — |
| CI-G1 Fidelity | Pass, scoped | Raised class, handler class relationship, clause order and `return None` remain separate citations. | — |
| CI-G2 Evidence closure | N.a. | No served claim consumes this relation yet. | Check FORMAT 7 closure. |
| CI-G3 Evaluation integrity | Pass | Fixture cases do not use FastMCP gold families as compiler inputs. | — |

## 7. Findings and principle verdicts

| ID | Finding | Principles · gate | Evidence and consequence | Correction |
|---|---|---|---|---|
| F01, deferred to L2/L3 | A modeled potential raise is not an observed exception, and this path does not establish other exits or normal operation completion. | DP-08, CI-06 · G7 | The row retains `target_modality`, `model_modality`, unresolved dispatch and region approximation. | Aggregate all candidates and exits before a verdict; otherwise `unknown`. |
| F02, deferred to L2 | Nested frames and finalizers are withheld. | DP-11, CI-04 · G6 | The direct ancestry rule excludes `nested_tries`, `with_intervening` and `finalizer_changes_return` in the focused fixture. | Add cited propagation/completion facts only if needed by registered questions. |

Within this bounded change DP-01/02/03/04/05/07/08/11/13/14/15/18/19/21/22/23/24 and CI-01/02/03/04/06/07/08/10/12 are satisfied. Other principles impose no new local obligation; summary, oracle and serving obligations remain open. No SHOULD exception is requested.

## 8. Library-leverage ledger

| Capability | Bespoke code | Built-in feature | Fit and limit | Recommendation |
|---|---|---|---|---|
| Bounded ancestry | Direct-path admission rule | DataFusion recursive CTE and anti-joins | Reuses the existing walk and checks inner control frames without custom traversal. | Keep SQL relation; maintain explicit coverage row. |
| Publication proof | None beyond row declaration | Arrow contracts, DataFusion reconstruction | Full-row equality rejects dropped rows. | Reuse for later L2 relations. |

## 12. Decision

**Accept the candidate-local conditional path.** Focused release Nextest passed 3/3 and targeted release Clippy passed on 2026-09-25. F01 and F02 prohibit treating this row as a served catch, escape or normal-return verdict. Integrated `just test-all`, `just pilot`, structured evaluation and formatting remain `not_run`.
