# Inquiry B: Compiler read path and catalog-profile stages

**Provenance.** This is the verbatim final report of supporting inquiry B, from one read-only
`design-reviewer` agent, for the [codebase efficiency-alignment review](../../reviews/design_review_efficiency-alignment-codebase_2026-10-11.md).
It was produced on 2026-10-11 at baseline `88f33bc0`, against the [inquiry briefs](inquiry-briefs.md).
Only this header was added.

How to read it:
- Candidate labels such as `B-01` are local to this report. They are not the review's finding
  IDs.
- Its judgments are bounded and supporting. The principal review owns the combined decision.
- See the [evidence index](README.md) for coordinator checks and reconciliation.

---

# Inquiry B: compiler read path and catalog-profile stages

Supporting assessment for the codebase efficiency-alignment review. Baseline is `main` at `88f33bc0`, 2026-10-11. The work was read-only: static source inspection plus read-only extraction of phase timings from existing `build/runs/*/output.log` files. Candidate labels (B-nn) are local. The principal reviewer assigns finding IDs.

## 1. Scope examined and coverage limits

**Read in full or at the decisive grain:**
- **`cpg-core`:**
  - `compilation.rs` (`UpperStage`, `PreparedCompilation::schedule`, `compile_driver`, `freeze_completed_inputs`, product-dependency helpers).
  - `workspace.rs`: `CompletedInputs::{session, prepare_session, predicate_session, query_template}`, `stage_inputs_selected`, `freeze_inputs_async`, `admit_semantics`, `validate_scope`, `validate_references`, `validation_premise`, `admit_frontier`, `distinct_reference_views`.
  - `sql.rs`.
  - `consumed_rows.rs`: `stream_artifact_admission`, `ConsumedInputs`, `stream_at`/`stream_where_at`, `PreparedEdges::{native_forward_targets, native_reverse_targets, batch_inner, grain, grain_roots, discover}`, `PreparedClosure::{select, ordered_input}`, and the edge-preparation `prepare`.
  - `catalog_core.rs`, `catalog_core_scope.rs`, the `catalog_evidence.rs` root loop, `local_semantics::run`, `semantic_execution::evaluate_base`, `analytic::produce`, `final_coverage.rs`, `artifact::admit`, `artifact_manifest::{populate, semantic_outcomes}`, `native_canonical::Lookup`, `workspace_products::product_request`.
  - Root loops in `normalize/mod.rs` (callable kernel), `normalize/receiver_scope.rs`, `normalize/call_scope.rs`, `scoped_inventory.rs` and `scoped_admission::root_predicate`.
- **`lctx-surrealdb`:**
  - `compiler_provider.rs` in full: `NativeTable`, `SupportedFilter`, `NativePartition`, `selected_batches`, `scan_batches`, `batches_from_rows`, `NativeExec`, and the tests module.
  - `compiler.rs`: `scan_rows`, `row_tokens_inner`, `prepare_memberships`, `scan_bound_rows_inner`, `MembershipLookup`, `MEMBERSHIP_KEY_QUERY`, `decode_membership`, `SelectedRows`, `registered_view`, `view_owners`, `bind_inner`, `completed_state*`, `state_identity_selected`, `validate_backing_batch`, and the schema DDL for `compiler_membership`.
  - `schema.rs` (`ScopeTable`, `atomic_scope_field`, `SCOPE_FIELDS`) and `ordered_rows.rs` (`Run`, `PreparedCandidates`).
- **`lctx-model`:** `analysis/expected.rs` (`inputs`, `contract`, `FrontierIndex::domain`, `CoverageAdmission`), `analysis/family/coverage.rs::admit`, `stages.rs::{is_epoch_shared, is_vocabulary, PublicationBoundary}`, `graph.rs::admit_entity`, `source.rs::CoverageScope`.
- **`lctx-serving`:** `behavior.rs`, for the NotRequested consumer only.
- **DataFusion 55.1.0 pinned source:**
  - `datafusion-expr-common/src/type_coercion/binary.rs`: FixedSizeBinary/Binary coercion.
  - `datafusion-expr-common/src/casts.rs` and `datafusion-optimizer/src/simplify_expressions/unwrap_cast.rs`: Binary→FixedSizeBinary unwrap, including `InList`.

