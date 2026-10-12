---
id: ADR-0160
title: Represent uniform unrequested work through exact owned domains
status: accepted
date: 2026-10-12
supersedes: []
superseded-by: null
design: ["semantic-model §15.8", "api-and-evidence-product §14.10"]
evidence: Proposed
---

## Context

The approved efficiency RC08/C06 scope removes repeated invocation and outcome work when a whole
declared provider family or analysis owner is uniformly unrequested. Missing rows alone cannot
distinguish that state from incomplete, absent or corrupt output. The compiler, frontier, publication
assessment, serving and named downstream analyses must agree even for an actual empty input domain.
The [efficiency coordinator](../plans/efficiency-alignment-plan_2026-10-11.md#4-package-sequence-and-sole-execution-checkpoint)
owns current implementation and acceptance. Source integration and earlier focused prerequisites
do not establish complete producer/native/consumer acceptance.

## Options

1. Keep ordinary invocation, coverage and outcome records for every unrequested kind and scope.
   This retains existing readers but repeats construction, storage and validation without adding
   evidence when the whole owner has one exact unrequested domain.
2. Treat empty output as NotRequested. This removes the work but conflates incomplete production,
   requested empty results and subjects outside the domain. It is rejected.
3. Bind one compact declaration to the real completed owner and independently reconstruct its exact
   domain. This removes repeated per-kind output while keeping uncertainty and selected ownership
   explicit. It requires coordinated model and consumer migration rather than a compatibility shim.

FP-07/A4 and H4/H6/H13 favor the third option: prepare shared meaning at the owner's actual lifetime,
keep physical ownership below consumers and retain ordinary records wherever status is mixed.
No latency or memory reduction is claimed without measurement.

## Decision

The model owns `UniformNotRequested`, exact domain reconstruction and private validated semantic
owner/domain values. A declaration binds input/context, the current producer declaration, ordered
source roles and exact selected method definitions. An explicit zero-member domain is retained.
Every ordinary output remains declared, with a zero count for a compact whole-owner result.
Contribution completion describes physical production; it does not turn NotRequested into requested
semantic coverage.

Assembly may compact the explicitly unrequested Flow family. Analysis compaction is limited to the
nine model-owned eligible whole producers: Local, Base, Completion, SourceCalls, Enriched, Models,
Summary, analytic embedding and optional analytics. Current profile and configuration must establish
that all their methods are unrequested. Mixed Structural work and requested catalog, selection,
synthesis or retrieval retain ordinary records. A caller cannot register arbitrary compact producers.

Compact analysis declarations are real epoch-shared owner outputs. Their selected boundary follows
the actual producer dependency order. Named downstream Structural, Synthesis, CatalogEvidence and
Selection consumers require validated compact predecessor domains; compact and ordinary output for
the same frame conflict. Model codebooks append new alternatives without changing existing codes.
Each checker consumes only the configuration its owner actually needs.

Frontier enumeration, exact frontier finishing, publication assessment and scoped serving use the
same model operation over independently protected selected owners. Subjects outside the exact domain
remain unknown/no-scope. Missing owners, changed input/context/definition, requested methods and
contradictory ordinary output refuse; missing records never supply the compact premise. Typed data
and semantic owner validation convey no native read, completion or artifact-admission capability.

Adjacent source5 migration preserves authentic ordinary NotRequested records and their original
assurance. It does not manufacture current compact-owner authority from historical descriptors.
Whole-provider NotRequested reuse remains disabled until its complete eligibility and all consumer
boundaries have functional acceptance; this representation change alone grants no cache hit.

## Consequences

Uniform non-requested work has one explicit meaning without per-kind synthetic invocations.
Requested, mixed, partial, unavailable and absent results retain their distinct contracts. The
model/schema cutover and all four availability boundaries must migrate together, including empty,
foreign-context, changed-definition and missing-proof controls. Native production and integrated
acceptance remain open at this decision's creation. Acceptance records the selected design, not
finding closure. Reopen if an owner has a mixed domain, changes its requested policy or cannot expose
an independently reconstructible selected input domain; do not broaden the compact eligibility rule
to work around such a case.

## Amendments

2026-10-12: ADR-0161 replaces the adjacent historical migration approach with practical current-view migration or explicit regeneration. The current compact-owner policy and its consumer gates are unchanged; no historical adapter or preserved-assurance path remains required.
