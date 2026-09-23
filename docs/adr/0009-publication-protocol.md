---
id: ADR-0009
title: Abort-on-error compile attempts published by a snapshots append; file-based serving generations
status: superseded
date: 2026-09-22
supersedes: []
superseded-by: ADR-0017
design: [§B7, §B12, §6]
evidence: Tested
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
- **Spike results** (2026-09-22, branch `spike/pyrefly-inproc`, `analysis/SPIKE_RESULTS.md`,
  S6 and P1–P4; all passed):
  - **Binary statistics (P1).** Delta log statistics carry no min/max for the Binary
    `snapshot_id`, so Delta skips no files. The Parquet footers do carry them, and row groups
    were pruned (3 → 1). A `snapshot_id` filter costs one footer read per file, not a full scan,
    so the first half of the revisit trigger is met and the second half is not.
  - **BinaryView on read (S6).** Ids read back as `BinaryView`. The two-step cast back to
    `FixedSizeBinary(16)` works; a direct cast is unsupported.
  - **Injected validation failure (P2).** It publishes nothing. A later snapshot's reader sees
    zero rows from it, although those rows are physically in the read version.
  - **Ambiguous append (P3).** An append whose result is lost is classified `published` by
    re-reading `snapshots`. An attempt that never appended is classified `unpublished`.
  - **Rebuild (P4).** A bundle rebuilt from Delta at the recorded versions is byte-identical,
    even after later appends and publications.

## Amendments

- 2026-09-22: pointer only. ADR-0014 adds table retention to the protocol this record decides.
  delta-rs's defaults delete the log that a pinned read of an old snapshot needs, so every table
  is created with expired-log cleanup off and effectively infinite log retention, and open-time
  verify refuses a table without them.

- 2026-09-23: H1 P3 (the library-leverage review) closes the revisit trigger "Binary columns lack
  Delta statistics and snapshot filtering becomes a full scan at pilot scale" without a schema
  change: a pinned read opens only the files its version's commit added (`read_commit_entry` →
  `Add` actions → `TableProviderBuilder::with_adds`), since one attempt writes one commit per
  table. The publication protocol is unchanged.
