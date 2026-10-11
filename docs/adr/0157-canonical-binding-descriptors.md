---
id: ADR-0157
title: Retain complete immutable binding descriptors independently of attempt slots
status: accepted
date: 2026-10-11
supersedes: []
superseded-by: null
design: ["storage-and-publication §6.1", "storage-and-publication §6.2"]
evidence: Proposed
---

## Context

The reconciled [N09 target](../plans/efficiency-alignment-native-plan_2026-10-11.md#N09)
requires eventual redundant attempt-binding release without losing exact backup or recovery meaning.
`CompletedBinding::key()` identifies the mutable relation/boundary slot in one attempt. It cannot
identify a retained descriptor: different source attribution or configuration may occupy that slot.
Selected backup currently searches all attempt records sharing a view and takes one matching copy.
That search cannot survive release of every redundant copy, and a view alone omits source meaning.
ADR-0152 supplies protected native completion; ADR-0156 supplies publication-root retention.

## Options

1. Keep every source attempt record. This preserves the current search but couples immutable
   publication lifetime to redundant mutable bookkeeping and prevents the scheduled compaction.
2. Retain one representative keyed only by view. This reduces copies, but incorrectly merges
   different source producers, implementation/model, boundary roles or configuration.
3. Retain one immutable full descriptor beneath the existing attempt slots. Native completion owns
   the exact descriptor and view; publication/backup/reader holders retain that completed closure.
   This adds one fixed codec and exact lookup while preserving the existing portable logical family.

FP-07/A4 and H13/H19/H22 favor sharing immutable meaning with its actual owning lifetime. The
canonical descriptor has a concrete consumer: selected backup after the attempt slot is released.
The full source and validated view remain compared, so a hash hit does not become an acceptance grant.

## Decision

Keep the mutable attempt-slot key. Add a separate versioned immutable descriptor identity over
the complete represented `CompletedBinding`: exact `SourceSnapshot` meaning, boundary role,
configuration and validated completed view including contribution inventory and cardinality.
The executable model owns this meaning. Publication binding inventory version two hashes complete
descriptor identities; changing attribution with unchanged view membership changes the inventory.
Requested/resolved or non-request meaning represented by a source/configuration/boundary cannot be
dropped or inferred from view identity. This change does not invent an absent selector field.

Native completion installs a full `compiler_binding_descriptor` row and establishes
completion→descriptor→view holds before earning completion. Full envelope, exact incarnation and
guarded hold checks remain mandatory at reacquisition and coherent snapshot selection. A serialized
descriptor or matching digest grants no current admission. Canonical rows are installation witnesses,
independent of mutable attempt incidence and excluded from portable authority.

Selected backup reads the exact canonical descriptor under its already checked published completion
and coherent transaction. It compares the full descriptor, then emits the existing data-only portable
binding projection. Destination restore independently validates and rebinds that logical row and earns
fresh physical completion. No source attempt is selected by a global first-match search. Redundant
attempt-slot release remains a separate guarded operation, after every actual consumer can retain the
authoritative descriptor and all other attempt-history obligations have been checked.

## Consequences

Exact backup can survive redundant attempt incidence release without a new portable family or a
scan across equivalent copies. Existing mutable slots retain their update semantics. Source-format
conversion must authenticate old bindings and recompute complete destination inventories rather than
retag old publication hashes. Canonical descriptor construction belongs to the charged frozen
completion operation, avoiding an unbudgeted duplicate at each bind.

On 2026-10-11 the model identity, strict native codecs/schema and selected-backup consumer are source
changes only. Completion earning, protected transaction checks and guarded compaction are under
integration; controls are written but not run because Rust compilation is blocked by ENOSPC.
[Coordinator §4](../plans/efficiency-alignment-plan_2026-10-11.md#4-package-sequence-and-sole-execution-checkpoint)
owns acceptance. No N09 finding is closed. Revisit if a new selector/role dimension is introduced
without entering the complete executable binding meaning, or a live consumer still requires mutable
attempt incidence after authoritative transfer.
