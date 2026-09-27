---
id: ADR-0061
title: Prove pinned call default availability independently of callee outcomes
status: accepted
date: 2026-09-27
supersedes: []
superseded-by: null
design: [§9.9]
evidence: Proposed
revisit: A pinned target needs default value or mutable-default stability semantics rather than availability alone, or its exact runtime optional formals disagree with the admitted signature domain.
---

## Context

The reached-call producer currently borrows `normal_return` to admit omitted optional formals.
Consequently `json.dump(value, stream)` stays unknown although the exact pinned implementation
has already-created keyword defaults, while spelling every optional argument activates its
potential partial I/O. Availability is neither a call outcome nor the identity/value of a default.
ADR-0057/0058 and [§9.9](../design/sections/behavioral-analysis.md#section-9-9) require those premises
to remain distinct. The real partial-I/O challenge confirms omitted-default execution in the
pinned runtime; that observation alone does not qualify all model targets.

## Options

1. **Retain total-normal or fully explicit arguments.** Smallest code path, but couples unrelated
   premises and withholds useful fallible calls. A total-return assertion is not the owner of a
   definition-time availability promise.
2. **Infer availability from optional stub formals.** Less authored data, but provider requiredness
   describes an interface, not runtime creation of defaults, and cannot supply the missing premise.
3. **A separate exact-target availability assertion (chosen).** Reuse the existing pinned catalog,
   signature binder, ordered invocation and normal evaluator. It adds one model premise and cited
   omitted-formal evidence; source and immutable consumers share its admission. No new interpreter
   or model-name-specific evaluator is needed.

## Decision

A pinned callable model may explicitly assert `call_defaults_available`. False or absence makes
no promise. True states that every omitted optional formal in every admitted signature has an
already-existing runtime default under that exact pinned implementation. Authorship must qualify
runtime formals/default creation from the pinned implementation, not merely stub optionality.
It supplies no normal return, default value, Boolean control, mutable-value stability, resource
identity or callback behavior. The same exact-target and invocation-phase admission applies.

The shared binder still requires known requiredness, valid argument shape and agreement across
all admitted signatures; variadic absence is not an omitted fixed default. Required arguments
never become optional. Invocation evidence cites the model availability promise and each omitted
formal's context fact. Derive and commit the complete omitted-formal count/digest from binding
independently of retained proof; shared admission checks exact model/formal coverage, including
whole-group deletion. Preserve signature-specific fact IDs even when names repeat. Source and
S6 raw signature/argument reconstruction own binding semantics beyond structural commitments.
No default expression is evaluated at the call. A model action path whose
subject is omitted remains unresolved unless a separate value identity proves it; the availability
promise never creates a source argument. Remove the existing normal-return shortcut.

Schema owns the premise and evidence shape. Analytics consumes it in the common invocation
owner reused by whole-expression evaluation. Core publishes/reconstructs it; native retains the
same structural support as its call/action consumers migrate. This promise is relative to the
same exact pinned implementation assumptions as other model assertions; it is not a universal
claim about mutated function metadata or arbitrary external state.

## Consequences

Implementation is **Proposed**. Initial declarations are qualified individually; no blanket
stdlib or optional-parameter rule is added. Positive and withholding source/Delta controls must
separate available defaults, missing required arguments, raising explicit arguments, omitted
subjects and fallible outcomes. Contract mutation controls remove or swap availability/formal
proofs, preserve typed work/proof caps and distinguish unasserted availability from false normal
completion. Independent generated runtime challenges remain evidence rather than compiler inputs.
[Plan §3.0](../plans/behavioral-model-forward-plan_2026-09-24.md#30-consolidated-execution) owns this
S1/S2/S3 refinement; S6 native action closure and the rest of Stage 3 remain open.
