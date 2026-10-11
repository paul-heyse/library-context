# Inquiry C: Analyses, reuse/invalidation and embedding realization

**Provenance.** This is the verbatim final report of supporting inquiry C, from one read-only
`design-reviewer` agent, for the [codebase efficiency-alignment review](../../reviews/design_review_efficiency-alignment-codebase_2026-10-11.md).
It was produced on 2026-10-11 at baseline `88f33bc0`, against the [inquiry briefs](inquiry-briefs.md).
Only this header was added.

How to read it:
- Candidate labels such as `C-01` are local to this report. They are not the review's finding
  IDs.
- Its judgments are bounded and supporting. The principal review owns the combined decision.
- See the [evidence index](README.md) for coordinator checks and reconciliation.

---

# Inquiry C: analyses, reuse/invalidation and embedding realization

## 1. What I examined, and what I did not

**Baseline.** `main` at `88f33bc0`, 2026-10-11. The work was read-only: rg, sed and file reads. I ran no builds, tests, services, `just` recipes or SurrealDB queries. The only thing I executed was an `rg` count over the installed pilot environment `build/envs/fastmcp` (§2).

**Standard.** I loaded core 3.3 (FP-07, DP-08 to DP-10, §D), the heuristics 1.0 (H1 to H28), the design-review and design-review-code-intelligence skills, and §14.10/§14.11 of `docs/design/sections/api-and-evidence-product.md`.

**Read in full or in the decisive parts:**
- `crates/cpg-core/src`:
  - `compilation.rs` (`UpperStage::{reuse, graphs, bindings}` and the `compile_driver` loops);
  - `compilation/{reuse.rs, preparation.rs}`;
  - `workspace_products.rs` (`product_request`, `reuse_product_inner`, `decode_product_rows`, `retain_product`);
  - `workspace.rs::{output, execution_implementation}`;
  - `normalize/{pipeline.rs, kernel_products.rs, reuse.rs}`;
  - the per-root loops in `normalize/mod.rs` (entities, callables, `receivers_produced`);
  - `consumed_rows.rs::PreparedRootBatch::content_domain`;
  - `analysis_graphs.rs::PreparedGraphs::load`, `analysis_prepare.rs::configuration`;
  - `analytic.rs::produce`, `local_semantics.rs::run`;
  - `retrieval.rs` (`realize` and the root loop), `analytic_embedding.rs`;
  - `embedding_realization.rs::{prepare, prepare_chunk, realize}`.
- `crates/lctx-surrealdb/src`:
  - `compiler.rs::{attach_retained_product, retain_attached_members, hold_contribution_inputs, retain_product_identity, validate_inputs, complete_contribution_inner}`;
  - `control.rs::{ensure_rows_owned, hold_many, guarded_effect}`;
  - `product_cache.rs` (lookup, insert, stage, reserve, reap, decode, acknowledge);
  - the `cache.rs` query shapes;
  - the deletion rules in `control/{retirement,cleanup}.rs`.
- `crates/lctx-model/src/domain`:
  - `completed.rs::{ContributionSpec, CompletedView, CompletedContribution}`;
  - `analysis/sources.rs::SourceSnapshot`;
  - `compilation_product.rs::{ProductSection, PortableProduct}`;
  - `stages.rs::Stage`;
  - `structural/build.rs::produce`, `analysis/delegation.rs::Inputs::traverse`;
  - `projection/snapshot.rs::reachable`;
  - `analytics/{neighbours, vectors, communities, concepts}.rs` and parts of `analytics/build.rs`;
  - `conditions/kernel.rs`, the `execution/summary_schedule.rs` SCC call;
  - `retrieval/consumption.rs::verify`.
- `crates/lctx-embed/src/lib.rs` (client, `embed`).
- The `reuse` block that `scripts/surrealdb_service.py` writes into the installed runtime config.

**Not examined:**
- The internals of the behavioral production kernels (`semantic_execution`, `semantic_models`, the summary worklist and transfer), beyond lifetimes and reuse eligibility.
- The synthesis kernels.
- Serving-side preparation (lead GR F01 and reuse-contract F02 belong to inquiry E).
- Publication and seal hold transfer (inquiry D), and re-admission (inquiries B and G). These seams are flagged where they apply.
- Row volumes per relation, which I cannot get without a run.

## 2. Complete operations examined

**O1. Cold compile with the product cache attached (S01, S02).**
- **Is the cache attached by default?** Yes. The installed runtime configuration always carries `reuse` (`scripts/surrealdb_service.py`, the per-database cfg). `crates/lctx/src/compile.rs::compile` therefore always calls `NativeProductCache::connect` and `Workspace::set_product_cache`. `admitted_product_reuse` defaults to true (`workspace.rs`).
- **Per normalization stage:**
  1. `product_request` builds a KeySink over the operation, the model digest, `implementation`, the policy revision, the result contract, the configuration, the profile and the input `SourceSnapshot` identities.
  2. `attach_retained_product` looks it up, one RT.
  3. On a miss, the stage computes.
