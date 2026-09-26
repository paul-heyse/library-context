---
id: ADR-0054
title: Persist source contribution identity across finite summaries and boundaries
status: accepted
date: 2026-09-26
supersedes: []
superseded-by: null
design: [§B5, §9.9]
evidence: Tested
revisit: A summary channel cannot identify its source contribution from the current unaggregated key, or one source contribution must carry distinct independently discharged conditions.
---

## Context

The [forward plan](../plans/behavioral-model-forward-plan_2026-09-24.md#6-findings-disposition)
W5 requires a refusal for one raw source origin to survive a positive proof for another.
[ADR-0050](0050-finite-summary-outcomes.md) put the outcome in one pure producer, but its
published `summary_boundaries` key still grouped a function, parameter, raw fact and condition.
It could keep that group conservatively open, but could not identify which contribution was
proved or refused. Recursive composition in [§9.9](../design/sections/behavioral-analysis.md#section-9-9)
also needs a stable origin for per-path caps and proof identity.

The unaggregated `value_flow_contributions` relation already has the semantic source key and
transfer flags. `flow_values` is a presentation merge and is not a suitable source identity.
No historical snapshot compatibility is required under [ADR-0048](0048-schema-rebuild-policy.md).

## Options

1. **Keep the grouped boundary and never discharge it when origins collide.** This preserves
   unknown, but erases the distinct cause and prevents a serving client from following a
   specific source path.
2. **Repeat the full contribution key in every seed, summary, boundary and serving row.** This
   retains exact identity but couples each consumer to all source-key fields and makes recursive
   proof and cursor contracts unwieldy.
3. **Derive one stable origin id from the unaggregated contribution key.** Chosen. The typed
   relation retains the contributing fields; the id gives downstream joins one checked handle.

## Decision

`cpg-schema::id::recipe::value_flow_origin` hashes the contribution's raw fact id, use id,
source key, identity/call flags, local call flag and upstream transfer flags, in the declared
`lctx-id/v1` encoding. `cpg-core::flow_model` writes `origin_id` on each unaggregated row;
the shared DataFusion semantic rule recomputes it with `lctx_id`. The direct, modeled,
assignment and local-call seed relations carry that exact id. The modeled exact-transfer
relation retains its originating contribution id; an assignment-return path also retains its
predecessor origin, source key and successor use so both sides join to matching contributions.

`summary_flows.source_origin_id` participates in canonical `summary_id`; a proof of another
origin cannot reuse that identity even when all other steps coincide. The pure finite producer
keys refusals and `summary_boundaries` by origin and condition. A positive suppresses only its
matching origin and condition, and an explicit refusal on that origin remains open. A narrower
positive condition does not discharge a broader source condition. The shared validator
reconstructs both relations. The native generation exposes the origin id on each open boundary
and continues to expose the raw fact as its source witness.
The obsolete SQL-only `summary_boundaries` complement is removed; the pure producer is the
sole finite decision owner, and tests inspect that producer and the published rows. The unused
native `value_paths` test method is removed so `inspect_value_paths` is the single path-local
response contract; no historical binary reader or response adapter is retained.

This is a current-schema migration: rebuild the store and generation from pinned inputs. No
legacy binary reader or dual-write path is added.

> Decision: ADR-0054

## Consequences

The pure sibling-origin control, reviewed schema contracts, real modeled-origin join fixture,
Delta/native refusal trace and native two-origin admission control passed in focused checks on
2026-09-26. The full same-source two-origin Delta/native trace and work/node-limit publication
controls remain under the plan's W5 row. The contribution hash and reference
checks add a small validation cost. Source-origin identity does not by itself prove general
predecessor completion, an exceptional exit, recursive transfer or a negative claim; those
remain separate Stage 3 obligations. Revisit if a new channel cannot express its origin through
this key, or if a single contribution needs multiple independently closed condition partitions.
