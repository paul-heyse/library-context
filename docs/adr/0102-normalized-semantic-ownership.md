---
id: ADR-0102
title: Normalize evidence through typed entities, complete events and stored policy admissions
status: accepted
date: 2026-09-30
supersedes: []
superseded-by: null
design: [§15.4, §15.5, §15.10]
evidence: Proposed
---

## Context

P0–P2 provide attributed provider-qualified symbols, calls, signatures and structural types.
Phase 3 must establish shared identities and policy views without turning provider agreement,
names or partial observations into runtime certainty. The current call kernel reasons over raw
provider targets; native signatures describe undecorated callables. Dormant normalized SQL and
graph declarations contain useful expectations but also legacy identities and classifiers.

[DESIGN §15.4, §15.5 and §15.10](../design/sections/semantic-model.md) own these contracts.
Existing C04, composition F04 and related P3/P4 obligations remain under the
[cutover plan §8](../plans/semantic-model-cutover-plan_2026-09-29.md).

## Options

1. **Mechanically port the dormant SQL and graph catalogs.** Fastest apparent restoration, but
   preserves competing identity/classification rules and cannot express missing evidence reliably.
2. **Use typed normalization with separate Rust and SQL policy predicates.** Improves records while
   leaving two policy authorities whose agreement must be maintained manually.
3. **Introduce a generic semantic/predicate language.** Could generate both forms, but its algebra,
   interpreter and versioning impose unnecessary work for five finite policy operations.
4. **Use ordinary typed domain operations and stored policy admissions.** The model owns meaning;
   SQL joins, tables and views lower the results. This is selected.

## Decision

**Accepted target:** establish nominal entities from declaration occurrences or explicitly qualified
synthetic/external identities. Preserve all correspondence evidence and total unresolved/ambiguous
outcomes. Source and stub declarations remain distinct; public exposure is a relationship rather
than an alternative entity identity. Reuse existing occurrence, module, type and place owners.

Keep source signatures separate from effective-callable assessments. Exact supported builtin
descriptors may establish binding behavior; arbitrary decorator transformations remain unknown
with the current producer envelope. Metadata recognition does not imply body admission.

Assemble complete normalized call events before policy filtering. Retain origin, context,
channel, phase, open remainders and all evidence. Agreement never strengthens modality or
completeness. One model evaluator produces total policy assessments and typed admission rows
for Invocation, Dataflow, Summary, Usage and Association. Generated views select membership
by policy code; they do not reimplement predicates. Association may retain Potential evidence
without making it an invocation or a summary premise.

Use the sole binder on complete raw arguments/signature variants and receiver premises. Persist
every attempt, including refusal and missing-signature outcomes. Shared validators recheck
members and digests; a private composition admission is constructed only from validated complete
event and binding inputs. The occurrence-owner rule supplies ownership.
Source-signature inspection and effective-invocation authority remain distinct on binding attempts.
The composition token additionally requires known effective-target/context/descriptor and body
admission for the compatible variant; successful source binding beneath an unknown wrapper is
insufficient.
The complete variant-set assessment distinguishes proven incompatibility from undetermined
semantics; one successful binding beside an undetermined variant cannot establish uniqueness.
Replace raw target/signature symbol equality with a private normalized applicability token backed
by same-entity, same-input/context evidence. Keep original provider rows and the one binding
algorithm; normalized equivalence must not be implemented by relabeling a raw observation.

Declare role-based program projections with an explicit entity universe, typed parallel arcs,
evidence and unresolved side records. Build bounded immutable petgraph graphs on demand. Dense
graph indices are private mechanics, never semantic IDs. Full paths and closures are not stored.

The exact supported envelope and work packages are in the
[Phase 3 plan §§3–5 and §8](../plans/semantic-model-phase3-detailed-plan_2026-09-30.md).

## Consequences

Consumers share identities and policy decisions while retaining uncertainty. This adds explicit
assessment/member records and validation work, and requires independent fixtures rather than
only generated self-consistency tests. P4 retains dispatch expansion, composition, guard/type
derivation, summaries and catalog conclusions; P5 retains serving. Unknown effective behavior is
an honest supported outcome, not permission to discard source signatures or evidence.

Reconsider a policy language only for a concrete requirement for independently extensible
policies. Expand effective-known claims only after typed producer evidence and independent controls
support them. Acceptance does not establish implementation or close a finding; the linked plan
owns implementation status and qualification.
