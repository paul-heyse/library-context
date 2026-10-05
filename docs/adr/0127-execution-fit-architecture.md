---
id: ADR-0127
title: Assess physical execution fit alongside semantic architecture
status: accepted
date: 2026-10-05
supersedes: []
superseded-by: null
design: [§2]
evidence: Implemented
revisit: Execution-fit judgments fail to expose structural amplification, weaken semantic assurance, or impose process without a concrete consumer.
---

## Context

The [architectural-criteria assessment](../design_review/reviews/design_review_architectural-criteria_2026-10-05.md)
found that a semantically structured system could pass A1–A3 while its operations scale with
incidental declarations, repeated full-input work or long-lived transactions (F01–F04).
Existing efficiency guidance did not supply a decisive architectural acceptance criterion.
DESIGN §2 owns this policy; the proposed graph-store replacement remains a separate decision.

## Options

1. **Keep the standard and ask for better performance.** Cheapest policy edit, but leaves the
   acceptance gap and categorical runtime placement intact.
2. **Add mandatory benchmarks or per-dimension gates.** Detects some damage later and introduces
   operational ceremony; does not itself improve physical architecture choices.
3. **Add execution fit to bounded architectural judgment (chosen).** Extend the existing owners
   and review slots, preserving independent semantic and evidence gates.

## Decision

Adopt repository-owned core/template 3.3 and code-intelligence profile 1.4. FP-07 and A4 assess a
credible physical route against functional-intent workloads, including relevant growth, skew,
concurrency and failure. Retain existing IDs and historical versions. ADR-0040's cadence and
ownership, ADR-0093's bounded model assessment and ADR-0109's coordination policy remain in force.

Declarations constrain meaning; derived layouts, execution plans, identity mappings and lifecycle
mechanisms are separately replaceable. Compare necessary work, composed library/optimizer
capabilities, locality, working sets, amortization, assurance placement, publication/recovery,
admission/contention, reusable views, work/output/completeness semantics and total machinery.
Static evidence can establish amplification; quantitative performance claims require measurements.
Safe refusal and bounded output alone do not establish workload fit.

CI-05 permits graph-native canonical artifacts, CI-07 separates analysis semantics from runtime
placement, CI-08 covers examined work and intermediates, and CI-13 pins the complete realization
for the declared consumer journey. All fidelity, uncertainty, evidence-closure and embedding
specification guarantees remain. No store choice follows automatically from this profile change.

Apply these criteria through existing review slots, planning and ADR alternatives. No new document
type, alignment linter, standing review or mandatory probe/benchmark campaign is introduced.

## Consequences

The policy and its skill consumers are Implemented. A future design can be revised for a known
bad work shape despite correct outputs. More expensive choices need a retained benefit against a
conforming alternative. Repeated checks can be removed only with a sound failure-coverage argument;
a content digest never establishes semantic correctness. Publication reconciliation follows
writer drain and content/definition freeze.

The source assessment §7 owns adoption closure. Documentation checks and independent static review
establish propagation scope; production performance and the graph-store pivot remain Proposed and
require their own implementation and qualification. No product code or store changes are included.
