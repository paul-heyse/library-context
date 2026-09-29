---
id: ADR-0083
title: Use PostgreSQL as the single relational store, with DataFusion as in-process compute
status: accepted
date: 2026-09-29
supersedes: [ADR-0067]
superseded-by: null
design: [§15, §1.2, §B2, §B6, §B7, §B12, §B13, §B14, §3.1, §3.2, §3.3, §3.4, §3.4.1, §3.5, §3.7, §3.8, §4.0, §4.1, §4.3, §5, §6, §6.4, §6.5, §8, §9, §10, §10.5]
evidence: Proposed
revisit: Measured publication or provider-read cost at a phase exit cannot be brought within the operator's tolerance by partitioning, COPY tuning, deferred constraint validation, unlogged staging or provider pushdown; a required consumer needs cross-generation history that generation partitions cannot serve; or PostgreSQL constraint enforcement and the DataFusion semantic validators disagree on an injected violation.
---

## Context

**How Delta is used today.** Delta is the canonical store (ADR-0067): 200 append-only tables, published
by a `snapshots` append. PostgreSQL is a rebuildable serving projection imported from an Arrow IPC
bundle. In practice:
- Compilation computes derivations and all validation rules on in-memory Arrow batches. Delta serves
  as the write sink and as the read source for rebuilds and `lctx query`.
- Time travel is unused under current-only cutover (ADR-0078).
- Delta enforces schemas but not keys, references or codebook membership. Those need about 1,100
  generated DataFusion rules.
- The Delta→bundle→IPC→COPY hop created a second, hand-written serving schema authority: 49 of 72
  serving files, and PostgreSQL DDL frozen then hand-edited (review F10).
- The relational basis the product actually uses is joined, constrained and indexed in PostgreSQL.

**The operator's decision (2026-09-29).** Consolidate on one database for the whole relational
structure. Keep DataFusion for intake computation. Treat publication and read performance as tuning
work for later, not a decision gate.

## Options

1. **Keep Delta canonical and PostgreSQL as the serving projection (current).** This keeps serverless
   files and columnar scans. It also keeps a second schema and constraint authority, an import pipeline,
   and a store that cannot enforce the model's relational invariants.
2. **Hybrid: Delta for raw code facts, PostgreSQL for the relational basis.** Every compile then spans
   two stores with no shared transaction (§B12), and every derivation crosses them.
3. **PostgreSQL as the single relational store, with DataFusion as compute (chosen).**
4. **Another database** (DuckDB, a graph database). Either is a new authority and operational surface
   with no advantage over the already-qualified PostgreSQL 18 deployment and provider fork.

## Decision

**The store.** **PostgreSQL 18 is the single relational store** for L0–L3 canonical relations and L4
serving (§15.11–§15.12).
- DDL is generated from the relation declarations (ADR-0082).
- Every relation is list-partitioned by `generation_id`, whose key leads every key.
- Generated constraints enforce keys, generation-qualified references, codebook membership and
  CHECKs.

**The generation lifecycle** is staging → validated → published → selected → retired.
- An attempt writes staging partitions under a writer role, by binary COPY from Arrow (pgpq).
- DataFusion semantic validators run on the attempt's in-memory batches. Publication is one
  transaction, after both they and the database constraints pass.
- Published partitions are read-only by privilege. Retirement drops partitions.
- An aborted attempt publishes nothing, and a retry is a new generation.

**Compute and reads.** DataFusion remains the in-process compute engine at intake. It reads published
relations through the owned `datafusion-table-providers` PostgreSQL fork, with its pushdown and
cancellation controls. `lctx query` runs over a pinned generation.

**Serving.** Serving reads the pinned canonical generation through generated views, grants and
indexes. The bundle, IPC import and serving-file declarations are removed. Arrow IPC remains only for
derived, content-addressed caches with manifests in PostgreSQL. Consumed-vector receipts and exact
vectors are canonical relations.

**Carried forward from ADR-0067, unchanged, with their current owners:**

| Clause | Owner |
|---|---|
| Typed family tables as the only writable authority; one producer per relation; merged records cite the facts they join | §3.1–§3.3, restated per relation by §15.2 |
| Every endpoint is a typed node; unresolved targets stay explicit; no catch-all reasons | §3.4–§3.7 |
| The registry-generated graph catalog, now role-generated and still materialized, and "rules must be able to fail" | §3.8, §8, §15.2 |
| Content-derived IDs, parallel sites distinct, snapshot/generation-qualified keys, recipes recomputed through the `lctx_id` UDF | §3.4.1, refined by §15.3 |
| Analysis results provenance in-row, the compiler run, completion versus truncation, and composition in memory; analysis-result IDs exclude configuration digests | §9, §10, §B6 |
| Content digest over run identities and consumed-vector receipts | §B14, §15.3 |

**Replaced:** Delta physical storage (append-only table properties, `DeltaTable::write` single
commits), the `snapshots`-append publication act, pinned-version Delta readers, and retention. The
generation lifecycle above takes their place.

**Dependency family.** ADR-0002 remains the pinned family until cutover phase 1 removes delta-rs. That
pin change goes through `pin-check` and a superseding family record, which then makes the provider
fork a core family member.

## Consequences

- **What gets easier.**
  - One schema and constraint authority.
  - The database enforces relational invariants.
  - There is no import pipeline or serving copy.
  - Explanations, policy views and derivation views are native SQL views.
  - Serving and canonical relations cannot diverge.
  - Removing delta-rs removes the family's tightest pin.
- **What gets harder.**
  - Publication needs a running PostgreSQL. Tests use testcontainers, and pure transformations stay
    store-free.
  - Row storage is larger than zstd Parquet.
  - Immutability is enforced by privileges and partitions rather than by the file format.
  - Publication and provider-read cost are measured at phase exits and tuned then.
- **Tooling that moves off Delta:**
  - evaluation, gold and probe scripts;
  - `lctx query`, rebuild and diff;
  - the `deltalake` skill selection.

  Each moves in cutover phase 1.
- **Status.** Implementation is Proposed, and Delta remains the implemented store until phase 1
  exits. The [cutover plan](../plans/semantic-model-cutover-plan_2026-09-29.md) owns execution.
