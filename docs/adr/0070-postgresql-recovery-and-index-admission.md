---
id: ADR-0070
title: Bind ANN admission to physical indexes and recover complete serving state
status: accepted
date: 2026-09-28
supersedes: [ADR-0069]
superseded-by: null
design: [§6.4, §6.5, §11.2, §11.4]
evidence: Tested
revisit: A qualified consumer needs a larger resource envelope or another numerical policy; the registered ANN profile fails its declared recall criteria.
---

## Context

PG16 inspection found that logical restore rebuilds HNSW indexes but also restores old
qualification rows. Database fingerprints alone omit the external artifact closure. The operator
accepted mixed routing and exact-only completion when ANN offers no measured benefit. This
record preserves ADR-0069 publication/query decisions and replaces its all-ANN admission policy.

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
3. Reuse qualification rows across restore or record only a logical policy hash. This is simpler,
   but recreated index topology and catalog plans can differ. Bind physical admission instead;
   recover exact first and retain historical qualification separately.
4. Keep every query on ANN or tune against the same query pack. Small/selective workloads can lose
   both speed and recall; mixed classes and independent confirmation constrain that choice.
5. Add another database driver, workflow framework or semantic query interpreter. No selected
   consumer requires these additional owners.

## Decision

**Implemented and Tested, 2026-09-28.** Projection FORMAT 2 includes typed library, requirement and the
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
unreported rank changes. Qualification comparison revision 2 therefore checks scores by entity
and each ordering against its own numerical values, and separately records cross-reference
ordinal/top-10/fusion agreement. A reported float64 crossing does not redefine PostgreSQL ties
or silently reorder its output. Existing BM25/discriminating terms, RRF K=60 and name promotion stay
Python-owned. Compact Arrow ranks replace dense Python embedding matrices.

Optional HNSW uses full-float cosine, generation/view partitions, m=16, ef_construction=128,
ef_search=100, strict iterative scanning, max_scan_tuples=20000, work_mem=8 MiB and memory
multiplier 2. Candidate chunk depths are 200/800/3200, widening on distinct-entity underfill.
Exact rescoring and fallback are bounded independently. Qualification requires at least 99%
mean entity recall@10 in each preregistered filter stratum, both per vector view and after fusion;
smaller eligible sets use min(10,N). These are measured profile criteria, not per-query guarantees.
The exact route remains default. Missing embeddings allow labelled lexical-only retrieval;
database failure never selects another backend. Responses identify requested profile, actual
route and fallback. Profile policy and qualification history are immutable and generation-qualified.

**PG16/PG17 implemented contract.** Exact format 1 stays byte-identical. Mixed format 2 uses exact
brief search and exact operation search at or below a calibrated 1024/4096 eligible-entity floor
or 10% selectivity. Counts use distinct vector-bearing operation IDs, not chunks. Only confirmed
broad/unfiltered classes use ANN. Calibration and confirmation have disjoint requests; confirmation
freezes the policy. Both paired runs independently require 99% recall per active view and fused
result, rank-stage p95 <=250 ms and at least 20% p95 improvement over exact. Exact-only deployment
can complete PG17; failed ANN activation stays an explicit measured deferral.

Physical admission is separate from policy/history: cluster system identifier, database OID,
generation/view index OID/filenode/definition/validity, server/extension versions and retrieval
engine revision identify the tested realization. Selection, pinning and every ANN transaction
validate admission. Shared generation advisory locks protect reads against exclusive owned index
maintenance; qualification measures the serving role under its generation lock, including routing and physical catalog costs, then releases it before the writer atomically rechecks the realization. Restore/rebuild
cannot inherit ANN admission. Refusal is explicit; recovery selects exact explicitly.

A versioned recovery receipt binds the exported-snapshot database dump and logical fingerprints
to every ready generation and required immutable native/lexical artifact. Stream fingerprints,
count logical relations once, verify artifacts before publishing completion, and verify restored
rows before relocation/selection. Projection reconstruction is distinct from durable history
recovery. Existing artifact registration and reconciliation own relocation.

Diagnostics distinguish publication, availability, selection/pin and admission. Existing SQLx,
provider and PostgreSQL metrics supply bounded observations without payload logging. Serving
deadlines remain bounded; finite maintenance gets up to 300 seconds with 305-second lease drain,
256 MiB maintenance memory and two parallel maintenance workers. Retain all ready generations
and artifacts. Editable Python/cached Cargo are the deployment environment; no wheel gate.

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
qualification. The [operations evidence](../design_review/evidence/2026-09-28_postgresql-operations/README.md)
records local exact deployment/recovery and measured ANN rejection; it establishes no positive
ANN admission, remote-topology qualification or Stage 3 semantic completion.
