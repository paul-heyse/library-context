# Design review: Boolean-literal `not` argument evaluation

**2026-09-26 · change/conformance · scoped author review.** Core 3.0,
code-intelligence profile 1.1 and the library-context binding in
`standard.toml` apply. [Plan order 1](../../plans/behavioral-model-forward-plan_2026-09-24.md#32-functional-completion)
owns general call-site and predecessor evaluation.

## Contract and scenario

A source-ordered pinned `typing.cast(object, not False)` preceding a direct
return has one unary syntax node whose operand is a direct Boolean literal.
Python 3.14's [expression reference](https://devdocs.io/python~3.14/reference/expressions)
states that `not` returns a Boolean after evaluating its operand. A literal
Boolean has no user-defined truth operation. The shared DataFusion simple
argument evaluator can therefore cite the unary syntax fact as a normal
operand witness. This feeds both modeled direct returns and preceding-call
completion; the finite producer and shared validator retain the same
source-ordered proof and boundary rules.

The predicate is restricted to `not` over an `ExprBooleanLiteral` child.
`not (1 / 0)` has a raising operand and must not acquire that witness. A
nonliteral operand may invoke user-defined truth testing; no generic unary
normal-completion assertion follows from the operator spelling. The
existing `+`/`-` numeric-literal cases remain separate.

| Gate | Scoped judgment |
|---|---|
| G1–G3, CI-G1 | Satisfied: the selected unary and literal syntax are attributed Ruff facts; the normal-call proof cites the unary fact and the shared validator reconstructs it. |
| G4–G6 | Satisfied: one owned DataFusion evaluator serves both consumers, with no parallel syntax walker. |
| G7, CI-G2 | Satisfied for this path-local normal witness; a raising/nonliteral operand remains unknown and is not turned into a negative claim. |
| G8 | Satisfied: the pinned Python expression contract, existing syntax codebook, DataFusion query and finite producer cover the bounded form. |
| CI-G3 | Unaffected: the gold reference is not a compiler input. |

FP-01–06 and applicable DP-01/02/03/04/08/11/16/18/19/21 and
CI-01–04/06 are satisfied within the literal-Boolean scope. General
expression evaluation, nested calls, possible raises and condition-specific
predecessor sequencing remain plan order 1 work; no §B decision changed.

**Tested 2026-09-26:** `RUST_MIN_STACK=16777216 INSTA_UPDATE=no cargo test
-p cpg-core --test bundle finite_depth_and_unsupported_refusals_reach_the_native_response
--quiet` passed the positive unary witness and raising withholding through
the real source, Delta and native reader. `just fmt`, `just test-all` and
`just pilot` are **not_run** pending functional completion.

**A1 satisfied, A2 satisfied, A3 satisfied. Accept scoped** at Tested
strength. The enclosing Stage 3 evaluation contract remains incomplete.
