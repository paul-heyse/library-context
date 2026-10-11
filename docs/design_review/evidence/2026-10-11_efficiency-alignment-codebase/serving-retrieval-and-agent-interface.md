# Inquiry E: Serving, retrieval and the agent interface

**Provenance.** This is the verbatim final report of supporting inquiry E, from one read-only
`design-reviewer` agent, for the [codebase efficiency-alignment review](../../reviews/design_review_efficiency-alignment-codebase_2026-10-11.md).
It was produced on 2026-10-11 at baseline `88f33bc0`, against the [inquiry briefs](inquiry-briefs.md).
Only this header was added.

How to read it:
- Candidate labels such as `E-01` are local to this report. They are not the review's finding
  IDs.
- Its judgments are bounded and supporting. The principal review owns the combined decision.
- See the [evidence index](README.md) for coordinator checks and reconciliation.

---

# Inquiry E: Serving, retrieval and the agent interface

**Kind:** supporting assessment for the codebase efficiency-alignment review (DESIGN/TARGET tier). It is bounded and read-only, and is not the principal review.
**Baseline:** `main` at `88f33bc0`, 2026-10-11.
**Reviewer:** `design-reviewer` (inquiry E). Local candidate labels are `E-nn`. The principal reviewer owns finding IDs and verdicts.

The repository's own uncertainty labels have one difference from the common brief. The working tree also holds **uncommitted edits by another session** to:
- `crates/lctx-surrealdb/src/{derived_search,reader,selection}.rs`;
- `docs/design/sections/synthesis-and-serving.md`.

I assessed the **HEAD** content of these files, using `git show HEAD:…`. The in-progress change is noted where it matters, in E-04 and the seam notes.

---

## 1. Scope examined and coverage limits

**Read in full or at the operative grain:**
- `crates/lctx-serving`:
  - `service.rs` (`NativeService::run`);
  - `operations.rs` (`dispatch`, `scores`, `fusion`, `search_*`, `browse`);
  - `search.rs` (`lexical_scoped`, `vector_scoped`, `rescore_union_scoped`, `score_documents`, `rank_occurrences`, `insert_witness`, `vector_selection_sql`);
  - `selection.rs` (`members`, `classify_inner`, the four discovery SQL functions);
  - `preparation.rs` (`PreparedCache::key`, `available_key`, `RequestAdmission`, `Preparation::hydrate_exact`);
  - `library.rs`;
  - `pagination.rs` (`binding`, `page`);
  - `ranked_results.rs` (outline, plus `resume` and the binding sites);
  - `evidence.rs` (`body`, `derivations`);
  - `originals.rs` (`Prepared::range`);
  - `lib.rs` (`operation_definition`).
- `crates/lctx-surrealdb` at HEAD:
  - `derived_search.rs` (`selected_occurrences`, `indexed_occurrence_rows`, `lexical_occurrences`, `hydrate_occurrence_nominations`, `occurrence_rows`);
  - `reader.rs` (`selection_preparation`, `candidate_payload_rows`, `selected_membership`);
  - `selection.rs` (`SelectedPayloads::sparse_membership`, `validate_nominal_membership`);
  - `scope.rs` (`library_roots`, `candidate_query`);
  - `materialization.rs` (the `vector` table DDL);
  - the index DDL in `schema.rs` and `compiler.rs::VIEW_SCHEMA`.
- `lctx_model::domain::serving`: `resources::ResourceLimits`, `dispatch::{schema_for, wire_identity, admit_envelope}`, `mappings::identity`, `native_requests::definition`.
- Python:
  - `lctx_mcp/{__main__,native,embedder,wire}.py`;
  - `lctx_semantics/src/session.rs`;
  - `lctx_semantics/src/lib.rs` (the function inventory).
- Consumers:
  - the `crates/lctx-eval` dependency list and module header;
  - `scripts/programmatic_native.py::native_numeric_reference`;
  - `crates/lctx-serving/tests/native_search.rs` (the exact-vector EXPLAIN control).
- SurrealDB 3.3.0 pinned source:
  - `surrealdb-idx-3.3.0/src/trees/hnsw/{filter,index}.rs`;
  - `surrealdb-core-3.3.0/src/exec/operators/knn_topk.rs`, `exec/topk_pushdown.rs`, `exec/physical_expr/{subquery,ops}.rs`;
  - `kvs/index/tests.rs` (the KNN/MATCHES planner cases).
- The `neo4j-surrealdb` skill (probes SB037, SB038, SB039 and SB098) and the `fastmcp` skill (`fm.http_asgi`).

**Not read:**
- `delivery.rs`, `capability.rs`, `inspection.rs`, `operation_packet.rs` and `source_*`, beyond grep;
- `CanonicalBatches` internals;
- `RankedResults` internals beyond the binding and resume sites;
- publication-side materialization of `vector`, `lex_occurs` and `vec_occurs` (sibling D/C);
- `lctx tool`, the CLI serving path.

**Physical premises that remain unknown:**
- FastMCP 4.0.5 row volumes per occurrence family (operation map §5.6);
- actual SurrealDB plans, since no `EXPLAIN` was run.

---

## 2. Complete operations examined

**MCP cold start**

