---
id: ADR-0088
title: Preserve complete artifacts as canonical bounded chunks
status: accepted
date: 2026-09-29
supersedes: []
superseded-by: null
design: [§15.3]
evidence: Tested
---

## Context

The typed source foundation stored each artifact body in one row. Its 64 MiB row admission limit
would reject larger original evidence. The operator selected chunked preservation over a file-size
limit while planning the remaining phases 0–2 on 2026-09-29. Source fidelity and bounded transport
must compose through the same semantic owner (ADR-0085/0086).

## Options

1. Keep one body row and explicitly refuse larger files: simplest physical shape, but loses the
   ability to retain complete evidence independently of whether a provider can interpret it.
2. PostgreSQL large objects or an external blob store: separates lifetime and grants from ordinary
   generated generation tables, adding another publication and cleanup protocol.
3. Canonical typed chunk relationships in the generation schema (chosen): uses the existing
   generated codecs, references, COPY, immutability and retirement mechanisms.

## Decision

`SourceArtifact` owns input/path/content identity and complete byte length. `ArtifactChunk` owns
original bytes in fixed 1 MiB chunks identified by artifact and ordinal; only the last chunk may
be shorter. Empty artifacts have no chunks. Chunk size is a model constant, not a transport option.
A model-owned streaming verifier reconstructs the complete length and digest, enforcing contiguous
ordinals and canonical lengths. Sealed publication requires this proof and input-manifest membership.
Chunk corruption, omission or reassignment cannot be hidden by a valid metadata row.

Acquisition captures the same bytes it hashes. A changed source fails the attempt. Interpretation
failure retains original bytes and explicit coverage. No old inline-body reader remains in the typed
model. PostgreSQL is the sole relational store; chunking adds no independent blob lifetime.

## Consequences

Artifact size no longer determines row size. Complete readback uses streams; convenience collection
remains bounded. Domain identity is independent of transport batching. Pure and real PostgreSQL
controls passed on 2026-09-29 with a 65 MiB artifact, empty/non-UTF-8 evidence and corrupted/missing/
noncanonical/misplaced chunks (`cargo test --release -p lctx-model --test domain -p lctx-postgres
--test generations`, through `scripts/build_environment.py`). This is a schema migration.

Acquisition integration and coordinated memory accounting remain open in the
[cutover plan](../plans/semantic-model-cutover-plan_2026-09-29.md). Canonical chunking alone does not
qualify a process memory envelope. Revisit only if measured artifact access requires a different
physical mechanism; any replacement must preserve the same complete-content proof and generation
lifecycle. No phase exit is implied by this bounded evidence.
