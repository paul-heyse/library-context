# Lane A2: what is graph-shaped, and where the time goes

Baseline: `main` at 1158ebe2, read-only. No compiles, tests or pipeline runs. SQL was read-only against `lctx` (lctx_superuser). Investigated 2026-10-05.
Labels: **Observed** means read in source, logs or the DB. **Inferred** means my interpretation.

## 0. Summary

1. **Few operations are pure graph computations, and most of the graph-shaped ones carry domain semantics.** Only 10 source files use petgraph (4,651 lines in total, including one 694-line test). There are 4 named projections (`projection.rs:56`). The graph kernels are:
   - SCC scheduling;
   - bounded delegation and control traversals;
   - import-route enumeration;
   - class-ancestry path expansion;
   - PageRank, Leiden and kNN;
   - the derivation cycle check.

   The rest of the 135k-line domain is mostly relational correspondence, Python-semantics algebra (argument binding, receivers, dispatch premises) and the summary/composition engine. That engine is a worklist over the call graph, but its state is a lattice of BDD-conditioned transfers with Pareto proof costs. A graph database would not express that state natively.
2. **No evidence shows graph computation as a material time consumer.** Graph-kernel timings come only from pre-cutover pilots (2026-09-24 to 2026-09-28), all run with analytics off. In those pilots, projection, Pass B and Pass C together took about 1.8 s of a 143–215 s run.
3. **Post-cutover facts compilation (2026-09-30, FastMCP) takes 474–669 s of wall time.** The split:
   - **Stored-content validation and publication: about 250–300 s.** Inferred as wall time minus instrumented stage time; none of this phase is instrumented.
   - **The Pyrefly provider stage: 156–235 s.** This includes analysis, fact building, COPY of about 7.1M rows, and per-stage completion framing. These are not separated.
   - ty flow: 78 s.
   - deployment: 25–31 s.
   - assemble: 14–18 s.
4. **No timing evidence exists after the facts frontier.** That covers the normalized, analysis and catalog frontiers on the real library. Real-library pilots are `not_run` or were interrupted (STATUS.md:51; phase 3 receipt). The current DB generation `d3a3fa02…` (2026-10-03) is still `staging`. It has stage receipts only through `normalize_*`, and every analysis and catalog table is empty.
5. **On a tiny fixture, a compile through catalog takes about 75–115 s.** The fixture is about 30 lines of Python, and the tests run under parallel nextest load. This is strong evidence of large fixed per-generation costs: schema, stage framing, checkpoint and final validation, and replay. These costs are unrelated to graph size.
6. **The developer loop is slow:**
   - Release test builds took 8–30 min in Phase 4 logs.
   - `test-all` takes about 955 s for 993 tests.
   - Store installation tests take 100–190 s each.
   - Fixture compile tests take 300–690 s each.

## 1. Graph-shaped computation inventory

Classification:
- **(i)** A plain graph query that a graph DB could express natively.
- **(ii)** Graph traversal plus a domain lattice or algebra that would need custom code anyway, in DB functions or the application.
- **(iii)** Not graph-shaped.

