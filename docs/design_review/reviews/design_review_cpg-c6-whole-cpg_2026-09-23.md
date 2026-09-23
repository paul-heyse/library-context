# Design review: the whole CPG after slices C1–C6 (deep)

**Date:** 2026-09-23
**Reviewer:** `design-reviewer` subagent (fresh context), running the `design-review` skill. The
author of C1–C6 is not the reviewer.
**Target:** the code property graph as built by C1–C6, commit range `6bf223e^..c07d848` (12
commits), against DESIGN.md (§1.2, §3.1–§3.8, §4.0, §4.3, §6.1, §8), ADR-0014, the ADR-0013,
ADR-0009 and ADR-0004 amendments, the charter, and `ADDENDUM.md` §5.
**Depth:** deep (ADR-0004's re-sequencing puts a deep review at C6).
**Prior reviews:** `design_review_adr-0014-cpg-graph-catalog_2026-09-22.md` and the C2, C3, C4
and C5 compact reviews, each with a Disposition and Deferred rows. Deferred rows are not
re-raised here unless the evidence below shows a trigger has fired or the reasoning was wrong
(§7.3).

---

## 1. Decision and scope

**Proposal.** The CPG as a whole: typed family tables (provenance, exports, signatures, calls,
coverage, syntax, lexical, types, docs), their Stage C/D derivations, the derived `nodes`/`edges`
catalogs generated from one registry (`cpg_schema::graph`), and the generated validation rules.
One compile attempt holds two runs: the library release and its source corpus (docs, examples,
tests and materialized code blocks).

**Status.** Implemented. The per-slice claims are labelled in DESIGN §3.2 and §3.8. The C6
measurement is in §4.3.

**Affected revisions.** Code at `c07d848`. Fork `6a93da34`. The published pilot snapshot is
`ab6d98a3cb3ff54dc06d9a07600c0668` (FastMCP 4.0.5, corpus at `004bf15a`), with content digest
`c6ed7392…` and compiler digest `813f8e15…`.

**Observable outcome.** Every later analysis (Passes A–C, FCA, community detection, §10
synthesis) reads one validated, snapshot-pinned graph. It has persistent node and edge
identities, typed endpoints, a declared derivation class per edge kind, and explicit
completeness.

**Baseline.** Before C1 there were five typed families with Stage C/D joins and no node or edge
relation (ADR-0008).

**Supported scope and non-goals.** Direct relations only: no closures, no CFG or dataflow
(§1.3, §13). No consumer is built yet. The first one, Pass A, is increment 1 slice 4. So
consumer claims can only be checked as "does a source exist for what the consumer reads".

**Constraints and uncertainty.** One pilot library. A second library has not been compiled end
to end at the C6 build. Pyrefly is pinned, and several ids are producer-scoped, as declared.

### Method and coverage

**Read in full, at the grain cited:**
- `crates/cpg-schema/src/graph.rs`, `derived.rs` and `rules.rs`;
- in `crates/cpg-core/src/`: `attempt.rs`, `validate.rs`, `derive.rs`, `sql.rs`, `snapshot.rs`
  and `delta.rs`;
- the compile path of `crates/lctx/src/main.rs` (L136–297);
- `crates/cpg-extract/src/lib.rs` L260–890 (`run`, `merge`, `run_release`);
- `library.rs` `select`/`corpus` (L525–628);
- `context.rs` L1–120.

**Read in part:**
- `tables.rs`: every key, plus the `source_files`, `releases` and docs-family contracts;
- `codebook.rs`: the node, edge, field and Pysa kinds;
- `udf.rs` L1–120 and `id.rs` recipes;
- the test inventory, plus the cases in `every_rule_kind_rejects_its_violation`, the graph.rs
  readiness and location tests, and `contracts.rs`;
- Pyrefly `lib/report/pysa/call_graph.rs` L2939–2960 at `6a93da3`.

**Not read:** `lexical.rs`, `types.rs`, `walk.rs`, `syntax.rs`, `docs.rs`, `pysa_map.rs`,
`public.rs` and most of `config.rs`. The per-slice reviews read those. Here their outputs were
tested at whole-graph level through the pilot store and the rules instead.

**Checks run in this session:**

| Check | Command | Outcome |
|---|---|---|
| Everything | `just test-all` | **passed** (3 min 33 s): fmt-check; clippy `-D warnings`; nextest 82/82 (slowest `every_rule_kind_rejects_its_violation` 211 s); pytest 21/21; pyrefly; rules-scan; rules-test 4/4; lint-agents; adr lint (14); fixtures 37; family; cargo-deny; pyrefly-fork `6a93da34`; gold |
| R1, rerun | `target/release/lctx compile fastmcp --store build/review-store` (binary built 05:01 from the committed sources; only a test file is newer), with VmRSS sampled every 0.25 s | **passed**: snapshot `41e7d32c…`, content `c6ed7392…`, 44.2 s, peak 7,303 MiB |
| R2, relocated | the same, with `--envs` and `--sources` in the session scratchpad (a fresh `uv sync` into a new path, and a copy of the tree) | **passed**: snapshot `e8f56962…`, content `c6ed7392…`, 44.5 s, peak 7,942 MiB |
| R3, arena-limited | R1 with `MALLOC_ARENA_MAX=2` | **passed**: snapshot `75ae32ff…`, 60.9 s, peak 4,321 MiB |
| Rerun and relocation identity | sha256 of `lctx query … ORDER BY` dumps of `nodes`, `edges`, `type_terms` and `contexts` | **passed**: R1 and R2 are byte-identical to `ab6d98a3` in every dump. Per-table row counts are identical |

All scratch stores and relocated copies were deleted afterwards. R1 used the default
`build/sources`, so it rewrote `_lctx_blocks/` inside the shared tree. The bytes are identical,
and `git status --porcelain` there shows only that untracked directory.

**Probes on `ab6d98a3`** (read-only `lctx query`): 25 queries. Each is cited where it is used, as
**Q:**.

**Not attacked, so asserted only:**
- the Pyrefly fork patch and the harness-equivalence oracle;
- the UDF's plan-time refusals;
- concurrent compiles into one store;
- the ambiguous-append classification (its test passed);
- retention beyond its passing test;
- a second library (`attrs`) at the C6 build;
- the C1–C5 Measured blocks, other than their counts that C6 carries forward.

---

## 2. Authority and lifecycle map (compressed)

| Concept or fact | Identity | Authority / owner | Revision boundary | Update path | Derived representations |
|---|---|---|---|---|---|
| Raw family rows | `fact_id` per run (payload-hashed) | the extractor, one surface per table | attempt (`snapshot_id`) | one append per attempt | every Stage C/D table |
| Merged records (Stage C/D) | keys plus the cited `fact_id`s | `cpg_schema::derived`, one query each | `compiler_digest` | code change | the catalogs |
| The registry | Rust values: `node_sources`, `edge_sources`, `node_columns` | `cpg_schema::graph` | `compiler_digest` (through the SQL and rules it generates) | code change | `nodes`, `edges`, `edge_kinds`, `graph_gaps`, about 240 of the 492 rules |
| Catalogs | `node_id`, `edge_id` (content-derived) | derived only; no writer | snapshot | none (rebuildable) | projections (§5, not built) |
| What two runs both assert | one node or edge, the "first fact" by `fact_id` | the raw tables keep both rows | snapshot | — | `nodes` merge rows for Export, ExternalModule, ExternalSymbol and Type (`graph.rs:1260-1276`); `first_fact` for `declared_in`, `type_arg` and `type_class` |
| Attempt structure | two runs (library, corpus), two contexts, one producer | `cpg_extract::run`/`merge` (`lib.rs:267-322`) | attempt | code | none. **Which release is the library** is inferred from `runs.families` (O3) |
| Publication | the `snapshots` row set | `attempt::publish` | one commit | append only | the reader session |

**Deliberately opaque.** Pyrefly's Pysa collectors and native types enter through exhaustive
mappers. Our recognizers (lexical, mentions) are labelled `recognizer`. Both are declared in
§3.2 and §3.5.

**Identity behaviour.**
- **Rerun and relocation are stable at pilot scale** (R1, R2). This covers both runs.
- **Some ids are producer-scoped, by declaration.** A Pyrefly or Ruff bump renames syntax ids,
  type-term ids, external-symbol ids and call-edge ids (§3.4.1).
- **A library upgrade renames both runs' ids.** It renames every library node, and every corpus
  node too, because the corpus release hashes the library `release_id` (C5 F1(b)).

---

## 3. Semantic contracts and invariants (whole graph)

| Invariant | Enforcement | Failure behaviour | Evidence today |
|---|---|---|---|
| One node, one kind, one id | `key:nodes`; merging only the four declared kinds (`graph.rs:1260-1276`) | validation fails; nothing publishes | 905,648 nodes. Merged: 1,853 type terms emitted by both runs (20,483 rows → 18,630 terms); 1,109 duplicate `context_definitions` rows (3,687 → 2,578); 267 duplicate `context_modules` rows (845 → 578 ids, 527 nodes, 51 `not_found`) (**Q**) |
| Merged rows agree across runs | `unique:type_terms` (kind, detail, display) | fail | `context_definitions` and `context_modules` have no agreement rule (C5 O3, deferred). Pilot: 0 ids whose rows differ in name, qualified name, top-level flag, module, origin, distribution or path (**Q**) |
| A release file is one module whatever run references it | the corpus names release files by `@path` (`lib.rs:384-416`); `unique:release-paths`; the shadow check (`lib.rs:436-451`) | compile error or validation failure | 0 `context_modules` rows carry a release module name or a release distribution. 0 type terms name a release module by dotted name. Class terms split by run: 0, apart from distinct enum members (**Q**) |
| Every endpoint is a typed node of an allowed kind | `endpoint:*` (35), `ref:*->nodes` (78) | fail | all pass |
| Every raw row yields its declared edges, or a provider reason | `lineage:*` (32), `typed:*` (11) | fail | 17 of the 32 lineages cannot fail on today's SQL (F1) |
| Per-kind modality and origin (Pass A's "never crosses `potential` or synthetic") | the edge SQL's `WHERE` clauses only; `edge_kinds` does not publish them | none | `call_target`: 50,999 definite and 17,141 candidate, 0 potential, 0 `synthetic_model`. `site_target`: 0 potential. `higher_order_target` (1,893) and `potential_target` (18,605) are all potential (**Q**) |
| `has_type` is one per subject | none (no rule) | none | 0 subjects with two edges (**Q**) |
| Determinism: rerun, location, order | fixture tests (`the_catalogs_are_the_same_across_runs_order_and_location`, `a_corpus_run_does_not_depend_on_its_location`) | — | at pilot scale, R1 and R2 are byte-identical (§1) |
| Coverage per declared family × module or document | `coverage:complete`, `coverage:family-has-scope` | fail | all pass. 9,358 coverage rows |

**Absence and uncertainty.** These are kept distinct and none is inferred:
- unresolved remainders on `resolutions` and `argument_resolutions`;
- provider reasons on derived rows;
- `boundaries`: 353 `types` boundaries, every one "Pyrefly records no type" (**Q**);
- `partial` coverage;
- `not_found` modules (51);
- 753 `unresolved_target` names, all in materialized doc blocks (C5 O2);
- `graph_gaps`, which is empty.

No catch-all reason survives. Each `typed:*` rule reads `reason IS NULL`, and no derivation
supplies a constant reason. I read all 17 derivations.

**Equivalence.** Node and edge ids are byte-equal under rerun and relocation (measured). The
evidence of a merged edge is whichever run's fact sorts first. That choice can flip when a run
id changes, for example when the corpus selection changes. The `edge_id` does not change
(DESIGN §3.2 declares this).

---

## 4. Derivation and execution

| Stage (R1, this host: 32 threads, 188 GB) | Wall | Peak RSS so far (VmHWM) |
|---|---|---|
| Stage A | 0.24 s | — |
| Library run: check 2.08, per-module 5.00, dependency check 3.75, definitions 0.66, batches 0.20 | 11.7 s | 2,321 MiB |
| Corpus run: check 3.36, per-module 7.66, dependency check 3.69, definitions 0.77, documents 0.20, batches 0.56 | 16.2 s | 3,727 MiB |
| Raw writes (36 tables) | 0.7 s | 3,727 MiB |
| Derive (21 tables; `edges` 1.04, `nodes` 0.52, `site_targets` 0.25) | 2.4 s | 5,363 MiB |
| Validate (492 queries) | 10.0 s | 7,303 MiB |
| Publish | 0.04 s | — |
| Total | 44.2 s | 7,303 MiB |

**Relationship structures.** The 35 published edge kinds keep ownership, containment, lexical
resolution, dispatch, typing and documentation apart (DM-34). Direction texts are published in
`edge_kinds`. The `exports` class is discussed at O1.

**Boundary contracts.**
- **Schema.** Every write goes through `write::<T>` (`attempt.rs:108-121`), which rejects a
  foreign schema and a foreign `snapshot_id`.
- **Derivation.** A derived table is cast strictly back to its declared schema (`derive.rs`).
- **Reads.** Every read is pinned and filtered (`snapshot.rs:47-63`).

**Coherent publication.** One `snapshots` append follows validation (`attempt.rs:217-261`). The
raw batches stay borrowed for the whole attempt (`main.rs:259-260`). After the raw writes, only
the run ids are read from them (`attempt.rs:242-247`); see F3.

---

## 5. Representative journeys

### 5.1 Do the named consumers get what they read?

Built from each consumer's DESIGN text (§5, §9, §10) and matched to a column or edge in the
store.

| Consumer and read | Source in the CPG | Verdict |
|---|---|---|
| Pass A seeds (§9.1): access path → declaration | `exports.declaration_node_id`; `exports` edges | sourced |
| Pass A / §5 invocation projection: call site, resolution, phase, remainder, parallel sites | `call_target` (discriminator `payload_id`); evidence `pysa_calls` and `facts`; `resolutions` keyed by call site; `encloses_call` | sourced. The exclusion of `potential` and synthetic arcs holds by kind (see §3), but it is not a published contract |
| Pass A subsystem scope | `nodes.module_node_id` → `source_files.module_name` | sourced |
| Pass B guards and handlers (§9.2) | `syntax_nodes` (field, ordinal, parent, detail); `ast_child` | sourced. C2 O2 (caught types in field `test`) is deferred |
| Pass B binding order (§4.2.4) | `bindings` (ordinal, span, static branch); `binds`, `shadows` | sourced |
| Pass B forwarding: argument → value → name → binding → parameter | `argument_value`, then a **column** join `references.name_node_id`, then `reads_binding` and `introduces` | sourced, with one hop off the catalog (O2) |
| Pass B `transformed_argument` (literals, defaults) | `syntax_nodes.detail`; field `default` | sourced. C2 O4 was answered by placing every expression |
| Pass B controls from record fields | `record_fields`, `has_field`, `field_type` | sourced. Pydantic constraints are C4 O3, deferred |
| Pass C handoffs (§9.3): usage calls into the release | 18,529 usage `call_target` edges and 35 `higher_order_target` edges on release nodes (DESIGN's 18,564 is their sum; **Q**); bindings; syntax | sourced |
| Pass C type compatibility | `has_type` and `type_observations.declared`; terms shared by both runs | sourced (1,853 shared terms; no split, per the §3 table) |
| **Pass C / §10.5: is a usage module an example, a test or a doc block?** | doc blocks: `code_blocks.module_path`. Examples and tests: only the path prefix. The selecting globs live in `libraries/fastmcp/pyproject.toml`, not in the snapshot | **no typed source** (F2) |
| **§10.3 usage pattern, §10.4 "every snippet parses", §10.5 trimming** | text exists for doc blocks (`code_blocks.code`, `tables.rs:948`), passages (`tables.rs:921`) and docstrings. For the 125 examples and 427 tests (5.55 MB) there is only `content_digest` and `byte_len` (`tables.rs:145-166`) | **no source** (F2) |
| FCA (§9.6): parameter names, parameter and return types, raised types, decorators | `parameter_syntax`; `has_type` by role; `raised` observations; field `decorator` | sourced. Re-raises are C4 O4, deferred |
| §9.4 co-use and co-mention; §9.5 usage counts | usage `call_target` with `encloses_call`; `mentions` | sourced. Prose precision is C5 O1, deferred |
| §10.3 Outcome; §10.2 `parameter`, `analysis_boundary`; §10.4 symbol grounding | docstring text, `passages.text`, `exact` mentions; `parameters`; `boundaries`/`coverage`; `exports` | sourced |

### 5.2 Meaningful change: upgrading the library, or adding a second one

1. **Every id is renamed.** The upgrade moves the library `release_id`. Every library node
   moves. Every corpus node moves too, because the corpus release id hashes the library's (C5
   F1(b)).
2. **Nothing is reused**, so nothing can be stale (no cache exists yet).
3. **A flat-layout library fails the compile, named** (C5 F3). A src or packaged layout
   works.
4. **A second library is supported by construction, not by rule.** The "one run per release"
   and "which release is the library" facts live in `run()` and in
   `runs.families ∋ 'exports'` (`derived.rs:1057`); see O3.

### 5.3 Boundary: two runs, two contexts, one attempt

`merge` (`lib.rs:287-322`) concatenates both runs' tables. It writes a producer once, and a
context and its distributions once when the two runs share them. On the pilot the contexts
differ, so both are written, with the paths relative to `$release` and `$venv`.

Relocating the environment (a fresh `uv sync` into a different directory) and the tree (a copy)
changes nothing that is published (R2). This is the §4.0 location claim, now shown for the
two-run attempt at pilot scale.

### 5.4 Interruption and failure

- **Pyrefly panics abort the attempt.** This path was not attacked; its test passed.
- **A validation failure publishes nothing.** The rows stay invisible (tested).
- **The corpus run writes into the fetched tree before extraction** (`library.rs:595-611`).
  A failed compile therefore leaves a rewritten `_lctx_blocks/` behind. That is deterministic
  and harmless, but see O4 for the ordering of `select` and the clear.

---

## 6. Acceptance gates

| Gate | Result | Evidence or scope rationale | Required action |
|---|---|---|---|
| **G1** Authority | **pass** | One writable authority per fact. The catalogs, references, rules and `edge_kinds` all come from one registry. Merged records are derivations. Duplicates across runs agree on the pilot (§3 table). No second copy of a family's facts was found | none |
| **G2** Semantic fidelity | **pass** | Modality and origin separate cleanly by edge kind (§3). Candidate sets are kept, remainders are explicit, and there are no catch-all reasons. The `exports` class wording is O1 | none |
| **G3** Validity | **pass** | Nothing reaches `snapshots` unvalidated (`attempt.rs:217-240`). Every invariant in §3 has a rule that fails closed, or holds by construction. The untested rules are a claim problem (G7, F1), not an open path | none |
| **G4** Hidden behaviour | **pass** | Inspection is read-only (`sql.rs`, `snapshot.rs`). Git and uv run hermetically. Residual, carried from C5 O4: the tree's working copy is not verified, and the analyzer reads one unselected tree module that no identity covers (O4) | keep O4 |
| **G5** Consistency and recovery | **pass** | One append after validation. Publication is refused twice for one `snapshot_id`. An ambiguous append is re-read. All tested | none |
| **G6** Transformation and reuse | **pass** | Rerun and relocation identity measured at pilot scale (R1, R2). `first_fact` is documented and sound on the pilot. The one deferred ordinal mapping is verified (§7.3) | none |
| **G7** Truthful capability claims | **fail (narrow)** | F1: the validation-strength claims. F2: consumer claims for the usage run. F3: the C6 memory attribution. All are claim-level; the built graph is not wrong | narrow the claims; F1's tests; F2's decision |

---

## 7. Findings

### 7.1 Findings

**F1. Validation's falsifiability is overstated, and ADR-0014's revisit trigger fired without a
record.** (DM-59, DM-60, DM-54 · G7)

*Evidence.* The claims:
- DESIGN §8 L1522: "A rule that could only pass (its target built from its own source column) is
  not generated". §3.8 L799-800 and ADR-0014 L140-141 say the same.
- DESIGN §8 L1477-1478: "Tested … each rule kind rejects an injected violation".
- ADR-0014 `revisit:` names "a lineage rule can pass vacuously" as a trigger.
- ADDENDUM §5 L129 names "the `pysa_calls` partition rules" as an oracle.

What the code does:
- **17 of the 32 `lineage:*` rules check their edge's own unfiltered source.** They are
  `declares`, `has_parameter`, `encloses_call`, `has_argument` and `ast_child`
  (`graph.rs:255-270, 349-364, 412-427, 437-452, 602-617`); the eight `simple()` kinds
  (`graph.rs:1098-1124`); `reads_binding` and `captures` (`graph.rs:1143-1161`); and
  `declared_in` and `type_arg` (pick = 1 plus pick > 1, `graph.rs:572-590, 917-935`).
- **`partition:pysa_calls-gaps` checks a table whose only branch ends `AND FALSE`**
  (`graph.rs:1358`).
- **C3 review O2 decided to keep these as edit guards.** Its Disposition says "§3.7 and §3.8 say
  so"; neither does.
- **14 of the 39 individually written rules have no injected case.** The individually written
  kinds are `semantic`, `partition`, `typed`, `id`, `unique`, `placed`, `contained`, `coverage`
  and `support`. One is the vacuous `partition:pysa_calls-gaps` above. The other 13:
  - `partition:pysa_calls-remainders`, so the `partition` kind has no exercised member;
  - `typed:exports`, `typed:ancestry_targets` and `typed:override_targets`;
  - 5 of the 6 `semantic:*` rules (only `resolution-has-boundary` has a case, `compile.rs:429`);
  - `id:arguments`, `id:context_definitions` and `id:context_modules` (known-answer vectors
    only);
  - `placed:call_syntax` (C2 F3 added only `placed:declarations`).

  The injected-case lists were checked in `compile.rs`, `graph.rs:438-523` and
  `syntax.rs:667-776`. Separately, §3.8 L790 lists "a run's release has modules or documents"
  (`ref:runs.release_id->source_files|documents`) among the C5 rules that each reject, and no
  case exercises it.
- **The rule count is 492, not 494.** DESIGN §4.3 L1270 and L1277 say 494;
  `contracts__rules_snapshot.snap`, which passed today, has 492.

*Consequence.*
- **Labels would be raised on inflated evidence.** A reader counts 492 falsifiable guards;
  18 cannot fail on today's SQL.
- **The one rule that accounts for every unresolved Pysa call record has never been shown to
  fail.** That is `partition:pysa_calls-remainders`. Suppose the span or ordinal join in
  `resolutions`/`argument_resolutions` regresses so an unresolved row lands nowhere. Then the
  snapshot publishes if that untested SQL is also wrong.
- **An ADR revisit trigger fired without anyone recording it.** That trigger is the mechanism
  ADR-0001 relies on.

*Correction.*
1. Say it honestly in §8 and §3.8. List the edit-guard rules by kind, add an ADR-0014 amendment
   recording that the trigger fired and C3 O2's answer, and fix "494" to 492.
2. Add 14 injected cases: the remainders rule (shift an unresolved call row's span),
   `typed:exports`, `typed:ancestry_targets`, `typed:override_targets`, the five semantic
   rules, the three `id:` rules, `placed:call_syntax`, and the `ref:runs.release_id` rule.
3. Name `the_corpus_rules_reject_their_violations` and `each_graph_rule_rejects_a_doctored_catalog`
   in ADDENDUM §5.

*Verification.* **Test:** a meta-test over `rules()`. It asserts two things. Every
individually written rule is named by an injected case or listed in a declared edit-guard set.
Every generated template is exercised at least once: `key`, `ref`, `fact`, `codebook`,
`endpoint`, `evidence`, `one-per-evidence`, `no-parallel` and `lineage`. With the vacuous gaps rule
declared a guard, it fails today on the 13 names above.

---

**F2. Two reads the usage run's named consumers need have no source in the snapshot: snippet
text, and a module's usage role.** (DM-23, DM-46, DM-10, DM-59 · G7 by claim; Unresolved by
decision)

*Evidence.*
- **The CPG names the usage run's consumers.** DESIGN §3.2's docs row (L456) lists "Pass C
  examples and tests, §10.5 usage patterns" as its consumers.
- **§10 depends on snippets.** §10.3 L1773 fills Usage pattern from an "official example, with
  setup preserved". §10.4 L1793 requires that "every snippet parses". §10.5 L1807-1811 trims
  test-only details.
- **The store has code text only for doc blocks.** `code_blocks.code` (`tables.rs:948`) holds
  it; so do passages and docstrings.
- **For the 552 example and test modules there is no text.** `source_files` holds only
  `content_digest` and `byte_len` (`tables.rs:145-166`); on the pilot that is 5.55 MB of source
  (**Q**).
- **Nothing says whether a module is an example or a test.** The distinction is recoverable
  only from the path prefix. The globs that selected each file are in
  `libraries/fastmcp/pyproject.toml` and in no table.
- **Re-reading the tree would break the rebuild promise.** §4.3 L1240 and §B6 promise derived
  rows rebuildable from Delta, and §6.4 L1435 promises byte-identical bundle rebuilds.

*Consequence.* The first usage pattern forces one of three choices:
- **(a)** Stage F reads `build/sources/<name>/<commit>`: an input the snapshot does not pin
  beyond a digest, over the unverified working tree (O4).
- **(b)** Stage F reconstructs code from `syntax_nodes`. That is not byte-faithful, so "every
  snippet parses" becomes a test of the reconstruction.
- **(c)** A schema migration at §10.5.

Separately, "a test-only detail" and "an official example" cannot be told apart except by a path
convention. That matters for §10.2's `documented` status ("doc or example") and for §10.5's
trimming rule.

*Correction.* Decide before §10.5. It is cheap now: 5.6 MB of example and test text per
snapshot (8.7 MB with the library's own modules).
1. **Text:** either store the text of usage modules (a column on the corpus run's
   `source_files`, or a small `usage_sources` table), or declare a content-verified re-read.
   Stage F would read `<tree>/<path>`, check `source_files.content_digest`, and fail on a
   mismatch; §6.4 and DM-23's wording would be amended to match.
2. **Role:** record the usage role (`example`, `test`, `doc_block`) from the glob that selected
   each file. That is an appended codebook.
3. **Until then,** narrow §3.2's consumer list to "structure; text owed to §10.5".

*Verification.* **Test:** a Stage-F test that builds a pattern from Delta alone, with the tree
absent. A `docs_shapes` assertion that each usage module has its role. Neither exists; none can
until the decision is made.

---

**F3. The C6 memory measurement misattributes the peak.** It is dominated by glibc arena
retention, not by the working set of derivation and validation, and it varies by ±0.6 GB between
identical runs. (DM-39, DM-59 · G7; measured cost)

*Evidence.* DESIGN §4.3 L1280-1284 says: "3.7 GB after extraction …, 5.4 GB after derivation,
and 7.4 GB after validation (`unique:type_terms` raised it most, by 0.5 GB)"; and "Derivation …
adds 1.7 GB".

| Run (identical inputs) | Peak | Notes |
|---|---|---|
| R1 | 7,303 MiB | sampled VmRSS climbs steadily across validation, from about 3.9 to 6.75 GB |
| R2 | 7,942 MiB | |
| Author's saved pre-review runs, one content digest `1b97ecc6…` (read from the session scratchpad: `pilot-c5b4.txt`, `pilot-c5b5.txt`, `pilot-c6.txt`) | 6,678, 7,675 and 8,044 MiB | |
| R3 (`MALLOC_ARENA_MAX=2`) | **4,321 MiB** | 3,705 MiB after extraction; validation adds nothing ("raised the peak most: +0 MiB"); but validation 17.05 s, `derive nodes` 6.60 s, total 60.9 s |

The raw batches also stay resident through derivation and validation (`main.rs:259-260`;
`attempt.rs:242-247` needs only their run ids). Their size was not measured.

The reported VmHWM also fell 6 MiB between two R3 stages (4,327 → 4,321 MiB). Linux updates
the high-water mark lazily, so "VmHWM, which only grows" (§4.3 L1261-1262) holds only
approximately.

*Consequence.*
- **The triggers read a noisy, allocator-driven number.** Two deferred items have memory
  triggers: streaming derive ("dominates … peak RSS", L1291) and validation over cached tables
  ("the peak nears the host's memory", L1292-1294). Both are judged on a number that is about
  40–45% allocator retention and moves by ±0.6 GB between runs.
- **The named remedies push the wrong way.** In-memory hot tables and concurrent rules would
  raise the peak.
- **The levers that do move it are unrecorded.** Those are the allocator or arena policy, and
  dropping the raw batches after the write.
- **A larger second library would be judged on the wrong mechanism.**

The C6 decision itself still holds, more strongly: under the arena limit, derivation's
working set is about 0.6 GB (3,705 → 4,327 MiB).

*Correction.*
1. Restate the C6 memory lines with their conditions (default glibc, 32 threads), the observed
   range (6.7–8.0 GB), and the arena-limited figure.
2. Point the memory triggers at a measure that separates working set from retention, for
   example the peak under a fixed arena setting. Or decide an allocator, which is a dependency
   decision.
3. Release the raw batches after `write_raw`, keeping the run ids.

*Verification.* **None mechanical.** `just pilot` under a recorded allocator setting is the
measurement, and DESIGN must state that setting beside the number.

### 7.2 Observations

None of these moves the three severity classes today. Each names where it would land.

| # | Observation | Evidence | Oracle |
|---|---|---|---|
| O1 | `exports` edges are published as `analyzer` ("a provider's own resolution"). But 334 of 1,312 target a module-scope binding chosen by our recognizer's last event by ordinal. 8 of 962 declaration targets are one of several same-named `def`s, picked by the seed rank; 1 binding target has several events. The class is per kind, so it cannot mark per-row policy choices | **Q**; `derived.rs:217-239`; `graph.rs:381` | none until a consumer selects by class; then split out `dst_kind = binding` or add a per-edge flag |
| O2 | No reference node has an in-edge (124,597 of 124,597), and 756 have no edge at all: 753 unresolved names and 3 builtin variables. Reference → name node and reference → scope are columns, by C2 O6's decision. New whole-graph evidence, not re-raised | **Q** | when Pass B/C's projection needs syntax → lexical traversal: a `reference_of` edge generated from `node_columns` |
| O3 | The library/corpus relation between releases is not stored. `import_targets` finds "the library" as `runs.families ∋ 'exports'` (`derived.rs:1057`). `releases.library` is documented as "null for a source tree" (`tables.rs:109`), yet the corpus row has it. "One run per release" holds by construction in `run()`; no rule checks it | **Q** `releases` | a `releases.corpus_of` column plus a `ref:` rule and a one-line `unique:runs-per-release` rule, when a second library or corpus appears |
| O4 | New evidence for C5 O4 (the tree as a scratch area), not re-raised. (a) The analyzer reads tree files the selection leaves out: 1 on the pilot, `scripts/auto_close_needs_mre.py` (`context_modules` origin `search_path`). Only the commit label covers its bytes, and the working copy is unverified. (b) `corpus()` selects usage files **before** clearing `_lctx_blocks/` (`library.rs:594-598`), so a usage glob that matches `_lctx_blocks/` would select stale blocks. FastMCP's globs do not | **Q**; `library.rs` | **test** when O4 reopens: clear before `select`, plus `git status --porcelain` clean except `_lctx_blocks/` |
| O5 | A reader cannot see its snapshot's `content_digest` or `compiler_digest`. `published()` does not register `snapshots`; this review had to use `--unpublished` | `snapshot.rs:137-148` | register `snapshots`, filtered to the snapshot, in the reader session |
| O6 | `documents.release_id` has no reference rule, while `source_files.release_id` has one (`rules.rs:54`). A document with a wrong release id escapes `coverage:complete` | `rules.rs:41-84` | a `REFERENCES` entry plus a case in `the_corpus_rules_reject_their_violations` |
| O7 | Two `row_number()`s break §8's "unique tie-break" rule when both runs describe a symbol: `reads_builtin` (`graph.rs:756`) and `exports.ext` (`derived.rs:204-205`) order by `d.kind, d.key`. The tied rows name one symbol, so output is deterministic by coincidence | `graph.rs:754-759` | edit: append `d.fact_id` |

### 7.3 Deferred rows of earlier reviews, re-checked

| Row | Trigger | Today | Consequence |
|---|---|---|---|
| ADR-0014: `higher_order_index` as the argument ordinal | a starred or keyword-first higher-order call | **fired.** 229 of 1,786 higher-order argument sites follow a keyword or starred argument, and 693 targets sit on keyword arguments (**Q**) | **Reasoning verified.** Pysa numbers them with `iter_source_order().enumerate()` (`call_graph.rs:2946-2955` at `6a93da3`), the same enumeration `arguments.ordinal` uses. **Close the row** |
| C4 O1: self-chosen `types` boundary reasons | any `types` boundary on a pilot | **fired** (353) | All 353 are Pyrefly's "records no type" and none is a lookup-miss kind. Keep the row, with only its first trigger (functional `NamedTuple`/`TypedDict` records) |
| C2 O3: "innermost" chained comparison | a chained site with more than one candidate | not fired: 83 chained sites, 0 ambiguous (recomputed) | keep |
| C3 O5: PEP 695 scopes | a library allowing 3.12 syntax | not fired: 0 in the corpus (grep, and `syntax_nodes` kind 6) | keep |
| C5 O3: `first_fact` over same-run duplicates | a same-run duplicate on a pilot | not fired: 0 disagreeing rows (§3 table) | keep |
| C5 O4: the tree as a scratch area | shared or concurrent compiles, or a `git status` check | not fired; new evidence in O4 | keep |
| ADR-0014 O5–O7 and retention parsing; C2 O2, O5; C3 O1, O3, O4, O6, O7; C4 O2–O4, O6; C5 O1, O2, O5 | consumer, pin or patch triggers | not fired: no consumer has landed; pins and patch unchanged since C4 | keep |

### 7.4 Applicability and principle verdicts

Every group bore on the scope. Group 4 bore only through the registry (DM-16, DM-17, DM-20);
provider selection (DM-19) is Pyrefly only. DM-45 (trust) is satisfied here: corpus code is
written to files and analyzed, never executed, and git runs hermetically.

| Principle | Verdict | Where |
|---|---|---|
| DM-02 one authority | Satisfied | the registry; derived-only catalogs |
| DM-07 validity rules at boundaries | Satisfied | generated rules before the append |
| DM-08 absence distinct | Satisfied | §3 absence note |
| DM-11/DM-15 stable identity, canonicalization | Satisfied | R1, R2 |
| DM-12 entity, revision, execution | Satisfied | `node_id`, `run_id`, `snapshot_id` |
| DM-14 consistent publication | Satisfied | one append |
| DM-23 derived traceable and rebuildable | Unresolved for §10.5 text | F2 |
| DM-28/DM-31 ambient inputs, dependencies | Satisfied, with the O4 residual | C5 O4 |
| DM-34 relationship kinds distinct | Satisfied | 35 kinds with direction texts; O1 |
| DM-39 end-to-end performance | Violated (attribution) | F3 |
| DM-40 determinism | Satisfied | R1, R2 |
| DM-42/DM-43 loss-aware, capabilities | Satisfied | boundaries, coverage |
| DM-46/DM-48 lineage, reproducibility | Satisfied | evidence facts; content digest reproduced |
| DM-53 invariants verified | Satisfied | rules plus injected cases |
| DM-54 negative tests | Violated in part | F1 (13 rules) |
| DM-58 proportionate machinery | Satisfied | §8 |
| DM-59 falsifiable, labelled claims | Violated | F1, F2, F3 |
| DM-60 review controls | Violated in part | F1: an ADR trigger fired unrecorded; a disposition's prose fix did not land |

---

## 8. Alternatives and architectural leverage

| Alternative | Duplication and locality | Risks | Cost | Performance evidence | Verdict |
|---|---|---|---|---|---|
| Baseline (ADR-0008): typed families, no catalogs | every consumer re-derives the union; endpoint and lineage rules hand-written per table | isolates and endpoint kinds implicit | none added | — | rejected by ADR-0014; still right |
| **As built:** typed families plus materialized catalogs from one registry; two runs per attempt | one declaration per kind; about 240 of 492 rules generated from it | merged kinds rest on `unique:type_terms` plus an unchecked agreement for `context_*` (C5 O3) | `nodes` 0.5 s and `edges` 1.0 s; 2.35M rows per snapshot | measured (§4) | **selected** |
| **Simpler:** catalogs as unmaterialized views over the same registry SQL, and the corpus as its own snapshot | the same registry; no merged kinds, no `first_fact`, no `unique:release-paths` | a separate corpus snapshot breaks §10.4's "cites only this snapshot" and Pass C's typed targets. A view re-runs the union in each of the ~240 rules that read `nodes`/`edges` | saves 1.5 s and the stored rows | ~240 recomputations of a 1.0–1.5 s union would cost far more than the 1.5 s saved | **rejected**: materializing pays for itself inside validation alone |

**Justified abstractions.**
- **The registry** removes a real duplication: C1's `REFERENCES` and endpoint lists had already
  drifted (ADR-0014 review F4).
- **The UDF** gives one id recipe in Rust and SQL.
- **`first_fact`** is 7 lines, and has three users.

**What stays ordinary code.** The recognizers, the Pysa mappers and the types pass. Nothing
there is over-generalized.

---

## 9. Verification and measurement: labels the evidence now supports

The author asked which labels can be raised. "Verified" below means checked in this session, on
2026-09-23.

| DESIGN claim | Label now | Supported label | How verified |
|---|---|---|---|
| §3.4.1 `edge_id` row, "(C1, Proposed)" | Proposed | **Implemented; Tested** (`the_catalogs_hold_every_graph_shape`, `the_catalogs_are_the_same_across_runs_order_and_location`) | tests passed in `just test-all`; at pilot scale, R1 and R2 give byte-identical `edges` |
| §3.4.1 `content_digest` row ("Slice 2 has the first two") | Proposed | **Implemented**; equal across a rerun and a relocation at pilot scale | R1 and R2 match `ab6d98a3`: `c6ed7392…` |
| §4.0 "where a checkout, tempdir or environment sits never changes an identity", for the **two-run** attempt | Tested (fixtures; one pilot environment path, slice 3) | **Tested at pilot scale for both runs** (review observation, not a repo test) | R2: a freshly synced environment in another directory plus a copied tree; identical `contexts`, `nodes` and `edges` |
| §3.4.1 "an identical rerun re-emits the same `node_id` and `fact_id`" | Proposed | **Tested at pilot scale** | R1: `nodes` including `existence_fact_id` byte-identical |
| §B2 Arrow schemas are the contract | Proposed | **Implemented; Tested** (`contracts_snapshot`, `registry_snapshot`, `batches_type_check_and_sort_canonically_regardless_of_input_order`). The serving and embedding clauses stay Proposed | `just test-all` |
| §B3 DataFusion constructs and validates; shared validators | Proposed | **Implemented; Tested**, once F1's list narrows "each rule kind" | `validate.rs` is used by `attempt.rs:217` and the tests |
| §B6 facts with provenance; disagreement kept | Proposed | **Implemented; Tested** (`fact:`/`fact-payload:` cases; `provider_disagreement` in the derived snapshots) | code and tests |
| §B7 Delta store, published by one append | Interface-checked | **Implemented; Tested** (`an_attempt_publishes_every_table_and_readers_see_only_published_rows`, `reads_pin_the_version_and_filter_the_snapshot`, `a_failed_snapshots_append_is_classified_by_rereading`, `retention_keeps_old_versions_loadable`) | passed |
| §3.3 physical profiles | section default Proposed | **Tested** (`every_table_round_trips_through_delta_exactly`) | passed |
| §3.5 codebooks append-only | Proposed | **Implemented; Tested** (`registry_snapshot`, `codes_are_dense_from_zero_and_names_unique`, the `codebook:boundaries.reason` case) | passed |
| §3.6 resolution sets | Proposed | **Implemented; Tested** by the derived-table snapshots and `modality_follows_the_variant_table` | passed |
| §8 "endpoint kinds … are Proposed" | Proposed (understated) | **Implemented; Tested** (`endpoint:call_target`, `endpoint:declares` cases) | passed |
| §4.3 C6 time, 44.6 s | Measured | **Measured**, reproduced: 44.2 s and 44.5 s | R1, R2 |
| §1.4 "275 modules, 103 distributions" (2026-09-22) | Tested | re-verified 2026-09-23 | R1 output |
| **Overstated:** §8 "each rule kind rejects an injected violation"; §8/§3.8 "a rule that could only pass is not generated"; §3.8 C5 "each C5 rule rejects"; "494 rules" | Tested | narrow per F1 | F1 |
| **Overstated:** §4.3 C6 memory attribution | Measured | Measured **with conditions**, re-attributed | F3 |
| **Overstated:** §3.2 docs row consumer "§10.5 usage patterns" for examples and tests | Implemented and Tested | narrow until F2 is decided | F2 |
| §5 container and determinism rules | Interface-checked | **keep.** The readiness test uses `DiGraph<String, ()>`, not §5's container | `graph.rs:279-330` |

**Cost accounting.**
- Extraction dominates: 28 of 44 s, two Pyrefly checks per run.
- Validation is 10 s for 492 queries; its cost is the query count.
- Derivation is 2.4 s.
- Memory: see F3.
- Storage: the catalogs add 2.35M rows per snapshot.
- Every deferred item's trigger is unmet, and F3's correction only strengthens that.

---

## 10. Exceptions and unresolved decisions

- **Unresolved: snippet text and usage role (F2).**
  - Principles: DM-23, DM-46, DM-10.
  - Scope: §10.3–§10.5 over the usage run.
  - Options: stored text, or a content-verified re-read; the role as an appended codebook.
  - Owner: the operator.
  - Revisit trigger: the §10.5 slice, or the first Pass C finding that must cite code text.
    Until then, §3.2 claims structure only.
- **Accepted as is:**
  - C3 O2's edit-guard lineages. A legitimate SHOULD-level choice (DM-58), once it is stated
    (F1).
  - O7's tie-breaks. Deterministic today.

---

## 11. Decision and implementation changes

**Decision: Accept, with the supported claims narrowed.**

*Reason.*
- **The built CPG holds up.** Adversarial whole-graph checks found no authority, fidelity,
  validity or consistency defect:
  - a rerun and a relocation are byte-identical at pilot scale, across both runs;
  - merged kinds agree;
  - modality and origin separate by kind;
  - every consumer read except two has a source;
  - no catch-all reason survives.
- **G7 fails on claims.** Validation strength (F1), usage-run consumers (F2) and memory
  attribution (F3).
- **The corrections are prose, 14 injected cases and one decision.** F1's prose and tests
  should land before the author raises §8's and §B3's labels. F2's narrowing lands with the
  label changes; its decision is owed before §10.5, not before slice 4.

| Priority | Change | Principle IDs | Acceptance evidence | Regression protection |
|---|---|---|---|---|
| 1 | F1: state the edit-guard rules in §8/§3.8; amend ADR-0014 with the fired trigger and its answer; add injected cases for `partition:pysa_calls-remainders`, `typed:exports`, `typed:ancestry_targets`, `typed:override_targets`, five `semantic:*`, three `id:*`, `placed:call_syntax` and `ref:runs.release_id`; fix 494 → 492; update ADDENDUM §5's test names | DM-59, DM-60, DM-54 | the new cases fail on their mutation and pass on the fixtures | **test:** the meta-test that every rule is exercised or declared a guard |
| 2 | F2: narrow the §3.2 consumer claim now; decide text and role before §10.5 | DM-23, DM-10, DM-59 | the decision recorded in DESIGN (and an ADR if §6.4 changes) | **test:** a Stage-F rebuild from Delta alone |
| 3 | F3: restate the C6 memory block with conditions, range and the arena-limited figure; retarget the memory triggers; drop raw batches after the write | DM-39, DM-59 | `just pilot` output with a recorded allocator setting | none mechanical |
| 4 | Labels per §9; close the ADR-0014 `higher_order_index` row; narrow C4 O1's trigger | DM-59 | this review's §9 and §7.3 | prose |
| 5 | O3, O5, O6, O7 as their triggers arrive; O6 and O7 are one-line edits worth folding into priority 1 | DM-09, DM-07, DM-40 | — | the tests named in §7.2 |

### Deferred

| Item | Why not now | Trigger that reopens it |
|---|---|---|
| O1: `exports` class hides per-row policy choices | 9 of 1,312 edges involve a choice; no consumer selects by class | a projection that selects edges by `derivation_class` |
| O2: no reference → name-node or reference → scope edge | C2 O6's decision; Pass B can join the column | Pass B/C's projection spec needs syntax → lexical traversal inside a graph |
| O3: library/corpus link and one run per release | one library, one corpus, constructed by `run()` | a second library or a second corpus in one attempt |
| O4: see C5 O4 | carried | C5 O4's trigger |
| O5: digests invisible to readers | no reader needs them yet | the serving bundle's manifest (§6.4) is built |

**Final check.**
- **Claims.** The design's claims match the evidence once F1–F3's narrowings land.
- **Scope.** The scope matches the implemented guarantees for everything but §10's reads of
  the usage run, which F2 makes explicit.
- **Extension path.** Clear and mostly generated: a new family is a registry entry plus its
  derivation and tests. The rule meta-test would keep "each rule rejects" true as it grows.

## Disposition (author, 2026-09-23)

The author accepted the decision and applied priorities 1, 3 and 4, and O6 and O7 from 5, in one
commit after `c07d848`. Priority 2's decision stays with the operator.

| # | Outcome | What changed | Oracle |
|---|---|---|---|
| F1 | fixed | `cpg_schema::rules::EDIT_GUARDS` declares the 18 guards (17 lineage rules, each checked to re-read its edge's own unfiltered source, and `partition:pysa_calls-gaps`); DESIGN §8 and §3.8 state them and count them apart (493 rules, 475 falsifiable); ADR-0014 amended (the fired trigger, C3 O2's answer). 14 new cases: 13 doctored views in `each_graph_rule_rejects_a_doctored_catalog` (the remainders partition, three `typed:*`, four `semantic:*`, three `id:*`, `placed:call_syntax`, the runs-release reference), each now running its own rule's query (the test takes 15 s, not 69 s), and `semantic:boundary-has-resolution` as a raw mutation in `compile.rs` (graph_shapes has no call boundary). ADDENDUM §5 names the four case tests. The count was 492 before O6; the C5 Disposition's "494" is corrected (the C5 commit message keeps it) | **test:** `every_rule_is_exercised_or_declared_an_edit_guard`, which a renamed case makes fail (`no injected case: ["typed:exports"]`, checked) |
| F2 | narrowed; decision open | §3.2 `docs` consumers now claim the usage run's structure only; the text-and-role choice is a §13 row owned by the operator, with the options and trigger from §10 | none until decided (a Stage-F rebuild from Delta alone) |
| F3 | fixed | §4.3 restates the peak with its conditions (default glibc, 32 threads), the 6.7–8.0 GB range over seven runs, and the arena-limited figure; the `VmHWM` claim is qualified; the memory triggers read the peak under `MALLOC_ARENA_MAX=2`, and a new deferred row names the levers that lower it. `attempt::compile_owned` releases the raw batches once written; `lctx compile` uses it | measured, no mechanical oracle: 7,096 MiB default (inside the range), 4,219 MiB under `MALLOC_ARENA_MAX=2` (R3: 4,321) |
| Labels | raised | per §9, each citing its tests: §B2, §B3, §B6, §B7, §3.3, §3.4.1 (`edge_id`, `content_digest`, rerun ids), §3.5, §3.6, §4.0 (pilot scale, as a review observation), §8 (endpoint kinds), §1.4 (re-verified) | the tests named; R1, R2 |
| §7.3 | recorded | the ADR-0014 `higher_order_index` row is closed (ADR-0014 amendment); C4 O1 keeps only its first trigger (functional `NamedTuple`/`TypedDict` records) | — |
| O6 | fixed | `ref:documents.release_id->runs` | test: its case in `the_corpus_rules_reject_their_violations` |
| O7 | fixed | `d.fact_id` appended to both `row_number()` orders | derivations snapshot |
| O1–O5 | deferred | as the review's table | — |

**Checks** (2026-09-23): `just test-all` passed (nextest 83/83; pytest 21/21; rule tests 4/4;
`lint-agents`; `adr lint`; fixtures; family; cargo-deny; `pyrefly-fork`; gold). `just pilot`
passed on a fresh store: snapshot `ddee0669…`, content `66cddc06…` (the compiler digest moved
with O6 and O7), 905,648 nodes, 1,449,162 edges, all 493 rules, 44.6 s at 7,096 MiB peak; the
same compile under `MALLOC_ARENA_MAX=2` into a scratch store (deleted): the same content digest,
61.2 s at 4,219 MiB.

**F2, decided** (operator, 2026-09-23): store the text in Delta. ADR-0015 stores every analyzed
module's text and role in `source_files`. `a_corpus_documents_its_library` now slices each
example and test call from Delta with the fetched tree and the environment deleted, the Stage-F
oracle this review asked for, in the only form possible before Stage F exists.
