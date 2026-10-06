---
id: ADR-0129
title: Bind source-field inventory to complete class-owned premises
status: accepted
date: 2026-10-06
supersedes: []
superseded-by: null
design: ["§15.6"]
evidence: Proposed
---

## Context

Graph-native remediation migrates CallableAspects and necessary source-field admission to
class-owned dependency scopes. The prior `symbolic_fields::ClassInventory` fingerprints every
CallableAspect input row for every class, including unrelated bodies. Preserving that incidental
invalidation would require global row fingerprints alongside the new owner-scoped operation.
The [coordinator §9](../plans/graph-native-pivot-plan_2026-10-05.md#9-remediation-and-improvement-execution)
owns the authorized hard pivot and remaining F01–F12 implementation acceptance.

## Options

1. **Retain whole-input inventory hashes.** A compact global fingerprint stream could preserve
   old keys without retaining every rich row. However, unrelated membership still invalidates all
   classes and every class operation depends on global input preparation. No current consumer
   requires that invalidation or old artifact identity.
2. **Fingerprint complete class-owned premises.** Include the actual class body, applicable
   context, source membership, native support and coverage, together with the dependencies needed
   for field/default/constructor/reader judgments. Preparation follows the dependency closure and
   unrelated bodies do not become inventory premises. The owner must include unsupported body
   members and absence-sensitive membership, not just rows that produced a successful field.

## Decision

Select option 2. `lctx-model::domain::normalized::symbolic_fields` owns the inventory meaning and
necessary property checks; `cpg-core` selects the complete declared class scope before rich
hydration. Revise the inventory digest domain and shared normalization policy. Preserve explicit
uncertainty, all relevant members and source/context fidelity. Remove the old whole-input digest
path; no compatibility reader or historical artifact retention is required.

This follows efficient-architecture H5, H13 and H17: use sufficient dependencies, propagate
relevant changes and release unrelated rich state. It does not establish runtime field allocation,
heap identity, mutation stability or a stronger behavioral verdict.

## Consequences

Unrelated sibling edits no longer alter every source-field inventory. Added or changed actual
class members, including unsupported syntax, remain relevant and must invalidate or refuse the
corresponding association. Fresh artifacts and native realizations must be rebuilt under the new
normalization policy; earlier keys are not retained.

The decision is accepted, while implementation and targeted functional verification are in
progress. Controls must establish complete owner membership, unrelated-sibling independence and
adverse source/default association refusal. The coordinator remains the sole finding disposition
owner; this ADR closes no finding. Revisit if a concrete consumer requires a broader premise than
the complete class-owned closure or if an actual membership omission is found.
