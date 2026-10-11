# Inquiry G: Assurance, identity and observation

**Provenance.** This is the verbatim final report of supporting inquiry G, from one read-only
`design-reviewer` agent, for the [codebase efficiency-alignment review](../../reviews/design_review_efficiency-alignment-codebase_2026-10-11.md).
It was produced on 2026-10-11 at baseline `88f33bc0`, against the [inquiry briefs](inquiry-briefs.md).
Only this header was added.

How to read it:
- Candidate labels such as `G-01` are local to this report. They are not the review's finding
  IDs.
- Its judgments are bounded and supporting. The principal review owns the combined decision.
- See the [evidence index](README.md) for coordinator checks and reconciliation.

---

# Inquiry G: cross-cutting assurance, identity and observation

This is a bounded supporting assessment for the codebase efficiency-alignment review (DESIGN/TARGET tier). Baseline is `main` at `88f33bc0`, read-only. Labels such as `G-01` are local candidates; the principal review assigns the finding IDs.

## 1. Scope examined and coverage limits

**Read in full or at decision grain:**
- Identity primitives:
  - `lctx_model::domain::identity` (`Id`, `ContentHash`, `ContentHasher`, `KeySink`);
  - `completed::{payload_address, attempt_contribution}`;
  - `scope_program::ScopeInterner::intern_with` (xxh3);
  - `graph::Entity::content`;
  - `record::{Batch::with_reservation, Batch::read}`;
  - `batching::BatchWriter::admit`;
  - `model::{Relation::canonical, canonical, hash_rows, Invariant::digest}`.
- The effect protocol: `lctx_surrealdb::control::{run_transaction, ensure_rows_owned, hold_many, guard_id, pin_views}` and `control/authorization.rs` in full.
- The write path: `compiler.rs::NativeCompilerStore::write_batch_inner`, `adapter::{select, physical_row}`, `loader::{entity_payload_id, payload_id, validate_native_row, NativeWindows}`.
- Completion: `complete_contribution_inner` (membership readback).
- State identity and cold validation:
  - `compiler.rs::{completed_state*, state_identity_*, capture_state, validate_state_row, validate_compiler_backing, validate_backing_batch, verify_state_selected, capture_audit, verify_captured}`;
  - `persist_original_chunk`;
  - `ensure_immutable_rows`.
- Admission:
  - `cpg_core::artifact::{admit, verify_export, verify_restored}`;
  - `native_canonical::{entity, assertion, verify, header_identity}`;
  - `artifact_manifest::populate`;
  - `workspace::{validation_premise, checked_premises}`;
  - `scoped_admission::{validate_support, validate_group, validate_grain, ownership_rows}`.
- Publication:
  - `lctx_publisher::{publish, seal_completed}`;
  - `native_publication::{Publication::seal, seal_staging}` and the readback;
  - `reconciliation::Loader::reconcile`;
  - `derived_search::{materialize_search, reconcile_search, Expected::reconcile}`;
  - `lexical_stats::{materialize, reconcile}`;
  - `inspection::audit_owned`.
- Capture and acquisition: `cpg_extract::capture::{CapturedInput::capture, emit_chunks, verify}`, `acquisition::capture_receipts`, the `Acquire` provider tail, `facts::compile_facts`, `lctx_model::domain::artifact::{ArtifactCapture, content_invariants}`.
- Build-time identity:
  - `lctx-model/build.rs`, `lctx-serving/build.rs`, `scripts/producer_fingerprint.rs`, `cpg-core/build.rs`;
  - `cpg_extract::bundle::build_digest`, `cpg_core::workspace::execution_implementation`;
  - `lctx_serving::{operation_definition, native_definitions}` and the `service::NativeService::run` definition and vector checks.
