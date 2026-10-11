# Inquiry A: Acquisition, providers and producer streaming

**Provenance.** This is the verbatim final report of supporting inquiry A, from one read-only
`design-reviewer` agent, for the [codebase efficiency-alignment review](../../reviews/design_review_efficiency-alignment-codebase_2026-10-11.md).
It was produced on 2026-10-11 at baseline `88f33bc0`, against the [inquiry briefs](inquiry-briefs.md).
Only this header was added.

How to read it:
- Candidate labels such as `A-01` are local to this report. They are not the review's finding
  IDs.
- Its judgments are bounded and supporting. The principal review owns the combined decision.
- See the [evidence index](README.md) for coordinator checks and reconciliation.

---

# Inquiry A: acquisition, providers and producer streaming

Supporting assessment for the codebase efficiency-alignment review (DESIGN/TARGET tier). It is read-only and static, done on 2026-10-11 at baseline `main` `88f33bc0`. Candidate labels are local (`A-nn`). The principal reviewer owns finding IDs and verdicts.

## 1. Scope examined and coverage limits

**Read in full or at the decisive grain:**
- `crates/cpg-extract/src/capture.rs` (`CapturedInput::{capture_with, emit_chunks, verify, with_receipts}`, `copy_digest`, `regular_file`).
- `crates/cpg-extract/src/acquisition.rs`: `library_inventory`, `capture_receipts`, `AcquiredInput::uses`, `Acquire::run` and `contract`.
- `crates/cpg-extract/src/contracts.rs`.
- In `crates/cpg-extract/src/bundle.rs`: `run_provider`, `StageContext::{declare, emit, contribute}` and `build_digest`.
- In `crates/cpg-extract/src/pyrefly_stage.rs`: `session` through the canonical parse, plus `roots` and `contract`.
- `crates/cpg-extract/src/ruff_context.rs::CanonicalSyntax::parse`.
- In `crates/cpg-extract/src/ty_flow.rs`: `contract` and the source read loop.
- The contracts for `crates/cpg-extract/src/{document_parser,deployment,assembly}.rs`, and `deployment.rs`'s uses lookups.
- `crates/cpg-flow/src/lib.rs::{rename, module}`.
- `crates/cpg-core/src/facts.rs::compile_facts`.
- `crates/cpg-core/src/native_bridge.rs` and `native_calls.rs`.
- In `crates/cpg-core/src/workspace.rs`: `Writer`, `owned_batch_write`, `submit_batch_write`, `ProducerOutput::{write, push, push_sync, declare_kind, registration_request, complete}` and `producer`.
- `lctx_model::domain::record::Batch`, `batching::BatchWriter`, `model::canonical`, `artifact::{ArtifactCapture, ArtifactVerifier}`, `source::SourceArtifact` and `input::InputRevision`.
- In `crates/lctx-surrealdb/src/compiler.rs`: `write_batch_inner`, `ensure_physical_rows`, `persist_original_chunk`, `ensure_immutable_rows`, `complete_contribution_inner`, `registered_or_install_views` and `bind_inner`.
- `control.rs::{ensure_rows_owned, hold_many}`, `control/authorization.rs::{effect_for_owner, execute_owner, ERA_GUARD}`, `loader.rs::NativeWindows` and `codec::batch_bodies`.
- `reconciliation.rs`, the originals readback only.
- `crates/lctx/src/acquisition.rs::Lease` and `crates/lctx/src/compile.rs::compile`.
- `scripts/storage_acquisition.py::{Lease.prepare, prune_finished_leases, configuration, check_source, sync}`.
- The pinned `surrealdb-types-3.3.0/src/value/bytes.rs` (`Serialize` uses `serialize_bytes`).

**Not read, or only skimmed:**
- `type_records.rs`, `syntax_records.rs`, `call_records.rs` and the remainder of `pyrefly_stage.rs` (row-emission details);
- most of `document_parser.rs` and `deployment.rs`;
- `Schedule::build` ordering logic beyond its writer/epoch checks;
- `prepare_memberships` and `attach_retained_product` internals;
- `Lease.close`/release in Python.

No library skill was consulted beyond the pinned SurrealDB types source. Pyrefly and ty incremental semantics are left to L2.