| # | Operation | Owner (Observed) | Algorithm / data structures | What it carries beyond reachability | Class |
|---|---|---|---|---|---|
| 1 | Projection construction (4 projections: CallableInvocation, DefinitionContainment, ImportReference, PublicExposure) | `lctx-model/src/domain/projection.rs:56`, `projection/normalization.rs` (1,162 lines, `normalize` :697), `projection/snapshot.rs` (399; `ProgramGraph = Graph<EntityRef, ArcId, Directed, u32>` :21; Postcard codec :20), `projection/native.rs` | Builds a petgraph `Graph` from normalized rows. Typed parallel arcs are preserved (`MultiplicityPolicy::PreserveTypedParallelArcs`). Snapshots are Postcard BYTEA chunks, hydrated per analysis collection (`cpg-core/src/analysis_graphs.rs:88`). | Gap and gap-subject rows. Source coverage and assessment (`ProjectionSourceAssessment`). Per-input/context keys. Budget reservation (`check_capacity`). | (i) for topology. The gap and coverage side is (ii)-light. |
| 2 | SCC schedule, callee-first | `execution/summary_schedule.rs:23` `invocation_sccs`; `canonical_order` :61–108 | `petgraph::algo::kosaraju_scc`, then a hand-rolled Kahn topological sort over the condensation with BTreeSet tie-breaking for a deterministic order. | Deterministic canonical order, budget reservation. The source states it does **not** certify dispatch or coverage completeness (:22). | (i). Native in Neo4j GDS (SCC). SurrealDB 3.3 SCC/topo: not checked; see lane covering SurrealDB. |
| 3 | SCC-local finite summary worklist (behavioral profile only; catalog profile short-circuits to NotRequested at :1600–1606) | `execution/summary_production.rs:1571` `produce` (2,300 lines), `summary_worklist.rs` (330: Pareto `Frontier`, `WorkQueue`, `ProofCost` depth/steps/rank), `summary_path.rs`, `summary_alias.rs`, `summary_exceptions.rs`, `summary_consequences.rs`, `summary_proof.rs`, `summary_replay.rs` | A per-SCC worklist over seeds, with call composition (`compose` :773) and residual emission on refusal. | BDD conditions (`Diagram`), guard rebasing (`conditions/rebase.rs`), stability witnesses, proof costs with Pareto dominance, refusal kinds (SummaryDepthLimit, SummaryProofLimit, IncompleteDomain), origin boundaries, verdicts, deterministic content digests. **Publication replays `produce`** (`summary_replay.rs:120`). | (ii), heavy. |
| 4 | Whole-call transfer composition and condition propagation | `composition.rs` (2,264), `conditions/kernel.rs` (886, biodivine BDD), `conditions/rebase.rs`, `conditions/substitution.rs`, `conditions/stability.rs`, `conditions/entry.rs` (1,225), `place_composition.rs` | Matches caller and callee transfers by call site, maps roots through binder and parameter links, then conjoins conditions with capture-safe substitution. | The condition algebra, stability witnesses, obligations instead of unconditional flow (`composition.rs:1–8`). | (ii). The edge join is graph-like; the payload is a BDD algebra. |
| 5 | Delegation traversal | `analysis/delegation.rs` (526); BFS at :352–470 | A hand-rolled BFS (`VecDeque`) over the union of the invocation and definition petgraph views. Containment arcs are filtered so they never count as calls. Bounds on depth, arcs, vertices and witnesses. Parent map gives the witness path. | Boundary classification (`self.boundary`), unresolved/incomplete/disagreeing event marking, typed stop reason (`Stop::Arcs`/`Vertices`, depth_limited), deterministic step sort (:391), direct-definite-call distinction. | (i) for bounded BFS with shortest witness. The boundary and unresolved semantics are a thin (ii) layer. |
| 6 | Structural controls (Pass B): parameter-identity paths | `structural/controls.rs` (935); traversal :525–680 | A hand-rolled BFS over the state `(callee, formal, suppressed)` along `argument_flows`. Adjacency is a **linear scan per pop**: `out.argument_flows.iter().filter(\|e\| e.caller==… && e.source==…)` (:653–656). Visited set is a ChargedSet. Bounds on depth, arcs and vertices. | A one-bit "suppressed" lattice (may_catch/value_tested), full path steps persisted (ControlPath/ControlStep), unfollowed-argument linkage, raise collection, TraversalStop. | (i). A graph DB can express this with path-property accumulation (any(...)). Raise collection is minor custom work. |
| 7 | Handoffs (Pass C) | `structural/handoffs.rs` (890) | Matches observed producer-result arguments in usage. | Observational, "never type compatibility". | (iii)/(i)-light (a join). |
| 8 | Dispatch / override expansion | `normalized/dispatch.rs` (656); `mro` :307; override walk :534–575 | Reads **captured** MRO linearizations (not computed; C3 comes from the provider). Each lookup **linearly scans** `data.ancestry` and `data.sequence_members` (:322, :348). DFS over trait symbols, capped by MAX_SYMBOL_NESTING. | Qualification and context agreement, a DispatchReason refusal lattice, premise sets per candidate, resource charge. "Complete captured ancestry is never a closed runtime subclass universe" (:2). | (ii). |
| 9 | Catalog access routes (public import/re-export routes) | `catalog/access_routes.rs` (1,444); DFS :530–700 | Hand-rolled DFS that enumerates **all routes** (not just reachability), with a per-route visited set, `max_depth` ≤32 and `max_routes` ≤256. Each hop runs **5 nested linear-scan joins** (bindings → native_bindings → imports → assessments → candidates). | RouteStop kinds (Cycle, Frontier, Declaration, CandidateOnly, OutsideCapturedModule), publicity and resolution status, stub flag, omitted-frontier counts, partial flag. | (ii). Path enumeration is native; the per-hop correspondence checks are domain logic. |
| 10 | Catalog public paths (containment plus class-ancestry expansion) | `catalog/paths.rs` (350); `expand` :52 | Bounded expansion (MAX_DEPTH) of member paths through class ancestry and ancestors. Linear scans over `data.ancestry` (:82, :103). | Resolution status, explicit finite boundaries on repeated classes. | (i)/(ii). A hierarchy traversal plus status. |
| 11 | Selection domain closure | `selection/build.rs:115`, closure :144–170, digest :641–668 | Builds a witness closure set per declaration domain, then emits it as `catalog_selection_domain_closure` rows. | Witness typing, closure digest, replay equality (`matches`). | (ii)/(iii). Mostly relational witness collection. |
| 12 | PageRank | `analytics/ranking.rs` (213) | Hand-rolled bounded weighted power iteration (damping, tolerance, max_iterations, work units). petgraph 0.8.3 (workspace pin, `Cargo.toml:35`) ships `petgraph::algo::page_rank`, per the rust-graphs skill index. | Explicit nominal selection and weight pairs, work bound, exact arc lineage, documented as heuristic. | (i). Native in Neo4j GDS. |
| 13 | Communities | `analytics/communities.rs` (273) | `leiden_rs` (library) with seeded RBER profiles, ARI/NMI stability. | Retained profiles, convergence flags. | (i). Library-backed already. |
| 14 | kNN neighbours | `analytics/neighbours.rs` (489) | Hand-rolled exact best-window cosine, top-k, canonical tie-breaking. | Window semantics, centroid floor, determinism. | Not a graph kernel. It is vector search (a DB vector index is a candidate). |
| 15 | FCA concepts | `analytics/concepts.rs` (247) | Hand-rolled charged NextClosure/Duquenne–Guigues over fixedbitset. | Nominal identities, charge. | (iii) (lattice, not graph). |
| 16 | Derivation proof cycle check | `derivation.rs` (177) | petgraph `DiGraphMap` plus `toposort` over RowRef proof premises. | None beyond acyclicity. | (i). Library-backed. |
| 17 | Dependency closure (validation universes and stage grants) | `dependency_closure.rs` (240); worklist :125 | Pending-stack closure over **schema-level** relation dependencies (hundreds of nodes, not data). | Lower-layer policy. | (i), but trivial in size and not data compute. |
| 18 | Reaching definitions and value flow | Provider-captured from ty (`local_semantics.rs` 1,511: "replay exact native flow"; `flow_inventory.rs` 935) | The fixpoint runs **inside ty's semantic index** (the analyzer), not in lctx. | Exact native correspondence, unsupported paths retained as assessments. | (iii) for lctx. The graph fixpoint is owned by the analyzer. |
| 19 | Normalization (entities, relations, callables, receivers, binding normalization, symbolic fields, overloads, generic specialization) | `normalized/*` (about 15k lines; e.g. `binding_normalization.rs` 1,639, `receiver.rs` 993) | Correspondence joins and Python call-binding algebra. Binder-ancestor walks in `relation_normalization.rs:445, :673` are short parent chains. | Unknown vs. absent, qualifications, ambiguity retention. | (iii). Relational, plus Python semantics. |
| 20 | Execution evaluation and completion (base, enriched, source calls, models, context execution) | `execution/evaluation.rs`, `completion*.rs`, `model_*.rs`, `context_execution.rs`, `read_*.rs` (about 20k lines) | Bounded closed-expression evaluation over captured syntax, plus protocol and model rules. | Five verdicts, conditions, premises. | (iii) for evaluation. Some steps follow call targets, but the core is an interpreter-like rule system. |

