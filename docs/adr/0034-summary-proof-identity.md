---
id: ADR-0034
title: Give finite summaries canonical proof identities and typed ordered witnesses
status: superseded
date: 2026-09-25
supersedes: []
superseded-by: [ADR-0045]
design: [§B5, §B6, §3.4.1, §9.9]
evidence: Tested
revisit: A finite summary path needs non-linear premises that an ordered witness sequence cannot represent without duplicating meaning, or a capped SCC fixture makes path identity collide.
---

## Context

The first direct `summary_flows` row was keyed by callable, input formal, one raw return fact
and condition. That is enough only while every positive path is a direct identity return.
Two different modeled calls can feed the same return fact, and an SCC can compose several
ordered calls. Keying by that raw fact would either merge their evidence or force parallel
paths into a display string. The Stage 3 target requires typed proof links and same-snapshot
explanation (DESIGN §B5, §B6, §9.9; DP-04, DP-07, DP-21, CI-03, CI-11).

## Options

1. **Keep the raw-return-fact key and derive witnesses at query time.** This is simplest for
   direct returns but merges distinct modeled paths and makes explanation depend on a second
   serve-time source analysis. Rejected.
2. **One nullable model/call tuple on each summary row.** Adequate for exactly one modeled
   call, but a two-call wrapper or recursive summary needs an unbounded number of tuples.
   Extending that shape would migrate the core table for every new path depth. Rejected.
3. **Canonical path id plus ordered typed proof steps.** Chosen. A step relation starts with
   the existing raw identity fact and gains a new codebook variant only when its producer and
   shared validator exist. The summary id hashes the callable, input/output paths, transfer
   kind, condition, exit and
   ordered `(step kind, evidence id, step condition)` sequence.

## Decision

`summary_flows` has a canonical `summary_id` key. `summary_flow_steps` retains ordered typed
evidence with `(snapshot_id, summary_id, ordinal)` identity and an explicit condition at each
step. The first producer admits only one `raw_identity` step citing its `flow_values` fact;
the return-site and region citations stay on the summary row. New step kinds are append-only
and need a checked source relation, a deterministic hash encoding, a publisher and shared
reconstruction before they can create positive summaries. `summary_boundaries` continues to
name raw paths without a completed proof.

The id is a content-derived identity for a path in the analyzed snapshot, not a petgraph index
or a display label. A changed step sequence gets a changed id. The shared validator
reconstructs both summary rows and steps and rejects missing or forged evidence. Publication
remains one Delta snapshot. This structure does not by itself establish a modeled call result
or a negative transfer verdict.

## Consequences

Parallel source paths can now stay distinct and can be explained through typed source facts.
SCC composition may reuse the same path recipe rather than a special one-call column family.
The cost is one small proof-step relation and a schema migration before model-call summaries
exist. An ordered sequence is enough for a single finite path; if a future rule needs a true
multi-parent proof DAG, supersede this decision instead of smuggling parents into opaque text.
