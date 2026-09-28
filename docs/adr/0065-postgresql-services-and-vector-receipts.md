---
id: ADR-0065
title: Use SQLx-owned PostgreSQL services with immutable Delta vector receipts
status: superseded
date: 2026-09-27
supersedes: []
superseded-by: ADR-0068
design: [§6.5, §11.4, §B14]
evidence: Proposed
revisit: A representative PostgreSQL-owned query catalog or typed binary COPY workload favors Cornucopia and Rust-Postgres; immutable vector replay fails; a measured serving requirement triggers PostgreSQL projection work; or operational complexity exceeds the capability or cost benefit.
---

## Context

The operator requested a deployment/integration plan after the
[storage review](../design_review/reviews/design_review_postgresql-storage_2026-09-27.md)
and [stack review](../design_review/reviews/design_review_postgresql-stack_2026-09-27.md).
PostgreSQL is installed but has no product adapter. Rust/Arrow declarations and DataFusion
remain the semantic authority; Delta snapshots and file generations supply reproducible
publication/serving. The current insert-only Delta cache already returns committed winners.

PostgreSQL is proposed for shared cache admission and operational records, with indexed
serving and authored workflows staged behind their consumers. Moving the cache requires
capturing all consumed vectors, including pre-synthesis analytics, in each immutable snapshot.
The proposal covers deployment, recovery and later library capabilities as well as SQL.

## Options

1. **Keep Delta/cache and file serving only.** The simplest implemented system. Retain it until
   the new path meets replay/correctness and capability/cost acceptance; it does not supply
   the planned transactional operational records.
2. **SQLx with its pool/migrations and Rustls (recommended).** SQL-first access composes with
   existing Rust/Arrow contracts; one integrated application lifecycle and offline query
   checking. Raw COPY needs an encoding boundary; database qualification remains required.
3. **Rust-Postgres, Deadpool, Cornucopia, Refinery and one TLS connector.** Credible generated
   SQL API and typed binary COPY alternative. Prefer it when those mechanisms demonstrably
   reduce total integration work; correct the stack review's dependency/image findings first.
4. **Diesel/SeaORM or wholesale PostgreSQL canonical storage.** No present application-domain
   or end-to-end cost case earns either change. Future consumers may reopen the choice.

## Decision

**Accepted for implementation, 2026-09-27; runtime qualification remains open.**

- Use SQLx's PostgreSQL driver, Tokio, pool, transactions, SQL migrations and one Rustls
  configuration. Test through real PostgreSQL 18 with Testcontainers and pinned images.
  Keep static SQL offline-checkable; no ambient database in ordinary builds.
- Keep database effects in `cpg-core`/`lctx` modules. No database dependency enters schema,
  analytics or the immutable native semantic executor. Python uses Psycopg 3 only when a
  direct database consumer is introduced; schema migrations remain Rust-owned.
- The mutable shared cache becomes a PostgreSQL insert-only key/value service. Each attempt
  freezes the committed vectors it consumed and writes a snapshot-qualified `used_embeddings`
  relation and receipt digest before Delta validation/publication. Delta remains the single
  publication act; bundle rebuilds never fetch PostgreSQL or recompute embeddings.
- Operational attempt/events are authoritative only for operational history. Publication
  and generation-discovery rows are reconciled projections of Delta/manifests. Missing or
  stale operational rows never unpublish a Delta snapshot or redefine semantic truth.
- Use separate lifecycles for rebuildable cache/projection data and durable operator records.
  Deploy to the installed PG18 cluster with explicit application/migration identities,
  bounded resources, backup/restore and failure behavior.
- Later SeaQuery, pgvector, Psycopg/SQLAlchemy, ADBC/federation and pgrx uses follow the
  named consumers and qualification in [§6.5](../design/sections/storage-and-publication.md#section-6-5)
  and [§11.4](../design/sections/synthesis-and-serving.md#section-11-4).
  Catalog availability is not a dependency-installation requirement.

ADR-0066 and ADR-0067 carry forward the unrelated serving and canonical-storage clauses of
ADR-0043 and ADR-0047 while replacing cache/replay ownership. ADR-0048's fresh-store schema
policy continues. Explicit uncached sessions are available for deterministic unit fixtures;
production cache failures never fall back silently.

## Consequences

The cache can serve concurrent compiles and operational transactions can be added without
turning compiler facts into mutable application entities. Every snapshot carries its own
embedding replay input. The costs are another service, duplicated consumed vector storage,
migrations, credentials, bounded pooling and tested recovery.

The [PostgreSQL implementation plan](../plans/postgresql-integration-plan_2026-09-27.md)
owns detailed work. The [forward plan §6](../plans/behavioral-model-forward-plan_2026-09-24.md#postgresql-findings)
owns review-finding dispositions and sequencing with Stage 3–5. All product implementation,
runtime integration and comparative measurements are **not_run** at this planning checkpoint.
Accepting an ADR later will not close the review findings without their stated evidence.

## Amendments

2026-09-27: Initial PG0–PG7 scope is implemented and deployed. The full gate, fake/live and
concurrent pilots, exact offline replay, populated restore and rollback controls passed;
[qualification evidence](../design_review/evidence/2026-09-27_postgresql/README.md) records
commands, measured costs and the scoped documentation exception. This does not implement
consumer-triggered future capabilities or complete Stage 3 semantic acceptance.
