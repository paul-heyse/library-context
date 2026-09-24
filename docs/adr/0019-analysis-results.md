---
id: ADR-0019
title: Analysis results are typed tables with in-row provenance and content ids, outside the CPG catalogs; new crates lctx-analytics and lctx-embed
status: accepted
date: 2026-09-23
supersedes: []
superseded-by: null
design: [§B6, §3.2, §3.4.1, §4.1, §5, §6.4, §9, §10]
evidence: Tested
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
  - `evidence_status`, the one its kind permits (`FINDING_STATUS`, a generated rule; review F6);
  - the stop reason, `witnesses_omitted` and depth;
  - a score where the method has one;
  - a condition node where a guard applies (Pass B).
  - A finding is never a sentence (§10.1).
  - A rooted analysis (Pass A) carries its root, the seed, as the subject. A result several
    invocations produce (increment 2's seed consensus) is published once, as the consensus, or
    linked by a `finding_invocations` table added with that slice (review F1).
- **`finding_members`**: the members of a finding that has several (a community, a ranking, a
  concept's extent or intent) and the boundaries it cites, by role and ordinal, each a node or a
  cited fact (`cited_fact_id`).
- **`witnesses`**: ordered path steps.
  - **Keyed** by `(finding, path, step)`, with the step's call-site and callee node ids, modality
    and phase.
  - The `edge_id` is **lineage**: call edge ids are producer-scoped (ADR-0014 O6), so they never
    enter a finding's identity. Modality and phase are codebook values, so they do (review F9).
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

**Identity** (§3.4.1). Each id is content-derived and contains **no analytics-config digest**,
so an unchanged result keeps its id when parameters change, and an ablation diff is a join.
- Each contract declares its **identity** columns and its **lineage** columns (ids like
  `edge_id` and `invocation_id`, and values like `score` whose change should not rename a
  result). A finding's id hashes every identity column, plus its child rows in key order: each
  witness step's call site, callee, modality and phase (codebook values, stable across producers;
  review F9), and each member's role, ordinal, node, cited fact and label.
- The recipes are Rust-only (`cpg_schema::findings::recipe`): `lctx_id` takes scalars, not lists.
  A property test per contract checks that perturbing an identity column changes the id and a
  lineage column does not (review F1).
- Two seeds that name one declaration refuse the compile.


| Id | Recipe |
|---|---|
| invocation | method, parameters digest, projection digest, subject, seed |
| finding | kind, subject, related node, condition node, status, depth, stop reason, `witnesses_omitted`, witness steps (call site, callee, modality, arc kind, phase), members |
| evidence | kind, node, module, span, digest of the resolved text; a cited `fact_id` is lineage, because fact ids are run-scoped (review F7) |
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
  - Only `semantic:source-role-by-run` needed scoping: the coverage rules pass for a run with no
    families, because unnesting an empty list yields no rows (review O1). `coverage:declared-family`
    now also refuses a run declaring a family that is no coverage unit (`graph`, `publication`,
    `findings`, `embedding_cache`).
  - Every invocation's run is the compiler's (`semantic:invocation-run-is-compiler`).
  - Without an analytics config there is no compiler run, and the analysis tables are written
    empty, so "not requested" is readable (review O9).

**Completion** (review F4). The depth bound and the subsystem, dependency and synthetic
boundaries are the stated model: a result inside them is complete under it. A vertex or arc
budget is operational truncation: the invocation is `partial` with its stop reason, and Stage F
turns it into a Limits entry. The witness cap is presentation only (`witnesses_omitted`).

**Parameters** hold every config field the method reads (review O3); the projection spec is
recorded by digest, and its SQL is part of the compiler digest.
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
- No analysis table has a column named `fact_id`: a cited fact is `cited_fact_id`, so the
  generator never demands a `facts` row for an analysis row (review O2).
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

## Amendments (in place; the record is still proposed)

- 2026-09-23: the standard review (`design_review_adr-0019-analysis-results_2026-09-23.md`,
  Revise) P1 items, decided before slice 1.4's contracts were committed: identity and lineage per
  contract, Rust-only recipes with a property test, and refused duplicate seeds (F1); DESIGN §6.4,
  §9 and §10.1 amended (F2); the analytics config's subsystem and distractors rewritten by a
  docs-only author (F3, deviation log D3); the completion mapping and the `witnesses_omitted`
  split (F4); §B6's labels and the ADR-0020 pointer (F5); the finding-status policy and the
  compiler-run rule (F6); modality and phase in the step key (F9); O1, O2, O3 and O9. F7, F8 and
  O5 are owed by slices 1.5 and 1.6 (F7 decided now: evidence identity by content, the fact as
  lineage); O6, O7 and O10 are deferred with the review's triggers.
- 2026-09-23: the slice 1.4 compact review (`design_review_inc1-slice1.4-pass-a_2026-09-23.md`,
  Revise): a witness step gains `arc_kind` (`call` | `definition`, a new codebook) as an identity
  column, and its `phase` is null on a definition arc (F1, deviation log D5). The finding recipe
  row above says so.
- 2026-09-23: the slice 1.5 compact review (`design_review_inc1-slice1.5-synthesis_2026-09-23.md`,
  Revise):
  - `finding_kind` appends `traversal_stop`, so a depth or budget Limits entry cites a finding
    (F1, deviation log D10).
  - An assertion's status is `findings::derive_status` of its supports, with `EVIDENCE_STATUS`
    and `STATUS_STRENGTH` declared beside `FINDING_STATUS`. A generated rule recomputes it (F1).
  - `AssertionKey` hashes its supports sorted, as the recipe table says (F6).
- 2026-09-23, slice 1.7: `brief_documents` gains `spec_hash`, since one text has a vector per
  spec. A new analysis table, `embedding_specs`, holds the snapshot's spec as canonical JSON, so
  the serving bundle is built from the store alone. Three rules are added:
  `semantic:document-key-whole`, `semantic:document-vector-cached` and
  `semantic:one-embedding-spec`.
- 2026-09-23, slice 2.1 (Pass B):
  - Appended values: `analytic_method` `pass_b_flows`; `finding_kind` `forwarding`,
    `transformed_argument` and `conditional_raise`; `member_role` `source_parameter`, `value`,
    `alias` and `formal`; `assertion_kind` `control`, `transformed_control` and `restriction`.
  - A Pass B invocation records the flows relations' digest as its projection digest.
- 2026-09-23, increment-1 deep review (U2 and F4, deviation log D20; the 1.5 review's O6
  decided):
  - The brief's **slot sections** are Outcome, Public access, Applicable case, Controls, Usage
    pattern and Limits (`findings::SLOT_SECTIONS`).
  - A slot section without an assertion is an **absent slot**. The manifest counts absent slots
    by section beside the explicit `unresolved` assertions (`absent_slots`, `slot_sections`), and
    `get_capability` names each brief's.
  - The §B11 gap metric counts both.
- 2026-09-23, slice 2.2 (Pass C, usage patterns; deviation log D24):
  - Appended values: `analytic_method` `pass_c_handoffs`; `finding_kind` `handoff`; `member_role`
    `producer_site` and `consumer_site`; `assertion_kind` `usage_pattern` and `handoff`.
  - A usage pattern is an assertion citing `example` evidence, not a `usage_patterns` table.

## Amendments

After acceptance (2026-09-23), amendments are appended here, dated. Those above were made while the record was proposed.

- 2026-09-23 (the slice 2.2 review's O3): one entry above is not what that sentence says. The
  slice 2.2 entry was appended after acceptance, at `cf0990f`, before this section existed.
  Later appended values go here:
  - slice 2.1 review: `finding_kind` `unfollowed_argument`; `member_role` `conditional_call` and
    `reason`; `assertion_kind` `unfollowed_control`;
  - slice 2.3: `analytic_method` `leiden` and `community_consensus`; `finding_kind` `community`;
    `member_role` `community_member` and `supporting_site`; `analysis_invocations.diagnostics`
    (a declared migration);
  - slice 2.4: `analytic_method` `pagerank`; `finding_kind` `centrality`.
  - slice 2.5: `analytic_method` `fca_next_closure`; `finding_kind` `applicable_case` and
    `implication`; `member_role` `extent_member`, `intent_attribute`, `premise` and `conclusion`;
    `assertion_kind` `applicable_case` and `implication`; `stop_reason` `concept_budget`;
  - slice 2.6: `analytic_method` `seed_selection`;
  - slice 3.1: `analytic_method` `knn`; `finding_kind` `doc_link` and `community_label`;
    `member_role` `label`; `assertion_kind` `doc_link`;
  - the increment-2 review: `analytic_method` `usage_count`; `finding_kind` `direct_usage`;
    `assertion_kind` `shared_signature` (the `applicable_case` kind is kept, reserved).
- 2026-09-23 (the increment-2 review's deferred rows):
  - **Stage E composes in memory** (ADR-0019 review O6). Every method's rows are collected in
    one `AnalysisRows`; a later method reads earlier ones (selection reads communities and direct
    usage, Stage F reads all) from memory, never from Delta. Each analysis table is written once,
    after Stage F, inside ADR-0017's one commit.
  - **A finding kind has one method** (`findings::FINDING_METHOD`); the rule
    `semantic:finding-kind-by-method` checks each finding's invocation against it, with an
    injected case.