Cardinalities:
- The only real-library numbers post-cutover are facts/normalized (DB `lctx_gd3a3fa…`, 2026-10-03):
  - call_resolutions 140,786;
  - call_resolution_members 142,032;
  - entity_refs about 1.04M;
  - occurrences 959,323;
  - binding_events 101,359.
- Projection, summary and catalog tables are empty in that generation, so post-cutover projection vertex and arc counts on FastMCP are **unknown**.
- Pre-cutover CPG (pg15 pilot, 2026-09-28): nodes 905,769, edges 1,449,462, argument_flows 32,075, delegations 1,986, handoffs 1,531, public_paths 4,763.

Bounds and refusals: every traversal above reserves against `ResourceBudget` and emits typed stop or refusal rows instead of silently truncating. This is a pervasive cross-cutting requirement (charged state, `StateCharge`, `ChargedMap`), and a DB-side implementation would need to reproduce it.

**Inferred balance.** Classes (i) and library-backed together: projection topology, SCC, Leiden, PageRank, derivation toposort, Pass B controls, bounded delegation BFS. That totals about 3–4k lines of this crate's code. The (ii) core (summary engine, composition, conditions, dispatch, access routes) is about 12–15k lines and needs custom code anyway. Most domain code is (iii).

## 2. Where the time goes

