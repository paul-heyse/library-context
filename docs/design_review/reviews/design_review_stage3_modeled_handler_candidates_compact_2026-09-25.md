# Stage 3 modeled handler candidates — compact design review

## 1. Scope, purpose and coverage

| | |
|---|---|
| Subject | Modeled exception source → enclosing `try` clauses, class comparison, and walk coverage |
| Standard | Core 2.0, code-intelligence 1.0, library-context binding |
| Tier · purpose | Change · conformance |
| Reviewer · date | Codex · 2026-09-25 |
| Decision | Accept as a candidate relation; completed handler fates remain open |

I inspected the relation SQL, Arrow and codebook contracts, publication reconstruction, and the focused `model_handler_shapes` cases. The walk follows `syntax_nodes.parent_node_id` only inside the source call's innermost function. It tests a call inside a `try` body, a call before the body, a nested function, and two nested frames. I did not inspect a full FastMCP generation, handler completion, subclass inheritance or an integrated pilot; no served catch claim is in scope.

## 6. Gates

| Gate | Verdict | Evidence or scope reason | Required action |
|---|---|---|---|
| G1 Authority | Pass | Ruff syntax, pinned handler class and authored model class retain distinct source identities. | — |
| G2 Fidelity | Pass | Bare, same class, different pinned class and unknown type are distinct codebook states; nested frames retain their `try`/clause IDs. | — |
| G3 Validity | Pass | Schema, reference rules and shared source-equality validator reject altered rows. | — |
| G4 Hidden behaviour | Pass | The walk uses pinned stored syntax and model facts; no source execution or ambient import. | — |
| G5 Consistency | Pass | One walk-coverage row per modeled raise records missing syntax or budget exhaustion; validation precedes snapshot publication. | — |
| G6 Transformation | Pass | One bounded recursive CTE definition is shared by candidate and coverage relations; no absence becomes a negative fate. | — |
| G7 Claims | Pass, scoped | DESIGN names the result a candidate and keeps catch/completion open. | Keep future consumers gated on walk coverage and handler proof. |
| G8 Library leverage | Pass | Pinned DataFusion recursive CTE performs the ancestry traversal and joins; Arrow declares the output. | — |
| CI-G1 Fidelity | Pass | Pysa candidate, synthetic model action and authored handler stay separately cited; different pinned classes remain `class_relation_unknown`. | — |
| CI-G2 Evidence closure | Not applicable | No served claim consumes this relation yet. | Trace a FORMAT 7 claim when one does. |
| CI-G3 Evaluation integrity | Pass | The fixture is a source shape; no gold or evaluation answer enters the compiler. | — |

## 7. Findings and principle verdicts

| ID | Finding | Principles · gate | Evidence and consequence | Correction |
|---|---|---|---|---|
| F01, deferred | A same-class or bare candidate is not a proven catch. | CI-06, DP-08 · CI-G1 | The relation does not decide earlier clauses, normal/exceptional call outcome, subclass relations, or handler control flow. Promoting it would serve a false fate. | L2 must decide those independently, and L3 must keep unresolved ones as boundaries. |

The initial candidate-only design had an unrecorded 128-edge cap. The added `modeled_exception_handler_walks` relation resolves that finding for this slice: every modeled raise has a complete or explicitly bounded walk state, reconstructed at publication. DP-01/02/03/04/05/07/08/09/11/13/14/15/18/19/20/21/22/23/24 and CI-01/02/03/04/06/07/08/10/12 are satisfied within the candidate scope. DP-12's future recursive summary, CI-11 and CI-13's serving obligations are unresolved for Stage 3 as a whole, but are not claims of this slice. CI-05/09 are inapplicable because no graph or heuristic is changed. No SHOULD exception is requested.

## 8. Library-leverage ledger

| Capability | Own code | Built-in feature | Fit and limit |
|---|---|---|---|
| Syntax ancestry | Domain predicate for `try` body and function boundary | DataFusion 55.1.0 `WITH RECURSIVE`, joins and aggregates | Focused executed fixture confirms the pinned query; 128 edges are explicitly reported. |
| Typed identity/status | Arrow table and codebook declarations | Existing schema macro and codebook registry | Append-only codebook snapshot and explicit output migration. |
| Handler decision | Not implemented | Pinned context class facts plus condition/flow kernels | Requires a domain transfer rule; generic reachability alone would be wrong. |

## 12. Decision

**Accept the structural candidate and coverage relations.** The focused release Nextest selection passed 5/5 on 2026-09-25 after reviewing the schema, rule and codebook snapshots; the modeled-handler fixture includes exact, bare, unresolved class/type, outside-body, nested-function and nested-`try` cases. Release Clippy, formatting and diff checks are recorded with the slice commit. `just test-all`, a fresh pilot, structured evaluation and clean-wheel serving remain `not_run` until all functional Stage 3 scope is assembled.