| Stage | Detail |
|---|---|
| Process | One stdio process per MCP client: `__main__.create_server` → `anyio.run`. |
| Session | `lifespan` → `to_thread(NativeSession)`. `session.rs::NativeSession::new` builds a **multi-thread Tokio runtime with default worker count** (one per logical CPU on this host) and runs `NativeReader::connect`. That costs about 3 RTs for connect, signin and `use`, plus the installation check, the publication read, and the durable reader pin and publication hold through control effects (operation map §3.1, about 12 RTs including writes to `native_installation:current`). |
| Service | `NativeService::new` does no I/O. |
| Definition check | Lazy, on the first request: one RT. |
| Live state | One gRPC session, the pin, `ReaderScope.originals`, a 256 MiB `shared_bytes` budget (`ResourceLimits::default`), `PreparedCache` (moka), `RankedResults`. |

**`search_operations` with a vector**

1. **Python.**
   - `EnvelopeAdmission.on_call_tool` pre-admits a refusal envelope.
   - `NativeExecutor.execute` runs PyO3 `wire_decode`, then takes `asyncio.Semaphore(2)`, waiting at most 1 s.
   - The task `_execute` calls `HttpEmbedder.embed`: a new `httpx2.AsyncClient`, one POST, then pydantic parsing, a pure-Python norm check over 4,096 components, and `tolist()` → `json.dumps`. It also makes two extra PyO3 crossings through `Spec.hash` and `Spec.projection_id`, each calling `embedding_recipe_identities`.
   - Finally `asyncio.to_thread(session.execute)`.
2. **Rust, `service.rs::NativeService::run`.**
   - `decode_request`, then `RequestAdmission`: one query permit plus one CPU permit held for the whole request, except while waiting on preparation.
   - Request `ResourceBudget::scoped` (64 MiB).
   - Definition `OnceCell`.
   - `ranked.resume`, which returns `None` here.
   - Two point reads to validate `EmbeddingSpec` and `ProjectionDefinition`.
   - `pagination::validate`, then `dispatch`.
3. **`operations::search_operations`.**
   - `library::resolve_prepared`, which runs **`scope::library_roots` every request** (a multi-hop membership-checked graph walk) before the cached hydrate.
   - `selection::members`, **every request**: discovery streams, a sort, then hydration of every `CatalogMember` in windows of 128.
   - `classify_prepared`: a cached hydrate keyed by the member roots, then `ClassificationData::visit` and `evaluate::Prepared::from_local_rows` **every request**.
   - `scores` runs four families **sequentially**, each as lexical then vector, then `rescore_union`.
   - `fusion`, then packets for every ranked hit, `retained.admit_copy`, and `pagination::ranked`.
4. **Lexical, per family** (`derived_search::lexical_occurrences`).
   - FULLTEXT `@OR@` nomination, then `lex_occurs` per 128 documents, then three exact-equality branches.
   - `hydrate_occurrence_nominations`, per 128 rows:
     - union the dependencies;
     - `selected_membership`, which is per window of 128 nodes **× per view**, plus alias-ancestry rounds and `validate_nominal_membership` (per relation × per view × per 128 keys);
     - `SELECT * FROM $nodes`;
     - push into `SortedRows`.
   - Then a full validation pass and a rewind, then emission.
   - `occurrence_frontier` re-sorts occurrences and documents into two more `SortedRows`.
   - `score_documents`: one BM25 `RETURN` block per 128 documents.
   - `rank_occurrences`: up to four full rewinds.
5. **Vector, per family.**
   - `selected_frontier` → `indexed_occurrence_rows` through `WITH INDEX eligible_input`. This takes **every eligible `vec_occurs` row for the input**, followed by the same hydration chain.
   - `score_documents` with `vector_selection_sql(1024)`: one RT per 128 *eligible* documents, each computing an exact cosine.
   - `rank_occurrences`.
6. **`rescore_union`.**
   - Re-queries `vec_occurs` through `window_occurrences` for at most about 1,024 nominated occurrences, hydrates them again (membership re-proved), then runs `SELECT id,full_key`.
   - Reads `FullValue` and `EmbeddingSpec` records per 64 keys, decodes each 4,096-float vector and computes the cosine in Rust.
7. **Response.**
   - `encode_json` (32 KiB default cap) → PyO3 `PyString`.
   - `wire_tool_result` (Rust re-decodes and re-encodes), then `CallToolResult.model_validate_json`.
   - The middleware `_admit` and the writer `admit_envelope`, each parsing and serializing again.

**Continuation of a ranked result**
- `ranked_results::resume` makes no store RT.
- It still calls `pagination::binding`, which runs `wire_identity()`, and runs `encode_json`.

**`get_evidence`**
- The original body is read as a range-bounded page: a cap of at most 32 KiB through `reader.original_bytes`, with chunk-key point reads. **Adequate.**
- `derivations` uses one `RETURN` block containing view-membership subqueries (see E-06).

**Paginated `browse_library` and `find_operations`**
- Each page re-runs `chosen` (members, then classification) and builds every packet before `pagination::page` filters `key > after`.

---

## 3. Candidate findings

### E-01: Compile-time constant identities are recomputed on every request, including continuations

