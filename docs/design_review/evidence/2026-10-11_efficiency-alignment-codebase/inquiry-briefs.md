# Inquiry briefs

The briefs given to the supporting inquiries of the
[codebase efficiency-alignment review](../../reviews/design_review_efficiency-alignment-codebase_2026-10-11.md).
They are recorded so later readers, including the separate audit, can see what each inquiry was
asked and what it was not asked.

Every inquiry received the same thing:
- a short launch prompt pointing at this file;
- the [operation map](operation-map.md);
- its role contract, `.agents/roles/worker.md` plus `design-reviewer.md` or `library-research.md`.

Nine inquiries (A–H) are `design-reviewer` agents and two (L1, L2) are `library-research` agents.
The coordinator approved the plan for these briefs with the operator on 2026-10-11.

## Common brief (given to every inquiry)

### Purpose and form

This is an independent, codebase-wide **DESIGN / TARGET** design review. Its purpose is to
identify every material opportunity to align library-context's physical realization with
FP-07/A4 of the design principles, using *Heuristics for Efficient Architecture* (H1–H28).

You are producing a **bounded supporting assessment** for one inquiry:
- A separate, fresh principal reviewer owns the combined judgment, the finding IDs and the
  verdicts. Use local candidate labels such as `A-01`.
- The coordinator reconciles the inquiries with each other.
- Your report is published verbatim as supporting evidence.

The operator selected a **full re-assessment**. No area is exempt because an earlier review
covered it. Earlier findings, and dispositions marked "Implemented", "focused Tested", "closed"
or "source corrected", are leads and contrary evidence, never conclusions. For each earlier lead
in your area, state one of these positions:
- *confirmed still present*;
- *resolved* (the work was actually removed);
- *residual* (partly removed);
- *moved* (the work now happens elsewhere);
- *refuted* (the diagnosis was wrong);
- *not examined*.

### Standard: read these, they own the criteria

- `docs/design_review/design_principles/standard.toml`: core 3.3, heuristics 1.0,
  code-intelligence profile 1.5, binding.
- `docs/design_review/design_principles/core/design-principles.md`. Especially:
  - §1;
  - FP-07 and the other foundations;
  - DP-08–DP-23;
  - §A gates;
  - §C;
  - §D evidence vocabulary;
  - §E change locality, execution fit and A1–A4;
  - §F;
  - §G false positives.
- `docs/design_review/design_principles/core/efficient-architecture-heuristics.md`: H1–H28 and
  its *Interpretation safeguards*.
- `docs/design_review/design_principles/core/design-review-template.md`: the finding standard
  and priority rules.
- `docs/design_review/design_principles/profiles/code-intelligence/principles.md` and
  `review.md`: CI-01–CI-13, the journeys, and the calibration row "Bounded answer, amplified work".
- `docs/design_review/design_principles/binding/library-context.md`: recurring question Q17, the
  §3 scenario seeds, and the §6 defect shapes.
- The skills `.claude/skills/design-review/` and `.claude/skills/design-review-code-intelligence/`.

### Grounding (important)

Judge only against the standard and the **functional target**, which is a version-pinned API and
evidence catalog for feature discovery and correct use. A coding agent can find a built-in library
capability, select its exact public API and configuration, implement or deploy it, and inspect the
evidence and limitations behind the choice. Exhaustive results name their supported domain and
coverage, unknowns stay local and explicit, and nothing generative runs in compilation or the
query path. The sources are:
- DESIGN §1.1;
- `docs/design/sections/api-and-evidence-product.md` §14, especially §14.10 (rebuild boundaries)
  and §14.12 (the usable pilot, whose reports include p50/p95 latency and compile/index cost).

Several things are the *current design*, not criteria:
- ADRs;
- DESIGN binding decisions §B1–§B14, including the choice of SurrealDB in §B12/§B13;
- plans and their disposition tables;
- AGENTS.md and CLAUDE.md policies;
- binding policies;
- `docs/pins.md`;
- operator memory.

Read them to understand the subject, but do not fit a recommendation to them. If a recommendation
would change one, name it as a candidate rule impact. Give the rule, where it is recorded, the
proposed change, and what the recommendation becomes if the rule is kept.

### Workload premises (qualitative; no SLA, no cost model)

| ID | Scenario |
|---|---|
| S01 | Cold `lctx compile fastmcp` (the 4.0.5 pilot) under the catalog profile, through publication and sealing |
| S02 | Growth: a second library of a different shape with 5–20× the first-party code and heavy dependency context; many libraries and releases retained in one store (`main` and `validation` databases); more semantic kinds |
| S03 | Warm recompile: unchanged inputs; a patch-release upgrade with small source changes; a changed analysis, embedding or rendering policy (the §14.10 table); a changed model declaration |
| S04 | A behavioral profile requested over an existing catalog compile |
| S05 | Publishing and selecting a new realization while agents serve the pinned one, then retiring the old one |
| S06 | Several coding agents concurrently issuing search, lookup, evidence and continuation calls; high-degree nodes; absence answers over incomplete coverage; MCP server cold start |
| S07 | Interruption: a cancelled compile, a crash during publication, a service restart, and recovery |
| S08 | Retained history grows: store content, attempts, control history, run directories, evidence captures, storage-lifecycle records |
| S09 | Extension: adding a fact family, analytic or model; upgrading an analyzer (mechanism substitution) |
| S10 | Development loop: changing one crate and running its focused controls; cold and warm `just verify --select`; concurrent fixture-backed tests on the shared stable SurrealDB service; `just test-all` |