**Light measurements I took myself** (read-only `find`/`du`, 2026-10-11):
- In the existing fastmcp environment `build/envs/fastmcp` (a legacy, pre-generation environment dated 2026-09-22): 8,173 analyzer-closure files totalling 58.3 MB, 238 of them empty and none over 1 MiB.
- The fastmcp source tree at commit `004bf15a`: 858 Python files and 669 Markdown/MDX files.
- No acquisition owner state exists under `~/.local/state/library-context-storage/owners`. No native compile has run on this host with the current helper, so there is no real-library receipt for this path.

## 2. Complete operations examined

**Acquisition (S01 and S03):**
1. `lctx::compile::compile` calls `Lease::open`. That spawns `uv run … python scripts/storage_acquisition.py` (two processes), sends a JSON request on stdin and reads one reply line.
2. `Lease.prepare` then:
   - takes ancestor admission flocks;
   - computes a SHA-256 over `pyproject`/`uv.lock`/`.python-version`;
   - takes the owner lock;
   - globs and reads every lease file in `prune_finished_leases`;
   - reuses the environment group when the key matches (no `uv sync`);
   - runs `git rev-parse HEAD` for the source;
   - always writes `durable_json(record.json)` and the lease receipt.
3. `cpg_extract::acquisition::inventory` reads every RECORD (103 distributions) and walks site-packages with stat only.
4. `capture_receipts` runs the reads listed below.
5. `Lease::finish` releases the helper immediately after capture. The lease covers only the copy window.

Reads of the captured closure per catalog compile:

| # | Owner | What it establishes |
|---|---|---|
| R1 | `CapturedInput::capture_with` → `copy_digest` | Copy to a private 0700 tempdir with a blake3 digest, two stats per file and a final re-stat sweep: content identity and isolation from the live tree |
| R2 | `acquisition::capture_receipts` SHA-256 loop | Frozen environment bytes match RECORD |
| R2′ | `capture_with` derivation loop | `fs::read` of each frozen document plus `python_blocks` parse |
| R3 | `facts::compile_facts` | `input.captured().verify()` over every input |
| R4 | `Acquire::run` → `emit_chunks` | Emission; `ArtifactCapture::finish` checks the digest inline |
| R5 | `Acquire::run` | `frozen.verify()` |
| R6 | `Loader::reconcile` at seal (D's area) | Store readback of every original |

**Facts providers (catalog):**
- `compile_facts` awaits `Schedule` stages one at a time.
- Each provider runs on its own 512 MiB-stack thread (`bundle::run_provider`).
- The declared data dependencies are:
  - Acquisition: none;
  - Pyrefly: none;
  - Assembly: none;
  - Deployment: depends on Acquisition outputs;
  - Documents: depends on Pyrefly outputs;
  - TyFlow (behavioral only): depends on Pyrefly outputs.

Parses per first-party root module:
- **Catalog:** Pyrefly's embedded Ruff 0.0.14, plus latest Ruff 0.0.16 (`CanonicalSyntax::parse`, over Pyrefly's own buffer, no extra disk read).
- **Behavioral adds:** ty's `rename` (`ruff_python_parser_ty` 0.0.16) and ty's salsa parse of the renamed text.

Disk reads per root:
- `pyrefly_stage::session` reads every root for a UTF-8 check, then Pyrefly loads it.
- `ty_flow` reads and hashes it again under behavioral.

**Row path:**
- **Provider side:** `StageContext::emit` → `BatchWriter::push` (4,096 rows or 8 MiB, plus the `seen` index) → `Batch::with_reservation` (validate, `sort_unstable_by_key(Record::id)`, dedup, `R::encode`) → `ProducerOutput::write` → `Writer::write_batch` → `NativeBridge::call_boxed` (blocks the provider thread) → `NativeCalls::call_boxed` (`Semaphore(8)`, `tokio::spawn`) → `NativeCompilerStore::write_batch_inner`.
- **Store side:** `relation.canonical` (decode, sort, encode), then `codec::batch_bodies` (Arrow → `Value`), then a canonical JSON hash per row, then the `nodes` and `members` maps. Per 128-row window (`NativeWindows`) it issues:
  - `ensure_rows_owned(anchors)`;
  - `ensure_rows_owned(rows)`;
  - `hold`;
  - `hold_many`;
  - for membership, `ensure_rows_owned` and `hold`.