- Serving decode: `batches::CanonicalBatches::canonical_nodes`.
- Product cache: `workspace_products::decode_product_rows`.
- Embedding tokenizer: `retrieval_tokenizer::LocalTokenizer::{load_verified, load}`.
- Tracing: `cpg_extract::logging`, `lctx_surrealdb::phase::Phase`, `cpg_core::stage_runtime::borrowed_cpu`.
- Tooling: `scripts/surrealdb_service.py::{_installed_binary, verify_binary}` and `verify_evidence/context.py::SourceObserver` (classification only; F2 owns these).
- MCP envelope admission in `lctx_mcp/wire.py`.
- Library check: `surrealdb-types-3.3.0` `Bytes` `Serialize`, which calls `serialize_bytes`, so serde_json emits an integer array. The SDK's `method/query.rs` sends every bound variable with the query.

**Not read:**
- `restore.rs`, `backup.rs` and `upgrade/*` beyond grep;
- `derived_search::lower` internals and `lexical_stats::prepare`;
- `selection.rs`, `loader::insert` / `ensure_window`;
- `lctx-eval` (non-circularity there is left to E);
- Python `embedder.py` and `native.py` beyond grep;
- `verify_evidence/capture.py`.

Volumes are unknown (no product run). Every magnitude below is unmeasured unless it is labelled as a historical receipt.

**Records read:** I read existing logs under `build/runs/*/verify/*.log` with `rg` only. The runs date from 2026-10-09 to 2026-10-11, mix revisions and use fixture-scale tests. They are cited as historical receipts, not fresh runs.

## 2. Complete operations examined (assurance and identity view)

1. **Provider row to durable row (S01).** Each stage re-establishes validity or identity that the previous stage already held.
   - `BatchWriter::admit` (validate, `content_digest`, duplicate index)
   - → `Batch::with_reservation` (validate again, sort, dedup, `R::encode`)
   - → bridge `mpsc(2)` / `NativeCalls`
   - → `write_batch_inner`:
     - `Relation::canonical`: decode, sort, dedup, encode;
     - `batch_bodies`: Arrow to `Value`;
     - `adapter.graph`: decode again, plus `validate`;
     - per row: `entity_payload_id` (`entity_views`, `content()`), `content()` again, JSON of the row, then `physical_row` parses that JSON back to recover the id and walks `validate_native_row`;
   - → `ensure_rows_owned` (`validate_native_row` again, `guard_id`)
   - → `effect_for_owner`, which hashes `serde_json(sql, bindings)`
   - → `execute_owner`, which hashes it again
   - → the intent transaction, carrying all bindings
   - → the payload transaction, carrying all bindings
   - → the release transaction.

   Server side, each row is compared as a full `Value` with any existing row.
2. **Completion.** One streamed readback of the contribution's memberships builds the per-output `KeySink` digests. These become the view identity.
3. **Admission** (`artifact::admit`).
   - Freeze, then `admit_semantics` with invariants memoised per exact-view premise.
   - `artifact_chunk_content` re-reads every original byte from the store and rehashes it.
   - A full canonical scan follows, with per-row `verify` (JSON decode, views ×2, `content()` ×3, `validate`).
   - Then the manifest: `completed_state(true)` re-validates and serialises every state row.
4. **Publication** (`seal_completed`).
   - `artifact.entities()/assertions()` rescans with per-row `verify` for references.
   - `materialize_search` lowers, writes and reconciles against actual rows; lexical statistics are materialised and checked.
   - `seal` then runs, in order:
     - `completed_state(true)` again;
     - `Loader::reconcile`: full canonical decode and physical rebuild, family hashes, expected edges and an all-originals rehash;
     - `reconcile_search`: the same `lower` run again, a readback and the lexical statistics again;
     - `seal_staging`;
     - a reader readback through `NativeReader::connect`.
5. **Serving call.**
   - The definition check runs once per session (`OnceCell`).
   - Every search call reads `EmbeddingSpec` and `ProjectionDefinition` and checks the input hash.
   - Canonical hydration validates each row two or three times (`CanonicalBatches::canonical_nodes`).
   - Envelopes are admitted twice in Python.
6. **Observation.** The default filter is `warn,lctx_phase=info`. Phase begin and terminal events go to stderr. `borrowed_cpu` emits them once per selected root.

## 3. Candidate findings

### G-01: The effect request digest serialises and hashes the whole payload twice, and nothing in production compares it with an independent value

