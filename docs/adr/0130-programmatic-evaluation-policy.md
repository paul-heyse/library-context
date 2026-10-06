---
id: ADR-0130
title: Use primary programmatic evaluation with grounded outer feedback
status: accepted
date: 2026-10-06
supersedes: [ADR-0127]
superseded-by: null
design: [§2, §B10, §8, §12, §14.12]
evidence: Proposed
revisit: The evaluator cannot diagnose meaningful losses independently, outer feedback changes the yardstick invisibly, or optional machinery displaces the finite baseline.
---

## Context

The operator selected primary programmatic evaluation and an outer agentic loop feeding both the
system and evaluator, then explicitly accepted review RC01/RC03/RC04/RC05 on 2026-10-06.
[The review](../design_review/reviews/design_review_evidence-retrieval-and-programmatic-evaluation_2026-10-06.md)
identified an evaluator revision gap and a conflict between finite-first design and solver-first
appendix recommendations. Literal CI-12 also prohibited development tuning. This record accepts
the new policy and restates ADR-0127's enduring architecture standard; it does not claim runtime
implementation, product usefulness or closure of F01/F03.

## Options

1. Keep agent comparison primary and prohibit all evaluation-guided optimization. Reject: it
   does not implement the requested repeatable information-sufficiency improvement loop.
2. Use unversioned automated scores or agents as expected truth. Reject: changed tasks can appear
   to be system improvements and incorrect/shared oracle meanings can pass unnoticed.
3. Use independent programmatic judgments with frozen inner experiments and grounded outer
   feedback (selected). Finite references work before optional solver/model deployment.

## Decision

The programmatic loop invokes actual selected production interfaces, observes delivered readable
information and bounded public journeys, and judges independently grounded task/context witnesses
and supported finite worlds. It emits explicit sufficient/insufficient/inconclusive outcomes,
minimized failures and stage-loss diagnoses. Private oracle fields are excluded from production
inputs, embeddings, compilation, retrieval and query prompts.

Outer agentic studies remain independent usability/differentiation evidence. Grounded feedback
can correct the system under unchanged expectations, revise evaluator models/coverage/judgments,
or correct navigation/presentation. Agent success/failure alone is not oracle truth. Freeze
meaning/population/observation/metric definitions within each comparison. Changed meanings require
an explicit evaluator revision and new baseline/stratum; same applicable tasks may rejudge the
same retained packets to isolate the changed yardstick. No permanent historical runtime is needed.

CI profile/review guidance becomes1.5 with stable IDs. CI-12/CI-G3 distinguish authorized development
optimization from protected untuned confirmation/gold/heldout and runtime contamination. Gold,
sealed data and existing confirmation admission remain protected; their prior blocked receipts
are not promoted or rewritten. Core/template3.3 and heuristics1.0 remain unchanged.

Finite sets/contextual circuits and bounded enumeration are the initial evaluator route. BDDs
and an optional theory/optimization solver serve only a named operation that simpler references
cannot reasonably express. Solver calls preserve actual status/reason/results/bounds; a returned
vector/model/unsat core alone does not establish completed reasoning or oracle adequacy.
§B10 permits this offline lane and optional qualified non-generative evidence scoring. General
production composition/theory solving and generative compilation/query output remain excluded.
Native SurrealDB was already selected by ADR-0128; an obsolete blanket graph-database exclusion
is not retained. Scorer availability cannot override qualification or context validity.

**Architecture policy retained from ADR-0127.** Repository-owned core/template3.3 supplies FP-07/A4
alongside FP-01–06/A1–A3 and independent fidelity gates. Assess credible physical work against
functional intent, growth, skew, concurrency and failure: necessary/repeated work, bulk/native
composition, access, movement, live state, reuse, assurance placement and recovery scope.
Semantic owners do not dictate physical units or placement. Static evidence can expose amplification;
quantitative claims require measurements. Protective limits alone do not establish workload fit.
Use the adopted heuristics companion before consequential physical choices become fixed.
ADR-0040 cadence/ownership, ADR-0093 bounded model assessment and ADR-0109 coordination remain.
No standing model review, checklist, mandatory benchmark/probe, accounting system or proof artifact
is introduced. Historical versions and review verdicts retain their original meaning.

## Consequences

The primary loop can begin with pure kernels and real serializer observations without native/live
acceptance. Native, live-model, outer-study, sealed confirmation and comparative claims each need
their actual prerequisites. Development optimization is allowed under a fixed yardstick while
confirmation cannot select parameters. Profile edits are Implemented policy; the evaluator and
feedback operation are Proposed implementation at the [combined plan](../plans/evidence-retrieval-and-evaluation-plan_2026-10-06.md).
That plan owns review findings; ADR acceptance does not close them. Optional solver/scorer/tool
lifecycle must buy a named capability, not delay the finite baseline.