**Observed cause.**
- `lctx-serving/src/lib.rs::operation_definition` blake3-hashes `serving-implementation.bin` and folds in `lctx_model::domain::implementation_digest()`, which hashes `model-implementation.bin`. It also folds in `native_requests::definition()` (ten more `include_bytes!` hashes), `mappings::identity()` and `wire_identity()`.
- `dispatch.rs::wire_identity` regenerates the JSON Schema of every tool's request and response through `schema_for`, which builds a new `schemars` generator per call, plus the failure and resource schemas.
- None of these values is memoized. Search coverage was `OnceLock`, `LazyLock` and `once_cell` over `crates/lctx-serving/src` and `lctx-model/src/domain/serving`; the only hits are unrelated `mappings` bindings.
- The per-request callers are:
  - `preparation.rs::PreparedCache::key` embeds `operation_definition()` in every exact cache key. `hydrate_exact` builds two keys (layout and closure) per `hydrate`, and a `search_operations` call performs at least two hydrates (library and classification), so it builds at least four keys. `available_key` builds the key a second time on a resource refusal.
  - `pagination::binding` calls `wire_identity()`. It runs for every page section and for `ranked_results::{resume, …}`.
- Observed sizes from existing build outputs dated 2026-10-10/11, under `~/.cargo/build/library-context/*/build/{lctx-model,lctx-serving}-*/out/`: `model-implementation.bin` is 8,259,597 bytes and `serving-implementation.bin` is about 838 KB. Each `operation_definition()` call therefore hashes about 9.1 MB and generates more than 20 schemas.

**Consequence.**
- **Under S06**, every call, including the "no store RT" ranked continuation and every cache-hit preparation, pays repeated whole-binary hashing and schema generation. Its value is fixed for the process.
- **Mechanism:** H12 (stable preparation is not separated from changing values), H26 (auxiliary hashing on the hot path) and H19 (an established property is re-derived).
- **Magnitude:** unmeasured.

**Simpler alternative.** Compute these process-constant identities once:
- `std::sync::LazyLock<ContentHash>` for `operation_definition`, `implementation_digest`, `native_requests::definition` and `wire_identity`; or
- emit the digest from the build scripts as a constant.

Use the hex string from the first-request definition check. The cache key can then include a 32-byte constant.

**Preservation.**
- The CI-13 binding of cursors, cache keys and the stored `fn::lctx_operation_definition` to the executable stays intact, because the value is identical.
- The first-use check against the stored function remains.

**Uncertainty.** Absolute cost per call. Schema generation could dominate hashing, or the reverse.

**Evidence label.** *Interface-checked* (source read). Sizes are observed files.

**Closure.** One computation per process. A control can assert pointer or identity stability across calls, or a counter if one is wanted. Existing tamper and definition-mismatch controls must still refuse.

### E-02: Publication-fixed domains are rediscovered per request because the cache key is the discovered set

**Observed cause.**
- `library::resolve_inner` calls `scope::library_roots` on every request. The walk covers:
  - packages → releases → distributions → inputs → corpora → runs;
  - sources → modules → coverage scopes over four fields → provider coverage.
  
  Each step is membership-checked through `candidate_query` and `selected_candidate_ids`. Only afterwards does it consult `Preparation::hydrate`, which is keyed by those `roots`.
- `operations::{search_operations, chosen, GetOperation}` call `selection::members`, which streams package → release → input → member discovery and hydrates every `CatalogMember` in windows of 128. That list becomes the roots of the classification cache key in `classify_inner`.
- After a cache hit, `ClassificationData::visit`, `evaluate::Prepared::from_local_rows`, `LibraryAdmissionData::visit` and `PreparedLibraryDomains::prepare` are rebuilt from the cached batches on every request.
- `service.rs::run` re-reads `EmbeddingSpec` and `ProjectionDefinition` on every vector search, although both are immutable under the pin for a given content-hash key.

**Consequence.**
- The cache saves only canonical hydration. The key derivation repeats the dominant discovery work (H14: "proving the input unchanged repeats most of the work being avoided").
- Rediscovery and member hydration grow with library size (S02).
- The packages scan without a library name grows with retained libraries (S08).
- This is paid by every `search_*`, `find_operations`, `browse_library`, `get_operation` and `compare_operations` call (S06).
- **Mechanism:** H12, H14, H6.

**Simpler alternative.** Key `PreparedCache` retention by **request parameters under the pin**, and retain the prepared products rather than only raw batches:
- `(library-name | none)` → `PreparedLibraryDomains` plus its roots;
- `(library, operation selector)` → `Members`;
- `(member set, selection inputs)` → `evaluate::Prepared`;
- `(spec, projection)` → the validated policy.

The existing moka retention, single-flight coalescing (`Flight`) and charging can hold these values.

**Preservation.**
- The pin makes these answers immutable, so CI-13 holds.
- Charges, eviction under budget and close-time draining (ADR-0145 active-state rules) must carry over.
- A miss must recompute rather than return a partial result (CI-08).

**Counterexample check.** A selection change (S05) cannot reach an existing process, because the service is pinned for its lifetime. A shared-process variant (see E-10) would add the publication handle to every key, which `PreparedCache::pin` already enforces.

**Uncertainty.** How expensive `from_local_rows` is relative to discovery.

**Evidence label.** *Interface-checked*.

**Closure.** A second identical request makes no discovery RTs. A control with a request budget smaller than the discovery cost passes on the second call. Unknown-library and empty-domain answers are unchanged.

### E-03: Exact view-membership answers are re-proved per window, channel, family and request

**Observed cause.**
- `derived_search::hydrate_occurrence_nominations` calls `reader.selected_membership` for each window of at most 128 nominations.
- `selection.rs::SelectedPayloads::sparse_membership` issues, with no memo:
  - one `compiler_view_member WITH INDEX view_nodes` query per 128 dependency nodes **per view**;
  - one alias-ancestry query per round;
  - `validate_nominal_membership` per relation × view × 128 keys.
