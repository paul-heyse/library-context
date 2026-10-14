---
id: ADR-0168
title: Execute against exact completed selections independently of publication prefixes
status: accepted
date: 2026-10-14
supersedes: [ADR-0105]
superseded-by: null
design: [§15.1, §15.11, §3.5, §6.1]
evidence: Proposed
---

## Context

The [Catalog creation review](../design_review/reviews/design_review_catalog-creation-performance-target_2026-10-14.md)
F02/F03 identifies unrelated cumulative execution barriers and repeated same-operation validation.
The operator approved RC01/RC02 on2026-10-14. ADR-0105's requirement that every private reader wait
for a whole closed publication prefix prevents exact relation readiness; its immutable publication
and cumulative-frontier guarantees remain necessary. The native realization is governed by
ADR-0145/0165/0166, replacing the older PostgreSQL-specific physical mechanisms.

## Options

1. Retain universal group/prefix execution barriers. This keeps the coordinator serial but makes
unrelated work a prerequisite and supplies input cutoffs through arrival order.
2. Execute from declared exact completed contributor sets, while separately closing immutable
publication prefixes. This requires coordinated selection/admission migration but exposes actual
independence without another scheduler, canonical store or portable validity cache.
3. Remove group edges and select latest completed rows. This is simpler locally but lets completion
arrival change consumer meaning, reuse keys and published prefixes; it is rejected.

## Decision

**Accepted target,2026-10-14:** choose option2. The model retains direct relation, invariant and
contributor-to-assembler prerequisites and ordered input-role recipes. Runtime resolves those recipes
to protected immutable completed owner sets. Native composition establishes canonical membership,
deduplicated counts and conflict refusal; metadata alone grants no read authority. Persist exact
bindings before exposing them. Binding one complete relation does not close its publication prefix.

The compiler alone coordinates readiness, owned private stage results and ordered whole-prefix
visibility. Genuine predecessors, all required vocabulary contributors, input invariants, explicit
empty outputs, EOF, coverage and failure/drainage remain necessary. Equal identities/equal payloads
coalesce; unequal payloads refuse. Earlier bindings never widen. No unfinished ordinary output or
assembly authorizes reads. Changed dependency meaning changes recipe/specification identity.

Facts, Normalized, Analysis and Catalog remain cumulative frontiers in one self-contained attempt;
ordinary results remain one-shot immutable owners. Intermediate completion grants neither publication
nor serving admission. Final admission establishes complete model-owned relation/coverage/frontier
meaning, current physical ownership, originals and manifest inventory. Detached/cold/foreign inputs
earn their own admission. No compatibility reader, linked-generation rebuild protocol or second store
is introduced. Current native access, spill, resource and finality contracts supersede obsolete
PostgreSQL-specific implementation clauses in ADR-0105.

RC02 additionally permits private reuse of genuinely executed individual checks and ordered search
expectations inside the same protected operation lifetime. Bind complete ordered roles, exact views,
requested/resolved epochs, model/profile/configuration, executable definitions/factories, applicable
NotRequested/companion premises and actual native/resource ownership. Terminal successful execution
is required; digest equality or a prior whole-scope success cannot mint individual evidence.
Changed/new applicability earns fresh checks. Actual native freeze, graph/frontier/original inventory
and fresh search/lexical readback retain their independently necessary checks. Failed/cancelled or
closed owners yield no reusable success. Expected runs retain charged backing and rewindable cursors
through final comparison; cold/foreign operations derive expectations independently.

Preparation ownership follows running plus remaining selected consumers, including cache and optional
branches. Release capacity only after the final actual borrower and worker/native drainage. Cancellation
signals siblings and preserves original outcome and cleanup uncertainty separately.

## Consequences

Useful independent work can overlap while completed selections and publication remain deterministic.
The migration crosses model recipes, compiler dispatch, native composition and independent admission;
these consumers must move together. Initial CC01 retains normalized admission and current root fencing.
No timing gain or whole-Catalog qualification is claimed. ADR-0105 is retired to Git after
current governing links transfer; dated reviews retain their original diagnoses. CN01/CN02 implementations remain scheduled
in the [coordinator §6](../plans/catalog-creation-plan_2026-10-14.md#6-sole-disposition-and-execution-checkpoint).

D02 retains root fencing: delayed requests, accepted effects, global freeze, Unknown and retirement
need the current complete protocol. Revisit only for material observed contention or a simpler complete
producer-owned replacement, with separate rule disposition and independent protocol assessment.
D04 retains flat published witnesses and permits internal predecessor arenas. Revisit if residual
published prefix storage/reconstruction is material after CA02. RC03 remains unused; semantic v4 is
unchanged. ADR-0108 and ADR-0145/0151/0165 remain in force.
