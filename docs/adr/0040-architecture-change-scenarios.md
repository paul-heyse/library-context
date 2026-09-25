---
id: ADR-0040
title: Review architecture through change scenarios and repository-owned foundations
status: accepted
date: 2026-09-25
supersedes: [ADR-0023]
superseded-by: null
design: [§1.2, §2]
evidence: Implemented
revisit: Reviews miss concrete change-propagation or isolated-testing defects, prescribe abstractions without a credible consumer, or duplicate current finding status across artifacts.
---

## Context

The operator approved an architecture-focused revision on 2026-09-25. Core 2.0 concentrates its
acceptance gates on semantic correctness and evidence. Module ownership is a SHOULD without a
gate; the finding template requires a wrong, ambiguous or unrecoverable outcome. Valid local
slices can therefore accumulate architectural problems without a system-level decision.

ADR-0023 requires a verbatim shared core but names no authoritative upstream location. The
approved change needs explicit ownership. The repository remains a personal project in design;
new hooks, tracking services and automatic architecture scores have no consumer.

## Options

1. **Keep 2.0 and add architectural questions to the binding.** Simpler in edit volume, but the
   core finding and acceptance rules would continue to underweight architectural consequences.
2. **Change an assumed shared source and synchronize other repositories.** Its canonical location
   and update contract are unspecified; this task authorizes changes in this repository.
3. **Own core 3.0 here, retaining the layered structure and historical IDs** (chosen). Makes
   responsibilities, contracts, composition and realistic changes decisive while preserving fidelity.

## Decision

- This repository owns core 3.0, template 3.0 and the local process skills. `standard.toml`
  declares versions and paths. Core content remains domain-independent and can be adopted
  elsewhere through an explicit revision; no shared-copy synchronization is implied.
- Six foundations FP-01–FP-06 organize assessment. Three independent architectural judgments
  A1–A3 assess change locality, structural meaning and composition. DP-01–DP-24, G1–G8 and
  CI-01–CI-13/CI-G1–CI-G3 retain identifiers. Profile 1.1 adapts its review additions.
- A concrete expected change exposing duplicated decisions, unrelated internals or inseparable
  tests can require revision even when behavior is correct. Fidelity gates remain independent.
- Library fit follows an owned capability and credible current or planned use. Assess total
  integration burden for adopted and bespoke mechanisms, and allow intentional shared library
  contracts. No hypothetical backend framework is required to demonstrate replaceability.
- The binding owns cadence: design/target before substantial stages or boundary/§B changes,
  change/conformance for bounded implementation, design/target for assembled stage/increment
  ends, and earlier reopening on material structural triggers. This replaces the review-cadence
  clauses carried by ADR-0021 and ADR-0026; their product and build decisions remain in force.
  Effort follows risk, replacing the odd/even increment depth schedule.
- Reviews preserve dated findings. One active plan table owns current execution disposition
  for scheduled findings; unscheduled deferrals stay in their source review until transferred.
  Group shared causes, retain source IDs and closure evidence, and link from STATUS. An accepted
  decision is distinct from implemented and verified closure.
- DESIGN distinguishes accepted architecture/targets from implemented state and links executable
  contracts. ADRs retain rationale and revisit triggers. No new register, hook or check runner.
- Process validation uses the existing metadata checks and two independently conducted calibration
  reviews: one bounded slice and the Stage 3 architecture. Their implementation findings enter the
  active plan, without expanding this process revision into production refactoring.

## Consequences

Architecture can receive a Revise judgment independently of a correctly implemented slice.
Reviewers need scenario evidence and component ownership, not a fixed amount of prose or probes.
Historic reviews retain their standards and conclusions; the new version does not certify them.
The record implements review policy; improved maintainability remains Proposed until exercised
by actual changes. Focused checks remain the development loop; integrated `just test-all` and
`just pilot` run when the functional scope is ready, as directed by the operator.