- Within one `search_operations` call, units, windows, parts and bindings shared by lexical and vector occurrences across four families are proven again in each pipeline. `rescore_union_scoped` then re-proves the vector winners, which `selected_frontier` has already shown to be eligible.
- Across requests, the same dependencies are proven again.

**Consequence.**
- Under S06 and S02, store RTs grow with the number of nominated occurrences × views, and are paid again for each of the roughly nine pipelines and each request.
- Because the answers are immutable for the pinned views, this is repeated assurance with no new uncertainty (H18, H19, H12).
- The SA-F06 window batching resolved the per-occurrence scalar form, but not the repetition.

**Simpler alternative.** Add a charged, bounded memo of per-node `NodeMembership` answers (direct witnesses plus ancestry provenance) owned by the pinned `ReaderScope`, for example a moka cache keyed by the physical `RecordId`. `selected_membership` then queries only the misses. SA-F06 itself anticipated "a charged operation-local memo"; the pin allows extending it to the reader lifetime.

**Preservation.**
- Per-node answers keep view and alias provenance.
- "One selected dependency must not make the occurrence eligible" still holds.
- `validate_nominal_membership`'s check for conflicting variants must run once per node set, before an answer is memoized.
- Eviction must recompute; it must never return "absent".

**Counterexample check.** A memo that recorded only "selected = true" would erase the nominal-variant conflict refusal. The memo must hold the validated per-node witness set.

**Uncertainty.** The uncommitted `eligible_context_rows` change adds `Basic::load` and `Companions::witness_context` reads per window, which raises the per-window cost that this memo would amortize.

**Evidence label.** *Interface-checked*.

**Closure.** A control with shared dependencies across the four families shows each dependency's membership queried once per reader. Changed and missing dependencies, aliases and conflicts still refuse or exclude.

### E-04: The vector channel hydrates the whole eligible occurrence domain before scoring; eligibility runs ahead of the score order

**Observed cause.**
- `search.rs::vector_scoped` → `selected_frontier` → `derived_search::selected_occurrences(vec_occurs, ELIGIBLE)` → `indexed_occurrence_rows` streams **every** `vec_occurs` row whose `eligible_input` index prefix (`eligible, scope_input, family, …`) matches. That includes rows for retained realizations of the same input that later fail view membership.
- Every row goes through membership proof, `SELECT *` hydration, a sort with possible spill, a full validation read (`while sorted.next_row()?.is_some() {}` then `rewind`) and a second sort into `occurrence_frontier`.
- Only then does `score_documents` compute the exact cosine over *all* eligible documents, 128 per RT. `vector_selection_sql`'s `LIMIT 1024` has no effect inside a 128-document window.
- `rank_occurrences` then consults at most the top 1,024 documents.

**Consequence.**
- Per-call work is proportional to the eligible corpus, times retained realizations whose occurrence rows differ, rather than to the frontier the answer can use.
- Under S02 (5–20× code) and S06 (concurrent agents), the 30 s deadline turns this into refusals. H11: a protective limit does not establish feasibility.
- **Mechanism:** H5, H7, H10, H6 and the CI calibration row "Bounded answer, amplified work".

**Library semantics, settled at the pinned 3.3.0 source.**
- **HNSW `<|k,ef|>` is approximate.**
  - `surrealdb-idx-3.3.0/src/trees/hnsw/index.rs::knn_search` takes `ef`.
  - WHERE conditions are applied *during traversal*, through `trees/hnsw/filter.rs::HnswTruthyDocumentFilter`, with optional allow-list bitmaps.
  - The graph is per index over the **shared** `vector` table (`materialization.rs`, `DEFINE INDEX neighbor … HNSW`). Inserting unrelated publications' vectors changes the graph, so approximate results for a pinned publication are not stable.
  - That conflicts with the frozen-ranking guarantee in DESIGN §11.2 ("inserting unrelated content cannot change an older publication's ranking") and with CI-13.
  - Brute force is therefore justified, and **the absence of `<|k|>` is not the defect.**
- **Exact top-K is available natively.**
  - `<|k,COSINE|>` is a brute-force scan (skill probe SB037, executed embedded).
  - The exec engine's `exec/operators/knn_topk.rs::KnnTopK` keeps a bounded heap, and `exec/topk_pushdown.rs` adds a reject-only threshold for `ORDER BY … LIMIT`.
  - KnnTopK breaks ties by scan sequence. `ORDER BY score DESC, id ASC LIMIT n` keeps the product's id tie-break.

**Simpler alternative: eligibility on demand in score order.** The exact answer stays the same.
1. Score the coarse cohort server-side, through `vector WITH INDEX cohort`, which `derived_search` already uses. Use `encoder_hash, policy_key, family, library_input IN $input_keys` with `ORDER BY score DESC, id ASC` in tiers of 128, 256, 512 or 1,024, without the eligibility pre-pass.
2. For the returned documents only, nominate `vec_occurs WITH INDEX document_occurrences WHERE in IN $docs AND ELIGIBLE` and run membership (plus the in-progress context checks) on those rows.
3. Count only documents that have at least one eligible occurrence toward the tier. Expand the tier until `cap` distinct targets or cohort exhaustion.

This gives the same frontier as today's "eligible documents ranked, top tier", because ineligible documents never consume a tier slot, and the order and ties are identical.