### 2.1 Real-library facts frontier, post-cutover (FastMCP 4.0.5)

Sources:
- `build/facts-pilots.log` (2026-09-30)
- `build/facts-qualification-pilots.log` (2026-09-30)
- Retired measurement receipt: `git show 9fcc2e6f:docs/design_review/evidence/2026-09-30_facts-qualification/measurement-receipt.json`
- Phase 3 qualification: `docs/design_review/evidence/2026-09-30_phase3-qualification/raw/facts-catalog.{stderr,stdout}` and `facts-catalog-connections.json`

| Run | Wall (s) | acquire | deployment | pyrefly | documents | ty_flow | assemble | Stage sum | Wall − stage sum |
|---|---|---|---|---|---|---|---|---|---|
| catalog (09-30 facts Q) | 474.3 | 1.5 | 27.3 | 178.2 | 2.3 | — | 14.3 | 223.7 | **~251** |
| behavioral (09-30) | 629.0 | 1.4 | 25.0 | 156.4 | 2.4 | 78.0 | 18.5 | 281.7 | **~347** |
| behavioral repeat | 669.5 | 1.5 | 30.6 | 235.4 | 2.4 | 79.4 | 17.9 | 367.3 | **~302** |
| catalog (phase 3 Q, 09-30) | 528.6 | 1.9 | 26.7 | 212.1 | 2.7 | — | 16.6 | 260.0 | **~268** |

Caveat: pairing the measurement-receipt stage times with the pilot-log wall times is assumed, since both are same-day runs. I could not link them by generation ID. The phase 3 row is self-consistent from one stderr file.

Phase 3 run, `/usr/bin/time -v`:
- user 477.8 s, sys 8.9 s, CPU 92%;
- max RSS 4.06 GB, reached in the pyrefly stage (sampled 4.17 GB).

The lctx process itself is CPU-bound, and that time does not include PostgreSQL backend CPU.

Connection samples (2 s interval):
- Store begin took about 0.3 s (created_at 20:55:16.94; acquire ended 20:55:19.11 after 1.9 s).
- From t≈260 s (assemble done) to t≈528 s, the `lctx_migrator` lifecycle connection alternates between `active` (93 samples) and `idle in transaction` (39 samples).

**Inferred:** about 268 s go to `seal` → `validate` → `publish` (`lctx-postgres/src/generations/lifecycle.rs:417, :655, :726`; `receipts.rs:197` `validate_step`, `:228` `validate_scope`). `validate_scope` does three things:
- Frames every relation in scope and digests its content (streams rows from PG into Rust).
- Runs all scoped invariants over physical views.
- For admitting frontiers, runs `AdmissionCheck` over `visit_physical` rows.

Before that, it installs the reference constraints (`ddl.rs:107` phase "validated").

That makes stored-content validation, not computation, the largest single block, at **about 47–55% of facts-only wall time**. The phase is **uninstrumented**: no tracing around seal, validate or publish. `ValidationStats` (row_scans, proof_hits, check_executions; `validation_session.rs:29`) is counted but never logged.

