---
id: ADR-0055
title: Require an independent normal-read witness for modeled source operands
status: accepted
date: 2026-09-26
supersedes: []
superseded-by: null
design: [§B5, §9.9]
evidence: Tested
revisit: A source expression other than a direct parameter read needs a normal-evaluation proof, or the lexical exclusion of deletion and exception handlers proves too coarse on real library paths.
---

## Context

The one-call `modeled_argument_evaluations` source row cited a raw parameter-origin
`flow_values` fact. That fact says where a value may flow; it does not prove that
evaluating the argument completes normally. [Plan order 1](../plans/behavioral-model-forward-plan_2026-09-24.md#32-functional-completion)
requires a separate argument-evaluation witness before a pinned target's
`normal_return` assertion can be used. The nested-chain producer exposed the same
distinction when it began requiring an exact ty reaching fact for its innermost
direct formal read.

An exact-only ty rule loses an existing valid path: ty marks reads in a `try`
body approximate, even when a directly resolved parameter remains bound and
the pending return crosses only pass finalizers. The pinned ty-flow evidence
records `try` ambiguity; the real `framed_modeled_identity` row has an
approximate reaching fact. A raw-flow-only rule cannot distinguish that case
from an unproved source expression.

## Options

1. **Keep the raw value fact as the sole source-argument evidence.** It needs
   no schema change, but conflates a value-flow observation with normal
   evaluation and permits a model's normal-return assertion to hide an
   unproved source read.
2. **Require only a non-approximate ty reaching fact.** It is simple and
   exact where available, but drops the pass-only `try` path because ty's
   region abstraction is approximate there.
3. **Keep raw flow and normal read as separate evidence, with a bounded
   lexical fallback.** Chosen. It preserves the valid `try` case without
   treating arbitrary names or expressions as normally evaluated.

## Decision

The current-store `modeled_argument_evaluations` schema adds a nullable
`source_normal_evidence_id`. A `source_operand` row still cites its raw flow
fact in `evidence_id` and must now cite a separate direct-parameter read in
the new field; absent evidence makes the row `unknown`. The shared producer
places both facts in its ordered proof, and the shared publication validator
reconstructs them. The schema invariant enforces the distinction. Historical
binary readers are not added; [ADR-0048](0048-schema-rebuild-policy.md)
governs fresh current-store rebuilds.

The existing exact ty route remains `parameter_name_normal`. The append-only
`lexical_parameter_normal` status (code 6) is the fallback: Ruff must show a
direct name argument, lexical resolution must point to a parameter binding in
the current function's scope, and that function must contain no explicit
`del` or exception-handler frame. It cites the resolution fact. The same
classifier serves preceding calls, one-call model arguments and nested model
chains. An approximate local assignment, captured parameter, other
expression or deletion/handler case remains unknown without another proof.

The custom finite producer owns the proof order; DataFusion owns the shared
fact joins and validator. This preserves the [§B5](../design/DESIGN.md#section-b5)
distinction between provider observation and semantic conclusion.

## Consequences

The `framed_modeled_identity` return keeps its positive source/Delta/native
path with a cited lexical resolution; a possible deletion in a `try` frame
has a raw model candidate but no normal-read witness or positive path. A direct formal with exact ty evidence cites its
reaching fact. One extra nullable column and append-only status require a
reviewed schema/codebook snapshot migration. The lexical exclusion is
deliberately coarse; proving a safe read through a function with a separate
unrelated deletion needs a more precise future rule. General expression
evaluation, path-specific predecessor completion and integrated acceptance
remain under [plan orders 1 and 10](../plans/behavioral-model-forward-plan_2026-09-24.md#32-functional-completion).

Focused evidence on 2026-09-26: `cargo test -p lctx-analytics --lib
summaries::finite --quiet`, `INSTA_UPDATE=no cargo test -p cpg-schema --test
codebooks --test contracts --quiet`, and `RUST_MIN_STACK=16777216
INSTA_UPDATE=no cargo test -p cpg-core --test compile
pinned_identity_models_require_and_publish_their_real_formals --quiet` passed.
The focused native check is recorded in the scoped review; `just test-all` and
`just pilot` are not_run until functional completion.