- Each of these is one M1 effect: 4 round trips, 3 writes to `native_installation:current`, and two `serde_json` fingerprints of `(sql, bindings)`.
- Live per in-flight batch: `Vec<R>` plus the Arrow batch (`Batch`), a decoded `Vec<R>` and re-encoded batch (`canonical`), the `Vec<Value>` bodies, `GraphRow`s, and the `nodes`/`members` maps with canonical JSON bytes. Because writes are synchronous, at most one batch per provider is in flight during facts.

**Completion:**
- `ProducerOutput::complete` closes the writers.
- It pads the outputs with every allowed contributed relation.
- `complete_contribution_inner` streams all of the contribution's membership rows (`ORDER BY relation, semantic_key`) into `KeySink`s, then marks the contribution complete (one effect).
- `registered_or_install_views` runs `prepare_memberships` per view and writes `compiler_view_member` rows plus a `hold` per 128 rows.
- `completed_contribution` reads the descriptor back.
- `native.bind` runs once per completed relation: `guarded_effect` plus `hold`, about 8 round trips.

## 3. Candidate findings

### A-01: Original bytes are persisted one artifact chunk at a time (12 round trips per file)

**Observed cause.**
- `NativeCompilerStore::write_batch_inner` loops `for row in &original_chunks { self.persist_original_chunk(row).await? }`.
- `persist_original_chunk` splits one `ArtifactChunk` (≤1 MiB) into 64 KiB slices and calls `ensure_immutable_rows("original_chunk", …)`.
- That helper windows only this one chunk's slices, so the 128-row window never fills. Each chunk still costs `ensure_rows_owned`, `hold` and `hold_many`: three effects of 4 round trips each.
- Every closure file under 1 MiB is exactly one chunk. On the measured environment that is about 7,935 non-empty files, plus up to about 1,500 corpus files and every derived doc block.
- That works out to roughly 110–120k sequential round trips and about 85k serialized updates to the `native_installation:current` row, for original bytes alone. This is derived from the code and the file counts, not measured.
- The operation map's M2 table ("12 per window") understates this: the real unit is per chunk, not per window.

**Consequence.** S01: probably the largest round-trip term in the acquisition provider. S03: repeated in full on every unchanged recompile, because the rows exist but the full payload is still sent and compared server-side (`$existing != $row` in `ensure_rows_owned`), and new holds are written. Mechanisms: H8 (bulk intent hidden behind per-item calls), H2, DP-10, FP-07. It is the same per-occurrence shape as the catalog-compilation-speed F05 lead, recurring in a new owner.

**Simpler alternative.** Gather the `original_chunk` slice rows of the whole batch (and their `source → chunk` references) into one row vector, then pass it through `NativeWindows` once. That is 3 effects per 128 slices instead of per file.

**Preservation.** Keep:
- the immutable-collision check;
- the fence that refuses canonical content once an attempt is admitted;
- per-row guard and epoch checks;
- the 8 MiB transfer cap;
- the `original` header and chunk ownership holds;
- validation of every row before any effect.

**Uncertainty.** The magnitude depends on per-round-trip latency, which is unmeasured. The current generation's file count may differ from the legacy environment.

**Evidence.** Diagnosis Implemented (static, 2026-10-11). Remedy Proposed.

**Closure.** Inspection showing one windowed pass per batch, plus a focused control that counts effects (or instruments `effect_for_owner` calls) for a capture of N small files.

### A-02: Each contributing provider's completion re-materializes and re-binds the full union view of every shared vocabulary relation

**Observed cause.**
- `Workspace::producer` sets `allowed = outputs ∪ contributes`.
- `ProducerOutput::complete` pads its pending outputs with every allowed relation, even one with zero rows written.
- `complete_contribution_inner` builds, for every output, `CompletedView::new(relation, previous.contributions ∪ {logical}, count)`. The view identity hashes the contribution set (`completed.rs::CompletedView::new`), so every contributor creates a new view for each of the 28 `assembly::vocabulary()` relations.
- `registered_or_install_views` then streams and sorts the whole union (`prepare_memberships`) and writes a `compiler_view_member` row plus a hold for every member.
- `complete` also calls `native.bind` (about 8 round trips) per relation.
- Pyrefly contributes `Evidence::Occurrence` for every occurrence, plus `SyntaxDetail`, `Literal` and others. So each later contributor (Documents, which depends on Pyrefly, and possibly Deployment and Assembly; TyFlow too under behavioral) rewrites all of Pyrefly's vocabulary membership again.
- No facts provider consumes a vocabulary relation as input (contracts checked for Documents, Deployment and TyFlow), and normalization contracts declare `contributes: vec![]`. The intermediate union views therefore have no consumer before `freeze_completed_inputs`.