Inside the pyrefly stage, time is not split between in-process Pyrefly/Ruff analysis, fact construction, the Arrow encode plus COPY of about 7.1M rows (`stage_receipts`: pyrefly 7,113,919 rows in the 10-03 generation), and per-stage completion framing (`receipts.rs:31` `complete_stage_step` re-frames and digests each output). The stage timer wraps all of them (`cpg-core/src/stage_runtime.rs:139–146`).

### 2.2 Normalized, analysis and catalog frontiers on the real library

- Phase 3 normalized/catalog compile: interrupted at 18 s, by user direction, after acquire (`…/phase3-qualification/raw/normalized-catalog.stderr`; README:54–60). "graph_hydration_measurement: not_run" (`pilot-receipt.json`).
- STATUS.md:51 lists "Comparable timings / total RSS" as **not_run**, and real-library activation as not_run/stopped.
- DB generation `d3a3fa026a1237a1c4e175b9b1019eeb`:
  - state `staging`, frontier `catalog`, created 2026-10-03;
  - stage receipts only for acquire, deployment, pyrefly, documents, assemble and normalize_*;
  - normalize_entities 2.19M rows, normalize_relations 363k, normalize_callables 298k;
  - `catalog_paths`, `projection_snapshots`, `summary_components`, `analytic_rank_scores`, `binding_projections` and `reaching_definitions` all have count 0;
  - `lctx_model_store.events` and `lctx_ops.events` are empty, and stage tables have no timestamps or durations.

  So it carries no timing and no explanation of why it stopped. I found no log of that run in `build/` or the evidence folders.
- **Therefore there is no evidence for or against graph compute or summary-engine cost at real-library scale post-cutover.**

### 2.3 Pre-cutover pipeline (Delta/DataFusion era, PG experiments), for contrast

Sources: `build/pg15-pilot.log` (2026-09-28), `pg11-pilot.log`, `pg9-live-{cold,warm}.log` (2026-09-27), `pilot-live.log` (2026-09-24). Every run reported `analytics techniques none`.

pg15 run, total 151.8 s:

| Phase | Time (s) |
|---|---|
| extract | 32.3 |
| `behavior: the flow model` | 29.99 |
| write model_frame_exits | 8.95 |
| write source_modeled_identities | 7.18 |
| write source_context_sites | 4.73 |
| derive edges | 1.24 |
| derive nodes | 0.67 |
| `analyze: projection` | 0.29 |
| `analyze: Pass B` | 1.04 |
| `analyze: Pass C` | 0.42 |
| communities, pagerank, kNN | 0.00 (off) |
| **`validate`** | **49.71** |
| publish | 0.31 |

Peak 3.98 GB. The other three runs are similar (143–215 s; validate 49–57 s; flow model 25–35 s).

Graph kernels were about 1–2% of wall time; validation was about 33%. This is a different architecture, so it is not comparable in absolute terms. It is consistent with §2.1 that validation and storage, not graph algorithms, dominate.

### 2.4 Fixture-scale compile through catalog (fixed-cost signal)

Source: `docs/design_review/evidence/2026-10-01_phase4-qualification/raw/phase4-q0-test-all{3,4,5}.log`.

- `lctx::compile_facts binary_catalog_with_briefs_replays_shared_embedding_winners` took 619–690 s. It runs **6 compiles** (2 profiles × 3 replays) of a source of about 30 lines (`crates/lctx/tests/compile_facts.rs:349–415`). That is about 100–115 s per compile.
- `binary_publishes_upper_frontiers_with_seedless_catalog_and_explicit_outcomes` took 298–372 s for 4 compiles plus a refusal (:177–217), about 75–90 s per compile.
- `cpg-core::analytic default_off_is_explicit_in_both_profiles` took 130–137 s.
- `cpg-core::summary_publication behavioral_summaries_publish_finite_proofs_and_validate` took 100–118 s.

Caveat: these tests ran concurrently with other nextest jobs, and include uv/pyrefly startup and the deployment stage. That stage was about 25 s on the real library; its fixture cost is unknown.

