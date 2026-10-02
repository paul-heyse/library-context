---
id: ADR-0114
title: Derive serving from canonical generations and prepare bounded consumers
status: proposed
date: 2026-10-01
supersedes: []
superseded-by: null
design: [§15.12, §11, §14.7, §14.8, §14.9]
evidence: Proposed
---

## Context

Phase 4 produces typed Catalog generations, but serving remains unavailable. Retained serving still
uses legacy projection tables, manifest-only pins, per-request selection preparation and native
condition authority. The [Phase 5 plan](../plans/semantic-model-phase5-detailed-plan_2026-10-01.md)
develops replacement consumers and focused foundation improvements under ADR-0085/0086.

The canonical embedding uses store exact little-endian f32 bytes. The inspected SQLx/pgvector
boundary binds decoded vectors but supplies no direct bytea conversion for a generated view.
Ranking a selected 1,024-dimensional exact profile therefore needs an explicit physical realization
without restoring a copied semantic serving schema.

## Options

1. **Direct generated views and request-local reconstruction.** Smallest storage transition, but
   repeats stable catalog/native preparation and leaves vector-byte conversion unresolved.
2. **Generated semantic views, admitted generation preparation and a narrow disposable vector
   artifact (proposed choice).** Reuses model operations, keeps one lifecycle and canonical store,
   and localizes numerical conversion to the shared codec. Costs preparation memory, cache
   publication/cleanup and qualification of its link to canonical content.
3. **Restore old bundle/import/projection tables.** Operationally familiar, but retains a second
   schema/semantic authority and conflicts with the accepted hard cutover.
4. **New SQL binary decoder or general projection/query language.** Avoids the vector artifact
   in principle, but adds unqualified generic machinery and another codec realization without
   a present benefit sufficient to justify it.

## Decision

This is a proposed refinement, not yet a decision in force.

Keep serving meanings, DTO mappings, request decoding, classification, finite native operations
and ranking/fusion policy in lctx-model. Generate semantic views/grants/indexes through the canonical
generation lowering before publication. No Serving frontier or second readiness registry is added.

Admit the read-only service against one canonical generation. Retain a process generation guard,
use bounded same-generation request connections, and release request connections before CPU work.
Guard loss is terminal; cancellation retains native capacity/guard until actual completion.
Prepare a narrow, charged classification inventory once, with receipt/content checks and a strict
broad replay route; original bodies and full explanations hydrate on demand.

Retain bm25s as numerical scoring and exact pgvector as vector scoring. Rust owns ranking decisions
and witnesses once. Python remains numerical adapter, transport and presentation.

Permit only a disposable vector artifact containing generation-qualified retrieval-use keys and
numerical vectors plus a derivation manifest. Derive it through the current embedding codec; keep
all identity/context/text/evidence/spec meaning in canonical relations/views. Prepare explicitly
under a pinned generation and publish atomically; never build or repair it during read-only startup.
Validate against canonical content, refuse required corruption and purge artifacts with generation
cleanup. Do not add persistent foreign keys into generation schemas.

Ranking/display policy and wire representation have separate identities, allowing preparation
rebuild and cursor invalidation without re-extracting unchanged canonical facts. Unit rendering
and embedding-spec changes retain their canonical invalidation boundaries.

## Consequences

The target removes duplicate classification/fusion and limits representation changes to owned
mappings. It adds preparation and cache lifecycle obligations; the artifact is not itself evidence
of retrieval quality or faster execution. The initial vector route remains the selected 1,024 exact
profile, not every dimension accepted by the canonical codec.

The native exact-request-scalar operation must be reconstructed through current conditions and
checked Entry/proof links; existing structural typing does not establish that interface. Finite
model compatibility and path-local refutation do not establish feasible execution or whole-operation
absence.

[DESIGN §15.12](../design/sections/semantic-model.md#section-15-12) identifies this proposed refinement;
the plan owns implementation/verification and the parent cutover §8 owns existing finding disposition.
Acceptance of this record requires the scoped target decision; implementation, native/MCP journeys
and functional/hygiene qualification remain not_run. Revisit if complete metadata preparation exceeds
the admitted envelope, the selected exact vector profile cannot be faithfully derived, or a new
consumer requires stronger native semantics. No new product/ANN scope is authorized by this record.
