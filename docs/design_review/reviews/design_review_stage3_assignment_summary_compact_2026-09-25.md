# Stage 3 assignment-to-return summary — compact review

## 1. Scope, purpose and coverage

| | |
|---|---|
| Subject | Two-hop pinned identity-model assignment result reaching a direct return |
| Standard | Core 2.0, code-intelligence 1.0, library-context binding |
| Tier · purpose | Change · conformance |
| Reviewer · date | Codex · 2026-09-25 |
| Decision | Accept unique-reaching finite path; preserve ambiguous paths as unknown |

I inspected the predecessor and compatibility sources, unique-reaching guard, four BDD
implications, shared call-proof builder, typed step identity, boundary complement and focused
fixture. Recursive summaries, nested control, effects, callback/resource/exception fates,
serving and pilot performance remain outside this review.

## 6. Gates

| Gate | Verdict | Evidence or scope reason | Required action |
|---|---|---|---|
| G1 Authority | Pass | Provider reaching rows, source/model call facts and the structural BDD retain separate identities. | — |
| G2 Fidelity | Pass, scoped | Exactly one reaching row, an admitted predecessor, normal modeled call and direct synchronous return are required. | Preserve condition-specific alternatives before widening. |
| G3 Validity | Pass | Shared writer/validator reconstruct both summary rows and ordered proof steps. | — |
| G4 Hidden behaviour | Pass, scoped | DataFusion groups reaching rows; the existing bounded BDD checks implication; a shared Rust call-proof builder avoids two meanings of argument completion. | Measure on pilot at integrated end. |
| G5 Consistency | Pass | Output version 62 and append-only step codes mark the semantic migration. | — |
| G6 Transformation | Pass | The path id includes call, model, reaching-definition and returned-value evidence in order. | Keep this identity when SCC composition lands. |
| G7 Claims | Pass, scoped | An ambiguous assignment is withheld, while its raw return retains an unknown boundary unless another path is proved. | Do not treat silence as a negative. |
| G8 Library leverage | Pass | DataFusion relation and window/cardinality operations plus BDD implication and the existing id recipe fit the exact operations. | — |
| CI-G1 Fidelity | Pass, scoped | Focused unique and competing-definition cases diverge. | Challenge condition-disjoint definitions in the later oracle. |
| CI-G2 Evidence closure | Not applicable | FORMAT 7 does not serve the new path yet. | Carry ordered steps to the native executor. |
| CI-G3 Evaluation integrity | Pass | Local fixture does not feed gold into compilation. | — |

## 7. Findings and principle verdicts

| ID | Finding | Principles · gate | Evidence and consequence | Correction |
|---|---|---|---|---|
| F01, deferred | Requiring one reaching row across all conditions withholds paths whose definitions are provably disjoint. | DP-09, CI-06 · CI-G1 | Some real conditional transfers remain `unknown`; the rule does not manufacture a false positive. | Add bounded condition partitioning with a path-specific uniqueness proof. |
| F02, deferred | The producer is limited to one local assignment and a direct return. | DP-08, CI-07 · CI-G1 | Further call chains, nested frames and side effects cannot be concluded from this row. | Compose finite summaries with explicit SCC/budget boundaries. |

DP-01/02/03/04/05/08/09/11/13/14/15/18/19/21/22/23/24 and CI-01/02/04/06/07/10/12 are satisfied for the narrow two-hop producer. Graph, heuristic and serving principles are outside this slice. No SHOULD exception is requested.

## 8. Library-leverage ledger

| Capability | Bespoke scope | Built-in feature | Fit |
|---|---|---|---|
| Reaching uniqueness | Domain choice to demand exactly one row | DataFusion grouped `count(*)` | Keeps candidate joins declarative. |
| Condition admission | The required predecessor→reaching→return implication chain | Bounded `Diagram::implies` | No string condition or DNF approximation. |
| Ordered identity | Source/model/definition/return step types | Existing `lctx_id` recipe | Parallel evidence yields distinct ids. |

## 12. Decision

**Accept the unique two-hop modeled may-flow after focused validation.**
`INSTA_UPDATE=no cargo nextest run --release -p cpg-core -E
'test(pinned_identity_models_require_and_publish_their_real_formals)' --no-tests=pass
--no-fail-fast` passed 1/1 on 2026-09-25. The fixture checks the positive assignment,
competing-definition withholding, retained dynamic boundaries and shared publication tamper
checks. The codebook/rule snapshots were reviewed before acceptance; the focused snapshot run
passed 3/4, with its sole failure the old fixture expectation that was then updated and rerun.
Targeted release Clippy, ADR lint and diff checks passed.
Formatting, full integrated testing, pilot and structured evaluation remain `not_run` until
the entire functional Stage 3 scope is implemented.