- **In the Entities, Callables and Aspects kernels, per selected root, sequentially:**
  - `content_domain`, which fetches content tokens once per union batch (`OnceCell`);
  - `NativeProductCache::lookup`: two flocks plus the entry flock, then `current()` and `entry()`, one RT each;
  - on a miss, after the kernel runs, `retain_*` → `R::encode` → `codec::batch_bodies` → `serde_json` → `insert`. `insert` is a spawned task that is awaited inline (detailed in C-05).
- **After each stage:** `retain_product_identity` (one RT, `ensure_rows` at 4 RTs, `hold` at 4 RTs).
- **Upper stages:** `PreparedGraphs::load` runs once, at the first graph-needing stage that misses. Graphs and bindings are released by `preparation::Needs::remaining` after every stage, whether it hit or ran fresh.

**O2. Warm recompile with unchanged inputs and the same binary (S03a).**
1. Facts providers re-run in full. `compile_driver` → `facts::compile_facts` has no reuse path.
2. Facts contributions complete with the same `CompletedContribution` identity, so their views are equal.
3. Each normalization and upper stage whose eligibility is `Rows` then hits through `attach_retained_product`:
   - `validate_inputs`;
   - `hold`, 4 RTs;
   - `guarded_effect`, 4 RTs;
   - `hold_contribution_inputs`, 4 RTs;
   - stream every source membership;
   - per 128 members, `retain_attached_members`: `ensure_rows_owned` (4 RTs, with a guard `FOR UPDATE`, a guard `UPSERT`, a SELECT and a CREATE per row) plus `hold_many` (4 RTs, two `native_hold` rows per member);
   - the `completed` effect, 4 RTs.
4. Checked stages (Receivers, Events, Bindings) also run `scan_contribution_batches` (a full read into `MemTable`s) and the owner predicates in `normalize/reuse.rs`.
5. These upper stages always run fresh:
   - under any profile: Configuration, Native, EmbeddingConfiguration, Embedding and the two frontier stages;
   - under Behavioral: Local, Base, Completion and SourceCalls.
6. Admission and publication repeat in full (operation map §2, verified only at its entry points).

**O3. Retrieval with an embedder (catalog frontier).**
- The stage seeds the session from the analytic `FullValue` and `ProjectedValue` rows (`retrieval.rs::produce`).
- Per frame, for 32-root render windows, and **per rendered root**, `realize` → `Session::prepare(windows of that root)`. That is one `cache.cached` call (`SELECT * FROM $ids`), one HTTP `POST /v1/embeddings` for misses, then `cache.admit`.
- The analytic embedding stage instead batches 64 windows per `prepare`.

**O4. Structural (on the default catalog path).**
- `structural::build::produce` builds the subsystem and public sets with nested linear scans.
- It then calls `delegation::Inputs::traverse` once per selected seed. Each call rebuilds the indexes described in C-09 before running a bounded BFS.

**Static size context, not a root count.** `rg --no-ignore` over `build/envs/fastmcp/.../site-packages/fastmcp` finds 2,394 `def` lines, 434 `class` lines and 257 files. The whole environment has 49,976 `def` lines in 7,888 files. These numbers bound the per-root and per-seed multipliers only loosely.

## 3. Candidate findings

### C-01 — Facts providers have no reuse unit, so every warm variant re-extracts

**Observed cause.**
- `compile_driver` runs `facts::compile_facts` unconditionally.
- `UpperStage::reuse` returns `Fresh("external effects retain their existing owner")` for anything that is not `Effect::Pure`, and the providers are declared `Effect::Extraction` or `Effect::Acquisition` (`cpg-extract/src/{pyrefly_stage,ty_flow,document_parser,deployment}.rs`, `acquisition.rs`).
- Yet the substitution premise already exists. `ContributionSpec` carries `captured_binding` (the captured-input contract), `implementation`, `configuration` and `profile`. `attach_retained_product` already refuses unless the specs are equal (`descriptor.spec != spec`).
- So the earliest reuse key in the system is a completed facts view identity, and that only exists after full extraction, row emission and the native writes (M2).

**Consequences.**
- **S03 unchanged rerun:** proving "unchanged" costs Pyrefly, ty, the document and deployment parses plus all facts writes and completion re-reads. That is the dominant work the reuse layer was meant to avoid (H14, H13, H12).
- **S03 changed analysis, embedding or rendering policy:** facts do not depend on these, yet the whole extraction repeats. §14.10's row "Association policy with unchanged facts → Catalog/evidence…" presumes facts are reused.
- **S07 recovery and S05 republish of the same release:** a crash after admission still forces re-extraction.

