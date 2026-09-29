---
id: ADR-0081
title: Bound agent packets and reuse explicit compilation stages
status: accepted
date: 2026-09-28
supersedes: []
superseded-by: null
design: [§14.9, §14.10]
evidence: Proposed
---

## Context

PR4 exposes complete catalog contracts, typed selection and addressable retrieval units.
Two behavioral packets exceed the expanded response limit because optional enrichment is
hydrated with the required invocation contract. Agents also need scoped vocabulary, independent
evidence discovery and comparison without a second classifier. Rebuilding retrieval currently
requires orchestration through the full compiler. The API/evidence product owns these boundaries
in §14.9–14.10; forward-plan PR5 owns execution and qualification.

## Options

1. **Keep full hydration and trim afterward.** Smallest change, but it loads unrequested
   behavior and can discard a required signature or fail because of optional content.
2. **Separate mandatory contracts from typed optional sections.** Selected: one existing
   operation route, explicit completeness, and bounded independent section hydration.
3. **Introduce generic incremental computation or a recipe service.** Rejected for this scope:
   neither has a consumer that the existing canonical facts, pure derivations and artifacts
   cannot satisfy. A finite stage plan is easier to invalidate and test locally.

## Decision

`cpg-schema` owns generated request/response contracts. `get_operation` has a tagged packet or
section view; section cursors bind generation, member, section and representation. The mandatory
core preserves complete signatures, defaults, invocation, provenance and limitations. Optional
behavior, relationships and evidence associations have explicit availability and continuation.
Original bytes remain at `get_evidence`. Default/expanded limits are 32/256 KiB; an indivisible
required contract is refused rather than truncated. FastMCP receives concise text plus structured
content, with the final serialized MCP result checked in addition to domain admission.

Browse uses canonical ownership domains and schema-derived vocabulary. Independent evidence
retrieval ranks units, including release-only material, using existing lexical/vector channels
and family fusion. Comparison composes the existing classifier and preserves caller order,
ambiguity, contradiction, uncertainty and witnesses. Python remains transport/ranking glue.

Coarse catalog and retrieval rebuilds consume validated published facts. Explicit stage keys
include complete membership, absence, coverage, input/provider/root/profile identities, source
coordinates, attribution, transformation/schema identity, rendering and the complete embedding
specification. The full compiler digest remains provenance, not the only reuse key. A catalog
rebuild publishes a fresh validated snapshot with schema-owned envelope rebinding; a retrieval
rebuild retains its canonical snapshot and produces a new immutable artifact receipt. Invalid
canonical inputs refuse; invalid disposable reuse recomputes. No partial ready state or automatic
selection. Clean/reused equality includes evidence and provenance.

## Consequences

The current-only migration removes superseded public encodings and runtime generations once the
replacement is validated; benchmark artifacts survive. No new cache authority, inference engine,
rollback generation, compatibility decoder or generative evaluation is introduced. Mandatory
contract growth beyond the expanded budget, or a demonstrated workload requiring finer reuse,
reopens this decision. Acceptance establishes the direction; implementation and bounded testing
remain open in the forward plan. Comparative confirmation remains PR6.