**Cause.**
- `control/authorization.rs::effect_for_owner` computes `ContentHash::of(serde_json::to_vec(&(sql, &bindings)))`.
- `execute_owner` recomputes the same digest and compares the two (the "native retry payload changed" check).
- The only caller that supplies a separately retained `NativeRequest` is `execute_request`, and its only callers are tests (`crates/lctx-surrealdb/tests/native_control.rs`). `NativeRequest::issue` and `history::retain_outcome` have no non-test callers.
- In production, every later comparison of `request` checks a value written from the same in-memory `NativeRequest`:
  - the intent `$__prior` branch, which a fresh `fresh_identity("effect")` can never reach;
  - the payload `$__operation.request` check;
  - `request_disposition` in the failure path.
- The payloads are `ensure_rows_owned` / `hold_many` windows of up to 128 rows and `TransferLimits.bytes`. They include `canonical` `Bytes` columns and 64 KiB `original_chunk` bodies. serde_json encodes `Bytes` as an integer array (`surrealdb-types 3.3.0 value/bytes.rs::Bytes::serialize` calls `serialize_bytes`), so the JSON is several times larger than the binary payload.

**Consequence.**
- Under S01 and S02, every write window pays two whole-payload JSON serialisations and two BLAKE3 passes of auxiliary work.
- That work grows with fact volume and produces no assurance: H20 says "repeats unchanged conclusions", and H26 asks for auxiliary hashing proportionate to the capability it supports.

**Simpler alternative.**
- Compute a request digest only where a retry handle is retained (the `NativeRequest::issue` / `execute_request` flow).
- For `effect_for_owner`, key the receipt by its operation identity, with `request` set to a cheap stable value, or to a digest over the rows' content-addressed `id`s rather than the JSON payload.
- If cross-process retry under S07 is later designed, derive the request identity from the already content-addressed row ids and the SQL text.

**Preserve:**
- refusal of a changed payload under a genuinely retained retry;
- ambiguous-failure disposition through the operation identity;
- era and owner fencing.

**Uncertainty.** Magnitude is unmeasured. A planned production retry consumer would change the remedy, but not the diagnosis.

**Label.** Diagnosis Implemented (source-inspected); serde behaviour Interface-checked at the pinned source; remedy Proposed.

**Closure.** One digest or none per effect, with retry-payload-change refusal kept for the retained flow.

### G-02: The intent transaction ships the full payload bindings it never references (seam with D/H)

**Cause.**
- `execute_owner` runs the intent with `bindings.clone()`.
- `control.rs::run_transaction` clones again for each try (`client.query(..).bind(bindings.clone())`).
- The intent SQL references only the `$__*` control variables, but the SDK sends every bound variable (`surrealdb` `method/query.rs`). Each effect therefore transmits its payload over gRPC twice.

**Consequence.** Under S01, S02 and S06, write-path bytes are doubled and a deep clone is taken for each transaction attempt (H16, H8).

**Alternative.** Bind only the control variables to the intent transaction.

**Preserve.** The intent must still record the request and owner identity.

**Uncertainty.** I did not check server-side parsing cost.

**Label.** Implemented; Interface-checked (SDK). **Closure.** Intent bindings contain only the `__*` keys.

### G-03: The store re-establishes `Batch<R>` invariants because its API erases the typed proof

**Cause.** The single production write leaf, `cpg_core::workspace::owned_batch_write`, passes `batch.arrow()` from a `Batch<R>`.
- `Batch::with_reservation` has already validated, sorted and dedup-checked those rows.
- `BatchWriter::admit` validated each row before that.

`write_batch_inner` then:
- runs `Relation::canonical` (`model.rs::canonical`: `R::decode`, sort, duplicate check, `R::encode`);
- decodes again in `adapter::select`'s graph decoder, with `entity.validate()` (a third validation);
- calls `loader::entity_payload_id`, which builds `entity_views` for one row and calls `content()`, while the caller calls `content()` again;
- for compiler records, hashes the same JSON bytes twice (`ContentHash::of(&bytes)` twice);
- in `adapter::physical_row`, parses the canonical JSON back (`serde_json::from_slice::<Entity>`) only to recover `entity.id()`, which the caller already holds;
- calls `validate_native_row` there, and `ensure_rows_owned` calls it again.

