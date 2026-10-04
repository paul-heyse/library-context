# Design review: library leverage and displacement

**2026-10-04 · design tier · target purpose · slug `library-leverage`**

| Field | Value |
|---|---|
| Subject | Workspace crates `cpg-core`, `cpg-extract`, `cpg-flow`, `lctx`, `lctx-analytics`, `lctx-embed`, `lctx-model`, `lctx-model-macros`, `lctx-postgres`; Python packages `lctx_mcp`, `lctx_semantics`, `lctx_storage`. Read at `main` `b0256115`. The evidence was gathered at `948b2a88`. The only commit since then changes the `model_describe` snapshot, so no cited source moved. The working tree has concurrent edits to `STATUS.md`, the enrichment plan and `docs/library-utilization.jsonl`, none of them in the subject. |
| Standard | Core principles and template 3.2; code-intelligence profile 1.3 (principles and review additions); [library-context binding](../design_principles/binding/library-context.md) |
| Tier · purpose | Design · target. Slot 11 proposes revising blocking authority text. |
| Reviewer · date | Principal design reviewer (fresh `design-reviewer` subagent), 2026-10-04 |
| Maturity and outcome | Phase 5 serving is implemented, with qualification stopped. Analytical-enrichment full-gate repairs are in progress, and product work is paused. The review settles, for each consequential generic mechanism, whether to use more of a pinned library, adopt a new one, extend our own mechanism, keep it bespoke (with the reason) or defer it (with a trigger). It also names library-enabled enhancements that have a product consumer. It feeds `create-plan` and, where a §B decision moves, an ADR. |
| Supported scope | Generic mechanisms and their library alternatives across the subject, with adjacent design owners: DESIGN §B2/§B3/§B4/§B13/§B14, §14.11, §15.1–§15.3, and ADR-0044/0052/0053/0073/0085/0121. **Excluded:** library-catalog scripts and recipes, `.mcp.json`, generic `scripts/`, fixtures, gold and heldout data, fork internals beyond the patch surface, performance, and pin upgrades (findings only). |
| Expected changes | S1 add an analytic; S2 add a fact family or a publication relation; S3 upgrade Arrow (60 is released); S4 replace the summary worklist with Datalog; S5 replace capped `exists` with biodivine's limited fused op; S6 move hand SQL to sea-query or the unparser; S7 add a rendering; S8 upgrade an analyzer or rebase a fork patch. CI journeys: add a fact family, add an analytic, add or upgrade an analyzer, trace a served claim, run an evaluation. |
| Baseline | A pure typed model in `lctx-model` (ADR-0085), with PostgreSQL generations (ADR-0086), DataFusion providers in `cpg-core`, linked Ruff/ty/Pyrefly forks (ADR-0117/0118), graph and condition kernels on petgraph, leiden-rs and biodivine, and FastMCP over PyO3. |
| Method and coverage | Static review. I read the decisive source at the grain cited here (the files listed in §10). The lane files under [`2026-10-04_library-leverage/`](../evidence/2026-10-04_library-leverage/README.md) were used as leads and checked where they bear on a finding. Library facts come from the lanes' pinned-source reading (Interface-checked). The L3 probes P1–P6 passed on 2026-10-04 in an isolated crate (`cargo run --offline --release`, `l3-probe/output.txt`); they are Tested only for their fixtures and are not product tests. No product build, test or probe ran for this review (`not_run`). |

**Profile additions to slot 1.** In scope:
- Fact families: lexical scopes, bindings and resolutions; static branches; exports; runtime-special names; flow definitions.
- Analyses: SCC schedule, the summary fixpoint, ranking, communities, FCA, kNN, the BDD condition kernel.
- Served answers: retrieval corpus text, mapping/wire/consumer identities, MCP envelopes.

Workloads considered: extraction, graph/program analysis, answering and change. Evaluation is touched only through the Griffe-oracle caveat (CI-12).

## Integrated assessment

The system's library posture is mostly sound, and the main architectural costs are elsewhere.

**What is sound.** Where a pinned or established library fits, the code generally uses it: Ruff, Pyrefly and ty through narrow in-process seams; serde_arrow codecs; pgpq COPY; petgraph SCC; leiden-rs Leiden and its metrics; biodivine as the condition kernel; FastMCP's `Tool`/`ToolResult` hooks; PyO3 0.29 idioms.

**The bespoke generic kernels are justified.** This covers the Pareto worklist, keyed callee-first ordering, weighted PageRank with a stop report, NextClosure with iceberg pseudo-intents, the innermost-containment interval index, the shadow-schema differ, and the MCP final-bytes mirror. Each carries a contract that no surveyed library honours as a whole: charged admission before allocation, a typed refusal instead of truncation, order-independent determinism, and nominal identities that never leak library indices. The L3 probes give concrete reasons:
- rustworkx-core's keyed topological sort returns a silently truncated `Ok` on cycles (P2, P6).
- Ascent has only a wall-clock stop and no public pending set (P4).
- graphops double-counts duplicated neighbours (P5).

None of these should be displaced now. Two pinned-surface uses are worth taking later at named triggers: biodivine's limited fused op for `exists`, and petgraph `condensation(make_acyclic=false)` for the Proposed evidence-preserving condensation.

**The material problems are own-mechanism and authority defects, not missing libraries.** They have four structural causes.

