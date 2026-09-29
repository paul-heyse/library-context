# The codebase as a data model: semantic relations, projections and operators — target review

## 1. Scope, outcome and coverage

| Field | Value |
|---|---|
| Subject | Code at `35afc09` (PR5 journeys and coarse rebuilds), with `crates/` and `python/` unchanged through `fedd4a0` (PR5 qualification, docs only). Source inspected 2026-09-29. Architectural collection at `fedd4a0`. The operator's external recommendation, `docs/external-review-fully-semantic-graph-data-model.md` (written at `5626941`/`7152f21`), is the prompt. It is treated as evidence, never as authority. |
| Standard | [Core 3.0](../design_principles/core/design-principles.md), [template 3.0](../design_principles/core/design-review-template.md), [code-intelligence 1.1](../design_principles/profiles/code-intelligence/principles.md), [repository binding](../design_principles/binding/library-context.md); `design-review` and `design-review-code-intelligence` skills. |
| Tier · purpose | **Design · target.** Judged against the best architecture for the §14 API/evidence product and the retained behavioral research, not against conformance to the current design. |
| Reviewer · date | Lead reviewer (Claude session; not the author of the reviewed code), 2026-09-29. Read-only reconstruction agents covered each layer and the library capabilities. A fresh `design-reviewer` subagent challenged the draft; its challenge and the revisions it caused are recorded in §10. The **proposed target in this review is authored by the reviewer and therefore stays Proposed**; it is not accepted by this document. |
| Maturity and outcome | Design phase with a single operator, no production users, and aggressive pivots permitted. This decision should let feature work (configuration, relationships, predicates, explanations) proceed on semantic relations that are defined once, rather than on re-interpretations local to each analysis. |
| Supported scope | The whole pipeline: extraction-side derivations (Stage C/D), the graph catalog, projections and analytics (Stage E), flow, conditions, summaries, behaviors and discharges, catalog, associations, selection and retrieval, stage orchestration and reuse, bundle, PostgreSQL and native serving. CI fact families: call resolution, flow, conditions, summaries, behaviors, catalog contracts, associations, retrieval units, findings. Served answers: all ten MCP tools. Workloads: extraction, analysis, answering, change (release, analyzer, compiler revision) and evaluation isolation. |
| Exclusions | PR0/PR6 execution, eval/heldout content, embedding accuracy, ANN, performance campaigns. No product code was changed. |
| Expected changes | The scenarios in §5 come from the active product queue (§14.4 configuration, §14.5 relationships, §14.7 predicates), from the frozen PR0 development tasks that bear on them (D06/I04 prefix→exposed name, D07 tag-filter configuration, U01/U02 evidence explanation), and from the variation axes (provider, analyzer, release, compiler revision). |
| Baseline | A strong relational foundation: 201 `table!` contracts, DataFusion derivations, generated validation, Delta snapshots, typed wire contracts. On top of it, **semantic decisions are repeatedly re-derived per consumer**: call edges in 8 sites, argument binding in 4, transfer strength in 8, refusal priority in 4, discharge validity in 4, conditions in 2 representations, serving schemas up to 5 times. Graph machinery is nominal: one declared projection, and an adapter that runs no algorithm. |
| Method and coverage | **Examined:** the sources listed in the [evidence README](../evidence/2026-09-29_semantic-data-model/README.md): four read-only layer reconstructions, two library-capability studies, three executed probes (P0, P3, P8), and read-only pilot queries (by the reviewer and by the challenger). Every citation that supports a finding was re-read by the reviewer at the grain used. **Resolved during the challenge:** none of the 2,107 pilot discharges is proved (1,990 are open with `call_transfer`). Decorated registrations do produce artificial `for-decorated-target` sites, which reach 4,458 scenario associations (challenger query). **Not examined:** extraction front-end internals below `cpg-extract`'s outputs, Python ranking internals beyond identity, evaluation assets, MCP rendering of behaviors. No integrated test ran; the probes and queries are the only executions. |

