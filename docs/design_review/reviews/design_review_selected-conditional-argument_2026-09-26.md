# Design review: selected literal conditional argument

**2026-09-26 · change/conformance · scoped author review.** Core 3.0,
code-intelligence profile 1.1 and the library-context binding apply.
[Plan order 1](../../plans/behavioral-model-forward-plan_2026-09-24.md#32-functional-completion)
owns complete argument evaluation. This form extends the bounded
`closed_expression_normal` category established by
[ADR-0056](../../adr/0056-closed-expression-evaluation.md); it changes no §B owner or codebook.

## Scenario and contract

For `cast(object, 1 if True else (1 / 0)); return value`, the selected direct
literal finishes and the raising else branch is not evaluated. Reversing the
Boolean test selects division, so no normal witness may be produced. The
modeled-return sibling checks the opposite branch: `(1 / 0) if False else 2`
is safe, while changing `False` to `True` withholds the positive path.
Python 3.14's [conditional-expression reference](https://docs.python.org/3.14/reference/expressions.html#conditional-expressions)
specifies test-first, selected-branch-only evaluation; a separate CPython 3.14
probe exercised both choices. The pinned Ruff 0.0.11 extractor exposes the
outer `ExprIf` and its direct `Test`, `Value` and `Orelse` children.

The shared DataFusion classifier requires the complete call argument span,
one direct Boolean test, and a direct literal child in exactly the selected
field. It cites the outer `ExprIf` fact with status 7. The unused branch is
intentionally unrestricted because it cannot execute under the proved test.
The existing pure producer, shared publication validator and native reader
consume that row; no source text parser, computed value or historical reader
is introduced. A nonliteral test, selected expression needing evaluation,
or a nested conditional remains unknown.

| Judgment | Scoped result |
|---|---|
| A1–A3, FP-01–06 | Satisfied: one owner classifies the direct syntax shape for both predecessor and modeled-return consumers; proof identity and typed serving reuse the existing status. |
| G1–G3, G6–G7, CI-G1–CI-G2 | Satisfied in the tested form: Ruff owns branch attribution, the selected expression is a direct safe literal, and reversing the test withholds completion. |
| G4–G5 | Satisfied within the slice: publication remains snapshot-pinned and the query adds no effects. |
| G8 | Satisfied: Ruff child fields and DataFusion joins implement the bounded rule without a second parser or evaluator. |
| CI-G3 | Unaffected: gold families are not compiler input. |

Applicable DP-01/02/03/08/11/13/16/18/21/23 and
CI-01/02/04/06/11 are satisfied for this direct-literal form. No result here
certifies general conditional evaluation or absence of exceptions.

**Tested 2026-09-26:** `uv run --no-project --offline --no-python-downloads
python -c ...` passed two selected safe literals and two selected raising
branches in CPython 3.14. The focused real-source, Delta/shared-validation and
FORMAT 8/native check is `RUST_MIN_STACK=16777216 INSTA_UPDATE=no cargo test
-p cpg-core --test bundle
finite_depth_and_unsupported_refusals_reach_the_native_response --quiet`;
its outcome and targeted Clippy/docs receipts are in STATUS. `just fmt`,
`just test-all` and `just pilot` remain **not_run** until functional completion.

**Accept scoped at Tested strength; the focused native check passed.**
The enclosing Stage 3 expression and exit model remains incomplete.