**Consequence.**
- S01: vocabulary membership is materialized about (1 + number of later contributors) times instead of once.
- S02 and S09: adding a provider, or more semantic kinds, multiplies the whole vocabulary.

Mechanisms: H6 (preparation without demand), H13, H2, DP-10.

**Simpler alternative.**
- Do not form a new view for a relation the contribution wrote no rows to.
- More generally, register contributions to epoch-shared relations at completion, and install and bind the union view once, at the consuming boundary (`freeze_inputs_async` already binds epoch-shared relations once per epoch).
- An alternative physical form is to resolve union membership through the contribution-level membership index instead of per-view member rows.

**Preservation.**
- Exact contribution sets and counts in the final view.
- Conflict detection across contributors.
- Completion atomicity for each producer's own outputs.
- `completion_gate` ordering.
- Views remain identical to the ones a cold run would produce.

**Counterexample.** A future in-facts consumer of a vocabulary relation would need its union materialized at its own boundary. A demand-driven install handles that case.

**Uncertainty.** Vocabulary row counts for fastmcp are unknown. The exact schedule position of Assembly and Deployment was not traced. Documents is always after Pyrefly.

**Evidence.** Implemented (static). Remedy Proposed.

**Closure.** A trace of the facts phase showing each vocabulary relation installed once. Equivalence of the final views and their counts against the current route.

### A-03: The physical unit of a producer write is a 128-row window split into 4–6 separately authorized effects

**Observed cause.**
- `ensure_physical_rows` issues per window, in sequence: `ensure_rows_owned(anchors)`, `ensure_rows_owned(rows)`, `hold(ids)` and `hold_many(refs)`.
- `ensure_immutable_rows` then adds `ensure_rows_owned` and `hold` for `compiler_membership`, plus the same pair for any aliases.
- Each effect is a complete M1 protocol: an owner read, an intent transaction, a payload transaction and an outcome-reference release, each transaction updating `native_installation:current` (`authorization.rs::ERA_GUARD`).
- A 4,096-row batch is therefore about 32 × 24 ≈ 770 round trips. Derived, not measured.

**Consequence.**
- S01: protocol overhead dominates payload for small rows.
- S02: grows linearly with rows, times 6.
- The separate effects also leave rows briefly present without their holds, protected only by guard epochs.

Mechanisms: H2, H8, H23.

**Alternative.** One effect per window (or per byte-bounded batch) whose payload does anchors, rows, holds, references and membership together. This reduces protocol round trips about sixfold, and atomic visibility then matches the logical write.

**Preservation.**
- Per-row guard, retiring and epoch checks.
- Idempotent identical replay.
- Collision refusal.
- Contribution fencing.
- Bounded transaction size.

**Seam.** D owns M1 itself (whether the effect protocol needs 4 round trips and 3 singleton writes). This candidate concerns only how producers compose it.

**Uncertainty.** The transaction size and conflict behavior of a combined payload on RocksDB at 3.3.0 is not qualified. L1 should settle it.

**Evidence.** Implemented. Remedy Proposed.

**Closure.** Effect-count inspection per window, plus the existing immutable-collision and fencing controls.

### A-04: Facts providers never reuse; an unchanged warm recompile re-runs all extraction and re-sends every row and original byte

**Observed cause.**
- `facts::compile_facts` builds a `ProducerOutput` and runs each provider unconditionally.
- No product request, `attach_retained_product` or `retain_product` call exists in `facts.rs`, `bundle.rs`, `pyrefly_stage.rs` or `ty_flow.rs`. Search: the `rg` the operation map reports, plus my reading of `compile_facts`; tests not searched.
- Each attempt gets a fresh contribution, so all M2 writes, completion and A-02 repeat.
- §14.10's invalidation table makes extraction invalidation conditional on "Source/environment/provider meaning" changing.

**Key completeness check (DP-09).** Every result-affecting input of a facts producer is known before it runs:
- the captured input revisions: `InputRevision.manifest` digests every closure byte;
- analysis context and configuration digests (`contracts::captured_sources`);
- the provider build digest. `bundle::build_digest` covers the producer source-closure digest, `Cargo.lock` and both fork patches;
- the model digest;
- the profile;
- the contract semantic revision.

