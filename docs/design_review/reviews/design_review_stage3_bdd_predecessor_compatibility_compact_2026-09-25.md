# Stage 3 BDD predecessor compatibility — compact review

## 1. Scope, purpose and coverage

| | |
|---|---|
| Subject | ADR-0039, the attributed preceding-call relation and direct-return admission |
| Standard | Core 2.0, code-intelligence 1.0, library-context binding |
| Tier · purpose | Change · conformance |
| Reviewer · date | Codex · 2026-09-25 |
| Decision | Accept the bounded refinement; retain predecessor normal completion as an open requirement |

This review examined the new DataFusion relation, BDD composition, direct summary producer,
analyzed fixture and shared publication validation. It does not certify general predecessor
execution, non-call effects or the unfinished Stage 3 product.

## 6. Gates

| Gate | Verdict | Evidence or scope reason | Required action |
|---|---|---|---|
| G1 Authority | Pass | Ruff call spans, ty statement regions and persisted BDD IDs are from the same analyzed source. | — |
| G2 Fidelity | Pass, scoped | An earlier call is ignored only on a proved incompatible branch; missing, approximate or capped work withholds the positive. | Prove compatible predecessors' normal outcome later. |
| G3 Validity | Pass, scoped | The shared validator reconstructs published finite summaries and boundaries; focused compile tests passed. | — |
| G4 Hidden behaviour | Pass | Fixture source is analyzed, not executed by the compiler. | — |
| G5 Consistency | Pass | Withheld positives retain `summary_boundaries` unknown rows. | — |
| G6 Transformation | Pass, scoped | DataFusion selects the narrowest enclosing region; the BDD kernel decides disjointness. No input-order rule controls the result. | — |
| G7 Claims | Pass | ADR-0039 calls this a compatibility screen, not completed-path proof. | — |
| G8 Library leverage | Pass | Existing DataFusion, ty region and biodivine-lib-bdd owners perform their declared operations. | — |
| CI-G1 Fidelity | Pass, scoped | Alternate-branch and base returns are admitted; unconditional prior calls are withheld. | Cover other predecessor effects later. |
| CI-G2 Evidence closure | Pass for emitted paths | The direct proof retains its raw value, condition and exit citations; incompatible calls need no completion citation. | — |
| CI-G3 Evaluation integrity | Pass | Fixture and oracle controls are outside the model catalog and compiler inputs. | — |

## 7. Findings and principle verdicts

| ID | Finding | Principles · gate | Evidence and consequence | Disposition |
|---|---|---|---|---|
| F01, deferred | Compatible calls still have no normal-outcome witness. | CI-06, CI-07, DP-08 · CI-G1 | The screen correctly withholds such direct paths even when a callee would return, losing precision. | Reopen with the Stage 3 predecessor execution relation and its SCC worklist. |
| F02, deferred | Non-call syntax can fail or execute user code before return. | CI-06, DP-22 · CI-G1 | Operators, descriptors, iterators and exceptional exits are outside this call-only screen; their positive direct paths need a typed execution premise. | Reopen when the source-path witness covers these forms. |

DP-01/02/03/04/07/08/11/13/15/19/21/22/23 and CI-01/02/03/04/06/07/08/10/11/12 are
satisfied within this bounded call-screen scope. General execution fidelity is unresolved at
F01–F02; no missing summary is a negative. Graph-wide composition and serving are outside
this review.

## 8. Library leverage and alternatives

| Operation | Chosen owner | Alternative and assessment |
|---|---|---|
| Attribute a call to a statement region | DataFusion join over Ruff and ty facts | A second AST walk would duplicate source attribution. |
| Prove path disjointness | Bounded biodivine-lib-bdd conjunction | Byte order alone over-withholds; string predicates cannot establish equivalence. |
| Prove a compatible predecessor completes | Future source/model/SCC witness | Treating compatibility as completion would promote a self-recursive false positive. |

## 12. Decision

**Accept the bounded refinement.** On 2026-09-25, the focused analyzed return fixture passed
with a source call in the incompatible branch, a finite return before recursion and two
withheld preceding-call cases. The focused modeled-return fixture also passed. Full repository
and pilot acceptance is reported separately in the Stage 3 checkpoint.
