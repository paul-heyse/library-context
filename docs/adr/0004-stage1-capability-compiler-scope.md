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

## Amendments

- 2026-09-22: pointer only. ADR-0012 now records the DESIGN.md budget (~1,450 lines), raised to
  hold the extraction and fact-construction detail (§4.2.1–§4.2.6, §4.3).
- 2026-09-22: operator decision. DESIGN.md has no line budget; the detail the design needs comes
  before length. This replaces the budget line in the Decision and the ~1,450 figure above (which
  ADR-0012 itself never recorded).
- 2026-09-22: operator decision. The pilot is FastMCP 4.0.5, acquired as `libraries/fastmcp`
  with the fastmcp skill's install line (release `fastmcp`, `fastmcp-slim`, `fastmcp-tasks`), and
  the skill moves to 4.0.5 with it (ADR-0013). This replaces "4.0.3" in the Decision and the
  "analyzed 4.0.3 and served 4.0.5" consequence above.
- 2026-09-22: operator decision. The CPG is completed before the analytics. Slices C1–C6
  (ADR-0014) run before Pass A: the node/edge catalogs, then the syntax, lexical, types and docs
  families (including the examples and tests usage corpus). Each family names the consumer that
  will read it. Review cadence: compact per slice, standard per ADR, deep at C6. This replaces
  "the CPG grows only as consumers need it" as a sequencing rule; the named-consumer requirement
  stays.
- 2026-09-23: remaining-scope plan, Phase 0 (operator: plan approved, "run straight through").
  - **Hybrid retrieval moves into increment 1.** BM25 over `lexical_text`, reciprocal-rank fusion,
    exact-symbol promotion and the degraded lexical-only mode (§11.2) are built with the first
    server (slice 1.8). The spike already had them, and the §9.8 keep rule (increment 3) must be
    judged on the stack that is served. Increment 4 becomes the generation lifecycle (smoke query,
    activation, byte-rebuild verify, manifest check at load) and serving reliability.
  - **The analytics config** is `libraries/<name>/analytics.toml`, a file of its own so its
    history is the pre-registration record. It is **frozen** (its commit recorded) before the
    first gold scoring, and changes after that only by ADR.
  - **Increment 2's "FCA within one community"** reads "FCA within one structural scope", as §9.6
    and ADR-0011 require: a community's membership is statistical.

- 2026-09-23, increment-1 deep review O1 (deviation log D21):
  - **The analytics config's freeze** is the first use of gold against compiled output: slice
    1.9's §1.5 check (6c86f53, config SHA-256 `306416fe…`), not the 3.3 scoring.
  - Two edits followed. This amendment is their record:
    - Slice 2.0 added the seed `fastmcp.FastMCP.mount` by the mechanical audit written in the file.
      Its author had read the gold catalog, where that method is an operation of `fm.mount_proxy`.
      The seed is kept: its selection rules are gold-free and reproducible. The overlap is
      disclosed here.
    - The review's F3 removed `[briefs] serve_unreviewed`, which nothing read. §10.4's review gate
      arrives with its consumer, the 3.5 workflow. `budget` now binds: more seeds than it are
      refused.
  - The config is frozen again at `eval/gold/analytics-freeze.json`. `just gold` fails when
    `libraries/fastmcp/analytics.toml` differs from it. Any later edit is an amendment naming the
    gold its author has seen.
