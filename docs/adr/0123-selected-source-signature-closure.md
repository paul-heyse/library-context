---
id: ADR-0123
title: Compose known source bodies from exact declared signature closure
status: accepted
date: 2026-10-04
supersedes: []
superseded-by: null
design: [§15.5, §15.6]
evidence: Interface-checked
revisit: Selected declared signatures cannot preserve complete member replay, or composition needs an unknown effective body or ambiguous call target.
---

## Context

The analytical-enrichment qualification retains ordinary authored functions with independently
Known effective identity and body, a complete original call event and a replayed Bound source
signature. Composition nevertheless refuses them because unrelated native-unavailable variants
make the artifact Signatures family Partial. The actual `source_calls` diagnostic establishes
that boundary, 2026-10-04; it does not establish corrected runtime acceptance.
[The coordinator §7](../plans/code-facts-analytical-enrichment-plan_2026-10-04.md#7-current-disposition-and-checkpoint)
owns the repair and qualification. ADR-0103 still governs the sole binder and independent
source/effective authority; ADR-0122 establishes the analogous per-use origin closure distinction.

## Options

1. **Require complete artifact signature coverage for every body.** Simple, but unavailable
   signatures belonging to another callable suppress a fully retained authored body.
2. **Use one visible Bound attempt.** Rejected: missing or undetermined declared variants can
   fabricate uniqueness, and shape metadata alone cannot establish effective body identity.
3. **Replay exact selected declared signature closure** (chosen). The model reuses the existing
   published enumeration, member digest, supports and binder; no new interpreter or provider.
   This adds a private composition premise while retaining global binding uncertainty.

## Decision

`lctx-model` may construct a private source-body signature-closure token from the existing
complete global variant-set assessment, or from the exact complete published Source-role
signature enumeration for the selected callable. Replay all members and their signature/support
receipts in the same input, context and qualification; retain the enumeration and member evidence
in the source proof. A Bound member and every competing declared member must satisfy the existing
unique-binding rules. Missing, foreign, unavailable or ambiguous selected members refuse closure.

This token applies only to source-body composition with an independently Known effective identity,
compatible descriptor and admitted Known body, a canonical source definition, complete original
native call-target event and Summary admission. Stub or synthesized metadata, unknown wrappers,
unknown bodies, annotation calls and open target domains do not acquire body authority.
`BindingSetAssessment` and `EffectiveInvocationAdmission` keep their existing global completeness
rules and truthful Partial outcomes. A scoped composition token cannot promote their status.

## Consequences

Ordinary authored bodies can compose despite unrelated artifact signature losses. Shared model
replay remains the authority, and hydration must retain the exact selected enumeration and members.
Proof consumers gain explicit selected-domain evidence; incomplete/foreign enumeration, selected
unavailable variants, actual ambiguity and unknown effective bodies remain mandatory refusal controls.
The decision is accepted; implementation and focused/full qualification remain pending in the
coordinator. No whole-program completeness, native overload winner or runtime heap claim is added.
