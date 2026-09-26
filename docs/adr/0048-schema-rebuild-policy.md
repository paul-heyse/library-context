---
id: ADR-0048
title: Rebuild the current store from pinned inputs after schema changes
status: accepted
date: 2026-09-25
supersedes: []
superseded-by: null
design: [§6.3]
evidence: Interface-checked
revisit: A real consumer needs historical snapshots, or a pinned input needed to rebuild the current library is unavailable.
---

## Context

The new `function_implementations` table is a reviewed schema migration. The current Delta
reader compares every table with its declared `cpg-schema` contract and refuses drift; there is
no in-place evolution. [§6.3](../design/sections/storage-and-publication.md#section-6-3) and
[plan W15](../plans/behavioral-model-forward-plan_2026-09-24.md#6-findings-disposition)
left the operational policy open. A current binary cannot silently reinterpret an old store.

## Options

1. **Rebuild the current library in a fresh store from pinned inputs.** Existing acquisition and
   compile commands produce a new snapshot under the new producer/compiler identity. The prior
   store may be held temporarily for rollback, but historical reads and old binaries are not a
   product contract. No second schema reader or migration writer is needed.
2. **Evolve Delta tables in place.** This would keep one store but add version-dependent reads,
   constraint migration and failure recovery across published snapshots. It also weakens the
   present exact-schema verification boundary.
3. **Version replacement tables in one store.** This preserves historical snapshot reads from
   one store at the cost of versioned table names, publication manifests and reader dispatch.
   No current consumer requires that complexity.

## Decision

Use option 1 for schema changes. A reviewed contract snapshot and explicit migration commit
announce the change. Build a fresh store from the committed library projects, locks, source pins
and compiler configuration; the new snapshot is the current analysis. Do not copy analysis tables
or snapshots from the prior store. The global embedding cache may be reused only through its own
verified cache contract; `embedding_specs` is produced again by the new compile. The old store
may be moved aside during the rebuild for rollback and removed after acceptance. Keeping its binary
is optional, not required. The new snapshot has new execution identity; it is not a rewrite of
the old snapshot. `delta::verify` continues to reject schema drift.

## Consequences

The policy keeps the write path and validators simple. Historical snapshots have no supported
readability guarantee after a schema change; this is appropriate while the product analyzes
current pinned Python libraries rather than serving historical analyses. The fresh-store rebuild
for this migration has not yet been exercised; it belongs to the integrated end-of-scope gate. If
a real consumer needs historical reads, revisit options 2 and 3 with that consumer's contract and
an ADR. This decision does not by itself close any production finding.
