---
id: ADR-0184
title: Prepare immutable windows exactly and reuse compatible ingress writers
status: accepted
date: 2026-10-17
supersedes: []
superseded-by: null
design: ["§6.1", "§15.1"]
evidence: Interface-checked
revisit: A native exact compound key or stronger execution primitive removes bounded fallback scans without changing equality, admission, rollback or lifecycle protection.
---

## Context

The collection plan's NI00–NI02 residual leads concern repeated immutable-window preparation,
linear coalescing and writer setup, and a possible shared-attempt contention point. Native control
owns authorization and atomic effects; compiler-store owns the ordered ingress lifetime. ADR-0178's
complete attempt cuts, original-request replay and independent receiving remain in force. The
[collection coordinator §6](../plans/catalog-collection-efficiency-plan_2026-10-17.md#6-sole-disposition-and-current-checkpoint)
owns implementation and actual acceptance, including protected historical Unknowns.

The pinned RocksDB realization registers `FOR UPDATE` reads for optimistic commit validation;
it does not hold an exclusive waiting row lock. Shared registered reads with distinct stripe writes
do not themselves conflict. SurrealDB Number equality crosses representations, while its hash uses representation-specific
backing. Compound record IDs inherit numeric comparisons. The generic equality/hash/ordering
combination supplies no established lawful key or representative-collapse contract for this route;
retain direct original comparisons rather than assuming one.

## Options

1. Retain every preparation and linear scan. This keeps local contracts simple but repeats guard
   classification, deferred searches, scalar coalescing and issuance setup across a real ingress.
2. Remove the attempt registered read, bulk-read every guarded record, use generic hashed Values
   or collapse deferred representatives. These mechanisms either lose independent stale-owner
   protection, replace necessary fences with ordinary reads, introduce a quadratic join, or change
   exact matching. None is justified by the apparent database primitive's name.
3. Share only stable preparation within its actual owner, index exact scalar identities, retain
   ordered equality fallback and use the native short-circuit first-match primitive. This adds a
   small private index and writer-taking seams while preserving public batch contracts and native
   lifecycle authority. Its physical savings come from avoiding known repeated work, with no cache
   protocol, scheduler or expanded transaction lifetime.

## Decision

Use option3. Retain the ordinary attempt `FOR UPDATE` read, all64 attempt stripes at complete
cuts, installation admission, exact original replay, absent-request fencing and terminal-reference
recovery. Distinct-slot registered reads are compatible; same-slot conflicts remain intentional.

Separate physical guard acquisition from packet-local classification. Supplied expectations do
not reacquire guards. Compute each expectation's deferred-incidence flag once before request
identity is formed, and consume that flag directly. Native installation uses `array::find` over
the original ordered deferred array, preserving the first matching full Value and NONE on absence.
Do not deduplicate membership candidates before classification. Keep necessary exact scalar
locked reads, original-blob completed mutability and all-window rollback.

A private index uses tagged `(table, String | i64 | UUID)` identities and candidate positions,
followed by original equality. Compound/unkeyed values retain ordered exact scans. Equal immutable
identity with unequal payload refuses; auxiliary arrays preserve unequal candidates for existing
native checks. Construction admission covers keys, positions and outputs before allocation and
lasts through their final consumer. No generic Value/RecordId hash, ordering, stringification or
digest grants equality. No permanent cache is introduced.
Generic immutable/hold lowering reserves index scratch from its active recovery budget before
growth. Covered window construction uses an explicit admitted path. Stable in-place compaction
reuses the input Vec backing, so a new output allocation does not outlive the scratch grant.
Borrowed table/string lookup clones keys only when an identity is first inserted.

One writer spans a compatible ordered ingress driver's async frame across physical rows,
original sources/chunks, aliases and memberships. Its exact client, attempt, contribution,
producing scope and budget must match. Reuse only stable issuance and fixed control sizing;
every window gets fresh expectations, request identity, authorization, outcome and recovery.
Standalone public batch calls retain one writer per bounded call. Post-freeze finalization prepares
an independent context. Validation-before-mutation, ordinal order, acceptance versus commit,
cancellation, joins, exact EOF and finality remain unchanged.

## Consequences

Scalar work uses exact keyed candidate lookup, native deferred matching can stop at the first
match, and repeated batches reuse their real owner's stable setup. Compound fallback remains
bounded and deliberately retains the library's equality semantics; no universal linear-time
claim is made. Source-first reasoning predicts less work, without measured latency or RSS claims.

The control definition change alters native schema identity even though no published columns or
codebooks change. Generated assets derive from the sole native schema authority; pre-cutover
controls and authenticated regeneration replace project state without old-format readers.
Decision acceptance does not establish native race acceptance, historical cleanup or full Catalog
qualification. The coordinator records focused evidence and later PC90/PC99 obligations separately.