`refuse_ambient` and explicit configurations (§B8) remove ambient inputs.

**Consequence.**
- S03 unchanged: Pyrefly analysis, all emission, all writes (including A-01) and completion repeat.
- S03 patch upgrade: any byte change produces a new installed `InputRevision`. Because one contribution spans both inputs, the corpus session reruns too, and the reverse holds for a docs-only edit.

Mechanisms: H13, H12, DP-09/DP-10.

**Alternative.**
- Extend the existing `ProductRequest`/`attach_retained_product` route (ADR-0145 checked attachment) to facts producers, keyed on the captured binding plus the identities above.
- Refinement: one contribution per (provider, input), so a corpus-only change keeps the installed-input analysis.
- Further refinement: on a full facts hit, a hash-only capture pass can replace the frozen copy, because no provider reads the tree and admission uses metadata only (`artifact::admit` reads `artifacts()` only).

**H14 check.** Proving the inputs unchanged costs one content pass over about 73 MB, plus key construction. That is small relative to analysis.

**Seam.** The hit still copies membership in O(rows) (`retain_attached_members`). C and D own attach-by-reference.

**Preservation.**
- Admitted-attempt provenance.
- Current attribution: run and provider rows stay exact.
- Facts produced under nondeterministic thread scheduling must be substitutable.
- Independent cold admission is unchanged.

**Uncertainty.** Whether Pyrefly's AllThreads output is byte-deterministic for equal inputs. L2 should confirm, and a replay control should test it.

**Evidence.** Implemented (gap). Remedy Proposed.

**Closure.** An unchanged recompile trace showing no provider execution, plus equivalence against a clean run. Changed-byte, changed-configuration and changed-build cases must miss.

### A-05: Provider writes are acknowledged synchronously per batch, so extraction and persistence never overlap

**Observed cause.**
- `Writer::write_batch` → `bridge.call_boxed(submit_batch_write(…))`, which is `futures::executor::block_on(call_boxed_async)` awaiting the write's terminal result.
- `ProducerOutput::write_inner` holds the `writers` mutex across it.
- Effective concurrency on the facts path is therefore one batch. The `mpsc(2)` channel and `Semaphore(8)` never bind.
- The bridge's own documentation ("submit writes while that stream remains open") describes the intended owned-task model.

**Consequence.** S01: provider CPU (emission, Arrow encode) idles while each batch's hundreds of round trips complete, and vice versa. H22, H21.

**Alternative.** A bounded write-behind per writer: submit and keep K acknowledgements outstanding, charged to the budget. `ProducerOutput::complete` already drains the producing scope (`close_and_wait`) before completion, and `CallGuard` already fails the attempt on a late error.

**Preservation.**
- A failed write must still fail the contribution before completion.
- Cancellation drains submitted work.
- Memory stays charged.
- Coordinate with the singleton-row contention in A-03/M1. Concurrent effects conflict on `native_installation:current`, so the overlap benefit is bounded by D's protocol.

**Evidence.** Implemented. Remedy Proposed. No speed claim.

**Closure.** A late-error control, a cancel-while-in-flight control, and a budget-charge check.

### A-06: Facts providers are awaited sequentially although the schedule orders only two dependencies

**Observed cause.** `compile_facts` runs `for declaration in schedule.stages() { … run_provider(…).await; complete().await }`. Declared inputs order only Deployment after Acquisition and Documents/TyFlow after Pyrefly. Acquisition (I/O and write bound) and Pyrefly (CPU bound in `transaction.run` on the AllThreads pool) are independent.

**Consequence.** S01: Acquisition's write phase and Pyrefly's analysis add up instead of overlapping. H22.

**Alternative.** Run ready stages concurrently, with completions serialized by the existing `completion_gate`.

**Preservation.**
- Exact input binding at start for dependent stages.
- Deterministic union views (A-02).
- Budget admission for concurrent providers. Pyrefly's state and Acquisition's buffers then coexist (H17).

**Uncertainty.** If A-01 and A-03 are fixed, Acquisition's share may become small enough that this is not worth the extra live-set and coordination. Assess after them.

**Evidence.** Implemented. Remedy Proposed.

### A-07: Original bytes are stored per revision-scoped artifact identity, with no content sharing across revisions or libraries

