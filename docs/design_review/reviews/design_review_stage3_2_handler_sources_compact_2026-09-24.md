# Stage 3 L2 handler sources — compact change review

**Scope:** except clauses and direct body actions as source/control relations, Arrow and
Delta contracts, publication validation. **Standard:** core 2.0, code-intelligence 1.0 and
repository binding. **Tier/purpose:** compact change/conformance. **Reviewer/date:** Codex,
2026-09-24. **Decision:** accept the attributed structural rows only; no catch or conversion
verdict follows.

## Authority and transformation

Ruff syntax ties each `except` clause to its `try` and optional type expression. Ty's
`flow_regions` supplies the `try` entry condition and the reachability of direct statements
in the handler body. Function identity uses the matching name span in the ty scope and release
declaration. The compiler writes these rows for every compile. The shared validator re-derives
the complete clause and action sets from the registered source views; foreign-key, codebook,
key and span rules apply before publication. No executable Python input is run.

## Gate verdicts

| Gate | Verdict | Evidence and limit |
|---|---|---|
| G1–G3 | Pass for source observations | Node and fact citations distinguish clause, type expression, try and action. Missing joins produce no semantic claim. |
| G4–G6 | Pass for the tables | Existing DataFusion relation, Arrow macro, Delta path and shared reconstruction cover the transformations and publication. |
| G7 | Pass for tested structural cases; unresolved for exception fates | The focused test exercises one typed handler/action, non-handler and finally withholding, and ordinal tamper. No match/completion proof exists. |
| G8 | Pass | Ruff and ty supply source/control facts; no new parser or bespoke exception solver was added. |
| CI-G1–CI-G3 | Pass for relation identity; unresolved for behavioral inference | Clause and action identity is explicit, but a type expression is not a resolved class and a try-entry condition is not a match condition. |

Applicable DP-01–05, DP-07–08, DP-11, DP-13–15, DP-18–19 and DP-21–24 and
CI-01–04, CI-06–07, CI-10–13 are satisfied for these stored source rows. A handler effect,
exception escape and completeness remain unresolved under DP-22 and CI-G3.

## Findings and disposition

| ID | Finding and consequence | Disposition |
|---|---|---|
| Deferred H01 | A handler's type expression may denote multiple or unknown runtime classes; the clause row cannot decide which raise it catches. | Resolve bindings and hierarchy under the runtime view; unresolved cases stay unknown. |
| Deferred H02 | A body assignment or return may be bypassed by a branch, exception or `finally` override. | Model normal and exceptional exits and action effects before summary conversion. |
| Deferred H03 | Pilot row counts and cost remain unmeasured for the assembled Stage 3. | Run the final fresh-store pilot and Q gates after the remaining scope is implemented. |

**Passed, 2026-09-24:** seven focused release-profile Nextest tests cover schema/rule
snapshots, references, handler positive/withholding/tamper, exit sites, table count and the
synthesis ledger. Contract snapshots were reviewed before acceptance. Focused release
Clippy on `cpg-schema` and `cpg-core` with `-D warnings` passed. The full integrated test and
pilot gate remains `not_run` at operator direction.