**Inferred:** these costs are dominated by fixed per-generation work:
- DDL for 1,086 tables, 1,061 indexes and views;
- per-stage completion framing;
- checkpoint validation at facts, normalized and analysis (`compilation.rs:557, :580, :597`);
- the final validate, which reinstalls reference constraints (`receipts.rs:350–396` installs and then rolls back FKs at each checkpoint);
- replay validators that re-run producers (`summary_replay.rs:120`; `structural/build.rs:1` "Reconcile … by replaying"; `selection/build.rs:1` "shared exact closure replay"; 79 files mention replay).

### 2.5 Developer loop

- **Release test builds:** 29m38s, 19m39s, 14m39s, 13m35s, 12m11s, 10m17s, 9m25s and 8m16s in Phase 4 logs (2026-10-01, `docs/design_review/evidence/2026-10-01_phase4-{qualification,restart}/raw/*`). Phase 3 had 5m51s and 7m09s. Late-September `build/pr4-*` logs show about 4–5 min. These are cold or partially warm whole-workspace test builds, not split per crate.
- **Older build-config probe:** 105–142 s full build (`docs/design_review/evidence/2026-09-24_rust-build-performance/README.md:13–18`), when the model crate was much smaller.
- **Full test suites:**
  - `test-all` 955.1 s for 993 tests (phase4-q0-test-all5);
  - 1,005 s and 854 s in earlier attempts;
  - a 374-test subset took 938.9 s.
- **Store installation tests:** `lctx-postgres::installation` tests took 100–188 s each (`check_detects_each_drift`, `check_clean_in_every_state`, `reset_is_phased_and_resumable`), and `lctx-postgres::domain_types` took about 103–109 s.
- **lctx-model size:** 178,595 lines in total, of which `crates/lctx-model/tests` is 42,972, leaving about 135.6k in src.
- **Not found:** `cargo --timings` output or any per-crate compile time for lctx-model (`target/cargo-timings` absent).

### 2.6 Instrumentation that exists

- Per-stage `elapsed_ms`, sampled peak RSS and reservation peak/end: `cpg-core/src/stage_runtime.rs:101–151`. These are emitted as tracing `facts stage` (`facts.rs:232`), `normalized compilation stage` (`normalize/pipeline.rs:137`) and `cumulative compilation stage` (`compilation.rs:515`), and returned as `stage_measurements` in the CLI JSON.
- `ValidationStats` counters exist (`validation_session.rs:29`) but are not emitted.
- `lctx_ops.events.recorded_at` and `lctx_model_store.events.at` exist, but both tables are empty in the current DB.
- Cache read/admit timing: `lctx-postgres/src/cache.rs:95, :175`.
- Prepared but unexecuted scripts: phase 3 `pilots.py` and `refusals.py` record time-v RSS, stage data, relation cardinalities, PG sizes and connection samples.

### 2.7 Candidate Phase B probes (measurements that would settle this)

- **P1 — instrument the lifecycle steps.** Add timing spans around `begin`, `complete_stage_step`, `checkpoint_step`, `seal_step`, `validate_step` (split: FK install, framing/digest, invariants, admission) and `publish_step`. Emit `ValidationStats`. Rerun the FastMCP facts/catalog compile. This would settle whether about 50% of wall time is validation, and which part.
- **P2 — split the pyrefly stage.** Separate analyzer time, fact build, Arrow encode and COPY, and completion framing (spans inside `bundle::run_stage` and `GenerationAttempt::copy`). Alternatively sample PG backend CPU through `pg_stat_statements` / `pg_stat_activity` with a query-text capture.
- **P3 — one authorized real-library `--through catalog --profile behavioral` run with stage tracing.** The existing per-stage tracing already covers Local, Base, Completion, SourceCalls, Enriched, Models, summaries, Structural, Analytics and Catalog. Add hydration time (`PreparedGraphs::load`) and projection vertex/arc counts. This is the only way to test premise (1)/(2) at scale for graph kernels and the summary engine.
- **P4 — fixed-cost probe.** Time an empty or tiny-input compile per frontier, single-threaded with no concurrent tests, splitting DDL from per-frontier validation. This would quantify the per-generation constant visible in §2.4.
- **P5 — developer loop.** Run `cargo build --release --timings -p lctx-model` (cold and after a one-line domain edit) to get per-crate frontend/codegen times.

## 3. Places that re-implement generic graph or relational mechanisms

