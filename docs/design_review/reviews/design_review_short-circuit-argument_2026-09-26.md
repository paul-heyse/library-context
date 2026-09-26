# Design review: closed short-circuit argument evaluation

**2026-09-26 · change/conformance · scoped author review.** Core 3.0,
code-intelligence profile 1.1 and the library-context binding apply.
[Plan order 1](../../plans/behavioral-model-forward-plan_2026-09-24.md#32-functional-completion)
owns complete argument and predecessor evaluation; [ADR-0056](../../adr/0056-closed-expression-evaluation.md)
owns this append-only status correction.

## Expected change and ownership

The real trigger is `cast(object, False and (1 / 0)); return value`: the right
operand cannot raise because it is not evaluated. In `True and (1 / 0)`, it is
evaluated and may raise. `True or ...` and `False or ...` give the corresponding
modeled-return sibling control. Python 3.14's
[Boolean-operation reference](https://docs.python.org/3.14/reference/expressions.html#boolean-operations)
owns short-circuit and operand-result semantics; a separate CPython 3.14 probe
executed both polarities. The pinned Ruff 0.0.11 extractor owns the outer
`ExprBoolOp` operator and ordered direct `Operand` children. DataFusion's
single `simple_argument_evidence_sql` classifier selects only a complete
two-operand expression whose first direct Boolean literal decides the result.
The finite producer, shared publication validator and native reader consume
the resulting typed witness without a second expression interpreter.

A future three-operand chain, name read or nested first operand changes the
classifier and its proof rule; it does not silently inherit this one. The
unexecuted second operand is allowed to be arbitrary because the decisive
literal prevents its evaluation. The evidence cites the outer Boolean syntax
fact, not the right expression or an invented computed value. Direct literals
retain `literal_normal` (code 1); safe whole unary, numeric binary and Boolean
expressions use append-only `closed_expression_normal` (code 7). The schema
invariant and generated codebook rule admit code 7, with current-store rebuild
under ADR-0048. No historical binary reader is added.

| Judgment | Scoped result |
|---|---|
| A1–A3, FP-01–06 | Satisfied: Ruff syntax is the sole expression-shape authority, the shared classifier serves predecessor and modeled sibling, and the status now states its actual evidence type. |
| G1–G3, G6–G7, CI-G1–CI-G2 | Satisfied in the tested form: the outer syntax fact and operand order are reconstructible; `True and ...` and `False or ...` with a raising right operand stay unknown. |
| G4–G5 | Satisfied within the slice: one query and existing pure producer add no effects; publication stays snapshot-pinned. |
| G8 | Satisfied: pinned Ruff operands, DataFusion windows and the shared codebook replace a bespoke parser or second rule engine. |
| CI-G3 | Unaffected: the gold reference never enters compilation. |

Applicable DP-01/02/03/08/11/13/16/18/21/23 and
CI-01/02/04/06/11 are satisfied for this form. The remaining expression
grammar, nested evaluation and path-specific exceptional completion are
explicitly outside this scoped acceptance. The enclosing Stage 3 design is
still incomplete; no negative claim follows from a missing witness.

**Tested 2026-09-26:** `uv run --no-project --offline --no-python-downloads
python -c ...` passed the four short-circuit polarities in CPython 3.14;
`INSTA_UPDATE=no cargo test -p cpg-schema --test codebooks --test contracts
--quiet` passed after reviewing and accepting the append-only codebook,
contract and generated-rule snapshots. The focused real-source, Delta,
shared-validation and FORMAT 8/native check is
`RUST_MIN_STACK=16777216 INSTA_UPDATE=no cargo test -p cpg-core --test bundle
finite_depth_and_unsupported_refusals_reach_the_native_response --quiet`.
Its outcome and targeted Clippy/docs receipts are recorded in STATUS.
`just fmt`, `just test-all` and `just pilot` remain **not_run** until full
functional completion.

**Accept scoped at Tested strength; the focused native check passed.**
The plan owns general expression and exit semantics.
