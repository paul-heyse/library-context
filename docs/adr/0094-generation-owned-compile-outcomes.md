---
id: ADR-0094
title: The generation registry owns new compile outcomes
status: accepted
date: 2026-09-30
supersedes: []
superseded-by: null
design: [§15]
evidence: Implemented
revisit: A consumer requires durable pre-generation attempt history or retains outcomes after generation cleanup
---

## Context

Facts compilation now uses the attempt-owned generation lifecycle in [DESIGN §15.11](../design/sections/semantic-model.md#section-15-11). The retained service baseline also contains operational attempt/events from the retired compiler. Writing both would introduce a second outcome authority: a crash after publication could leave the operational record unfinished ([store-lifecycle F09](../design_review/reviews/design_review_p1-store-lifecycle_2026-09-29.md#F09)).

## Options

1. **Keep the retained operational records historical-only.** New compiles use generation state, its failure record and its live attempt lock. No reconciliation protocol or additional schema is needed. The cost is that aborted generations leave no durable attempt history.
2. **Link operational attempts to generation ids.** Derive outcomes from the generation registry and make retention/cleanup explicit. This provides durable history but adds lifecycle coupling and a retention policy without a current consumer.

## Decision

Choose option 1. `cpg-core::facts` writes only the generation lifecycle. Publication never selects. A required producer failure aborts its generation and all its registry records. The `generation list|show` interface reports new work, including live and interrupted attempts. `runs` reads historical retained operational records; new compiles never append to them. Acquisition or preflight refusal before generation creation leaves no generation to inspect.

## Consequences

There is one authority for publication and failure. A missing operational event cannot relabel a published generation. The retained service schemas and their historical records stay available to their existing readers. This decision is in force; the [cutover plan](../plans/semantic-model-cutover-plan_2026-09-29.md#8-findings-disposition) owns focused validation and assembled qualification. Revisit only when an actual consumer needs durable attempt history before generation creation or after cleanup.
