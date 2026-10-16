---
id: ADR-0179
title: Schedule independent producers under joined workspace ownership
status: accepted
date: 2026-10-16
supersedes: []
superseded-by: null
design: ["§15.1"]
evidence: Proposed
revisit: A producer cannot transfer its actual input, cancellation or finality owners into an independently scheduled task.
---

## Context

Runtime review R:F02 found synchronous borrowed CPU work inside producer futures polled by
one coordinator task. Tokio block_in_place permits other tasks to progress, but suspends siblings
inside that same task. RC04 authorizes independent placement while preserving deterministic
integration and exact descendant drainage. [Runtime coordinator §6](../plans/catalog-runtime-efficiency-plan_2026-10-16.md#6-sole-disposition-and-current-checkpoint)
owns implementation and acceptance; ADR-0168 continues to govern readiness and private composition.

## Options

1. Keep unspawned FuturesUnordered producers. Ownership is simple, but one synchronous producer
   prevents ready siblings from progressing. Polling that set during coordinator waits is insufficient.
2. Use the existing workspace execution owner to retain actual independently scheduled Tokio
   producer joins, with result delivery to the coordinator. Selected: placement changes without a
   second workflow runtime, semantic scheduler or global lock.
3. Move every kernel to detached spawn_blocking work. Started blocking work cannot be aborted;
   all borrowed inputs would need a separate owning migration and join. No such blanket migration
   is required when independently scheduled producers use the existing borrowed CPU adapter.

## Decision

The workspace execution owner admits, registers and retains each actual producer join before its
launch gate opens. The coordinator owns result receivers and integrates in model-defined order;
arrival does not establish dependency readiness or prefix completion. Owned Arc configuration and
embedder capabilities replace borrowed caller lifetimes without copying semantic state.

A producer carries its actual workspace/input/resource owners and captured tracing dispatcher/span,
retry admission and exact recovery scope. ProducingScope remains bounded to actual provider ports,
not the enclosing stage, whose completion may itself close and wait for those ports. Cancellation
stops new work; panic and JoinError retain structured causes. A dropped receiver or interrupted
drain never drops the last actual join. Task, queued result and join-snapshot reservations follow
their respective lifetimes, separately from semantic input reservations.

Drain seals producer launch, joins stages, then snapshots and drains providers, bridge/native calls,
scoped memo, native owners and shared inputs in their existing order. No mutex is held across await.
Poisoned or refused ownership reports Outstanding and retains descendants for actual settlement.
The multithread runtime permits independent sibling progress. Current-thread runtime correctness
and drainage remain supported; it does not promise CPU overlap.

## Consequences

Ready independent work can progress while another producer uses borrowed synchronous CPU work.
Semantic integration remains centralized. There is no worker/job/thread cap, detached cleanup or
new readiness framework. Source is integrated; compile, revealing functional controls and enclosing
acceptance remain pending at the coordinator. This accepted target is not a measured speed claim.