**Host.** 32 logical CPUs and about 188 GiB, shared with other repositories' workloads. SurrealDB
3.3 runs as a remote server with a RocksDB store, as a systemd user unit, reached over gRPC
through the patched SDK in `third_party/surrealdb`. vLLM serves Qwen3-Embedding-8B over HTTP.

### How to assess

- **Assess the complete physical operation.** Look at:
  - necessary versus repeated work;
  - access paths;
  - crossings: Python↔Rust (PyO3), gRPC round trips, Arrow↔`surrealdb::types::Value`, threads and
    runtimes, files, subprocesses, HTTP;
  - live working set;
  - whether reuse is valid;
  - resource and transaction lifetimes;
  - failure and recovery behavior, under growth, skew and concurrency where these matter.
- **Ask questions earlier reviews never asked:**
  - memory high-water and peak live state of a full compile and of serving;
  - process start-up (the `lctx` binary, `domain::model()` construction, MCP server cold start);
  - log and tracing volume;
  - SurrealDB service cold start.
- **What a useful candidate finding names:**
  - the avoidable work;
  - why it arises (the design cause);
  - the consequence under a named scenario (S01–S10), with the H, FP, DP or CI mechanism;
  - a simpler conforming alternative, including any library-owned option;
  - preservation constraints, meaning the guarantees that must survive;
  - material uncertainty.

  Static evidence can establish structural amplification. A quantitative speed or capacity claim
  needs measurement, so do not claim speedups.
- **Preserve what the system promises.** Keep correctness and fidelity, CI-04 unknown-is-not-
  absent, complete answers (never a truncated answer presented as complete), attribution and
  provenance, independent semantic controls, coherent publication and pinned readers. Removing a
  check is not a remedy unless its guarantee is preserved elsewhere or provably unnecessary.
- **Record strengths too.** Note strengths and preservation constraints worth keeping, and
  anything you checked and found adequate. An adequate area is a valid result; do not
  manufacture findings.
- **Cite precisely.** Cite evidence as `path::symbol` (file path plus function or type name),
  never line numbers alone.
- **Label claims** with the §D vocabulary: *Proposed*, *Interface-checked*, *Implemented*,
  *Tested*, *Measured*. Attribute any historical receipt you rely on, such as a coordinator §9.1
  receipt or an existing run summary, with its date and scope. Never present one as a fresh run.

### Calibration safeguards

The independent audit of a comparable review in a sibling repository corrected these specific
mistakes. Avoid them:

1. **Settle library semantics at the pinned or patched version.** Use the library skill, the
   pinned source under `~/.cargo/registry/src/`, or `third_party/`. Documentation alone is a lead.
   An earlier review there misread SurrealDB's `FOR UPDATE` conflict behavior.
2. **State search coverage for every absence claim.** Cover tests, generated SurrealQL and
   definitions, Python, scripts and internal callers. A missing facade or entry point is not a
   missing capability.
3. **A digest, stat tuple, name or ID match is not semantic equivalence.** Carrying validity
   forward needs an owner of the unchanged premise (H19/H20).
4. **Incomplete, bounded or truncated discovery is never absence** (CI-04/CI-08). A remedy cannot
   turn bounded discovery into "none".
5. **A copied snapshot and a live pinned handle have different retirement semantics.** Retire or
   collect only with replay or late-effect authority, never by age or deadline alone.
6. **Laziness must beat eager amortization.** A demand-shaped remedy has to show that the later
   upgrade does not cost more where every caller needs the rich result (H6 safeguard).
7. **Keep checks that independently challenge production semantics.** Repeating the producer's
   own logic is not independent assurance, but a genuinely independent readback or oracle may be
   worth its cost.
8. **One structural cause per candidate.** Do not bundle mechanisms that have different closure
   obligations, and do not split one cause into several candidates.
9. **Count the exact critical section before claiming serialization.** A lock or semaphore does
   not mean the whole operation is serialized.
10. **Priority follows consequence.** Label unmeasured magnitudes, and never let arithmetic stand
    in for a measurement.
11. **Report adequate areas.**
12. **Challenge each proposed remedy with a legitimate counterexample.** Could it reject valid
    work, erase a distinction, weaken a guarantee or move policy to the wrong owner?

### Permitted effects

**Read-only.** Do not edit tracked files.

Allowed:
- `rg`, `ast-grep`;
- `git log/show/diff/grep`;
- offline `cargo metadata --offline` and `cargo tree --offline`;
- the LSP;
- reading files under `~/.cargo/registry/src/`, `third_party/`, `~/.local/share/library-skills/`
  and `build/runs/*/summary.json`, and other existing records, read-only.

Not allowed:
- any `just` recipe (they enroll runs and storage records);
- builds or tests;
- starting services or querying SurrealDB;
- syncing environments;
- `uv run` with sync.

If a **light probe** would settle a doubt that could change a verdict, *propose* it in your
report: the question, the exact command, and the outcome that would discriminate. A light probe
needs no build, service or fixture. The coordinator decides whether to run it.

### Library evidence route

1. The library skills: repository links under `.claude/skills/` and the shared store under
   `~/.local/share/library-skills/skills/`. Relevant ones are `datafusion`, `datafusion-tracing`,
   `neo4j-surrealdb`, `pyo3`, `serde-arrow`, `salsa`, `python-analyzers`, `rust-graphs`,
   `rust-reasoning`, `fixedbitset`, `fastmcp` and `ast-grep-ripgrep`.
2. Context7.
3. Pinned or patched source.

Check that a version applies against `docs/pins.md`, `Cargo.lock`, `uv.lock` and `third_party/`.
An empty search result is not evidence of absence.

