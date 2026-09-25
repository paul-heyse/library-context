# Stage 3 source-call SCC topology — compact review

## 1. Scope, purpose and coverage

| | |
|---|---|
| Subject | Attributed local caller→callee SCCs and callee-first `summary_components` order |
| Standard | Core 2.0, code-intelligence 1.0, library-context binding |
| Tier · purpose | Change · conformance |
| Reviewer · date | Codex · 2026-09-25 |
| Decision | Accept deterministic topology, with no behavioral promotion |

I inspected the local-call projection, pinned petgraph 0.8.3 `tarjan_scc` contract, canonical
ordering, component id recipe, writer/validator and focused graph/fixture checks. This review
does not assess the bounded SCC summary worklist or claim complete call-target coverage.

## 6. Gates

| Gate | Verdict | Evidence or scope reason | Required action |
|---|---|---|---|
| G1 Authority | Pass | `call_targets`, `call_syntax` and local declarations own edges and vertices; SCC rows are derived. | — |
| G2 Fidelity | Pass, scoped | Parallel arcs do not alter SCC membership; candidate/open edges contribute topology without a behavior verdict. | Preserve edge modality and openness in composition. |
| G3 Validity | Pass | Source-equality validation reconstructs the full membership and order; a missing row is detected. | — |
| G4 Hidden behaviour | Pass | Petgraph owns Tarjan SCCs; a deterministic condensation schedule handles ties and does not trust traversal order. | — |
| G5 Consistency | Pass | Compiler output version 63 and reviewed Arrow schema declare the new analysis table. | — |
| G6 Transformation | Pass | Sorted member ids determine component identity; schedule order is separate metadata. | — |
| G7 Claims | Pass, scoped | Table explicitly describes topology only; no transfer/effect conclusion is published from an edge. | — |
| G8 Library leverage | Pass | Reuses pinned petgraph instead of implementing SCCs; DataFusion supplies source attribution. | — |
| CI-G1 Fidelity | Pass, scoped | Shuffled/parallel arcs, mutual recursion, self-loop and local caller→callee order are exercised. | Add complete/partial dispatch worklist cases next. |
| CI-G2 Evidence closure | Not applicable | Components are not served as a behavior claim. | Cite component/edge proof in later summaries. |
| CI-G3 Evaluation integrity | Pass | The topology uses compiled source facts, not the FastMCP gold. | — |

## 7. Findings and principle verdicts

| ID | Finding | Principles · gate | Evidence and consequence | Correction |
|---|---|---|---|---|
| F01, deferred | A local candidate edge can belong to an open dispatch set. | DP-08, CI-06 · CI-G1 | Treating an SCC edge as definite transfer would invent behavior and treating no edge as absence would invent a negative. | Carry exact target-set closure, modality and unresolved remainder into each composed path. |
| F02, deferred | Component order alone does not bound recursive path growth. | DP-09, CI-10 · G7 | A worklist without a depth/node/iteration cap could fail to terminate or silently truncate. | Add monotone bounded composition with explicit boundary rows. |

DP-01/02/03/04/05/08/09/11/13/14/15/18/19/21/22/23/24 and CI-01/02/04/06/07/10/12 are satisfied for this topology-only scope. Heuristic and serving principles are outside it. No SHOULD exception is requested.

## 8. Library-leverage ledger

| Capability | Bespoke scope | Built-in feature | Fit |
|---|---|---|---|
| Strongly connected components | Canonical domain ids and schedule tie-breaking | petgraph 0.8.3 `tarjan_scc` | O(V+E) SCC kernel; no custom DFS. |
| Attributed input | Which release call arcs enter the graph | DataFusion source joins | No new target resolver. |

## 12. Decision

**Accept the attributed, deterministic SCC schedule after focused validation.** The targeted
`cargo nextest run --release -p lctx-analytics -E
'test(call_sccs_are_callee_first_and_input_order_independent)' --no-tests=pass
--no-fail-fast` passed 1/1 on 2026-09-25. The selected release Nextest run passed 5/7,
including the local source-call order and publication tamper fixture; only the two expected
new schema/rule snapshots failed. After diff review and acceptance, the focused schema,
rule and reference run passed 4/4. Targeted release Clippy, ADR lint and diff checks passed.
No composed transfer or negative behavior claim follows from this table. `just fmt`,
`just test-all`, `just pilot` and integrated evaluation remain `not_run` until all functional
Stage 3 scope is implemented.