**Observed cause.**
- `SourceArtifact`'s key is `(input: Id<InputRevision>, path, content)`, where `InputRevision.manifest` digests the whole closure.
- `original` and `original_chunk` IDs derive from `EntityId::of(SourceArtifact id)` (`write_batch_inner`, `persist_original_chunk`).

**Consequence.**
- S03 patch release: one changed file re-keys every artifact, so all ~58 MB of environment bytes are re-stored as new rows, with new holds and a new seal readback.
- S02 and S08: shared dependencies such as pydantic, httpx and typing-extensions are stored once per library per retained revision.

Mechanisms: H15, H16, H27.

**Alternative.** Keep the semantic `SourceArtifact` and `original` header per artifact-in-revision. Store chunk bytes by content (`original_blob:<content>`, chunked), referenced from the header.

**Preservation.**
- Evidence byte retrieval by artifact.
- Independent seal readback against the manifest's `(content, byte_len)`.
- Retirement only when no live holder remains (existing `native_hold` guards; calibration safeguard 5).

**Rule impact.** Candidate RC-A2.

**Evidence.** Implemented. Remedy Proposed.

**Closure.** A patch-upgrade trace writing only changed blobs, plus a retirement control with a shared blob.

### A-08: The store re-proves a canonical form the `Batch<R>` type already guarantees

**Observed cause.**
- `Batch::with_reservation` validates, sorts by id, dedups and encodes from typed rows.
- `owned_batch_write` passes `batch.arrow()` as an erased `&RecordBatch`.
- `write_batch_inner` → `relation.canonical` (`model.rs::canonical`) does `R::decode`, then `sort_by_key(Record::id)` (recomputing blake3 ids per comparison), then a duplicate check, then `R::encode`.
- The only production caller is `owned_batch_write`; the other callers are tests.
- Rows are also validated twice: `push_sync` or `BatchWriter::admit`, then `with_reservation`.

**Consequence.** One full decode-sort-encode per batch on every write (S01/S02/S03), and two extra live representations (H17). H19, H16.

**Alternative.** Carry the property in the type: a `CanonicalBatch` constructible only by `Batch`. Alternatively, have the store check strict id ordering on column 0 in O(n) without decoding. In both, use `sort_by_cached_key` for the remaining sorts.

**Counterexample.** An external or untyped caller must still be canonicalized. Keep the decoding path for that entry point.

**Evidence.** Implemented. Remedy Proposed.

### A-09: Whole-closure verification passes are placed where no uncertainty enters

