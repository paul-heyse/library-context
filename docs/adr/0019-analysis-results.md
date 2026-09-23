---
id: ADR-0019
title: Analysis results are typed tables with in-row provenance and content ids, outside the CPG catalogs; new crates lctx-analytics and lctx-embed
status: proposed
date: 2026-09-23
supersedes: []
superseded-by: null
design: [§B6, §3.2, §3.4.1, §4.1, §5, §6.4, §9, §10]
evidence: Proposed
revisit: A consumer needs a finding, assertion or brief as a node or edge of the CPG catalogs (a query-time traversal through analysis results); an analysis result cannot be rebuilt from its snapshot, analytics config and compiler digest (it would then be an input, so a `facts` row); or the language-neutral serving schema form and the store's `canonical_schema` diverge in a way a reader notices.
---

## Context

Increment 1 slice 4 onward (DESIGN §1.2) adds the first consumers of the CPG:
- projections (§5);
- Passes A–C and the statistical analytics (§9);
- findings, assertions and briefs (§10);
- the serving bundle (§6.4).

§3.2 lists the `findings` family's tables but no contract. §B6 says `facts` rows are "the extracted
assertions, and later the analytic ones (findings)".

**The survey** (2026-09-23, recorded in the remaining-scope plan) found three conflicts with the
code as built:
- **`facts` is committed once per attempt, in `write_raw`, before any derivation.** ADR-0017's
  reader contract rests on one commit per snapshot-qualified table per attempt. A `facts` row per
  finding would need a second commit or a restructured attempt.
- **An `lctx-compiler` run over the library release** would trip `semantic:source-role-by-run`
  and `coverage:complete` if it declared families. Both assume every run is an extractor run.
- **The new tables have no table group.** `write_raw`, the rule generator's `shapes()`/
  `contracts()` and `schema_digest_of` iterate only `for_each_table!` and
  `for_each_derived_table!`. A table outside both would silently get no key or ref rules.
  `schema_digest_of` falls back to the `snapshots` schema for an unknown name.

**ADR-0008 and ADR-0014 already decided** that a Stage-C/D row rebuildable from cited `fact_id`s
and the snapshot's `compiler_digest` is not a `facts` row. An analysis result has the same
property: it is rebuildable from the snapshot, the analytics config and the compiler digest.

## Options

1. **The simpler alternative: findings as `facts` rows**, with the `facts` commit moved to the
   end of Stage F and the extractor's facts served from memory until then.
   - It keeps §B6's sentence.
   - It restructures the attempt, undoes H1's release of raw batches after writing, and makes
     `facts` mix inputs with derived output.
   - Rejected by the operator (2026-09-23).
2. **Findings as nodes and edges of the CPG catalogs.** `nodes`/`edges` are derived in Stage D,
   before analysis, and a catalog of the analysis's own results would be a second graph authority.
   Rejected.
3. **Typed analysis tables with in-row provenance, outside the catalogs (chosen).**

## Decision

**Tables** (family `findings`, a new `fact_family` code; a new table group
`for_each_analysis_table!`). The column contracts live in `cpg_schema::findings`.
- **`analysis_invocations`**: one row per invocation of an analytic method.
  - Its `run_id` and `model_id`, `extraction_mode` and `method` (`analytic_method`).
  - Its **parameters as canonical JSON** and their digest.
  - The projection spec's digest.
  - The library versions it ran.
  - Its seed.
  - Its diagnostics where the method has them: iterations, residual, convergence, quality
    history, candidate-set size.
  - Its completion as a `coverage_status`.
  - This is guidelines §8/§10's run record: a digest alone would not say what ran.
- **`findings`**:
  - kind (`finding_kind`), subject node and related node, invocation;
  - `evidence_status`;
  - the stop reason, `omitted_paths` and depth;
  - a score where the method has one;
  - a condition node where a guard applies (Pass B).
  - A finding is never a sentence (§10.1).
