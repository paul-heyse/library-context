# Design review: closed Boolean control in a recursive value summary

**2026-09-26 · change/conformance · scoped author review.** Core 3.0,
code-intelligence profile 1.1 and the library-context binding apply.
[Plan order 6 and W12](../../plans/behavioral-model-forward-plan_2026-09-24.md#32-functional-completion)
own general argument and condition composition. This slice stays within the
SCC worklist of [ADR-0053](../../adr/0053-bounded-scc-summary-worklist.md)
and the typed closed-expression witness of
[ADR-0056](../../adr/0056-closed-expression-evaluation.md).

## Scenario and boundary

`recursive_closed_true(value, not False)` calls a conditional recursive
function with a control value that is not a direct literal syntax node.
Python's `not` of a direct Boolean literal has a fixed Boolean result; the
pinned Ruff unary/operand facts identify its exact source. The shared
`simple_argument_evidence_sql` classifier now supplies that fixed result as a
separate query field only for this safe unary shape. It does not infer the
value from the general `closed_expression_normal` status. The local-call seed
requires a definite mapping of that explicit argument to the distinct
callee formal, its normal-evaluation fact and the callee's direct test link.
The existing bounded BDD restriction and SCC worklist then compose the cited
finite base. A `not True` control cannot select that base, while
`not (1 / 0)` supplies neither a static Boolean value nor a local seed.

Ruff, the argument-flow mapping and the test-value link are separate
authorities. DataFusion joins them once for both publication and shared
validation; the pure producer does not parse Python or invent a guard value.
The proof orders the unary `argument_evaluation` before the
`callee_condition_link`, and native serving reads that same checked proof.
Another closed expression may use this route only after the shared classifier
proves both its normal completion and exact Boolean result. Multi-control
substitution, arbitrary expressions and cross-scope condition conjunction
remain outside the slice.

| Judgment | Scoped result |
|---|---|
| A1–A3, FP-01–06 | Satisfied: one shared classifier owns the value, the local-call relation binds it to the callee formal, and the existing pure SCC producer owns composition. |
| G1–G3, G6–G7, CI-G1–CI-G2 | Satisfied in the tested case: the normal-evaluation and guard-link facts are distinct; false and raising controls retain open boundaries. |
| G4–G5 | Satisfied: no new effect, storage format or serving lifecycle is introduced. |
| G8 | Satisfied: Ruff facts, DataFusion joins and the existing biodivine BDD restriction avoid a second expression evaluator. |
| CI-G3 | Unaffected: reference families remain outside compiler input. |

Applicable DP-01/02/03/08/11/13/16/18/21/23 and
CI-01/02/04/06/11 are satisfied for the one closed Boolean control. This
does not certify general recursive argument substitution or other summary
channels.

**Tested 2026-09-26:** `RUST_MIN_STACK=16777216 INSTA_UPDATE=no cargo test
-p cpg-core --test bundle
finite_depth_and_unsupported_refusals_reach_the_native_response --quiet`
passes the real-source positive and false/raising controls through Delta,
shared validation and FORMAT 8/native. An isolated
`uv run --no-project --offline --no-python-downloads python -c ...`
probe passed the two `not` literal values and the raising operand. Targeted Clippy/docs results are in
STATUS; `just fmt`, `just test-all` and `just pilot` remain **not_run** until
functional completion.

**Accept scoped at Tested strength; the final raising control passed.**
The enclosing Stage 3 recursive composition remains incomplete.
