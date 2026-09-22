---
id: ADR-0004
title: Stage 1 is a capability compiler for a FastMCP pilot on a demand-driven CPG
status: accepted
date: 2026-09-22
supersedes: []
superseded-by: null
design: [§1.1, §1.2, §1.4, §1.5, §12]
evidence: Proposed
revisit: Increment 1 cannot produce one end-to-end brief for fm.register without facts outside the declared v1 slice, or the gold subsystem proves too broad for ~15-25 briefs.
---

## Context

The updated Initial_plan (IP L1650–L3085) turns the fact graph into a product. It is an offline
**capability compiler**: code facts plus official docs, examples and tests are compiled into a
few dozen evidence-backed briefs, served through `search_capabilities` and `get_capability`.

The earlier spine (DESIGN §1.2 as seeded) built the fact substrate, then projections, then
execution semantics. It had no product and no definition of done.

The operator chose:
- FastMCP as the pilot;
- FastMCP as the agent interface;
- programmatic synthesis (ADR-0005).

## Options

1. **The simpler alternative: finish the fact substrate first**, then decide the product. That
   was rejected: it builds tables with no consumer (DM-58), and it was the pattern the baseline
   review flagged (F1).
2. **PyArrow `dataset` as the pilot**, as IP L1689 suggests. That was rejected for stage 1:
   - `Scanner` and the write path are Cython with no stubs, so Pass A stops at a native boundary
     one call deep;
   - docs and examples need a separate tarball;
   - there is no gold reference.
3. **FastMCP 4.0.3, server-components subsystem, demand-driven CPG, five vertical increments.**
   Chosen.

## Decision

- **Scope, subsystem and done.** DESIGN §1.1, §1.2, §1.4 and §1.5 as written.
  - **Subsystem.** The pilot is FastMCP 4.0.3's server-components surface. The compiler takes it
    **only** from the pre-registered analytics config (module prefixes and public roots), whose
    digest is in `content_digest`.
  - **Evaluation.** Its correspondence to gold families `fm.register`, `fm.inputs`,
    `fm.outputs`, `fm.errors`, `fm.resources` and `fm.middleware` is measured, never used as
    input.
  - **Increment 1** has a hand-registered seed (`fastmcp.FastMCP.tool`) plus 2–4 distractor
    briefs, so retrieval by task wording is a real test.
- **Demand-driven CPG.** The CPG grows only as consumers need it (§3.2 families by increment).
  The full ontology remains the target model.
- **Evaluation-only gold.** The `fastmcp` skill is the gold reference for §12 and is never a
  compiler input. Analytics parameters are pre-registered. The gold is the development metric
  for keeping or removing techniques (DESIGN §9.8); the increment-5 held-out tasks are the
  unbiased check.
- **Review cadence** (replaces "deep at every increment end" in ADR-0001):
  - `deep` after increments 1, 3 and 5;
  - `compact` after increments 2 and 4.
- **DESIGN.md budget.** About 1,200 lines. Column contracts live in `cpg-schema`, and the
  research-input disposition lives in `docs/initial_plan/DISPOSITION.md`.

## Consequences

- **What we gain.** Every table and analytic must name a consumer in a brief. The gold skill
  gives a scoring oracle (§12) that the research input did not have.
- **What we give up.** The native-extension case (PyArrow) is deferred to a later corpus.
- **Risk.** FastMCP is both the subject and the serving framework. Analyzed 4.0.3 and served 4.0.5
  are kept apart by pins (`docs/pins.md`).
