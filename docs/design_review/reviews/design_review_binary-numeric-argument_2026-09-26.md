# Design review: direct numeric binary argument evaluation

**2026-09-26 · change/conformance · scoped author review.** Core 3.0,
code-intelligence profile 1.1 and the library-context binding apply.
[Plan order 1](../../plans/behavioral-model-forward-plan_2026-09-24.md#32-functional-completion)
owns general expression evaluation.

## Contract and scenario

The shared `simple_argument_evidence_sql` classifier now recognizes an explicit
call argument whose complete Ruff expression is `ExprBinOp` with detail `+`
or `-` and exactly the direct left and right `ExprNumberLiteral` children.
The pinned Ruff 0.0.11 extractor supplies the outer expression and its
`left`/`right` child fields. Python 3.14's
[expression reference](https://devdocs.io/python~3.14/reference/expressions)
defines additive expressions separately from division and records that
division by zero raises. Direct numeric literals have built-in operands;
the bounded form has no user-defined arithmetic dispatch. The normal witness
cites the outer binary syntax fact, while the source-read and model-target
proofs remain separate.

The expected extension is `cast(object, 1 + 2); return value`, followed by
the same expression as a sibling in `return cast(1 + 2, value)`. Both consume
the same DataFusion classifier, finite proof and shared validator. The
withholding controls replace `+` with `/` and `2` with `0`; they receive no
normal-evaluation witness or positive summary. Nested operations, nonliteral
operands, other operators and overloaded numeric-looking values remain
unknown. This is not a constant evaluator: it recognizes only a closed
syntax shape and does not store a computed result.

| Gate | Scoped judgment |
|---|---|
| A1–A3, FP-01–06 | Satisfied: one shared classifier serves both consumers; the finite producer and native reader reuse typed proof evidence without a second expression interpreter. |
| G1–G3, G6–G7, CI-G1–CI-G2 | Satisfied in tested cases: Ruff syntax owns the shape, unsupported division stays open, and the shared publication validator reconstructs the cited binary fact. |
| G4–G5 | Satisfied within the slice: the query and pure producer add no effects; publication remains snapshot-pinned. |
| G8 | Satisfied: Ruff's existing binary/child facts and DataFusion joins replace any bespoke source parse. |
| CI-G3 | Unaffected: gold reference families are not compiler input. |

Applicable DP-01/02/03/08/11/13/16/18/21/23 and CI-01/02/04/06/11 are
satisfied for the direct-literal form. No §B decision or schema changes.
General argument evaluation, possible raises and path-specific predecessor
sequencing remain in the plan.

**Tested 2026-09-26:** `RUST_MIN_STACK=16777216 INSTA_UPDATE=no cargo test
-p cpg-core --test bundle
finite_depth_and_unsupported_refusals_reach_the_native_response --quiet`
passed real Ruff source, Delta publication, shared validation and FORMAT 8
native positives/withholding controls. The positive predecessor and modeled
sibling proofs cite the outer addition syntax fact. Targeted Clippy and docs
checks are recorded in STATUS. `just fmt`, `just test-all` and `just pilot`
are **not_run** until functional completion.

**Accept scoped at Tested strength.** The enclosing Stage 3 expression and
exit models remain incomplete.