**Preservation.**
- Exactness, which the EXPLAIN control in `tests/native_search.rs` (`!plan.contains("KnnScan")`) protects.
- Deterministic ties.
- Eligibility before caps.
- Zero-score and exact-match branches, which stay independent.
- CI-04: an empty cohort or a cohort with no eligible documents reports the same as today.

**Counterexample check.** In `member_mode` with narrow `pairs`, most top documents may be ineligible. The lazy route then examines more documents before stopping, but never more than today's full pass. Every examined occurrence is still checked exactly once.

**Uncertainty.**
- Whether the cohort contains many vectors with no eligible occurrence in this view: retained realizations, or other libraries sharing an input. If it does, server-side scoring grows while client-side hydration shrinks.
- The same reorder applies to the lexical channel for broad `@OR@` queries. That was not analysed in detail.

**Evidence label.** *Interface-checked* for the diagnosis and the library semantics. The remedy is *Proposed*.

**Closure.**
- An equivalence control (old route against new) over a corpus with ineligible top documents, ties at the tier boundary, and retained-realization rows.
- A count of hydrated occurrences bounded by the examined frontier rather than the corpus.

### E-05: The vector contract is stated two ways, and the shared HNSW index has no production reader

**Observed cause.**
- Current owners disagree:
  - `api-and-evidence-product.md` §14 says "native HNSW candidate ranks" and "scoped HNSW candidate paths remain selected";
  - ADR-0131 says "declared approximate HNSW";
  - `docs/surrealdb.md` says "shared-vector HNSW candidates";
  - but `synthesis-and-serving.md` §11.2 (ADR-0145) says "explicitly identified exact eligible-vector route", and `search.rs::vector` says "approximate candidate behavior is not part of this policy".
- `materialization.rs` still defines `INDEX neighbor … HNSW DIMENSION 1024`. The only `<|k,ef|>` users are:
  - `scripts/programmatic_native.py::native_numeric_reference`, whose basis reads "actual indexed HNSW1024 … k/ef128";
  - a 2026-10-05 evidence probe.
  
  Search coverage: `rg '<\|'` over the repository excluding `target`, `third_party` and `.md` files, plus `rg 'neighbor|HNSW'` over `crates/*/src`, `crates/*/tests`, `python` and `scripts`.

**Consequence.**
- Every vector insert pays HNSW graph maintenance and storage on a shared table with no serving consumer (H27; S02, S05, S08).
- The numerical evaluation control qualifies a mechanism the product does not use, so it gives no evidence about the served route (CI-12 relevance).
- A future reader of §14 may "restore" HNSW and silently break the frozen-ranking guarantee.

**Simpler alternative.**
- Make one owner (§11.2) declare the exact route.
- Retire the `neighbor` index, or keep it only under a named consumer.
- Point `native_numeric_reference` at the served exact route.

**Preservation.** The analytical exact-neighbor contracts, which are separate.

**Uncertainty.** Whether an E1 analytics or rebuild path intends to use `neighbor` later. No consumer was found.

**Evidence label.** *Interface-checked*.

**Closure.** After one owner statement is chosen, the index exists only alongside a reader, and the eval control exercises the production SQL.

### E-06: View-membership subqueries risk per-row evaluation of the whole view, and one path joins through an unindexed field

**Observed cause.** Six serving queries filter with `… IN (SELECT VALUE node FROM compiler_view_member WHERE view IN $lctx_views)`:
- `selection.rs::{library_packages_sql, library_releases_sql, library_inputs_sql, member_candidates_sql}`;
- `evidence.rs::derivations` (twice).

In pinned 3.3.0, `exec/physical_expr/ops.rs` evaluates the right operand per row (`eval!(self.right)`), and `exec/physical_expr/subquery.rs::ScalarSubquery::evaluate_batch` states "Each row runs the subquery plan". I found no hoisting of uncorrelated subqueries. Search coverage: `exec/planner`, `exec/physical_expr` and `exec/planner/select/*` for subquery, fold and hoist terms.

`member_candidates_sql`'s `$path` branch additionally scans `compiler_view_member WHERE view IN $lctx_views AND node.anchor IN $access` per candidate. `node.anchor` is a record-link dereference with no index; `view_nodes` is on `(view, node)`.

**Consequence.**
- If per-row evaluation holds, each outer row (packages, releases, member candidates, or edges incident to an evidence root) re-materializes the complete view membership, or every view member's `node.anchor`. The cost is O(rows × |view|).
- Evidence derivations on high-degree roots are the worst case (H10; S02, S06).
- The rest of the codebase already uses the sparse shape (`view_nodes` membership for a node set).

**Simpler alternative.** Do one of the following:
- nominate first and check through `NativeReader::selected_candidate_ids` (the established sparse route);
- use an index-backed correlated point probe, such as `count(SELECT … FROM compiler_view_member WITH INDEX view_nodes WHERE view IN $lctx_views AND node=$parent.id LIMIT 1)`;
- for the path predicate, resolve the access module's node first, then compare.

**Preservation.** Exact multi-view, alias and nominal-conflict semantics, which `selected_membership` already owns. This remedy *removes* a second, SQL-local definition of view membership (DP-01).

**Uncertainty.** Material. The legacy compute engine or a planner rewrite might evaluate the subquery once. See probe P-E1. If it does, this candidate reduces to a duplicated membership definition.

**Evidence label.** *Interface-checked* (pinned source). Behavior is not observed.

**Closure.** `EXPLAIN ANALYZE` on the fixture service for `member_candidates_sql` with and without `$path`, then a high-degree-root `get_evidence` control.

