---
id: ADR-0086
title: Publish immutable PostgreSQL generation schemas from the model
status: superseded
date: 2026-09-29
supersedes: [ADR-0083]
superseded-by: ADR-0128
design: [§15, §1.2, §B2, §B6, §B7, §B12, §B13, §B14, §3.1, §3.2, §3.3, §3.4, §3.4.1, §3.5, §3.7, §3.8, §4.0, §4.1, §4.3, §5, §6, §6.4, §6.5, §8, §9, §10, §10.5]
evidence: Proposed
---

## Context

The operator retains PostgreSQL as the sole relational store and accepts discontinuing the old
runtime while layers are reconstructed. The partition lifecycle in the first phase-0 implementation
cannot retire self-referencing relations safely (core review C01), and readers omit the schema
digest check (C02). Cross-generation parent scans have no required consumer.

## Options

1. Repair partition attach/detach and resumable retirement. Retains substantial lifecycle machinery.
2. Ordinary generated tables in one schema per immutable generation (chosen). Self/mutual foreign
   keys remain local, publication is a registry/grant transaction, retirement drops one isolated schema.

## Decision

PostgreSQL 18 remains the only canonical relational store. A stable control schema owns generation
metadata, manifests, events and selection. Each generation owns a schema of ordinary tables produced
from the same validated model. There are no shared partition parents or ATTACH/DETACH operations.
Tables/keys precede foreign-key installation so reference cycles are supported. Keys and references
remain generation-qualified. Persistent dependencies into generation schemas are forbidden.

Lifecycle: staging → sealed → validated → published; failures remain invisible. Selection is a
separate reference. Sealing waits for active writes and removes writer privileges; validation concerns
the sealed stored contents, not a caller promise. Publication changes grants and state atomically.
Readers check model and physical-schema digests and hold a generation lease. Retirement requires an
unselected generation and exclusive access; schema deletion and registry retirement are transactional.
A lost lease invalidates a reader. Retry creates a new attempt. Contract changes rebuild current state.

SQLx owns PostgreSQL effects, pgpq owns binary COPY, and SeaQuery lowers validated descriptors.
`cpg-core` owns DataFusion sessions and provider registration; `lctx-postgres` does not link DataFusion.
Published data is immutable by privilege. Provider reads enforce declared Arrow types. Native IPC,
when later required, is only a derived cache with a PostgreSQL manifest. Serving later reads generated
views over one pinned canonical generation, without import or copied serving tables.

All unaffected ADR-0083 clauses carry forward: attributed typed family authority, generation-qualified
references, content-derived semantic identities, input and producer digests, recorded performance
without a performance acceptance gate, and current-only rebuilds. Delta is removed in phase 1.

## Consequences

The physical layout removes partition lifecycle complexity without weakening referential integrity.
Readers and retirement require explicit coordination. Database infrastructure remains necessary;
pure model operations remain store-free. Current implementation is tracked by the cutover plan,
whose lifecycle/concurrency/type-matrix tests establish qualification.
