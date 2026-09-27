---
id: ADR-0059
title: Bind synchronous context protocols to pinned classes and source lifecycle sites
status: accepted
date: 2026-09-27
supersedes: []
superseded-by: null
design: [§B5, §9.9, §11.3]
evidence: Proposed
revisit: A supported context protocol needs arbitrary alias or deferred execution semantics, or a provider supplies runtime protocol facts that can replace the authored assertion without changing its proof boundary.
---

## Context

ADR-0057/0058 accept ordered synchronous entry/exit and channel-specific proof. Existing call
models apply to an observed callable only on exact authored/provider phase equality. That rule
cannot express source-reached implicit exits when the provider omits them. The pinned provider
probe records `nullcontext` and `suppress` construction and entry but no exit; the typeshed view
reports inherited `AbstractContextManager.__enter__` for `suppress`, while CPython 3.14.7 defines
its runtime entry separately. Class definitions have no callable signatures; initializer
signatures are separate facts. See the [source probe](../design_review/evidence/2026-09-27_sync-contexts/README.md).

## Options

1. **Require observed protocol methods and extend call application.** Reuses the smallest
   apparent interface but leaves supported exits unanalyzable and makes runtime behavior depend
   on a stub's inherited method presentation. Reusing an initializer as a class identity loses
   a semantic distinction.
2. **Authored class protocols, with explicit source lifecycle proofs (chosen).** Reuses the
   existing catalog pin/identity machinery, expression evaluation, pure completion and proof
   publication. Provider observations and synthetic model assertions retain separate authority.
3. **General protocol interpretation or a second runtime executor.** Requires arbitrary alias,
   object-state, metaclass and deferred-execution semantics beyond Stage 3, with another owner
   of evaluation and cleanup order.

## Decision

Add a typed synchronous class-protocol category to the existing model catalog. Bind the class
and required construction roles to their own exact pinned definitions; initializer signatures
admit argument shape, never stand in for class identity or prove runtime behavior. Authored
entry-result and exit-transition assertions supply runtime meaning. A missing provider exit
observation is not a fabricated call, and a stub's inherited entry remains attributed as such.
Existing observed-call models retain exact phase matching unchanged.

Source admission proves a fresh direct construction at a synchronous `with` item, stable unique
callee binding, the declared constructor target set, and source-ordered argument completion and
binding. Identifier/annotation constructor observations are not executed calls; admission matches
the explicit source role and construction occurrence. Broader aliases, custom metaclasses, unknown targets, asynchronous/generator execution
and unsupported target assignments remain open. The fresh manager occurrence is distinct from its entry result; a return of an `as` binding
requires its own value-transfer certificate. Model applicability, a reached site, entered
state, positive effects and complete coverage are independent facts.

The existing pure completion owner evaluates context items in order. Successful entry registers
its exit before `as` assignment. A failed entry does not register that exit; a later expression,
construction, entry or assignment failure unwinds every previously entered context in reverse.
Exit transitions consume a typed pending outcome: suppression changes only Raise to Normal;
normal exits preserve Return/Break/Continue; an exit-raised exception replaces the pending outcome
before the next outer exit. Unsupported or bounded work retains an explicit unknown, never
assumed completion. Exception groups and user-defined class matching remain outside the initial
exact builtin exception domain. Suppression uses its own modeled `issubclass` operation;
sharing pinned MRO facts with handler matching does not identify the two semantic operations.

Persist model/site/action evidence with structural identity, source and phase applicability.
Source publication reconstructs admission; native consumers share structural proof admission.
Neither Python serving nor a model-name-specific classifier repeats lifecycle semantics. Initial
CPython `nullcontext` and `suppress` declarations exercise this category; additional models using
these transitions add declarations and focused controls. New transitions require a deliberate
contract extension, not an untyped callback or arbitrary executable model.

## Consequences

This is an additive implementation decision under ADR-0057/0058, not a replacement of call
models or a claim of completion. Implementation is **Proposed**; source/provider observation is
**Tested** only by the narrow probe. Models need independent CPython challenges and real
source→Delta→native positive/withholding controls, including partial entry, assignment failure,
reverse cleanup, suppression, outcome replacement and unsupported async cases. Source/native
contracts migrate together. [Plan §3.0 S2b/S3a](../plans/behavioral-model-forward-plan_2026-09-24.md#30-consolidated-execution)
owns execution and §6 remains the finding-disposition owner. No Stage 5 work is admitted.
