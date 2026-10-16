---
id: ADR-0177
title: Preserve tracing dispatch through polling and destruction
status: accepted
date: 2026-10-16
supersedes: []
superseded-by: null
design: ["§4.3", "§6.1"]
evidence: Interface-checked
revisit: The pinned tracing libraries provide dispatcher-correct destruction or these execution boundaries no longer cross subscriber contexts.
---

## Context

The efficiency [coordinator §6](../plans/catalog-efficiency-plan_2026-10-15.md#6-sole-disposition-and-current-checkpoint)
owns the observed Summary failure in traced run`20261016T112422.334Z-6c31fc` and its correction.
Pinned tracing0.1.44 attaches a subscriber during polling; its `WithDispatch` has no scoped drop.
Pinned tracing-subscriber0.3.23 closes a span's parent through the current dispatcher. Provider
threads and native bridge futures can therefore release a captured span under another registry.
Explicit diagnostics exposed a real lifetime boundary, not a Summary semantic failure.

The [library contract](https://docs.rs/tracing/0.1.44/tracing/instrument/trait.WithSubscriber.html)
promises polling context. Exact locked source establishes the narrower destruction behavior;
no library upgrade or upstream absence claim is implied.

## Options

1. Keep polling-only wrappers or disable the new diagnostic subscriber. The former omits actual
   destruction; the latter conceals the defect and loses useful observations.
2. Implement separate wrappers in each provider, native worker and bridge. This avoids a crate
   but repeats a subtle contract across independent owners and cancellation paths.
3. Share a small tracing utility leaf. Chosen: neither a database dependency in extraction nor
   runtime instrumentation in the pure semantic model is necessary. Existing pinned libraries
   supply dispatch, spans and safe pin projection; the additional code covers their missing
   composition contract, without another runtime, scheduler or allocation per delivered row.

## Decision

`lctx-tracing` owns tracing context through polling and destruction. It retains the span's actual
originating dispatch, enters that context for work, and destroys the inner pinned value and span
there on completion, cancellation or never-polled rejection. Synchronous workers keep owned spans
inside the same scope through return. Provider/bridge/worker consumers use that shared construct;
native phase observations retain their captured context without emitting a terminal merely on drop.
Semantic, recovery, resource and native finality contexts remain independent and compose as before.

## Consequences

Distinct registry, pending cancellation, unpolled rejection and normal completion controls can
exercise this lifetime without a database. There is no new logging backend, global subscriber
policy, mandatory hook, I/O owner or semantic-model dependency. Ordinary worker parallelism remains.
The coordinator records implementation and functional acceptance; accepting this decision does
not close the failed Summary control or establish a performance improvement.
