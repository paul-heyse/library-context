# Semantic-model cutover, phase-0 core contracts — design/target review (WP0.10)

## 1. Scope, outcome and coverage

| Field | Value |
|---|---|
| Subject | Phase-0 code at **`f8870ce`** (commits `e66d1f8..f8870ce`; `git log --oneline cfd8f2a..f8870ce`), clean tree when the review began. `crates/lctx-model/` (whole crate), `crates/lctx-postgres/` (`migrations/202609300013_store.sql`, `src/store.rs`, `src/bootstrap.rs`, `sql/database.sql`, `tests/store.rs`), `crates/cpg-core/` (`session.rs`, `store_read.rs`, `parity.rs`, `udf.rs`, `build.rs`, `attempt.rs` producer identity, `tests/dependency_audit.rs`, `tests/store_read.rs`, `tests/model_policies.rs`), `crates/lctx/src/parity.rs`. Main advanced during the review to `e267f3d` (WP1.0, out of scope) and `eeb83c0` (cutover plan §4.2 execution status). The §4.2 "deviations to confirm in WP0.10" are answered in §11. |
| Standard | [Core 3.0](../design_principles/core/design-principles.md) and [template 3.0](../design_principles/core/design-review-template.md), [code-intelligence 1.1](../design_principles/profiles/code-intelligence/principles.md) and [review additions](../design_principles/profiles/code-intelligence/review.md), [repository binding](../design_principles/binding/library-context.md); `design-review` and `design-review-code-intelligence` skills. |
| Tier · purpose | **Design · target** (binding: before a substantial stage and at the phase-0 exit, ADR-0084). Judged against DESIGN [§15](../../design/sections/semantic-model.md), ADR-0082/0083/0084, the Stage-3 semantics ADR-0082 keeps in force (ADR-0045, ADR-0050–0064), and the best contracts for phases 1–5. |
| Reviewer · date | Fresh `design-reviewer` subagent (not the author), 2026-09-29. |
| Maturity and outcome | Phase 0 of a hard layered cutover. Contracts stay open until the layer that produces them cuts over. This review should say whether they are the right contracts to evolve from, and what would force a second authority or a rewrite later. |
| Supported scope | Declarations and generated DDL; identity v2; vocabulary (places, access paths, owner rule, region join); the five call policies and the one binder; the transfer algebra and `compose_call`; the condition kernel; obligations, budgets, verdict and discharge; derivation sources; projection declarations; the stage scheduler; the store kernel lifecycle and canonical reads; producer identity; adapters, legacy-ID relations and the parity harness. CI fact families: occurrences, entities, places, call targets and bindings, transfers, conditions and atoms, obligations, derivations. Served answers: none yet (phase 5). |
| Exclusions | Legacy code paths (unchanged in phase 0); WP1.0 and later; performance (operator: not a gate); integrated gates (deferred to the phase-1 exit by operator decision, not a finding). |
| Expected changes | The six scenarios in the request (§5): L0 declarations and publication in phase 2; policy views and the one binder in phase 3; SCC composition, obligations, rendering and "why unresolved" in phase 4; a mid-cutover contract change; phase-5 serving and the native executor; parity with changed ids. Also the store failure modes: partial COPY, a crash between validate and publish, concurrent attempts, retirement while a reader is pinned. |
| Baseline | The [semantic data model review](design_review_semantic-data-model_2026-09-29.md) (F01–F13; disposition in [cutover plan §8](../../plans/semantic-model-cutover-plan_2026-09-29.md#8-findings-disposition)). Delta, `cpg-schema` and the bundle import remain the implemented pipeline. |
| Method and coverage | **Read at the cited grain:** every file in the subject, §15, the plan's §3, §4, §4.1 and §4.2, ADR-0082/0083/0084, ADR-0045, ADR-0057 and ADR-0064, [§3.9](../../design/sections/behavior-model.md) verdicts and approximation, and the legacy consumers that fix current meaning (`cpg-extract/src/pysa_map.rs`, `cpg-core/src/behavior.rs`, `cpg-schema/src/graph.rs`, `derived.rs`, `behavior.rs`). The pinned library sources were read: biodivine-lib-bdd 0.6.3 quantification, DataFusion 55.1.0 `RecursiveQueryExec`, and the provider fork `e6fc4c4` `declared_table`. **Executed:** one PostgreSQL 18.6 probe of partition, reference and detach behavior (§10, Appendix). **Not re-run:** the author's targeted tests (historical receipts, §10). **Not examined:** WP1.0, the legacy stage bodies, the Python packages. |

This review is evidence, not a status register. Current disposition of C01–C14 belongs in the cutover plan's findings table (binding §4). §11 proposes the rows.

## 2. Responsibilities, dependencies and semantic ownership

| Component | Coherent responsibility and hidden decisions | Consumer contract | Allowed dependencies / direction | Expected reason for change |
|---|---|---|---|---|
| `lctx-model::decl`, `relations` | One `relation!` per relation; row codec; metadata (layer, family, stage, roles, coverage, polarity, fidelity, exposure); `validate`; identity recompute | `Relation` trait, `RelationDecl`, `RelationId` registry (empty until phase 2) | `arrow-*`, `blake3`, `serde`, `schemars` only (pure; `Cargo.toml:10–19`) | A new relation or column |
| `lctx-model::id` | `IdKind` codebook; `lctx-id/v2`; the presence-byte encoding (`RecipeField`); `recipe!` and recipe metadata | Recipe functions and `Recipe` data; `lctx_id_v2` UDF parity | pure | A new identity kind or recipe |
| `lctx-model::ddl` | `TableSpec` IR; partitioned parents, generation-qualified FKs, codebook FKs; staging, index, attach and detach templates; install script and its digest | `Install { statements, tables, digest }` | pure | A column type, constraint or template change |
| `lctx-model::vocab`, `calls`, `transfer`, `obligation`, `condition`, `derivation`, `projection` | The §15 semantics: places, owner rule, region join; policies, binder, effective callable; transfer kinds, composition, merge, `compose_call`; obligation codebook, priority, budgets, verdict, discharge; BDD kernel, substitution, ∃, theory; derivation-source views; projection digest | Pure functions and typed values, tested store-free | pure (+ `biodivine-lib-bdd`) | Each semantic question has one module (A1 holds, §12) |
| `lctx-model::stage` | Stage declarations and the derived schedule (Kahn with declaration-order tie-break; writer/transient/cycle refusals) | `plan(table, profile) -> Schedule` | pure | A stage or profile added |
| `lctx-model::legacy` (temporary) | v1 hasher, adapter/quirk/divergence declarations, legacy-ID side-relation specs, arrow-row multiset diff | Data declarations; `parity::diff` | pure | Deleted in phase 5 |
| `lctx-postgres::store` + migration `…013_store` | Static kernel (`lctx_store`: install record, relation catalog, registry, receipts, events, selections, SECURITY DEFINER lifecycle); owner install/reset/retire; writer lifecycle with pgpq binary COPY | SQL functions (writer); `MigrationStore` (owner) | SQLx, pgpq; consumes `lctx-model::ddl` | Lifecycle state or privilege change |
| `cpg-core::session`, `store_read`, `udf`, `parity` | One session factory with memory pool; canonical provider reads pinned to a generation (D14); `lctx_id`/`lctx_id_v2`; parity comparison under declared divergences | `SessionContext`s; `CanonicalReader::register` | DataFusion, provider fork, `lctx-postgres` | Read mode, UDF, harness |

Compile-time direction is sound: `lctx-model` ← `lctx-postgres` (DDL only) ← `cpg-core` ← `lctx`. `lctx-model` depends on neither `cpg-schema` nor `cpg-core` (ADR-0084's phase-0 exit criterion holds). `cpg-schema` re-exports the moved primitives (D1; `cpg-schema/src/id.rs:7`, `hash.rs:4`, `codebook.rs:10`).

| Concept | Semantic authority and identity | Update boundary | Derived forms / consumers | Second copy? |
|---|---|---|---|---|
| Relation contract | `relation!` → `RelationDecl` | Declaration edit → install digest → `lctx store reset` (D3) | DDL, templates, COPY columns, Arrow schema, codec, recompute validator | None found for columns, keys, roles or codebooks |
| Producing stage of a relation | **Two**: `RelationDecl.stage` (`decl/relation.rs:144`) and `StageDecl.outputs` (`stage.rs:77`) | Independent edits | Scheduler uses only the stage table; nothing reads `decl.stage` | **Yes — C03** |
| Identity encoding | `IdHasher::v2` + `RecipeField` (`id.rs:210–268`) | `IdKind` append-only; recipe fields | Rust recipes, `decl::identity::recompute`, `lctx_id_v2` UDF (tested equal per recipe, `udf.rs` test) | None |
| Codebook membership | Rust `codebook!` | Append-only | Generated `codebook_<name>` tables and FKs; the install list of codebooks is passed by hand but a missing one is refused (`ddl.rs:598–608`) | Legacy twins: only `BoundaryReason`↔`ObligationKind` is equality-tested (O4) |
| Call admission | `CallPolicy::predicate` → Rust `admits` and SQL `sql` (tested equal on 5,040 combinations) | One predicate edit | Policy views (phase 3) | The site aggregates the predicate reads (`is_unique`, `is_complete`) have **no owner** (C04) |
| "Caller" (owner rule), observation join | `vocab::owner_of`, `innermost_region_join` | — | Phase 2/3 producers | No relational form; the join's syntax-kind literals have no shared type (C11, C12) |
| Transfer identity and merge | `Transfer::id` (condition included) and `Transfer::key` (condition excluded) | — | Merge, derivations, findings | The identity and the semantic key disagree (C08) |
| Verdict, obligation priority, discharge | `obligation::verdict`, `priority`, `discharge` | — | Producers, validators, native executor (planned) | One owner, but its classification contradicts Tested semantics (C09) |
| Role names | Migration SQL literals, `database.sql`, `bootstrap::ROLES`, `serving::Role::name`, `store::CANONICAL.reader` | D5 fixes them until phase 5 | Grants | Five spellings, no check (O1; accepted by D5) |

**CI fact and fidelity table** (the target contracts as declared or implied at `f8870ce`; all **Proposed** until their phase lands).

| Fact family or relation | Provider and revision | Fidelity | Coverage and unknowns | Identity | Consumers |
|---|---|---|---|---|---|
| Occurrences (L0 syntax, L1 with owner) | Ruff/Pyrefly parse (§B1) | extracted; owner resolved | Region join `Unmatched` is a coverage gap (`vocab.rs:343`) | `occurrence(module entity, start, end, syntax_kind: &str)`; entity recipe not yet declared, recursive with modules (C11) | Region join, places, atoms, bindings |
| Places | derived by producers | derived | `unknown_suffix` saturates k = 2 (`vocab.rs:161–172`) | `place(root_kind, owner, position, name, path text)`; attribute text unescaped (C11) | Transfers, atoms, bindings |
| Call targets | Pysa (Pyrefly fork), ty, models | resolved; modality, origin, phase, kind, implicit | `Unresolved` target kind; `is_complete` site aggregate unowned (C04) | recipe not declared (IdKind `call_target` reserved) | Five policy views |
| Call bindings | the one binder (`calls::bind`) | derived | Ambiguous / unmapped / refused rows, never dropped | recipe not declared | `compose_call` |
| Transfers | flow, summaries, composition, models, TITO, field links | derived / authored / provider | Open call → obligation, never a kind | `transfer(owner, in, out, kind, condition, context, provenance)` — condition inside (C08) | Behaviors, catalog, explanations |
| Conditions and atoms | BDD kernel over v2 atoms | derived | Kernel budgets → obligations; rendering `truncated` | Merkle nodes; atom = `(evaluation, kind, operand, argument)` (C11) | Transfers, verdicts |
| Obligations | producers via one codebook | derived | Classes Budget … Scope | recipe not declared | Verdict, "why unresolved" |
| Derivations | step relations declared as sources | derived | Premise existence unenforced (C13) | step id | "why" views |

## 3. Contracts, constraints and testing boundaries

| Contract | Inputs/outputs and consumer expectation | Invariants and enforcement | Effects / lifecycle / failure | Substitution | Isolated verification |
|---|---|---|---|---|---|
| Declaration → DDL | `TableSpec::from_decl`, `install` | `validate` (names, keys non-null, identity inputs not provenance, single-column role targets, types equal); cycles among references refused (`ddl.rs:631–656`); a referenced codebook must be supplied | Pure; digest over statements **and** templates (`ddl.rs:611–622`) | Legacy contracts enter through `TableSpec::build` | Snapshot of sample DDL and digest; DDL types equal pgpq's encoding (`tests/store.rs:409–419`, runs without Docker) |
| Identity v2 | Recipes; recompute over Arrow; UDF | Presence byte on every field, so Rust equals SQL over nullable columns; kind from a codebook | Pure | v1 kept for legacy (`legacy.rs:11–21`) | Hand-written encoding control, known answers, proptest, recipe = SQL per recipe |
| Binder | `bind(call, alternative, variant) -> Vec<CallBinding>` | Every actual and every non-variadic formal gets a row; unpacking is ambiguous, never an error | Pure | — | Seven binder known-answer cases (`tests/semantics.rs:46–142`) |
| `compose_call` | caller transfer, one binding, callee transfer → transfer / disjoint / selection / obligation | Binding bound and connecting the exact roots; site matched | Pure; kernel budgets become obligations | — | Config, pair, select/fetch, facade shapes — **identity kinds only** (C05, C06) |
| Verdict / discharge | `verdict(condition, open, coverage_complete, approximated)`; `discharge(members, decisions)` | Five verdicts; open obligation ⇒ unknown or not analysed; refutation needs coverage and no approximation | Pure | — | Verdict and discharge cases (`tests/semantics.rs:406–439`) |
| Store lifecycle | create → COPY → validate_relation → mark_validated → publish → select → retire | COPY: NOT NULL, CHECK, partition CHECK; validate: count = receipt, unique indexes; publish: one transaction, ATTACH validates FKs and codebook FKs; writer revoked; `ddl_digest` equality at create | Staging invisible to the reader; a failed generation stays inspectable by the writer; retire is owner-only | Legacy relations keep `snapshot_id` (D6) | Seven real-PG tests (historical receipt, §10) |
| Canonical reads | `CanonicalReader::register(ctx, generation, relations)` | Registry state admits by role; reads the generation's own partitions; list items cast to declared (a forbidden null is refused) | One registry query per registration | Legacy or model partition column | Type-matrix round trip (historical receipt) |
| Stage table | `plan(table, profile)` | One writer per published relation per profile; no unwritten transient; acyclic; externals unwritten | Pure | Legacy stages marked `legacy` | Five refusal controls (`tests/stage.rs`) |
| Parity | `diff` (arrow-row multiset) + `compare` (declared divergences as SQL predicates) | Exact multiset with multiplicity; floats by bits; one-sided declarations fail | Pure over batches | — | Seven self-test controls (`cpg-core/src/parity.rs:194–268`) |

**Absence states.** The binder distinguishes bound, ambiguous, unmapped and refused. Obligations distinguish budget, resolution, model, evidence, selection and scope. Places distinguish a known path from an unknown suffix. The region join distinguishes exact, innermost and unmatched. Gaps: a receiver cannot be *unknown* (C10); a refutation that fails for lack of coverage and one that fails for approximation share the selection reason `incomplete_domain` (C09); a published relation left empty because the profile skipped its writer is not recorded as "not requested" (O6).

## 4. Composition and execution

| Capability or stage | Semantic inputs/outputs | Owning mechanism | Dependencies and reuse boundary | Policy vs orchestration | Effects and publication | Limits and determinism |
|---|---|---|---|---|---|---|
| Declarations → install | Decls + codebooks → script, templates, digest | `ddl::install` | Digest over everything the store executes | No policy in orchestration | Owner `install`; `create_generation` refuses a different digest (LX001) | Deterministic text; dependency order is stable |
| Attempt → generation | Arrow batches → staged, validated, published partitions | `Writer` + SECURITY DEFINER functions | Each COPY statement atomic; count and key checks per relation | DataFusion validators run before `mark_validated` (the caller's promise) | One publish transaction; privileges make it immutable | 50k rows per COPY; writer timeouts |
| Retirement | Published generation → dropped partitions | `MigrationStore::retire` | **Not atomic, not resumable (C01)** | — | DETACH CONCURRENTLY outside transactions | Owner `statement_timeout` ≤ 300 s |
| Pinned read | Generation id → registered views | `CanonicalReader` | Pins by partition; checks state, **not the digest (C02)** | — | Read-only connections | Session memory pool (DataFusion 55.1 `RecursiveQueryExec` reserves from it: Interface-checked) |
| Schedule | Stage table → order, empty set | `stage::plan` | Stage names are strings (C03) | — | none | Kahn with declaration-order tie-break |

**CI analysis record** for the semantic operators.

| Operator | Question answered | Projection / inputs | Method and settings | Exact / conservative / heuristic, model | Budgets and partial results | Output and evidence linkage |
|---|---|---|---|---|---|---|
| `calls::bind` | Which formal each actual reaches for one alternative × variant | Call syntax, one alternative, one variant | Python binding rules; receiver shift by input flag | Conservative (unpacking → ambiguous); receiver rule external (C10) | none needed | `CallBinding` rows; recipe not yet declared |
| `CallPolicy` | Which targets a consumer sees | Call-target policy columns | One predicate, two renderings | Exact over its columns; the columns' meaning is partly unowned (C04) | — | Views; the projection digest includes the SQL (`projection.rs:220–227`) |
| `compose_call` | What flows through a call | Caller transfer, one binding, callee transfer, formal-atom map | Path prefix relation; kind table; substitution then ∃ | May-analysis; **path and condition rules are not sound as written (C05, C06)** | Kernel budgets → obligations | Composed transfer with `context = site`; derivation rows are the caller's |
| `verdict` | Verdict and named reason | Condition state, open obligations, coverage, approximation | Class-ordered priority | Relative to the §3.9 model; **classification departs from Tested semantics (C09)** | Budget class ⇒ unknown | Reason = first obligation |
| Condition kernel | and/or/not/given/implies/compatible/∃/substitution/rendering | Diagrams over atom ids | biodivine bounded apply; own simultaneous substitution | Exact within budgets | Every operation preflighted; refusal is never `false` | Merkle node catalog, hydration validator |
| `parity::diff` + `compare` | Legacy vs new relation equality | Two batches | arrow-row encodings in a `BTreeMap` | Exact multiset | Memory outside the DataFusion pool (O5) | Counts and samples per relation |

## 5. Change and failure scenarios

| Scenario and trigger | Owning component | Contract change | Expected vs observed affected consumers | Independent edits / hidden knowledge / test setup | Evidence |
|---|---|---|---|---|---|
| **S1. Phase 2 declares L0** (occurrences, entities, syntax, Pysa call facts with origin/implicit, ty flow facts with structured places, occurrence-keyed atoms), installs them beside the legacy shim, publishes with rewritten extractors, legacy-ID side relations and raw adapters | `lctx-model` declarations; extraction stages | New `relation!`s, recipes, stage entries | Expected: one declaration per relation, DDL/codec/validator follow. Observed: holds for columns, keys, roles and codebooks; the install list is one `Vec<TableSpec>` for model, legacy shim and side relations (D6 handled by `Partition::supplied`). **But** each relation's producing stage is written twice (C03); the entity recipe and module ↔ occurrence recursion, the syntax-kind type and the atom discriminator are unsettled (C11); the owner rule and region join have no batch form (C12) | Two stage names per relation; syntax-kind spelling shared by convention with `innermost_region_join`; the Pysa L0 relation must carry callee kind and higher-order index for C04 | Code read (Implemented); sample model `decl_sample.rs` |
| **S2. Phase 3 policy views and the one binder**; a provider adds an alternative that downgrades a unique resolution | `calls` | Policy views generated from `CallPolicy::sql`; `call_bindings` from `bind` | Expected: a policy edit is one predicate change; the SQL binders are deleted. Observed: the predicate side holds (SQL = Rust, tested). Whether an agreeing ty alternative, a `__new__` + `__init__` pair or a potential target breaks `is_unique` is undecided, so the downgrade in the scenario is decided wherever phase 3 computes `is_unique` (C04). The higher-order/`ifCalled` exclusion that every legacy consumer applies cannot be expressed, and `dataflow` admits potential targets (C04). A class-attribute call has no unknown-receiver state (C10) | Phase 3 must write site-aggregate SQL: a second call-admission authority, which is F02 again | Code read; legacy `pysa_map.rs:644–648`, `graph.rs:466`, `derived.rs:497` |
| **S3. Phase 4 SCC composition**, obligations (budget vs dominance vs evidence), rendering, "why unresolved" | `transfer`, `obligation`, `condition`, `derivation` | Summaries over `compose_call`; obligations; derivation views | Expected: composition, verdict and explanation reuse the core. Observed: the kernel, budgets (named `Meter`s) and rendering (truncation marker) are ready. `compose_call` is sound only for identity kinds and the bound formal (C05). It drops conditions it cannot express (C06). Selection has no payload (C07). The transfer id churns with every merged alternative, so SCC iterations and derivation premises chase ids (C08). Verdict classes contradict §3.9 (C09). Premises are not enforced, and generated views carry no reader grant (C13). "Dominance" has no representation yet (phase-4 frontier; not a finding) | Phase-4 producers would restate the approximation rule and map the legacy rankings as "quirks", which parity cannot flag | Code read; known-answer shapes cover identity kinds only |
| **S4. A contract changes mid-cutover** (a column added) | `decl` → `ddl` → store | Regenerate; digest changes; `lctx store reset`; recompile | Expected: one declaration edit. Observed: holds — `create_generation` refuses the old digest (LX001, tested), `install` refuses a different digest (tested), `reset` drops every generation (tested). Gaps: readers do not check the digest (C02); the digest certifies rendered text, not the live catalog (O2) | Reset is all libraries at once (accepted by D3/ADR-0078) | `tests/store.rs:386–406` (historical receipt) |
| **S5. Phase 5 serves generated views** over the pinned generation, with grants and lookup indexes; the native executor reads the generation | `ddl::view`, store reads | Views over parents filtered by generation; lookup indexes from `Exposure` | Expected: views are generated from declarations. Observed: parents carry partitioned lookup indexes, and each staging table's matching index is adopted at ATTACH. `ddl::view` emits no GRANT (C13). A parent-filtered read cannot tell a retired or partly retired generation from an empty one (C01): the test suite asserts exactly that (`tests/store.rs:379–380`). The native executor (no DataFusion, D14) will read through SQL, so it needs a store-owned pin check | — | Code read |
| **S6. Parity for a phase-2 adapter with changed ids** | `legacy`, `cpg-core::parity` | Adapter joins `legacy_ids_<family>`; conditions recomputed by the legacy kernel | Expected: exact legacy ids through the side relation. Observed: holds for direct id columns (self-test). The side relation is 1:1 by key and unique (`legacy.rs:86–105`), so a split or merge fails publication rather than surfacing in the report. Ids nested in content hashes (fact ids, condition nodes, whose variable order follows atom ids) need Rust adapters, as planned | Divergence predicates are unpaired; the report lacks per-divergence counts (O5) | Self-test (historical receipt) |
| **F1. Partial COPY / writer crash** | Writer | — | Each COPY statement is atomic; an erroring chunk stops `copy`; validation checks count = receipt and the keys; staging stays invisible | — | Tested (count and key refusals, receipt) |
| **F2. Crash between validate and publish** | Registry | — | The generation stays `validated`, never visible; a retry is a new generation (LX002). Stale `staging`/`validated` generations accumulate until retired (no sweep; acceptable under ADR-0078 if WP1.9 retires them) | — | Code read + tested retry refusal |
| **F3. Concurrent attempts** | Registry, parents | — | Distinct ids and schemas; publish serializes on the parent locks in one ordinal order (no lock-order inversion); selection is last-writer-wins by explicit call | A zombie writer can still select an older generation (O3) | Code read |
| **F4. Retire while a reader is pinned; interrupted retire** | `MigrationStore::retire` | — | Partition-pinned readers are refused after the DROP (good). Parent-filtered readers see empty relations. A detach cancelled by the owner's `statement_timeout` while a reader holds a snapshot leaves the partition *pending detach*: a new parent read of that generation returns 0 rows, a retry fails, and no other generation of that relation can be detached until `FINALIZE`, which the code never runs. A self-referencing relation cannot be detached at all (C01) | — | **Probe P1/P2 (executed, §10)** |

## 6. Correctness and fidelity gates

| Gate | Verdict | Own evidence or scope reason | Required action |
|---|---|---|---|
| G1 Authority | **fail** | C03: the producing stage is two independently editable facts with no reconciliation. C11: the syntax-kind spelling is shared by convention. C13: a premise's relation is restated beside the column's role. C14: the owner line (§15.1 vs D14). Declarations → DDL/codec/validators and codebooks → tables are otherwise single-authority | C03 before WP1.2/WP2.1; C11 at WP2.1; C13 at WP4.7; C14 in the next docs commit |
| G2 Semantic fidelity | **fail** | C09: `scope_boundary` → not analysed; a failed refutation is labelled with a selection reason; value approximation cannot be expressed. C05(a): spurious sub-path flows across derived steps. C06: conditions dropped, so callee-conditional flows are Established. C02: a reader interprets a generation under its own declarations (DP-24) | Correct in `lctx-model` before the owning phase (cheap now) |
| G3 Validity | **pass** | The write path refuses NOT NULL, CHECK, partition, key, codebook and reference violations, a count mismatch, and a foreign DDL digest (tested). `validate` refuses malformed declarations; the kernel validates catalogs; the parity harness refuses schema mismatches | — |
| G4 Hidden behaviour | **pass** | `lctx-model` performs no I/O. `build.rs` reads sources at build time (declared). `register` performs one declared registry query. The session factory is deterministic | — |
| G5 Consistency and recovery | **fail** | C01: a partially retired or pending-detach generation reads as empty through parents while the registry says `published`; retire is not idempotent; a pending detach blocks later retirements; a self-referencing relation cannot be retired. Publication itself is one transaction (pass) | C01 before phase 1 depends on retire and readers (WP1.3, WP1.9) |
| G6 Transformation and reuse | **fail** | C05: `compose_call` changes meaning across derived steps and leaks callee roots. C08: transfer identity is not the merge key. Producer identity now covers every canonical producer (F11 progress, pass) | C05, C08 before WP4.2/4.3 |
| G7 Truthful capability claims | **pass** | Commit messages and plan §4.2 label targeted evidence and `not_run` items accurately. §15 keeps "implementation Proposed". The memory-pool claim for recursive CTEs is Interface-checked (DataFusion 55.1 `RecursiveQueryExec` reserves via `try_grow`), not Tested; only a sort is tested (`session.rs:269–280`) | Label the recursive-CTE bound Interface-checked where cited |
| G8 Library leverage | **pass** | Bounded biodivine operations, arrow-row, pgpq, the provider fork and DataFusion are used where they fit. Own code is justified: simultaneous substitution (ADR-0082), the deterministic Kahn tie-break, the multiset diff (O8 notes a refinement) | — |
| CI-G1 Fidelity | **fail (contract level; nothing is published yet)** | C04(b): potential and higher-order targets admitted as calls by `dataflow` and bindable. C09: `scope_boundary` relabelled. C06: callee conditions dropped | C04 before WP2.1 (fields) and WP3.1 (policy); C06, C09 before phase 4 |
| CI-G2 Evidence closure | **unresolved** | C13: derivation premises are unenforced, and ids go stale on merge (C08). Nothing is served yet | C13 at WP4.7 |
| CI-G3 Evaluation integrity | **pass** | The shape library is independent model data; the P0 fixture lives in `fixtures/python/semantic_shapes/`; no gold path is read; parity is declared "never a semantic control" (`parity.rs:4`) | — |

## 7. Findings and applicability

**Priority.** *Before phase 1 relies on it*: C01, C02, C03. *Cheap corrections to phase-0 code whose consumer is a later phase*: C05, C06, C08 and C09 (fix before their owning phase; recommended now, because later producers would otherwise restate the rule). *Contract decisions allowed to evolve until the owning phase*, each with its latest trigger: C04 (WP2.1 fields, WP3.1 policy), C07 (WP4.2), C10 (WP3.1), C11 and C12 (WP2.1–2.2), C13 (WP4.7). *Documentation*: C14.

<a id="C01"></a>
### C01 — Retirement is neither atomic nor resumable; self-referencing relations cannot be retired; parent reads cannot tell retired from empty

- **Principles · gate:** DP-19, DP-03, DP-12 · G5; FP-05.
- **Evidence.**
  - `lctx-postgres/src/store.rs::MigrationStore::retire:125–182` works in four steps:
    - it reads the state and checks the selection outside any lock (`:127–141`);
    - for a published generation, it runs `DETACH … CONCURRENTLY` and `DROP TABLE` per relation in reverse ordinal, outside a transaction (`:146–165`);
    - it records `retired` only at the end (`:166–181`);
    - it has no intermediate state, no `FINALIZE` path and no check for an already-detached partition.
  - `select_generation` (`migrations/…013_store.sql:247–262`) takes no `lctx_store.lock`.
  - `ddl.rs::dependency_order:639` admits self-references. §15.3's entity recipe (owner entity) and §15.7's condition nodes (low/high) make them likely.
  - `tests/store.rs:379–380` asserts that a retired generation reads as **0 rows through the parent**.
  - The migrator pool sets `statement_timeout` (≤ 300 s) and `lock_timeout` (`lib.rs:193–196`).
  - **Probe P1** (PostgreSQL 18.6, §10): ATTACH of a self-referencing partition succeeds. Plain and concurrent DETACH both fail with `removing partition "entities" violates foreign key constraint … still referenced`. DROP TABLE is refused. DELETE of the partition's rows followed by DETACH CONCURRENTLY succeeds.
  - **Probe P2:** a DETACH CONCURRENTLY cancelled by `statement_timeout`, while another session holds a snapshot on the parent, leaves `inhdetachpending = t`. A new parent read of that generation returns 0 rows. A retry fails: `already pending detach … Use … FINALIZE`. Detaching *another* generation's partition of the same parent fails the same way. ATTACH still works. `DETACH … FINALIZE` completes it.
- **Consequence.**
  1. With any self-referencing relation, retire detaches and drops every higher-ordinal referrer, then fails. The generation stays `published` with relations missing. Parent-filtered readers see them as empty, and the registry claims completeness.
  2. A retire during a long pinned read (the scenario D4 was designed for) can leave a pending detach. The generation then reads empty, the retry fails, and every later retirement of that relation is blocked until someone runs FINALIZE by hand. WP1.9 (operator cutover, "delete superseded … generations") is the first place this runs on real data.
  3. A concurrent `select_generation` can select a generation that retire is dismantling.
- **Correction** (owner: `lctx-postgres` migration and `store.rs`; about one function and one state).
  1. Under `lctx_store.lock(g)`, in one transaction: set state `published → retiring` (select already requires `published`); revoke the reader's `USAGE` on `lctx_g<hex>` and `SELECT` on its partitions; record an event.
  2. Per relation, in reverse ordinal and idempotently: skip a partition that is already detached or dropped (`pg_inherits`); run `DETACH … FINALIZE` when `inhdetachpending`; otherwise `DETACH … CONCURRENTLY`. Then drop the partition.
  3. Mark the generation `retired`.
  4. For self-references, choose one route: either `from_decl` renders no FK when `fk.relation == name` and a generated semantic validator checks it (the DDL already knows the reference), or the owner deletes the partition's rows in the `retiring` state before detaching (probe case 7).
  5. Parent-scoped readers (phase-5 views, the native executor) pin through the registry state, not through partition presence.
- **Closure evidence:** store tests for (a) publish and retire of a relation with a self-reference; (b) an injected interrupted detach (a reader holding a snapshot, a short owner timeout) followed by a resumed retire that completes; (c) `select_generation` refused during `retiring`; (d) a parent read of a `retiring` generation refused rather than empty.

<a id="C02"></a>
### C02 — Canonical readers do not check the installed contract digest

- **Principles · gate:** DP-24, CI-13 · G2.
- **Evidence.**
  - `cpg-core/src/store_read.rs::CanonicalReader::register:134–152` admits a generation by role and registry state only. `declared_table` (provider fork `e6fc4c4`, `crates/postgres/src/lib.rs:144–156`) takes the caller's schema without comparing it to the catalog.
  - The writer's check exists only at `create_generation` (`migration:118–121`).
  - `lctx_store.generations.ddl_digest` records the digest per generation.
- **Consequence:** after a newer binary runs `lctx store reset`, an older `lctx query`, `rebuild` or phase-5 server reads generations built under other declarations. An added column fails to plan, which is benign. An appended codebook value is passed through as a raw `Int16` that the old binary's codebook refuses or misrenders. An incompatible version is interpreted under current defaults, which DP-24 forbids.
- **Correction:** `register` (and the phase-5 server pin) compares the generation's `ddl_digest` with the reader's own install digest and refuses on mismatch. A reader that intentionally spans versions needs its own declared decision. Owner: `cpg-core::store_read`; WP1.3.
- **Closure evidence:** a store-read test in which a mismatched digest is refused with a typed error.

<a id="C03"></a>
### C03 — A relation's producing stage is declared twice, and stage tables name relations by unchecked strings

- **Principles · judgment/gate:** DP-01, FP-04 · A2, G1.
- **Evidence:**
  - `decl/relation.rs::RelationDecl::stage:143–144` is a free string that nothing reads (searched `crates/`);
  - `stage.rs::RelRef::Model(&'static str):15` and `StageDecl::outputs:77`;
  - `stage::plan:122` never sees the registry, and no check links the two.
- **Consequence** (S1): phase 2 declares `occurrences` with `stage: "extract"` and lists it under another stage's outputs, or misspells it (`Model("occurences")`). Nothing refuses the disagreement. An unknown output name becomes a "published relation" that is never installed, and the real one then fails `mark_validated` late, at run time. The per-stage code identity (plan §3.3, F11's generalization) would digest the wrong stage's code for that relation.
- **Correction:**
  - `RelRef::Model(RelationId)`;
  - derive each model relation's producing stage from the stage table (remove `RelationDecl.stage`), or keep it and add a registry test that `plan`'s writer equals `decl.stage` for every `RelationId` in each profile;
  - in WP1.2, add a check that `stage::published(table)` equals the install list for the legacy names.
  - Owner: `lctx-model::stage`, `decl`.
- **Closure evidence:** the check fails on an injected mismatch; no string names model relations.

<a id="C04"></a>
### C04 — The call-policy fact contract is incomplete: site aggregates are unowned, and potential and higher-order targets are admitted as calls

- **Principles · judgment/gate:** CI-02, DP-01, DP-02, FP-04 · A2, G1, CI-G1.
- **Evidence.**
  - `calls.rs::TargetFacts:62–73`: `unique` ("the site has exactly one target alternative") and `complete` are **site** aggregates, computed outside `lctx-model`.
  - `Fact::column:88–98` names relation columns that do not yet exist.
  - `CallPolicy::Dataflow:210–213` admits every modality and origin; so does §15.5's table.
  - Legacy meaning:
    - Pysa emits `call_targets`, `init_targets` and `new_targets` at one site (`cpg-extract/src/pysa_map.rs:644–648`);
    - `if_called` and higher-order parameter targets are `Potential` with phase `Call` (`:616–621`, `:659–670`);
    - every legacy call-edge consumer excludes `higher_order_index IS NOT NULL` or non-`call` callee kinds (`cpg-schema/src/graph.rs:466`, `derived.rs:497`, `behavior.rs:2086`).
  - The binding's §6 lists "Pysa `ifCalled` targets emitted as call targets" as a known defect shape.
- **Consequence.**
  - (a) In S2, whether a ty candidate that *agrees* with Pysa's definite target breaks uniqueness, whether `__new__` + `__init__` at a `C()` site are two alternatives, and whether a potential target counts are all undecided. Phase 3 decides them in the SQL that computes `is_unique`: a second call-admission authority, the defect F02 closes. Counting rows would fail `summary` for every constructor call and every cross-provider agreement, so nothing composes through them.
  - (b) For `map(f, xs)`, Pysa's higher-order target `f` (potential, `Call`, function) is admitted by `dataflow`. The one binder binds `(f, xs)` to `f`'s formals, a relabelled relation reaching composition (CI-02).
- **Correction** (owner: `calls`; amend §15.5 via an ADR-0082 follow-up or the phase-3 ADR):
  - Add the provider's callee kind and higher-order position to the policy facts, or keep higher-order parameters in their own relation.
  - Restrict `dataflow` (and anything feeding the binder) to direct, non-potential call targets.
  - Add `site_facts(alternatives) -> (unique, complete)` in `calls.rs` with a generated SQL form: distinct target *entities* per conjunctive phase group; agreeing providers keep uniqueness; disagreement breaks it; potential and higher-order entries are excluded.
  - WP2.1's Pysa L0 relation must carry the fields.
- **Closure evidence:** the admission matrix and the SQL = Rust test extended; known answers for provider agreement and disagreement, `C()` with new + init, and `map(f, xs)`.

<a id="C05"></a>
### C05 — `compose_call` composes places wrongly across derived steps and leaks callee-specific roots

- **Principles · judgment/gate:** CI-06, DP-08 · A3, G2, G6.
- **Evidence:** `transfer.rs::compose_call:188–205`.
  - (a) `PathRelation::Rest(rest) => caller.input.extended(&rest)` regardless of `caller.kind`, and `Shorter(rest) => base_output.extended(&rest)` regardless of `callee.kind`. Path arithmetic is valid only across `identity`.
  - (b) `base_output` maps only the callee's `Return` and the *bound* formal. Any other root is "kept" (`:195`): another formal, the callee's `Yield`/`Raise`, the receiver when composing a non-receiver binding.
  - The known-answer shapes cover identity kinds and the bound formal only (`tests/shapes/mod.rs`, `tests/semantics.rs:239–354`).
- **Consequence.**
  - (a) `def caller(s): d = parse(s); return read_timeout(d)` composes `s.timeout → result` (derived), a sub-path of a string.
  - (a) `def dump(cfg): return json.dumps(cfg)`, with the caller delivering `X` at `c.timeout`, composes `X → result.timeout`. A later `.timeout` read of the result then reconnects `X` by the Rest rule: a spurious flow.
  - (b) `def update(target, key, value): target[key] = value`, composed through `value`, yields a *caller-owned* transfer into `update.target[*]`. It never composes further (`AmbiguousBinding` at the next level) and renders as a flow into another function's parameter.
- **Correction** (owner: `transfer`):
  - (a) Extend paths only when the step on that side is `identity`; across `derived`, use the whole place (root path) with kind `derived`.
  - (b) Give `CallSite` the site's complete binding set from the one binder; map every formal-rooted output through its binding; map `Raise`/`Yield` to declared caller-side places or return a typed obligation.
- **Closure evidence:** shapes for a derived caller, a derived callee, a cross-argument mutation and a raise, with pre-written answers. Before WP4.3.

<a id="C06"></a>
### C06 — Condition composition drops what it cannot express instead of refusing or stating it

- **Principles · gate:** CI-06, DP-08 · G2, CI-G1. Tested semantics: ADR-0057:49, ADR-0045:88.
- **Evidence:**
  - `compose_call:206–224` existentially eliminates every callee atom that is neither a substituted formal atom nor in the caller's support;
  - `tests/semantics.rs:365` asserts `∃ local . local` is true;
  - ADR-0057: "Missing identity/stability links refuse semantic substitution";
  - ADR-0045: "An opaque condition is still stated (`conditional`)".
- **Consequence.**
  - (i) If the stability link for `timeout is None` is missing (the formal is reassigned before the guard), the caller omits it from `formal_atoms`, and both `select_timeout` branches compose to `true`. The per-branch supply that F06's closure evidence requires becomes two unconditional, Established flows.
  - (ii) A callee flow guarded only by callee-internal state (`if self._debug: return x`) is served as Established in the caller.
- **Correction** (owner: `transfer`, with atom→operand information in `CallSite`):
  - Refuse (`condition_transfer_unsupported`) when an atom whose operand is rooted at a callee formal has no substitution.
  - Replace callee-local atoms by one opaque caller-side atom at the call-site evaluation, which keeps the flow conditional. Alternatively, eliminate them and set a flag the verdict function reads as conditional.
- **Closure evidence:** shapes for a missing stability link (refused) and a callee-local guard (conditional). Before WP4.3.

<a id="C07"></a>
### C07 — Control influence and selection have no contract

- **Principles · judgment/gate:** DP-02, DP-15 · A2, G7.
- **Evidence:**
  - `TransferKind::Control` (`transfer.rs:26`) is a transfer kind hashed into transfer identity;
  - §15.1:47 lists "control influences" as a separate L2 relation;
  - `compose_seq:60–67` returns `Selection` whenever `control` is involved;
  - `CallComposition::Selection:147` carries no guard atom, influencing place or guarded transfer.
- **Consequence:** phase 4 cannot produce §15.6's "selection relation" without re-deriving the pairing elsewhere. A caller value whose only role is to decide a callee branch yields neither a transfer, an obligation nor a selection: silently nothing. `TransferKind` is append-only, so `control` freezes at the codebook's first publication whether or not it is the right encoding.
- **Correction:** decide in §15.6 (ADR-0082 follow-up):
  - control influences as their own relation (place → atom it is read by);
  - selection as a declared relation (influence, atom, guarded transfer key), with `Selection` carrying those ids;
  - remove `Control` from `TransferKind` before publication if influences are separate.
- **Closure evidence:** a shape where a caller value selects a callee branch, with the expected selection rows. Before WP4.2.

<a id="C08"></a>
### C08 — Transfer identity includes the merged condition, contradicting the merge key

- **Principles · judgment/gate:** DP-04 · A2, G1, G6.
- **Evidence:**
  - §15.3:132 (recipe includes the condition) and §15.6:244 (equal semantic keys merge by OR);
  - `transfer.rs::Transfer::id:85–95` (includes `condition.id()`) against `Transfer::key:98–107` (excludes it);
  - `id.rs::recipes::transfer:370–378`;
  - §15.9 says alternatives are separate derivations **of one conclusion**.
- **Consequence:** after merge the id is not the semantic key.
  - Every added alternative (a provider, a path, an SCC iteration) changes an existing conclusion's id.
  - Derivation steps and obligations recorded against a pre-merge id dangle, and C13 means nothing rejects them.
  - In an SCC fixpoint, premise ids churn each iteration.
  - A cross-generation diff shows a widened condition as delete + add.
- **Correction:** transfer id over the merge key; the condition becomes a payload column. Amend §15.3's recipe. Owner: `id::recipes`, `transfer`.
- **Closure evidence:** a test that merging an alternative leaves the id unchanged. Cheap now; before phase 4 at the latest.

<a id="C09"></a>
### C09 — The verdict and obligation policy contradicts Tested semantics

- **Principles · gate:** CI-06, CI-04, DP-02 · G2, CI-G1.
- **Evidence.**
  - `obligation.rs:146` places `ScopeBoundary` in the `Scope` class, so `verdict:312–323` returns **not analysed** when it is the only open obligation. §3.9 (`behavior-model.md:328`) and the implemented grading (`cpg-core/src/behavior.rs:985–987`) serve **unknown (`scope_boundary`)**. §3.9:382 reserves `not_analyzed` for "out of scope or not requested, nothing else".
  - `verdict:330` reports a refutation that failed for lack of coverage, or for approximation, as `incomplete_domain`, a *selection* reason (`:66`, class `:145`).
  - `VerdictInput:296–303` has one `approximated` flag that only blocks refutation. The Tested flow grading demotes an approximated positive to unknown (`missing_evidence`) until discharged (`behavior.rs:989–991`; ADR-0064:61–67, "certified value approximation").
  - `ResponseBudget` (`:72`, Budget class `:113`) would turn an established conclusion unknown if passed as open, against §15.9: "A smaller budget never removes an established conclusion".
- **Consequence:**
  - a captured value's fate is served `not_analyzed`;
  - a failed negative names an unrelated reason;
  - each phase-4 producer must re-add `missing_evidence` for value approximation, which restates the verdict rule per producer (F07).
  - Because the four legacy rankings become "declared quirks" of phase-4 adapters (`obligation.rs:151–153`), parity would not flag the served change.
- **Correction** (owner: `obligation`):
  - move `ScopeBoundary` to the Model class;
  - add obligation kinds for incomplete coverage and approximated refutation;
  - separate value approximation (open until discharged) from condition approximation in `VerdictInput`;
  - keep `ResponseBudget` out of verdict inputs (renderings mark truncation);
  - record the priority rationale (O10).
- **Closure evidence:** verdict cases for each rule. Cheap now; before WP4.2.

<a id="C10"></a>
### C10 — The receiver binding cannot express the Tested receiver rule

- **Principles · judgment/gate:** DP-02 · A2, G2. Tested semantics: ADR-0045:147–150.
- **Evidence:**
  - `calls.rs::Receiver:300–307` offers `None` or `Bound`, and its doc says "called through an instance **or class** attribute";
  - `bind:413–421` shifts on `Bound`;
  - ADR-0045 allows the shift only with Pysa's `true_with_object_receiver` **and** an attribute callee, and says "Class receivers, unsupported callees and disagreement stay unknown".
- **Consequence:** phase 3 must choose `None` or `Bound` for `C.method(obj, x)` or an undecided receiver. `Bound`, as documented, binds `C` to `self` and shifts every argument, producing wrong formals with no obligation. The rule deciding `Bound` has no owner.
- **Correction:**
  - add `Receiver::Unknown`, which yields ambiguous receiver and positional bindings (`ambiguous_binding`);
  - add a `receiver_of(provider flag, callee syntax)` function in `calls.rs` that owns ADR-0045's rule;
  - fix the doc.
- **Closure evidence:** binder known answers for a class-attribute call, a static method and an unknown receiver. Before WP3.1.

<a id="C11"></a>
### C11 — Phase-2 identity contracts are unsettled

- **Principles · judgment/gate:** DP-04, DP-02 · A2, G1.
- **Evidence.**
  - §15.3's occurrence `(module entity, …)` and entity `(kind, owner entity, name, declaring occurrence)` are mutually recursive for a module. `IdKind::Entity` has no recipe (`id.rs:188–207`, `recipes::ALL:382–391`).
  - `recipes::occurrence:320` takes `syntax_kind: &str`, while `vocab::innermost_region_join:356–360` hard-codes `"name"`, `"attribute"` and `"subscript"`.
  - Atoms are `(evaluation, kind, operand, argument)` with no discriminator (`condition/atom.rs:164–171`). ADR-0082 dropped the provider predicate digest.
  - `AccessPath::encode:214–235` writes attribute names unescaped.
  - The sample model has an L0 relation referencing an L1 relation (`tests/decl_sample.rs:52`).
- **Consequence:**
  - An extractor spelling that differs from the join's literals turns every join `Unmatched`: a silent coverage gap, not an error.
  - Two synthetic predicates with fixed text attributed to one occurrence (for example both suppression predicates of `with a, b:`) collapse into one BDD variable, which correlates independent conditions. It surfaces only as a legacy-ID uniqueness failure during dual-run.
  - `getattr(x, "a.b")` modelled as an attribute has the same place id as the path `.a.b`.
- **Correction** (owner: `id`, `vocab`, WP2.1):
  - module entities keyed without a declaring occurrence (release and module path);
  - a `SyntaxKind` codebook used by both the recipe and the join;
  - an atom's evaluation must be the innermost evaluating occurrence (withitem, pattern), plus an in-memory check that each provider predicate maps to exactly one atom id per generation;
  - escape or length-prefix attribute segments;
  - occurrences reference the L0 module relation.
- **Closure evidence:** WP2.1's recipes with known answers and the one-predicate-one-atom validator.

<a id="C12"></a>
### C12 — The owner rule and the region join have no relational form

- **Principles · judgment:** FP-04, DP-10 · A3.
- **Evidence:** `vocab.rs::owner_of:305–315` and `innermost_region_join:349–376` take one span and a slice of every region or occurrence. The call policies have a Rust and a generated SQL rendering; these do not.
- **Consequence:** phases 2 and 3 must either call them per row over millions of occurrences or restate "caller" in DataFusion SQL. That recreates the second owner definition that §15.4 says is "the only definition of caller" (F02's `graph::owner_of` duplication).
- **Correction:** a batch operator in `lctx-model` over sorted Arrow spans (a sweep per module, with the same tie-breaks), used by the producing stage. The per-item function stays as its oracle.
- **Closure evidence:** the operator equals the per-item function on shuffled input. Before WP2.2/WP3.1.

<a id="C13"></a>
### C13 — Derivation sources restate premise relations and do not enforce premise existence

- **Principles · gate:** CI-11, DP-01 · CI-G2, G1.
- **Evidence:**
  - `derivation.rs::Premise:17–24` names `relation` independently of the column's role reference;
  - `validate:47–75` checks names only;
  - the sample declares `binding_id` with relation `transfers` and no reference (`tests/derivation.rs:32, :47`);
  - `ddl::view:561–566` emits no GRANT.
- **Consequence:** "why" can cite a premise absent from the generation (with C08, every pre-merge id), and the generated views are unreadable by `lctx_serving`.
- **Correction:** premises must be declared role references (`[ref r.c]`), and `Premise.relation` derives from them, so the FKs enforce closure. `view()` emits the reader grant.
- **Closure evidence:** `derivation::validate` refuses a non-reference premise; a store test reads the views as the reader. Before WP4.7.

<a id="C14"></a>
### C14 — D14 amends §15.1's owner line in the plan only

- **Principles · gate:** DP-01 (documentation authority) · G1.
- **Evidence:** plan §4.1 D14 ("amending §15.1's owner line") against `semantic-model.md:63–64` ("`lctx-postgres` owns … provider registration").
- **Consequence:** two owners are named for provider registration. The sound reason (the PyO3 `lctx_storage` wheel must not link DataFusion) lives only in the plan.
- **Correction and closure:** amend §15.1 with D14's reason in the next documentation commit.

**Foundation and supporting-rule verdicts.**

| Foundation | Verdict | Basis |
|---|---|---|
| FP-01 | satisfied | Pure model, store effects and reads each have one owner |
| FP-02 | unresolved | C02, C04, C10 |
| FP-03 | unresolved | C05, C12 |
| FP-04 | violated | C03, C08, C11, C13 |
| FP-05 | violated | C01, C07 |
| FP-06 | satisfied | Every semantic operator is tested store-free |

| Supporting rule | Verdict |
|---|---|
| DP-19 | violated (C01) |
| DP-24 | violated (C02) |
| DP-13 | satisfied |
| DP-18 | satisfied |
| DP-11 | satisfied (canonical order and deterministic tie-breaks) |
| DP-12 | satisfied for the kernel |
| CI-02 | violated (C04) |
| CI-06 | violated (C05, C06, C09) |
| CI-12 | satisfied |

**Observations** (no finding; examined and unsettled or deliberate).

- **O1.** Role names appear in five places. D5 accepts this until phase 5. A test asserting `CANONICAL.reader == Role::Serving.name()` would reconcile the Rust sites.
- **O2.** `ddl_digest` certifies rendered text, not the live catalog. Probe case 5: `DROP SCHEMA lctx_g… CASCADE` on a published generation silently drops the **parent's** FK for every generation. Only the owner can run it, and `retire` never reaches it with attached partitions. A `store verify` against `pg_constraint` would close it.
- **O3.**
  - `reset` reads the registry before its transaction (`store.rs:95–99`), so a concurrently created generation leaves an orphan schema.
  - Selection is last-writer-wins, so a zombie writer can select an older generation.
- **O4.** Co-declared legacy and model codebooks (`Origin`/`CallOrigin`, `Modality`, `InvocationPhase`/`CallPhase`, `Verdict`) have no code-and-text equality test like `obligation_mapping.rs`.
- **O5.** In parity:
  - divergence predicates are unpaired and unbounded;
  - the report aggregates `declared` per relation, so per-divergence counts would aid review;
  - `diff` holds every row encoding in a `BTreeMap` outside the DataFusion memory pool (DP-20; performance is not a gate).
- **O6.** `Schedule.empty` (published relations whose writer the profile skips) is not persisted. WP1.2 receipts should record it, so that an empty relation reads as "not requested" (CI-04).
- **O7.** `Recipe` metadata carries field names, not types. A recipe bound to a column of another type fails only at recompute.
- **O8.** biodivine 0.6.3's bounded single-variable ∃ is `fused_binary_flip_op_with_limit(limit, (b, None), (b, Some(v)), None, or)`, the same operation `var_exists` uses. The kernel's restrict-restrict-or is correct and bounded, and this would do it in one pass.
- **O9.** `family`, `coverage`, `polarity` and `fidelity` have no consumer yet, and `coverage` is free text. Type them when §15.8's coverage gating lands.
- **O10.** The budget-first obligation priority has no recorded rationale.
- **O11.** `CallBinding::obligation` maps `refused` (a variant that does not apply) to `ambiguous_binding`. With several signature variants, a refusal is a variant filter.

## 8. Library fit and total complexity

| Capability and owner | Consumer | Candidates | Pinned fit and gaps | Burden | Choice |
|---|---|---|---|---|---|
| Bounded BDD operations, ∃ (`condition`) | Composition, verdicts | biodivine 0.6.3 `binary_op_with_limit`, `check_binary_op`, `exists` (unbounded), `fused_binary_flip_op_with_limit` | Limited apply fits. `exists`/`binary_op_with_exists` have no limit, so restrict + bounded `or` is justified. The fused limited flip is a one-pass refinement (O8) | Already a dependency | Keep; optional refinement |
| Simultaneous substitution | `compose_call` | biodivine `substitute` (single variable, unbounded) | Unfit (ADR-0082, P3) | Own ~180 lines, bounded, tested | Keep |
| Multiset diff (`legacy::parity`) | Parity | arrow-row; DataFusion `GROUP BY` all columns + counts + full join | arrow-row is exact and simple. DataFusion would run under the memory pool (O5). DataFusion's EXCEPT ALL is not relied on | Temporary code | Keep; revisit if a pilot relation exhausts memory |
| Binary COPY (`store`) | Writer | pgpq 0.12 | Types asserted equal to the DDL (`generated_column_types_equal_the_copy_encoding`) | Small | Keep |
| Provider reads (`store_read`) | Canonical reads | Owned `datafusion-table-providers` fork, `declared_table` | List items decode nullable; `cast` refuses forbidden nulls (a type-matrix receipt). No catalog check (C02) | Owned fork | Keep; add the digest check |
| Scheduling and acyclicity (`stage`, `derivation::acyclic`) | Stage table, premises | petgraph `toposort`, `is_cyclic_directed` | petgraph's order follows insertion, not a declared tie-break; `lctx-model` stays free of petgraph | ~40 lines | Keep |
| Partition lifecycle (`store`) | Generations | PostgreSQL 18 list partitioning, ATTACH/DETACH CONCURRENTLY | Fits, with the self-reference and pending-detach constraints the probe showed (C01) | — | Keep; adapt the protocol |

## 9. Alternatives and tradeoffs

| Alternative | Change propagation and local reasoning | Semantic authority and composition | Test / substitution boundary | Machinery and operational risk | Evidence, decision, revisit |
|---|---|---|---|---|---|
| Current baseline (phase 0 as built) | Seams hold for declarations, identity, kernel and binder | Duplicate stage fact; unowned site aggregates; composition rules incomplete | Store-free unit tests | Retire fragile | This review: Revise |
| Proposed corrections (C01–C14) | Same seams; site aggregates, receiver rule and owner batch operator move into `lctx-model` | Removes the second authorities; composition gains a full binding set | Same | One lifecycle state, one FINALIZE path | Recommended |
| Self-references validated in DataFusion instead of the database | No DB FK for `fk.relation == name` | One declaration still generates both | A semantic validator (generated from the declaration) | Simplest retire | Preferred to row deletion. Revisit if a consumer needs DB enforcement of self-references |
| Transfer identity including the condition (keep) | Ids churn with alternatives | Contradicts the merge key | — | Re-keying pass after every fixpoint | Rejected (C08) |
| Site aggregates computed in phase-3 SQL (keep) | Each consumer's SQL encodes uniqueness | Second admission authority (F02) | — | — | Rejected (C04) |

## 10. Verification and uncertainty

| Claim or scenario | Evidence label / date | Inspection, test or probe | Conditions and expected result | Outcome |
|---|---|---|---|---|
| Phase-0 targeted suites | **Tested — historical receipts**, author session, 2026-09-29 (commit messages `d9832c6`, `0d9fc8a`, `fe0e99a`, `4e3418e`, `f8870ce`; plan §4.2) | `cargo nextest run -p lctx-model --release` (64 passed); `-p cpg-core --test model_policies` (1 passed, 5,040 combinations); `-p cpg-schema --test obligation_mapping` (2 passed); `-p lctx-postgres --test store` (7 passed, real PG18); `-p cpg-core --test store_read` (1 passed); `--lib session` (2 passed); `--test stage` (5 passed); `--test dependency_audit` (2 passed); `--lib parity` (1 passed, 7 controls); `lctx parity self-test` (7 behaved) | Receipts not re-run in this review | `not_run` here; receipts as recorded |
| **Probe P1** (self-referencing FK) | **Tested (probe)**, this review, 2026-09-29 | `docker run --rm postgres:18.6-trixie` (PostgreSQL 18.6; the repository pins `pgvector/pgvector:0.8.6-pg18-trixie`, the same major version); psql scripts in the Appendix | Case 1: ATTACH with in-partition references succeeds. Case 1b: a dangling reference is refused (23503). Case 2: DETACH CONCURRENTLY is refused. Case 3: DETACH is refused. Case 4: DROP TABLE is refused. Case 5: DROP SCHEMA CASCADE drops the parent FK. Case 6: TRUNCATE is refused. Case 7: DELETE then DETACH CONCURRENTLY succeeds | ATTACH `passed`; detach `failed` for the current protocol (C01); case 5 recorded as O2 |
| **Probe P2** (interrupted DETACH CONCURRENTLY) | **Tested (probe)**, 2026-09-29 | A second session holds a REPEATABLE READ snapshot on the parent; `SET statement_timeout='3s'` then DETACH CONCURRENTLY | Detach pending; parent read of the generation returns 0 rows; retry and another generation's detach both refused, citing FINALIZE; ATTACH allowed; FINALIZE completes | As listed; the protocol has no recovery (C01) |
| Recursive CTEs bounded by the session pool | **Interface-checked**, 2026-09-29 | DataFusion 55.1.0 `recursive_query.rs:316–348` (`MemoryConsumer::register`, `try_grow`) | Exhaustion is an error | Not tested (a sort is tested) |
| C05, C06, C08, C09 semantic defects | Implemented code read, 2026-09-29 | Source lines cited | — | Not executed; the reasoning is direct from code |
| Missing known-answer shapes | Gap | — | Derived caller or callee through a field read; cross-argument mutation; missing stability link; callee-local guard; caller-selected branch; provider agreement and disagreement; `C()` new + init; `map(f, xs)`; unknown receiver; self-referencing retire | To add with their corrections |
| Independent controls | Unchanged | The Pysa TITO control, flow-soundness oracle and runtime challenges remain in legacy until their phase (plan §3.7) | Parity never substitutes for them | — |

## 11. Authority changes and dispositions

**Proposed plan rows.** These should be added to the cutover plan's findings table (binding §4: one disposition owner). This review cannot edit the plan.

| Required change | Decision route and owner | Source findings | Proposed disposition | Closure evidence or revisit trigger |
|---|---|---|---|---|
| Retire state machine, FINALIZE recovery, self-reference route, state-checked parent reads | Implementation (`lctx-postgres`) within ADR-0083 | C01 | open → before WP1.3/WP1.9 | Store tests (a)–(d) in C01 |
| Reader digest check | Implementation (`cpg-core::store_read`) | C02 | open → WP1.3 | Mismatch refused |
| Typed stage references; one producing-stage authority | Implementation (`lctx-model`) | C03 | open → before WP1.2 | Mismatch refused |
| Policy facts, site aggregates, higher-order exclusion | ADR-0082 follow-up amending §15.5; `calls` | C04 | open → WP2.1 (fields) / WP3.1 | Extended matrix and shapes |
| Composition place, condition and selection contracts | ADR-0082 follow-up amending §15.6; `transfer` | C05, C06, C07 | open → before WP4.2/4.3 | Shapes in §10 |
| Transfer identity = merge key | Amend §15.3 (same follow-up) | C08 | open → now (cheap) | Merge keeps the id |
| Verdict classes and inputs | Implementation (`obligation`); a note in §15.8 | C09 | open → now (cheap) | Verdict cases |
| Receiver unknown and its rule | Implementation (`calls`) | C10 | open → WP3.1 | Binder cases |
| Phase-2 identity (entity, syntax kind, atom discriminator, path escaping) | WP2.1 declarations | C11 | open → WP2.1 | Recipes and validator |
| Owner and region-join batch operator | Implementation (`vocab`) | C12 | open → WP2.2 | Equality on shuffled input |
| Derivation premises as references; view grants | Implementation (`derivation`, `ddl`) | C13 | open → WP4.7 | Validator and reader test |
| §15.1 owner line | Docs amendment | C14 | open → next docs commit | Section text |

**The plan's §4.2 "deviations to confirm in WP0.10"**, answered:

| Deviation | Answer |
|---|---|
| No provider-fork patch (nullable list items read and cast) | Confirmed. The cast refuses a forbidden null |
| Kernel schema `lctx_store` and generated `lctx` | Confirmed |
| `validate_relation` called once per relation | Confirmed |
| Publish grants the reader SELECT on partitions | Confirmed. It makes partition-pinned reads fail after retire. Retire must revoke first (C01) |
| Retire referrers first | Confirmed as an order. It is insufficient for self-references and interruptions (C01) |
| `Occurrence` root covering arguments and call results | Confirmed. `compose_call` needs it |
| `summary` ⊆ `dataflow` | Confirmed |
| A budget obligation's verdict is unknown | Confirmed. `scope_boundary` must follow the same rule (C09) |
| `IdKind` gains `ddl` and `projection` | Confirmed (domain separation for digests) |
| Legacy stage table re-sequenced to WP1.2 | **Sound.** A stage table needs its enforcing consumer: input-restricted sessions and read recording beside the extracted stage bodies. Declaring it in WP0.7 would have produced an unchecked declaration. The WP0.7 audit pins the four known undeclared reads for WP1.2. Add to WP1.2's "done when": typed stage references (C03), `published(table)` equals the install list, and the persisted empty set (O6) |
| F11 producer identity | Confirmed for phase 0. The hard-coded `serving_only` path list in `build.rs` is interim, until per-stage code identity replaces it |

No exception records (§H) are requested.

## 12. Architectural judgment and decision

| Judgment | Verdict | Scenario evidence and scope | Required action / disposition |
|---|---|---|---|
| A1 Localize change | **satisfied** | `lctx-model` is pure, and every semantic operator is testable without a store. Store effects sit in `lctx-postgres`, reads in `cpg-core` (D14). S4 (a contract change) is one declaration edit plus the digest-enforced reset. A policy change is one predicate edit, with SQL and Rust tested equal. Retire failures (C01) are a correctness defect inside the right owner, not a locality defect | — |
| A2 Encode meaning structurally | **violated** | S1: the producing stage has two authorities (C03). S2: the site aggregates that decide call admission are unowned and would be restated in phase-3 SQL (C04). The receiver rule is unowned (C10). Transfer identity contradicts the merge key (C08). Premise relations are restated (C13). Control/selection is unowned (C07) | C03 before phase 1 relies on stages; the others by their owning phase (§11) |
| A3 Extend through composition | **unresolved** | S3: `compose_call`'s one-binding contract cannot compose cross-argument effects without a contract change (C05). S1/S2: the owner rule and region join lack the form their producers need (C12). The declaration, policy, kernel and adapter seams otherwise compose as intended | C05 and C12 by their owning phase |

**Bounded decision: Revise.**
- **Why.** The phase-0 core is the right foundation to evolve from:
  - one declaration authority with generated DDL, codec, validators and install digest;
  - v2 identity with a presence-byte encoding that equals the UDF;
  - a ported kernel with bounded operations and truncation-marked rendering;
  - one binder and one policy predicate with two tested renderings;
  - a store with one-transaction publication, privilege immutability and fail-closed digests;
  - a stage scheduler with refusal controls;
  - a parity harness with self-tests.

  Nothing requires a rewrite. However:
  - G1, G2, G5, G6 and CI-G1 fail on specific, cheap points;
  - A2 is violated;
  - the store's retirement protocol fails on real PostgreSQL behavior the probe demonstrated.
- **Required before phase 1 relies on the store:** C01, C02, C03.
- **Recommended now** (small edits to phase-0 code; otherwise restated by later producers): C05(a), C08, C09.
- **Contract evolution permitted until the owning phase**, with the triggers in §11: C04, C05(b), C06, C07, C10–C13.
- **Documentation:** C14.
- **WP0.10's exit:** "accepted, or its revisions applied" is met when C01–C03 are applied and the rest are entered in the plan's findings table with their triggers.

**Enclosing architecture (§15 target):** accepted for scenarios S4 and S6 and for the declaration, identity, kernel, binder and publication seams, at Implemented/Tested (historical receipt) strength. It needs revision at §15.5 (C04), §15.6 (C05–C07), §15.3 (C08) and §15.8 (C09) before their phases. Phases 1–5 are not assessed (not built). This slice does not certify the assembled subsystem, and review acceptance is not release qualification.

| Priority | Change and responsible component | Source findings | Closure evidence or revisit trigger |
|---|---|---|---|
| 1 | Retire protocol and state-checked reads (`lctx-postgres`) | C01 | Store tests (a)–(d) |
| 1 | Digest check for readers (`cpg-core::store_read`) | C02 | Mismatch refused |
| 1 | Typed stage references and one producing-stage authority (`lctx-model::stage`, `decl`) | C03 | Mismatch refused |
| 2 | Verdict classes and inputs; transfer id; derived-path rule (`obligation`, `id`, `transfer`) | C09, C08, C05(a) | Verdict, merge and derived-path cases |
| 3 | Policy facts and site aggregates; receiver rule (`calls`) | C04, C10 | Admission matrix and binder shapes |
| 3 | Phase-2 identity; batch owner and join (`id`, `vocab`) | C11, C12 | WP2.1 recipes; operator equality |
| 4 | Composition contracts; selection; derivation premises (`transfer`, `derivation`, `ddl`) | C05(b), C06, C07, C13 | Shapes in §10; reader-grant test |
| 5 | §15.1 owner line | C14 | Section text |

**Next step:** the cutover plan's owner applies C01–C03 (with C05(a), C08, C09 if taken now), enters the remaining rows in the plan's findings table, and then closes WP0.10.

## Appendix — Probe SQL (PostgreSQL 18.6, 2026-09-29; run in a disposable container, not committed)

```sql
-- P1: self-referencing, generation-qualified FK on a list-partitioned parent
CREATE SCHEMA lctx;
CREATE TABLE lctx.entities (generation_id bytea NOT NULL, entity_id bytea NOT NULL, owner_id bytea,
  CONSTRAINT entities_pk PRIMARY KEY (generation_id, entity_id),
  CONSTRAINT entities_fk0 FOREIGN KEY (generation_id, owner_id) REFERENCES lctx.entities (generation_id, entity_id)
) PARTITION BY LIST (generation_id);
CREATE SCHEMA g_a;
CREATE TABLE g_a.entities (generation_id bytea NOT NULL DEFAULT '\x0a'::bytea, entity_id bytea NOT NULL, owner_id bytea,
  CONSTRAINT "partition" CHECK (generation_id = '\x0a'::bytea));
INSERT INTO g_a.entities (entity_id, owner_id) VALUES ('\x01', NULL), ('\x02', '\x01'), ('\x03', '\x02');
CREATE UNIQUE INDEX entities_pk ON g_a.entities (generation_id, entity_id);
ALTER TABLE lctx.entities ATTACH PARTITION g_a.entities FOR VALUES IN ('\x0a'::bytea);   -- succeeds
ALTER TABLE lctx.entities DETACH PARTITION g_a.entities CONCURRENTLY;                     -- ERROR 23503 "still referenced"
ALTER TABLE lctx.entities DETACH PARTITION g_a.entities;                                  -- same error
DROP TABLE g_a.entities;                                                                  -- refused: parent FK depends on it
DROP SCHEMA g_a CASCADE;                                                                  -- drops lctx.entities' FK constraint (O2)
DELETE FROM g_a.entities; ALTER TABLE lctx.entities DETACH PARTITION g_a.entities CONCURRENTLY;  -- succeeds

-- P2: interrupted DETACH CONCURRENTLY
-- session A: BEGIN ISOLATION LEVEL REPEATABLE READ; SELECT count(*) FROM lctx.samples; SELECT pg_sleep(12); COMMIT;
SET statement_timeout = '3s';
ALTER TABLE lctx.samples DETACH PARTITION g_a.samples CONCURRENTLY;   -- canceled; pg_inherits.inhdetachpending = t
SELECT count(*) FROM lctx.samples WHERE generation_id = '\x0a';       -- 0
ALTER TABLE lctx.samples DETACH PARTITION g_a.samples CONCURRENTLY;   -- ERROR already pending detach (use FINALIZE)
ALTER TABLE lctx.samples DETACH PARTITION g_b.samples CONCURRENTLY;   -- same ERROR for another generation
ALTER TABLE lctx.samples DETACH PARTITION g_a.samples FINALIZE;       -- completes
```
