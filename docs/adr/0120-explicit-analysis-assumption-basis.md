---
id: ADR-0120
title: Preserve explicit premises through finite analysis and served claims
status: accepted
date: 2026-10-03
supersedes: []
superseded-by: null
design: [§B8, §3.9, §15.3, §15.8]
evidence: Interface-checked
revisit: A new conditional analysis cannot express its premise as a supported lower observation or pinned model definition without a qualification cycle.
---

## Context

The [code-facts coordinator](../plans/code-facts-expansion-plan_2026-10-03.md) schedules typing-based
finite refinement alongside conservative runtime results. Existing assertion qualifications record
context, scope, condition, modality and approximation, but cannot distinguish an unconditional
result from one requiring typing conformance or a modeled dispatch universe. Renderers cannot
recover that distinction after transfer and summary composition.

## Options

1. **Keep typing as characterization only.** This retains the existing conservative behavior, but
   cannot deliver the planned conditional analytical results. It remains the correct policy for
   observations without an admitted refinement question.
2. **Store ad hoc premise flags in each producer/output.** Small initially, but spreads composition,
   generation resolution and identity rules through Local, execution, Summary and serving.
3. **Required model-owned canonical assumption sets** (chosen). A shared qualified claim identifies
   its premises once; composition and every output retain that basis mechanically.

## Decision

`AssertionQualification` requires an explicit canonical assumption-set reference. Empty sets are
explicit. Typed assumption members refer to supported lower observations or pinned model/universe
definitions, never the conditional assessment whose qualification would create a content-key cycle.
The shared model validates membership and context/environment compatibility.

Conjunction unions compatible premise sets through an explicit resolver and materializes the
canonical set before publication. Alternative union groups only matching complete governing bases;
unconditional and conditional alternatives remain separately qualified. Missing, incompatible or
unsupported premises refuse the conditional operation; they never become an empty basis.

Question-specific model policy decides whether a premise permits characterization, a typed answer
or runtime refinement. There is no provider-wide trust switch. A conservative runtime path remains
available when typing supplies an additional conditional derivation. Pinned universe membership
uses the actual authored model and digest; a supplied arbitrary digest is not a closure certificate.
Late sets and support are published at their producing vocabulary epoch, not predicted by preflight.

## Consequences

Transfer, Summary, selection, synthesis and packets share one basis contract. Served claims resolve
premises to actual lower observations, native support and original source or pinned model evidence.
Stored and wire formats hard-migrate; a missing basis is invalid. New conditional decisions require
explicit supported assumptions and cannot silently strengthen an existing unconditional result.
The finite BDD kernel, five verdicts and programmatic synthesis remain the analysis mechanisms.

B0 implementation and scoped qualification are in progress; B1/B3 activation and assembled Q0 are
pending. Acceptance establishes this decision, not tested closure of those packages or a retrieval
benefit. The coordinator owns current findings and qualification.
