---
id: ADR-0057
title: Compose Stage 3 evaluation, completion and summaries through shared typed contracts
status: accepted
date: 2026-09-26
supersedes: []
superseded-by: null
design: [§B5, §9.9, §11.3]
evidence: Proposed
revisit: A supported model cannot compose through the shared evaluation, completion, binding and coverage contracts, or the same-state multi-channel comparison demonstrates a smaller conforming scheduler.
---

## Context

Stage 3 currently admits several narrow return shapes and one optional Boolean control on a
local call. Adding effect, exception and role channels to these shapes would repeat evaluation,
binding and completion decisions. Native proof admission also knows the adjacency of the
one-control proof. The operator selected consolidation before completing Stage 3 on 2026-09-26.
The [target review](../design_review/reviews/design_review_stage3-composition_2026-09-26.md)
records the inspected boundaries and architectural scenarios; the [active plan](../plans/behavioral-model-forward-plan_2026-09-24.md#30-consolidated-execution)
owns implementation and qualification.

## Options

1. **Extend each existing seed shape.** Least immediate migration, but a second control or new
   channel independently changes SQL admission, pure transfer and native proof interpretation.
2. **Shared typed contracts in existing modules (chosen).** Ordered argument relations,
   independently cited evaluation/completion, condition substitution and channel coverage
   compose through pure functions. Arrow relations and proof checks have one schema owner.
3. **Introduce a generic evaluator/plugin framework or switch reasoning engines.** Neither
   supplies the missing Python semantic obligations. This adds lifecycle and representation
   burden before demonstrating a consumer advantage.

## Decision

`cpg-schema` owns meaning, checked types, identities and shared proof invariants. Provider
adapters expose attributed observations; `lctx-analytics` owns pure ordered evaluation, frame
unwinding and summary composition; `cpg-core` acquires relations, orders phases and publishes.
The immutable native executor selects the same semantics; Python adapts the protocol.

Evaluation order and argument-to-formal binding are separate relations. A normal-evaluation
witness is distinct from an exact primitive value, and completion is distinct from a transfer
or potential effect. Completion and channel coverage are explicit; incomplete paths never
become negatives. Definite/potential modalities and invocation phases survive composition.

Condition substitution is simultaneous over repository atom identities. Replacement functions
are not recursively substituted. Compose using biodivine's bounded Boolean operations, with
one cumulative work and retained-intermediate budget, rather than its unbounded `substitute`.
Missing identity/stability links refuse semantic substitution; Boolean substitution alone
does not establish a Python value link.

Retain the existing SCC scheduler while separating semantic progress from bounded proof
alternatives. Preserve source origins and parallel call sites. ADR-0053 still governs the
implemented value engine; its multi-channel comparison must use the actual shared contract.
The existing depth and work limits remain defaults, and each refusal has an explicit cause.

Expanded semantic serving is FORMAT 9, with typed effect/role/compatibility filters and checked
evidence closure. FORMAT 8 remains the implemented format until that migration lands. Rebuild
current stores and generations under ADR-0048; no historical reader is required.

## Consequences

The target is accepted, not implemented or qualified by this record. Existing narrow routes
must migrate and then be deleted, rather than coexist with a second authority. Ordinary model
additions should require a declaration and focused tests; genuinely new semantics can extend
the typed contract. No general Python interpreter, alias analysis, deferred execution, registry
or solver migration is included. Targeted checks accompany implementation; formatting and
integrated qualification follow the entire functional scope. Independent runtime/oracle
checks challenge shared semantics rather than merely reconstructing them.