**Consequence.** Under S01 and S02, an extra Arrow round trip per batch and several per-row re-derivations of properties the typed value already carries (H19, H16).

**Alternative.**
- Carry validity through types: a `CanonicalBatch` token constructible only by `Batch::with_reservation` or `Batch::read`. The store skips `canonical()` and decodes graph rows once.
- Pass the typed graph row and its id into `physical_row`, and compute `content()` once per row.

**Preserve.** Refusal of non-canonical or conflicting input from any untyped entry point. Tests use `write_batch` with raw `R::encode`, so keep a checked constructor for them.

**Label.** Implemented; remedy Proposed. **Closure.** Trace one decode per batch on the hot path, with duplicate and conflict controls still refusing.

### G-04: Cold-exposure backing validation runs twice on every ordinary compile

**Cause.**
- `compiler.rs::completed_state` → `completed_state_inner` → `state_identity_inner(true, …)`. The `validate_backing = true` path:
  - runs `validate_state_row` → `validate_compiler_backing` on every `compiler_record` row, which serialises and parses the body, rehashes, rebuilds `physical_row` and compares the JSON;
  - runs `validate_backing_batch` per window: `ProjectedBuilder` (`Value` to Arrow), `Relation::canonical`, `batch_bodies`, then JSON, BLAKE3 ×2, `physical_row` and a JSON compare;
  - serialises every state row to JSON into the `KeySink`.
- It runs once in `artifact_manifest::populate` and again in `Publication::seal` (`completed_state_after_closure`).
- The import path uses `state_identity_inner(false, …)` after its own independent validation (comment at `state_identity_inner`: "without repeating model reconstruction"). The persisted-graph F01 correction placed this reconstruction at cold exposure: "ordinary selective views carry their completed validity". The ordinary compile path contradicts that boundary.

**Consequence.**
- Under S01, S02 and S03 (including unchanged warm recompiles), every backing row is reconstructed about ten serialise/parse steps deep, twice per compile.
- **Historical receipt (unattributed):** in `build/runs/20261011T095410.471Z-4cb00b` (`serving:mcp` journey, 2026-10-11):
  - `artifact_manifest` took 65.1 s;
  - `publication_seal` took 63.4 s, of which `publication_final_reconciliation` took 14.2 s;
  - `artifact_graph_admission` took 0.8 s.

  Other journey runs on 2026-10-11 show `artifact_manifest` at 84–98 s. A shared `completed_state(true)` cost is consistent with this pattern, but the phases do not attribute it. Mechanism: Q17, A4, H19, H20.

**Alternative.**
- Same-process manifest and seal use `validate_backing = false`, as import already does.
- Keep `true` for `inspection::audit`, import, export and restore.

**Preserve.** The cold corruption controls of persisted-graph F01 (`tests/compiler_views.rs`), and an exact state identity.

**Uncertainty.** Whether every closure row in an ordinary attempt is either written by this attempt or attached from earlier admitted content. I believe it is, from the attach path; this is not traced exhaustively.

**Label.** Implemented; remedy Proposed. **Closure.** Ordinary compile shows no `validate_backing_batch` call, and the cold controls still refuse.

### G-05: Same-process seal re-derives what admission and materialisation already established

**Cause.** `Publication::seal` treats `Admission::Completed` (a same-process `AdmittedArtifact`) like `Admission::Export`. After `begin_finalization` drains operations, it repeats four things.
- **(a)** `completed_state_after_closure`, compared with the manifest identity computed minutes earlier (see G-04).
- **(b)** `reconciliation::Loader::reconcile`:
  - a third full decode, validation and `physical_row` rebuild of every entity and assertion, with a JSON comparison;
  - family hashes recomputed and compared with `manifest.families`, which `artifact::admit` computed from the same rows;
  - expected edges rebuilt with the same `codec::entity_references` the loader used to write them;
  - every original's chunks rehashed. The admission invariant `artifact_chunk_content` (`lctx_model::domain::artifact::content_invariants`) already re-read and rehashed those bytes against the same captured digests.