### Baseline

`main` at `88f33bc0dd8bc67ef9291115c3a487b5f54bcb57`, 2026-10-11. Production source is clean at
that commit. Another session has uncommitted edits in these files only:
- `STATUS.md`;
- `crates/lctx-publisher/tests/publication.rs`;
- `crates/lctx-publisher/tests/stream_search.rs`;
- `docs/plans/persisted-graph-execution-plan_2026-10-07.md`.

Read HEAD content for those files (`git show HEAD:<path>`). Other sessions may commit during the
review. If one changes a file you rely on, cite the baseline and note the change.

### Architecture entry points

- `docs/design/README.md` (the architecture map), `docs/design/DESIGN.md` and the owning sections
  in `docs/design/sections/`:
  - `semantic-model.md` §15;
  - `storage-and-publication.md`;
  - `synthesis-and-serving.md`;
  - `acquisition-and-extraction.md`;
  - `analytics.md`;
  - `behavioral-analysis.md`;
  - `api-and-evidence-product.md`.
- `docs/surrealdb.md` and `docs/workspace-storage.md`.
- The [operation map](operation-map.md) prepared for this review. It is facts only and may
  contain errors, so verify anything decisive yourself.

### Lead sources (earlier reviews and dispositions)

All reviews are in `docs/design_review/reviews/`. The single current disposition owner for most
of them is `docs/plans/persisted-graph-execution-plan_2026-10-07.md` **§8 "Sole finding
disposition"**. Its §7 holds the execution packages, and its §9.1 holds dated receipts.

Package prefixes:

| Prefix | Plan |
|---|---|
| PC | persisted-execution corrections |
| PJ | populated-journey |
| GK | graph-compilation kernels and hashing |
| GR | graph reuse and invalidation |
| NE | native-execution efficiency |
| UP | unified persistence |
| HS | holistic state management |
| SA | SurrealDB architecture and capability leverage |
| CU | coroutine containment |
| BC | build and compilation cost |
| SM | storage management |

These plans keep their own tables:
- `rust-compilation-costs-plan_2026-10-08.md` §9;
- `storage-lifecycle-management-plan_2026-10-09.md` §9;
- `graph-native-pivot-plan_2026-10-05.md` §7 and §9;
- `agent-test-evidence-and-causal-analysis-plan_2026-10-11.md` §8.

Note that these dispositions are claims by the implementing plans. Almost none of them is
measured, and most sit at "Implemented / focused Tested, enclosing acceptance open".

### Report format

Return the report as your **final message**. The coordinator publishes it. Do not write files in
the repository.

1. Scope actually examined, and coverage limits (what you did not read).
2. Complete operations examined, as short stage → owner → crossing → live-state notes.
3. Candidate findings (`X-nn`). Each one gives:
   - a title;
   - the observed cause, with `path::symbol` evidence;
   - the consequence under a named scenario, with the H, FP, DP or CI mechanism involved;
   - a simpler conforming alternative, including any library-owned option;
   - preservation constraints;
   - uncertainty;
   - an evidence label;
   - suggested closure evidence.
4. Opportunities and uncertainties that are not findings, and why not.
5. Strengths and preservation constraints you observed, and areas you checked and found adequate.
6. Positions on earlier leads in your area, as a table: lead ID → position → evidence.
7. Candidate rule impacts, if any.
8. Proposed light probes, if any, each with a command and a discriminating outcome.

Be thorough but concise. Aim for roughly 2,500–5,000 words; precision matters more than volume.

## Inquiry-specific briefs

Sibling ownership:

| Inquiry | Owns |
|---|---|
| A | Acquisition, providers and producer streaming |
| B | Compiler read path and catalog-profile stages |
| C | Analyses, reuse/invalidation and embedding realization |
| D | Native persistence, publication and state lifecycle |
| E | Serving, retrieval and the agent interface |
| F1 | Rust build and test composition |
| F2 | Verification, evidence and environment tooling |
| G | Cross-cutting assurance, identity and observation |
| H | Cross-cutting capacity, concurrency and live working set |
| L1 | Data, persistence and boundary libraries |
| L2 | Analysis and compute libraries |

Follow dependencies into a sibling's area when your question needs it, but leave that sibling's
own questions to them, and flag any seams you see.

### A — Acquisition, providers and producer streaming

**Scope:**
- `crates/cpg-extract` (acquisition, capture, `bundle`, `contracts::composition`, `pyrefly_stage`,
  `ty_flow`, `ruff_context`, `diagnostic_records`, `syntax_records`/`type_records`/`call_records`,
  `document_parser`, `deployment`);
- `crates/cpg-flow`;
- `crates/lctx/src/acquisition.rs` (`Lease`) and `scripts/storage_acquisition.py`;
- the producer side in `crates/cpg-core`: `facts::compile_facts`, `workspace::ProducerOutput`
  (`write`, `push_sync`, `complete`), `native_bridge`, `native_calls`;
- in `crates/lctx-surrealdb`: `NativeCompilerStore::write_batch`,
  `complete_contribution_scoped`, `codec::batch_bodies`;
- `lctx_model::domain::record::{Record, Batch}`.

**Revealing questions:**
- Do the providers run sequentially where no data dependency requires it (H22)?
- How many times is each analysed module read, parsed and indexed across providers (latest Ruff,
  Pyrefly's embedded Ruff, ty's own parse)? Which re-parses are semantic necessities under §B1,
  and which come only from the physical arrangement (H2, H12)?
- `Batch` holds both a `Vec<R>` and an Arrow `RecordBatch`. What is live at once, and does the
  combined live set grow with module or library size (H15–H17)?