**Simpler conforming alternative.** Give each provider a `ProductRequest` keyed on its existing spec: captured binding, provider implementation, configuration and profile. Admit attach for `Extraction` and `Acquisition` stages whose inputs are only the frozen capture. This reuses the existing `attach_retained_product` and admission gate (`contribution.attempt.admitted=true`). The necessary proof of "unchanged" is then the capture digest, which is one closure read in inquiry A's area, instead of a full analysis.

**Preservation constraints.**
- CI-10 hermeticity: no ambient analyzer configuration. Analyzer settings must come only from the captured inputs and the pinned implementation; anything ambient must enter the spec.
- Only admitted attempts are reusable.
- A provider that is nondeterministic in the order or identity of its rows still yields a valid admitted output for the same input. Whether every provider is set-deterministic is an assumption to check.
- Coverage receipts must keep naming the providers.

**Uncertainty.**
- The relative cost of extraction against admission and publication is unmeasured.
- The P4 receipt (about 280 s per extraction test) is a test harness, not a compile, and attributes nothing.
- Whether `captured_binding` covers every provider input (for example the native context config) needs a check in `cpg-extract::contracts::composition`.

**Evidence:** Implemented (static) for the absence of the path. Search coverage: `rg "reuse|attach_retained|retain_product"` over `cpg-core/src/facts.rs` and `cpg-extract/src/{bundle,pyrefly_stage,ty_flow}.rs` (with operation map §6), plus my reading of `compile_driver`.

**Closure evidence:** a same-binary recompile whose providers attach without running the analyzers; a changed captured byte forces a miss; an unadmitted prior attempt is never attached.

**Seam:** inquiry A (capture cost) and inquiry D (publication repeats).

### C-02 — Reuse keys are view identities that chain provenance and couple sibling outputs, so there is no value cutoff

**Observed cause.**
- **Identity chain.** `ProducerOutput::product_request` uses `source.snapshot.identity()` as an `ExactView` token. `SourceSnapshot::of_completed_view` embeds `view.identity`. `CompletedView::new` hashes the relation name plus the set of logical contribution identities. `CompletedContribution::identity` hashes `spec.identity()`, which includes the input `SourceSnapshot` identities, plus the outcome plus **every** output relation's `OutputContent` (`lctx_surrealdb::compiler::complete_contribution_inner`; `lctx_model::domain::completed`).
- **Effect 1: no cutoff after a recompute.** A view's identity changes whenever any upstream spec changes, even when this stage's rows are byte-identical. No early cutoff exists after a recompute.
- **Effect 2: sibling coupling.** A view changes when a *sibling* relation of the same contribution changes. `analysis_prepare::configuration` writes ModelCatalog, AuthoredModel, MethodParameters, AnalysisDefinition, ProjectionDefinition, AnalyticsConfiguration and RetrievalDefinition in **one** contribution. Toggling one optional A1 technique therefore changes the AnalysisDefinition, RetrievalDefinition and ProjectionDefinition views for every consumer.
- `compilation/reuse.rs::DependencyGraph` documents a value cutoff ("pure-value dependency tokens may stop value propagation"). No producer emits such tokens, and the graph has no decision consumer; it is used only for `trace` and a debug count.

**Consequences.**
- **S03 changed policy:** an analytics, rendering or retrieval-definition change misses every stage that reads any configuration relation, including catalog, retrieval and the behavioral stages. This contradicts §14.10's finer intent ("Unit rendering or embedding spec → Retrieval/vector artifacts"; "Ranking/display policy … unchanged source facts and units need no reinterpretation") and its premise that catalog construction is independent of optional analysis.
- **S03 patch:** any upstream change propagates through every downstream key even where intermediate outputs are unchanged.
- Mechanisms: H13 ("stop downstream propagation when a recomputed value is substitutable"), DP-09 (over-invalidation stated qualitatively), H14.

**Simpler conforming alternative, in two steps.**
1. Derive per-relation dependency tokens from that relation's own `OutputContent` (rows plus content digest, already computed at completion) together with the stage's declared semantic parameters, not from the whole contribution. This removes the sibling coupling, which is the cheapest part.
2. For consumers whose outputs do not embed input provenance, key on content (the existing `DependencyKind` already distinguishes kinds). On a hit, attach the old membership under a **new spec** that records the current inputs, so provenance refreshes while the value is reused.

Stages that embed `SourceSnapshot` receipts, such as `analytic.rs::produce` and `local_semantics::run` via `CapturedSources::capture(access.snapshots())`, stay exact-view dependent unless receipts cite content identity. That is a model decision (see RC-01).