- **(c)** `derived_search::reconcile_search`: the same `lower` function a second time (labelled "independent cold lowering", but it is the producer), a second actual-row comparison, and `lexical_stats::reconcile` running `prepare` and `check` again. `materialize_search` had already reconciled expected rows against actual rows, and `lexical_stats::materialize` ends in `check`.
- **(d)** Before seal, `AdmittedArtifact::entities()/assertions()` re-run `native_canonical::verify` for every row merely to read references.

**Premise ownership.** The premises behind these conclusions are owned:
- the freeze (`freeze_content`) and the admitted fence in `ensure_rows_owned` ("native canonical content already admitted");
- content-addressed immutable rows with server-side full-value collision checks;
- holds and guards against retirement;
- the finalization lease.

**Consequence.** Under S01, S03 and S05, the publication critical path re-reads and re-derives the whole artifact two or three times. Historical receipts show `publication_final_reconciliation` up to 14.3 s and `publication_seal` up to 71.6 s, unattributed. Mechanism: H19, H20 ("repeating the producer is not an independent oracle"), A4.

**Alternative.** For `Admission::Completed`:
- Seal checks the premises in O(1):
  - the attempt is frozen and admitted;
  - no pending effects;
  - the lease is held;
  - optionally, an attempt-scoped mutation revision recorded at admission matches.
- Seal carries admission's family and state identities forward.
- Keep exactly one post-write readback per derived family, inside `materialize_search` (it catches extra eligible lineage rows that the collision check cannot).
- Replace the seal's original-byte rehash with a metadata equality check between `manifest.originals` and the `SourceArtifact` content and length. The admission invariant has already proved the bytes.
- Have `AdmittedArtifact` stream already-verified rows; `admit` verified them under the same freeze.

Keep the full cold path for `publish(VerifiedExport)`, restore and `inspection::audit`.

**Counter-case.** A foreign process writing into the same attempt id, or an operator's direct SQL, bypasses the protocol. The O(1) revision or pending-effect check covers the first case. Out-of-protocol mutation belongs to the cold audit, for example at `snapshot select`, not to every publication.

**Preserve:**
- CI-13 pinned serving;
- the reader-open readback;
- the eligibility reconciliation;
- independent cold admission at trust boundaries.

**Uncertainty.** I did not read the H23 and contention interplay of an attempt revision counter.

**Label.** Implemented; remedy Proposed. **Closure.** The seal trace shows no second `lower`, `completed_state` or canonical decode, and export, restore and audit corruption controls still refuse.

### G-06: Admission validity is carried only within one workspace, although its premise is content-addressed

**Cause.**
- `workspace::validation_premise` keys each invariant result by:
  - the model digest;
  - `Invariant::digest`;
  - the profile;
  - selectors;
  - the exact `view.identity` and `SourceSnapshot::identity`, none of which is attempt-scoped.
- The result is held in memory in `checked_premises`.
- An unchanged warm recompile (S03) yields equal view identities, but admission re-runs every invariant.

**Consequence.** Under S03 with unchanged inputs, full invariant admission is repeated. NE-F02 cited invariant-admission receipts near 900 s; I did not re-measure that (H12, H19).

**Alternative.** Durable checked-premise records owned by the admitted attempt, keyed as now plus `implementation_digest()` and `LCTX_COMPILER_SOURCE_DIGEST`. `Invariant::digest` covers name, purpose and input contracts, not code.

This is not "digest as semantics" (safeguard 3). The key identifies the exact inputs and the exact check code, so the carried property is a pure function of that key.

**Counter-case.** An invariant that reads anything outside its declared inputs would be carried wrongly. The in-workspace memo already relies on that same premise.

**Label.** Implemented (memo scope); remedy Proposed. **Rule impact:** RC-1.

### G-07: Per-root phase events at INFO by default

