# Stage 3 recursive summary boundary — compact review

## 1. Scope, purpose and coverage

| | |
|---|---|
| Subject | Finite value-summary admission for functions in attributed recursive SCCs |
| Standard | Core 2.0, code-intelligence 1.0, library-context binding |
| Tier · purpose | Change · conformance |
| Reviewer · date | Codex · 2026-09-25 |
| Decision | Accept as an interim unknown boundary; full SCC execution proof remains required |

The existing petgraph SCC schedule already marked recursive functions, while the acyclic local
call producer withheld them. Direct and modeled return producers did not use that status: a
syntactically simple `return value` after an unconditional self-call could become a positive
finite flow. This review follows the published component row into every finite producer and
the resulting boundary. It does not claim a general normal-completion proof for preceding
statements or effects.

## 6. Gates

| Gate | Verdict | Evidence or scope reason | Required action |
|---|---|---|---|
| G1 Authority | Pass | `summary_components` is the single attributed call-graph/SCC authority; the producer reads its published rows. | — |
| G2 Fidelity | Pass, conservative | Candidate/open call arcs can only enlarge the recursive set; no SCC membership becomes a positive transfer. | — |
| G3 Validity | Pass | The existing shared validator reconstructs both components and finite summaries; focused compile includes publication validation. | — |
| G4 Hidden behaviour | Pass | The compiler never executes the analyzed recursive source. | — |
| G5 Consistency | Pass | Withheld paths retain `summary_boundaries`; no incomplete positive is silently published. | — |
| G6 Transformation | Pass, scoped | SCC members are canonical IDs; positive paths remain suppressed independent of input row order. | Add bounded worklist before expanding. |
| G7 Claims | Pass, scoped | Documentation labels this a conservative interim boundary. | Do not present missing summaries as negative results. |
| G8 Library leverage | Pass | Existing petgraph `tarjan_scc` and deterministic condensation are reused, with a small `HashSet` admission lookup. | — |
| CI-G1 Fidelity | Pass, scoped | A self-recursive predecessor no longer licenses a finite completed value path from region truth alone. | Prove other predecessor normal exits later. |
| CI-G2 Evidence closure | Pass for emitted paths | The excluded recursive path has an explicit source-fact boundary rather than a forged proof. | — |
| CI-G3 Evaluation integrity | Pass | The recursive fixture is analyzed input, not gold or an oracle fed into the compiler. | — |

## 7. Findings and principle verdicts

| ID | Finding | Principles · gate | Evidence and consequence | Correction |
|---|---|---|---|---|
| F01, deferred | SCC-wide withholding loses a terminating base branch. | CI-06, DP-11 · G6 | `summaries.rs` filters each seed by its owning component; a branch that returns before recursion is also unknown. | The bounded per-path SCC worklist must admit a cited finite base and carry path conditions. |
| F02, deferred | Nonrecursive predecessor calls can also fail to complete before a simple return. | CI-06, CI-07 · CI-G1 | Region truth and return syntax do not prove normal completion of every preceding expression; this slice only closes the observed recursive form. | Compose statement/call normal-outcome witnesses or retain a boundary where they are missing. |

Within this scoped change, DP-01/02/03/04/07/08/11/13/15/19/21/22/23 and CI-01/02/03/04/05/06/07/08/10/11/12 are satisfied by the existing component contract, typed boundary and publication reconstruction. Full Stage 3 semantic fidelity remains unresolved at F01–F02. Other graph algorithms, retrieval and serving projections were outside this review.

## 8. Library leverage

| Capability | Current owner | Alternative | Assessment |
|---|---|---|---|
| Recursive component detection | petgraph `tarjan_scc` through `lctx-analytics` | Handwritten DFS | Existing tested component schedule is the correct owner. |
| Admission lookup | Rust `HashSet<Id>` | Repeated DataFusion joins | One materialization for the finite producer is smaller and does not create a second graph. |

## 12. Decision

**Accept this conservative boundary.** On 2026-09-25,
`nested_returns_need_an_uncontrolled_exit_before_becoming_value_summaries` passed with a
self-recursive source, a zero positive summary and a retained unknown boundary;
`pinned_identity_models_require_and_publish_their_real_formals` passed after the same producer
change. Full recursion, preceding-call completion and all integrated Stage 3 gates remain
`not_run` or unresolved as stated above.