- Producer → bridge (channel capacity 2) → `native_calls` `Semaphore(8)` → Arrow→`Value` codec →
  gRPC write:
  - What is the physical unit of a write (row, batch, family)?
  - Does serialization repeat across layers (H8, H16)?
  - Is backpressure coordinated (H21)?
- Acquisition:
  - What do the `uv`/`git` subprocess protocol, the frozen-capture copy into a tempdir and the
    per-file blake3/sha256 cost on an unchanged acquisition (S03)?
  - What is re-established that an earlier acquisition already proved (H12, H19)?
- Completion and contribution admission: per-family round trips, and what grows with the number
  of families or semantic kinds (S02).

**Leads:**
- `design_review_catalog-compilation-speed_2026-10-07.md` F05 (one native crossing per
  occurrence);
- `design_review_native-execution-efficiency_2026-10-09.md` F01–F05;
- `design_review_persisted-execution-correction-causes_2026-10-07.md` F04;
- `design_review_unified-persistent-surrealdb_2026-10-09.md` F04 (reuse is value replay).

**Library skills:** `python-analyzers` (pyrefly, ruff, ty, salsa) and `serde-arrow`.

### B — Compiler read path and catalog-profile stages

**Scope:**
- `crates/cpg-core`:
  - `workspace.rs` (`Workspace`, `CompletedInputs`, `stage_inputs_selected`, `freeze_inputs_async`);
  - `consumed_rows.rs`;
  - `normalize/*`;
  - the `*_scope.rs` and `scoped_*` modules;
  - `base_scope.rs`;
  - `catalog_core`, `catalog_evidence`, `catalog_selection`;
  - `final_coverage`;
  - `sql.rs` (`query`, which has about 116 call sites);
  - `compilation` (`UpperStage`, its schedule, and `compile_driver`);
  - `artifact::admit`;
- `crates/lctx-surrealdb/src/compiler_provider.rs` (`NativeTable`) and `projected_arrow`;
- in `crates/lctx-model`: the scope programs (`compiler_scope_program`, `catalog_scope_program`,
  `normalized/normalization_scope_program`, `scope_program`), the normalized kernels, `catalog/`,
  `selection/`, `admission::Frontier` and `stages`.

**Revealing questions:**
- The read loop runs scan → SurrealQL stream over gRPC → `projected_arrow` → DataFusion → `R::decode`
  → kernel → write-back.
  - Which filters, projections and limits actually reach SurrealDB, and which are applied after
    transfer and decode (H7, H8)?
  - Is the DataFusion layer paying its way (planning, pushdown, joins), or is it a container
    around bespoke iteration (H4)?
- Per stage, what is constructed again: the session, provider registration, freeze, inventories,
  scope programs (H12)?
- Does each of the 9 normalization stages and the catalog-profile upper stages automatically
  carry its own physical unit of execution, persistence and transaction (H2)? Where would grouping
  or fusing help, and where does isolation have a real purpose?
- Is selective demand (a selected scope, a limited relation) served from the smallest sufficient
  input, or by preparing the whole universe (H5, H6, H10)?
- What hydration happens before the kernel needs it (H16)?
- How does the work grow with S02 (library size, retained releases, more kinds)?
- Under S03, what does a changed model declaration force?

**Leads:**
- `catalog-compilation-speed_2026-10-07` F01–F04 and the plan's D01–D04;
- `persisted-execution-correction-causes_2026-10-07` F03–F06;
- `populated-journey-execution-amplification_2026-10-08` F01–F02;
- `graph-compilation-kernels-and-hashing_2026-10-09` F01–F02;
- `assembled-graph-scope-compilation_2026-10-09` (VS-F01, C-F01–F05, GK-F01–F03);
- `persisted-execution-corrections-integrated_2026-10-08` F01–F04;
- `native-execution-efficiency_2026-10-09` F06–F08.

**Library skills:** `datafusion` and `datafusion-tracing`.

### C — Analyses, reuse/invalidation and embedding realization

**Scope:**
- `crates/cpg-core`:
  - `semantic_execution`, `semantic_models`, `semantic_summaries`, `local_semantics`;
  - `structural`, `analytic`;
  - `analysis_graphs` (`PreparedGraphs::load`), `analysis_prepare`;
  - `synthesis`, `retrieval`;
  - `analytic_text`, `analytic_embedding`, `embedding_realization`, `retrieval_tokenizer`;
  - `compilation/reuse` (`DependencyGraph`), `UpperStage::reuse`;
  - `workspace_products` (`reuse_product*`), `normalize::{reuse, kernel_products}`;
- in `crates/lctx-surrealdb`: `product_cache.rs` (`NativeProductCache`, file leases) and `cache.rs`
  (`NativeEmbeddingCache`);
- in `crates/lctx-model`: `execution/`, `conditions/` (BDD kernel), `analytics/` (Leiden, FCA,
  ranking, neighbours), `synthesis/`, `retrieval/`, `compilation_product`, `embedding/`;
- `crates/lctx-embed`.

**Revealing questions:**
- Does warm recompile work follow the change (S03; H13)? Consider:
  - unchanged inputs;
  - a patch upgrade;
  - a changed analysis, embedding or rendering policy, against the §14.10 invalidation table;
  - a changed model declaration.
- What does a cache hit still cost: key construction, proving the inputs unchanged, readback,
  validation (H14)?