**Preservation constraints.**
- The content digest covers the full membership, including additions and removals, so negative domains stay exact.
- §14.10 requires that evidence-only or moved-span changes still miss. They do, because spans are content.
- Admission still runs on the new attempt's views.

**Counterexample check.** A stage that reads view metadata (row counts or producer identity) rather than rows would wrongly hit under pure content keys. Such stages must declare an `ExactView` dependency.

**Uncertainty.** I did not trace exactly which upper stages read which configuration relation. The coupling itself is certain from the identity definition.

**Evidence:** Implemented (static).

**Closure evidence:** a changed `AnalyticsConfiguration` with the analytics techniques left off hits CatalogCore, Retrieval and Models; a recomputed stage with identical rows lets its consumers hit; a changed span still misses.

### C-03 — Profile is in every spec and request, so S04 reuses nothing

**Observed cause.**
- `ContributionSpec::identity` encodes `profile`, and `product_request` sets `profile`.
- The per-root requests in `kernel_products::{entity,callable,aspect}_request` inherit both through `output.product_request()`.
- Some declarations are profile-independent: `entity_normalization::stage()` takes no profile, and the `entities` body (`normalize/mod.rs::entities`) does not read it.

**Consequences.**
- **S04 (behavioral over an existing catalog compile):** every facts view differs, so every whole-stage key misses. The per-root complete-domain tokens are content-based (`content_domain`) and would otherwise match across profiles, but they miss on the profile field.
- So a behavioral compile repeats the entire catalog-profile work: extraction, normalization and the catalog stages (DP-09 over-keying, H13).

**Alternative.** Key on profile only where the declaration or code is profile-dependent, using the same `stage(profile)` versus `stage()` distinction the model already makes. Profile identity remains explicit on the attempt, the manifest and the publication.

**Preservation.**
- Stages with profile-specific behavior keep profile in the key, for example Local's `profile == Catalog` → `NotRequested` path.
- A catalog-produced contribution attached into a behavioral attempt must not assert a behavioral outcome it did not compute.

**Uncertainty.** Whether facts providers emit identical rows under both profiles. If behavioral adds relations to a shared contribution, sibling coupling (C-02) still blocks reuse.

**Evidence:** Implemented (static). **Closure:** a behavioral compile following a catalog compile hits the profile-independent normalization kernels.

### C-04 — A whole-stage hit is O(rows) of native writes

**Observed cause.**
- `attach_retained_product` copies every source `compiler_membership` row to a new attempt-scoped physical contribution (`retain_attached_members` → `control::ensure_rows_owned`).
- It also creates `(contribution → membership id)` and `(contribution → node)` holds per member (`control::hold_many`).
- That is about 8 RTs per 128 members plus per-row guard `UPSERT`s. The fresh membership write is about 8 RTs per 128 members (operation map M2).
- So a hit avoids computation, input reads and node existence checks, but not the membership and hold writes.
- Two retention facts:
  - Retirement already protects the source contribution, because `hold(attempt → source)` is created and retirement deletes an object only when it has no holds (`control/retirement.rs`, finalize).
  - The copy exists for attempt-scoped enumeration: `state_owners` and the `attempt_contributions` index (`compiler.rs` state sources).
- On checked hits, the attach (and its writes) happens **before** the receiver, event or binding predicates run (`reuse_product_inner`: attach, then `scan_contribution_batches`, then `check`). A rejection fails the attempt instead of falling back to fresh work.

**Consequences.**
- **S03a:** hit cost grows with product rows, not with change (H14, "proving unchanged repeats the work avoided").
- The two hold rows per member also enlarge the seal's per-attempt hold transfer (inquiry D), and they grow retained control state (S08).

**Alternative.**
- Attach by reference: register the logical-to-source physical mapping (`remember_contributor` already maps logical to physical) and let scans read the retained contribution's memberships under the existing attempt hold.
- Attempt-scoped enumerations would then follow `attempt → contribution` holds, not only `attempt_contributions`.
- Run checked predicates on `scan_contribution_batches(source_id)` **before** attaching.

**Preservation.**
- Pinned readers and export closure must include referenced contributions.
- Retirement must still be hold-governed.
- View identity stays unchanged.

**Uncertainty (medium).** I did not read whether the publication loader, `reconcile` and export require attempt-owned membership rows.

**Evidence:** Implemented (static); RT counts derived from the code.

**Closure:** a hit issues O(1) membership effects; retirement of the old attempt leaves the attached view readable; a checked rejection falls through to fresh production.

### C-05 — The per-root portable cache crosses the native and file boundaries per root, sequentially, on every cold compile

