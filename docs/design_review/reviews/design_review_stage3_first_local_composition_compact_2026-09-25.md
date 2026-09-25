# Stage 3 first local-call composition — compact review

## 1. Scope, purpose and coverage

| | |
|---|---|
| Subject | Acyclic one-argument local wrapper inheriting an unconditional callee value summary |
| Standard | Core 2.0, code-intelligence 1.0, library-context binding |
| Tier · purpose | Change · conformance |
| Reviewer · date | Codex · 2026-09-25 |
| Decision | Accept the bounded unconditional may-flow; defer general composition |

I inspected the exact raw call-result path, Pass B formal mapping, closed source target and
lexical callee joins, BDD/exit admission, callee-first SCC schedule, typed proof id, shared
writer/validator and focused local wrapper. I did not examine recursive transfer closure,
callee-local condition substitution, effects, callbacks, resources, exceptions or serving.

## 6. Gates

| Gate | Verdict | Evidence or scope reason | Required action |
|---|---|---|---|
| G1 Authority | Pass | ty owns the raw value path, Ruff the one-argument call, Pysa the closed target and Pass B the formal mapping; the callee's finite summary supplies transfer meaning. | — |
| G2 Fidelity | Pass, scoped | A call edge alone cannot enter: target/formal/value/exit identities agree and the callee path is unconditional. Open, multi-argument and recursive cases are withheld. | Add cross-scope condition mapping separately. |
| G3 Validity | Pass | Shared summary/step source equality reconstructs the entire composed path; generated evidence reference includes the callee summary id. | — |
| G4 Hidden behaviour | Pass, scoped | DataFusion supplies candidate joins, petgraph's component schedule supplies order, and the existing BDD checks caller→return implication. | Measure candidate volume on pilot. |
| G5 Consistency | Pass | Append-only `callee_summary` step and output version 64 mark the semantic migration. | — |
| G6 Transformation | Pass | Canonical path id includes the cited callee summary, preserving parallel callee proofs. | — |
| G7 Claims | Pass, scoped | The result is a may-flow under the model; recursive/conditional/depth-capped paths retain unknown boundaries. | Name depth caps specifically before full closure. |
| G8 Library leverage | Pass | Existing DataFusion, petgraph, BDD and id recipe own generic operations. | — |
| CI-G1 Fidelity | Pass, scoped | `local_wrapper → plain_identity` has a cited path, while existing unsupported call shapes remain withheld. | Add open-override and conditional callee fixtures. |
| CI-G2 Evidence closure | Not applicable | FORMAT 7 does not serve this path yet. | Follow `callee_summary` to a generation-pinned native answer. |
| CI-G3 Evaluation integrity | Pass | The fixture and source summaries do not use the FastMCP gold. | — |

## 7. Findings and principle verdicts

| ID | Finding | Principles · gate | Evidence and consequence | Correction |
|---|---|---|---|---|
| F01, deferred | A true callee condition is required because evaluation atoms are scoped to the callee. | DP-08, CI-06 · CI-G1 | Combining a conditional callee BDD without formal substitution could assert an impossible caller path; this implementation withholds it. | Add typed atom/path substitution with a bounded kernel proof. |
| F02, deferred | Recursive components and depth-eight refusals retain the generic `call_transfer` boundary. | DP-09, CI-10 · G7 | Unknown stays unknown, but a caller cannot distinguish missing transfer proof from a bound reached. | Add an explicit `budget_reached` composition boundary with work accounting. |
| F03, deferred | The target is restricted to one exact positional argument and a definite closed dispatch. | CI-07 · CI-G1 | Keyword, sibling-effect and open-target wrappers remain unknown. | Reuse ordered argument evidence and candidate-set coverage when widening. |

DP-01/02/03/04/05/08/09/11/13/14/15/18/19/21/22/23/24 and CI-01/02/04/06/07/10/12 are satisfied for the narrow admitted path. Heuristic and serving principles are outside this slice. No SHOULD exception is requested.

## 8. Library-leverage ledger

| Capability | Bespoke scope | Built-in feature | Fit |
|---|---|---|---|
| Local call/formal attribution | Domain proof preconditions | DataFusion joins over persisted calls, arguments and value facts | No second resolver. |
| Callee-first schedule | Acyclic admission policy | Pinned petgraph SCC topology | Stable order independent of provider rows. |
| Condition admission | Caller path implies exit; callee is unconditional | Existing bounded BDD kernel | No unsafe cross-scope atom conjunction. |

## 12. Decision

**Accept the first local wrapper may-flow after focused validation.** The selected release
fixture asserts a `local_wrapper` proof step citing `plain_identity` and runs shared tamper
checks; the first run passed that fixture and derivation snapshot while recording only the
expected new codebook/rule snapshots. After reviewing and accepting those snapshots, the
focused snapshot run, targeted release Clippy, ADR lint and diff check complete this slice.
`just fmt`, `just test-all`, `just pilot` and integrated evaluation remain `not_run` until all
functional Stage 3 scope is implemented.
