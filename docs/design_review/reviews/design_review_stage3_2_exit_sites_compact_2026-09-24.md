# Stage 3 L2 exit sites — compact change review

**Scope:** `exit_sites` producer, Arrow/codebook contract, Delta publication and shared
validator. **Standard:** core 2.0, code-intelligence 1.0 and the repository binding.
**Tier/purpose:** compact change/conformance. **Reviewer/date:** Codex, 2026-09-24.
**Decision:** accept the structural site relation; exception fate remains unresolved.

## Authority and transformation

Ruff's placed `syntax_nodes` identify `return`, `raise`, and direct statements in a `try`
`finalbody`. Ty's `flow_regions` contributes a statement condition and approximation flag.
The join binds the ty function scope's name span to the declaration's name span and cites
both raw fact ids. A direct `return` or `raise` in `finally` receives both applicable kinds.
No row means no joined site; it does not mean the function lacks exits. Every compile derives
the relation, including one without Stage E analysis. The shared validator reconstructs
exact rows from registered raw views before publication or consumption.

## Gate verdicts

| Gate | Verdict | Evidence and boundary |
|---|---|---|
| G1–G3 authority, fidelity, validity | Pass in scope | Source and flow fact ids, exact owner/scope join, closed site kinds and schema rules. |
| G4 effects | Pass | The fixture is extracted, never run. |
| G5 publication | Pass in scope | Reconstructed row equality rejects a doctored span; Delta writes are validated before publication. |
| G6 transformation | Pass in scope | One DataFusion relation is used by compile and validation; `ExitSiteKind` is appended. |
| G7 claims | Pass for tested sites; unresolved for fates | The focused test covers a raise, finally action, non-finally withholding and tamper. There is no handler/escape conclusion. |
| G8 library leverage | Pass | Ruff/ty source facts, DataFusion join, Arrow table macro and Delta validation cover the relation without a new parser or graph. |
| CI-G1–CI-G3 | Pass for site identity; unresolved for execution fate | Nodes and facts are attributed and distinguish finally actions from normal/exceptional exits. No path fate or served projection is claimed. |

Applicable DP-01–05, DP-07–08, DP-11, DP-13–15, DP-18–19 and DP-21–24,
plus CI-01–04, CI-06–07 and CI-10–13, are satisfied for this structural relation.
Completion/escape claims under DP-22 and CI-G3 remain unresolved outside this scope.

## Findings and disposition

| ID | Finding and consequence | Disposition |
|---|---|---|
| Resolved X01 | Matching ty's function scope to the full declaration span suppressed known exits. | Join to declaration name span; the `guarded` raise case now publishes. |
| Deferred X02 | A caught exception, implicit return, or suppressing context manager cannot be decided from a statement site. | Build handler/resource fate relations and finite summaries before negative or served verdicts. |
| Deferred X03 | The focused fixture cannot establish whole-library count or cost. | Fresh pilot and Stage 3 evaluation at the assembled end gate. |

**Passed, 2026-09-24:** focused release-profile Nextest selection (7 tests) for
schema/codebook/rule snapshots, references, positive/withholding/tamper, table count and
synthesis ledger; focused release Clippy with `-D warnings`. The earlier first join probe
failed on the `guarded` raise and produced the correction above. `just test-all`, fresh-store
`just pilot`, Q01/Q03/Q05/Q09 and native wheel acceptance are `not_run` at the operator's
design-phase direction.