**Cause.**
- `cpg_core::stage_runtime::borrowed_cpu` creates an `info_span` plus `Phase::begin` and `Phase::finish` for each selected root. Call sites include `normalize/mod.rs` (per callable partition), `scoped_aspects.rs` (per partition), structural, summaries and analytic.
- `DEFAULT_LOG_FILTER = "warn,lctx_phase=info"`, so each root writes two stderr lines.
- `Phase::emit` also calls `register_callsite` on every emission.

**Consequence.**
- Under S02, observation volume grows with root count across stages. That contradicts `phase.rs`'s own "coarse runtime observations" (H26: no "event for every primitive operation").
- **Historical receipt:** `build/runs/20261009T214543.898Z-81f4a8/verify/compiler-producer.log` (fixture tests, 2026-10-09) has 4,602 of 5,074 `lctx_phase` lines (91%) as `borrowed_cpu`, each about 0.9 ms of work.

**Alternative.** Aggregate per stage (count, total and maximum elapsed) in the enclosing coarse phase, and move per-root events to `debug` or `trace`.

**Preserve.** Coarse begin and terminal semantics, and the cancellation and drain visibility.

**Label.** Implemented; receipt historical.

### G-08: Frozen-capture verification sits where no uncertainty enters, and is missing where it does (seam with A)

**Cause.**
- `capture_receipts` re-reads each frozen file for SHA-256 after `CapturedInput::capture` has streamed the same bytes through `copy_digest`.
- `facts::compile_facts` calls `captured().verify()` before the providers.
- The `Acquire` provider emits chunks through `emit_chunks`, which already verifies (`ArtifactCapture::finish` calls `ArtifactVerifier::finish`), and then calls `frozen.verify()` immediately afterwards.
- Pyrefly and ty then read the tree from disk, and no verification follows them.

**Consequence.** Under S01 and S03, three full closure reads repeat a conclusion that has not changed, while the stated property ("what is verified is what is analyzed") is not bracketed around the analysers' reads (H18).

**Alternative.**
- Compute SHA-256 inside `copy_digest`.
- Drop both extra `verify()` calls.
- If analyser-read integrity matters, run one verification after the last tree-reading provider, or a stat and ctime sweep against the capture `Stamp`s.

**Preserve.** The RECORD trust boundary and the stamp checks during copy.

**Label.** Implemented; remedy Proposed.

### G-09: Tokenizer assets are verified on one read and loaded from another

**Cause.** `LocalTokenizer::load_verified` hashes `tokenizer.json` and `config.json`, then `LocalTokenizer::load` reads both files again from their paths.

**Consequence.** The verified bytes are not the bytes in use (a TOCTOU gap; H18, CI-10), plus an extra read.

**Alternative.** Construct the tokenizer from the verified buffers with `Tokenizer::from_bytes` on the same `Vec`.

**Label.** Implemented; Low priority.

## 4. Opportunities and uncertainties that are not findings

- **Implementation identities are coarse.** The relevant sources:
  - `lctx-model/build.rs` (all of `src`, inline `#[cfg(test)]` modules, the root `Cargo.toml` and `Cargo.lock`, about 8 MB embedded);
  - `lctx-serving/build.rs::capture`;
  - `scripts/producer_fingerprint.rs` (plus `bundle::build_digest`, which adds `Cargo.lock` again).

  Equality is sound in the "same digest, same code" direction, except that Cargo features and build profile are not included. In the other direction, any test or comment edit, or any dev-dependency lock change, invalidates every product. Through `operation_definition`, it also makes `NativeService::run` return `Incompatible` for every publication, which forces a re-seal (S10, S05). This is conservative invalidation (H13 safeguard), so I list it as an opportunity: narrow `Cargo.lock` to each crate's resolved normal-dependency closure.
- **`completed_state` is a second, physical-level digest** of content that view and contribution identities already digest. It is useful for transport, restore and audit comparison. Lazy computation (H14) would change the manifest identity contract, so it is not proposed here.
- **Serving** (seam with E):
  - `EmbeddingSpec` and `ProjectionDefinition` point reads happen on every search call against an immutable pinned realization; they could be memoised per (spec, projection) under the pin.
  - The per-call input-hash check crosses a trust boundary, so keep it.
  - `CanonicalBatches::canonical_nodes` validates each row, and `Batch::with_reservation` validates it again.