- Does proving inputs unchanged repeat most of the work it avoids?
- Are the product cache and embedding cache distinct scopes with distinct benefits?
- Are the algorithms and access paths for SCC summaries and BDD conditions, Leiden/FCA, ranking,
  neighbours and petgraph projections suited to S02 scale and skew (H9, H10)?
- Are projections hydrated once per selected root or per stage?
- Do unrequested analyses still prepare inputs (H6)? Consider the catalog versus the behavioral
  profile, and S04.
- Embedding realization: batching to vLLM, tokenization reuse, cache lookup granularity, and the
  live set of vectors.

**Leads:**
- `compiled-product-reuse-contracts_2026-10-09` F01–F04;
- `graph-compilation-and-reuse-plan_2026-10-09` F01 and the GR plan
  (`docs/plans/graph-compilation-reuse-and-invalidation-plan_2026-10-09.md`);
- `holistic-state-management_2026-10-10` HS-F07;
- `graph-native-compiler-stage_2026-10-05` F01–F02;
- `catalog-compilation-speed_2026-10-07` F04 (unrequested A1 analytics prepare inputs).

**Library skills:** `rust-graphs`, `rust-reasoning`, `fixedbitset` and `salsa`.

### D — Native persistence, publication and state lifecycle

**Scope:**
- `crates/lctx-surrealdb`:
  - `compiler.rs` (`NativeCompilerStore`: attempts, completion, scans, import and export of
    state);
  - `control/*` (history, retirement, cleanup, authorization);
  - `upgrade/*`;
  - `loader`, `reconciliation`;
  - `derived_search` (materialization and `reconcile_search`), `lexical_stats`, `materialization`;
  - `selection`, `definitions`;
  - `config::RuntimeConfig` (selection lock, serving JSON);
- `crates/lctx-publisher` (`publish`, `seal_completed`, `native_publication::Publication`,
  `backup`, `restore`, `inspection`);
- `crates/lctx/src/newnative*`;
- the server and service integration in `third_party/surrealdb-server` and `docs/surrealdb.md`,
  for orientation only.

**Revealing questions:**
- **Publication critical path.** How many sequential round trips are there? Consider the
  128-row reference and transfer windows, `materialize_search`, `seal_staging`, readback. Which
  are required by the guarantee, and which only by the mechanism (H1, H8, H23)?
- **Rework on unchanged content.** Does publishing a new realization over mostly unchanged
  content re-materialize, re-lower or re-inventory it (S03, S05)?
- **Reconciliation.** `reconcile_search` (an independent second lowering), `Loader::reconcile`
  and the seal readback: which give independent assurance worth their cost, and which repeat
  unchanged conclusions (H20)?
- **Backup and restore.** These go through SQL dumps (`restore.rs`). Compare the native
  export/import capability and its contract gaps (H4, H16).
- **Selection.** The lock is polled every 10 ms; reader pins are durable.
- **Retention, retirement and control history.** Does ordinary work scan growing history (S08;
  H25, H27)? How do transaction and lock scopes compare with the effects they protect (H23)?
- **Recovery after interruption (S07).** What unit is replayed (H24)?

**Leads:**
- `surrealdb-architecture-and-capability-leverage_2026-10-10` SA-F01–F07;
- `surrealdb-capability-integration_2026-10-10` and its follow-up, F01;
- `holistic-state-management_2026-10-10` HS-F01–F06 and F08–F10;
- `holistic-state-management-integrated_2026-10-10` F01–F03;
- `unified-persistent-surrealdb_2026-10-09` F01–F08 and its plan's TF01;
- `assembled-populated-execution-correction_2026-10-09` F01–F06;
- `surrealdb-graph-store_2026-10-05` F01–F14. These are from the PostgreSQL era and have no
  owner; most may be obsolete, so say which survive.

**Library skill:** `neo4j-surrealdb` (the SurrealDB 3.3 part).

### E — Serving, retrieval and the agent interface

**Scope:**
- `crates/lctx-serving`:
  - `service` (`NativeService`), `operations` (`dispatch`, `browse`, `search_*`);
  - `search` (`lexical`, `vector`, `rescore_union`, `vector_selection_sql`);
  - `scores`;
  - `preparation` (`PreparedCache`, moka), `ranked_results`;
  - `capability`, `evidence`, `inspection`, `library`, `selection`, `candidates`,
    `operation_packet`;
  - `delivery`, `pagination`;
- `python/lctx_semantics` (`session::NativeSession`, `wire_decode`, `wire_tool_result`,
  `admit_envelope`);
- `python/lctx_mcp` (`__main__.create_server`, `native.NativeExecutor`, `embedder.HttpEmbedder`,
  `wire`);
- in `crates/lctx-surrealdb`: `reader::NativeReader`, `batches::CanonicalBatches`,
  `derived_search::lexical_occurrences`;
- `lctx_model::domain::serving` (ranking, dispatch);
- consumers: `crates/lctx-eval` and `scripts/programmatic_eval.py`.

**Revealing questions:**
- **Fixed cost per call.** Consider decode, admission, budget, the one-time definition check,
  vector-spec and projection validation through `reader.records`, `CanonicalBatches` conversion
  to Arrow for the kernels, and JSON encoding. Which of these re-establish properties that are
  fixed for the pinned realization (H12, H19)?
- **Vector search.** `search::vector` computes `vector::similarity::cosine` with
  `ORDER BY … LIMIT`, and no `<|k|>` KNN operator appears in `src/`, even though an HNSW index is
  defined.
  - Settle whether exactness, filtering or determinism semantics require brute force at the pinned
    or patched SurrealDB version before calling this a defect.
  - Assess how the work grows with corpus size and retained releases (S02, S06; H4, H9, H10).
