---
id: ADR-0056
title: Give closed argument expressions their own normal-evaluation status
status: accepted
date: 2026-09-26
supersedes: []
superseded-by: null
design: [§B5, §9.9]
evidence: Tested
revisit: A closed expression needs an evaluated value or a nested operand proof rather than only a normal-completion witness.
---

## Context

The shared argument classifier admitted direct literals and then bounded unary and numeric
binary expressions as normally evaluated siblings of a pinned call. It gave all three the
`literal_normal` status, even though a unary or binary expression is not a literal. Adding
Python `and`/`or` short-circuit evaluation made that conflation consequential: a stored row
would cite an outer Boolean expression but label it a literal. The [Stage 3 order-1
plan](../plans/behavioral-model-forward-plan_2026-09-24.md#32-functional-completion) requires
typed, source-attributed argument completion; [§B2](../design/DESIGN.md#section-b2) makes the
codebook a durable Arrow contract. The existing direct-literal code remains correctly named.

## Options

1. **Keep `literal_normal` for every closed syntax form.** No codebook migration, but consumers
   must infer whether a cited fact is actually a literal. The status would contradict its name
   and obscure which proof rule was used.
2. **Add an append-only `closed_expression_normal` status.** Chosen. It distinguishes direct
   literals from a closed, locally proved expression without adding a second relation or
   changing the source/model proof shape.
3. **Add a separate expression-evaluation graph now.** It could compose nested operands later,
   but adds a new schema, validator and consumer before this bounded form needs them.

## Decision

`ModeledArgumentEvaluationStatus` appends code 7, `closed_expression_normal`. Direct literals
remain code 1. The shared DataFusion classifier assigns code 7 only to a whole Ruff unary
expression with a direct safe literal operand, `+`/`-` over two direct numeric literals, or a
two-operand `and`/`or` expression whose first direct Boolean literal decides the result and
skips the second operand. In the short-circuit case, `False and ...` and `True or ...` complete
without evaluating the right operand. The witness is the outer syntax fact. Python 3.14's
[Boolean-operation reference](https://docs.python.org/3.14/reference/expressions.html#boolean-operations)
establishes that order and short-circuit behavior; pinned Ruff 0.0.11 supplies ordered operand
children. This status proves only normal evaluation of the complete expression, not its value,
dispatch, enclosing return or any arbitrary nested expression.

The existing pure producer and shared publication validator consume the same typed relation;
the `modeled_argument_evaluations` invariant and generated codebook rule admit code 7. Current
stores and serving generations are rebuilt under [ADR-0048](0048-schema-rebuild-policy.md);
no historical binary reader is introduced.

## Consequences

The codebook and contract snapshots change, so this is an explicit codebook migration. The
shared status now describes its evidence accurately across preceding-call and modeled-return
consumers. A source expression, an unproved right operand that is actually evaluated, chained
Boolean operations and general nested evaluation remain unknown. They require later order-1
rules, not a broader reading of code 7.

Focused real-source, Delta/shared-validation and FORMAT 8 native positives and raising controls
are recorded in the [scoped review](../design_review/reviews/design_review_short-circuit-argument_2026-09-26.md).
The integrated Stage 3 gate and pilot remain `not_run` until the full functional scope is ready.
