---
id: ADR-0135
title: Persisted execution retains captured supplier contracts and complete operation outcomes
status: accepted
date: 2026-10-08
supersedes: []
superseded-by: null
design: [§15, §4, §5, §6]
evidence: Proposed
revisit: A supported producer semantic revision cannot be represented by captured bindings, or qualified native execution can preserve exact selection and bounded state with a simpler shared mechanism.
---

## Context

The [correction-causes review](../design_review/reviews/design_review_persisted-execution-correction-causes_2026-10-07.md)
traced setup, restore/admission and selected-access corrections to incomplete operation contracts.
The [accepted target plan](../plans/persisted-execution-corrections-plan_2026-10-07.md) addresses F01–F06.
ADR-0133 continues to own native persisted compilation and direct sealing. This record adds the
operation contracts needed to complete that architecture; implementation and acceptance remain
with the persisted execution coordinator. The operator accepted table-specific scope layout and
multi-unit import failure semantics on 2026-10-07 and authorized execution on 2026-10-08.

## Options

1. **Continue local repairs.** Small setup windows and local error strings can fix individual
   failures, but leave workflow siblings with competing completion rules, cold admission tied
   to current executables, and whole-match selection state. This does not meet the complete contract.
2. **Shared domain contracts with existing native mechanisms (selected).** Model-owned outcomes
   and producer contracts remove independent classifiers. The existing native external-run owner
   orders compact candidates; bounded whole-unit import requests amortize HTTP crossings. Effects
   remain with workflow owners, avoiding a generic executor or retry subsystem.
3. **Native union ordering or a new DataFusion runtime per scan.** Native union-index deduplication
   in the pinned engine retains a growing seen set. DataFusion sorting is useful when a caller
   already owns its runtime, but adding one inside the store duplicates configuration and lifetime
   ownership. Neither is selected for the common scanner. Native specialization requires actual
   plan evidence and a concrete need.
4. **One import request per whole unit.** This has simpler failure attribution but repeats transport
   for many small statements. Whole-unit batching retains grammar/transaction boundaries and
   accepts effects of later independent units within a failed request only inside disposable staging.

## Decision

- `lctx-model` owns structured operation completion: primary and secondary failures, local
  terminality, remote-effect certainty, private resource disposition and already committed effects.
  Workflow owners perform finalization. Unknown acknowledgement never admits publication; a
  later cleanup failure never implies rollback of a sealed database or persisted backup.
- Provider declarations distinguish supported semantic contracts from executable provenance.
  Exact captured supplier/configuration/source bindings travel in contribution identity; cold
  admission chooses the supported contract independently of reported outcomes. Availability and
  manifest reporting consume one admitted result, including all nine completed-view premises.
- Migrate contribution-spec digest and native content domains to v2, completed-state format to2
  and artifact format to3. Refuse unsupported headers before decoding; rebuild old captures from
  pinned inputs. No compatibility reader, invented supplier or default binding is introduced.
- `lctx-surrealdb` owns prepared demand, bindings, result positions and terminal handling. Generate
  scope keys for each actual table inventory; retain only required graph scope_context scalars
  and the separately owned search columns. DDL, writing, reconciliation and realization identity
  change together. Imported derived values require independent reconciliation.
- Stream compact candidates from one atomic equality branch or exact-contributor scan at a time.
  Reuse bounded external ordering, then exact membership and bounded hydration/residual filtering.
  Preserve nominal versus physical ordering, aliases and cancellation ownership. Retire whole-match
  arrays and growing deduplication sets. Fetch delegation cannot precede required filtering.
- Parser-owned whole units aggregate into checked sequential HTTP import requests under existing
  byte/count bounds. A failed request can have later private effects; submit no subsequent request
  and abandon failed staging through the same completion contract. Runtime deadlines do not increase.

## Consequences

New provider roles and scope fields extend their declarations rather than central exception lists.
Completion can be checked independently while actual cleanup stays locally owned. Necessary full
semantic examination remains; compact ordering may finish candidate collection before the first
ordered row is available. Spilling and captured bindings add explicit lifetime/format obligations.
Compact Arrow buffers and shared drain futures remain intentional library compositions.

This accepted decision is **Proposed implementation strength, 2026-10-08**. PC1–PC6 own migration,
native query-plan qualification, failed-import experiments and fresh-format cold round trips.
The [coordinator §8](../plans/persisted-graph-execution-plan_2026-10-07.md#8-sole-finding-disposition)
retains every open finding until its named evidence exists. No speed measurement, operator-store
replacement, live model use or whole-pivot qualification follows from accepting this record.