**Observed cause.**
- In `normalize/mod.rs` (the entity `produce`, the callable window loop and the aspect loop), each root awaits `kernel_products::*_hit` → `NativeProductCache::lookup` one after another.
- Each miss later awaits `retain_*` → `insert`, a spawned task that is awaited. `insert_owned` → `stage_product` → `reserve_stage` → `write_stage` → `reconcile_publication` costs about:
  - roughly 8 flock acquisitions, which poll with `sleep(10ms)` when contended;
  - `current()` three times;
  - `entry()` three times;
  - a `reap_stages` page query;
  - an aggregate SUM (C-06);
  - the quota and stage transaction;
  - one write per chunk;
  - the entry transaction;
  - a full exact readback: chunk read, JSON parse and canonical comparison;
  - `acknowledge`: tempfile, file fsync and directory fsync;
  - `remove_stage`.
- That is about 13 RTs, 2 fsyncs and a full re-decode per root miss.
- Keys include the whole compiler source-closure digest (`workspace.rs::execution_implementation` with `LCTX_COMPILER_SOURCE_DIGEST`) and the whole model digest. Every code edit therefore turns every per-root entry into a miss and a reinsert.

**Consequences.**
- **S01 and S02 cold compiles** pay about 13 serialized RTs and 2 fsyncs per root miss on every compile, because the installed configuration always attaches the cache. The pilot has thousands of callables (2,394 `def`s in `fastmcp` alone) times three callable kernels, plus the entity and aspect kernels; S02 multiplies that by 5 to 20.
- **S10 and S03 with code changes:** pure cost, with no hits.
- Mechanisms: H8, DP-10 ("crossed per element inside an inner loop"), H14, H21.

**Alternative.**
- Batch per root window: one lifecycle lease per window, one `SELECT * FROM $ids` lookup for all keys, and one staged multi-product insert with one receipt.
- Or defer insertion to one bounded owned writer after emission, off the critical path (H6 safeguard: insertion is optional acceleration).
- Or attach only on demand, for example when a prior generation exists.

**Preservation.**
- The reuse-contract F01 fences and acknowledgement semantics.
- Complete-domain keys.
- Resource-refusal fall-through.
- No visibility before exact readback.

**Uncertainty.** Actual root counts and RT latency are unmeasured; the RT counts come from the code.

**Evidence:** Implemented (static).

**Closure:** the per-window lookup and insert counts are independent of the root count; the late-mutation and acknowledgement controls still pass.

### C-06 — Capacity accounting scans every retained entry on each insert

**Observed cause.**
- `NativeProductCache::reserve_stage` runs, under the exclusive `capacity.lock`, `SELECT VALUE math::sum(bytes) FROM product_entry WHERE partition=$partition GROUP ALL` plus the same over `product_stage`.
- Over capacity, it pages `ORDER BY created,key LIMIT 64 START $offset`. Only `partition` is indexed.
- The transactional `product_quota:current.used` counter already carries the same total, entries plus stages. It is enforced in the same function's fenced transaction.
- Entries from superseded implementation or model keys are never retired by generation; they are removed only by capacity eviction (8 GiB, `PRODUCT_CAPACITY_BYTES`).

**Consequences.**
- **S08 and S02:** each insert costs O(retained entries), so a cold fill is quadratic in the number of entries.
- **S10:** dead entries accumulate on every code change, which lengthens the scan and triggers repeated eviction paging.
- Mechanisms: H25, H14, H27. Two accounting authorities exist for one total (a DP-01 smell).

**Alternative.** Read `product_quota:current.used`, which is O(1) and fenced, and index `created` for eviction order.

**Preservation.** Quota fencing and the exact-readback receipts.

**Uncertainty.** None on structure; magnitude unmeasured.

**Evidence:** Implemented (static).

**Closure:** insert cost is independent of the entry count; a quota-drift control passes.

### C-07 — Portable product sections serialize their bytes as JSON integer arrays

**Observed cause.**
- `compilation_product::ProductSection { bytes: Vec<u8> }` has no bytes adapter, so `serde_json` (`product_cache::canonical`, `encoded_size`) writes `[123,34,…]`.
- Each section's `bytes` field holds a JSON array of surreal `Value`s (`kernel_products::section`).
- The hit path then:
  1. parses the outer product;
  2. re-serializes it to compare canonical bytes (`decode`);
  3. parses the section JSON;
  4. runs Value → Arrow (`codec::decode_bodies`);
  5. runs `hash_rows`, whose digest is discarded; only its ordering and uniqueness check is used;
  6. runs Arrow → Value (`canonical_bodies`) for comparison;
  7. runs Arrow → typed rows.

**Consequences.** By serde semantics, stored and transferred chunk bytes, quota consumption, decode CPU and readback cost are roughly 3.5 to 4 times the inner JSON. This amplifies C-05 and C-06 (H16, H15, H26).

