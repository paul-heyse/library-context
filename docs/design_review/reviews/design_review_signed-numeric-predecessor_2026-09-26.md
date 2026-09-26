# Design review: signed numeric predecessor argument

**2026-09-26 · change/conformance · compact.** Core standard 3.0,
code-intelligence profile 1.1 and library-context binding. Subject: the shared
argument-evaluation SQL and its finite direct-return consumer, under
[plan order 1](../../plans/behavioral-model-forward-plan_2026-09-24.md#32-functional-completion).

## 1–5. Owner, scenario and alternatives

Ruff syntax owns the placed expression tree. `cpg-schema::behavior` nominates
one source-cited normal argument evaluation; `lctx-analytics::summaries` checks
all arguments and ordered predecessor steps. The shared publisher/validator
reconstructs that transformation. A signed numeric literal was previously
unresolved because the direct argument node is `ExprUnaryOp`, not
`ExprNumberLiteral`. The new case requires the outer node to be the exact
argument span, its operator to be `+` or `-`, and its single `Operand` child to
be a numeric literal. Its evidence is the outer unary syntax fact. It does
not infer normal completion from the target's `normal_return` until every
argument has independent evaluation evidence.

The change scenario is `cast(object, -1); return value` versus
`cast(object, -(1 / 0)); return value`. The first gets the same cited
`preceding_call_normal` proof as a direct literal; the second has a binary
operand and stays `unsupported_control_flow`. The real extractor, Delta
publication, FORMAT 8 native reader and shared validator were exercised on
both. The direct modeled-return consumer of the same SQL also passed its
existing focused real fixture. More general unary operands, nested calls and
other callee forms remain plan order 1 work.

Ruff's placed syntax fields are the smallest owner for this fact. A source
text heuristic would relabel arbitrary nested expressions, while a separate
argument evaluator would duplicate the existing exact-span and ty-reaching
checks. No new table, codebook code, dependency or architecture decision is
needed. The SQL text does change the compiler digest, whose integrated guard
is deferred until functional completion.

## 6–8. Judgment and disposition

A1–A3, FP-01–FP-06 and applicable DP-01/02/03/08/11/13/15/16/21/23/24 and
CI-01/02/04/06/08/11 are **satisfied for this slice**: syntax remains the
source authority, the proof retains its identity, and an unresolved operand
cannot become a positive normal-return assertion. G1–G8 and CI-G1/CI-G2 pass
for the tested argument cases; CI-G3 is unchanged because gold is not an
analysis input. General predecessor status, path-specific boundary identity,
and assembled Stage 3 claims remain **unresolved** in the forward plan.

**Tested 2026-09-26:**
`RUST_MIN_STACK=16777216 INSTA_UPDATE=no cargo test -p cpg-core --test bundle finite_depth_and_unsupported_refusals_reach_the_native_response --quiet`
passed the signed/raising controls and native generation query;
`RUST_MIN_STACK=16777216 INSTA_UPDATE=no cargo test -p cpg-core --test compile pinned_identity_models_require_and_publish_their_real_formals --quiet`
passed the shared modeled-return consumer. `just fmt`, `just test-all`, fresh
`just pilot` and the all-techniques digest repin are `not_run` by the active
Stage 3 acceptance timing.

**Decision:** accept this bounded extension; no §B decision changed and no
ADR is required. The forward plan retains the enclosing open scope.
