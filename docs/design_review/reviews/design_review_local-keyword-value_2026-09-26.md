# Design review: one explicit keyword local value call

**2026-09-26 · change/conformance · scoped author review.** Core 3.0,
code-intelligence profile 1.1 and the library-context binding in
`standard.toml` apply. The subject is the local-call seed relation and its
existing finite value producer, shared validator and native inspection.
[Plan order 1](../../plans/behavioral-model-forward-plan_2026-09-24.md#3-stage-3-execution-queue)
owns general argument evaluation and normal-completion work.

## Contract and scenario

The extension is `return f(value=value)` for a sole closed local `f` whose
formal `value` has a cited finite summary. Ruff gives one explicit keyword
argument, ty ties the tracked source use to that argument, and Pass B maps it
definitely to the callee formal. The same SCC worklist, predecessor checks,
ordered call proof, source origin and shared publication validator used for a
single positional argument now apply. A `**` mapping has no exact explicit
argument row and remains an origin-specific `call_transfer` unknown. A nested
or raising keyword expression remains outside this admitted source shape.

Adding another explicit binding form is a change to the DataFusion seed's
selection contract, not a new path producer or native interpretation. The
selected relation is derived from attributed Ruff/ty/Pysa and Pass B facts;
the value path is a may-path under their stated model. Empty selection does
not establish absence. Multiple targets, nondefinite mapping, unpacking and
unproved predecessor completion continue to withhold.

| Gate | Scoped judgment |
|---|---|
| G1–G3, CI-G1 | Satisfied: the keyword source fact, definite formal mapping, callee summary and source origin retain distinct identities; shared validation reconstructs both positive and withholding outcomes. |
| G4–G6 | Satisfied: one declared query change feeds the existing bounded deterministic producer and Delta/native route. |
| G7, CI-G2 | Satisfied for path-local inspection; no operation-wide or negative behavior is inferred from an unpacked call. |
| G8 | Satisfied: DataFusion selection and the existing Arrow/native contracts cover the case without a new evaluator or compatibility adapter. |
| CI-G3 | Unaffected: gold reference data is not an input. |

FP-01–06 and applicable DP-01/02/03/04/08/11/16/18/19/21 and
CI-01–04/06 are satisfied within the one-keyword scope. General argument
expressions, keyword order among multiple arguments, unpacking and possible
raises remain [plan order 1](../../plans/behavioral-model-forward-plan_2026-09-24.md#3-stage-3-execution-queue)
work; there is no new architectural finding.

**Tested 2026-09-26:** `RUST_MIN_STACK=16777216 INSTA_UPDATE=no cargo test
-p cpg-core --test compile nested_returns_need_an_uncontrolled_exit_before_becoming_value_summaries
--quiet` passed the real source positive/unpacked withholding and publication
validation; `RUST_MIN_STACK=16777216 INSTA_UPDATE=no cargo test -p cpg-core
--test bundle finite_depth_and_unsupported_refusals_reach_the_native_response
--quiet` passed the same distinction through Delta/native. `just fmt`,
`just test-all` and `just pilot` are **not_run** pending functional completion.

**A1 satisfied, A2 satisfied, A3 satisfied. Accept scoped** at Tested
strength. The enclosing Stage 3 architecture and integrated qualification
remain unresolved.