**Alternative.** Encode `bytes` as one opaque byte string: `serde_bytes`, a nested `RawValue`, or storing sections as `Bytes` chunks. Keep the one typed canonical check that the F04 closure requires.

**Preservation.** The F04 rejection of extra fields, cross-window duplicates and reordering.

**Uncertainty.** The exact ratio depends on the byte distribution (see LP-C1).

**Evidence:** Implemented (static).

**Closure:** stored size is about equal to the canonical body bytes; the F04 controls pass.

### C-08 — Retrieval embedding effects are batched per rendered root

**Observed cause.**
- `retrieval.rs::produce` renders 32-root windows, then calls `realize` → `Session::prepare` once per partition (one root).
- Each call issues one `NativeEmbeddingCache::cached` RT and, for misses, one `Embedder::embed` POST plus `admit`.
- `embedding_realization::prepare` can fill batches of 64 under budget, and `analytic_embedding` does use 64-window batches.

**Consequences.** With embeddings selected (the vector retrieval route, S01 and S02), requests are numerous, small and sequential, and they starve vLLM's batching. Native RTs scale with the number of roots rather than with windows/64 (H8, DP-10, H21).

**Alternative.** For each render window, collect every ready window text across its partitions, call `prepare` once, then run `realize` per root; values are already in the session `values` index.

**Preservation.**
- Per-root `verify_uses` and the publication order.
- Token and service refusals per text.
- The budget-bounded chunking.

**Evidence:** Implemented (static).

**Closure:** service batch counts follow windows/64 in a fake-embedder control, which matches the existing `effect_batches_follow_available_budget…` pattern.

### C-09 — The structural kernel rebuilds the whole-universe index per seed and uses nested linear scans on the default catalog path

**Observed cause.** `structural::build::produce` calls `delegation::Inputs::traverse` for each selected seed. Each call:
- builds `call_nodes` and `definition_nodes` BTreeMaps over **all** nodes of both graphs, although `MaterializedGraph` already keeps an entity → node `index` (`projection/snapshot.rs`);
- rebuilds the `selected` subsystem set and the `unresolved` owner index by scanning all `event_assessments`;
- reserves a budget sized on the whole universe: vertices×1024 + arcs×(10·Step + 512) + (arcs + events)×min(V, depth)×2·Step;
- and only then runs a bounded BFS.

In `produce` itself:
- for each subsystem callable, `data.projection.modules.iter().filter(m.source==source)` and `data.uses.iter().filter(…)`;
- for each public callable row, `data.core_links.iter().filter(link.member==…)`;
- for each evidence row, `output.usage_sites.iter().find(…)`.

**Consequences.**
- Structural runs under the default catalog profile (operation map §1.5). Its work grows as O(seeds × V log V) plus O(n·m) joins, so S02 (5 to 20 times the code) grows it quadratically (25 to 400 times) while every per-seed answer is bounded.
- The per-seed reservation, sized on the universe, can refuse legitimate bounded searches on large graphs (H11: "refusal ≠ feasibility").
- Mechanisms: H9, H10, H12, DP-10.