| Mechanism | Location | Lines | A DB or library equivalent |
|---|---|---|---|
| Deterministic Kahn topological sort over the SCC condensation | `execution/summary_schedule.rs:61–108` | ~48 | petgraph `condensation` + `toposort` (custom tie-break needed); GDS SCC |
| Bounded BFS with parent-map witness over two graph views | `analysis/delegation.rs:352–470` | ~120 | petgraph `Bfs`; Cypher/SurrealQL bounded recursive traversal / shortestPath |
| Bounded BFS with linear-scan adjacency | `structural/controls.rs:525–680` | ~155 | Graph DB traversal; an indexed adjacency map would fix O(V·E) scanning |
| DFS route enumeration with 5-level nested scan joins per hop | `catalog/access_routes.rs:530–700` (file 1,444) | ~170 core | Recursive CTE or graph path enumeration plus SQL joins |
| Class-ancestry expansion with linear scans | `catalog/paths.rs:52–150` | ~100 | Recursive CTE / graph traversal |
| MRO lookup by linear scan over ancestry and sequence members | `normalized/dispatch.rs:307–360` | ~55 | Indexed join |
| PageRank power iteration | `analytics/ranking.rs` | 213 | `petgraph::algo::page_rank` (petgraph 0.8.3 pinned); Neo4j GDS |
| Exact kNN cosine | `analytics/neighbours.rs` | 489 | pgvector, or a DB vector index |
| FCA NextClosure | `analytics/concepts.rs` | 247 | fcars (already used as an oracle per AGENTS.md) |
| In-memory relational engine: whole-relation hydration into `Rows<R>` (BTreeMap by Id), with joins as `.iter().filter(..)` scans | `normalized/rows.rs` (114), `charged.rs` (215), `record.rs` (668), and macro-generated `visit/decode` per stage input set (e.g. `summary_production.rs:219–245`, `selection/build.rs:19–51`) | ~1k infra. ~986 `.iter()…filter(` sites across `domain/` (crude count; many are not joins) | SQL joins in PG, or DataFusion (already a workspace dependency in cpg-core) |
| Stored-content validation in Rust (framing, digests, invariants, admission scans, replay of producers) | `lctx-postgres/src/generations/validation_session.rs` (2,135), `receipts.rs` (545), `stage_validation.rs` (305), `verify.rs` (650); `lctx-model/src/domain/validation.rs` (1,136); 137 `fn finish(` validator impls in domain | ~5k+ | Partly DB constraints (FKs are also installed in PG, so reference checking is duplicated). Replay equality has no DB equivalent. |

## 4. Coverage and limits

- **Petgraph use:** `grep -rl petgraph crates/**/*.rs` found 10 files. **Traversal vocabulary scan:** VecDeque, worklist, fixpoint, visited, transitive, closure, adjacency, successors, pop, run over `lctx-model/src`, `cpg-core/src`, `cpg-extract/src`, `cpg-flow/src` and `lctx-analytics/src`. I read the top hits. Unread (vocabulary hits only): `selection/evaluate.rs`, `selection/preparation.rs`, `catalog/evidence/build.rs`, `analysis/family*.rs`, `attachment.rs`, `assertion.rs`, `types.rs`, `retrieval/build.rs`. They may hold small additional traversals; I believe none is a dominant graph kernel, but that is unverified.
- **Timing search:**
  - `build/*` (all files);
  - `docs/design_review/evidence/*` for elapsed, wall clock, seconds, RSS and Finished-in patterns;
  - the retired `2026-09-30_facts-qualification` folder via Git at 9fcc2e6f;
  - DB tables `lctx_model_store.*`, `lctx_ops.*` and `pg_stat_user_tables`.

  I did not search logs outside the repository (except a check of `~/.local/state`) or shell history. The 2026-10-03 run's log may exist elsewhere.
- **The "about 268 s validation" figure is inferred** from wall time minus the instrumented stage sum, plus connection-state samples. It could include unmeasured CLI-side work after assemble, such as report building; I consider that unlikely to be large.
- **Pre-cutover numbers** come from a different storage and compute architecture and from analytics-off configurations.
- **Fixture test durations** are under parallel load and include Python-side startup.
- **I did not render SurrealDB or Neo4j capability claims.** The (i) classification means expressible as a standard traversal or algorithm, not verified against SurrealDB 3.3.