**Observed cause.**
- R3 (`compile_facts`) re-verifies a `CapturedInput` that can only be built by `capture_with`, which digested while copying, seconds earlier in the same process, into a private, read-only 0700 tempdir.
- R5 (`Acquire::run`'s `frozen.verify()`) repeats the digest check that R4's `ArtifactCapture::finish` completed moments before.
- R2's SHA-256 is a distinct property (RECORD integrity) over the same bytes R1 already streamed.
- Point-of-use checks already guard the analyzed text: `CanonicalSyntax::parse` compares `ContentHash::of(source)` with `artifact.content`, and `ty_flow` does the same.
- `CapturedInput::verify`'s documented placement ("after a provider finishes, before its rows can publish") matches neither call site. No verify follows Pyrefly or ty.

**Consequence.**
- Two to three redundant full passes, each with about 10k opens and per-path-component `symlink_metadata` calls (`regular_file`).
- The P1 probe measured SHA-256 at about 2.3 GB/s on this host, so the byte cost at pilot scale is small, about tens of milliseconds per pass.
- Priority is low. The defect is mostly assurance placement (H18, H19, H20).

**Alternative.**
- Compute SHA-256 inside `copy_digest` for RECORD-listed files.
- Drop R3 and R5.
- If protection of Pyrefly's non-root dependency reads against same-user tampering is wanted, run one `verify()` after the last provider that reads the frozen tree.

**Preservation.**
- The RECORD check itself.
- Refusal of tampered frozen content (`tests/capture.rs` exercises `verify()` directly; keep it).
- Per-use digest checks.

**Evidence.** Implemented. Remedy Proposed. Byte cost Measured only by analogy (P1).

### A-10: Quadratic artifact-to-use lookups in provider loops (S02)

**Observed cause.**
- `AcquiredInput::uses` runs, for each artifact, `library.files.iter().filter(|f| f.path == artifact.path)`. That is about 67M path comparisons at 8.2k files, and it is called from `Acquire::run`, `pyrefly_stage::roots`, `document_parser`, and `deployment` (twice).
- `deployment.rs` runs, for each artifact, `uses.iter().find(|u| u.artifact == artifact.id())`.
- `pyrefly_stage::session` runs, for each root, `analyzed.iter().position(|a| a.id() == artifact.id())`, recomputing blake3 ids.

**Consequence.** At 5–20× closure size (S02, heavy dependency context) this grows quadratically, to roughly 1.6–25 billion comparisons per call. H9.

**Alternative.** Build a path or id map once per input (as `admission::analysis_roots` already does with `by_id`), and memoize `uses()`.

**Evidence.** Implemented. Remedy Proposed. Magnitude unmeasured.

### A-11: The provider-side cross-batch dedup index grows with total rows and duplicates store enforcement

**Observed cause.**
- `BatchWriter.seen: HashMap<Id<R>, ContentHash>` lives for the whole provider run, per relation, charging about 130 bytes per distinct row.
- The store already makes identical replay idempotent and refuses same-key/different-payload: membership IDs derive from `(contribution, relation, key)`, and `ensure_rows_owned` throws on `$existing != $row`.

**Consequence.** The live set grows with library size (S02; H17). It is a second mechanism for the same invariant (H4, H20).

**Alternative.** Keep cross-batch dedup only for small vocabulary relations, or rely on per-batch dedup plus the store.

**Preservation.** A typed `Conflict` classification and early, provider-local failure. A store-side throw currently surfaces as a codec error, so the classification must be mapped.

**Evidence.** Implemented. Remedy Proposed.

## 4. Opportunities and uncertainties that are not findings

- **Parse count is mostly a §B1 necessity.** Pyrefly's embedded Ruff (registry 0.0.14) and latest Ruff (git 0.0.16) are different parsers (checked in `Cargo.lock`). Under behavioral, ty's `rename` uses the same 0.0.16 parser as latest Ruff over identical text, so its tokenization duplicates the canonical parse. Sharing it would cross provider-local parsing (§B1), so this is not a finding.
- **One Pyrefly `State` per captured input.** The corpus session re-solves library modules as dependencies. The corpus contains `src/fastmcp`, so module resolution can genuinely differ between contexts. Whether one multi-config `State` can preserve that is for L2.
- **Documents are parsed twice.** Once for block derivation at capture (`derive_blocks`), once by the Documents provider. This is small and ordered by necessity: blocks must exist before Pyrefly.
- **Pyrefly's UTF-8 admission read.** It re-reads each root before Pyrefly loads it. A UTF-8 flag could be recorded during R1. Minor.
- **Effect fingerprints over byte payloads** (G and D seam). `effect_for_owner` and `execute_owner` each `serde_json` the `(sql, bindings)` pair. `surrealdb::types::Bytes` serializes via `serialize_bytes`, which `serde_json` writes as an integer array (about 3.5× expansion). This happens twice per original-chunk effect.
- **Seal readback of originals** (D seam). `Loader::reconcile` issues a header read and at least two paged chunk queries per original, the last returning empty. That is about 30k round trips and a sixth full read of the closure. The check is independent and worth keeping (safeguard 7). The terminating empty page could be avoided by stopping at `byte_len`.
- **Peak memory of a compile is unresolved.** The likely dominant terms are Pyrefly's `State`, the `seen` indexes (A-11) and up to (relation count × 8 MiB) of pending writer buffers. All are charged to a 64 GiB default budget. Measuring this needs an authorized pilot (`/usr/bin/time -v`), not a light probe.
- **Fidelity note, not efficiency** (for G or the principal). `check_source` verifies only `git rev-parse HEAD`, not a clean worktree. `InputOrigin::Corpus{revision: commit}` can therefore attribute locally modified bytes to the commit. Content identity stays exact via `InputRevision`.

## 5. Strengths, preservation constraints and adequate areas

- **Frozen capture decouples analysis from the live inputs.** The helper lease is released right after the copy (`compile.rs`, `Lease::finish`). This is H23 done well: exclusion is scoped to the copy window.
- **Point-of-use content checks** (`CanonicalSyntax::parse`, `ty_flow`) are correctly placed (H18). Keep them.
- **`ArtifactCapture`/`ArtifactVerifier`** give one bounded-memory content proof shared by capture and sealing. Keep this single authority.
- **Provider build identity is complete and conservative** (`bundle::build_digest`: source closure, `Cargo.lock`, patches). It is the right base for A-04.
- **The completion readback of persisted membership** (`complete_contribution_inner`) is an independent store readback, done once per contribution and proportional to output size. Adequate.
- **The bridge is wake-driven** (tokio `mpsc` plus `oneshot`; the sleeps are test-only). The native-execution-efficiency F05 polling is gone.
- **The acquisition helper's fixed costs** (uv/Python start-up, a lease glob bounded by `action_receipt_days`, about five durable JSON writes) are proportionate to a compile. Adequate.
- **The per-batch live set** is bounded by `TransferLimits`, and it does not grow with library size. A-11 and pending-writer breadth are the exceptions.

## 6. Positions on earlier leads

| Lead | Position | Evidence |
|---|---|---|
| catalog-compilation-speed F05 (one native crossing per external occurrence) | Resolved for its owner; the same shape now recurs in A-01 | `loader.rs` collects `pending.endpoints` and calls `ensure("external", …)` per window. `persist_original_chunk` repeats the per-item pattern in the producer. |
| native-execution-efficiency F01 | Not examined | Read-path membership (B/D) |
| native-execution-efficiency F02 | Not examined | Support admission (B/G) |
| native-execution-efficiency F03, F04 | Not examined | Product reuse (C) |
| native-execution-efficiency F05 (fixed-interval polling) | Resolved; residual moved to A-05 | `native_bridge.rs` is wake-driven; sleeps occur only in `#[cfg(test)]`. An already-registered `declare_kind` still crosses the bridge with a ready future (negligible). Synchronous per-batch acknowledgement remains (A-05). |
| persisted-execution-correction-causes F04 | Not examined | Selected-stream preparation (B/D) |
| unified-persistent-surrealdb F04 (reuse is value replay) | Residual | `attach_retained_product` now attaches without payload replay but copies membership in O(rows). Facts producers have no reuse at all (A-04). |

## 7. Candidate rule impacts

- **RC-A1 (A-04).** Rule: GR plan §4 / ADR-0145, "checked attachment", which is applied today only to normalization and upper stages (`ProducerOutput::reuse_product*`). Proposed change: extend attachment to facts producers keyed on the captured binding plus the provider build digest. If the rule is kept, warm S03 still reruns extraction, and only A-01, A-03 and A-05 mitigate it.
- **RC-A2 (A-07).** Rule: the `storage-and-publication.md` physical lowering of originals (`original` and `original_chunk` keyed per artifact-in-revision; `schema.rs` DDL). Proposed change: content-keyed chunk storage referenced by per-artifact headers. If the rule is kept, patch upgrades and retained releases store full closure copies; A-01 still reduces round trips.
- **None for §B1.** The repeated parses are consistent with provider-local parsing.

## 8. Proposed light probes

- **P-A1.** Count the A-01 multiplier on the generation compile actually uses: non-empty closure files plus derived doc blocks. Run `find <selected site-packages> -name __pycache__ -prune -o -type f \( -name '*.py' -o -name '*.pyi' -o -name py.typed -o -name '*.pth' -o -name METADATA -o -name entry_points.txt \) ! -empty -print | wc -l`, plus the same over the corpus with the document set. On the legacy environment I measured 7,935 non-empty environment files. Discriminating outcome: thousands of files means at least about 12k-file-scale round trips for original bytes alone, confirming A-01 as High. Hundreds would lower it to Medium.
- **P-A2.** Static check, no run: confirm that no facts or normalization consumer binds a vocabulary relation before `freeze_completed_inputs`. Run `rg -n "Evidence|SyntaxDetail|AssertionQualification" crates/cpg-extract/src/{document_parser,deployment,ty_flow}.rs | rg "uses!|inputs"` together with `rg -n "contributes:" crates/lctx-model/src/domain`. Discriminating outcome: any such input makes the intermediate union views in A-02 consumed, so the remedy narrows to skipping zero-row contributions.
- **No other probe settles a verdict** without a build or a service. The A-02, A-03 and A-05 magnitudes, and peak memory, need the next authorized real-library pilot with phase timings and `/usr/bin/time -v`.
