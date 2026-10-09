---
id: ADR-0141
title: Share immutable preparation and complete only relevant producing work
status: accepted
date: 2026-10-09
supersedes: []
superseded-by: null
design: ["§B3", "§B7", "§6.2", "§15.11", "§15.12"]
evidence: Implemented
revisit: Qualified native membership and shared preparation cannot serve required complete inputs without disproportionate unrelated work, or scoped terminality cannot preserve final global failure and drainage.
---

## Context

The [native-efficiency review](../design_review/reviews/design_review_native-execution-efficiency_2026-10-09.md)
identifies owner/key enumeration, repeated immutable-input preparation and decoding, polling,
global contribution waits, retention-dependent initialization and excessive observation.
The operator accepted RC01–RC05 on 2026-10-09. The [independent target review](../design_review/reviews/design_review_native-execution-efficiency-plan_2026-10-09.md)
accepts the Proposed correction. Implementation now has independent source assessment and focused
query/session/lifecycle passes; coordinated qualification remains open in the receipt owner.

ADR-0138 retains canonical native state, independent admission and final closure. ADR-0140
retains exact product dependencies, fresh replay ownership and pinned serving. Their incidental
access and preparation mechanisms need refinement for many overlapping owners/roots and readers.

Execution exposed a transport boundary beneath native owners: SDK3.3.0 stops gRPC consumers at
application End before checking physical EOF/trailing status. Independent inspection finds the
same gap in published3.3.2. PSE-arrow's locally patched WebSocket is a useful lifecycle precedent,
but still buffers complete query results. Server-side WebSocket `query_stream` exists; the Rust
SDK does not integrate it. A URL change therefore cannot preserve progressive broad reads.

## Options

1. **Keep point enumeration and per-check reconstruction.** Local mechanisms are simple, but
   absent owner/key combinations and unchanged inputs repeatedly incur native work. Global
   waits couple local output to unrelated readers, and optional eviction can split live work.
2. **Share exact preparation and scope producing work (selected).** Existing completed views,
   spillable ordering, DataFusion logical providers and Moka retention remain useful. Small
   typed owners close their actual gaps without changing independent semantic checks.
3. **Bulk native IN or a universal graph executor.** SurrealDB 3.3 compound prefixes may expand
   combinations; Union retains result-sized distinct state. A universal executor adds policy
   and lifecycle interpretation without a concrete consumer. Neither removes these obligations.
4. **Typed/spilled intermediate computation followed by final native materialization.** Credible
   if qualified native preparation still performs disproportionate work; not a parallel backend
   selected now. Revisit the placement through its owning decision when that trigger occurs.
5. **Replace gRPC with WebSocket.** The existing SDK path buffers query results and lacks the
   current backup surface. A progressive `query_stream` adapter with equivalent cancellation,
   session ownership and backup support remains a credible future transport implementation;
   it is a larger integration than correcting the identified physical-terminal gap.

## Decision

Core resolves model-declared ordered roles/selectors against exact completed inputs once.
Compatible consumers share immutable descriptors and physical preparation, while independent
validators retain full negative universes, their own state and current acceptance. Captured
providers additionally retain their actual budget and operational lifetime scope; input identity
does not merge those owners. Selected logical ports are explicitly rebound, not reinterpreted
through a mutable shared catalog. No SQL statement permission is added.

Native reads select actual exact-view memberships before rich hydration. Deterministic point
reads remain suitable for one verified owner; qualified scalar equality-prefix access and
verified pointers replace multi-owner Cartesian enumeration. Repeated broad consumers may retain
charged compact exact-view selection and independent spill cursors. Sparse queries do not require
whole-view preparation. Native plan qualification must reject relocated Cartesian/Union distinct
state and preserve conflicts, ordering, projection, empty-demand validation and terminality.

Contribution completion closes and drains its producing work, including relevant read descendants,
before the native descriptor/visibility handoff. An opaque native-owned terminal scope binds
this handoff to the actual contribution; immutable view identity alone does not classify work.
Unrelated readers cannot extend that local wait. Global operation failure, final content freeze,
seal and abandonment retain their existing drainage and certainty obligations. Locally complete
output is not publication authority. Submitted arguments, charges and clients survive caller
cancellation until actual acknowledgement and transport cleanup.

Native handoffs use wakeups with explicit synchronous and asynchronous consumers; long-lived
read drivers cannot occupy dispatch capacity needed by writes they await. Pinned serving owns
stable charged live flights independently of optional Moka retention. Pressure may drop retained
values without splitting a loader; close fences new work and drains before reader invalidation.
Extraction controls explicitly select observation demand or complete inventory and reject
undeclared reads; required workspace admission and independent complete controls remain.

Portable product data decodes once into charged typed batches for canonical checks, current
predicates and fresh ingress. Retain the actual singleton-output read for cold capture until a
duplicate-equivalent submitted fold can be compared to acknowledged native completion without
another whole-output owner. Cache acknowledgement/readback and independent stored-state admission
remain distinct protections. No compiler/test threading cap or deadline increase follows.

Retain progressive gRPC for this correction and backport checked physical drainage into the
exact3.3.0 SDK through the existing local-source patch mechanism. Keep application framing and
late errors checked through transport termination; intentional receiver cancellation remains
cancellation. This establishes a concrete finality correction, not a proven cause for every
historical HTTP/2 failure. Qualify it against actual native selection and transport controls.

## Consequences

Useful work follows actual membership and stable inputs, with local ownership rather than
global barriers. Scalar access trades key-level crossings for avoiding absent owner/key work;
compact preparation amortizes repeated broad demands but retains visible startup/charge costs.
Full-universe checks still perform their necessary fold. Private scope/flight state must handle
cancellation, late failures and close races; focused controls precede integrated qualification.

The [native-efficiency companion](../plans/native-execution-efficiency-plan_2026-10-09.md) owns
NE0–NE9 details; the [persisted coordinator](../plans/persisted-graph-execution-plan_2026-10-07.md#8-sole-finding-disposition)
owns source findings and combined acceptance. Acceptance establishes this target, not completed
implementation, measured benefit or closure. No published artifact-format migration, operator
activation, protected evaluation, dependency upgrade or alternate canonical store is implied.
