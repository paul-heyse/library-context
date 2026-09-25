---
id: ADR-0033
title: Model total normal completion as a pinned assertion separate from transfer
status: accepted
date: 2026-09-25
supersedes: []
superseded-by: null
design: [§B5, §9.9]
evidence: Tested
revisit: A pinned model marked normal_return gains an exceptional or non-returning runtime path, or a Stage 3 fixture shows that complete exception coverage and a transfer rule do not justify the asserted normal exit.
---

## Context

L3 has a cited source-to-model transfer path, but a transfer rule describes the
value *if the callee returns*. A complete exception catalog describes which
modeled exceptions exist; absence of an exception row is not evidence that a
call terminates normally. Treating either as an implicit completion proof would
upgrade candidate `call_transfer` rows without a premise (DESIGN §B5, §9.9;
DP-08, CI-06). Python 3.14's `typing.cast` and `typing.assert_type` are a narrow
source-backed first case: the pinned CPython 3.14.7 `typing.py` bodies directly
return their value argument. Python 3.14 documentation also describes their
unchanged runtime return. Argument evaluation remains the caller's question.

## Options

1. **Infer normal completion from a transfer rule and no modeled exception.** This needs no
   new schema, but conflates a rule conditional on normal return with the
   existence of that return. Even complete exception coverage does not rule
   out divergence or another non-returning path.
2. **Analyze every pinned dependency's body for termination.** This could avoid
   authored completion claims, but the pinned context may be a typeshed stub
   and complete Python termination is not a bounded Stage 3 analysis.
3. **Author an explicit pinned normal-return assertion.** Chosen. It is a
   positive, model-scoped fact whose target, revision and catalog bytes are
   already in compiler identity. A missing assertion remains unknown.

## Decision

The typed model has an optional `normal_return` assertion, default false. It
means the target callable itself completes normally after its arguments are
evaluated, under the pinned model. The catalog accepts it only for a function
target with complete exception coverage and no authored exception rule. Its
compiled `model_targets` row carries the assertion alongside the pinned
definition fact and synthetic-model provenance. The shared publication
validator reconstructs it from the committed catalog; a doctored flag is
rejected.

The first assertions are limited to `typing.cast` and `typing.assert_type`,
whose pinned CPython 3.14.7 implementations directly `return val`. A modeled
source call must *also* prove exact argument/result endpoints, target selection,
modality, path condition and enclosing exit fate before L3 can use this field to
publish a positive flow. It does not change any unknown to a negative claim.

The source check used `inspect.getsource` under `uv run --no-project --python 3.14.7`;
the [Python 3.14 typing documentation](https://docs.python.org/3.14/library/typing.html#typing.cast)
describes the same runtime return. Focused release Nextest verifies the catalog,
binding and publication equality, including a forged flag; this does not yet test
call-result composition.

## Consequences

L3 can distinguish a transfer conditional on return from an asserted total
normal return without inferring completion from silence. Authored completion
claims need pinned source/documentation and independent oracles; a model of a
dependency with a callback, I/O, blocking path or partial exception coverage
remains unasserted until that work is done. This deliberately narrows immediate
`call_transfer` discharge. The exact source-to-summary application and
integrated pilot remain separate acceptance work.