1. **Stage I/O is declared by hand in many places instead of being generated from one declaration** ([F01](#F01), [F02](#F02)). ADR-0085 promises generated inventories and typed operation ports. In practice there are:
   - 66 exported relation-list macros and hundreds of local re-expansions;
   - the analysis publication set restated in more than 16 places across two crates;
   - writer declarations that duplicate stage outputs, checked only at run time;
   - 12 hand-rolled copies of the stage-grant closure beside its owner, one of which panics.

   The fix is to extend our own derive. No reflection library fits better.
2. **Physical and identity encodings lack a single declared form** ([F03](#F03), [F04](#F04)). The PostgreSQL column layout is derived four times. A persisted model digest and the serving mapping identity hash `Debug` renderings, including Arrow `Field` Debug, which demonstrably changed between Arrow majors.
3. **Domain values have no owned rendering** ([F05](#F05)). Served and indexed retrieval text uses `{:?}` of records; a float default is served as raw IEEE bits. Synthesis renders the same values with a different hand match.
4. **Authority text has drifted from the code** ([F06](#F06), [F10](#F10)). §B3 and §15 say DataFusion computes joins and derivations. It does not: every relational derivation runs in pure model kernels over `Rows`, and DataFusion is the scan, admission, memory-pool and inspection runtime. **I judge the code to be the better design**, because purity gives store-free testing, nominal types and charged refusal in one owner. The documents should move, through an ADR, rather than the code.

Smaller items follow: a thin `Rows` API ([F07](#F07)), dropped Ruff facts and doubly implemented version/platform decisions ([F08](#F08)), and minor bespoke duplicates of compiled crates ([F09](#F09)).

**Decision: Revise.**
- A1 and A3 are violated for the S1/S2 extension journeys.
- A2 is violated in scope by F02 (a stage-grant rule authored 13 times), F04 (DP-04 MUST: identity without a declared canonical form) and F05 (value meaning re-rendered per consumer).
- G1 fails on F02; G7 fails on F06/F10 (design text); G8 fails, minor, on F09.
- The core architecture is not in question.

Each correction is an extension of an existing owner, plus one documentation ADR. None needs a new library, and none should interrupt the in-flight full-gate repair. See §12 for order.

## 2. Responsibilities, dependencies and semantic ownership

| Component | Coherent responsibility and hidden decisions | Consumer contract | Allowed dependencies | Expected reason for change |
|---|---|---|---|---|
| `lctx-model` (pure) | Typed relations, identities, stage declarations, validators, all relational derivations and analytic kernels over `Rows<R>`; charged state (`charged.rs`, `resources.rs`) | `ValidatedModel`, `Stage`, `Rows`, typed kernels; no I/O | serde_arrow, arrow-{row,select,schema,array}, blake3, petgraph, leiden-rs, biodivine, fixedbitset, postcard, schemars | New fact family or analytic; policy changes |
| `lctx-model-macros` | `Domain`/`DomainCode`/`DomainSum` derives: keys, identity recipe, Arrow codec, `HeapSize`, codes | Generated impls | syn/quote | New generated forms (F01, F05) |
| `cpg-core` | Stage orchestration; DataFusion provider registration, scan admission, memory pool; generation-read provider with closed pushdown | `AttemptRuntime`, stage adapters, `sql.rs` read-only gate | lctx-model, DataFusion, forked PG provider | New stage adapters (currently hand-written per stage, F01) |
| `lctx-postgres` | Generation lifecycle, DDL lowering, COPY, leases, validation SQL, serving services | `GenerationStore`, leases, typed rows | sqlx, sea-query (tables only), pgpq | Physical layout (F03), lifecycle |
| `cpg-extract` / `cpg-flow` | Provider adapters (Pyrefly, Ruff seam, ty), the bespoke lexical recognizer, static branches, flow lowering | Attributed observation rows | Pinned forks | Analyzer upgrades and patch rebases (F08) |
| `lctx-analytics` | Re-exports of model kernels; independent oracle tests | Native entry points | (dev) fcars, odis, leiden-rs | none |
| Python (`lctx_mcp`, `lctx_storage`, `lctx_semantics`) | FastMCP transport, byte-exact envelope mirror, numerical BM25, query embedder twin | MCP tools/resources | FastMCP, mcp, pydantic, bm25s, PyO3 | SDK upgrades (mirror drift) |

Dependencies point from mechanism to meaning: `cpg-core` and `lctx-postgres` depend on `lctx-model`, never the reverse. Python reaches the store only through PyO3. That direction is a strength to preserve. Any move of relational work into DataFusion would invert part of it (F06).

**Semantic authority, restricted to the decisions this review touches:**

| Decision | Authority | Restated or derived where | Status |
|---|---|---|---|
| Which relations an analysis family publishes | `analysis_family!` → `publication_relations()` (`analysis/family.rs`) | Restated in at least 16 stage-output and writer lists in lctx-model and cpg-core | Duplicated by hand; reconciled at run time ([F01](#F01)) |
| A stage's outputs (producer ownership) | `Stage.outputs` (ADR-0085: "typed outputs alone declare producer ownership") | Writer `declare!` lists in cpg-core and cpg-extract | Restated; reconciled at run time (`stages.rs:2129`, `:2236`) ([F01](#F01)) |
| A stage's grant closure | `DependencyClosure::build` (`dependency_closure.rs`) | 12 hand loops in `stage()` functions | Independent definitions; no reconciliation ([F02](#F02)) |
| Physical column layout of a relation | DDL `lower` (`ddl.rs`) | View text in `ddl.rs`; `serving_shape.rs` expected text; `column_signature`; `codec.rs` | Restated; reconciled at lease time ([F03](#F03)) |
| Model and mapping identity encoding | `KeySink` framing (`identity.rs`) | `Debug` text inside `model.rs:311` and `serving/mappings.rs` | Undeclared canonical form ([F04](#F04)) |
| Textual rendering of domain values | none | `{:?}` in `retrieval/build.rs`; own match in `synthesis/assertions.rs` | No owner ([F05](#F05)) |
| DataFusion's role | DESIGN §B3, §15 | Code: scans, admission, pool, inspection only | Stale text ([F06](#F06)) |

**Fact and fidelity table** (profile addition; families touched here only):

| Fact family or relation | Provider and revision | Fidelity | Coverage and unknowns | Identity | Consumers |
|---|---|---|---|---|---|
| Lexical scopes, bindings, resolutions | `lctx-lexical` recognizer (`lexical.rs`), outside names from Pyrefly 1.4.0-dev.3 | Extracted / recognizer | Unresolved and star imports explicit; PEP 695 type-parameter scopes not modelled | Occurrence span and kind | Normalization, ty flow root selection |
| Ruff lexical observations | Ruff 0.16.10 fork `semantic_facts` | Extracted (supplement) | `Incomplete` explicit; Generator/Type scopes unlocated; `Export`/`Branch` facts dropped | Span-attached; native ids local | Context rows, overload resolution |
| Flow definitions and reachability | ty 0.0.16 `semantic_index` (behavioural profile) | Extracted, flow-sensitive | Catalog profile `NotRequested`; type-parameter scopes skipped to match the recognizer | Span plus definition | Behavioural analyses |
| Static branch marks | Pyrefly `SysInfo` (spelling) | Resolved by Pyrefly | Undecided is `None` | Clause range | Lexical binding marks |
| Runtime version/platform decisions | `cpg-flow/predicate.rs` (resolved roots) | Derived | Tuples longer than 3 fields undecided | Condition atom | Flow conditions |
| `__all__` | Own syntactic detector plus Pyrefly `Exports` | Extracted / resolved | Literal versus computed explicit | Span | Public surface |
| Retrieval corpus text | Model rendering (`retrieval/build.rs`) | Derived rendering | Partly `Debug` text | `ContentHash` of text plus `RENDER_VERSION` | Served view, bm25s, embeddings |

Attribution (CI-01) and explicit unknowns (CI-04) hold for these families as far as I examined them. The two weaknesses are that a library's own observations are dropped ([F08](#F08)) and that served renderings are representation-shaped ([F05](#F05)).

## 3. Contracts, constraints and testing boundaries

Every replacement candidate was judged against these preserved guarantees:

| Guarantee | Where enforced | Why it decides library fit |
|---|---|---|
| Charge before allocation; unbound refuses | `StateCharge`/`ChargedMap` (`charged.rs`), `ResourceBudget` reservations, DataFusion `MemoryPool` via `ComputePool` | Most container, graph and Datalog crates allocate uncharged (multi_index_map, rustworkx-core, ascent, odis, leiden `compute_flow`) |
| Typed refusal, never silent truncation | `Stop::{…}`, `Admission::Refused`, `KernelBoundary`, `RouteStop` | rustworkx `lexicographical_topological_sort` truncates on cycles (P2/P6); ascent's `BoundedSet` saturates without a reason |
| Determinism independent of input order | Sorted `Id` keys, canonical tie-breaks | indexmap, multi_index_map and hashbrown iterate in insertion or hash order |
| Nominal identity; no library index escapes | `Id<T>`; dense indices private (M1-14) | Every graph adapter keeps library indices private |
| Five distinct verdicts; unknown is not absent | `obligation::Verdict`, `KernelBoundary` means unknown (`kernel.rs` docs) | biodivine's limit-taking `None` maps to unknown; unbounded variants do not |
| Model purity | `lctx-model/Cargo.toml` (no DataFusion, SQLx or I/O) | Rules out DataFusion joins and polars inside the model (F06) |
| Append-only codebooks | `DomainCode` explicit i16 codes | strum or num_enum would create a second label/discriminant source for codebook enums |

**Testing boundaries.** Model kernels are testable with no store: `tests/domain_*`, the oracle tests in `lctx-analytics`, and the in-memory validator. Stage I/O consistency (F01) and the physical layout (F03) are testable only through real-PostgreSQL stage runs and leases. That is why their duplication is caught late.

## 4. Composition and execution

| Stage | Inputs → outputs | Owning mechanism | Policy versus orchestration | Limits and determinism |
|---|---|---|---|---|
| Stage adapter (cpg-core) | Permit-gated `SELECT * FROM "<R>"` scans → `Rows<R>` → model `build` → `StageOutput` | Hand-written per stage: 13 `load<R>` copies, read/write/declare macros | Orchestration restates domain lists ([F01](#F01)) | Single-flight, pool-charged |
| Relational derivation | `Rows` → typed outputs | `lctx-model` kernels (scans, `ChargedMap` indexes) | Domain-owned | Retained state charged; scan work uncharged by design |
| Validation | Batches → pass or refuse | `InvariantCheck` folds plus PostgreSQL SQL | Shared model validators | Streamed in declared order |
| Publication | Validated generation → selection | `lctx-postgres` lifecycle | Owned | Leases re-check model and physical digests |

**Analysis record columns** (profile addition):

| Analysis | Projection | Method and settings | Exact / conservative / heuristic | Budgets and partial results | Library position |
|---|---|---|---|---|---|
| Callee-first SCC schedule (`summary_schedule.rs`) | Invocation graph; parallel arcs collapse (ordering only) | `kosaraju_scc` plus keyed Kahn sort on minimum member | Exact | Malformed partition refuses; memory reserved | Keep (rustworkx is order-equivalent but truncates on cycles and adds rayon, ndarray and rand) |
| Summary fixpoint (`summary_production.rs`) | SCC-local | Pareto frontier plus charged work queue | Conservative under the stated model | Residuals publish pending items | Keep (ADR-0053; P4 and the 2026-09-26 probe confirm Ascent gaps) |
| Weighted PageRank (`ranking.rs`) | Aggregated weighted arcs | Power iteration, L1 residual | Heuristic (CI-09) | `Stop::{Converged, Empty, IterationLimit, WorkLimit}` | Keep (graphops is the closest match; adds a crate and double-counts duplicate neighbours) |
| Leiden profile (`communities.rs`) | Normalized undirected pairs | leiden-rs RBER, fixed seeds | Heuristic | Preflighted work | Library already used; statistics stay own |
| FCA (`concepts.rs`) | Formal context | NextClosure plus iceberg pseudo-intents | Exact; partial on budget | `budget_reached` | Keep (ADR-0121 holds; odis's lazy iterator noted) |
| Bounded `exists` (`kernel.rs:723-748`) | Condition BDD | restrict ×2 plus capped `or` per variable | Exact or unknown | Pair-product preflight | Pinned `fused_binary_flip_op_with_limit` deferred (changes refusal thresholds) |

## 5. Change and failure scenarios

| Scenario (kind) | Owner | Contract change | Expected versus observed consumers | Independent edits, hidden knowledge, setup | Evidence |
|---|---|---|---|---|---|
| **S1 Add an analytic** (domain concept) | New `analysis_family!` owner | New typed outputs | Expected: one family instance plus the kernel. Observed: an inventory macro, a data-struct expansion, stage outputs restating the 11 publication relations, a cpg-core loader, a writer `declare!` restating them again, and the expected-domain-inputs fan-out | Duplicated lists in two crates; mismatch found only by a real stage run | Implemented (source), [F01](#F01) |
| **S2 Add a publication relation** to every analysis family (domain concept) | `analysis_family!` | One more generated relation | Expected: the macro. Observed: at least 16 hand lists (the 11-name list appears in 21 files; `CoverageRequiredSource` in 22) | As above | Implemented, [F01](#F01) |
| **S2' Add a fact family** (domain concept) | Model declaration plus provider | New relation | Inventory macro, then each expansion site, plus provider `uses!`/`declare!` pairs (`pyrefly_stage.rs:116-140` and `:321-345`, identical) | Restated lists | Implemented, [F01](#F01) |
| Change a closure policy (policy) | `DependencyClosure` | Grant rule | Expected: one owner. Observed: 13 | Copies diverge silently | [F02](#F02) |
| Change the physical layout (mechanism) | DDL lowering | Column convention | Four derivations | Detected at lease time | [F03](#F03) |
| **S3 Upgrade Arrow 59→60** (provider upgrade) | Model codecs | `Metadata` struct; serde_arrow `arrow-60` | Identity bytes change through `Field` Debug with no semantic change | Hidden coupling to a library rendering | Interface-checked (arrow-schema source), [F04](#F04) |
| **S4 Summary worklist → Ascent** (mechanism substitution) | `summary_production.rs` | Same contract | Charging, deterministic work bound, residual publication and refusal with a reason cannot be honoured (P4: time-only stop, hidden deltas) | Substitution would rebuild the contract around the engine | Tested P4 (2026-10-04); the 2026-09-26 probe agrees. Keep. |
| **S5 Capped `exists` → limited fused op** (mechanism substitution) | `kernel.rs` | Same result | Local to the kernel; equal on 4,490/4,490 cases (P1); **refusal thresholds and published boundaries change** | A policy change that needs qualification | Tested P1; defer (§11) |
| **S6 Hand SQL → sea-query or the unparser** (mechanism substitution) | `ddl.rs`, `generation_read.rs` | Same SQL meaning | sea-query 1.0.2 lacks view, grant and schema builders (L1); the rendered DDL digest changes (a rebuild under ADR-0078); `plan_to_sql` could render whole scans | Small gain; F4 not fired | Interface-checked; defer |
| **S7 Add a rendering** (composition) | none | n/a | Each renderer reinterprets values (`{:?}` versus a hand match) | No owned labels | [F05](#F05) |
| **S8 Analyzer upgrade or fork rebase** (provider upgrade) | Provider adapters | Patch surface | The Ruff patch emits `Export`/`Branch` facts nobody consumes; the Pyrefly patch is now +348 lines in the solver | Rebase cost without a consumer | [F08](#F08), [F10](#F10) |
| CI: trace a served claim (retrieval text) | Retrieval rendering | n/a | Served text cites real rows (same generation) but renders representation | Not an evidence-closure failure | [F05](#F05) |
| CI: an evaluation run | Oracles | n/a | Griffe shares an extractor with the fastmcp gold skill | Agreement is not independent corroboration of gold items | CI-12 constraint, §8 |

## 6. Correctness and fidelity gates

| Gate | Verdict | Own evidence or scope reason | Required action |
|---|---|---|---|
| G1 Authority | **fail** | The stage-grant closure rule is defined independently in 12 `stage()` functions beside `DependencyClosure::build`, with no reconciliation ([F02](#F02)). The F01 and F03 duplicates **are** reconciled at run time (`stages.rs:2129`, `serving_shape.rs` comparison), so they fail locality, not G1. | F02 |
| G2 Semantic fidelity | pass | `Debug` renderings are lossless though opaque (F05). Arrow `Field` Debug omits only defaults. No relation is reinterpreted. | none |
| G3 Validity | pass (with note) | `local_semantics.rs:~915` panics on a model contract violation during schedule construction. The model is static and the failure aborts rather than proceeding, but the rejection is untyped. | Typed error inside F02 |
| G4 Hidden behaviour | pass | No ambient read found in the mechanisms examined. The `Field` metadata `HashMap` Debug order is latent (no metadata is set today). | F04 |
| G5 Consistency and recovery | pass (with note) | The enriched round cap (`enriched_production.rs:330`, 64 rounds) ends in explicit `ExecutionBoundary` rows, but their reason does not name the cap (observation O1). Adopting rustworkx ordering would fail G5 without a length check; recorded as a preservation constraint. | O1 (low) |
| G6 Transformation and reuse | pass | Identity drift is conservative: a spurious mismatch makes leases refuse. Corpus reuse is keyed on text digest plus `RENDER_VERSION`. | none |
| G7 Truthful capability claims | **fail** (design text) | §B3 and §15 label DataFusion joins and derivations "Implemented"; none exist ([F06](#F06)). Stale ADR, plan and skill text ([F10](#F10)). | ADR plus DESIGN amendment; owner edits |
| G8 Library leverage | **fail** (minor) | Hand hex codec where `hex` is compiled; hand docstring prefix/quote scan where Ruff `str` helpers are linked; no stated reason for bypassing biodivine's limit-taking op ([F09](#F09)) | F09 |
| CI-G1 Fidelity | pass | The spelling-matched `isinstance` atom is admitted only with a resolved builtin witness (`local_theory.rs::builtin_operand`), so no relabelling. Ruff `Unresolved` builtins are attributed to Ruff. | Keep disagreement visible (F08) |
| CI-G2 Evidence closure | not assessed beyond F05 | No served-claim path is changed by any recommendation. Corpus text is a same-generation rendering, not a claim. | none |
| CI-G3 Evaluation integrity | pass (constraint) | No gold path reaches inputs. Griffe, if adopted as an oracle, must not count as corroboration of gold-scored items (shared extractor). | §8 constraint |

## 7. Findings

| ID | Finding (short) | Principles · judgment/gate | Priority | Route |
|---|---|---|---|---|
| [F01](#F01) | Stage ports, publication sets and writer declarations restated by hand instead of generated | FP-01/03/04, DP-01/06/16/24 · A1, A3 | Architectural barrier to planned work | create-plan package (implementation inside ADR-0085) |
| [F02](#F02) | Stage-grant closure authored 12 more times beside its owner; one copy panics | FP-04/05, DP-01/03/14 · A2, G1 | Repeated semantic authority | Implementation inside an accepted contract |
| [F03](#F03) | Physical column layout derived in four places; quoting helper duplicated | DP-01/06, FP-04 · A1 | Repeated derivation | Implementation |
| [F04](#F04) | Persisted and serving identities hash `Debug` renderings; JSON identities depend on formatter and features | DP-04 (MUST), FP-02, DP-24 · A2 | MUST gap (fail-closed) | Implementation (identity migration); JSON part deferred |
| [F05](#F05) | Served and indexed text renders domain values through `Debug`; no owned value rendering | FP-04, DP-01/02 · A2 | Repeated semantic interpretation in presentation | create-plan package, sequenced with §14.12 |
| [F06](#F06) | §B3/§15 claim DataFusion relational compute that the code does not perform | DP-22/24 · G7 | Stale authority | ADR plus DESIGN amendment |
| [F07](#F07) | `Rows` lacks the lookup, index and semijoin primitives consumers repeat | DP-14/16 · A1 (minor) | Code economy | Implementation |
| [F08](#F08) | Ruff facts dropped; version/platform decisions implemented twice without a cross-check; type-parameter scopes capped by the recognizer | CI-01/04, FP-04, DP-01/16 · A2 (scoped) | Fidelity hygiene | Code-facts coordinator / create-plan |
| [F09](#F09) | Small bespoke duplicates of owners or of compiled crates | DP-13/14/16 · G8 | Code economy | Implementation |
| [F10](#F10) | Stale ADR, plan, trigger and skill text | DP-22/24 · G7 | Documentation | Owner amendments |

<a id="F01"></a>
### F01 · Stage ports are restated by hand instead of generated from one declaration

**Evidence** (Implemented; source read 2026-10-04):
- `lctx-model` has **66** `#[macro_export]` relation-list macros (count verified). M2's heuristic survey finds about 268 local re-expansions and 140 hand-written type lists across the workspace.
- `analysis_family!` generates `publication_relations()` per owner (`analysis/family.rs`, used by `analysis/mod.rs:71-102`). The same 11 relations (`Invocation` … `CoverageRequiredSource`) are nevertheless spelled by hand:
  - in stage outputs, for example `catalog/build.rs` `stage_outputs()` `add!(…)` (~808–822);
  - in writer declarations, for example `cpg-core/src/catalog_core.rs` `declare!(…)` (~111–124), which adds `catalog::CatalogMemberInvocation` to the same list.
  The list's lines occur in 21 files; `CoverageRequiredSource` appears in 22 files across `lctx-model`, `cpg-core` and `cpg-extract`.
- Producer ownership is checked only at run time. `StageOutput::declare` refuses an output outside the stage or declared twice (`stages.rs:2129-2152`), and finishing refuses undeclared outputs (`:2236`).
- Providers duplicate their own lists. `cpg-extract/src/pyrefly_stage.rs` has `uses!` (116–140) and `declare!` (321–345), and they are identical.
- `cpg-core` repeats per-stage loader code: 13 `async fn load<R>` copies plus `read!`/`meta!`/`expected!`/`write!` macros (for example `catalog_core.rs:60-77`; M4-09).
- ADR-0085's Decision says "Generated forms include … inventories, model descriptions and typed operation ports" and "A stage's typed outputs alone declare producer ownership". DESIGN §15.2 limits the derive to "typed key, identity recipe, physical Arrow codec and metadata". `lctx-model-macros` generates no ports or inventories and has no test directory.

**Consequence.** For S1 and S2, adding one relation to the analysis publication contract means one generator edit plus at least 16 hand edits in two crates. Adding an analytic owner means an inventory macro, data-struct expansions, a cpg-core loader and writer, and a writer `declare!` that restates the stage outputs. An omission compiles. It surfaces only when a real stage runs against PostgreSQL, so a model-local change needs store-backed testing to validate. `cpg-core` also has to know each family's publication set, which is domain knowledge in orchestration (FP-01). This is the main obstacle to the post-Phase-5 "add an analytic" and "add a fact family" journeys.

**Correction (Proposed).** Extend `lctx-model-macros` with a **port-set derive**.

1. A port struct is declared once, with each field's record type, transport (stored, completed, epoch-bound vocabulary), profile membership and optional validator.
2. The derive generates:
   - the data struct, name-dispatch `visit`, `validation_inputs()`, `stage_inputs()` and `matches`;
   - a generic visitor, `fn each<V: PortVisitor>(v)` with `fn port<R: Record>(&mut self, name, spec)`.
3. `cpg-core` implements the visitor once each for register/load, declare and write. It then stops restating any list.
4. Port sets compose (for example by tuples). The analysis publication set becomes one named type derived from the `analysis_family!` instance and referenced from stage outputs and writer declarations alike.
5. The runtime checks stay as defensive enforcement.

The same derive work should let `DomainCode` emit a schema, `label()` and `ALL` (shared with F05 and F09).

Library options were considered and rejected (N1 §2.1):
- facet and bevy_reflect give runtime shapes whose vtables cover only std traits, so a hand registry of `fn(&mut dyn Any, &RecordBatch)` would still be needed. That turns types into runtime data, against §15.2.
- frunk HLists can iterate types, at a large cost in signatures and diagnostics.

**Challenge.** Two cases could reject valid work:
- Profile-dependent exclusions (for example the Catalog-profile `retain` list in `binding_normalization.rs:1553-1586`).
- Inputs built as unions of upstream stages' inputs.

The port spec must express both. Any that cannot be expressed stays an explicit, bounded exception rather than forcing a general DSL. Async loading through a visitor on the pinned nightly is the open premise; a compile spike settles it.

**Verification.** Trace S2. One declaration edit should produce compile errors, not runtime refusals, wherever behaviour must change. The hand lists should be gone and the runtime checks should still pass. Add compile-fail cases for the new derive's errors (`compile_fail` doctests; trybuild only if diagnostics matter).

<a id="F02"></a>
### F02 · The stage-grant closure is authored 12 more times beside its owner, and one copy panics

**Evidence** (Implemented). The owner is `dependency_closure.rs` `DependencyClosure::build`. It:
- resolves vocabulary epochs through `PublicationOrder`;
- keys on (name, prefix, order);
- refuses unfinished own outputs with a typed `ModelError`;
- states that validation universes and grants are different products.

Twelve `stage()` functions hand-roll the same walk (`while let Some(name) = pending.pop()` over `field.target()` plus invariant inputs, minus facts, refusing own outputs). The files are `local_semantics.rs`, `execution/{production, enriched_production, model_production, completion_production, summary_replay, source_call_records}.rs`, `analysis/{frontier, preparation}.rs`, `structural/build.rs`, `retrieval/build.rs` and `embedding/text.rs` (grep count 12). The copies dedup by name only. `local_semantics.rs:~897-915` uses `.expect("Local predecessor declared")` and `panic!("Local predecessor depends on its own output…")`, whereas `execution/production.rs:~549-580` returns typed errors. Relation lookup by name is a linear `model.relations().iter().find(…)` at about 46 sites, because `ValidatedModel` has no by-name accessor.

**Consequence.** One rule, the grant closure over a stage's inputs, has 13 independent definitions (G1). A policy change in the owner, such as a new `LowerLayerPolicy` or different epoch handling for vocabulary relations, does not reach the copies, and nothing would reconcile them. A contract violation in the local stage aborts the process instead of returning a typed refusal (DP-03). Whether the name-only copies are already wrong for vocabulary relations was not traced (§10).

**Correction.** Add a grant-only operation to `dependency_closure.rs`, parameterized by the stage's own outputs and the lower-layer policy, and give `ValidatedModel` a name index. Replace the 12 copies and delete them. No library is needed; a petgraph DFS is optional, and a plain function is sufficient.

**Verification.** All call sites use the owner. A negative control (a local predecessor that depends on its own output) returns `ModelError`. Schedule digests are unchanged, or change only where a copy was wrong, which then becomes a recorded correction.

<a id="F03"></a>
### F03 · The physical column layout is derived in four places

**Evidence** (Implemented). The rule is: `generation_id`, `id`, `introduced_epoch` for vocabulary relations, then the fields, plus `__{field}_tag` for subtype references. It is built:
- in `ddl.rs` table lowering (~225–260);
- again in `ddl.rs` view text (~440–458);
- again in `serving_shape.rs:27-45`, which compares its text with `pg_get_viewdef` after `compact()` removes whitespace and quote characters;
- in `column_signature` (`ddl.rs:67-97`), which restates the Scalar→PostgreSQL type map from lowering. `codec.rs` holds a third type mapping (M4-06).

The quoting helper exists twice with identical bodies (`generations/mod.rs:1172`, `cpg-core/src/generation_read.rs:614`).

**Consequence.** Any change to the physical convention (another hidden column, a type mapping) needs coordinated edits in four places. A mismatch is caught at real-PostgreSQL install or lease time, not where the change is made.

**Correction.** Create one `physical_columns(relation)` (name, PostgreSQL type, nullability, list) owned by the DDL lowering. Table creation, views, `serving_shape`'s expected text, `column_signature` and codec dispatch should all consume it. Export one quoting helper. View and grant text stays text, because sea-query 1.0.2 has no view, grant or schema builders (L1), but it is built from one list. A structural comparison via sqlparser (pinned through DataFusion) is optional and not needed.

<a id="F04"></a>
### F04 · Identities hash `Debug` renderings without a declared canonical form

**Evidence** (Implemented / Interface-checked):
- **Model digest.** `model.rs:311` does `digest.part(b"sum", format!("{sum:?}"))`. Every other part of that function is framed explicitly. The model digest is persisted with generations and receipts and gates leases and selection (`generations/mod.rs:329,524,572`, `receipts.rs:319,444,462`, `selection.rs:296`).
- **Mapping identity.** `serving/mappings.rs:119,127,132,146,149,169` hashes `Debug` of the prepared relation, `Capability`, Arrow `Field` and `Sum`. `identity_for` already encodes key, target, subtype and codes explicitly beside the `Debug` part. At arrow-schema 59.3.0, `Field`'s `Debug` is a hand implementation that omits a default name, `nullable: false` and empty metadata, and prints a `HashMap` (`field.rs:63-100`, read 2026-10-04). At 56.2.0 it was derived (N1). The same schema therefore hashed differently across Arrow 56→57.
- **No production consumer yet.** `MappingIdentity` and `consumer_identity` are used only in `tests/serving_contracts.rs:307-346`.
- **JSON-byte identities.** `selection_digest`, `wire_identity` and `EmbeddingSpec::canonical_json` hash `serde_json` output. Map order depends on workspace feature unification (`preserve_order` via `lctx-workspace-hack`), and the float formatter is an implementation detail (N1).

**Consequence.** For S3 (Arrow 60, whose `Metadata` struct is changing), or any refactor that renames `Arm`/`ArmField` fields, identity bytes change with no semantic change. A changed model digest makes leases refuse existing generations, forcing an undeclared rebuild. That is fail-closed and tolerable under ADR-0078, but it is not a declared migration.

The mapping identity is latent but defective in a worse way: if field metadata is ever set, it becomes process-nondeterministic, and the first consumer (cache reuse under CI-13 via `consumer_identity`) would inherit an identity whose equivalence level is undefined. This violates DP-04's MUST to define canonicalization (ordering, encoding, defaults, versions) before using content hashes as identity.

**Correction.**
- Encode the model's own declarations through `KeySink` parts: sum tag, arm codes, arm field names and required flags; for mappings, scalar, nullability, list, target, subtype and codes.
- Drop the Arrow `Debug` parts. No library supplies a canonical Arrow-schema encoding; IPC flatbuffers and serde on `DataType` are not contracts (N1).
- Record the one-time byte change as an identity migration (rebuild).
- Add the missing known-answer vectors: the `ids__*` snapshots have no owning test, and `KeySink` v3 has no known-answer test (M2).
- **JSON identities (deferred):** declare their canonical form, either RFC 8785 via `serde_json_canonicalizer` 0.3.2 or postcard/`KeySink`, at the trigger in §11.

**Verification.** Known-answer vectors independent of `Debug`. A control showing that adding Arrow field metadata does not change the mapping identity.

<a id="F05"></a>
### F05 · Served and indexed text renders domain values through `Debug`; no owned value rendering

**Evidence** (Implemented):
- `retrieval/build.rs` renders exposures, callables, signatures, parameters, aspects, constructors, scenarios and diagnostics with `{:?}` (lines 127, 150, 163, 178, 192, 213, 228, 250, 422–504, 639).
- `:228` renders a whole `value::Literal` record, so a float default appears as `Float { bits: <i64> }`.
- The text becomes `retrieval_corpus_texts` (`retrieval/mod.rs:77`), is served through `serving_retrieval_texts` (`mappings.rs:93`), and is tokenized for bm25s and embedded.
- Synthesis renders the same `Literal` with its own match (`synthesis/assertions.rs:751-763`: floats as "IEEE bits …", strings via `Debug`).

**Consequence.** Under S7 every new renderer reinterprets the value, and the existing two already disagree. Served retrieval text exposes Rust representation names, wrapper types and raw IEEE bits, and those become lexical tokens and embedding input. A query for a documented default like `30.0` cannot match a float rendered as bits. A variant rename silently changes served text. Reuse stays sound because of the text digest, `RENDER_VERSION` and the model source hash, so this is a model-authority defect (A2), not a G6 failure.

**Correction.** Give domain values owner-defined renderings:
- `DomainCode` emits `label()` from its existing `codes()` names (shared derive work with F01).
- Value types own their Python-facing text. For `Literal`: `True`/`False`/`None`, the canonical decimal, a quoted string, and the float's Python `repr` from its bits, including `inf`, `nan` and `-0.0`.

Retrieval and synthesis consume these, and `RENDER_VERSION` is bumped. Template engines are not indicated (N1 §2.5): they would not supply labels, and the brief-restructuring trigger has not fired.

**Sequencing.** This changes the retrieval corpus, so it interacts with §14.12 ("freeze a development set before changing retrieval"). Land it before the dev-set baseline is measured, or treat it as a measured retrieval change after the freeze.

<a id="F06"></a>
### F06 · §B3 and §15 claim DataFusion relational compute that the code does not perform

**Evidence.**

The design text says DataFusion does relational compute:
- DESIGN §B3: "Implemented … Joins, projections and unions construct suitable normalized and analysis relations".
- §15.1: "`cpg-core` orchestrates stages and runs DataFusion derivations and semantic validators".
- The §15 layer table lists "Normalization stages (DataFusion, one Rust binder)" and "Analysis stages (… DataFusion)".
- §15 Compute: "DataFusion computes derivations and semantic validators over in-memory Arrow batches".
- ADR-0085: "DataFusion retains relational compute".
- ADR-0073: "Keep suitable bulk joins in DataFusion".

The code does not:
- `cpg-core` issues only whole-relation `SELECT * FROM "<R::NAME>"` scans through `sql.rs`. Grepping `cpg-core/src` and `lctx/src` on 2026-10-04 found no join, aggregate or union; L1 agrees.
- Results decode into `Rows<R>`, and all derivations are `lctx-model` kernels.
- Validators are `InvariantCheck` folds and PostgreSQL SQL.

**Judgment on question 3a.** The text is stale; the code is not under-using DataFusion.

The current placement is the better design for this target:
- Model purity (ADR-0085) lets every derivation be tested without a runtime (FP-06).
- It keeps nominal `Id<T>` types, charged refusal and ambiguity detection with their semantic owner.
- DataFusion operators carry none of those (L1 §2.2), and its hash joins cannot spill (the disk manager is disabled).

Moving joins into DataFusion would split each derivation between `cpg-core` and `lctx-model` (an FP-01 regression) and make kernels store-dependent to test. DataFusion's real responsibilities are:
- generation-bound providers with closed-predicate pushdown;
- scan admission;
- the memory pool behind `ResourceBudget`;
- read-only inspection (`lctx query`).

**Correction.** A new ADR superseding the DataFusion-compute clauses of ADR-0085 and ADR-0073 (accepted records are immutable). It should amend §B3, §15.1, the §15 layer table, §15 Compute and the binding's §B3 row to state DataFusion's actual role and the contract of the in-model relational kernels. Its revisit trigger appears in §11.

<a id="F07"></a>
### F07 · `Rows` lacks the lookup, index and semijoin primitives its consumers repeat

**Evidence.**
- There are 52 identical `fn need<R>` helpers (49 in `lctx-model`, 3 in `lctx-postgres`), differing only in the error message.
- `Rows` (`normalized/rows.rs`) offers `get` only.
- Secondary indexes are built by hand, and semijoins are `iter().any` scans (M2-02, for example `synthesis/documentary_templates.rs:46-65`).

**Correction.** Add `Rows::need` (whose error names the relation), a charged `index_by` returning `ChargedMap<K, Vec<Id<R>>>`, and a `unique_by` that reports ambiguity. Migrate opportunistically. Rejected libraries (N1 §2.2):
- multi_index_map: insertion order, no charging, per-row-type indexes only;
- indexmap: insertion order;
- hashbrown: unordered.

Scan work stays uncharged by the documented design (`charged.rs` header). Performance is not a gate.

<a id="F08"></a>
### F08 · Ruff facts are dropped; version and platform decisions are implemented twice; type-parameter scopes are capped by the recognizer

**Evidence.**
- **(a) Dropped facts.** `ruff_lexical.rs:17-24` `NativeRows::capture` keeps Scope, Definition, Binding, Reference and Unresolved. `Fact::Export` and `Fact::Branch`, which our fork patch emits, are referenced only in tests (`tests/ruff_context.rs:53,83`).
- **(b) Two implementations.** Pyrefly `SysInfo::evaluate_bool` decides branches by spelling (via `native_branches.rs`). `cpg-flow/src/predicate.rs:359-467` decides by lexically resolved roots and leaves `version_info` tuples longer than 3 fields undecided. `ty_flow.rs:438-478` string-matches `resolved_module` for `typing`/`sys`/`os`. TYPE_CHECKING legitimately differs by view (typing versus runtime). `sys.version_info`, `sys.platform` and `os.name` do not depend on the view for a single analysis context, yet they are decided by two implementations with known different coverage, and disagreements are not recorded.
- **(c) Capped coverage.** `cpg-flow/src/lib.rs:529-540` and `755-766` skip ty's PEP 695 type-parameter scopes because they are "not modelled by our lexical recognizer".

**Not a finding: replacing `lexical.rs`.** L2's comparison stands. Ruff resolves one binding per read, has no `LOAD_NAME` dual candidates and uses its own builtins. ty's index has no builtins or class-body fallback and runs only in the behavioural profile. Neither preserves candidate sets. The 2026-09-23 reasons have partly changed (the fork seam now exists), but the decisive one, candidate sets, holds. Keep `lexical.rs`.

Also not a finding: the spelling-matched `isinstance` (`predicate.rs:233-266`) is guarded downstream by a resolved builtin witness.

**Consequence.**
- For S8, fork rebases carry output with no consumer.
- A second, independent assertion for `__all__` and TYPE_CHECKING exists but is not recorded, although CI-01 asks that disagreeing providers be kept.
- A version-test disagreement is invisible.
- Type parameters get no flow definitions. Whether that is recorded as explicit coverage was not traced (§10).

**Correction (Proposed).**
- (a) Lower `Export` and `Branch` as attributed Ruff observations, keeping the own `__all__` detector for the literal/computed distinction (L2), or remove their emission from the patch.
- (b) Add a range-joined cross-check between Pyrefly's clause decision and `predicate.rs`'s runtime decision for version and platform tests, recording disagreements as observations rather than changing either authority.
- (c) Pass Pyrefly's builtins through Ruff `custom_builtins`.
- (d) Add Generator and Type scope ranges to the existing Ruff patch, then either anchor type-parameter scopes or record them as explicit unknown coverage.

**Route.** The [code-facts coordinator](../../plans/code-facts-expansion-plan_2026-10-03.md#7-current-checkpoint-and-next-action), which owns analyzer migration, under the migration plan's spelling-recognizer gate.

<a id="F09"></a>
### F09 · Small bespoke duplicates of an owner or of an already-compiled crate

| Item | Evidence | Correction |
|---|---|---|
| Hex codec | `serving/cursor.rs:62-91`, `Id::hex`, `ContentHash::hex`; `hex` 0.4.3 is compiled (workspace-hack) and behaves the same for cursor tokens (N1) | Use `hex`. The token format and its binding checks are unchanged. |
| Docstring literal scan | `cpg-extract/src/docstrings.rs` `literal_body` | `ruff_python_ast::str::{leading_quote, trailing_quote}` / `AnyStringFlags` (linked; L2) |
| Retry eligibility | `lctx-postgres/src/lib.rs:74-82` `retryable()` keys on raw SQLSTATEs | Express retry eligibility as a policy over `FailureClass` in `failure.rs`. **Refuted part of the lead:** `Error::class` (`generations/mod.rs:~123-139`) and `cpg-core` `classify` (`generation_read.rs:~100-117`) already delegate to `FailureClass::sqlstate`. Retry policy legitimately differs from class (57014 must not retry). |
| Unused native exports | `python/lctx_storage/src/serving.rs:999-1008` `tools`/`resources`/`schema` delegate to `wire::*` and have no consumer (L4 grep) | Delete. They are not a second authority. |
| Stale dependency | `python/lctx_mcp/pyproject.toml:8` `pyarrow==25.0.1`; no import in `python/` | Remove after confirming no transitive need |
| Capped `exists` | `kernel.rs:723-748`; no stated reason for not using `fused_binary_flip_op_with_limit` | State the reason in code now. The swap is deferred (§11) because it changes refusal thresholds. |

<a id="F10"></a>
### F10 · Stale authority text and trigger records

| Text | Owner to amend | Correction |
|---|---|---|
| ADR-0044 body: `tarjan_scc` order; D3 "condensation merges parallel arcs"; D1 never evaluated leiden-rs `infomap` | ADR-0044 Amendments (lifecycle metadata). The 2026-09-29 amendment already records `kosaraju_scc`, but it cites the retired ADR-0082. | Add that D3 holds only for `make_acyclic=true` (Tested P3, 2026-10-04): `false` keeps every arc and turns intra-SCC arcs into self-loops. This opens a pinned route for the Proposed evidence-preserving condensation. Record leiden-rs `compute_flow` as oracle-only (no stop report or work bound). |
| Forward plan §5 table: "PyStemmer, DataFusion `WITH RECURSIVE` (compile time), schemars \| Stage 4 adopt" | [Forward plan §5/§7](../../plans/behavioral-model-forward-plan_2026-09-24.md#7-deferred-each-with-a-trigger) | PyStemmer conflicts with ADR-0114 (Rust owns tokens); the route is a Rust Snowball stemmer at the §14.12 trigger. `WITH RECURSIVE` still has no recursion limit at DataFusion 55.1 (L1). Schemars is done. |
| B0 trigger and reason rows | Forward plan §7 rows, the `docs/pins.md` odis row, ADR-0044 amendment | Corrections: graphops "pulls petgraph 0.6" is true only for an optional feature. datafusion-tracing no longer brings OpenTelemetry; its deferral rests on "no consumer". The RSS-sampler rejection now rests on fit (memory-stats reads all of smaps). odis's "batch contracts" applies to basis and Titanic only; the concept iterator is lazy; production non-fit holds. Pyrefly `parse_parameter_documentation` is already consumed. |
| `python-analyzers` skill, library-context layer | Shared skill, labelled project layer (operator exception, 2026-10-02); a `library-research` worker under its maintenance guide | `show semantic` and `show migration` state old versions and "nothing fed from `SemanticModel`". Unused stamps list members now called. The "visibility-only patch" invariant no longer describes the Pyrefly fork (+348 lines in the solver). |
| Python analyzer migration plan §1 baseline (Pyrefly 1.3.1, Ruff 0.0.11) | Plan owner (code-facts coordinator) | Mark it historical or update it to the linked pins |
| §B3 / §15 | See [F06](#F06) | ADR plus DESIGN |

### Observations (not findings)

- **O1.** The first enriched loop is capped at 64 rounds (`enriched_production.rs:330`). Exhaustion produces explicit `ExecutionBoundary` rows whose reason does not name the cap. Bounding the loop by the count of with-statement occurrences, as the second loop does with `headers().len()`, makes it exact. Low.
- **O2.** `MIGRATOR.run(&pool)` can leave the advisory lock on a pooled connection after a `VersionMismatch` (L1; skill B024). Low impact for a one-shot CLI. Running migration on a dedicated, closed-after-error connection removes it.
- **O3.** The `wire.py` final-bytes mirror is the minimum bespoke code for §11.3: no FastMCP or SDK hook sees the final framed bytes, and byte-equality tests guard drift. The pins lag fastmcp 4.0.11 (security fixes) and mcp 2.3.0 (wire-byte changes). That is a pin-check finding only; an upgrade must re-run the transport byte tests.
- **O4. Strengths to preserve:**
  - pure, store-free model kernels;
  - charge-before-allocate admission;
  - explicit `Stop`/`Refused`/`KernelBoundary` outcomes;
  - nominal identities with private dense indices;
  - serde_arrow codecs with exact-schema checks;
  - iterative SCC (ADR-0052);
  - the `sql.rs` read-only gate;
  - the closed pushdown algebra in `generation_read.rs`;
  - independent oracles in `lctx-analytics` (fcars, odis, leiden-rs).

### Foundation verdicts

| Foundation | Verdict | Evidence |
|---|---|---|
| FP-01 Separation of concerns | **violated** (scoped) | `cpg-core` restates analysis publication sets ([F01](#F01)); otherwise mechanism→meaning direction holds |
| FP-02 Stable contracts | **violated** (scoped) | Identity depends on Arrow's `Debug` ([F04](#F04)); analyzer adapters otherwise absorb private APIs |
| FP-03 Composition | **violated** for S1/S2 | Stage composition needs restated lists ([F01](#F01)) |
| FP-04 Domain model and authority | **violated** | [F02](#F02), [F05](#F05), [F08](#F08)(b); the rest of the model governs behaviour |
| FP-05 Explicit structure | satisfied, with F02 panic note | Runtime checks enforce stage I/O; refusals are typed elsewhere |
| FP-06 Local reasoning | satisfied | Kernels testable without a store. F01 and F03 push validation to real-PostgreSQL runs (a cost, not a violation). |

## 8. Library fit and total complexity

| Capability and owner | Consumer | Candidates | Pinned semantic fit and gaps | Burden | Choice and reason |
|---|---|---|---|---|---|
| Stage ports and inventories (`lctx-model`) | Every stage; S1/S2 | Own derive; facet; frunk; bevy_reflect | Reflection crates need a hand registry for generic `Record` calls | Library: new proc-macro or runtime reflection. Own: about one `Domain`-sized derive. | **Extend own** ([F01](#F01)) |
| Grant closure | Every stage | Owner function; petgraph | Owner already exists | none | **Use own owner** ([F02](#F02)) |
| Canonical identity encoding | Model digest, mappings | KeySink; postcard; serde_json_canonicalizer; CBOR/bcs/borsh | No canonical Arrow-schema encoding exists. bcs has no floats; borsh and dcbor would be second authorities. | none | **Extend own** ([F04](#F04)); JCS deferred |
| Value rendering and labels | Retrieval, synthesis | Own `label()`/repr; askama; minijinja; strum; derive_more | Templates would not supply labels and would add engine output to identity | none | **Extend own** ([F05](#F05)) |
| Relational derivation | Model kernels | DataFusion joins; arrow-row/arrow-ord; Datalog | DataFusion lacks typed ids, charged refusal and spill. arrow-ord sorts are unstable. | Large (purity) | **Keep bespoke** in-model; amend docs ([F06](#F06)) |
| Container lookups and indexes | Model kernels | multi_index_map; indexmap; hashbrown | Order, charging and conflict semantics differ | none | **Extend own** ([F07](#F07)) |
| Bounded `exists` | Condition kernel | biodivine `fused_binary_flip_op_with_limit` plus `check_fused_binary_flip_op` | Equal results (P1, 4,490/4,490). The task-count preflight over-refuses less than the pair product. Changes refusal thresholds. | Requalify published boundaries | **Use pinned surface — deferred** (§11) |
| Simultaneous BDD composition | `substitution.rs` | biodivine `substitute` (single-variable, unbounded); OxiDD | No bounded simultaneous form; OxiDD has only manager-level capacity | — | **Keep bespoke** (library lacks the capability) |
| Summary fixpoint | `summary_production.rs` | ascent; datafrog; crepe | Time-only stop, hidden deltas, no charge-before-mutation (P4) | Large | **Keep bespoke** (ADR-0053; W12 trigger unchanged) |
| Keyed topological order | SCC schedule; stage sort | rustworkx-core `lexicographical_topological_sort` | Order-equivalent (P6), but returns a truncated `Ok` on cycles (P2) and is uncharged | rayon, ndarray, rand 0.10 | **Keep bespoke** |
| Weighted PageRank | Analytics | graphops `pagerank_weighted_run`; leiden-rs `compute_flow`; petgraph | graphops reports convergence but double-counts duplicate neighbours, has no work bound and needs caller dedup (P5) | New crate, rand family | **Keep bespoke**; leiden-rs stays the oracle |
| FCA with iceberg pseudo-intents | Analytics | odis; fcars | No crate combines iceberg pruning with interleaved basis enumeration under a per-step budget | — | **Keep bespoke** (ADR-0121 holds) |
| Evidence-preserving condensation (Proposed in ADR-0044) | Future consumer | petgraph `condensation(g, false)` | Keeps arcs and weights; intra-SCC arcs become self-loops (P3) | Pinned | **Use pinned surface at trigger** |
| Interval containment | `attachment.rs` | rust-lapper, iset, coitrees | Overlap queries only; no innermost containment or work count | — | **Keep bespoke**; proptest differential optional |
| PostgreSQL DDL and SQL | `ddl.rs`, services | sea-query (+`sqlx-utils`); DataFusion `Unparser`; sqlx `QueryBuilder` | sea-query lacks view, grant and schema builders; F4 not fired | DDL digest change → rebuild | **Defer** (F4) |
| Shadow schema diff | `verify.rs` | pgdiff, dibs, pg_query (PG17 grammar) | None runs inside a rolled-back shadow transaction or reports ACLs | — | **Keep bespoke** |
| Advisory locks | `locks.rs` | sqlx `PgAdvisoryLock` | No shared or transaction-scoped variants | — | **Keep raw SQL** |
| Serving drain | `runtime.rs` | tokio-util `TaskTracker` (needs feature `rt`) | Grants cross `spawn_blocking` and survive caller cancellation | One feature | **Defer** (no consumer) |
| Retry | `cache.rs`, `locks.rs` | backon | Two sites; committed-winner readback stays ours | New crate | **Keep bespoke** |
| MCP final-bytes admission | `wire.py`, `serving.rs` | FastMCP `ResponseLimitingMiddleware`; SDK hooks | Middleware is lossy and runs pre-serialization; no pre-write hook | — | **Keep bespoke** (§11.3) |
| Query embedding twin | `embedder.py` / `lctx-embed` | Move into the Rust service | Removes the parity corpus's second consumer. Changes the cdylib closure and §B14's "conformance-checked Rust and Python clients". | ADR | **Keep** (§B14); trigger in §11 |
| Public-surface and signature oracle | Test lane | Griffe 2.3.0 | Independent of Ruff, Pyrefly and ty. **Shares an extractor with the fastmcp gold skill (CI-12):** agreement must not corroborate gold-scored items. | Python dev-only | **Defer** (trigger in §11) |
| Docstring sections (Raises, Returns, Examples, NumPy) | Documentary evidence | `ruff_linter::docstrings` (crate-private) | Byte-ranged sections; needs a patch hunk | Rebase surface | **Defer** (PR0 trigger) |
| Library skills | Agents working on `lctx_mcp` and `lctx-embed` | `pydantic`, `vllm` shared skills | pydantic matches exactly; vLLM's 0.30.0 claims need checking against the custom 0.30.1rc1 build | Config only | **Adopt** (enable; vLLM with a pin-check note) |

### Decision index (inventory coverage)

| Items | Choice | Reason or trigger |
|---|---|---|
| M2-04, M2 §2, M4-09, M2-16 (codebook enums) | Extend own derive | F01; F05 labels |
| M1-06, M2-03 | Use own owner | F02 |
| M4-01 (column layout), M4-02 | Extend own | F03 |
| M2-05 (Debug parts) | Extend own | F04 |
| M2-05 (JSON bytes) | Defer | Trigger: serde_json upgrade, an out-of-workspace identity consumer, or a persisted cache keyed on a JSON identity |
| M2-13 | Extend own | F05 |
| M2-01, M2-02 | Extend own | F07 |
| M2-06 | Use compiled `hex` | F09 |
| M2-07 | Extend own (`DomainCode`/`DomainSum` emit schema) or schemars derive | Low; the exhaustiveness guard already exists |
| M2-08 | Keep | ADR-0073 closed-request contract |
| M2-09, M4-17 | Keep | ADR-0114; F13 not fired |
| M2-10 | Keep | No containment library; proptest differential optional |
| M2-11 | Keep | Typed stops and charging; `all_simple_paths` unbounded |
| M2-12 | Keep | Byte-exact windows |
| M2-14, M1-16 | Keep | get-size2 changes charged sizes and therefore admission thresholds |
| M2-15, M1-20 | Keep | nutype, garde and bon triggers not met (N1 §2.4) |
| M2-16 (non-codebook `ALL` arrays) | Keep (optionally strum, already compiled) | Wire names must stay explicit |
| M1-01–M1-03 | Keep | ADR-0053; P4; W12 trigger unchanged |
| M1-04, M1-05 | Keep | P2/P6 truncation hazard and footprint |
| M1-07 | Keep, except reuse `Diagram::support()` in `local_semantics.rs:800-830` | Existing kernel |
| M1-08 | Keep; O1 | — |
| M1-09 | Keep | ADR-0044 D4 ordering facts hold |
| M1-10 | Keep | Stop report and work bound; graphops not pinned |
| M1-11 | Keep | ADR-0121 |
| M1-12, M1-13 | Keep | Library already used / no deterministic exact crate |
| M1-14 | Defer | Trigger: a dense index crosses a module (typed-index-collections; index_vec is compiled) |
| M1-15 | Use pinned surface — deferred | §11 |
| M1-17, M1-18, M1-19 | Keep | arrow-row stable sort is correct; own validator framework |
| M3-01 | Keep | F08 non-finding |
| M3-02, M3-03, M3-04, M3-05 | Use pinned surface / cross-check | F08 |
| M3-06 | Keep | No ty option for runtime TYPE_CHECKING (TY03/TY18) |
| M3-07 | Use pinned surface (token comments in `cpg-flow`) | Low |
| M3-08 | Keep own detector; consume Ruff export ranges | F08 |
| M3-09 | Keep; Ruff `str` helpers | F09 |
| M3-10, M3-11, M3-12, M3-16, M3-17 | Keep | Adapter or domain; no lossless library route found |
| M3-13 | Use `pep508_rs::PackageName` normalization in place of the hand `normalize` | Already a dependency; low |
| M3-14, M3-15 | Keep (`is_identifier` from `ruff_python_stdlib` for M3-15) | §14.6 observation-preserving parsing |
| M4-03 | Keep (prefix-scoped visibility rule) | — |
| M4-04, M4-05, M4-10, M4-19, M4-20 | Keep | §8 rows |
| M4-06, M4-07 | Keep; optional `plan_to_sql` for whole scans | Two decoders follow two drivers |
| M4-08 | Keep | Scan admission |
| M4-11 | Defer `TaskTracker` | No consumer |
| M4-12 | Own consolidation (three option builders, two `verify-full` checks) | Low; figment and config rejected |
| M4-13 | Own (F09) | — |
| M4-14 | Keep | — |
| M4-15 | Keep | §B14 |
| M4-16 | Keep | O3 |
| M4-18 | Keep | — |
| N2: Glean, SCIP, tantivy, pg_textsearch, pg_trgm, HF tokenizers, rust-stemmers, camel-case split | Defer | §11 triggers |
| N2: stack-graphs, LSIF, libcst, jedi, python-pkginfo, uv crates, `bm25` crate; cognee, neo4j, jena, deltalake, native-solver and symbolica skills | Reject | N2 §8 reasons (archived, second parser or engine, weaker contract, unstable API, hashed tokens, §B10/§B11) |

## 9. Alternatives and tradeoffs

**Stage ports ([F01](#F01)):**

| Alternative | Propagation and local reasoning | Authority and composition | Test boundary | Machinery and risk | Decision |
|---|---|---|---|---|---|
| Current: hand lists plus runtime checks | One change, many edits; detected at run time | Stated once, restated many times | Real PostgreSQL needed to catch omissions | Low machinery, high repetition | Revise |
| Own port-set derive with a visitor | One declaration; compile-time propagation | One authority; port sets compose | Compile errors plus existing runtime checks | About one derive's size; async-visitor premise | **Recommended** (Proposed) |
| Runtime reflection (facet) | Lists become data | A registry duplicates the type mapping | Runtime only | Pre-1.0 proc-macro | Rejected |
| Simplest viable: named list macros for the shared publication set only | Removes the 16+ restatements; writers still restate | Partial | Unchanged | Minimal | Acceptable first step if the derive spike fails |

**Relational compute ([F06](#F06)).** DataFusion joins in `cpg-core` would remove nothing semantic. Each kernel would gain a split owner and a runtime dependency for testing, and lose typed ids and charged refusal. The in-model `Rows` route with F07 helpers is the simplest viable design. The premise that would reverse this is an admitted stage input that cannot be held under the attempt budget, which would require streaming or spilling relational execution.

**Summary engine (S4).** Ascent can express the recursion and the Pareto lattice (P4). Charging, the deterministic work bound, residual publication and refusal with a reason would have to be rebuilt around it, so the substitution does not preserve the contract. ADR-0053's trigger (two or more channels sharing a recursive rule) has not fired; M1 found no second rule family.

## 10. Verification and uncertainty

| Claim | Label and date | Basis | Gap |
|---|---|---|---|
| F01 repetition and runtime-only reconciliation | Implemented, 2026-10-04 | 66 exports counted; `catalog/build.rs` and `catalog_core.rs` lists read; `stages.rs:2129/2236` read | Local re-expansion counts are M2's heuristic (±) |
| F02 twelve copies, one panic | Implemented, 2026-10-04 | grep count 12; `local_semantics.rs` and `production.rs` read | **Unresolved:** whether name-only copies mishandle vocabulary epochs relative to the owner. Settle by comparing owner and copy grant sets for each stage on the current model. This affects F02's severity, not its diagnosis. |
| F03 four derivations | Implemented | `ddl.rs`, `serving_shape.rs` read | none |
| F04 `Debug` identity; Arrow change | Implemented; Interface-checked (arrow-schema 59.3.0 source; 56.2.0 per N1) | `model.rs:311`, `mappings.rs`, `field.rs` read | Arrow 60's `DataType`/`Field` Debug not checked |
| F05 served `Debug` text | Implemented | `retrieval/build.rs`, `assertions.rs`, `mappings.rs:93` read | Retrieval-quality impact unmeasured (performance and quality are not gates; §14.12) |
| F06 no DataFusion joins | Implemented (absence over `cpg-core/src` and `lctx/src` grep) | `sql.rs` read | Run-time SQL through `lctx query` is out of the claim |
| F08(c) type-parameter coverage | Implemented (skip read) | `cpg-flow/src/lib.rs` | **Unresolved:** whether skipped type-parameter definitions are recorded as coverage or silently absent downstream (CI-04). Settle by tracing a PEP 695 fixture's reaching rows. |
| L3 P1–P6 library behaviour | Tested, 2026-10-04 | `l3-probe/`, `cargo run --offline --release`, rustc nightly 2026-09-28, isolated crate | Fixture-scoped; no timing |
| Port-set derive feasibility (async visitor) | Proposed | — | Compile spike (below) |
| Remedies F01–F09 | Proposed | — | Each closes on its stated verification |

Unexamined breadth, not defects: most `normalized/*`, `structural/*` and `native_requests/*` bodies (header-level only), service files in `lctx-postgres`, and Python tests beyond the headers.

## 11. Authority changes and dispositions

No active plan owns these findings. Under binding §4, each has either a recommended route or a Deferred row below. On transfer to a plan, the plan's findings table becomes the single disposition owner.

| Required change | Decision route and responsible component | Source findings | Disposition location | Closure evidence or trigger |
|---|---|---|---|---|
| Port-set derive; publication set as one type; `DomainCode` labels, `ALL` and schema | create-plan package; implementation inside ADR-0085 ("typed operation ports"); `lctx-model-macros`, `lctx-model` stages, `cpg-core` adapters, `cpg-extract` providers | F01, F05 (labels), F09/M2-07 | New plan (to be created) | S2 trace: one edit, compile-time propagation, hand lists deleted |
| Grant-only closure operation; name index on `ValidatedModel` | Implementation; `lctx-model::dependency_closure` | F02 | Same plan or a standalone change | 12 copies deleted; typed refusal control |
| `physical_columns` owner; one quoting helper | Implementation; `lctx-postgres::generations::ddl` | F03, F09 | Same plan | All consumers call the owner |
| Explicit identity encoding; known-answer vectors | Implementation (identity migration, rebuild); `lctx-model` `model.rs`, `serving/mappings.rs`, `identity.rs` tests | F04 | Same plan | Vectors independent of `Debug` |
| Owned value renderings; `RENDER_VERSION` bump | create-plan package, sequenced with §14.12; `lctx-model` retrieval and synthesis | F05 | Same plan | Retrieval and synthesis consume one renderer; no `{:?}` in corpus text |
| DataFusion's role restated | **ADR** superseding the DataFusion-compute clauses of ADR-0085 and ADR-0073, plus amendments to DESIGN §B3, §15.1, the §15 layer table, §15 Compute and the binding §1 §B3 row; root/design owner | F06 | ADR | ADR accepted and sections amended. Revisit when an admitted stage input exceeds the attempt budget on a real library. |
| `Rows` helpers | Implementation; `lctx-model::normalized::rows` | F07 | Same plan | `need` copies deleted |
| Ruff `Export`/`Branch` lowering or removal; version/platform cross-check; `custom_builtins`; type-parameter scope ranges | [Code-facts coordinator §7](../../plans/code-facts-expansion-plan_2026-10-03.md#7-current-checkpoint-and-next-action) (analyzer migration owner) or create-plan; `cpg-extract`, `cpg-flow`, Ruff patch | F08 | Coordinator §5/§7 on transfer | Attributed observations; disagreement rows; explicit type-parameter coverage |
| F09 items | Implementation; the components named in F09 | F09 | Same plan | Deletions and uses landed |
| ADR-0044 amendment; forward plan §5/§7 rows; pins odis row; migration plan §1 | Owner edits (ADR lifecycle amendment; plan owners) | F10 | Those owners | Text matches §8 and L1–L3 evidence |
| `python-analyzers` skill project layer | `library-research` worker under the skill's maintenance guide (operator exception, 2026-10-02) | F10 | Shared skill store | `show project`/`migration`/`unused` restamped |
| Enable `pydantic` and `vllm` skills | Config change in `.config/library-skills.toml`; vLLM with a pin-check note | §8 | Operator / root | Skills linked |

**Deferred rows** (owned here until a plan or trigger takes them):

| Deferred item | Scope consequence | Trigger that reopens it |
|---|---|---|
| biodivine `fused_binary_flip_op_with_limit` (+ `check_fused_binary_flip_op` preflight) for `exists` | Kernel keeps the pair-product preflight, which over-refuses relative to task counts (P1) | Published boundaries show `WorkPreflight` refusals on a real library, or the next condition-kernel change. Adopt as a qualified policy change. |
| JSON-byte identity canonicalization (RFC 8785 or KeySink) | De facto canonical form | serde_json upgrade; an out-of-workspace identity consumer; a persisted cache keyed on a JSON identity beyond the current embedding spec |
| petgraph `condensation(g, false)` | — | A consumer for the evidence-preserving condensation (ADR-0044 Proposed) |
| sea-query `sqlx-utils` binder / `plan_to_sql` for whole scans | `format!` SQL with validated identifiers | F4: substantive dynamic SQL expressions |
| tokio-util `TaskTracker` | Hand drain stays | A second drain consumer, or grants become tracked tasks |
| Query embedder moved into Rust (removing the Python twin) | Parity corpora keep two consumers (§B14) | A parity-corpus drift failure, or a second required change to the Python client |
| Griffe 2.3.0 dev-only public-surface oracle | Public surface has Pyrefly-only plus own-detector checks | Next `public_records` qualification or PR0 resumption. Must never count as corroboration of gold-scored items (CI-12). |
| Ruff docstring sections (patch hunk) | NumPy-style and multi-paragraph parameter docs stay a boundary; Raises/Returns/Examples not harvested | A frozen PR0 task needs documented exceptions, returns or docstring examples, or a NumPy-style library is selected |
| Camel-case and acronym token split; Rust Snowball stemmer | Lexical channel misses `streamable http transport` → `StreamableHttpTransport` | §14.12 dev set frozen and a task misses for this reason (a policy-identity change) |
| Pyrefly Glean collector | Typed-receiver attribute references stay partial | A served "where is this used" answer graded partial for typed non-`self` receivers (§13) |
| SCIP export | — | A named external client consumer (§14.11) |
| pg_trgm, pg_textsearch, tantivy | — | F13 (a named fuzzy, phrase or measured lexical gap) |
| HF `tokenizers` (Rust) | `/tokenize` stays the admission authority | Admission must run with the service unavailable |
| Typed dense-index crates | — | A dense index crosses a module boundary |
| `lexical.rs` replacement by Ruff or ty | Bespoke recognizer stays | Ruff exposes multi-candidate or `LOAD_NAME` semantics, or ty's builtins and class-body fallback become available in the index in both profiles |

## 12. Architectural judgment and decision

| Judgment | Verdict | Scenario evidence and scope | Required action |
|---|---|---|---|
| A1 Localize change | **violated** | S1/S2 (F01), the closure-policy change (F02), the physical-layout change (F03), S3 identity coupling (F04). S4 and S5 substitutions are local. | F01–F04 |
| A2 Encode domain meaning explicitly | **violated** (scoped) | The grant rule is authored 13 times (F02); identity has no declared canonical form (F04, DP-04 MUST); value meaning is re-rendered per consumer (F05). The rest of the model governs behaviour. | F02, F04, F05 |
| A3 Extend through composition | **violated** for S1/S2 | New analytics need restated machinery rather than one declaration (F01). The new own derive has a credible, current need. | F01 |

**Bounded decision: Revise.** In-scope A1–A3 violations, G1 (F02), G7 (F06/F10) and G8 (F09) failures, and a DP-04 MUST gap (F04). None is a correctness failure of current outputs: every reconciled duplicate fails closed, and identity drift is conservative.

**Enclosing architecture: needs revision in the named areas.** The architecture's library choices and its pure-model, PostgreSQL, kernel decomposition are accepted for the library-leverage question, at Implemented and Interface-checked strength with fixture-scoped Tested library probes. This review does not certify Phase 5 qualification or any release qualification.

| Priority | Change and responsible component | Findings | Closure or trigger |
|---|---|---|---|
| 1 | Grant closure to its owner (`lctx-model::dependency_closure`) | F02 | Copies deleted; typed refusal |
| 2 | Port-set derive and named publication set (`lctx-model-macros`, `cpg-core`) | F01 | S2 trace |
| 3 | Explicit identity encoding (`lctx-model`) | F04 | Known-answer vectors |
| 4 | Owned value renderings (`lctx-model` retrieval and synthesis) | F05 | No `{:?}` corpus text |
| 5 | ADR plus DESIGN for DataFusion's role | F06 | ADR accepted |
| 6 | Physical column owner (`lctx-postgres` DDL) | F03 | One derivation |
| 7 | Analyzer facts and cross-check (code-facts coordinator) | F08 | Observations recorded |
| 8 | `Rows` helpers; F09 items; F10 text | F07, F09, F10 | Deletions and amendments |

**Prerequisite order versus priority.**
- F02 comes first. It is small and independent, and a grant-only owner operation is what F01's generated stage inputs should call. Doing F01 first would generate code against the copies.
- F05's labels and F01's derive share `lctx-model-macros` work and should be planned together.
- F04 and F05 change persisted bytes (model digest, corpus text) and force a rebuild under ADR-0078. They belong **between** qualification runs, not inside the in-flight enrichment full gate. F05 must also be ordered against the §14.12 dev-set freeze.
- F06 and F10 are documentation-only and can land at any time.
- F01 and F02 touch many `stage()` functions that the concurrent enrichment repair is still editing. Plan them after the enrichment checkpoint to avoid conflicting edits.

**Next decision and owner.** The root agent should run `create-plan` for one package covering F01, F02, F04, F05, F03 and F07/F09, with a compile spike of the port-set visitor (async loading on `nightly-2026-09-29`) as its first step. In parallel, draft the F06 ADR. F08 goes to the code-facts coordinator, and the skill and text corrections in F10 go to their owners.

## Reviewer follow-up requests

**Probes I would want run** (none is required for the diagnoses):
1. A compile spike of the port-set derive with an async load visitor in `cpg-core` on the pinned nightly. This settles F01's remedy maturity; the diagnosis does not depend on it.
2. For each stage on the current model, compare the grant set from `DependencyClosure` with the hand-copied closure, focusing on vocabulary epochs. This settles F02's severity.
3. Trace a PEP 695 fixture's flow and reaching rows to see whether skipped type-parameter definitions are recorded as coverage (F08c, CI-04).

**What I could not settle:**
- Whether the name-only closure copies already produce different grants from the owner for vocabulary relations (F02 severity).
- Whether skipped PEP 695 type-parameter definitions surface as explicit unknown coverage (F08c).
- How Arrow 60's `Debug` output differs. This doesn't affect F04's diagnosis or correction, which removes the dependency entirely.
- The retrieval-quality effect of F05 and of the camel-case gap, which by design waits on the §14.12 freeze.
