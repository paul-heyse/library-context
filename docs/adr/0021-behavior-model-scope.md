---
id: ADR-0021
title: The product is an evidence-carrying behavioral model of a pinned library's whole public surface; briefs are one rendering
status: proposed
date: 2026-09-24
supersedes: [ADR-0004]
superseded-by: null
design: [§1.1, §1.2, §1.3, §1.4, §1.5, §9, §12, §13]
evidence: Proposed
revisit: A stage's exit criterion (plan §6) fails twice in a row, or the Stage 1 structured evaluation shows the whole-surface tools answer no question the briefs could not.
---

## Context

ADR-0004 made stage 1 a capability compiler: about 20 briefs, each anchored to one public
operation, served by `search_capabilities` and `get_capability`.

The operator now targets a rich, programmatic behavioral basis. The direction comes from
`docs/behavioral_model_pivot.md`. The deep review of that document
(`docs/design_review/reviews/design_review_behavioral-model-pivot_2026-09-24.md`, decision
**Revise**) measured three things on the pilot snapshot `ec03626f`:
- Pass A–C findings exist for **20 of 1,534** public declaration nodes. The analysis loops over
  seeds, not over the public surface.
- The whole-release relations the passes read are computed every compile and never persisted,
  including 32,075 mapped argument flows.
- Of 12 behavioral questions grounded in FastMCP 4.0.5's source, the pivot's proposed local and
  interprocedural relations answer five. The rest need `self` fields, the settings singleton,
  ContextVars, registries, and exceptions converted to values.

The plan `docs/plans/behavioral-model-pivot-plan_2026-09-24.md` stages the change. The operator's
instruction to implement it took each of its decisions D-1 to D-9 as recommended (deviation log
B1).

## Options

1. **The simpler alternative: keep the capability compiler and raise the brief budget.** Rejected.
   Briefs are sentences over seed-scoped findings, so a larger budget adds prose, not queryable
   behavior. Every question about an operation outside the budget still gets silence, and silence
   reads as "none" (review F2).
2. **The pivot as written:** seven layers, including Graph-FCA and on-demand RCA, with no exit
   criteria. Rejected as a design. It leaves four gates unresolved (review §6), and it would split
   concept and predicate authority (F7, F12).
3. **A behavioral model of the whole public surface, built in stages with exit tests** (plan §6).
   Chosen.

## Decision

- **The product.** A pinned library compiles into an evidence-carrying **behavioral model of its
  whole public surface**, served to coding agents:
  - exhaustive answers over materialized relations where the analysis is complete;
  - ranked discovery where it is not;
  - a named `unknown` where the analysis stopped.

  Briefs remain as one rendering, for a curated subset (D-6).
- **The universe** is `public_paths` under the config's public roots. Every analysis runs per
  public callable, and its scan follows parameters into any release function; a seed traversal
  becomes a query.
- **The subsystem scopes briefs only** (the standard review's F1): seed selection and the seed
  passes behind briefs. On the pilot, 1,107 of the 1,534 public nodes lie outside its prefixes,
  and the behavior model covers them.
- **Unchanged:**
  - programmatic synthesis, with no generative model in the pipeline or the query path (§B11);
  - immutable, byte-identical generations (§B7, §B12);
  - one pinned generation per server process (§B13);
  - the gold is evaluation-only, never a compiler input;
  - **Every freeze ADR-0004 held is this ADR's** (the standard review's F3), with the disclosure
    clause that an edit names the gold its author has seen. That covers `analytics.toml`, the
    code parameters (communities, PageRank, FCA, kNN), the selection parameters and the variant
    policies. They are recorded in `eval/gold/analytics-freeze.json`, and an edit to any of them
    is an amendment to this ADR.
- **The pilot** stays FastMCP 4.0.5 (`libraries/fastmcp`), with the same subsystem and analytics
  config ADR-0004 chose. It restates ADR-0004's evaluation-only gold rule and its review cadence
  (`deep` after increments 1, 3 and 5; `compact` after 2 and 4).
- **Increments** (DESIGN §1.2):
  - increments 1 and 2 are done;
  - **increment 3** closes with plan Stages 0–1 (the whole public surface);
  - **increment 4** is Stages 2–3 (flow IR, conditions, verdicts, summaries, models);
  - **increment 5** is Stages 4–5 (the capability registry, framework models, protocols), then
    the held-out structured evaluation.

  Each stage also ends with a `compact` review.
- **Evaluation** (the operator, 2026-09-23). Pre-registered behavioral question sets
  (`eval/behavior/`) whose targets are written from the library's source and docs before any
  output is read, assessed qualitatively (present / partial / absent / incorrect / misleading),
  plus mechanical known-answer fixtures. No API agents. The gold's retrieval metrics (§12(a)–(c))
  stay a record for briefs.
- **§9's technique rule** changes from "a named consumer in the brief" to "a named consumer in the
  served model": a tool's output or a brief.
- **The keep criterion** (the standard review's F4; DESIGN §9.8).
  - **Turned on by default** only if the structured evaluation of the stage that introduces the
    variant's tool consumer shows three things:
    - it supplies at least one target item, rated present or partial, that the default lacks;
    - it introduces no item rated incorrect or misleading;
    - it pushes no target operation or item out of a tool's first page.
  - **Deleted by ADR,** with its tests and frozen parameters, if it has no tool consumer by the end
    of increment 5, or fails the criterion at two consecutive stages.
  - **Communities have no tool consumer today.**

## Consequences

- **Easier:** asking about any public operation; writing new analyses as persisted relations that
  have consumers at once; judging analyses by the questions they answer.
- **Harder:**
  - the serving bundle grows;
  - the server gains tools and a bounded executor (ADR-0010 amendment);
  - behavioral relations need their own known-answer fixtures and coverage rules.
- **Retired from ADR-0004:**
  - "a small set of searchable capability briefs" as the product;
  - the definition of done built on three derivation families' briefs. That record stays in
    DESIGN §1.5 as history.
- **Revisit:** a stage's exit criterion fails twice, or Stage 1's structured evaluation shows no
  gain over briefs.
