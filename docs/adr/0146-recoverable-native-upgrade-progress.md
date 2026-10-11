---
id: ADR-0146
title: Native upgrades preserve atomic progress under explicit successor authority
status: accepted
date: 2026-10-10
supersedes: []
superseded-by: null
design: ["§6"]
evidence: Proposed
revisit: A supported upgrade cannot preserve exact source identity, bounded atomic progress or original authority through the installed service.
---

## Context

The operator accepted review RC01 and supplemental SA-RC04 on2026-10-10, then authorized
the [SA0–SA9 execution approach](../plans/surrealdb-architecture-and-capability-leverage-plan_2026-10-10.md).
An immutable format3→4 installer left main published and validation partially translated.
Ordinary defaults did not backfill required retirement counters. Coarse phase journals cannot
resume individual preflight, translation, protection, cleanup or verification pages.
The native upgrade and host service remain the respective data-transition and authority owners.
This complements ADR-0145; implementation and integrated acceptance remain open.

## Options

1. **Complete assignments with coarse replay.** Repairs the immediate field defect but repeats
   completed scans and leaves unknown page effects without exact progress.
2. **Native atomic progress under existing immutable host planning (selected).** Keeps effect
   and cursor publication in one transaction and reuses actual authority, session and recovery
   owners. Adds one bounded progress record per migration/database.
3. **General rollout framework.** SurrealKit's frozen planning is useful, but its inspected
   step/completion and lock-loss behavior does not replace native page atomicity or host fencing.
   Adopt only for a named additional planning consumer that removes overlapping machinery.

## Decision

Native upgrades classify supported original, translated and explicitly repairable forms, assign
complete target rows and refuse unknown mixtures. Protected legacy retirement counters are zero
current-protocol registrations, never execution authority or proof of an empty legacy scope.

An immutable execution contract binds original migration, active successor, database/generation,
source/final target and overlay/protocol/preflight/verifier identities. A separately identified
migration-only overlay holds bounded current pass/cursors/revision. It is excluded from runtime
schema identity and portable content because ordinary compilation, serving and admission do not
consume it. Privileged cold recovery includes it and the private host evidence.

Preflight participates in durable progress before target transformation. Compatible transitions
preserve acknowledged prefixes under closed exclusive effects; relevant predicate/source changes
invalidate them. Data-page effects, references and progress commit atomically. Lost acknowledgement
reconciles the same revision and operation. Declaration readback/index readiness are separate
boundaries. Independent verification precedes native journal/marker/overlay sealing in one DML
transaction. Executable-definition completion and durable host scope completion follow separately.

The host may create an explicit checked reconciliation successor for recognized advanced unpublished
state after predecessor drainage, daemon certainty and credential reconciliation. Preserve original
native/credential identities and completed exact publications. Reconciliation creates new receipts,
never fabricated predecessor progress. Ordinary attachment and intent-only replacement remain strict.
Narrow validation-only upgrade qualification may use production page primitives under that owner;
it cannot bypass pending maintenance or select main. Normal advancement retains one client.

Complete the existing exact format3→4 target before the follow-on format4→5 transition needed
for bounded history-page receipts. Each collector advancement commits one bounded outcome page and
revision; callers reconcile/replay by expected revision before advancing. Store only the current
page on its checkpoint, preserving original horizon/era. Do not allocate rich per-item effect history
or repurpose existing cursor fields. The new runtime declaration belongs in the new schema identity;
ordinary initialization cannot stamp a changed meaning onto an installed marker.

## Consequences

Late retries preserve compatible acknowledged work without adding a second task ledger. More
explicit transition/receipt contracts are required at host/native/CLI boundaries. Sealed overlay
retention ends only after durable terminal evidence and named consumers release it; publication
does not drop its schema. Known redundant work is corrected without requiring a timing attribution.

The [storage owner §6.3](../design/sections/storage-and-publication.md#63-schema-evolution)
governs the transition; the [persisted coordinator §8/§9.1](../plans/persisted-graph-execution-plan_2026-10-07.md#8-sole-finding-disposition)
owns findings and actual receipts. This accepted decision remains Proposed implementation evidence,
not migration, runtime or enclosing-plan closure. Preserve compiler/test parallelism, original
references, independent cold admission, operator state and protected recovery assets.