### E-07: Ranking kernels scan the whole frontier for every row

**Observed cause.**
- `search.rs::rank_occurrences` does `documents.iter().take(tier).find(|(id,_)| id==&source)` for every occurrence row. That is a linear search over up to 1,024 documents, repeated across up to four rewinds of the full occurrence run.
- `score_documents` calls `best.rows.sort_unstable_by(document_order)` after **every** pushed score.
- `insert_witness` scans the whole `BTreeMap` with `max_by` on each eviction.

**Consequence.** With E-04 unremedied, the CPU work is O(occurrences × tier) per pass, per family, per request (H9, H10). This is CPU inside the request's CPU permit, while other agents wait.

**Simpler alternative.**
- Build a `HashMap<RecordId, (rank, score)>` once per tier.
- Keep a bounded `BinaryHeap` ordered by `(score desc, id asc)`.
- Keep a heap or ordered set for the witness frontier.

**Preservation.** Identical total orders and tie-breaks. The existing unit controls (`primary_witness_ties_use_occurrence_key` and `bounded_witness_frontier_can_reenter_after_eviction`) already pin these.

**Evidence label.** *Interface-checked*.

**Closure.** The existing ranking controls pass unchanged, plus a large-tier equivalence case.

### E-08: Key continuations recompute the complete domain on every page

**Observed cause.** `pagination::page` sorts the full `values` vector and applies `key > after`. The callers build that vector completely on every page:
- `operations::browse` (members, classification, all packets, and vocabulary classification for every eligible candidate);
- `FindOperations`, three `page` calls;
- evidence section pages.

**Consequence.** Enumerating a library through `browse_library` costs one complete computation per page, quadratic in pages (H6, H12; S02, S06). Ranked search avoids this through `RankedResults`; key continuations do not.

**Simpler alternative.** Do either or both:
- once E-02 retains `Members` and `Prepared`, the per-page recomputation shrinks to packet building;
- build packets only for keys after the cursor, if `candidates::key` can be derived from the candidate without the packet.

**Preservation.** Complete and deterministic paging, `CompleteDomain` totals and cursor binding (CI-08, CI-13).

**Uncertainty.** Whether `candidates::key` needs the full packet. Not checked.

**Evidence label.** *Interface-checked*.

**Closure.** A page-N request performs no member discovery, and its items and continuation are byte-identical to today's.

### E-09: Python admission holds one of only two slots during the embedding HTTP call, independently of Rust admission

**Observed cause.**
- `native.py::NativeExecutor` takes `asyncio.Semaphore(2)` (1 s wait, then `resource_refused`) and holds it through `HttpEmbedder.embed` (10 s timeout) and the whole native call.
- Rust then applies `ResourceLimits::default().query_connections = 2` and `cpu_jobs = 2` through `RequestAdmission`.
- The two limits are configured independently: the Python constant does not read the session's limits.

**Consequence.**
- Two searches waiting on vLLM refuse every other call, including `get_evidence` with no embedding, after 1 s, while Rust capacity is idle.
- In the other direction, Rust's semaphore is never contended by MCP alone.
- **Mechanism:** H21, layers admitting independently. Scenario S06.

**Simpler alternative.**
- Bound only what Python owns: pending worker tasks, plus a separate embedding-concurrency limit derived from the vLLM capacity.
- Let Rust `RequestAdmission` own native capacity, with its limits exposed through `NativeSession`.

**Preservation.**
- Python's ownership of cancelled workers until drain.
- The 30 s end-to-end deadline, which already threads `remaining_ms`.

**Evidence label.** *Interface-checked*. The consequence is reasoned, not observed.

**Closure.** A control with two hanging embeddings shows that a third non-search call is admitted.

### E-10: Each agent's stdio server holds private copies of state and admits work alone (deployment shape)

**Observed cause.** Stdio MCP gives one process per client. Each process:
- builds its own default multi-thread runtime (`session.rs`);
- runs a cold `PreparedCache` with a 256 MiB service budget;
- holds its own durable reader pin, written through installation-row effects;
- uses its own gRPC session, `RankedResults` and admission.

Nothing coordinates admission or memory across processes against the shared SurrealDB server and vLLM.

**Consequence.** Under S06, N agents mean:
- N cold preparations of the same immutable publication;
- N × 256 MiB of potential retention;
- N × 2 concurrent native pipelines, each with several streams;
- N startup pin effects contending on `native_installation:current`.

This shares H21 and H17 with inquiry H.

**Alternative.** A shared serving process over FastMCP 4.0.5's HTTP transport. `server.http_app(...)` exists with session management, per the skill brief `fm.http_asgi.r14.1` (Interface-checked from the skill). It would share one pin, cache, memo (E-03) and capacity pool. Costs and constraints:
- it needs byte admission at the HTTP envelope (`wire.py::response_encodings` already shapes an HTTP form);
- `RankedResults` session binding would have to be per MCP session;
- it changes the operator's client registration.

**Uncertainty.**
- How agents are actually deployed. Several agents might already share one server process.
- The magnitude of the effect.

This is a candidate, not an established defect.

**Evidence label.** *Interface-checked* / *Proposed*.

**Closure.** A deployment decision recorded with its capacity owner.

---

## 4. Opportunities and uncertainties that are not findings