- **Product cache hit** (seam with C): `decode_product_rows` calls `hash_rows`, which computes `content_digest` per row into a `RelationContent` that is never finished or compared. Only the ordering check is used.
- **MCP double envelope admission** (`EnvelopeAdmission._admit` and `BoundedStdioWriter.send`): adequate. The two guards buy different responses (a domain refusal versus closing the transport), and the writer checks the actual bytes.
- **Fixture binary digest** (F2): `_installed_binary` and `verify_binary` hash the 368.7 MB executable 4–6 times per boundary.
  - P1 measured 0.155–0.161 s per pass.
  - These repeat a conclusion within one invocation (H20). Memoise per invocation by path and stat identity.
- **SourceObserver** (F2): hashes the whole allowed tree once per invocation. Git's index stat cache could supply blob ids for clean tracked files. Its disclaimer that it does not establish the source-to-binary link is honest and should be kept.
- **Cold audit:** `verify_state_selected` issues one point query per membership row (O(rows) round trips). This is a rare operator path.
- **ERA_GUARD** writes `native_installation:current` on every effect, so effects conflict with each other and not only with era closure (H23). The `FOR UPDATE` semantics at 3.3.0 are unsettled here (safeguard 1); this is for D or L1.
- **`ContentHash` carries both SHA-256 and BLAKE3 values** (`retrieval_tokenizer`, `embedding::value::input_hash`) without the algorithm in its type. A mismatch would refuse rather than accept, so the risk is low.
- **Cold reconciliation compares serialised JSON** (`serde_json::to_vec(actual) != …`), which expands `canonical` `Bytes` into integer arrays. On the retained cold paths, compare the canonical bytes directly.

## 5. Strengths, preservation constraints and adequate areas

- **Identity framing.** `KeySink` uses a tagged, length-framed, versioned encoding with a pinned framing test. `Id` (16-byte nominal) is kept distinct from `ContentHash`, and counting and recording reuse the one encoder. Keep this.
- **Addresses are not equality.** `ensure_rows_owned` compares full values under the guard ("content hashes are addresses, not substitutes for collision checking"). The scope interner uses xxh3 only for buckets and compares full bytes. Both match the graph-hashing review's guidance. No fast hash is used as an identity.
- **Admission is separated from producer replay.** `InvariantPurpose::DiagnosticReplay` checks are excluded from admission (`model.rs::select_checks_for_scope`). This is exactly the H20/CI-12 distinction.
- **Premise memoisation within a workspace.** The invariant memo (`validation_premise`) and the reference memo (`compiler-reference-premise/v1`) are sound H19 carries. The second `admit_semantics` is therefore incremental, not a full repeat.
- **Carried validity on reads.** `Batch::read` shares Arrow buffers for canonical stored batches without re-encoding. Import skips model reconstruction after its own validation. Both are templates for G-03 and G-04.
- **Independent checks at real trust boundaries** (keep):
  - `verify_export`, `verify_restored` and `inspection::audit`;
  - portable product-cache decode;
  - RECORD SHA-256;
  - the per-call query-vector input hash.
- **Publication readback** goes through `NativeReader::connect`, a different consumer path, at a fixed cost (CI-13).
- **The serving definition check** runs once per session (`OnceCell`).
- **Membership-completion readback** derives view identity from durable, server-sorted state. That is the right physical choice: not a repeat, and no sorting of the live set in-process.
- **Logging is otherwise coarse.** `native_batch_write` emits per batch, bounded by rows/4096.

## 6. Positions on earlier leads

