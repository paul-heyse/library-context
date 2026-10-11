---
id: ADR-0152
title: Protect retained native realizations and freeze complete logical state
status: accepted
date: 2026-10-11
supersedes: []
superseded-by: null
design: ["storage-and-publication §6.1", "storage-and-publication §6.2"]
evidence: Proposed
---

## Context

The reconciled efficiency track accepts by-reference reuse and complete composed state, but a
logical descriptor or row count cannot establish an installed physical realization. Reusing the
first matching logical contributor can select a different incarnation after retirement. Ordinary
seal currently reconstructs completed state while descriptors and ownership remain separate.
[Native N04/N06/N11](../plans/efficiency-alignment-native-plan_2026-10-11.md#N04)
owns the corrective obligations; [coordinator §4](../plans/efficiency-alignment-plan_2026-10-11.md#4-package-sequence-and-sole-execution-checkpoint)
owns actual execution evidence. This decision accepts the target; implementation remains partial.

## Options

1. Copy memberships into every receiving attempt and rescan all backing at each consumer. This
   preserves a simple ownership boundary but repeats work proportional to retained content and
   still requires an exact freeze before reusing assurance.
2. Attach a logical key and trust descriptor/count agreement. This loses incarnation, exact
   membership and absence premises, so it is rejected.
3. Hold the expected physical incarnation, earn a view-local installation completion, and freeze
   complete logical families separately. This keeps physical authority with its owner and lets
   independent compiler products share immutable content. It adds a bounded installation and
   explicit recovery obligation rather than a general incremental-maintenance system.

The design principles FP-07/A4 and architecture heuristics H4/H6/H8/H13/H18 favor preparing a
stable result once at its true lifetime while retaining complete uncertainty and finality boundaries.

## Decision

Native control compares expected record bytes and object incarnation while atomically establishing
an incoming hold. Receiver semantic preflight precedes attachment. Exact-spec attachment holds
existing contributors directly rather than copying their memberships.

A view-local physical realization binds the complete selected logical-to-physical contributor and
dependency closure, object incarnations and member codec. Its completion is published only after
checked EOF, complete member/backing validation, collision checks and missing/extra checks.
Direct member addresses use that physical realization; absence is authoritative only beneath
completed installation. Cold recovery validates equivalent copies explicitly, without first-match
selection. Partial realization remains privately rooted and cannot grant a completed capability.

Protected logical freeze closes and drains every identity-bearing mutation surface. Logical state
v4 composes ordered canonical roots for contributions, memberships, views, backing, bindings and
aliases, preserving explicit empty/selective/absence meaning. Physical completion and logical
identity remain separate. Durable admission premises bind the validator, relevant producer,
configuration, exact logical inputs and protected physical completion/epoch. Cold service recovery
invalidates installation-local trust; detached transport never carries acceptance authority.

Model, compiler, publication, serving, selected backup, detached import, recovery and retirement
migrate together before target-format publication. Existing conservative checks remain until their
replacement witness and all consumers are implemented and tested.

## Consequences

Retained reads and publication can consume owner-earned stable completion without repeated payload
copying. The target requires actual incarnation, mutation, interrupted-installation, corrupt-backing,
restart and cold-roundtrip controls. Accepted design does not close a source finding. The coordinator
retains migration and final acceptance; the original review and older plans remain unchanged.