- **`finding_members`**: the members of a finding that has several (a community, a ranking, a
  concept's extent or intent) and the boundaries it cites, by role and ordinal, each a node or a
  `fact_id`.
- **`witnesses`**: ordered path steps.
  - **Keyed** by `(finding, path, step)`, with the step's call-site and callee node ids.
  - The `edge_id`, modality and phase are **lineage**. Call edge ids are producer-scoped
    (ADR-0014 O6), so they never enter a finding's identity.
  - This is §5's persisted "projection row: links an arc to its evidence rows".
- **`evidence`**: one row per cited thing: a fact, a byte span of a source file, a passage, an
  example module, a fixture run. Each holds the **resolved text** the serving bundle carries,
  because the server reads no Delta.
- **`assertions`**:
  - kind, subject, section (`brief_section`), applicable case, conditions and limitations;
  - derived `evidence_status`;
  - text (null when `unresolved`);
  - template version;
  - run, model and extraction mode.
- **`assertion_support`**: an assertion's findings and evidence, each with a **role**: `support`,
  or `scope` for a finding that defined the assertion's scope (§10.2's "any finding that defined
  its scope").
- **`briefs`**:
  - the seed operation, applicable case and title;
  - `documentation_only` (§1.5's label);
  - `review_state`, which is **not** part of `brief_id`.
- **`brief_assertions`**: brief → assertion, by section and ordinal.
- **`brief_members`**: the public access paths and declarations a brief is about.
  `symbol_map` (§6.4) is built from it.
- **`brief_documents`**: the §11.1 embedding projection of a brief, chunked, with its
  `input_hash`: how the bundle finds a brief's vectors.
- **`assertion_policy`**: the kind policy (§10.2), `assertion_kind` → section and permitted
  statuses, published per snapshot like `edge_kinds`. The policy and status-propagation rules are
  generated from it.
- `usage_patterns` and later kinds' tables join the group with the slice that produces them.

**Identity** (§3.4.1, `lctx_id` kinds, one recipe in Rust and SQL). Each id is content-derived
and contains **no analytics-config digest**, so an unchanged result keeps its id when parameters
change, and an ablation diff is a join:

| Id | Recipe |
|---|---|
| invocation | method, parameters digest, projection digest, subject, seed |
| finding | kind, subject, related node, canonical payload (witness steps by node ids, depth, stop reason, omitted flag) |
| evidence | kind, fact, node, module, span |
| assertion | kind, subject, section, applicable case, text, status, sorted supports |
| brief | seed, applicable case, sorted (section, ordinal, assertion) |

- `capability_id = brief_id`.
- A changed brief is a new brief, which is what makes a recorded review content-bound
  (ADR-0020, increment 3).

**Provenance in-row.**
- Analysis tables have no `fact_id` column. So no `fact:` rules apply, and the `facts` write is
  untouched.
- `run_id` and `model_id` are checked by generated `ref:` rules plus a model-to-producer rule.
- The run is the **`lctx-compiler`** run (§3.4.1):
  - its config digest is the analytics config's;
  - its tool revision is the `compiler_digest`;
  - its `runs` and `producers` rows go in the raw write, because every input is known before
    Stage B, so `content_digest` includes it.
  - **It declares no families.** Its completion lives in `analysis_invocations`, not `coverage`.
  - The extractor-only rules (`semantic:source-role-by-run`, the coverage rules) are scoped to
    runs that declare a code family.
- **Operator review verdicts** are the one analysis-side *input*. They are `facts` of a
  `manual-review` producer, written in the raw write (ADR-0020).

**Outside the catalogs.** Findings, assertions and briefs do not enter `nodes`/`edges`, which
Stage D derives before analysis runs. They reference catalog nodes and edges by id.

**The serving schema digest.**
- The serving bundle's files (§6.4) are described by a **language-neutral canonical form**: field
  name, a declared type grammar (`fixed_size_binary(16)`, `utf8`,
  `fixed_size_list(float32 not null "item", 4096)`, …), nullability and sorted metadata,
  **hashed with SHA-256**.
- Python recomputes the digest with its standard library, and known answers are shared by the
  Rust and Python tests.
- The store's `canonical_schema` (the `snapshots.schema_digest` and `compiler_digest` input) is
  unchanged.

**Crates** (ADR-0012's single workspace):
- **`lctx-analytics`**: the §5 adapter and the analysis kernels, Arrow in and Arrow out, with no
  DataFusion or Delta.
- **`lctx-embed`**: the embedding spec, the vLLM client and the fake embedder, behind a trait
  that `cpg-core` declares, so `reqwest` stays out of `cpg-core`.
- **Stage F** (evidence SQL, templates) lives in `cpg-core::synth`.

## Consequences

- **§B6 is amended:** `facts` rows are the extracted assertions and operator review verdicts.
  An analysis result is traced by its run, its invocation record, the ids it cites and the
  snapshot's `compiler_digest`, and is rebuildable from them. The ADDENDUM §B6 row and ADR-0014's
  scope sentence say the same.
- **Every new table gets generated `key`/`ref`/`codebook` rules** through its table group.
  `schema_digest_of` refuses an unknown table name.
- **`compiler_digest` gains:**
  - an analytics output version;
  - the projection and synthesis SQL;
  - the kind policy;
  - the template version;
  - the locked versions of the analytics crates' dependencies.
- **The `findings` family is not a coverage unit.**
- **Ablation** (§9.8) joins on content ids across two snapshots.
- **Tested** as each slice lands: generated rules reject injected violations; shuffled input
  gives byte-identical analysis tables; two identical compiles give identical ids.