- **Sequential families and channels** (`operations::scores`). These are independent reads; running them concurrently would cut latency but not work, and would multiply concurrent server streams under one request permit. Remove work first (E-03, E-04, H28). Revisit after those land.
- **A new `httpx2.AsyncClient` per call** (`embedder.py::HttpEmbedder.embed`). It costs one local TCP connection setup per search. A session-lifetime client closed in `NativeExecutor._drain` is a small clean-up. It is not material next to E-04.
- **Six or more PyO3 crossings and repeated JSON passes per call.** Responses are capped at 32 KiB by default and 256 KiB when expanded. `wire_decode` before embedding is purposeful: it refuses invalid requests before inference. The two-stage envelope admission (middleware to allow a structured refusal, writer for the actual bytes) has distinct purposes. These costs are proportionate.
- **The 4,096-float query vector crosses as JSON** (Python `tolist`/`json.dumps`, then Rust `serde_json`) and is checked twice (Python `check_vector`, Rust `check_vector`). It is bounded at about 100 KB. A float32 buffer through PyO3 would remove the conversions, at an integration cost. Low value.
- **Eager packets for every ranked hit** (`evidence::hits`, `candidates::packet`). Retention needs them for continuation without rediscovery. Laziness would beat eager amortization only if most sessions read only page 1. That is unmeasured, so this stays an opportunity (safeguard 6).
- **Lexical `@OR@` nomination of broad queries.** Exact BM25 over OR semantics with frozen statistics needs every matching document's score. The E-04 reorder may also apply, but it is not established here.
- **`ReaderScope.originals` linear `find` per range check** (`reader.rs`). This is minor.

---

## 5. Strengths, preservation constraints and adequate areas

1. **Continuations do no rediscovery and bind to the realization.**
   - `RankedResults` binds to the session, result digest, request, policy, wire and channel identity, and the snapshot.
   - An evicted entry refuses rather than re-executing (`ranked_results::resume`; the `expiry_foreign_session_and_eviction_refuse_without_rediscovery` control).
   - Original-body pages are offset-bound to the source key.
   - This is the CI-13 "pin covers the journey" property. Preserve it.
2. **Eligibility before caps; exact and frozen ranking.**
   - BM25 uses frozen publication-scoped statistics (the `RETURN` block in `lexical_scoped`).
   - Exact-match branches are independent.
   - Zero-score matches are retained.
   - Rejecting shared-index ANN is justified by the frozen-ranking guarantee, not by preference.
3. **Sparse membership with per-node provenance and nominal-variant refusal** (`sparse_membership`, `validate_nominal_membership`). This is the correct semantic owner. E-03 and E-06 recommend using it more, not less.
4. **Budgeted, spill-capable request state** (`SortedRows::with_budget`, `StateCharge`). Cancellation releases charges, as the tests in `search.rs` show.
5. **Original evidence is range-bounded** (`evidence::body` with a 32 KiB cap and chunk point reads). This is adequate for H5 and H16.
6. **No startup hydration of unrelated corpus.** Definition checking is lazy and done once.
7. **`lctx-eval` is deliberately independent.** It depends only on `serde`, `serde_json`, `schemars` and `blake3`, and its module header says "No production imports or serving dependencies". This is a non-circular evaluation decoder (CI-12). Keep the duplication.
8. **Python does not classify.** Rust owns operation meaning, schemas and failure kinds (the `wire.py::register` inventory from Rust).
9. **Preparation coalescing.** `PreparedCache` single-flight with `suspend` and `resume` admission avoids a waiting request starving the only loader slot. This is a real H21 design point inside one process.

---

## 6. Positions on earlier leads

| Lead | Position | Evidence |
|---|---|---|
| HS-integrated F01: sparse reads hydrate the complete selected payload universe | **Resolved** (for its stated cause). A related residual has moved to E-04. | `reader.rs::candidate_payload_rows` and `selected_membership` bypass `selection_preparation`. The search channels instead enumerate the complete eligible *occurrence* domain (`indexed_occurrence_rows`). |
| HS-integrated F03: retained preparation bypasses the composed budget | **Resolved** (source-read) | `selection_preparation` requires a caller budget and checks `shares_pool`. `NativeService` threads shared and request budgets (`with_budget`, `with_request_budget`). Not tested. |
| SA-F04: limited relation read prepares the whole membership | **Resolved** (source-read, not examined deeply; sibling D) | `reader.rs::relation_bodies` uses `SelectedPayloads::sparse_relation_rows`. |
| SA-F05: lexical abandons native selective nomination | **Resolved** | `derived_search::lexical_nomination_sql` uses table-source `WITH INDEX lexical … @OR@`. Broad terms remain inherently large (§4). |
| SA-F06: occurrence eligibility sequential per occurrence | **Residual** | Window batching with a dependency union and batched payload is implemented (`hydrate_occurrence_nominations`). Repetition across windows, channels, families and requests remains (E-03). The uncommitted context-eligibility work adds per-window reads. |
| SC-F01 (2026-10-05): native fusion does not implement member/family ranking | **Resolved** | Fusion is Rust `CandidateFusion` (`operations::fusion`). `rg 'search::rrf\|search::linear'` over `crates` finds nothing. |
| SC-F02: DEFINE API authorization | **Refuted for current scope** (not adopted) | No `DEFINE API` in `crates` (rg). |
| SC-F03: executable serving behavior mutable | **Resolved** | `fn::lctx_operation_definition` is checked on first use against `operation_definition()` (`service.rs::run`). The handle carries `definition_epoch`. The cost side is E-01. |
| NE-I01: failed local preparation retained as immutable | **Not examined** (membership preparation recovery, sibling D/G) | — |
| NE-I02 to NE-I11 | **Not examined** (compile-side) | — |
| Operation map: new HTTP client per call | **Confirmed still present**; judged minor (§4) | `embedder.py::HttpEmbedder.embed` |
| Operation map: six PyO3 crossings and repeated JSON | **Confirmed** (at least 8 counting the `Spec` identity calls); judged proportionate | `native.py`, `wire.py`, `embedder.py::Spec.hash` |
| Operation map: uncached `selection::members` | **Confirmed**, and broader than mapped | E-02, which also covers `library_roots` and classification preparation |
| Operation map: four families sequential | **Confirmed**; an opportunity, not a finding (§4) | `operations::scores` |
| Operation map: vector nominates all eligible rows, scored brute force | **Confirmed**; brute force justified, ordering not | E-04 and E-05 |
| Operation map: sparse membership per W × per view | **Confirmed** | E-03 |
| Operation map: continuation makes no store RT | **Confirmed**, but still recomputes constant identities | E-01 and §5.1 |