**Alternative.**
- Prepare the delegation inputs once per frame: node maps (or the graph's own index), the unresolved index and the subsystem set.
- Size per-seed charges by `Bounds` (vertices, arcs, depth).
- Replace the nested filters with keyed maps built once.
- petgraph's index already gives the node lookup.

**Preservation.**
- Bounds semantics and explicit `Stop` and partial outcomes.
- Deterministic order.
- Boundary classification.

**Uncertainty.** Seed and V counts are unmeasured. The model kernels contain about 450 `.iter().find/filter/any/position` sites; I verified only these.

**Evidence:** Implemented (static).

**Closure:** per-seed work is independent of the universe size, beyond the bounded search; a known-answer delegation control returns identical traversals.

### C-10 — Optional A1 neighbours are exact all-pairs under a global cap

**Observed cause.**
- `analytics::neighbours::Prepared::nearest` iterates queries × targets × windows × windows.
- `vectors.rs` caps it at `build::MAX_WORK` (100M window pairs) and turns the whole relation into `Partial/WorkLimit`.

**Consequence.** At S02 the work grows quadratically, so neighbours predictably become Partial for the whole analysis (H9, H11). This is honest under CI-04 but not feasible.

**Alternative.** Prefilter per subject block, or use the already-selected HNSW-backed vector index for candidate nomination with exact rescoring. The heuristic contract (CI-09) permits this.

**Evidence:** Implemented (static). Optional, off by default.

### C-11 — Optional co-use communities expand each owner's targets into a clique

**Observed cause.** `analytics::build` (co-use layer) emits n² pairs per owner scope, and a single global `required > MAX_WORK` sets the whole communities analysis to Partial.

**Consequence.** One high-degree owner (H10 skew) disables communities for every frame.

**Alternative.**
- Weight co-use through the bipartite owner–target graph, or use degree-normalized clique weights with per-owner caps reported locally.
- Leiden itself (leiden-rs) is appropriate.

**Evidence:** Implemented (static). Optional.

## 4. Opportunities and uncertainties that are not findings

- **Two product-reuse layers (the H14 question).** Their scopes are distinct and their benefits real:
  - The whole-stage `native_product` layer is exact on provenance and cheap to key. It holds no payload bytes and hits essentially only when the upstream closure is unchanged.
  - The per-root portable layer is the **only value-based cutoff** in the system: complete-domain content tokens with Present, Absent and Virtual outcomes. It is the only mechanism that lets a patch upgrade (S03b) follow the change, and only in 3 of about 30 stages.
  - They overlap in invalidation: both carry the whole compiler digest, the model digest and the profile.
  - The per-root layer duplicates canonical content that already lives in the same database, in a second byte store with its own quota, acknowledgement, fence and file-lease lifecycle (`product_cache.rs`, 1,837 lines). Pointer-based per-root products (domain token → retained memberships, protected by holds) would unify the two layers on one mechanism. The design route is plausible but unsettled.
  - Not a finding until C-05 to C-07 are resolved, because those determine what such products would cost.
- **Patch-upgrade granularity.** Extending complete-domain products beyond Entities, Callables and Aspects is the route that §14.10 points to. Its value is bounded by extraction, admission and publication costs (C-01; inquiries A and D), which are unmeasured. The prerequisites are C-05 and C-06.
- **Behavioral private-owner kernels are always Fresh.** Local, Base, Completion and SourceCalls are justified as "validation = recomputation" (`UpperStage::reuse`; `local_semantics::run`). For fixpoint-style transfer kernels, checking a candidate fixpoint can be cheaper than iterating to it, and per-root domain products could apply. Any such route must keep the reuse-contract F03 binding of public rows to the reconstructed owner. This is Proposed and bears on S03 behavioral and S04 cost.
- **`DependencyGraph`** (`compilation/reuse.rs`) is built, charged and bound per stage, but feeds only `trace` and a debug count (H26, H27). It is a small cost, but it suggests an invalidation authority that does not exist. Either make it the cutoff owner under C-02 or remove it.
- **The Embedding stage is always Fresh** ("own inference cache"). Under S03a it re-reads every window, makes about windows/64 cache RTs and rewrites its rows. A whole-stage attach of the admitted winners would be equivalent. Minor.
- **The model and compiler digest in every key** (S03 "changed model declaration", S10) make every change a full rebuild. This fits the ADR-0131 design-phase cutover, so it is not a finding. It does make C-05 and C-06 pure cost during development.
- **Seams.**
  - Re-admission of attached views repeats invariants that the prior admitted attempt established (inquiries B and G; lead catalog-speed F03).
  - The seal's per-attempt hold transfer grows with C-04's holds (inquiry D).
  - `Leiden::partition` reserves 16 KiB per vertex (inquiry H).

## 5. Strengths and adequate areas

- **Whole-stage reuse never replays payloads.** It requires spec equality and `contribution.attempt.admitted=true`, so an unadmitted attempt is never reused (`attach_retained_product`). Optional capture refusal does not fail a compile (`Workspace::retain_product`).
- **Complete-domain tokens** (`PreparedRootBatch::content_domain`) are sound under additions, deletions and absent roots. Content tokens are fetched once per union batch, not per root. Keep this as a preservation constraint.
- **Graph and binding lifetimes** follow the remaining selected consumers, including hit branches (`compilation/preparation.rs::Needs`). Graphs hydrate once, only on a miss.
- **Unrequested analyses skip their preparation.** The catalog profile skips Local scopes; analytics skips `FrameScopes` when nothing is requested (`analytic.rs::produce`). Explicit `NotRequested` outcomes are kept.
- **Embedding realization:**
  - it deduplicates before effects and uses budget-bounded batches of 64 or fewer;
  - it uses an exact local tokenizer;
  - retrieval adopts analytic winners (`seed_full_value`, `seed_projection`);
  - it spills to private storage, and the live vector set is bounded;
  - the HTTP client is reused (`lctx-embed`).
- **Retrieval builds once in production.** `ConsumptionData::verify`'s replay is reserved for controls.
- **Kernels:**
  - Kosaraju SCC (linear) with a checked condensation;
  - biodivine `binary_op_with_limit` with node caps;
  - FCA with a `FixedBitSet` and an examined cap;
  - ranking and neighbours with explicit `Partial/WorkLimit` outcomes (CI-04 honest);
  - per-seed BFS bounded by depth, vertices and arcs.
- **Product-cache lifecycle fences** (reuse-contract F01) are present: generation fences, point-stage checks and acknowledgement receipts.

## 6. Positions on earlier leads

| Lead | Position | Evidence |
|---|---|---|
| reuse-contract F01 (unknown native completion) | resolved in source; not re-tested | `product_cache.rs`: `fenced_sql` on reserve, chunk, entry and cleanup; `acknowledge` after exact readback |
| reuse-contract F02 (uncharged wait owners) | not examined | serving (inquiry E) |
| reuse-contract F03 (private and public binding) | resolved by removal; the cost has moved | `UpperStage::reuse`: Local, Base, Completion and SourceCalls are Fresh under Behavioral; recomputation is now the price (§4) |
| reuse-contract F04 (canonical decoding) | resolved | `workspace_products::decode_product_rows`: canonical comparison plus the `hash_rows` order check; C-07 notes its cost |
| GR plan-review F01 (viewer retirement) | not examined | inquiry E |
| HS-F07 (graph and binding lifetimes) | resolved | `preparation::Needs::remaining` and `release_unused` after every stage, hit or fresh |
| graph-native compiler-stage F01 (stage-local winners, scalar effects) | residual | winners are shared and batching exists; retrieval still batches per root (C-08) |
| graph-native compiler-stage F02 (retrieval replays its producer) | resolved | the production path uses `verify_uses` and `verify_frames`; `verify` is only for controls |
| catalog-speed F04 (unrequested A1 preparation) | resolved, with a small residual | `analytic.rs::produce` skips `FrameScopes`; metadata and expected-domain inputs are still read, as total outcomes require |
| op-map §2 "checked hits rerun predicates over `MemTable`s" | confirmed; ordering defect added | C-04 |

## 7. Candidate rule impacts

- **RC-01. View and spec identity composition.**
  - *Rule:* `lctx-model::domain::completed::{CompletedView, CompletedContribution, ContributionSpec}` and `analysis::sources::SourceSnapshot`, the accepted "exact-view" reuse dependency in the GR plan, and ADR-0145 ("checked attachment reuses compatible admitted content", as cited by the operation map).
  - *Proposed change:* per-relation content tokens for value-only consumers; attach with a refreshed spec; receipts possibly citing content identity.
  - *If the rule is kept:* C-02 shrinks to splitting the Configuration contribution into one contribution per relation, which removes only the sibling coupling.
- **RC-02. The reuse eligibility rule "external effects retain their existing owner"** (`compilation.rs::UpperStage::reuse`, and the facts path without reuse).
  - *Proposed change:* captured-input Extraction and Acquisition become reusable under CI-10.
  - *If the rule is kept:* C-01 stays open, and S03 policy changes cost a full extraction.
- **RC-03. "Catalog and behavioral profiles have explicit identity"** (§14.10), as currently realized per contribution.
  - *Proposed change:* keep profile identity at the attempt, manifest and publication level and in profile-dependent stages only.
  - *If the rule is kept:* C-03 is unremediable.
- **RC-04. The installation default that always configures `reuse`** (`scripts/surrealdb_service.py`).
  - *If C-05 to C-07 are not fixed:* make the per-root cache opt-in. Otherwise no change is needed.

## 8. Proposed light probes

- **LP-C1 (C-07 magnitude).**
  - Command: `python3 -I -c 'import json,sys;b=open(sys.argv[1],"rb").read();print(len(b),len(json.dumps(list(b),separators=(",",":"))))' /home/paul/library-context/docs/design_review/evidence/2026-10-11_efficiency-alignment-codebase/probes/p3_run_summaries.txt`.
  - Use any representative JSON-like text instead if available.
  - *Discriminating outcome:* a ratio of about 3.5 or more confirms that the expansion is material; a ratio below 1.5 would downgrade C-07.
- **LP-C2 (C-09 multiplier).**
  - Command: `rg --no-ignore -c '^\s*(async\s+)?def ' /home/paul/library-context/build/envs/fastmcp/lib/python3.14/site-packages/fastmcp -g '*.py' | awk -F: '{s+=$2} END{print s}'` (already run: 2,394).
  - Also count module-level public `def`s, as a proxy for seeds: `rg --no-ignore -c '^(async\s+)?def [a-z]' …`.
  - *Discriminating outcome:* hundreds of seeds or more with V in the tens of thousands supports a material quadratic term at pilot scale; tens of seeds would make C-09 an S02-only concern.
- **No light probe can settle** the RT costs in C-04 and C-05, or the share of extraction in a compile (C-01). Closure evidence for those needs an authorized compile with `lctx_phase` spans at debug level, plus the `compiler product disposition` trace that `DependencyGraph::trace` already emits.