- **Query embedding.** Each search call makes an HTTP embedding request; the 4096→1024 prefix
  projection follows.
- **Admission and capacity.** Python `asyncio.Semaphore(2)` plus a 30 s deadline, the Rust
  `query_connections`/`cpu_jobs` semaphores and `ResourceBudget`. Are these coordinated, or nested
  independently (H21)?
- **Sparse reads.** Do they hydrate whole selected payloads (H5, H16)?
- **High-degree nodes and absence answers over incomplete coverage** (H10, H11; CI-04, CI-08).
- **Continuations.** Do they avoid new discovery, and how do they live alongside selection
  changes (S05)?
- **MCP cold start.** What does startup cost: the reader pin, definition checks, cache warm-up?
- **`lctx-eval`.** Its independent decoder is a consumer; note whether it duplicates the decoder
  or deliberately provides independence.

**Leads:**
- `holistic-state-management-integrated_2026-10-10` F01 and F03;
- `surrealdb-architecture-and-capability-leverage_2026-10-10` SA-F04–F06;
- `surrealdb-capabilities_2026-10-05` F01–F03;
- the serving-related NE-I rows in coordinator §8.

**Library skills:** `fastmcp`, `pyo3` and `neo4j-surrealdb`.

### F1 — Rust build and test composition

**Scope:**
- `Cargo.toml`: workspace dependencies, features, `[profile.*]` (release is incremental,
  `package."*"` is O3) and `[patch.crates-io]`;
- `.cargo/config.toml`: sccache, the clang/mold linker, the shared build-dir
  `~/.cargo/build/library-context` (about 108 GB), unstable feature unification and fine-grain
  locking;
- `rust-toolchain.toml`;
- `crates/lctx-workspace-hack` and `.config/hakari.toml` (only `lctx` depends on the hack);
- `.config/nextest.toml`;
- the build scripts:
  - `crates/lctx-model/build.rs` and `crates/lctx-serving/build.rs` concatenate sources for
    `include_bytes!` implementation digests;
  - `crates/cpg-core/build.rs` and `crates/cpg-extract/build.rs` use
    `scripts/producer_fingerprint.rs`;
- test-target structure:
  - 146 integration test files, 86 of them in lctx-model;
  - 58 test files that include shared modules via `#[path]`;
  - `cpg-core/tests/fixtures/native.rs`, compiled into about 20 binaries;
  - cpg-extract dev-dependencies that pull in every heavy stack;
- `crates/lctx-analytics`, a facade with no workspace consumer;
- `crates/lctx-model-macros`.

**Revealing questions:**
- Does changing one crate compile and link only its actual dependency closure (S10; H5, H13)?
- Which test targets link SurrealDB, DataFusion, Pyrefly/Ruff/ty or PyO3 without needing them?
- Do the build-script implementation digests cause recompiles or relinks broader than a semantic
  change? Is the digest a true input to some consumer, and which one?
- How do sccache, incremental compilation, the shared build-dir and the release default for
  verification interact? Settle what sccache actually caches for incremental workspace crates.
- What is the current state of the coroutine and type-size costs (the CU work), and of
  feature-unification breadth?
- Is the test count or structure (86 lctx-model integration files, each a separate binary)
  proportionate to what it isolates?

**Leads:**
- `rust-compilation-costs_2026-10-08` F01–F04 and RC01–RC02;
- `rust-coroutine-compilation-amplification_2026-10-08` F01;
- `docs/plans/rust-compilation-costs-plan_2026-10-08.md` §9;
- `docs/plans/rust-coroutine-compilation-amplification-plan_2026-10-08.md`;
- `docs/design_review/evidence/2026-09-24_rust-build-performance/` and `2026-09-28_cargo-cache/`.

Existing compile-profile captures under `build/` may be read; attribute them.

### F2 — Verification, evidence and environment tooling

**Scope:**
- the `justfile`, including its shell, which runs
  `uv run … python scripts/build_environment.py -- bash` on every recipe line;
- in `scripts/`:
  - `verify.py` (`BOUNDARIES`, `expand`, `resolve`, `_run_boundary`, `execute`, `wrap_in_run`,
    `test-all`/`qualify`);
  - `verify_evidence/`: `capture` (`CaptureOwner`, `SourceObserver`), the nextest and pytest
    adapters, `cli`, `projection`;
  - `test_scope_coverage.py`;
  - `surrealdb_fixture.py`, `surrealdb_service.py`, `surrealdb_server.py`, `storage_service.py`;
  - `runs.py`, `harness.py`;
  - `storage.py`, `storage_lifecycle.py`, `storage_owners.py`, `storage_archive.py`;
  - `workspace_env.py` (`observe`, `sync`), `build_environment.py`;
  - `compile_profile*.py`, `worktree.py`, `freshness.py`, `maintenance.py`;
- `docs/workspace-storage.md`;
- ADR-0134, ADR-0142, ADR-0144 and ADR-0147.

**Revealing questions:**
- **Repeated work per boundary.** A fixture-backed boundary reaches `_installed_binary`, a full
  SHA-256 of the 368.7 MB server executable plus a `surreal version` subprocess, about six times.
  `Installation.check` runs its SQL twice. What property does each check establish, and when can
  its premise change (H18–H20)?
- **Evidence capture** stats and SHA-256s about 2,558 allowed files before and after every step.
  What is this proportionate to (H26)?
- **Storage lifecycle.**
  - `Storage.records()` globs and validates every object at the end of each verify run.
  - Every enrollment takes the global exclusive `metadata.lock` with an fsync.
  - Admission takes one flock per lexical ancestor.

  How does this grow under S08 (H25)? Is the hot path coupled to retained history?