---

## 7. Candidate rule impacts

| ID | Rule and where recorded | Proposed change | Depends on | If the rule is kept |
|---|---|---|---|---|
| RC-E1 | "Declared approximate HNSW" and "native HNSW candidate ranks": ADR-0131, `api-and-evidence-product.md` §14 (two passages), `docs/surrealdb.md` (retrieval paragraph) | Declare the exact eligible-vector route as the sole contract (as §11.2 and ADR-0145 already do). Retire the shared HNSW index, or name its consumer. | E-04, E-05 | If approximate HNSW is retained as the contract, the shared-table index makes rankings depend on unrelated content, which contradicts the §11.2 frozen-ranking guarantee. The recommendation becomes a per-publication immutable index, which is not available per table. |
| RC-E2 | "Eligibility precedes ranking" / "Eligibility checks those dependencies before ranking limits": §14, §11.2 | Clarify that eligibility precedes **caps and tier counting**, not the evaluation order of scores. | E-04 | Kept as a work-order rule, it forbids the lazy route. E-04 then reduces to E-03 and E-07 only. |
| RC-E3 | Process-per-client stdio serving with process-lifetime pins and session-bound continuations: §11.3, CI-13 implementation note | Allow a shared HTTP serving process with per-MCP-session continuation binding. | E-10 | E-10 reduces to documenting the per-process capacity envelope. E-02 and E-03 still apply within each process. |

---

## 8. Proposed light probes

- **P-E1 (decides E-06).** A source-only check of whether 3.3.0 evaluates the uncorrelated `IN (SELECT …)` once or per row, and which engine runs these statements.
  - **Command:**
    ```
    rg -n "plan_or_compute|fn plan_or_compute|ComputeOnly|AllReadOnlyStatements" ~/.cargo/registry/src/*/surrealdb-core-3.3.0/src/exec/plan_or_compute.rs ~/.cargo/registry/src/*/surrealdb-core-3.3.0/src/dbs/*.rs
    ```
    then read the select planner path that lowers `Expr::Select` in a WHERE operand to `ScalarSubquery`.
  - **Discriminating outcome:**
    - a stream-open, plan-time or `Literal` materialization of row-independent subqueries means E-06 is reduced to the duplicated-definition concern;
    - `ScalarSubquery::evaluate` per row (the current reading) confirms E-06.
  - **Stronger check (not a light probe; for the coordinator to schedule):** `EXPLAIN ANALYZE` of `member_candidates_sql` on the fixture service.
- **P-E2 (sizes E-01).** Count the `operation_definition()` and `wire_identity()` calls per request.
  - **Command:**
    ```
    rg -n "operation_definition\(\)|wire_identity\(\)|pagination::binding\(|available_key\(" crates/lctx-serving/src
    ```
  - **Discriminating outcome:** more than one call per request on the cache-hit and continuation paths confirms the structural repetition, which the static read already indicates. Magnitude needs a built benchmark, which is outside light-probe scope.
- **P-E3 (bounds E-04's growth claim).** Read-only inspection of existing run evidence, without querying SurrealDB:
  ```
  rg -l "vec_occurs|eligible_input" build/runs/*/ 2>/dev/null
  ```
  - **Discriminating outcome:** a retained capture listing eligible `vec_occurs` counts per family for the FastMCP pilot would turn "proportional to corpus" into a stated FastMCP-scale fact.
  - If none exists, the count needs an authorized store query (coordinator decision).

---

**Summary for the principal reviewer**

**Main defect.** The serving path is semantically careful but re-derives pinned, immutable facts on every call:
- E-01: constant identities;
- E-02: library and member domains, and cache keys built by rediscovery;
- E-03: membership proofs.

It also enumerates the complete eligible vector domain before a frontier-bounded scoring step (E-04).

**Brute-force vector scoring is required.** It is required by the shared-store frozen-ranking guarantee at SurrealDB 3.3.0. The remedy is to reverse the order of eligibility and scoring, not to adopt HNSW. The unused HNSW index and the contradictory "approximate HNSW" statements should be retired together (E-05, RC-E1).

**Lower priority:**
- E-06 depends on probe P-E1;
- E-07 is algorithmic clean-up;
- E-08 covers paging recomputation;
- E-09 and E-10 cover capacity coordination, and E-10 depends on how agents are actually deployed.
