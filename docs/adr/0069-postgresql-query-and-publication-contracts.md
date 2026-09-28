---
id: ADR-0069
title: Bind serving context and qualify PostgreSQL query profiles explicitly
status: accepted
date: 2026-09-28
supersedes: []
superseded-by: null
design: [§6.4, §6.5, §11.2, §11.4]
evidence: Proposed
revisit: A qualified consumer needs a larger resource envelope or another numerical policy; the registered ANN profile fails its declared recall criteria.
---

## Context

The accepted PG12–PG15 execution plan completes ADR-0068's PostgreSQL target. Inspection
found that release identity and the coverage summary existed only in the outer file manifest;
a database reader could not reconstruct the complete MCP response from its projection manifest.
Import batching, retries, physical index construction and query approximation also need explicit
contracts without changing canonical Delta evidence (ADR-0067).

## Options

1. Store the outer manifest as independent mutable metadata and inherit query-planner defaults.
   This leaves response meaning outside projection identity and lets index installation change
   exhaustive retrieval.
2. **Bind serving context and select qualified query profiles (chosen).** Derive one typed
   context in the existing bundle producer; keep content identity, import receipts, profile
   qualification and operator selection distinct. Reuse SQLx/pgpq, pgvector and DataFusion.
3. Add another database driver, workflow framework or semantic query interpreter. No selected
   consumer requires these additional owners.

## Decision

**Accepted target, 2026-09-28.** Projection FORMAT 2 includes typed library, requirement and the
complete coverage summary. Bundle FORMAT 12 renders its outer context from that same value.
Rebuild serving projections from compatible canonical snapshots; canonical IDs, stored vectors,
compiler semantics and the standard 1024 embedding policy do not change.

Import uses a generation-scoped session advisory lock and atomically committed row/batch receipts.
The frozen transport recipe is separate from the order-independent content identity. Transient
interruption ends an attempt without terminally failing its generation. Validation reads actual
PostgreSQL rows back through shared codecs/validators after the write barrier. Only complete
validated content/artifacts and a qualified exact profile become ready; selection is explicit.
Runtime DDL capabilities are finite migration-owned functions, never arbitrary SQL grants.

The exact retrieval profile enumerates all eligible vectors, aggregates best chunks per entity
and view, and uses PostgreSQL cosine values with entity-ID tie ordering. Independent float64
scores must agree within absolute 1e-5; this tolerance does not create fuzzy ties or excuse
unreported rank changes. Existing BM25/discriminating terms, RRF K=60 and name promotion stay
Python-owned. Compact Arrow ranks replace dense Python embedding matrices.

Optional HNSW uses full-float cosine, generation/view partitions, m=16, ef_construction=128,
ef_search=100, strict iterative scanning, max_scan_tuples=20000, work_mem=8 MiB and memory
multiplier 2. Candidate chunk depths are 200/800/3200, widening on distinct-entity underfill.
Exact rescoring and fallback are bounded independently. Qualification requires at least 99%
mean entity recall@10 in each preregistered filter stratum, both per vector view and after fusion;
smaller eligible sets use min(10,N). These are measured profile criteria, not per-query guarantees.
The exact route remains default. Missing embeddings allow labelled lexical-only retrieval;
database failure never selects another backend. Responses identify requested profile, actual
route and fallback. Profile policy and qualification are immutable and generation-qualified.

Exact tool cursors retain ADR-0068's offsets; native cursors advance by examined work. The storage
boundary binds the full projection identity and request. Native evaluation stays pure, releases
the GIL and runs in bounded CPU slots without holding database leases.

Federation admits a finite expression policy at both scans and complete optimizer subtrees.
Declared schemas, not catalog inference, own Arrow meaning. Mutable operational report inputs
are captured in one bounded read-only repeatable-read transaction, then joined to explicit
immutable PostgreSQL and Delta inputs. ADBC remains conditional on a measured consumer need.

## Consequences

The context migration closes an actual response-provenance gap without a second descriptor
artifact. Database readiness is established from stored content, while retries and index changes
do not change semantic IDs. The costs are explicit lifecycle functions, profile qualification,
query budgets and an owned provider adapter. The PostgreSQL plan owns execution and the forward
plan §6.1 owns finding dispositions. Acceptance of this decision does not establish runtime
qualification; PG16/PG17 retain production rollout/recovery and assembled acceptance.