- **Ordering.** Boundaries run sequentially in `test-all`. Is that a real dependency or broad
  ordering (H22)?
- **Process overhead.** The justfile shell and wrappers start Python per recipe line, plus nested
  `uv run`. Readiness runs `uv sync --check` and reads native input fingerprints (H12).
- **Proportionality.** Is about 26.7k lines of Python tooling matched to its consumers? Look for
  duplicated mechanisms (H27).
- **Fixture concurrency on one shared service** (S10).

**Leads:**
- `build-storage_2026-10-09` F01–F03;
- `docs/plans/storage-lifecycle-management-plan_2026-10-09.md` §9;
- `storage-lifecycle-plan_2026-10-09` F01–F02;
- `agent-effectiveness-followup_2026-10-09` F04 (small log reads scale with the whole log);
- `agent-test-evidence-and-causal-analysis_2026-10-11` F03 (reruns cover a whole boundary);
- `agent-test-evidence-integrated_2026-10-11`;
- `agent-workspace-effectiveness_2026-10-07`;
- `agent-effectiveness-integrated_2026-10-09`.

Do not run any `just` recipe. Read-only inspection of `build/runs/*/summary.json`, run
directories and `~/.local/state/library-context*` is allowed.

### G — Cross-cutting assurance, identity and observation

Follow this responsibility across every component: model, extract, core, surrealdb, publisher,
serving, Python and scripts.

**Scope:**
- `lctx_model::domain::identity` (`ContentHash` blake3, `ContentHasher`, `KeySink`) and every
  identity or key construction;
- xxh3 in `domain/scope_program.rs`;
- sha256 in acquisition, deployment, RECORD checks and the embedding spec;
- build-time implementation digests (`build.rs`, `producer_fingerprint.rs`,
  `serving-implementation.bin`, `lctx_serving::operation_definition`);
- receipts (`capture_receipts`, `source_receipts`, receipt claim types);
- every readback, reconciliation or validation:
  - `lctx_surrealdb::{reconciliation, derived_search::reconcile_search, lexical_stats::reconcile}`;
  - `NativeCompilerStore::verify_state`;
  - `cpg_core::artifact::{admit, verify_export}`;
  - `lctx_publisher::inspection::audit`;
  - `lctx_model::domain::validation`;
  - the seal readback;
  - `control::history` hashing;
- serving-time validation of definitions and specs;
- the hashing done by tooling (fixture executable digests, evidence source digests);
- tracing and logging configuration and volume.

**Revealing questions:**
- Where does each check enforce, relative to where uncertainty actually enters or changes (H18)?
  Are internal crossings re-checked although nothing changed?
- Are established properties carried through operations that preserve their premises (through
  types, ownership, pins or immutability), or re-proved (H19)?
- Is any digest used as if it established semantics (the H20 safeguard)?
- Which checks are independent oracles that the code-intelligence profile wants kept (non-circular
  evaluation, independent readback), and which repeat the producer?
- Auxiliary proportionality (H26): whole-payload hashing on hot paths, provenance copies,
  per-primitive events, wide payloads in inner loops.
- What do identity framing and key construction cost compared with recomputation (H14)?

**Leads:**
- `native-execution-efficiency_2026-10-09` F02 (admission preparation repeated per grain);
- `holistic-state-management_2026-10-10` HS-F06 (cold audit repeats capture);
- `populated-journey-execution-plan_2026-10-09` F01;
- `persisted-graph-implementation_2026-10-07` F01;
- `graph-compilation-kernels-and-hashing_2026-10-09`;
- `docs/graph_hashing_reference/`.

### H — Cross-cutting capacity, concurrency and live working set

This covers compile, serving and the shared SurrealDB service together.

**Scope:**
- every runtime and pool:
  - the CLI multi-thread tokio runtime (`lctx::main::run`);
  - the `native_bridge` thread with its current-thread runtime;
  - the per-provider `std::thread`s;
  - Pyrefly's `ThreadPool(AllThreads)`;
  - `spawn_blocking` (`catalog_core`, `catalog_evidence`, `acknowledged_candidates`);
  - the PyO3 session's multi-thread runtime;
  - the `lctx_mcp` asyncio/anyio loop;
  - rayon, if present;
- every semaphore, budget or reservation:
  - `native_calls` `Semaphore(8)`;
  - `NativeService` `query_connections`/`cpu_jobs`;
  - `ResourceBudget`;
  - Python `Semaphore(2)`;
  - compiler reservations;
  - `RequestAdmission`;
- gRPC connection usage;
- product-cache file leases;
- the selection lock;
- cancellation and drain paths;
- polling loops.

**Revealing questions:**
- Do downstream capacities (the SurrealDB server, the gRPC connection, vLLM) constrain upstream
  admission, or does each layer admit independently (H21)?
- Do reservations match actual demand? What is the peak combined live set of a full compile (S01
  and S02) and of concurrent serving (S06): input buffers, `Vec<R>` plus `RecordBatch`,
  completed views, prepared graphs, caches, outgoing results (H17)?
- What happens when a compile and serving contend on one RocksDB-backed SurrealDB server (S05,
  S06)? Consider transaction scopes, contention and retries (H23, H24).
- Is ordering broader than the real dependency anywhere (H22)? Consider barriers and per-stage
  freezes.
- Is cancellation of work with no consumer prompt? What does drain do, and what does polling cost?

**Leads:**
- `assembled-graph-scope-compilation_2026-10-09` GK-F03 (reservations far beyond actual
  allocation) and C-F02;