| Lead | Position | Evidence |
|---|---|---|
| NE F02 (support admission repeats preparation per grain) | **Residual** | `scoped_admission::validate_support` lowers the program once, and `validate_group` hydrates the union once per root group. `validate_grain` still creates a check and plans a SQL stream per grain. `OwnershipRows::Full` scans the full spilled run per grain (`ownership_rows`). |
| HS-F06 (cold audit repeats capture) | **Resolved** for capture; **residual** for validation | `audit_owned` calls `capture_audit` once. `verify_captured` (`verify_state_selected`) and `completed_state_captured` (`state_identity_selected(true)`) still both validate backing. |
| PJ-plan F01 (admission must certify the post-freeze state) | **Confirmed implemented** | `artifact::admit` freezes before `admit_semantics`. This explains the second admission call; the memo bounds it. |
| Persisted-graph-impl F01 (cold backing disagreement) | **Resolved** (correctness); **moved** (cost) | `validate_compiler_backing` exists; the same cold validation now also runs on the ordinary path (G-04). |
| GK-hashing review and `docs/graph_hashing_reference` | Hashing guidance **confirmed conformant**; F01/F02 kernels **not examined** (B/C) | `intern_with`, `ensure_rows_owned`. |
| Op-map M1 (effect fingerprint ×2) | **Confirmed**, and it has no production consumer | G-01. |
| Op-map M2 (per-row JSON plus hash) | **Confirmed** (canonical JSON is necessary; the duplicate hash and JSON round trip are not) | G-03. |
| Op-map: double `admit_semantics` | **Refuted** as a full repeat | Premise memo. |
| Op-map §1.6–1.7 (full scans, reconciliation, second lowering) | **Confirmed** | G-04, G-05. |
| Op-map §3.2 (serving re-validation per call) | **Confirmed** for spec and projection reads; the definition check runs once | §4. |
| Op-map §4.3 (tooling digests) | **Confirmed** in code; pass count inferred | §4 (F2). |

## 7. Candidate rule impacts

- **RC-1.** `docs/design/sections/storage-and-publication.md` says: "Current provenance, exact dependencies and independent cold admission remain mandatory" for attached retained content (ADR-0145).
  - **Proposed change:** cold admission may be satisfied by a durable checked-premise record keyed by exact views and implementation digests.
  - **Depends on it:** G-06.
  - **If the rule is kept:** G-06 is withdrawn, and the in-workspace memo remains the only carry.
- **RC-2.** `storage-and-publication.md`, the physical-lowering paragraph ("Complete independent expected-row reconstruction rejects … coherent corruption"), together with `synthesis-and-serving.md` ("Publication reconstructs and reconciles these derived statistics").
  - **Proposed change:** full independent reconstruction applies at cold exposure (import, restore, audit). Same-process publication reconciles derived search once after writing and carries admission's identities to seal.
  - **Depends on it:** G-05.
  - **If the rule is kept:** G-05 narrows to removing the second `lower`, the second `completed_state` and the duplicate original rehash, and keeps one `Loader::reconcile`.

## 8. Proposed light probes

1. **Attribute the manifest and seal cost.**
   - **Command:** for each log returned by `rg -l 'phase="artifact_manifest"' build/runs -g '*.log'`, extract the `elapsed_ms` of `artifact_manifest`, `publication_seal` and `publication_final_reconciliation` (the shell loop used in §3, G-04).
   - **Discriminating outcome:** if `artifact_manifest` tracks `publication_seal − publication_final_reconciliation` across runs, a shared `completed_state(true)` cost is likely dominant (supports G-04 priority). If not, hold transfer or the outcome inventory dominates.
2. **Check the observation ratio at library scale.**
   - **Command:** `rg -c 'phase="borrowed_cpu"' <log>` against `rg -c lctx_phase <log>`, run on any existing `lctx compile fastmcp` log.
   - **Discriminating outcome:** a ratio near 0.9 at real-library scale confirms G-07's magnitude.
3. **Confirm the G-01 absence claim.**
   - **Command:** `rg -n '__request|\.request\b|"request"' crates python scripts -g '!**/tests/**'`.
   - **Discriminating outcome:** any reader that compares the stored `native_effect.request` with an independently computed digest refutes G-01.
4. **Not light; needs a build, so schedule later.** Time `serde_json::to_vec(&(sql, &bindings))` plus BLAKE3 for a 128-row `original_chunk` window, against that window's gRPC write latency. This would size G-01 and G-02.

The coordinator should hand G-02 and the ERA_GUARD question to D/L1, G-08 to A, and the fixture and source-observer items to F2.
