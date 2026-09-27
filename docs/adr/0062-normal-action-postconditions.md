---
id: ADR-0062
title: Keep normal-exit model postconditions separate from activated actions
status: accepted
date: 2026-09-27
supersedes: []
superseded-by: null
design: [§9.9, §11.3]
evidence: Proposed
revisit: An outcome-qualified claim needs disjunctive exit domains, alias or resource-pair identity, or a consumer cannot preserve its undischarged outcome obligation.
---

## Context

ADR-0060 requires independently proved triggers before activating an action. This correctly
withholds completed serialization, callback registration and acquisition for general fallible
calls, even after compiler95 proves their invocation inputs. Their exact pinned declarations
still supply useful postconditions: a normal return entails the authored action with its declared
modality. Conflating this implication with a proved outcome would invent execution. Keeping only
an undifferentiated refusal hides a meaningful part of the behavioral model. The Stage 3 plan
requires both typed model meaning and honest unknowns through composition and serving.

## Options

1. **Only activated assessments and refusals.** Retains the current safe implementation, but
   exposes no outcome-qualified behavior for common fallible APIs until arbitrary input-dependent
   success can be proved. More model declarations alone cannot address that gap.
2. **Treat reaching the call as sufficient, or add a success BDD atom.** Either weakens Normal
   or creates an execution premise without evidence. Exceptional is not the complement of Normal:
   divergence and open outcomes remain possible. Rejected.
3. **A separate, typed normal-exit postcondition (chosen).** Reuse exact candidates, binding and
   reached invocation. Retain Normal as an explicit undischarged obligation in a distinct relation;
   shared admission and every consumer preserve its implication meaning. No generic interpreter,
   per-API completion exception or second model catalog is introduced.

## Decision

Keep ADR-0060 activated assessments unchanged. `cpg-schema::action` additionally owns a typed
postcondition over the same effect/callback/resource candidate: under this exact reached and
bound invocation, **if this callee returns normally**, its authored Normal rule holds with the
original Definite/Potential modality. The relation requires the same closed exact target,
invocation phase, candidate descriptors, input/schema bindings and ordered call-input proof.
It does not assert that Normal is feasible, occurs or exhausts the outcome domain.

Only authored Normal rules enter the initial relation. Invocation rules already have assessments;
Exceptional and Finally require their own explicit domains in a future extension. Refused
invocation, unsupported binding, wrong phase, missing input subjects or capped proof never become
a usable postcondition. A subjectless effect stays subjectless.

For a Normal acquisition rule, its declared return-value path is a **symbolic result endpoint**
inside the implication, not an existing runtime resource or alias. A candidate's call-expression
coordinates are occurrence support only. Admission checks the exact Output/ReturnValue shape;
it creates no resource identity, acquire/release pair, release absence or live resource claim.
Concrete pairing still requires independent value identity and fate evidence. Input resources
and callbacks require their existing exact bound-argument identities.

Analytics composes the relation through the same candidate/invocation admission as assessments;
core acquires inputs and reconstructs publication. No action or expression is promoted to normal,
no `call_transfer` boundary is discharged, and no coverage domain is closed by a postcondition.
Future source/local-callee completion can discharge the exact occurrence's obligation only when
its entry condition has a closed normal-completion domain, not merely one normal witness.
Interprocedural composition must retain the call occurrence, phase and pending outcome obligation;
it cannot convert it to an ordinary path condition or erase it under caller return/finalization.

S6 must render/select these as explicitly outcome-qualified claims, separate from activated
actions and from Potential modality. Until a consumer supports that distinction it must omit
the postcondition or report unsupported, never reuse a success verdict. The immutable executor
uses the same shared structural contract plus raw-fact closure; Python only adapts it.

## Consequences

Implementation is **Proposed**. Positive and withholding controls must separate normal-exit
registration/serialization promises from actual callback invocation or completed transforms,
including a runtime counterexample that enters then raises. Shared mutation controls challenge
obligation removal, foreign occurrence, subject binding, target openness and the existing proof
bounds. Resources also require a symbolic-result versus actual-identity control. Independent
runtime examples challenge the model; they are not compiler inputs. Source/local completion,
concrete resource fates, transform/value composition and S6 serving remain separate work in
[plan §3.0](../plans/behavioral-model-forward-plan_2026-09-24.md#30-consolidated-execution).
