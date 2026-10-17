---
id: ADR-0185
title: Prepare native content by demand with exact page retries and owned computation
status: accepted
date: 2026-10-17
supersedes: []
superseded-by: null
design: ["§6.1", "§6.2", "§15.1"]
evidence: Interface-checked
revisit: A consumer needs a different complete identity or noncanonical original ranges, or a pinned grouping operation no longer preserves authoritative typed equality.
---

## Context

The [upper execution coordinator](../plans/catalog-upper-execution-plan_2026-10-17.md#6-sole-disposition-and-current-checkpoint)
owns F01–F04/F14 and investigations I04/I05. The operator approved RC02/RC05/RC06/RC08 and focused functional acceptance on 2026-10-17.
Ordinary selectors eagerly normalize backing rows they never consume; whole-contributor Resource retry replays completed membership work; originals repeat scalar reads and byte scans; reference agreement plans every owner pair. Audit sorting also performs substantial file work on async executors.
ADR-0181's actual checked pin/context and ADR-0184's exact immutable acquisition remain necessary foundations. Independent receiving is not producer proof.

## Options

1. Retain eager full preparation, whole-contributor replay and pairwise agreement. This avoids new internal seams but repeats physical work and retains unnecessary backing runs for ordinary selectors.
2. Globally cache proofs, weaken checks, raise deadlines or add a pool/caps. This introduces invalidation or scheduling machinery without removing the diagnosed work; it can also lose independent ownership and native uncertainty.
3. Prepare sufficient products inside their existing protected operation, use exact set/page work and the existing acknowledged blocking owner. Selected. It requires explicit demand and cleanup seams but keeps one normalization recipe, current trust boundaries and library execution machinery.

## Decision

`lctx-native` retains compact audited canonical memberships and relation partitions for ordinary selection. Full logical backing runs are demanded by actual identity/checksum/export consumers. Fresh full demand feeds normalized backing from necessary validated acquisition; later demand reuses completed memberships and reacquires only backing not retained. Demand does not mutate a running flight. Complete promised contributor assurance, including unselected backing, EOF, aliases and final context checks remains mandatory. Compact or full success publishes only after its own terminal checks; failed preparation grants no provisional capability. Binding-selected compound identities and terminal recovery explicitly request full preparation. Checksum and detached import/export retain their independent reconstruction/ownership routes and unchanged six-family flat v4 meaning.

Backing acquisition distinguishes typed reducible page workspace from other Resource, transport and finality failures. A failed exact slice may split after all opened streams drain; accepted pages and membership preparation are not replayed. Singleton, poisoned sorter, append/finish failure and Unknown remain terminal.

Compiler original completion consumes earned family counts and bounded exact payload sets. One sealed operation orders distinct layout IDs, validates every source/layout/hold and drains pending work for all layouts before completing unique blobs. This avoids both later own-replay invalidation and a retained revision cache. Each blob still earns independent readback and its existing incarnation/revision CAS with conclusive same-incarnation recapture only. A receiving audit independently streams every physical byte once into full and canonical chunk hashes; its protected incarnation owns compact chunk preparation. Equal replay/peer holds do not invalidate frozen bytes merely by changing an incidental revision. Layout/source/hold/range obligations remain distinct. Arbitrary retrieval ranges retain their existing bounded union reader.

`lctx-compiler-store` groups applicable original reference targets by relation, materializes each original once and performs full typed row agreement through pinned DataFusion. Full-row distinct followed by ID-variant grouping must match authoritative per-field null-safe equality, including lists, nested values and signed zero. Unsupported equality uses exact sorted ID-group comparison, never a digest shortcut. Every original owner remains independently observed and pending reference premises publish atomically only after the complete phase.

Substantial audit sort/merge/fold/spill uses the existing acknowledged blocking command owner, with bounded charged row batches or complete owned units. Accepted work registers its actual terminal before awaits; inputs, queued results, scratch and guards remain owned through late failures and lost waiters. Audit close drains async and blocking workers before pin release. No new scheduler, pool, global cache, concurrency cap or persisted compatibility format is introduced. Wider contributor overlap remains US02 scope.

## Consequences

The design removes unnecessary full backing retention, scalar access, replay and pair planning while preserving the independent boundaries that establish correctness. Compact membership and canonical chunk access layouts add only operation-owned, charged preparation with real consumers. Expected work reduction follows source structure; latency and memory improvements are not measured here.

Decision acceptance does not close implementation, focused controls, historical Unknowns or full Catalog qualification. The coordinator records actual outcomes and remaining scope. No schema migration, dependency move, operator adoption or disruptive service action follows from this decision.