- `compiled-product-reuse-contracts_2026-10-09` F01–F02;
- `holistic-state-management-integrated_2026-10-10` F03;
- `native-execution-efficiency_2026-10-09` F05 (fixed-interval polling) and F06;
- `populated-journey-execution-amplification_2026-10-08` F03 (read barrier spans the whole
  attempt).

### L1 / L2 — Library utilization and fit (`library-research` role)

These two inquiries give the cross-cutting library view. They cover how the codebase actually
uses each pinned major library, and which library-owned capabilities would remove bespoke work or
are duplicated by wrappers (H4, DP-13/DP-14/DP-16, G8). Area inquiries A–H assess library fit
inside their own operations. L1 and L2 own the per-library inventory and the comparison against
each library's capability surface at the pinned version.

Full library eligibility applies: no library or licence is refused, and integration cost is
assessed on its own merits.

Method for each major library:
1. **Pin and features.** Read them from `Cargo.toml` `[workspace.dependencies]`, member
   manifests, `[patch.crates-io]`, `third_party/`, `pyproject.toml` files, `uv.lock` and
   `docs/pins.md`, including holds.
2. **Integration owners.** Use `cargo tree -i <crate> -e normal --offline` and `rg`. Is the
   integration owned in one place or spread across consumers?
3. **Capabilities used.** Which APIs and features are called, on which operational path (hot or
   cold), and how: bulk or item by item, prepared and reused or rebuilt?
4. **Capability surface.** Use the library skill, Context7, upstream source and release notes at
   or after the pin. Identify capabilities relevant to S01–S10 that would:
   - replace bespoke code;
   - remove repeated work;
   - preserve optimizer or native batch visibility;
   - or that existing wrappers recreate (planning, caching, validation, retries, transactions,
     parallelism).
5. **Version applicability.** Confirm the capability exists at the pinned or patched version. If
   it needs an upgrade, say so; bumps are ordinary agent judgment here.
6. **Report bespoke-versus-library pairs.** Give evidence on both sides, the semantic fit, the
   integration owner, what machinery would be removed or introduced, and the uncertainty. A
   library capability is a candidate, not a finding, unless the bespoke path causes material
   avoidable work or burden.

**L1 permitted writes:** none in the repository. Scratch outside it is allowed if needed, but
return everything in your final message. **L2:** the same.

#### L1 — Data, persistence and boundary libraries

**Libraries:**
- **SurrealDB 3.3.0**: the SDK and server as patched in `third_party/surrealdb` and
  `third_party/surrealdb-server`, the gRPC engine, `surrealdb-syn`/`-sql`, and the RocksDB
  backend;
- **DataFusion 55.1** (`sql` and `parquet` features);
- **Arrow 59**, including IPC;
- **serde_arrow 0.15.1**;
- **PyO3 0.29.2**;
- **moka**;
- **tokio**;
- **blake3**, sha2 and xxhash;
- **tokenizers**;
- **reqwest** and Python httpx;
- **FastMCP 4.0.5**, server side in `python/lctx_mcp`;
- **vLLM**: the embedding service's batching, the MRL dimension options and the tokenize
  endpoint.

**Particular questions:**
- SurrealDB:
  - multi-row `INSERT` and bulk paths versus per-window statements;
  - transaction and conflict semantics on RocksDB, at the exact patched version;
  - the FULLTEXT/BM25 analyzer and the cost of the `@OR@` query;
  - HNSW index plus `<|k|>`/`<|k,ef|>` KNN: whether it is exact or approximate, how it combines
    with `WHERE` filters, and whether brute-force cosine `ORDER BY LIMIT` is needed for exactness;
  - index use and `EXPLAIN`;
  - record links and graph edges versus per-row hydration;
  - `DEFINE FUNCTION` and API costs;
  - native `EXPORT`/`IMPORT` versus the custom SQL-dump backup and restore;
  - streaming of query results over gRPC;
  - `LIVE` queries.
- DataFusion:
  - `TableProvider` filter, projection and limit pushdown (`supports_filters_pushdown`), and
    whether the custom provider reports statistics;
  - plan reuse;
  - streaming versus collect.
- Arrow: IPC options.
- serde_arrow: its costs compared with direct builders.
- PyO3: GIL detach and allocation crossings.
- moka: coalescing via `get_with`.
- blake3: incremental, mmap and rayon hashing.
- tracing: whether the overhead is proportionate.

#### L2 — Analysis and compute libraries

**Libraries:**
- the pyrefly fork, 1.4.0-dev.3;
- the ruff fork at 0.16.10 (crates 0.0.16);
- the ty 0.0.16 crates (fork at the pinned rev);
- salsa 0.28.5;
- petgraph, rustworkx-core, leiden-rs and graphops if present;
- biodivine-lib-bdd and OxiDD, if present;
- fixedbitset;
- conditional alternatives named in §14.11: Salsa for compile reuse, ascent and datafrog.

**Particular questions:**
- Do the analyzers' incremental capabilities (pyrefly `State`/`Transaction` reuse across runs,
  salsa durability, LRU and early cutoff in ty, shared parse or AST between ty and latest Ruff)
  make S03 proportional to change within the §B1 provider-role separation?
- Are their thread pools coordinated?
- Do graph or condition algorithms in bespoke `lctx-model` kernels duplicate library algorithms
  (G8)?
- Are the chosen algorithms and containers suited to S02 scale?

**Library skills:** `python-analyzers` (it has a labelled library-context layer), `salsa`,
`rust-graphs`, `rust-reasoning` and `fixedbitset`.