This review is evidence, not a status register. Finding disposition moves to the [forward plan §6](../../plans/behavioral-model-forward-plan_2026-09-24.md#6-findings-disposition) once the operator accepts a route (§11).

## 2. Responsibilities, dependencies and semantic ownership

| Component | Coherent responsibility and hidden decisions | Consumer contract | Allowed dependencies | Expected reason for change |
|---|---|---|---|---|
| `cpg-schema` | Arrow contracts (`table!`), codebooks, identity recipes, the graph registry (`EdgeSource`/`NodeSource`), projection spec, condition kernel, summary contracts, wire contracts, selection classifier, validation rules | Typed rows, generated rules, wire schemas | `arrow-*`, `blake3`, `biodivine-lib-bdd`, `serde`/`schemars` | A new relation, codebook value, identity or contract |
| `cpg-extract`, `cpg-flow` | Provider observations (Pyrefly/Ruff 0.0.11 parse; ty 0.0.14 flow index joined by byte span) | Raw family batches | Pinned front ends | Analyzer upgrade; new fact family |
| `cpg-core` | Attempt orchestration (`attempt::finish`), derivations, flow model, behaviors, catalog/surface/evidence derivations, stage cache/rebuild, bundle, validation, publication | Published snapshot; bundle; stage receipts | `cpg-schema`, `lctx-analytics`, DataFusion, Delta | Every semantic change touches it today (see §5) |
| `lctx-analytics` | Pure passes over Arrow: Pass A/B/C, communities, ranking, FCA/RCA, kNN, SCC schedule, finite summaries, completion, evaluation | Arrow in, Arrow out; no store | petgraph, fixedbitset, leiden-rs | New analysis; engine change |
| `lctx-postgres`, `lctx_semantics`, `lctx_mcp` | Generation-pinned serving, hydration, packets, selection, journeys, native value paths, MCP transport | Ten tools plus one resource; typed refusals | SQLx/pgpq, PyO3, FastMCP | A new tool, section or serving relation |

The dependency direction is sound: orchestration → capability owners → schema. The defects are **semantic ownership** inside that direction.

| Concept | Semantic authority today | Independent re-derivations (evidence) | Consequence |
|---|---|---|---|
| Call edge (site→target with modality/origin/phase/caller) | None. `graph::owner_of` and the accepted lists in `projection.rs:120` are shared by only some consumers | 8 sites: projection `projection.rs:148-179`; flows `flows.rs:80-112` (lists copied); summaries `summaries.rs:121-132`; `model_applications` `behavior.rs:2069-2087`; local seeds `behavior.rs:2806-2880`; usage `usage.rs:62-82`; **catalog evidence `evidence.rs:177-183`**; a test | F02: served origin hidden and admission policy divergent; different answers for property reads, `getattr`, `__new__`, async, candidates |
| Argument → formal binding | `summary_contract::bind_arguments` (`summary_contract.rs:566-676`) | 3 SQL binders: `flows.rs:214+`, `behavior.rs:2092-2190`, `behavior.rs:2901`; a cross-check requiring agreement, `call_binding.rs:271-280` | Precision loss where they disagree; every binding change is edited four times |
| Place / port | None | 7 encodings: node-hex `source_key` (`flow_model.rs:1733`), name-based `InputPath`/`OutputPath` (`models.rs:372-474`), rendered `summary_flows` paths (`finite.rs:910-914`), mixed `negative_premises.place_key`, `surface.rs` private field-link proof, frontier subject, `value_flows.place` text | F06: no access paths on reads; name-versus-node mismatches |
| Transfer kind and composition | None; `max` over the derived `Ord` of codebook order (`flow_model.rs:942`) | 8 interpretations (`FlowTransfer::of`, the `v2_flows` class, Pass B `ValueClass`, behaviors text, seed SQL filters, `ModelTransferKind`, `SummaryFlowKind`, store association) | F04: served "computed" for a proved identity |
| Condition | BDD `Diagram` decides verdicts | Legacy DNF computed in parallel and served as text (`condition_kernel.rs:794-987`, `:951-953`); atom hash recipe in 5 production sites | F05 |
| Refusal / obligation | None | Boundary priority in 4 policies (`finite.rs:235`, `:307-324`, `discharge.rs:40`, `behavior.rs:126`); discharge validity in 4 checks; 2 coverage producers | F07 |
| Derived result / derivation | `FINDING_STATUS`, `FINDING_METHOD`, `FindingKey` shared | Emission repeated in 7 analyses; ≥8 typed step tables with no uniform conclusion→derivation→premise index | F08 |
| Serving representation | `table!` for 18 catalog files (`catalog::serving_files`) | 49 of 72 serving files hand-declared (schema, SQL, key); PostgreSQL DDL frozen and hand-edited; FK lists diverged; codebook mapping stated in 3 places | F10 |

**CI fact and fidelity table** (condensed; the full per-relation tables are in the evidence README sources).

| Fact family or relation | Provider and revision | Fidelity | Coverage and unknowns | Identity | Consumers |
|---|---|---|---|---|---|
| Call targets / site targets | Pysa collectors in Pyrefly fork 1.3.1+patch | resolved (modality definite/candidate/potential; origin incl. `synthetic_model`) | `resolutions` + `boundaries`; attribute/artificial sites have no `resolutions` row | `edge_id = H(edge, kind, src, dst, discriminator)` | 8 consumers (above), 2 served |
| Flow facts (uses, definitions, reaching, values, tests) | ty 0.0.14 semantic index, joined by span | extracted/derived | narrowing constraints never read; attribute/subscript places have no parity rules | `(module, span)` plus ty-internal indices in synthetic atoms | flow model, conditions |
| Conditions | ty reachability TDDs → BDD kernel | derived | kernel boundaries (`KernelBoundary`) | `H(condition-bdd, root)`; DNF `H(condition, text)` | verdicts; **served as DNF text** |
| Value flows / behaviors | flow model + Pass B + discharges | derived; conservative | `unknown` + `boundary_reason` | `behavior` Rust-only recipe | `get_operation` (behavioral profile) |
| Finite summaries | `lctx-analytics::summaries` | derived; conservative | 3,705 boundaries vs 23 flows (pilot) | `SemanticFlow` key; `summary_id` witness | `inspect_value_paths` |
| Catalog contracts | surface/catalog derivations | extracted/derived | `default_state ∈ {…, optional_expression_unavailable, unknown}` | table keys | selection, packets |
| Catalog field links | `surface.rs` constructor/storage/reader proof | derived; recognized records only | pilot 432 `declared_parameter`, 136 `exact_storage`, 23 `exact_reader`; plain classes produce none | link id | selection Configuration contexts, packets |
| Associations / scenarios | `cpg-core::evidence` | derived; `basis` resolved/candidate/ambiguous | basis dropped in 4 projections (F03) | association id | packets, selection, retrieval |
| Findings | Passes A–C, communities, ranking, FCA/RCA, kNN | exact/heuristic by `FINDING_STATUS` | per method | `FindingKey`; label omitted for Pass C/FCA members | Related support, briefs |

## 3. Contracts, constraints and testing boundaries

| Contract | Consumer expectation | Enforcement | Failure behavior | Isolated verification |
|---|---|---|---|---|
| `table!` row contracts | One schema, key, checks, decoder per table | Macro generation; `INSTA_UPDATE=no` snapshots | Build/validation error | Yes: contract tests |
| Graph registry (`EdgeSource`) | Every edge kind declared once; generated SQL and rules | Generated `key:`/`typed:`/lineage rules; edit guards | Publication refusal | Yes: `graph_shapes` |
| Projection spec | Declared universe, arcs and policies (CI-05) | Digest; **policies are free text**, SQL hand-written | None for policy drift | Adapter test only |
| Selection tri-state (§14.7) | Supported / contradicted / unresolved with closure | Rust classifier; oversized domains refuse with a typed error (`selection.rs:37-47`) | **Unknown default state evaluates as false** (F01). A joint-work budget hit becomes `Unresolved` under `RequireCompatible` but carries no budget reason (`selection.rs:250-257, 326-330`) | Classifier tests exist; no unknown-state control |
| Condition verdicts | BDD decides; five verdicts | Kernel caps and boundaries | Served text may be `over_budget` (F05) | Kernel tests |
| Stage cache (ADR-0081, PR5/F04) | Reuse never changes output | Recompute and compare canonical batches | Mismatch is a miss | `tests/rebuild.rs` |
| Serving files | Served relation equals canonical relation | Runtime name-based import checks | Adding a Delta column is silently not served for hand-written files (F10) | Bundle test pin |

**Absence lattice.** `resolutions`, `boundaries` and the five verdicts distinguish not analysed, unresolved and refused well at the fact level. Three places collapse this:
- **Selection (F01):** an unknown default state is evaluated as a definite `false`.
- **Coverage (F07):** `summary_origin_coverage` is produced (73,636 pilot rows) and validated, but nothing consumes it for conclusions yet. Its owner stages it for the S4/S5 research sequence (behavioral analysis §9.9).
- **Summary cost cap (F07):** a witness over the cost cap is dropped silently and reported as `call_transfer`. It is not reported as a proof limit.
- **Unresolved attribute and artificial sites:** these have no `resolutions` row, so they never enter the projection's unresolved relation (`projection.rs:180-187`, property arcs hard-code `has_unresolved_remainder = false` at `:163`).

## 4. Composition and execution

| Stage | Semantic inputs/outputs | Owning mechanism | Reuse boundary | Limits/determinism |
|---|---|---|---|---|
| A–B extraction | pinned env → raw batches | `cpg-extract`, `cpg-flow` | run identity | pilot 36.3 s of 473.9 s |
| C/D derivation | raw → 22 derived tables incl. `nodes`/`edges` (2,355,231 rows, 38% of canonical bytes, never served) | `Derived::sql` via `derive.rs` | none (recomputed) | canonical sort; 3.0 s for `nodes`+`edges` |
| Behavior/flow | flow facts → value flows, behaviors, summaries | `flow_model` (32.2 s), `behavior`, `lctx-analytics` | none | explicit caps; two fixpoint engines with incompatible merge/cap semantics |
| Catalog | facts → contracts, associations, domains | `catalog`, `surface`, `evidence` | stage cache recomputes then compares (ContractNormalization 58.6 s "Recomputed") | pure after loading |
| E analytics | projection/SQL → findings | `analyze.rs` | none | seeds recorded; lineage gaps (F08) |
| Validation, publication | all → snapshot | `validate` (177.2 s), `delta` | none | full validation per attempt |
| Serving | bundle → PG → tools | `lctx-postgres`, native | `generation_digest` | no prepared-object cache; `get_operation` 15+2L–20+2L sequential round trips |

**Stage ordering is implicit.** `attempt::finish` (`attempt.rs:696-1709`) hard-codes the sequence:
- The writer of each table is implicit.
- A table read before its writer runs returns a typed empty batch, not an error.
- Stage F writes `brief_status` back into `catalog_members`.
- PR5's `Stage` dependencies are receipt labels only. The CLI calls them "explanatory, never consulted to admit reuse".

**Analysis records** (CI slot 4, condensed):

| Question | Projection | Method | Model/status | Budgets | Output linkage |
|---|---|---|---|---|---|
| Transitive delegation (Pass A) | invocation spec (only registered spec) | own BFS over adapter | exact over projection | depth/vertex/arc budgets → typed stop findings | witnesses with `edge_id` |
| Argument forwarding (Pass B) | `flows` SQL (lists copied from the spec) | own worklist `(callable, formal, source, suppressed)` | conservative | caps | witnesses |
| Handoffs (Pass C) | `handoffs_sql` | group-by | exact | — | concept incidences; label omitted from `MemberKey` |
| Communities | own dense index over spec arcs + co-use | leiden-rs RBER, 10 seeds | heuristic, governed | fixed | least site per pair |
| Direct usage / PageRank | own dense index | counts; own weighted PageRank | structural / heuristic | iterations and residual recorded | none (Related order) |
| FCA/RCA | concepts SQL + spec arcs + Pass C | own NextClosure (fcars oracle) | exact over context | — | no projection/flows digest on RCA |
| Summary schedule | `summary_call_arcs` (own SQL) | `kosaraju_scc` + own callee-first order | exact | — | no invocation row or digest |
| Finite summaries | ~30 relations | SCC-local worklist, nondominated `(depth, cost)` frontier | conservative | depth 8, cost 64, pair cap 1M | ordered steps sealed by digest |
| Source reach | use→def reverse dependencies | capped fixpoint (`flow_model.rs:957-1025`) | conservative | 1M cap → widen to unknown | none (no witness) |

## 5. Change and failure scenarios

The current route is traced in code at `35afc09` (Interface-checked); the target route is Proposed.

| # | Scenario and trigger | Current: owner, independent semantic edits, hidden knowledge | Target route (Proposed) | Evidence |
|---|---|---|---|---|
| S1 | A new call-resolution provider or origin (for example ty-resolved attribute calls), or disagreement with Pysa; a new alternative downgrades a unique resolution | Enters `call_targets`/`site_targets`. Three consumers admit it by the copied lists; **five decide independently**. The `synthetic_model` precedent is already inconsistent (F02). Unique-resolution consumers (`target_count=1 ∧ complete`) must each be checked | Provider adapter plus origin code; **one owner for the call relation** (named policy relations) with declared admission policies. `conflicting`/`unresolved` states come from `resolutions` once | §2 table; A1 §2.1 divergences |
| S2 | Configuration influence for §14.4 (PR0 D06/I04/D07), plus an explanatory slice (U01/U02) reusing it | **Partly expressible.** For recognized records, catalog field links give option→field→reader (pilot 432/136/23 links). Not expressible for plain classes: P0's `read_timeout` records `config → return` with no field path and there are no field links. Guarded per-branch supply through a summarized callee is also not expressible, because summaries compose only Parameter→Return (P0 `fetch`). A second consumer would re-read the catalog field-link proof, three place encodings and four binders | First generalize the catalog field-link relation into place-based `transfers`. Behavioral per-branch supply then adds composition over `calls`, bindings and BDD conditions, delivered as a two-consumer slice where the explanation consumer does not reinterpret anything | P0 raw output; pilot counts |
| S3 | New §14.7 predicate `accepts_keyword` through `**kwargs` forwarding | Classifier in `selection/catalog.rs`. Forwarding facts are name-based (Pass B); unmapped arguments are a deferred trigger | One predicate over bindings and transfers, with the closure scope taken from the relation's declared coverage | §7 trigger list |
| S4 | New §14.5 relationship: facade delegation, registered-then-invoked callable | P0: the function facade composes. The method facade stays `unknown` (`override_dispatch`, which is correct open-world behavior). Registered-then-invoked is absent | Transfers with dispatch alternatives as an OR over targets, each with its target condition | P0 |
| S5 | New transfer kind (control influence, mutation), or a change in condition semantics | 8 transfer interpretations; DNF+BDD; 5 atom-hash sites; ty-internal indices in synthetic atom ids | One transfer algebra module; BDD canonical with rendering derived; atom id = evaluation occurrence | §2 table |
| S6 | A decorator model changes the effective signature | PR2 centralized decorator normalization for catalog and synthesis, but the 4 binders do not read it | Binder reads the one effective-callable relation | A2 §4.1 |
| S7 | Analyzer upgrade, new release, dependency library (pydantic/starlette summaries), release diff | Content ids are rerun-stable, but ty upgrades can change synthetic atom ids on unchanged source. No stable cross-release symbol key | Occurrence-keyed atoms; stable symbol keys for entities (SCIP-style descriptor) when the first cross-release or dependency consumer is admitted | A2 §5 |
| S8 | Catalog rebuild after an association rule change; compiler-developer iteration | The stage cache recomputes 58.6 s and compares, and publication validation derives the contracts again. The whole compile is 473.9 s; validation is 177.2 s. The association key does include its code (`LCTX_ASSOCIATION_SOURCE_DIGEST`), and snapshots record the full `compiler_digest`. Only `producers.build_digest` omits the association code | Declared stage writers and read-before-write refusal. Skip-on-key reuse stays deferred: validation re-derives contracts anyway, and ADR-0081's measured-workload trigger has not fired | timings; A3 §4; challenge |
| S9 | Serve-time "why is this unresolved?", or a projection-backed question | Support scattered over ≥8 typed step tables; obligations encoded as reasons in ~6 tables; `nodes`/`edges` not served | Generated `derivations`/`obligations` index served; set-based hydration from a generation-scoped prepared object | P8 volume |

**Gate checks within scenarios:**
- **Heuristic reaching a claim:** not found. Communities feed only Related support, and ranking only orders (CI-09).
- **Presentation budget change:** leaves summary semantics unchanged (`witnesses_omitted` is separate). Selection's joint budget yields `Unresolved` but does not say that a budget caused it.
- **Local operator test:** `lctx-analytics` and the catalog pure derivations are testable without store or PostgreSQL.
- **Tracing a served claim:** behaviors → discharges → `summary_flows` → steps → facts is closable, except that the served condition is display text only.
- **A module full of unresolved references:** unresolved call sites are explicit; unresolved attribute/artificial sites are not in `resolutions`.
- **Evaluation run:** n.a. No evaluation asset was touched, and S2 is not tied to a scored PR0 failure.

## 6. Correctness and fidelity gates

| Gate | Verdict | Own evidence or scope reason | Required action |
|---|---|---|---|
| G1 Authority | **fail** | Call-edge meaning (8), binder (4), transfer (8), refusal (4), discharge validity (4), condition (2), serving schema (up to 5 per relation); FK lists already diverge (`rules::REFERENCES` vs `serving_projection::foreign_keys`) | F02, F04–F07, F10 |
| G2 Semantic fidelity | **fail** | F01 unknown→false; F03 basis dropped; F02 origin dropped from served support; F04 identity labelled computed (latent on the pilot) | F01–F04 |
| G3 Validity | unresolved | Publication validators are strong, but three gaps remain. Stage read-before-write yields a typed empty batch, not a refusal. The native and SQL discharge validators are weaker than the producer. PostgreSQL enforces a composite FK (`catalog_field_links → catalog_parameters`) with no `rules::REFERENCES` counterpart, so a canonical row can publish and then fail import | F07, F10, F11 |
| G4 Hidden behavior | pass (inspected) | No ambient analyzer input found. The FastMCP registration recognizer reads a recorded release fact, so it is an explicit input (F13 concerns its governance, not hidden input) | — |
| G5 Consistency and recovery | **fail** | Silent truncation: `Frontier::insert` returns `false` both over the cost cap and when dominated (`worklist.rs:35`), and the caller continues (`finite.rs:2419`). A capped proof therefore reports as `call_transfer` rather than a proof limit. DataFusion recursive CTEs (4 sites) run with no memory pool | F07 |
| G6 Transformation and reuse | unresolved | Stage reuse is correctness-first (sound). Three identity gaps remain: `MemberKey` omits labels for Pass C/FCA and no validator recomputes finding ids; ty-internal indices enter atom ids; `producers.build_digest` omits association code (DP-21 labelling) | F05, F08, F11 |
| G7 Truthful capability claims | **fail** (documentation) | ADR-0044 says filtered and reversed views are used and never copies; the adapter uses none. DESIGN §3.8 and `graph.rs:1368` say projections select by `edge_kinds`; nothing reads it | F09 |
| G8 Library leverage | pass | Our own PageRank is justified: petgraph's `page_rank` ignores weights and differs by up to 3.2e-2 (P3). Our own simultaneous bounded substitution is justified: biodivine's `substitute` is single-variable and unbounded. `kosaraju_scc` is used, not the recursive Tarjan, which overflows at a 100k-deep path (P3). Repeated dense-index code is a local-copy issue, not missing library leverage | — |
| CI-G1 Fidelity | **fail** | F01 (unknown served as absent) and F03 (candidate and ambiguous links served as resolved) reach default-profile tools. F04 is a behavioral-profile relabel with no pilot instance | F01, F03 first; F04 |
| CI-G2 Evidence closure | unresolved | Associations cite existing facts but hide origin (F02). Usage patterns do not cite the edge behind the chosen site. Served conditions lack `condition_id` | F02, F05 |
| CI-G3 Evaluation integrity | pass (inspected paths), with a risk | No evaluation path reaches compiler inputs, and the PR0 tasks were read only to map scenarios. **Risk:** the pilot-specific registration recognizer (F13) sits inside the product that PR6 compares | F13 |

## 7. Findings and applicability

IDs are stable. §12 orders the work: first the served fidelity breaches (F01, F03, F05, F02, F13), then the latent F04, then architectural barriers and repeated semantic ownership.

<a id="F01"></a>
### F01 — An unknown default state is served as a definite mismatch

**Principles:** CI-04, CI-06, DP-02 · CI-G1, G2. **Interface-checked.** The pilot counts are Measured, 2026-09-29.
- **Evidence:**
  - `selection/catalog.rs:897-899` evaluates `ParameterDefaultState` as `Some(p.default_state == word(state))`.
  - The vocabulary includes `unknown` and `optional_expression_unavailable` (`cpg-schema/src/catalog.rs:288-295`), produced for provider-generated constructors (`cpg-core/src/catalog.rs:429-438, 1146`).
  - `classify` turns all-negative claims over a closed domain into `Contradicted/ClosedAbsence` (`selection.rs:125-129`).
  - The pilot has 764 of 5,139 parameters in these two states.
- **Consequence:** `find_operations`/`compare_operations` can report a closed absence ("no default of this state") for a dataclass field whose default merely has unrecoverable text.
- **Also (challenge):** `DefaultMatchState` includes `FactoryExpression` (`wire/requirements.rs:91-99`), which a parameter never produces. That makes `factory_expression` always a definite `false`. `Unknown` is also queryable as if it were a value. §14.4 confirms this behavior is a defect against the design.
- **Correction:** tri-state evaluation in the classifier owner. Return `None` for `unknown`, and for `optional_expression_unavailable` against literal-valued states. Decide explicitly whether `unknown` may be requested as a state.
- **Closure:** provider-constructor and factory-default fixture controls that return `Unresolved`.

<a id="F02"></a>
### F02 — The call relation has no owner; served associations hide the call's origin

**Principles:** FP-04, DP-01, CI-02, CI-11 · G1, G2, CI-G2. **Implemented; divergences Interface-checked; pilot exposure Measured by the challenger's read-only queries, 2026-09-29.**
- **Evidence:**
  - Eight sites decide what a call edge is (§2).
  - The catalog evidence derivation keeps every `CallTarget|SiteTarget` edge regardless of origin (`cpg-core/src/evidence.rs:177-183`).
  - It labels the association `invokes` with basis `resolved_target` whenever the fact is definite (`:805-835`).
  - `AssociationSupport` has no origin field (`cpg-schema/src/evidence.rs:172-185`).
  - The invocation projection admits only analyzer assertions (`projection.rs:121`).
  - On the pilot, the synthetic arcs that reach associations are Pysa `for-decorated-target` artificial calls (`pysa_map.rs:184`). There are 4,458 scenario associations, 170 of them `resolved_target`. No other synthetic site kind reaches an association. A fixture-level `getattr` artificial site shows the same path is open for modeled accesses.
- **Character:** decorator applications are real runtime invocations and registrations, which §14.5 wants kept. So this is **not a relabelled relation**. The defect is that the origin is hidden and the admission policy differs from the projection's without any declaration. `usage.rs:62-82` has the same undeclared policy.
- **Consequence:** an agent cannot tell a decorator-registration invocation from an ordinary call, and S1 (a new origin) must be decided independently at five sites.
- **Correction:**
  - One owner for the call relation. Express it as **named policy relations** over `call_targets`/`site_targets`/`resolutions`, using the existing `relations!`/`owner_of` pattern, not as a new materialized table.
  - Each consumer class declares its admission policy.
  - `AssociationSupport` carries the origin and an implicit/registration flag.
  - Delete the independent SQL.
  - Do **not** filter registrations out by the invocation policy.
- **Closure:** S1 traced as one policy edit; a decorator-registration fixture stays `invokes` with its origin visible.

<a id="F03"></a>
### F03 — Association basis is dropped when associations are composed or projected

**Principles:** CI-02, DP-07 · CI-G1. **Implemented. Pilot exposure Measured by the challenger's read-only queries, 2026-09-29:** 399 deployment `suggests`/`same_document_passage` associations are admitted as `SourceDeclaration`; there are 1,864 `ambiguous_textual_mention` associations; and 49,895 scenario `invokes` associations (69%) are `candidate_targets`.
- **Evidence:**
  - Config and task associations are attached to every member of any scenario association, including `candidate_targets` (`evidence.rs:1077-1131`).
  - Deployment declarations accept `suggests`/`same_document_passage` as `SourceDeclaration` (`selection/catalog.rs:663-692`).
  - Scenario and source domain contexts use associations of any basis (`catalog_domains.rs:223-256`).
  - `retrieval_subjects` has no basis column.
  - Storage evidence is labelled `reader` (`catalog.rs:1361-1371`; the vocabulary has no storage role).
- **Consequence:** candidate or ambiguous links count as resolved in selection and retrieval.
- **Correction:** basis and origin are required, propagated columns of every association projection; add a storage role.
- **Closure:** candidate-only fixtures remain `Unresolved`.

<a id="F04"></a>
### F04 — Transfer kind has no algebra; a proved identity is labelled "computed"

**Principles:** DP-02, FP-04, CI-02 · G2. **Tested for the canonical `behaviors` row on the P0 fixture, 2026-09-29.** MCP rendering was not traced. **Latent on the pilot:** none of its 2,107 discharges is proved. The behavioral profile only.
- **Evidence:**
  - Transfer strength is interpreted in 8 places.
  - Composition is `max` over the order of an append-only codebook (`codebook.rs:1403-1407`, `flow_model.rs:942`), so **any appended transfer kind silently becomes the lattice top** (S5).
  - `behavior.rs:967-977` stamps `computed:<byte>` whenever the intra-procedural flow went through a call. `codebook.rs:1245-1247` confirms that `computed` is an affirmative claim.
  - A successful discharge then sets the verdict to Established (`:1259-1272`) without revisiting the value.
  - On P0, `facade(x) = _inner(x)` is recorded as "returns x, computed, established", although its summary proves an identity (`path_depth 1`).
- **Consequence:** once discharges are proved, agents would be told a value is transformed when it is proved unchanged. S5 requires 8 coordinated edits.
- **Correction:**
  - One transfer algebra: kinds plus an explicit composition table. An open or unknown call is an obligation, not a kind.
  - Behaviors render from the final composed kind.
  - **Trap:** `flow_behavior_id` hashes `value` and `transfer` (`cpg-schema/src/behavior.rs:1823-1836`), and discharges are looked up by the pre-discharge id (`cpg-core/src/behavior.rs:1259`; rule `semantic:call-transfer-discharged`, `rules.rs:3469-3482`). The correction must therefore re-key consistently or take the rendered value out of identity.
- **Closure:** the P0 control recorded as `unchanged`, with discharge lookup intact.

<a id="F05"></a>
### F05 — Conditions have two representations, and agents receive the non-authoritative one

**Principles:** DP-01, DP-04, CI-02 · G1, CI-G2. **Implemented. Measured on the pilot, 2026-09-29:** 38 Conditional and 90 Unknown served behaviors carry the condition text `over_budget`.
- **Evidence:**
  - `BoundedCondition` computes the legacy DNF in parallel with the BDD (`condition_kernel.rs:794-987`). `encode()` returns the DNF (`:951-953`).
  - The DNF becomes `over_budget` past 16×8 terms independently of the BDD (`condition.rs:36-40, 560-571`).
  - Served `behaviors` has `condition` text and no `condition_id` (`cpg-schema/src/bundle.rs:298-322`).
  - The atom-id recipe `IdHasher::new("bdd-atom")` is written in 5 production sites.
  - Synthetic atom identity includes ty-internal indices (A2 §5).
- **Consequence:** a served Conditional fate can read `over_budget` while the decisive BDD exists. A ty upgrade can change atom ids on unchanged source (S7).
- **Correction:** implement and accept **ADR-0024's existing display decision**. The BDD is canonical, and the display takes capped paths from `sat_clauses` with a truncation marker (ADR-0024 lines 59 and 74). In addition:
  - Serve `condition_id` together with the condition catalog.
  - Adopt one atom-identity function keyed by the evaluation occurrence.
  - The internal DNF readers to migrate are:
    - the provider-condition reparse (`flow_model.rs:1360`);
    - the boundary-condition id, which hashes the legacy text (`condition_kernel.rs:955-961`);
    - atom lookup by encoded text (`condition_kernel.rs:517`).
  - No MCP or synthesis path parses DNF; native parsing exists only in developer smoke probes.
- **Closure:** S5 traced as one kernel edit; a ty-upgrade identity control; no `over_budget` text beside a decided BDD.

<a id="F06"></a>
### F06 — There is no place/transfer vocabulary, so configuration and relationship features cannot compose

**Principles:** FP-01, FP-03, FP-04, FP-05 · A1, A3. **Tested for the gaps on P0.**
- **Evidence:**
  - Places have 7 encodings (§2).
  - Summaries are Parameter→Return only; argument sinks and read access paths are not represented.
  - P0: `read_timeout` records `config → return` with no field path, so option→field→reader cannot be joined.
  - P0: `fetch` supplies `timeout`/`fallback` to `transport` only as through-call rows with condition `true`. The guarded per-branch supply the external review describes is not derivable, although `select_timeout`'s guarded summary exists.
  - The `surface.rs` field-link proof is a catalog-private place model. It gives option→field→reader for recognized records (pilot: 432 `declared_parameter`, 136 `exact_storage`, 23 `exact_reader`). Plain classes produce no field links (P0: no field-link rows).
  - Controls held: `title` never reaches `timeout`; the two identity calls never cross; the unresolved `transport` leaves `fetch`'s return `unknown/call_transfer`.
- **Consequence:**
  - A plain-class configuration or a behavioral per-branch question (S2) needs yet another place encoding.
  - S4 and S6 each need their own binder reading.
  - Any explanation consumer must reinterpret the catalog proof and the behavioral flows separately.
- **Correction** (A3 is violated here only for plain-class configuration and behavioral composition):
  - A `Place` (root plus bounded access path with an unknown suffix).
  - A `transfers` relation: owner, in/out place, kind, condition, context, provenance class, status/obligation, support.
  - One binder producing `call_bindings` (n-ary: site × target alternative × signature variant × actual → formal).
  - **Order:**
    1. Generalize the existing catalog field-link relation into place-based `transfers` first.
    2. Add authored models, Pysa TITO rows, finite summaries and intra-procedural value flows as further provenance classes ("models as data"); disagreement stays visible.
    3. Deliver behavioral per-branch composition with the first product consumer that needs it, as a two-consumer slice.
- **Closure:** a P0 rerun answers option→field→reader for a plain class and per-branch supply, and the controls still hold.

<a id="F07"></a>
### F07 — Refusal, obligation and coverage semantics are distributed; one cap truncates silently

**Principles:** CI-04, CI-08, DP-01, DP-12, DP-16 · G1, G3, G5. **Implemented; confirmed by the challenge. The boundary/coverage divergence claim is Interface-checked only and did not reproduce.**
- **Evidence:**
  - Boundary-reason priority is ranked in 4 policies (`finite.rs:235-246`, `finite.rs:307-324`, `discharge.rs:34-42`, `behavior.rs:126-135`).
  - Discharge validity is restated 4 times. The native (`lctx_semantics/src/lib.rs:1260-1290`) and SQL (`rules.rs:3469-3505`) versions are weaker than the producer: they do not check that a proved origin has no boundary row. This is a G3 detection gap, not a served defect.
  - **Silent truncation (G5):** `Frontier::insert` returns `false` both for cost over `MAX_PROOF_COST` and for a dominated witness (`worklist.rs:35`). The caller `continue`s (`finite.rs:2419`), and the fallback reason becomes `call_transfer` (`finite.rs:298-303`), not a proof limit.
  - The four DataFusion `WITH RECURSIVE` sites run without a memory pool.
  - The selection joint-work budget yields `Unresolved` with no budget reason. `evaluation_complete: true` is truthful, because oversized domains refuse with a typed error.
  - P0's `read_timeout` field read is refused as `unsupported_control_flow`.
  - `summary_origin_coverage` has no conclusion consumer. Its owner stages it for S4/S5.
- **Consequence:**
  - A new refusal reason or negative-claim rule must be edited in 4 places.
  - A capped proof is indistinguishable from an open call.
  - Callers cannot tell budget from evidence when a result is unresolved.
- **Correction:**
  - One obligation/refusal owner in `cpg-schema`: reasons, priority, and a record type.
  - An `obligations` relation replacing the reason encodings.
  - Distinguish the cap from dominance in the frontier.
  - Add a selection budget reason to the §14.7 envelope.
  - Set a DataFusion memory pool.
  - Derive validity checks from the producer contract.
  - The consume-or-delete decision on coverage goes to the research-sequence owner (forward plan §3.0.1).
- **Closure:** one reason addition traced; a cost-cap control reports a proof limit.

<a id="F08"></a>
### F08 — Derived results have no shared contract; derivations are typed but unindexed; lineage has gaps

**Principles:** CI-01, CI-11, DP-04, DP-21 · G6, CI-G2. **Implemented.**
- **Evidence:**
  - The finding-emission sequence is repeated in 7 analyses.
  - `MemberKey` omits labels for Pass C and FCA members, although it is documented as every column but the weight (`findings.rs:619-620`; `pass_c.rs:211-221`), and no validator recomputes finding ids.
  - Support lives in ≥8 typed step tables with digests. OR exists only as separate rows.
  - The kNN, RCA, seed-selection and SCC-schedule outputs record no input invocations.
  - P8: all support rows total 158,765 (7.0 MiB, 2.2% of canonical bytes).
- **Consequence:** an explanation consumer (S9) must learn every step table. Reverse dependency ("what depends on this fact") is not answerable.
- **Correction:**
  - One derived-result emitter.
  - `analysis_invocations.input_invocations`.
  - A finding-id recompute rule.
  - A **derivation index generated as views**, not a materialized table: `derivations` (conclusion, derivation, rule, invocation) and `derivation_premises` (ordinal, premise kind/id, role). Each is declared per sealed step table in a registry, in the same generation pattern that `EdgeSource` uses for the graph catalog (ADR-0067). The typed step tables and their digests stay the premise payloads. Alternatives remain separate derivations of one conclusion. Finite summaries already keep acyclic ordered witnesses, so no SCC-level support record is added: it would create circular explanations.
  - Delivered with its first consumer, S9.
- **Closure:** S9 traced over the index; the finding-id rule rejects a mislabelled member.

<a id="F09"></a>
### F09 — The projection layer is nominal, and design claims exceed it

**Principles:** CI-05, FP-05, DP-06 · G7. **Implemented.**
- **Evidence:**
  - One registered spec, with free-text policies and hand SQL (`projection.rs`).
  - The adapter runs no petgraph algorithm (`lctx-analytics/src/graph.rs:204-212`).
  - Ranking, communities and the SCC schedule build their own dense indices (4 constructions). Syntax child indexes are rebuilt 6 times.
  - `edge_kinds` has no reader.
  - ADR-0044 claims view usage that does not exist.
- **Consequence:** a new topology analysis (S5-type projection) repeats index, validation and ordering code. The documentation misstates the capability.
- **Correction:**
  - **Now:**
    - Correct the ADR-0044 and §3.8 claims.
    - Replace the free-text policies with typed enums.
    - Share one dense-index/adjacency helper among the four same-domain copies.
    - Give `edge_kinds` its reader or retire it.
  - **When a second topology consumer is admitted:** a projection runtime generated from relation declarations. It would provide a shared node dictionary, canonical sorted adjacency in both directions with arc ids, and petgraph views.
  - It is used **only by topology analyses**. Relational questions stay joins (CI-07).
  - `nodes`/`edges` stay materialized. ADR-0067's trigger is unmet (derivation takes 3.0 s), and they are the reference universe for 107 generated and declared rules. Their 38% storage share is recorded as an observation, with a trigger of storage or validation pressure.
- **Closure:** a new projection added by declaration only.

<a id="F10"></a>
### F10 — The serving representation is a second hand-written authority

**Principles:** FP-04, DP-01, DP-06 · G1. **Implemented.**
- **Evidence:**
  - 49 of 72 serving files re-declare schema and SQL (`bundle.rs:113-495`). `behaviors` and `summary_flows` have up to 5 schema statements each.
  - `retrieval_units` has no `table!`.
  - PostgreSQL DDL is generated once, then frozen and hand-edited (migration `0012:111`), with no drift test.
  - The two FK lists already disagree. PostgreSQL enforces a composite `catalog_field_links → catalog_parameters` key that has no `rules::REFERENCES` counterpart, so a canonical row can publish and then fail import (G3 ordering).
  - Codebook mapping is stated in 3 places.
  - More than 15 hand relation-name inventories.
- **Consequence:** a served column addition needs 3–6 edits and is silently dropped if one is missed.
- **Correction:** extend the existing derived path (`catalog::serving_files`, 18 files) to every served relation. Derive DDL, FK lists, codebook maps and inventories from the declarations, with a DDL drift test.
- **Closure:** add a served column by declaration plus migration only.

<a id="F11"></a>
### F11 — Pipeline composition is implicit; producer identity is mislabelled

**Principles:** FP-05, DP-21 · G3, G6. **Implemented; timings Measured by the PR5 qualification run** (`build/pr5-qualified/behavioral.log`, a historical receipt).
- **Evidence:**
  - Stage order is hard-coded. Which stage writes each table is implicit, a read-before-write returns an empty batch, and Stage F writes back into `catalog_members` (§4).
  - `semantic_digest` excludes `evidence.rs`/`catalog_domains.rs`, which produce canonical tables. The `build.rs:101` comment wrongly calls them serving-only.
    - **Compensated for reuse:** the association stage key adds `LCTX_ASSOCIATION_SOURCE_DIGEST` (`stage_cache.rs:312-315`); reuse recomputes and compares; snapshots record the full `compiler_digest`.
    - **Remaining gap:** `producers.build_digest` (`analyze.rs:185, 208`), which is a DP-21 labelling defect.
- **Consequence:**
  - A missing or misordered writer is silent.
  - Published producer rows understate the code that made the association tables.
- **Correction** (a refactor inside ADR-0081, with no admission change):
  - A declared finite stage table: inputs from the loaders' declared scans, declared outputs, the scheduler derived from it, and an unwritten declared output treated as an error.
  - Correct the `build_digest` coverage and the comment.
  - **Skip-on-key reuse stays deferred.** Publication validation re-derives the catalog contracts anyway. ADR-0081's measured-workload trigger has not fired. Per-stage code identity would rest on hand-kept source lists, the kind of mechanism this review finds drifting.
- **Closure:** injected read-before-write and missing-writer controls refuse; producer identity changes when association code changes.

<a id="F12"></a>
### F12 — Serving rebuilds immutable state per request and chains round trips

**Principles:** DP-10, DP-20. **Implemented; triggered improvement, not a correctness defect.**
- **Evidence:**
  - `PreparedCatalog` is rebuilt from full-generation loads on every selection, compare and page.
  - `catalog_record` makes 9+2L sequential round trips; `compare_operations` up to about 58; `search_evidence` makes N+1.
  - The native files are stored twice and loaded whole.
- **Correction:**
  - A generation-scoped immutable prepared object (`Arc` built once per `generation_digest`; no general cache library needed).
  - Set-based hydration.
  - Moka only under its §14.11 trigger, for request-level caching.

<a id="F13"></a>
### F13 — A pilot-specific recognizer inside the compared product; ranking parameters outside the profile digest

**Principles:** CI-09, CI-12, CI-13 · CI-G3 (risk). **Implemented.**
- **Evidence:**
  - FastMCP registration recognition activates only when a distribution string equals `"fastmcp==4.0.5"` (`surface.rs:641-644, 236-241, 313-337`). It is not a named model.
  - This is not a reuse-key gap: `surface.rs` is inside the semantic digest, and the activation string is an input fact.
  - BM25 parameters are Python literals outside `Policy` (`retrieval.py:25-60`; `profiles.rs:14-43`).
- **Consequence:**
  - Pilot-specific compiler behavior shapes the default product that PR6 compares against Context7, which undermines attribution and a second-library claim.
  - Lexical-parameter changes are invisible in served identity.
- **Correction** (priority 1, before PR6):
  - Express the recognizer as a pinned, named authored model in the models catalog, with its identity recorded (models as data).
  - Digest the lexical parameters in the retrieval policy.

**Foundation verdicts.**
- **FP-01 (separation of concerns):** satisfied at crate level; violated for semantic concepts (F02, F06).
- **FP-02 (stable contracts):** satisfied for Arrow/wire contracts; unresolved for projection and transfer contracts.
- **FP-03 (composition):** violated for plain-class configuration and behavioral composition (S2, S4); satisfied for recognized-record configuration and for Arrow/DataFusion/Delta.
- **FP-04 (one authority):** violated (F02, F04, F05, F07, F10).
- **FP-05 (explicit structure):** violated (F09, F11).
- **FP-06 (local reasoning):** satisfied for `lctx-analytics` and pure catalog derivation; violated where a change needs knowledge of 4–8 private re-derivations.

**Observations that are not findings:**
- The owner-only caller in `summary_call_arcs` drops module-level callers but cannot change function ordering. No served effect was found.
- Stale "definite only" comments on direct usage (`codebook.rs:1036-1038`, `findings.rs:695-696`, `analyze.rs:848-849`).
- The method facade's `override_dispatch` unknown is correct open-world behavior (P0).
- Three served-path gaps from reconstruction A3 §2.3 (Interface-checked). Disposition: route them to PR5 follow-up usability in forward plan §6.2 as bounded repairs, not target architecture:
  - Browse silently excludes members with unknown ownership.
  - `EmptyUnderCoverage` is asserted without association coverage; its reason text is honest.
  - Search reports `lexical-only` without the reason.
- `nodes`/`edges` hold 38% of canonical bytes and are not served (P8). They are kept (F09).

## 8. Library fit and total complexity

| Capability and owner | Consumer | Candidates | Pinned semantic fit and gaps (evidence) | Burden | Choice and reason |
|---|---|---|---|---|---|
| Multigraph topology (`lctx-analytics` projection runtime) | Pass A, ranking, communities, SCC schedule, future projections | petgraph 0.8.3 `Graph`, `Csr`, `StableGraph`; own CSR | `Graph<(),u32,Directed,u32>` keeps parallel arcs, 8 B/node and 20 B/edge. `Csr` rejects duplicate pairs and has no incoming traversal (P3). Views compose with Tarjan, Kosaraju, `toposort`, `has_path_connecting`, `simple_fast` (P3) | already pinned | **Keep petgraph `Graph`** with arc-id weights; add precomputed sorted adjacency for repeated traversals; reject `Csr` |
| SCC, schedule | summaries, stage table | petgraph `kosaraju_scc`, `TarjanScc`, `condensation`; rustworkx-core 0.18.1 `lexicographical_topological_sort` | Tarjan is recursive and overflows at a 100k path (P3 control). `condensation` keeps only one weight per pair. rustworkx reproduces the existing callee-first order exactly, but adds 7 crates and mandatory rayon | own order is ~90 lines, tested (ADR-0052) | **Keep `kosaraju_scc` + own canonical order; reuse it to order the stage table.** Reject `condensation` for evidence-bearing quotients (ADR-0044 already says so); rustworkx-core not justified |
| Dominators / control dependence | only if D1 selects a CFG | petgraph `dominators::simple_fast` on `Reversed` | Works on views; needs one root; O(V²); hash-ordered iterator (P3) | pinned | **Defer:** control influence comes from ty reachability conditions keyed by evaluation occurrence; a CFG projection only for claims those cannot express |
| Typed dense indices | projection runtime | typed-index-collections 3.5.0; cranelift-entity 0.136 | TiVec/TiSlice has no dependencies and gives typed binary search. cranelift-entity pins wasmtime, breaks monthly and truncates in release builds (L1) | small | **Defer.** The §14.11 trigger is *distinct* index domains inside one kernel. The four copies are the same domain, so share one helper instead. Reject cranelift-entity |
| Set states and masks | projections, FCA | fixedbitset 0.5.7; roaring 0.11.5 (already in the lock) | FixedBitSet equality includes capacity; union has no changed flag. roaring lacks `Hash`/`Ord` | none/wrapper | **Keep fixedbitset;** roaring only on measured sparsity |
| Conditions | condition kernel | biodivine-lib-bdd 0.6.3; OxiDD; z3 | `exists`/`var_exists` unbounded, but a bounded `fused_binary_flip_op_with_limit` exists. `substitute` is single-variable and unbounded, so our own bounded simultaneous substitution is justified. `cmp_implies` gives the lattice order. `from_bytes` panics, so persist node tables, not bytes (L1) | pinned | **Keep biodivine as the only canonical representation** (F05); bounded existential elimination for callee-local atoms |
| Recursive rules / provenance | transfers composition, derivations | ascent 0.8.1; datafrog 2.0.1; DataFusion `WITH RECURSIVE` | ascent accepts a BDD-valued implication lattice with bounded OR and `unknown` as top (P3 on a cycle), but its only budget is wall-clock (non-deterministic refusal), it has no provenance, and it needs one variable set per program. datafrog cannot hold BDD values. DataFusion recursion allows one self-reference, is single-threaded, and has no memory pool configured (L2) | ascent new crate | **Do not make ascent the semantic core.** Native SCC worklists remain. Ascent stays conditional for recursive multi-relation rules over finite domains (§14.11 trigger unchanged). DataFusion `UNION` recursion only for linear set closure, **with a memory pool set** |
| Incremental reuse | stage table | declared finite stage table; salsa 0.28.2; differential dataflow 0.25.1 | Salsa cycles work, but the dominant change is compiler code, which salsa does not version. Persistence is keyed on query layout. The version is frozen by ty. DD adds about 10 crates for a batch workload. Delta CDF does not fit append-per-attempt (L2) | — | **Declared finite stage table (own code), with recompute-compare reuse kept.** Skip-on-key deferred (F11). Salsa conditional (unchanged trigger); DD and CDF rejected |
| Artifacts / interchange | stage artifacts, bundle, projections | arrow-ipc 59.3 | `FileDecoder` zero-copy; `lexsort_to_indices` unstable, so it needs unique keys (L2) | pinned | **Keep;** canonical adjacency from sorted Arrow columns if persisted |
| Serving cache | prepared objects | `Arc` + generation key; moka 0.12.16 (transitive) | Moka capacity is approximate and caller-weighted; `try_get_with` coalesces initialization (L2) | new direct dep | **`Arc` per generation now;** moka only on its trigger |
| Independent oracles | transfer/condition controls | python-oracles skill (Pysa TITO, CrossHair, sys.monitoring + Hypothesis) | The existing Pysa TITO control covers one shape | dev-time | **Use** for the transfer relation's independent controls (CI-12) |
| Graph DB, egg/egglog, IFDS frameworks | none | — | No consumer; a second authority (CI-13) | — | Reject |

## 9. Alternatives and tradeoffs

**Proposed target (Alternative F, relation-centric).** The data model is the declared relational contract. Every semantic question has one owner: a declared relation, or a named policy over relations. Graphs are one execution mechanism, used where topology is the question.

Layers:
- **L0 attributed observations:** unchanged.
- **L1 normalized semantic relations:**
  - entities, with stable symbol keys when a cross-release or dependency consumer needs them;
  - occurrences, owned by the Pyrefly/Ruff 0.0.11 parse; ty facts attach by span;
  - `places`;
  - the call relation, as named policy relations over `call_targets`/`site_targets`/`resolutions`;
  - `call_bindings` (n-ary);
  - `resolutions`.
- **L2 derived semantic relations:**
  - `transfers` (models as data, starting from the catalog field-link relation);
  - canonical BDD conditions;
  - `obligations`;
  - a derivation index generated as views over the sealed step tables;
  - findings via one emitter with input lineage.
- **L3 product relations:** catalog contracts, predicates, associations with basis and origin, retrieval units. These consume L1/L2 only through declared relations.
- **L4 serving projections:** derived mechanically from the declarations.

Mechanisms:
- named semantic policies in `cpg-schema`, as the `owner_of` pattern generalized;
- shared analysis components (SCC schedule, named deterministic budgets, one obligation policy, witness selection);
- a declared finite stage table;
- a projection runtime only when a second topology consumer is admitted.

Three graphs stay distinct: program projections, the derivation index, and the stage table. Of the external review's four constructions:
- **Incidence:** exists as n-ary relations.
- **Quotient:** exists as SCC/community membership. It becomes a projection only for a named consumer.
- **Overlay:** would be the projection runtime's typed arc sources.
- **Product/state:** stays implicit in worklists.

**Two tracks.**
- **(i) Consolidation.** Behavior-preserving: clean output equals rebuilt output, apart from the fidelity repairs.
  - one owner for the call relation's policies;
  - one binder;
  - the transfer-algebra module;
  - BDD-canonical conditions (ADR-0024);
  - one refusal/obligation policy;
  - the derived-result emitter and lineage;
  - derived serving files;
  - the declared stage table.

  Consolidation has named consumers today: the duplicated decisions in §2. It can proceed now.
- **(ii) New vocabulary:** `places`/`transfers` beyond the catalog generalization, the derivation index and the projection runtime. Each is delivered with the first product consumer that needs it (PR0/PR6 tasks D06, I04, D07, U01, U02 are candidates), as a two-consumer slice where the second consumer does not reinterpret anything.

**Cost is substantial, not moderate** (Proposed; unmeasured). It means:
- the new relations and policies;
- rewriting Pass B, the flows, behaviors, summary seeds, discharges and native FORMAT admission onto them;
- derived serving;
- migrations.

That is why the vocabulary track is consumer-gated.

Normative schemas and tables belong in DESIGN through the ADR, not here.

| Alternative | Change propagation and local reasoning | Semantic authority and composition | Test/substitution boundary | Total machinery and operational risk | Evidence, decision and revisit |
|---|---|---|---|---|---|
| Current baseline | S1: 5 independent edits; S5: 8+2+5; served column: 3–6 | G1/CI-G1 fail (F01–F10) | Good for pure passes | Low new machinery, high duplication | Revise |
| **Proposed (F, relation-centric hybrid)** | S1: 1 policy edit; S2/S4: one new composition over existing relations; S5: one module; S8: declared writers | One owner per semantic question; typed step payloads kept | Operators remain Arrow-in/out; P0 shapes as known answers | Substantial but consumer-gated. Consolidation now; new vocabulary with product consumers; no new engine | **Proposed.** Revisit if the P0 rerun cannot express per-branch supply, or if generated derivation views prove too costly to query |
| A (external review as written) | Similar, but a generic operator framework and fixed-point driver | Right direction; composition centralized | — | A shared driver is **not justified**: only 2 of 12 engines iterate to a fixpoint, and they conflict on merge, cap semantics and key/witness split (A2 §2). Infrastructure for four constructions exceeds consumers | Adopt its relations, derivations and composition rules; reject the universal driver and projection-for-everything |
| B (Datalog/ascent core) | Rules become declarative | One rule base | Tests by rule | Deterministic budgets impossible (wall-clock); no provenance; one variable set; rewrite of Tested Stage-3 semantics (P3, summary-engine-comparison) | Reject as core; conditional niche |
| C (salsa semantic database) | Fine-grained reuse on input change | Queries as owners | — | Misses code-change invalidation; version frozen; persistence layout-bound (L2) | Reject; §14.11 trigger unchanged |
| Simplest viable (consolidate only) | Fixes S1/S5/S8 | Removes duplicates in existing shapes | same | Lowest | **Adopted as F's first track.** It is insufficient alone only for plain-class and behavioral per-branch features (F06), and that remainder is consumer-gated |
| D (differential dataflow), E (graph DB) | — | E is a second authority | — | Batch workload; heavy | Reject (one line each, §8) |

**External-review claim ledger.** Detailed per claim in the evidence README.
- **Confirmed:** E1–E4, E7, E9, E11, E12, E14.
- **Refined:**
  - E5: the policies are free text.
  - E6: `Techniques` flags, not the analytics config, separate the methods.
  - E8: refusals collapse to one reason, and field/captured origins are never candidates.
  - E10: narrowing constraints and structured places are lost.
  - E13: native artifacts are behavioral-only, stored twice.
  - E15: hydration batches keys but runs sequentially.
  - E16: Stage E interleaves SQL reads.
  - E17: generation identity is sound for serving, too coarse for compile.
- **Contradicted by evidence:** the recommended generic fixed-point driver and the "four constructions as infrastructure" both fail DP-16 against current consumers.
- **Missed by the external review:** the F01–F05 fidelity defects (served, or latent in F04); the serving second authority (F10); the catalog/association breadth; the producer-identity gap; `nodes`/`edges` storage share (38%).

## 10. Verification and uncertainty

| Claim or scenario | Label/date | Check | Conditions and result | Outcome |
|---|---|---|---|---|
| Existing relations map to one transfer relation; controls hold; gaps in access paths and argument-sink composition; F04 label | **Tested** for canonical Delta rows, 2026-09-29; MCP rendering not traced | P0: `p0_queries.sh` (compile-fixture, behavioral profile). Snapshot ids are random per run; the committed raw output is snapshot `13b64de0…` | 6 summary flows, 26 value flows, 28 behaviors; controls held; `facade` recorded as `computed`/established | passed (probe executed; findings as stated) |
| Library capabilities (Csr, views, PageRank, Tarjan depth, ascent BDD lattice, rustworkx order) | **Tested** (probe scope), 2026-09-29 | P3: `cargo run --locked --manifest-path …/p3-graph-reasoning/Cargo.toml --bin …` | PageRank maxdiff 3.196e-2 vs 0 on the 3-cycle control; `Csr` duplicate refused; `deep tarjan 100000` overflows (control) while kosaraju 1M passes; ascent lattice `reach(1)=(a\|b)` over a cycle; `views_neg` compile failure is the intended control | passed |
| Derivation volume; `nodes`/`edges` share; summary yield | **Measured**, 2026-09-29 | P8: `python3 …/p8_volume.py build/store` (snapshot `d0cf20ff…`, behavioral) | support 158,765 rows / 7.0 MiB; graph 2,355,231 rows / 121.5 MiB of 321.5 MiB; 23 summary flows | passed |
| Compile stage costs | Measured by others (historical receipt) | `build/pr5-qualified/behavioral.log` | 473.9 s total; validation 177.2 s | attributed, not re-run |
| Registry second authorities (P7) | Interface-checked | paper aspect table over 5 relations (evidence README) | 18 derived vs 49 hand serving files | code probe unnecessary: in-tree derived path exists |
| Shared driver justified? (P2) | Interface-checked | paper signature table over 12 engines | 2 fixpoint engines, conflicting semantics | resolved: no |
| F01 reachability | Interface-checked + Measured counts | classifier source; pilot `catalog_parameters` | 764/5,139 unknown or unavailable defaults | fixture control not yet run |
| Boundary/coverage divergence (A2 claim) | Interface-checked only | P0 | not reproduced | unresolved; not a finding |
| Independent challenge | 2026-09-29 | fresh `design-reviewer` subagent | recorded below | see below |

Known-answer shapes for implementation acceptance:
- two calls to one callee;
- parallel guarded transfers;
- incompatible signature alternatives;
- an unresolved external call;
- recursion;
- shuffled input order;
- incomplete support;
- a candidate-only association;
- an unknown default state;
- a synthetic-origin site.

The P0 fixture already holds several of these.

**Independent challenge (fresh `design-reviewer` subagent, 2026-09-29).** It read the draft, the cited source and the evidence, and ran six read-only pilot queries (`lctx query --store build/store --snapshot d0cf20ff…`, **passed**). It wrote no file. The reviewer re-verified the decisive counter-claims before accepting them:
- `behavior_discharges`: 0 of 2,107 proved;
- 38 + 90 `over_budget` conditions;
- field links 432/136/23;
- the `flow_behavior_id` key;
- `Frontier::insert` conflation;
- the `selection.rs` budget path;
- ADR-0024's display decision.

| Challenge point | Disposition in this review |
|---|---|
| Sequencing: PR6 should not wait for substrate work with no consumer; split consolidation from new vocabulary | **Accepted.** §9 two tracks; §12 priorities |
| F02 is disclosure and policy divergence (decorator registrations are real invocations), not a relabel | **Accepted.** F02 rewritten; CI-G1 now rests on F01/F03 |
| F04 is latent on the pilot, the Tested scope is Delta rows only, and the correction must handle the identity key | **Accepted** |
| F05 is Measured; route it through ADR-0024's existing display decision; list the internal DNF readers | **Accepted** |
| F01 widened (`FactoryExpression`; `unknown` as a queryable value) | **Accepted** |
| F03 measured exposure; it outranks F02 | **Accepted** |
| F07: `evaluation_complete` is truthful; the real G5 evidence is the frontier cap and the missing memory pool | **Accepted.** F07 and the G5 row rewritten |
| F11: the digest gap is compensated for reuse; skip-on-key buys little | **Accepted.** F11 narrowed to declared writers and `build_digest` labelling; skip deferred |
| F13 is not a key gap but a risk to PR6 attribution | **Accepted.** Moved to priority 1 |
| S2 was overstated (catalog field links exist); derivation index as views; no SCC-level support; call relation as policy relations; typed-index-collections trigger not met; `nodes`/`edges` disposition | **Accepted** |
| §11 omits ADR-0045/0054/0058/0064 | **Accepted.** Added |
| Missed served-path gaps (A3 G6, G7, G9) and PostgreSQL FK stricter than Delta | **Accepted.** Observation and G3 |
| Overall: Revise is correct; target acceptable only as Proposed and split | Agreed |

## 11. Authority changes and dispositions

**Operator disposition (2026-09-29).** The operator adopted a hard layered cutover to the relation-centric
model with PostgreSQL as the single relational store (ADR-0082/0083/0084). The served-fidelity findings
are not repaired in the legacy code; the new contracts close them by construction. Current disposition
for F01–F13 is owned by the [cutover plan §8](../../plans/semantic-model-cutover-plan_2026-09-29.md#8-findings-disposition).
The rows below keep this review's original assessment.

| Required change | Decision route and owner | Source findings | Current disposition location | Closure evidence or revisit trigger |
|---|---|---|---|---|
| Fidelity repairs: tri-state defaults; basis propagation; origin disclosure; `condition_id` served with capped rendering; governed recognizer | Refactors inside existing contracts: §14.7 and ADR-0076/0074; **accept ADR-0024** for the display decision; the models catalog for the recognizer. Owners: `cpg-schema::selection`, `cpg-core::evidence`, `cpg-schema::condition_kernel` and bundle, `cpg-core::surface` | F01, F03, F02, F05, F13 | Deferred row here until the operator accepts; then forward plan §6.2 | Named fixture controls (§7) |
| Transfer label after discharge, with a consistent identity key | Refactor inside ADR-0045/0064 | F04 | same | P0 control plus discharge lookup |
| Relation-centric semantic model. Consolidation track: call-relation policies, one binder, transfer algebra, one obligation policy, emitter and lineage. Vocabulary track: `places`/`transfers`, derivation views, projection runtime, each consumer-gated | **New ADR (Proposed)**. It supersedes ADR-0044's view and flow-algorithm clauses and the ADR-0067 graph-catalog role clauses; the successor keeps authority, identity and materialized `nodes`/`edges`. It **amends the relation contracts of ADR-0045, ADR-0054, ADR-0058 and ADR-0064** without changing their Tested semantics. DESIGN §3.4–3.9, §9, §9.9 | F02, F04, F06–F09 | Deferred here → forward-plan track after acceptance | S1, S5 traced; P0 rerun; S9 with its consumer |
| Serving derived from declarations; DDL drift test; FK list alignment | Refactor under ADR-0073/0078, plus a DESIGN §6.4 amendment | F10 | same | served-column scenario |
| Declared finite stage table; `build_digest` coverage | Refactor inside ADR-0081 (no admission change); skip-on-key keeps ADR-0081's trigger | F11 | same | missing-writer control |
| Generation-scoped prepared objects; set-based hydration | Refactor inside §14.9/§11; moka trigger unchanged | F12 | same | trigger: measured request cost |
| ADR-0071 product-first priority | **Unchanged.** The pivot is justified by served fidelity (CI-G1) and A1–A3 extension locality, not by new analysis. New consumers (S2/S3) still need a PR0/PR6 task trigger (candidates D06, D07, I04, U01, U02) | — | forward plan §3.0 work-selection rule: clarify that substrate consolidation is not "a new analysis" | — |

No §H exception records are required: every deliberately retained bespoke mechanism has its reason in §8.

## 12. Architectural judgment and decision

| Judgment | Verdict | Scenario evidence and scope | Required action/disposition |
|---|---|---|---|
| A1 Localize change | **violated** | S1 (5 independent call-edge decisions), S5 (8 transfer + 2 condition + 5 atom sites; an appended kind becomes the `max` top), served-column addition (3–6 hand sites) | F02, F04, F05, F10 |
| A2 Encode meaning structurally | **violated** | Second authorities for call edge, binding, transfer, condition, refusal and serving schema; free-text projection policies; implicit stage writers | F02, F04–F07, F09–F11 |
| A3 Extend through composition | **violated** for plain-class configuration and behavioral composition; satisfied for recognized-record configuration (catalog field links) and for the Arrow/DataFusion/Delta foundation | S2 per-branch supply and plain-class option→field→reader, and S4, need new places, binders and composition (P0) | F06, F08 |

**Bounded decision: Revise.** CI-G1 and G2 fail on default-profile served paths (F01, F03). A1–A3 are violated for the product's next change scenarios.

**Enclosing architecture: needs revision.** The relational foundation is sound and is retained: Arrow contracts, DataFusion, Delta, generated validation, typed wire. The **proposed target is the relation-centric model in §9 (Alternative F)**, at Proposed/Interface-checked strength with Tested probe support, split into a consolidation track and a consumer-gated vocabulary track. It is not accepted by this review. This review is not a release qualification.

| Priority | Change and responsible component | Source findings | Closure evidence or revisit trigger |
|---|---|---|---|
| 1, now | Served fidelity repairs — `selection`, `evidence`, condition kernel/bundle, `surface`: tri-state defaults; basis propagation; origin disclosure; `condition_id` with ADR-0024 rendering; governed recognizer; lexical parameters in policy | F01, F03, F02, F05, F13 | fixture controls |
| 2, alongside | Behavior-preserving consolidation (clean = rebuilt except the repairs); the latent F04 label fix with its key handling | F02, F04, F05, F07, F08, F10, F11 | S1 one-edit trace; DDL drift test; missing-writer control |
| 3, with the first consumer | New vocabulary as two-consumer slices: catalog field links generalized to `places`/`transfers` (plain-class configuration), then behavioral per-branch composition, derivation views (S9), projection runtime (second topology consumer) | F06, F08, F09 | the second consumer reuses without reinterpretation; P0 rerun |
| deferred | Skip-on-key reuse (ADR-0081 trigger), prepared-object caching (§14.11 trigger), salsa/ascent/moka/typed-index-collections/roaring (their triggers), CFG/dominators, stable symbol keys (first cross-release consumer) | F11, F12 | named triggers |

**Sequencing against PR6.** PR6 measures the default catalog product. Run it after priority 1, so that F01, F03 and F13 cannot bias the comparison. Priority 2 is behavior-preserving, so it can proceed in parallel without confounding PR6. Priority 3 follows product task outcomes, consistent with ADR-0071 and the forward plan's work-selection rule. PR0 comparison is independently blocked on parity.

**Next step and owner:** the operator reads this review. On acceptance:
- the target ADR is written with the `adr` skill, recording the target architecture now so that feature work builds on it;
- the DESIGN owners are amended with the normative relation, algebra and schema tables;
- these findings move to a forward-plan track, together with the deletion obligations of each consolidation step: the independent call-edge SQL, the three SQL binders, the parallel DNF, the duplicate refusal policies, the 7 emitter copies and the hand serving declarations.
