---
id: ADR-0017
title: Abort-on-error attempts published by a snapshots append; each pinned read opens its own snapshot's commit
status: accepted
date: 2026-09-23
supersedes: [ADR-0009]
superseded-by: null
design: [§B7, §B12, §4.0, §6]
evidence: Tested
revisit: One attempt writes a snapshot-qualified table in more than one commit (a streaming or retried write); a reader needs rows of a table across snapshots (a relational diff); or an injected failure test finds a reader seeing unpublished rows.
---

## Context

ADR-0009 decided the publication protocol: abort-on-error attempts, one `snapshots` append,
and readers that load each table at its recorded version and filter by `snapshot_id`. Its
revisit trigger was "Binary columns lack Delta statistics and snapshot filtering becomes a full
scan". Delta records no log statistics for the Binary `snapshot_id`, so every read opened every
snapshot's files and paid one footer read per file.

The library-leverage review (H1 P3) closed that by reading each snapshot's own commit. The H1
review then showed that the change altered ADR-0009's reader contract, not just its
implementation:
- **F2:** the version became a file selector, which the old contract called "an audit coordinate
  and a lower bound". It rests on an invariant nothing checked: one commit per table per attempt.
  `--unpublished` returned an empty snapshot for any attempt that was not a table's latest
  writer.
- **F3:** §6.2's step became unscoped. The global `embedding_cache` accumulates across attempts,
  so reading only its last commit would drop vectors cached earlier.
- **F4:** a closed trigger was still listed.

## Options

1. **Keep the snapshot filter over every file** (ADR-0009 as written). Correct, but every read
   costs one footer per file of every snapshot, and the cost grows with the store.
2. **Partition by `snapshot_id`.** As Binary it silently returned wrong results (0 of 1,000
   rows); as a hex column it is a schema migration of every contract.
3. **Read each snapshot's own commit, and check that it is (chosen).**

## Decision

Everything ADR-0009 decided stands:
- abort-on-error attempts, and a retry is a new attempt with a new `snapshot_id`;
- the `snapshots` append is the only publication act, and an ambiguous append is classified by
  re-reading `snapshots`;
- no `SaveMode::Ignore`, vacuum or optimize;
- serving generations are built only after publication (§6.4);
- table retention: ADR-0014's pointer, log cleanup off, verified at open.

The reader contract becomes:
- **A snapshot-qualified table is read at its recorded version, opening only the files that
  version's commit added** (`snapshot::commit_provider`: `read_commit_entry` → `Add` actions →
  `TableProviderBuilder::with_adds`). A selected file that is gone fails the scan.
- **The invariant this rests on:** one attempt writes each snapshot-qualified table in exactly
  one commit (`delta::append`), and that commit records `lctx.snapshot_id` in its `commitInfo`.
- **The check:** before reading, `commit_adds` confirms that the commit's `lctx.snapshot_id` names
  the snapshot being read, and refuses otherwise (`ForeignCommit`), never reading it as empty.
  `snapshots` remains the authority for *which* version; the commit metadata only confirms it.
- **Then filter `snapshot_id`** as the row predicate, and project by name.
- **An unpublished attempt** (inspection only, `lctx query --unpublished`) is read at the commits
  carrying its own `lctx.snapshot_id` (`snapshot::attempt_versions`, which walks each table's
  kept JSON commits). Tables the attempt did not write are left out, so a query naming them
  fails instead of reading empty.
- **Global tables that accumulate across attempts** (`embedding_cache`, §3.2) are **not**
  snapshot-qualified. They are read at their recorded version over **all** active files, with no
  commit selection and no snapshot filter (§6.2, §6.4).

## Consequences

- **Reads cost what the snapshot holds.** A 200-snapshot table answered in 4.7 ms instead of
  15.7 ms (the leverage review's probe), and the pilot's reads are unchanged in content: every
  table fingerprint was identical through H1b.
- **Tested:**
  - `a_pinned_read_opens_only_its_commits_files` (an unfiltered read of one commit, an empty
    commit, a missing file);
  - `reads_pin_the_version_and_filter_the_snapshot` (another snapshot's commit is refused);
  - `a_rejected_attempt_is_inspected_at_its_own_commits` (an attempt rejected, then superseded
    by a published one, reads its own rows; a foreign commit is refused; an unknown attempt
    errors).
- **A future writer that splits a table's write** across commits (the deferred streaming
  derive, or a retried raw write) must record all of them, or this contract must change. That is
  the revisit trigger.
- **ADR-0009's trigger is closed;** this record's trigger replaces it.

## Amendments

- 2026-09-23: **global tables** (remaining-scope plan, Phase 0; ADR-0019). A table declares a
  **read mode**, `snapshot` or `global`, in `cpg-schema`, and `register`, `session`,
  `attempt_versions` and the serving bundle honour it:
  - **A `global` table** (`embedding_cache`, §3.2) has no `snapshot_id` column. It is read at its
    recorded version over **all** active files, with no commit selection, no `lctx.snapshot_id`
    check and no snapshot filter. Its recorded version may be another attempt's commit.
  - **It may be committed more than once per attempt** (before Stage E and after Stage F, from
    increment 3). The one-commit invariant above is for snapshot-qualified tables, and it is
    unchanged.
  - **`embedding_cache` is written by an insert-only MERGE** on `(spec_hash, input_hash)`
    (`DeltaTable::merge(..).when_not_matched_insert(..)`), with a retry on a commit conflict,
    beside `DeltaTable::write` as a second write path (§6.1, §4.3). delta-rs never marks a commit
    a blind append, so two concurrent appends of one key would both commit. vLLM vectors differ
    between requests (E2), so a duplicate key would make the bundle's vector choice, and its
    byte-identical rebuild, unstable. The merge reads the target, so a racing merge conflicts and
    re-runs. The source rows pass the local type, length, finiteness and norm checks first. The
    behaviour is probed before use (slice 1.6). If two concurrent merges can leave two rows, the
    fallback is an exclusive lock file in the store around an anti-join and append.
  - **The snapshot's row** for a global table records the version read after the last write and
    `row_count` = the rows **used** by the snapshot. `content_digest` takes a digest of the sorted
    `(spec_hash, input_hash)` keys used, never the shared version, which another library's
    compile can move (§3.4.1).
  - **Uniqueness** of the key over the whole table is a validation rule. The table is
    append-only and validators never repair, so a violation's declared recovery is a migration to
    `embedding_cache_v2` keeping the earliest-version row per key.