**Not read, or only skimmed:**
- Normalization kernels other than the callable loop.
- `base_scope`, `source_call_scope`, `scoped_execution`/`scoped_aspects`/`scoped_retrieval` internals.
- `catalog_selection`, `synthesis`, `retrieval` and `retrieval_preparation` beyond their grain loops.
- `kernel_products` (inquiry C's area).
- `lctx-model` scope-program compilers (`compiler_scope_program`, `catalog_scope_program`, `normalization_scope_program`).
- `projected_arrow` internals.
- `ProducerOutput` registration and completion (inquiries A and D).
- `prepare_canonical`/`scan_canonical` internals and `freeze_for_admission`.

I did not load the `datafusion` skill or query Context7. DataFusion semantics were settled from pinned registry source (safeguard 1). SurrealDB 3.3 index-planner behaviour for `IN` over compound indexes is **not** settled; it is flagged where it matters.

## 2. Complete operations examined

**O1: upper-stage execution unit, catalog profile.** `compile_driver` runs all 21 `UpperStage::ALL` bindings (`PreparedCompilation::schedule` admits every Analysis-phase binding plus Catalog-phase ones). Each binding does the following:
- `stage_inputs_selected` builds a `CompletedInputs`. This is CPU only.
- `producer` and `product_request` are CPU only.
- `reuse_product` is a 1-RT miss when cold.
- The kernel runs.
- `finish`/complete happens in the native completion path (inquiry A/D).
- `freeze_completed_inputs`: at each publication boundary, `freeze_inputs_async` calls `native.bind` once per epoch-shared relation. These are the vocabulary relations, about 17 (`stages::is_epoch_shared`). Each bind costs about 9 RTs: one `SELECT`, a `guarded_effect` and a `hold`.

Under catalog, seven behavioural owners run as full stages that end in `NotRequested`: Local, Base, Completion, SourceCalls, Enriched, Models, Summary. Analytic and Embedding also run with nothing requested.

**O2: read loop for one completed input.**
1. `CompletedInputs::session` builds one `SessionContext` per input set, with a copied config, the shared `RuntimeEnv` and `NativeTable` registration. There is no I/O.
2. SQL text goes to `sql::query` (or `query_template`) and DataFusion plans it. `supports_filters_pushdown` returns Exact for supported predicates. Projection reaches `body.<field>`. `scan` ignores the `_limit` argument; `with_fetch` is delegated to `StreamingTableExec`.
3. `scan_rows` → `scan_bound_rows_inner`:
   - **No predicate, or a plain SQL predicate:** `prepare_memberships` runs once per view identity per attempt. It issues one streamed query per owner over `contribution_rows`, sorted into a file-backed spillable `Run` and cached in `membership_preparations`. Then `SelectedRows` issues one payload RT per 128 candidates (`SELECT … FROM $__compiler_nodes WHERE semantic_type=$relation AND (<pred>)`).
   - **Explicit keys:** one RT per 128 keys for a single-owner view, **one RT per key for a multi-owner view**, plus payload.
   - **Atomic scope field:** the `by_scope` index with a correlated membership subquery.
4. `projected_arrow::ProjectedBuilder` accumulates up to `batch_rows` (4,096) rows or 8 MiB before emitting a `RecordBatch`.
5. Visitors decode with `R::decode` (serde_arrow) into typed rows.

**O3: grain-shaped kernels.**
- `catalog_core::produce` reads `SELECT id FROM names ORDER BY id`. For every 32 roots it calls `PreparedEdges::grain(root, "id IN (32 literals)")` (a seed query), then discovers edges through the explicit-key `select_table`, then `load_scope`. That load issues one `PreparedClosure::select` per relation in `catalog_inputs!` (about 30 relations), each an explicit-key provider plus a SQL parse.
- Normalization callable kernels group 32 Physical roots and call `batch_with_cancellation`. Its presence check is `SELECT … WHERE id IN (…)`.

**O4: normalized admission (`admit_semantics`) and artifact admission.**
- `validate_scope` creates a fresh session over every completed relation and runs `validate_references`: one LEFT JOIN per reference field, memoised by an exact premise.
- Per invariant it runs a `SELECT id … LIMIT 1` emptiness probe, then the check. Premises are memoised in `checked_premises`.
- `artifact::admit` then runs:
  1. `freeze_for_admission`;
  2. `facts_availability_async`;
  3. a second `admit_semantics`;
  4. `admit_frontier`, which reads `SELECT * … ORDER BY id` for every frontier-invariant input;
  5. `prepare_canonical`;
  6. `Lookup::load`, which holds every entity header and assertion ID in an in-memory `BTreeMap`/`BTreeSet`;
  7. a full entity scan then a full assertion scan with `admit_entity`/`admit_assertion` and `FamilyHasher`;
  8. `artifact_manifest::populate`, which calls `NativeCompilerStore::completed_state`. That is a full scan of all six `STATE_TABLES`, one row at a time (`next_batch(1)`), with `serde_json` per row into a `KeySink` and `validate_backing_batch` over `compiler_record`.

**Live state across the compile.** Membership runs are file-backed per scanned view for the attempt's lifetime (disk, not RAM). `Lookup` grows with the final graph (H17). Each stage's `FrontierIndex` is charged and dropped per stage.

## 3. Candidate findings

### B-01: `id` predicates on completed native tables scan the whole root membership run, once per grain

**Observed cause.** `compiler_provider::SupportedFilter::interpret` accepts `id = lit` and `id IN (…)`. `SupportedFilter::driver` yields a driver only for `atomic_scope_field`, and `id` is not one (`schema::ScopeTable::layout` admits target references and `SCOPE_FIELDS` only). So `NativeTable::scan` produces `NativePredicate::Sql { "semantic_key IN $v" }`.

`compiler.rs::scan_bound_rows_inner` treats `Sql` as non-selective. It takes the `prepare_memberships` run, which is cached, and builds `SelectionKeys::Verified`. `SelectedRows::next`/`start_payload` then issues one payload RT for every 128 candidates of the **entire view**, filtering server-side.

At DataFusion 55.1 the literal `X'..'` (Binary) is coerced against the `FixedSizeBinary(16)` column through `Binary`. `unwrap_cast` then unwraps both `=` and `InList` back to FixedSizeBinary (`datafusion-expr-common-55.1.0/src/casts.rs`; `datafusion-optimizer-55.1.0/src/simplify_expressions/unwrap_cast.rs`). So the predicate most likely reaches the provider as Exact (Interface-checked), and the full-run iteration is the result.

Production callers on this route:
- `catalog_core::produce`: `id IN (32)` per grain, through `PreparedEdges::grain_roots`.
- `consumed_rows::PreparedEdges::batch_inner`: the presence query `SELECT … WHERE id IN (…)` for every Physical-root batch. It is reached from the 32-root normalization callable loops in `normalize/mod.rs`.
- `scoped_inventory::validate_inventory`: one `grain` per root during admission.

The per-root `grain` helpers in `receiver_scope`, `call_scope` and `catalog_evidence_scope` are `#[cfg(test)]` or test-only. Search: `rg '\.grain\(|grain_roots\(|id IN \(|id=X'` over `crates/cpg-core/src`, then checking whether each caller sits under `cfg(test)`.

**Consequence.** Seeding or presence-checking g grains over a root relation of N rows costs about g·⌈N/128⌉ RTs and g·N server-side node reads. With g = N/32 that is **quadratic in the root count**. This holds for public names in C0 and for callables and overloads in normalization. For S01 the magnitude is unmeasured. Under S02 (5–20× first-party code) the growth is structurally quadratic.

This is a CI calibration case of "bounded answer, amplified work". Mechanisms: H5/H7/H8/H9/H10; FP-07, DP-10, CI-08; A4.

The selective path already exists and is unused here: `select_table` with `NativePredicate::Keys`, and `PreparedClosure::select`.

**Simpler conforming alternative.**
1. In `NativeTable::scan`, recognise a mandatory conjunct `id = lit` or `id IN (lits)` as an explicit-key driver: `NativePredicate::KeysSql` with the residual SQL. This mirrors the atomic-field driver rule, where OR supplies no driver.
2. For presence-only demand (`batch_inner`), merge-intersect the sorted requested keys with the cached verified membership run. `row_tokens_inner` already does this when demand is at least the view's rows. No payload RT is needed.

Either fix makes root seeding proportional to the grain.

**Preservation constraints.**
- Exact absent/present outcomes, including virtual roots.
- Sorted, distinct key order.
- Foreign and duplicate backing detection.
- The OR-is-not-a-driver rule.
- Empty demand still checks the exact registered view, as `scan_bound_rows_inner` already does for `Keys(vec![])`.
- Budget charges on key vectors.

Counterexample check: `id IN (…) OR other` must stay a residual. Keys on a multi-owner view currently degrade to one RT per key (B-02). If those views are common, B-01's explicit-key form must land with B-02, or B-01 should use the membership-run merge.

**Uncertainty.** Row counts for FastMCP are not known (op-map §5.6). Whether DataFusion keeps a residual `CAST` in some forms was checked in pinned source but not executed. Either way the work is O(N) per grain: if the filter were Unsupported, the full id column would be transferred instead.

**Evidence label.** Diagnosis Implemented/static-inspected; DataFusion coercion Interface-checked; remedy Proposed.

**Closure evidence.** A planning-only control in the style of `compiler_provider::tests::textual_predicates_push_down_before_projection`, showing that `WHERE id IN (X'…')` yields an explicit-key partition. A focused native control with a view of more than 128 rows showing that payload RTs scale with demand, not view size. Existing absent/virtual-root controls (C-F01 family) must still pass.

### B-02: multi-owner explicit-key lookups cost one RT per key and scan memberships across retained history

**Observed cause.** `compiler.rs::MembershipLookup::collect` uses point record IDs (`membership_id`) only when `owners.len() == 1`. Otherwise it issues `MEMBERSHIP_KEY_QUERY` once per key:

```sql
SELECT … FROM compiler_membership WITH INDEX member_keys WHERE relation=$relation AND semantic_key=$key
```

`decode_membership(…, allow_foreign = true)` then discards rows from foreign contributions **client-side**. The index `member_keys` is `(relation, semantic_key, contribution)`, so the probe returns every contribution in the store, across every retained attempt, library and release, that ever held that key.

**Consequence.**
- Per-key RTs for every closure or selected load over a multi-owner view.
- Examined and transferred rows grow with retained history (S02 with many libraries and releases; S08). This is worst for content-addressed keys that recur across attempts, which describes the vocabulary relations, which are multi-owner by construction (`is_epoch_shared`).

Mechanisms: H8/H9/H10/H25; FP-07, DP-10; A4. Ordinary work becomes coupled to retained history.

**Simpler conforming alternative.**
- (a) When demand is a large fraction of the view, merge against the cached `prepare_memberships` run. It is already exact and owner-filtered (`row_tokens_inner` precedent).
- (b) Otherwise, push the owner restriction server-side and batch the probes. Either use one request carrying W equality-prefix statements (the multi-statement `prepared_rows` path already exists), each `… AND contribution IN $owners`; or use a per-owner `contribution_rows` probe with `semantic_key IN $window`.
- The NE-F01 remedy (no Cartesian owner×key point IDs) is preserved by both.

**Preservation constraints.** No owner×key Cartesian, exact collision detection (`selected nominal backing collision`), and an independent check of the physical membership ID.

**Uncertainty.**
- How many catalog-path views are multi-owner. This is not established statically.
- Whether SurrealDB 3.3 uses `contribution_rows` or `member_keys` for `IN` on a trailing column. Settle this with the `neo4j-surrealdb` skill or pinned `third_party/surrealdb` before choosing (b).

**Evidence label.** Implemented/static-inspected; remedy Proposed.

**Closure evidence.** A count of multi-owner views in a representative attempt. A focused native control in which a key that exists in an unrelated retained attempt does not increase examined rows.

### B-03: unrequested behavioural owners run as full stage units and materialise per-artifact NotRequested coverage

**Observed cause.** `compile_driver` runs Local, Base, Completion, SourceCalls, Enriched, Models and Summary under catalog. Examples are `local_semantics::run` and `semantic_execution::evaluate_base`. Each one:
- loads metadata and the expected domain;
- iterates (input, context) frames;
- calls `coverage::admit`;
- emits for **every expected scope**: a `CoverageRequirement`, the required source rows, the observation sources, an `AnalysisCoverage`, and premises;
- and finally emits an invocation plus an `AnalysisOutcome { status: NotRequested }`.

`analysis::expected::contract` gives these methods `Grain::Artifact(PythonSource)` scopes, so the rows are **per Python source artifact × method × frame**. They go through the native write path (op-map M2), contribution completion, product retention, their own publication-boundary freeze (`Local`, `Model`, `Summary`, `Analytic`), and are then re-read by:
- `final_coverage::produce` (×2 targets);
- `Workspace::admit_frontier` (`SELECT * … ORDER BY id`);
- `artifact_manifest::semantic_outcomes`;
- the completed-state hash (B-05);
- publication (inquiry D).

The serving consumer derives availability from **invocation outcomes only** (`lctx_serving::behavior`: "all outcomes NotRequested → `Availability::NotRequested`").

**Consequence.** Under S01 and S02, catalog compile and admission work grows with Python artifacts × 7 methods for answers that are uniformly "not requested" by profile. Mechanisms: H2, H6, H16; FP-07, DP-10/16; A4. Analytic kernel preparation is already skipped (`analytic::produce` builds `FrameScopes` only `if requested`), so earlier lead CS-F04's kernel cause is resolved. The coverage materialisation remains.

Historical receipt, not a fresh run and not attributing cost to a mechanism: a fixture native journey run `20261011T090609.217Z-6c0bdc` (2026-10-11, shared host) logged `lctx_phase` timings with:
- analyze_local 3.5 s, evaluate_base 2.2 s, complete_base 2.1 s, prepare_source_calls 2.2 s, enrich_execution 2.6 s, apply_models 3.6 s, analyze_summaries 4.5 s, analyze_analytic 10.0 s, analytic_embedding 2.1 s;
- against catalog_core 4.8 s.

Run `20261009T213536.960Z-54edcb` shows a similar order. The cache state and revision mix are unknown.

**Simpler conforming alternative.** Make NotRequested an **invocation-level, domain-total outcome**:
- Keep the per-frame invocation and the explicit `NotRequested` outcome (CI-04: explicit, never absent).
- Emit no per-scope coverage rows when the status is NotRequested. The frontier and coverage-frontier contract derives scope status from the invocation for the method's declared contract.
- Physically, emit all unrequested methods' invocations and outcomes from one contribution, instead of seven or more stage units each with registration, freeze, retention and completion (H2). Keep separate units for methods that actually run.

**Preservation constraints.**
- Explicit per-method, per-frame NotRequested.
- Frontier totality: every method has an outcome per frame.
- S04 (behavioural over an existing catalog) must not reuse NotRequested products. It does not today, because `product_request` includes the profile.
- Model-owned semantics. The coverage-frontier invariant changes in `lctx-model`, not in core orchestration.

Counterexample check: a future partial request, where only some scopes are requested, is not NotRequested. It would carry `Partial` with per-scope rows, so the distinction is not erased.

**Uncertainty.** Whether any other consumer (inspection, eval) reads per-scope NotRequested coverage. I checked serving `behavior.rs` and the `NotRequested` uses in `operations`, `operation_packet`, `core` and `ranked_results`, which are constants. I did not check `lctx-eval`.

**Evidence label.** Implemented/static-inspected; receipt attributed; remedy Proposed. **Seam:** inquiry C owns unrequested-analysis demand (H6). This candidate is about the stage and physical unit.

**Closure evidence.** The static route shows unrequested methods emit only invocations and outcomes. The frontier and serving controls still report NotRequested. A behavioural compile is unchanged.

### B-04: the expected coverage domain is rebuilt from immutable inputs by every analysis stage

**Observed cause.** Each analysis owner constructs its own `CapturedSources::capture` and `CoverageAdmission::new` (a fresh `FrontierIndex`), then streams `analysis::expected::inputs(method)`:
- `InputRevision`, `ArtifactUse`, `CoverageScope`, `NormalizationComputation`, `NormalizationCoverage`, `ProviderCoverage`, read with `SELECT *` (`consumed_rows::stream_where_at`);
- `SourceArtifact` property columns (`stream_artifact_admission`).

The construction sites, from an `rg` for `CoverageAdmission::new|expected::inputs(`, are `local_semantics`, `semantic_execution` (×4), `semantic_models`, `semantic_summaries`, `structural`, `analytic`, `analytic_embedding`, `catalog_core`, `catalog_evidence`, `catalog_selection`, `synthesis`, `retrieval`, `retrieval_preparation`. The `coverage_frontier` invariant (`analysis/family/coverage.rs`) builds another. All of these inputs are fixed once normalization completes.

**Consequence.** The same domain is read, decoded and indexed about 15 times per compile. Each read is O(artifacts + scopes × families), so it grows linearly with S02's heavy dependency context. Mechanisms: H12/H19; DP-10. The B-03 remedy removes some consumers but not the repetition for the methods that run.

**Simpler conforming alternative.** Prepare one immutable `FrontierIndex` per (exact expected-domain view identities, profile) after normalized admission and lend it to stages. `CoverageAdmission::domain` already takes `(input, context, method, capability, source_digest)` per call, so the per-method contract stays per call.

**Preservation constraints.**
- `CapturedSources::accepts(permit)` and the source-digest check must bind to the identities the shared index was built from. Mismatched identities rebuild.
- Budget charge ownership moves to the shared owner.
- The publication callback's independent recomputation (`coverage::admit` doc comment: "the publication callback independently repeats this operation against actual physical input rows") is a deliberate independent check. Keep it, at admission only.

**Uncertainty.** Row volumes are not known. The relative cost against the stage kernels is unmeasured.

**Evidence label.** Implemented/static-inspected; remedy Proposed.

**Closure evidence.** One index build per compile, observed by tracing or a counter. Coverage-admission controls still pass.

### B-05: the manifest's completed-state identity re-serialises and hashes the whole native compiler state, row by row

**Observed cause.** `artifact_manifest::populate` calls `NativeCompilerStore::completed_state` → `state_identity_inner` → `capture_state` → `state_identity_selected`. That function walks all six `STATE_TABLES` (contributions, memberships, …, backing `compiler_record` rows) with `rows.next_batch(1)`, runs `serde_json::to_vec` on every row into one `KeySink`, and runs `validate_backing_batch` (Arrow re-projection) over every backing row.

`lctx_publisher::native_publication` recomputes `completed_state_after_closure` at seal and compares the two (inquiry D). Completion has already produced exact per-contribution digests: `complete_contribution_inner` hashes membership `(semantic_key, content)` per output, and logical view identities derive from them (op-map §1.3.7, §2).

**Consequence.** Whole-state, one-row-at-a-time hashing at admission grows with **all** memberships of every stage, including B-03's NotRequested coverage rows (S01, S02). Under S03 it repeats in full for unchanged content. Mechanisms: H14/H19/H20/H26; FP-07; A4.

Historical receipts: `artifact_manifest` took 88.8 s (`20261011T090609.217Z-6c0bdc`) and 81.9–83.7 s (`20261011T083443.010Z-7f65a2`), against 0.4–8.6 s in other runs. Those runs mix revisions, profiles and fixtures, so this is unattributed. The figures only show that the phase can dominate.

**Simpler conforming alternative.**
- Define the completed-state identity compositionally from the completion-time contribution digests plus view and binding descriptors (a Merkle-style composition).
- Carry an attempt revision or era witness from admission to seal instead of re-deriving. `bind_inner` already increments `$__attempt.revision`.
- Keep a full independent recompute only where uncertainty genuinely enters: portable export/import (`import_state`) and the explicit audit (`inspection::audit` uses `completed_state_captured`).

**Preservation constraints.**
- Detection of any mutation between admission and seal, including another writer (needs a guard over attempt-owned rows).
- Import equality.
- The backing-fidelity check. That check is a real independent decoder check (implementation-review F01), and it belongs where backing rows enter (write or completion), not in an identity hash.

Counterexample check: an external process mutating attempt rows after admission. The revision or era witness must cover that, or the recompute is kept at seal.

**Uncertainty.** I have not separated how much of the receipt time is the state hash and how much is the other `populate` steps (`producer_inventory`, `semantic_outcomes`, `projections`, `embeddings`).

**Evidence label.** Implemented/static-inspected; remedy Proposed. **Seams:** inquiry D (seal) and inquiry G (identity and assurance).

**Closure evidence.** The completed-state digest is composed without a full table walk on the ordinary path. Tamper, late-mutation and import-mismatch controls still reject.

### B-06: final graph admission holds an O(graph) in-memory lookup and makes a separate full canonical pass

**Observed cause.** `artifact::admit` calls `native_canonical::Lookup::load`. That loads every entity header (`BTreeMap<EntityId, Header>`) and every assertion ID (`BTreeSet`), charged at 192 bytes each. Admission then streams all entities and all assertions (`admit_entity`/`admit_assertion` plus `FamilyHasher`). Publication then streams both families twice more and reconciles a third time (op-map §1.7, inquiry D).

**Consequence.** Peak live set grows linearly with final graph size during admission (S02; H17). Admission and publication are separate physical passes over identical immutable content (H2/H16).

The family digests are required: they are the manifest's family content identity. The graph-level reference admission checks the canonical lowering independently of the typed-relation reference checks. That independence is worth keeping (safeguard 7).

**Simpler conforming alternative.**
- Replace the in-memory `Lookup` with a sorted spillable key run, reusing the `ordered_rows` mechanism, or with batched native existence checks per window.
- Separately, consider letting publication's reference loading consume the admission traversal, while keeping seal-time `Loader::reconcile` as the independent readback.

**Preservation constraints.** Exact kind and subtype checks, occurrence and evidence span bounds, the derivation proof set, and deterministic family digests.

**Uncertainty.** Graph sizes for FastMCP and for S02 are not known. Whether the BTreeMap is material against the budget is unmeasured.

**Evidence label.** Implemented/static-inspected; remedy Proposed. Lower priority than B-01 to B-05.

### B-07: id-only reads re-fetch node payloads that the verified membership run already holds

**Observed cause.** For an unpredicated read projecting only `id` (`semantic_key AS id`), `scan_bound_rows_inner` still iterates `SelectionKeys::Verified` candidates. Each of them already carries the key, verified at completion. `SelectedRows` nevertheless issues one payload RT per 128 candidates.

Callers include:
- every root enumeration (`SELECT id FROM … ORDER BY id` in `catalog_core::produce`, the `normalize/mod.rs` callable loops, `local_semantics`, `scoped_inventory`);
- every `validate_scope` emptiness probe;
- the target side of `validate_references` joins, where DataFusion's projection pushdown trims the target to `id`, plus the tag when there is one.

**Consequence.** About N/128 RTs and N node reads per enumeration of a relation whose keys are already local. This repeats across stages and reference fields (H16/H19; DP-10).

**Simpler conforming alternative.** Serve id-only, unpredicated projections directly from the verified membership run, in order, with no payload query.

**Preservation constraints.** The payload query also re-confirms that each node exists with the right `semantic_type`. If that is relied on as a backing-presence check, keep it at completion or seal (the independent readback) rather than on every read.

**Uncertainty.** Whether implementation-review F01's backing-fidelity closure relies on this per-read confirmation.

**Evidence label.** Implemented/static-inspected; remedy Proposed. Low-to-medium priority.

## 4. Opportunities and uncertainties that are not findings

- **Emptiness probes and fetch (op-map lead).** `NativeTable::scan` ignores `_limit`. `with_fetch` is delegated to `StreamingTableExec` (and tested by `fetch_delegates_after_native_filtering_without_scan_limit`). But `batches_from_rows` accumulates up to 4,096 rows before emitting, so `SELECT id … LIMIT 1` on a non-empty view can still pull up to 32 payload windows.
  - Unfiltered emptiness is already known: `NativeTable::statistics` reports `Precision::Exact(view.rows)`, and `scan_bound_rows_inner` returns empty without I/O when `view.rows == 0`.
  - Answering non-emptiness from `view.rows > 0` would be symmetric (H19).
  - Waste per invariant is bounded at 32 windows or fewer. Not material enough to be a finding; it mostly disappears under B-07.
- **Per-stage `SessionContext` (op-map lead).** Construction is in memory only: copied config, shared `RuntimeEnv`, Arc providers (`CompletedInputs::prepare_session`). View registration (`registered_view` → `known_views`) and membership preparation are cached per view identity in the store across stages. The lead is present but not an amplification.
- **SQL parse/plan per grain × relation.** `catalog_core::load_record` → `PreparedClosure::select` → SQL string → `stream_query_at`, about 30 times per 32-root grain. `PreparedClosure::ordered_input` already scans selected native providers directly. Extending that to `select` would remove per-grain planning. This is CPU-only, and secondary to B-01.
- **Value→Arrow→typed decode.** Most visitors use DataFusion only as a typed stream container. Arrow is justified where joins run (reference validation, pair SQL, predicate sessions). A direct Value→`R` path for visitor-only reads is possible but unproven to matter.
- **Is DataFusion paying its way (H4)?** Partly. It provides real planning for reference anti-joins, pair-edge queries and checked-hit predicate sessions. Elsewhere it is a stream adapter. The cost lies in B-01/B-07 access routing, not in DataFusion.
- **Whole-model digest in every product key** (`workspace_products::product_request` encodes `model.digest()` per output). Under S03, a changed model declaration invalidates every normalization and upper product, even where the stage's relations, invariants and program are unchanged. This is sound and conservative (DP-09). A finer key needs a complete per-stage dependency declaration (`declaration.code` plus relation and invariant digests). Inquiry C owns reuse.
- **Nested epoch views in the second admission.** `bind_admission_epochs` adds each frozen boundary's view. `validate_references` unions every distinct epoch view of a vocabulary target (`distinct_reference_views`) and validates each distinct source epoch. When vocabulary grows across boundaries, nested supersets are re-joined. Monotonic containment (a later contribution set ⊇ an earlier one) could carry earlier checks forward (H19). The magnitude depends on vocabulary growth under catalog, which is unknown.
- **Fixed per-stage native overhead.** Registration, reuse lookup, retention and roughly 17 binds × 9 boundaries in freezes. Constant in library size, so not material for S02. It grows with the number of kinds and boundaries.

## 5. Strengths, preservation constraints and adequate areas

- **Membership preparation** (`prepare_memberships`) is a shared, coalesced future per view identity. It is spillable (`ordered_rows::Run`, file-backed) and reused across stages, with pool-mismatch eviction. Keep it, and build on it (B-01/B-02/B-07).
- **`checked_premises`** memoises on an exact premise: model, invariant digest, profile, and requested/resolved selectors with exact view identities (`validation_premise`). The second `admit_semantics` correctly re-checks only premises that changed. This is the H19 pattern done right.
- **Declared output ordering** on `StreamingTableExec` (by `id`) avoids sorts. Exact row statistics reach the planner (the earlier CS-F01 IPC-statistics complaint is resolved).
- **Projection pushdown** reaches `body.<field>`. The atomic scope-field driver uses the `by_scope` index. The explicit-key `select_table` route serves closure expansion.
- **Graph preparation** is lazy and released (`preparation::Needs::remaining`, `release_unused`). Analytic kernel scopes are skipped when nothing is requested.
- **The read-only SQL gate** (`sql::read_only`) and typed error preservation (`sql::model_error`).
- **Separate normalization stage units** have real purposes: per-stage reuse keys, and ordered owner authorities (Receivers → Events → Bindings). They are adequate.

## 6. Positions on earlier leads

| Lead | Position | Evidence |
|---|---|---|
| catalog-speed F01 (selected grains lose selectivity) | **residual / moved** | The IPC route is gone; explicit-key native selection exists. Root seeding and presence via SQL `id` predicates still scan whole membership runs (B-01). |
| catalog-speed F02 (prefix rebuild at completion) | **resolved (moved)**, not re-examined in depth | Native membership. Completion re-reads only its own contribution (op-map §1.3.7). |
| catalog-speed F03 (stable validity discarded) | **residual** | Exact-premise memo in `validate_scope`. Nested-epoch re-joins and whole-state hashing (B-05) remain. `verify_export` not examined. |
| catalog-speed F04 (unrequested A1 prepare) | **resolved** for kernel scopes; **residual** for the expected domain and coverage | `analytic::produce` gates `FrameScopes` on `requested`. B-03/B-04. |
| catalog-speed D01–D04 | not examined | Serving, restore and codec (inquiries E, D, A). |
| correction-causes F03 (planner-sensitive lowering in consumers) | **residual** | Consumers author `id` predicates as SQL text, and the physical route depends on provider recognition (B-01). |
| correction-causes F04 (whole match arrays) | **resolved** | Spillable `AsyncCandidateSort`/`Run`, `SelectedRows` windows. |
| correction-causes F05, F06 | not examined | Schema registry and restore (inquiry D). |
| populated-journey F01 | not examined | Codec and schema dispatch (inquiries A, D). |
| populated-journey F02 (repeated canonical preparation) | **residual / moved** | `prepare_canonical` is one shared future. The canonical families are still traversed at admission, then again at publication (B-06; inquiry D). |
| graph/hash F01 (repeated per-root planning and hydration) | **residual** | 32-root batching shares discovery, but per-grain seeding is O(view) (B-01) and SQL is planned per grain × relation (§4). |
| graph/hash F02, VS-F01, C-F01–F05, GK-F01, GK-F02 | not examined | Correctness and lowering semantics, outside this inquiry's focus. |
| GK-F03 (reservations) | not examined | Inquiry H. |
| corrections-integrated F01–F04 | not examined | Registry and serving hydration (inquiries D, E). |
| native-efficiency F06–F08 | not examined | Completion wait (H), serving (E), harness (F2). |
| Op-map: all 21 bindings run under catalog | **confirmed** | `PreparedCompilation::schedule`, `compile_driver`; consequence in B-03. |
| Op-map: a `SessionContext` per `CompletedInputs` | **confirmed present; refuted as amplification** | §4. |
| Op-map: fetch/limit not pushed to SurrealDB | **confirmed; bounded consequence** | §4. |
| Op-map: multi-owner key lookup is one RT per key | **confirmed, plus retained-history coupling** | B-02. |
| Op-map: `admit_semantics` runs twice | **residual** | Memoised premises; changed vocabulary premises and nested epochs re-check. Historical receipts show artifact invariant admission at 16–45 s on 2026-10-11 runs (unattributed). |
| Op-map: artifact admission does a full entity and assertion scan | **confirmed; partly necessary** | Family digests and graph admission are needed; the O(graph) lookup and separate passes are not (B-06). The larger hidden cost is the completed-state hash (B-05). |

## 7. Candidate rule impacts

- **B-03.** The semantic-model coverage and frontier contract (`docs/design/sections/semantic-model.md`, profile-bound NotRequested results; the `lctx-model` `coverage_frontier` invariant) requires coverage per expected scope for every invocation.
  - Proposed change: a NotRequested invocation is domain-total and carries no per-scope coverage rows.
  - If the rule is kept: B-03 narrows to physical fusion. One contribution emits all unrequested methods' rows, and B-04's shared index lowers the read cost. The write volume stays.
- **B-05.** The completed-state identity format (`native-completed-state/v3`, `STATE_FORMAT_VERSION`; the storage-and-publication owner).
  - Proposed change: a compositional digest over completion digests plus an attempt revision witness to seal.
  - If the rule is kept: the full walk stays, but should be batched (not `next_batch(1)`) and run once per attempt, with seal comparing a witness. The backing validation moves to where rows enter.
- No ADR-level rule is needed for B-01, B-02, B-04, B-06 or B-07.

## 8. Proposed light probes

1. **Production route inventory for B-01** (read-only).
   ```
   rg -n '\.grain\(|grain_roots\(|"id IN \(|id=X\x27|WHERE id IN' crates/cpg-core/src --type rust
   ```
   Then mark each hit as `#[cfg(test)]` or production by inspecting the enclosing item. **Discriminating outcome:** any production Physical-root presence check or `grain` seed on a relation that grows with library size confirms the quadratic route. If only `catalog_core` and the normalization callable and overload loops remain, the scope of B-01 narrows to those.
2. **Phase-receipt aggregation** (read-only, over existing logs).
   ```
   rg -o --no-filename 'compilation\{profile="catalog".*phase="[a-z_]+".*status="passed" elapsed_ms=[0-9.]+' build/runs/*/output.log
   ```
   Group by run and phase, then report each run's sum of {analyze_local, evaluate_base, complete_base, prepare_source_calls, enrich_execution, apply_models, analyze_summaries, analyze_analytic, analytic_embedding} against {catalog_core, catalog_evidence, catalog_selection, synthesis, retrieval}, plus artifact_manifest. **Discriminating outcome:** if the unrequested-stage sum and artifact_manifest are consistently comparable to or larger than the catalog product stages, B-03 and B-05 priorities rise. If they are small and noisy, both drop to medium. These are historical receipts only and attribute nothing to a mechanism.

Not a light probe, because it needs a build; it belongs at the coordinator's acceptance timing: a planning-only DataFusion control in `compiler_provider::tests` asserting that `WHERE id IN (X'…')` reaches `NativeTable::scan` as an Exact filter and becomes `NativePredicate::Sql`, not explicit keys. That would establish B-01 at Tested strength.
