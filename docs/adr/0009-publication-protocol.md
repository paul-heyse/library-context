---
id: ADR-0009
title: Abort-on-error compile attempts published by a snapshots append; file-based serving generations
status: proposed
date: 2026-09-22
supersedes: []
superseded-by: null
design: [§B7, §B12, §6]
evidence: Interface-checked
revisit: The local probe shows Binary columns lack Delta statistics and snapshot filtering becomes a full scan at pilot scale, or an injected failure test finds a reader seeing unpublished rows.
---

## Context

Baseline review finding **F3**: the publication protocol was not decidable. The open points
were:
- where the manifest lives;
- whether it is atomic;
- how rows from failed snapshots stay out of shared tables;
- whether a crash means "no snapshot" or "a snapshot marked failed".

Delta has no multi-table commit. deltalake skill evidence at our exact pin (2026-09-22):
- a write can return `Err` after its commit became visible (`delta.commit.1`);
- transaction markers don't suppress a sequential repeat append (`delta.replay.1`);
- `SaveMode::Ignore` appends (`delta.write.3`);
- a provider built on a loaded handle ignores the requested version (`delta.open.2`,
  `delta.read.4`);
- raw Parquet scans diverge from Delta (`delta.read.2`);
- vacuum defaults to `dry_run=false` (`delta.retention.1`).

## Options

1. **An idempotent-retry protocol:** a monotonic `snapshot_seq`, an application transaction per
   table, `write_id` columns and history reconciliation. Rejected as disproportionate for one
   local operator (DM-58).
2. **A manifest file beside the tables.** Rejected: that is a second publication mechanism with
   its own atomicity story.
3. **The simpler alternative, chosen:**
   - any write error aborts the attempt;
   - a retry is a new attempt with a new `snapshot_id`;
   - publication is one append to a `snapshots` Delta table, after validation;
   - readers pin versions and filter by `snapshot_id`.

## Decision

- **Canonical store.** DESIGN §6.1–§6.3 as written:
  - family tables are append-only, with `snapshot_id` as a plain column;
  - the `snapshots` append is the only publication act;
  - an adapter failure on a module becomes `coverage.status = failed` and does not block
    publication;
  - no `SaveMode::Ignore`, no vacuum or optimize, and no `gc` in stage 1.
- **An error on the `snapshots` append itself** is ambiguous. Re-read `snapshots` and classify
  the attempt before any retry.
- **The cache version** is recorded with the snapshot's row set.
- **Readers.** Load each table at its recorded version and assert `version()`. Filter by
  `snapshot_id`, project by name, and never scan Parquet directly.
- **Serving generations** (§6.4).
  - Built **only after** a successful `snapshots` append, by reading the published snapshot at
    its recorded versions, including `embedding_cache`. Rebuilding from Delta is byte-identical.
  - An immutable Arrow IPC bundle, with a manifest that carries per-file schema digests.
  - The `active` symlink is switched by atomic rename, and there is one generation per server
    process.
  - There are no cross-store transactions.

## Consequences

- **Wasted rows.** Unpublished attempts leave invisible rows. At pilot scale that costs disk
  only. A later `gc` is an ADR if it matters.
- **Pinned reads.** Versions pinned in `snapshots` remain readable as long as we don't vacuum.
  That's acceptable because we don't vacuum.
- **Spike before acceptance:** a local Delta probe that checks
  - Binary column statistics;
  - BinaryView on read;
  - that an injected validation failure publishes nothing, and a later snapshot's reader sees
    zero rows from it;
  - that an injected error after the `snapshots` commit is classified as published;
  - that a generation rebuilt from Delta is byte-identical.
