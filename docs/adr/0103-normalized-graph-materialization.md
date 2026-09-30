---
id: ADR-0103
title: Materialize typed graph snapshots with each normalized generation
status: accepted
date: 2026-09-30
supersedes: [ADR-0102]
superseded-by: null
design: [§15.4, §15.5, §15.10]
evidence: Proposed
---

## Context

Library compilation captures a whole pinned input and reconstructs one self-contained generation.
There is no partial update lifecycle. Reconstructing the same deterministic topology for each
analysis request adds work without changing its meaning. The operator selected generation-owned
serialized petgraph snapshots on 2026-09-30. N1–N5 are implemented with focused controls; N6 is
in progress. This record preserves the normalized semantic decisions formerly in ADR-0102.
[DESIGN §15](../design/sections/semantic-model.md) owns the contracts and the
[Phase 3 plan §11](../plans/semantic-model-phase3-detailed-plan_2026-09-30.md) owns execution status.

## Options

1. Build every projection on demand from canonical relations. Simple storage, repeated topology
   construction and SQL reads for an immutable snapshot. This former choice is replaced.
2. Persist canonical records and versioned computational graph snapshots together. Selected:
   construct once per collection lifecycle, hydrate for repeated analysis, retain relational evidence.
3. Make serialized graphs canonical or retain incremental graph caches. Adds identity and invalidation
   authorities without a current consumer. Rejected.
4. Use StableGraph for serialized topology. Its deletion stability is useful for mutation; this
   lifecycle rebuilds immutable graphs and never exposes indices. Compact Graph preserves the wider
   algorithm trait surface and needs no hole handling. Revisit only for an actual mutation consumer.

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

Declare role-based program projections with an explicit universe, typed parallel arcs, evidence
and unresolved side records. Construct all built-in projections during normalization, once per
input/context in each fresh generation. Serialize the actual immutable directed petgraph Graph
using its serde-1 support and pinned Postcard binary encoding. Store a versioned wrapper and bounded
BYTEA chunks through the ordinary generation store. The wrapper identifies input, analysis context,
named projection, projection/format version and petgraph version. Domain IDs inside weights are
stable; dense indices are private computational coordinates, never durable domain identities.

Canonical normalized relations own meaning. Graph bytes are a reproducible computational
materialization, checked against the canonical projection before publication. Readers hydrate the
snapshot and validate its wrapper and structure; they do not requery and reinsert canonical edges
for every request. Every new collection reconstructs snapshots unconditionally. There is no graph
reuse digest, cache invalidation protocol, compatibility reader or cross-generation artifact link.
Existing generation receipts still protect all stored contents uniformly. Full paths and closures
are not materialized. PostgreSQL remains the only durable substrate.

## Consequences

Normalization spends graph construction/serialization work once and stores extra bytes. The
shared attempt budget covers graph, encoding, decode validation and adjacency/index state; bounded
chunks avoid a single unbounded relation row. Publication requires all named snapshots, including
empty graphs and explicit incomplete availability. Failure cannot publish a partial normalized
frontier. A format/library change rebuilds project generations from pinned inputs.

P4 retains analytics, dispatch expansion, composition, guard/type derivation, summaries and catalog
conclusions; P5 retains serving. Hydration performance remains unmeasured until Q. Acceptance does
not establish implementation or verified closure. Revisit for an incremental-update consumer,
measured graph storage/hydration pressure, or new topology semantics.
