# Inquiry F1: Rust build and test composition

**Provenance.** This is the verbatim final report of supporting inquiry F1, from one read-only
`design-reviewer` agent, for the [codebase efficiency-alignment review](../../reviews/design_review_efficiency-alignment-codebase_2026-10-11.md).
It was produced on 2026-10-11 at baseline `88f33bc0`, against the [inquiry briefs](inquiry-briefs.md).
Only this header was added. This is the inquiry's **revised final report**. It was issued after the coordinator's mid-run correction to probe P4 (check V03), and it supersedes the first version that inquiry returned.

How to read it:
- Candidate labels such as `F1-01` are local to this report. They are not the review's finding
  IDs.
- Its judgments are bounded and supporting. The principal review owns the combined decision.
- See the [evidence index](README.md) for coordinator checks and reconciliation.

---

The coordinator's correction is incorporated. The ~280 s figure is treated only as the cancellation instant and carries no weight. F1-01 below rests on static source evidence alone. The petgraph, fixedbitset and aws_lc_sys question is settled in §4.

---

# F1 — Rust build and test composition: supporting assessment

**Baseline:** `main` at `88f33bc0`, 2026-10-11. Production source is clean at that commit. For the four files with another session's uncommitted edits I read the HEAD content and relied on none of those edits. **Role:** read-only design-reviewer, bounded supporting assessment. Standard: core 3.3, heuristics 1.0, code-intelligence 1.5, plus the binding.

## 1. Scope examined and coverage limits

**Read in full or in the relevant parts:**
- Build configuration: `Cargo.toml` (workspace dependencies, profiles, `[patch.crates-io]`), every member manifest, `.cargo/config.toml`, `rust-toolchain.toml`, `.config/nextest.toml`, `.config/hakari.toml`.
- Build scripts and digest consumers: all build scripts, `scripts/producer_fingerprint.rs`, `cpg-extract/src/bundle.rs::build_digest`, `lctx-model/src/domain/model.rs::implementation_digest`, `lctx-serving/src/lib.rs::operation_definition`, `lctx-serving/src/preparation.rs` (`PreparedCache::key`, `available_key`, `hydrate*`), `cpg-core/src/workspace.rs::execution_implementation`, `cpg-core/src/native_calls.rs::NativeCalls::call`.
- Test drivers and cases: `cpg-extract/tests/typed_driver/mod.rs`, `tests/fixtures/native.rs`, `tests/extraction_calls.rs`, and the case modules `execution_channels`, `local_semantics` and `entry_value_witnesses`; `cpg-core/tests/fixtures/native.rs` and `catalog_runtime.rs`.
- Tooling and design records: `scripts/verify.py` (`BOUNDARIES`, profile resolution), ADR-0137 and DESIGN's build paragraph.
- Leads: the compilation-cost review (F01–F04, RC01–RC02), plan §9, §12 and §13, and the cargo-cache evidence README.
- Dependency closures: from `cargo metadata --offline`.

**Historical receipts read, read-only:**
- Run `20261011T071240.860Z-5e33c9`: summary, three `junit.xml`, `nextest-events.jsonl`, `nextest-recording-info.txt`.
- Run summaries and `output.log` TIMEOUT/SLOW lines across `build/runs/*` for model, providers and compiler boundaries, 2026-10-08 to 2026-10-11.
- The cargo `--timings` jsonl of compile-profile capture `20261009T003236.093Z-dfb25d`.
- Fingerprint JSON under `~/.cargo/build/library-context/release/build/tonic/*`, plus `du` of the shared build-dir.

**Not read:** the coroutine plan body beyond §13 of the compilation plan; emitted symbols or IR of the current tree; the `lctx-model-macros` expansion; the Python extension's maturin build path (F2's area); the 2026-09-24 build-performance evidence beyond its table, which describes a much smaller workspace. Nothing was built, tested or run. Every timing below is a historical receipt with its run ID.

## 2. Complete operations examined

**O1 — Edit one workspace crate, then run its focused controls (S10).**
- Edit → Cargo fingerprint (mtime and dep-info) → build scripts rerun if a watched path changed → rustc.
  - Workspace crates compile at O2, incremental, with no sccache.
  - Imported crates compile at O3, non-incremental, with sccache eligible.
- → each dependent workspace crate recompiles because the upstream SVH changed.
- → each test target compiles and links separately (mold).
- → nextest runs one process per test.
- Crossings: process per rustc unit, process per test, gRPC to the shared SurrealDB.
- Live state: up to 32 concurrent rustc processes, then 32 concurrent test processes, each with its own runtimes and pools.

**O2 — Build-time provenance → runtime identity.**
- `cpg-extract/build.rs` and `cpg-core/build.rs` call `producer_fingerprint::capture`. It walks the owner crate's normal and build path-dependency closure, every `[patch]` path package, the root manifests, lockfile and toolchain, the `third_party` directory watch and `*.patch` files, the runtime scripts, and `specs/`. It emits `cargo:rustc-env` digests.
  - Consumers: `bundle::build_digest` (provider identity, which also `include_str!`s the whole `Cargo.lock` and both fork patches) and `workspace::execution_implementation` (compiler contribution identity).
- `lctx-model/build.rs` and `lctx-serving/build.rs` concatenate raw sources into `OUT_DIR` blobs that are `include_bytes!`'d.
  - Consumer: `lctx_serving::operation_definition`. It is reached on every `PreparedCache::key`, as traced in F1-05.

**O3 — A fixture-backed provider test (`providers:extract`).**
- One process per test → `typed_driver::fixture_model` (`OnceLock`, per process) → `native_fixture::create_native` (new multi-thread runtime plus `NativeCompilerStore::begin` over gRPC) → `compile_facts` with Acquire, Pyrefly (`pyrefly_stage::analysis_pool`, `ThreadCount::AllThreads`), Documents, Deployment, Assemble and, for behavioral tests, TyFlow → `workspace.validate` → `observe`, a complete read-back of every relation for `Complete` demand → abandon the attempt.

**O4 — Profile resolution.**
- Bare `cargo build` or `cargo test` resolves to `dev`/`test` in `~/.cargo/build/library-context/debug`: 63 GB on 2026-10-11.
- `just verify` and `just fixture -- … --release` resolve to `release`: 48 GB.
- The `local-test-candidate` capture produced its own directory. It was absent at observation.

## 3. Candidate findings

### F1-01 — Fixture-driven provider tests recompile the same fixture once per test process

**Observed cause** (Implemented, static):
- Three case modules each wrap a full behavioral compile in a shared helper that every test awaits:
  - `crates/cpg-extract/tests/cases/execution_channels.rs::data` calls `typed_driver::run_behavioral(&files("execution_channels"), …)` with `ObservationDemand::Complete`; 11 tests call it.
  - `cases/local_semantics.rs::fixture` does the same over `local_semantics`; 13 tests.
  - `cases/entry_value_witnesses.rs::fixture` does the same; 5 tests.
- nextest runs each test in its own process, so the `OnceLock`s in `typed_driver::fixture_model` and `fixtures/native.rs::create_native` live for one test only.
- Each test therefore repeats, over identical input:
  - model validation;
  - a new native attempt;
  - a complete provider pipeline, Pyrefly plus ty included;
  - native persistence;
  - a complete read-back of every relation.
- That is 29 complete compiles of three fixtures. The tests then derive different assertions from the same observed tables.

**Consequence** (S10, and `test-all`/`qualify`): total work grows with the number of assertions, not with the number of distinct fixtures (H12). The repeated work also runs concurrently, which compounds F1-02. **Magnitude unmeasured.** The interrupted run 20261011T071240 is no evidence of duration. It does show that the 29–32 processes running together had not finished within the window before cancellation, but nothing more.

**Simpler conforming alternatives:**
- **(a) In-test grouping.** One test per (fixture, profile) compiles once and runs the existing check functions, collecting each named failure rather than stopping at the first. cpg-core already runs several compile scenarios inside one test (`tests/cases/catalog_core.rs`, `behavioral_frontiers.rs`).
- **(b) Library-owned setup step.** A nextest setup script (`[scripts.setup.*]` with `[[profile.*.scripts]]`, exporting via `NEXTEST_ENV`) produces a run-owned observed-tables artifact (Arrow IPC), keyed by the fixture content and `LCTX_PRODUCER_SOURCE_DIGEST`. The tests decode it.
  - Setup scripts are experimental and need `experimental = ["setup-scripts"]`. That is docs-level evidence via Context7 and is not settled against the pinned 0.9.146 source.
  - The artifact needs a storage-lifecycle owner (F2).

**Preservation constraints:**
- Each assertion stays independent and individually reported.
- Tests that vary limits, budgets, provider order, profile or abandonment behaviour keep their own compiles.
- Determinism checks keep two compiles.
- Fixture attempts stay owned and abandoned; a shared result must not be mutated.

**Counterexample check:** under (a), a panic in one check could mask later checks. Collect failures per check rather than relying on `#[should_panic]` semantics. Under (b), a stale artifact would silently feed old facts, so the key must include the producer digest and the fixture bytes. A name match is not enough (safeguard 3).

**Uncertainty:** what share of each test is the compile rather than the assertion. Static reading puts the compile as by far the larger part, but this is unmeasured.

**Label:** Implemented (cause). Consequence Proposed.

**Closure evidence:** a target inventory showing one compile per (fixture, profile); unchanged assertion inventories; a run receipt with per-test durations.

### F1-02 — Heavy fixture tests nest independently sized pools under nextest's full parallelism

**Observed cause** (Implemented, static): each fixture test process sizes its own pools to the whole host, and nextest admits tests up to its default slot count with no weighting:
- Pyrefly's pool is `ThreadCount::AllThreads` (`cpg-extract/src/pyrefly_stage.rs::analysis_pool`, which is correct for a single production compile).
- A default multi-thread tokio runtime (`tests/fixtures/native.rs::create_native`; `cpg-core/tests/fixtures/native.rs::store`), plus the test's own runtime.
- All of these processes share one SurrealDB server.

`.config/nextest.toml` declares no `threads-required` or `test-groups`. Each layer admits work as if it owned the machine (H21; DP-20).

**Consequence** (S10, `qualify`). Historical receipts, all `providers:extract` under the boundary filter, so not the F1-01 modules:

| Run | Result |
|---|---|
| `20261008T225907.834Z-dd7ee2` (qualify) | 22 TIMEOUT at 300 s |
| `20261009T170649.056Z-032c30` (qualify) | 21 TIMEOUT |
| `20261011T010025.404Z-5e92c3` (qualify) | 24 TIMEOUT |
| `20261009T205753.562Z-7c4894` | 68 of 69 passed in 176 s; slowest 176 s |
| `20261009T214543.898Z-81f4a8` (rerun) | 158 of 159 passed in 256 s |

The same selection is bimodal: it completes in under 4 minutes or about 20 tests time out together. That is the signature of contention-sensitive admission, not of fixed per-test work.

**Uncertainty:** whether CPU, the shared SurrealDB server or other sessions' host load dominates is not attributed. The 2026-10-08 run predates later source changes.

**Simpler conforming alternative:** nextest's own weighting, `threads-required` (a number or a CPU-relative value) on the heavy provider and compiler filters, so the runner's admission accounts for each test's real parallel demand. Optionally a `test-group` around SurrealDB-heavy tests. Both are declarative and owned by the runner. They do not change production pools, so Pyrefly keeps `AllThreads` for real compiles.

**Preservation:** production parallelism is unchanged. Light tests keep full concurrency. Concurrency defects must still be fixed in code; weighting must not hide a deadlock. Keep the timeout so a hang still fails.

**Counterexample check:** weighting could hide a genuine concurrency defect that only shows at full overlap. Keep at least one deliberately concurrent control, or periodic unweighted runs.

**Rule impact:** see RC-F1-a.

**Label:** Implemented (cause). Consequence supported by historical receipts.

**Closure evidence:** a matched-load comparison of the same selection with and without weighting (proposed probe P-4).

### F1-03 — cpg-core's normal dependency on cpg-extract puts the whole analyzer stack into publication, embedding and serving-test closures

**Observed cause** (Interface-checked):
- `crates/cpg-core/Cargo.toml` depends on `cpg-extract` (normal). `lctx-publisher` and `lctx-embed` depend on `cpg-core`.
- What they actually use:
  - `lctx-publisher` uses only `cpg_core::artifact::{AdmittedArtifact, VerifiedExport, RestoredAdmission, verify_restored}` and `cpg_core::workspace::{Workspace, WorkspaceOptions}` (`restore.rs`, `lib.rs`, `inspection.rs`, `backup.rs`, `native_publication.rs`).
  - `lctx-embed`, an HTTP client, uses only `cpg_core::embedding_service::Embedder`, `retrieval_tokenizer::LocalTokenizer` and `CoreError`.
- The coupling in cpg-core is the provider contract (`cpg_extract::bundle::{CapturedInputs, ProviderSink, ProviderStage, Declared, StageContext}`, `contracts`) and the default provider list in `facts.rs`. cpg-core uses `cpg_extract` in `workspace.rs`, `facts.rs`, `compilation.rs`, `compilation/cache_controls.rs` and one signature in `artifact.rs`.
- Offline `cargo metadata` closures:

| Crate | Normal closure (packages) | Analyzer stack in closure |
|---|---|---|
| `lctx-publisher` | 791 | pyrefly, ruff_linter, salsa, tokenizers |
| `lctx-embed` | 793 | pyrefly, ruff_linter, salsa, tokenizers |
| `lctx-surrealdb` | 475 | none |
| `lctx-serving` (normal) | 481 | none |
| `lctx-serving` (with dev) | 795 | pyrefly, ruff_linter, salsa, tokenizers |

`lctx-serving` dev-depends on `cpg-core`, `cpg-extract` and `lctx-publisher`.

**Consequence** (S10; H5, H13, H27; DP-17; A1/A4 change locality):
- Any provider edit in cpg-extract recompiles cpg-core, then `lctx-publisher`, `lctx-embed`, `lctx` and every serving and publisher test target.
- `store:rust` and `serving:rust` test binaries link Pyrefly, Ruff and ty that they never execute.
- Receipt from capture `20261009T003236` (O1, instrumented): the cpg-core library took 95.9 s, on the dependency path of everything above it.
- The coordinator supplied focused `store:rust` receipts from 2026-10-10: build and link took 34 s to 5 min 54 s against a 1–24 s nextest suite. That is consistent with the store loop being dominated by its compile closure.

**Simpler conforming alternative:**
- Move the provider contract types, which carry no analyzer, below cpg-core: into a small contract crate, or into `lctx-model` where they are model contracts. Let the composition owner (the `lctx` binary, and the facts test drivers) construct the analyzer providers and pass them in. `compile_facts` already receives `Vec<Box<dyn ProviderStage<…>>>`.
- Move `Embedder` and `LocalTokenizer` below cpg-core, or invert `lctx-embed` so it no longer depends on cpg-core.
- Splitting cpg-core's admission, artifact and workspace layer from facts orchestration is the larger variant.

**Preservation:**
- ADR-0118 adapter isolation (no AST types cross).
- Provider identity, from `contracts` and `bundle::build_digest`, continues to enter admission unchanged.
- The ordering assertions in `compilation.rs` remain tied to the declared provider names.
- No new provider framework; a crate move is justified only because it removes a real compile edge.

**Uncertainty:** how much of `facts.rs` and `compilation.rs` needs concrete provider statics (`ty_flow::TY_FLOW`, `deployment::DEPLOYMENT`) rather than declarations. Some may need to stay where they are.

**Label:** Interface-checked.

**Closure evidence:** `cargo tree -i pyrefly` shows no publisher or embed path; a provider-only edit leaves `lctx-publisher` fresh.

### F1-04 — lctx-surrealdb combines the compiler-side store and DataFusion provider with the serving reader, coupling both rebuilds and compiler identity

**Observed cause** (Implemented, static):
- DataFusion is used only in `lctx-surrealdb/src/compiler.rs` (7,810 lines) and `compiler_provider.rs`.
- `lctx-serving` and the Python binding (`python/lctx_semantics`) use reader, serving, completion and resources APIs and never touch `lctx_surrealdb::compiler`. Both still link DataFusion and compile the compiler store.
- In the other direction, cpg-core's `producer_fingerprint::capture` walks all of `crates/lctx-surrealdb/src`, so `LCTX_COMPILER_SOURCE_DIGEST` (`workspace.rs::execution_implementation`) changes when `reader.rs`, `derived_search.rs` or `selection.rs` change.

**Consequence:**
- S10: a compiler-store edit rebuilds serving and the native Python extension.
- S03: a serving-only edit changes every compiler contribution's implementation identity, so all compiler reuse for the next compile is invalidated. That is conservative and safe, but reuse is lost for an unrelated change (H13, DP-09). The over-inclusion is caused by the crate boundary, not by a semantic dependency.

**Simpler conforming alternative:** split `compiler`, `compiler_provider` and `projected_arrow` into a compiler-store crate, or feature-gate them, above a shared native core (codec, control, config, completion helpers). The provenance closure then follows the split automatically, because `capture` is crate-rooted.

**Preservation:** one native codec and control authority with no duplicated store logic; publication and seal semantics; pinned readers.

**Uncertainty:** how much compiler and reader share internally (control, codec, `materialization`). These seams belong to D and E. Whether `derived_search` and `reconciliation` belong with the compiler or with publication is D's question.

**Label:** Implemented (cause). Consequence Proposed.

**Closure evidence:** the serving and binding closures without `datafusion`; a reader-only edit that leaves `LCTX_COMPILER_SOURCE_DIGEST` unchanged.

### F1-05 — The model and serving implementation digests embed raw sources and are recomputed on every serving preparation key

**Observed cause** (Implemented, static):
- `crates/lctx-model/build.rs` writes every model and macro source file, plus the root manifest and lockfile, into `model-implementation.bin` (8,259,597 bytes in the release build dir, 2026-10-11).
- `lctx_model::domain::implementation_digest` computes `ContentHash::of(include_bytes!(…))`. That is `blake3::hash`, single-threaded, with no memo.
- `lctx_serving::operation_definition` hashes the 584 KB serving blob, then that model digest, then three further identities, also unmemoized.
- `operation_definition` is called inside `preparation.rs::PreparedCache::key`. `available_key` calls `key` for the `layout` key and the `closure` key in `hydrate`/`hydrate_scope`. Those are reached from `capability`, `evidence`, `library`, `selection`, `inspection`, `operation_packet` and `scope`.
- So each serving preparation lookup, cache hits included, re-hashes about 8.8 MB of fixed embedded bytes at least once per key.

The digest is a true input to one consumer: serving's executable identity (operation definitions, prepared-cache keys and the one-time definition check in `service.rs`). It is not consumed by compile reuse.

**Consequence** (S06; H12, H19, H26; DP-10): a fixed property of the binary is re-established on every request, and the rich representation (raw sources) also adds about 8.8 MB to every binary that reaches it. The per-call cost is unmeasured; see probe P-3.

**Simpler conforming alternative:** have the build script compute the hash itself. `ContentHash::of` is plain `blake3::hash` over these exact bytes, so a `blake3` build-dependency (cpg-core and cpg-extract already have one) can emit a bit-identical 32-byte value. Alternatively, memoize both functions behind a `LazyLock`. Either preserves the identity value exactly.

**Preservation:** identical digest values, so published definitions and stored `fn::lctx_operation_definition` stay valid; membership and deletion sensitivity.

**Seam:** E owns the serving per-call cost; G owns identity.

**Label:** Implemented.

**Closure evidence:** the digest value is unchanged before and after; there is no `include_bytes!` of the blob; key construction does no large hashing.

### F1-06 — Two near-identical optimized profiles (and a third candidate) keep separate full build trees

**Observed cause** (Implemented, static), from `Cargo.toml`:

| Profile | Workspace crates | Imported crates | Codegen units | Assertions |
|---|---|---|---|---|
| `dev` | O2, incremental, `debug = 0` | O3 | 256 (default) | on (default) |
| `release` | O2, incremental | O3 | 16 (default) | off (default) |

- `local-test-candidate` inherits `test`, which inherits `dev`. It lives in its own directory, so imported crates are recompiled there too, though sccache can serve identical non-incremental units.
- AGENTS.md routes implementation "compile checks" to `cargo check`/`cargo build`, which resolves to `dev`. Verification and fixtures use `release` (`scripts/verify.py::DEFAULT_CARGO_PROFILE`).
- Observed 2026-10-11: debug 63 GB, release 48 GB in the shared build-dir.

**Consequence** (S10; H12, H27): when an agent uses `cargo build` or `cargo test` and then `just verify`, the edited closure is compiled twice at nearly the same optimization level. Every dependency move compiles its O3 subtree once per profile directory. The profiles differ mainly in assertions and codegen-unit count, which reflects no workload distinction anyone has stated.

**Simpler conforming alternative:** map profiles to the two real workload classes:
- one local profile, used by bare `cargo build`/`cargo test` and by ordinary verification (the qualified BC3 candidate), set as `dev`/`test` rather than as an extra named profile;
- `release` for production, oracle and acceptance builds.

Alternatively, make `cargo check` the documented compile check.

**Relation to the lead:** this extends compilation-cost F03 rather than duplicating it. F03 concerns optimizing the test loop; F1-06 concerns profile multiplicity. The principal reviewer may merge them under one owner.

**Uncertainty:** how often `cargo build` (codegen) rather than `cargo check` is used. The debug-tree size shows codegen use but not how often.

**Rule impact:** see RC-F1-b.

**Label:** Implemented (cause). Consequence Proposed.

## 4. Opportunities and uncertainties that are not findings

- **The analytics:rust "second copies" (coordinator question): refuted as a feature-unification failure.**
  - In run `20261011T071240`, step `d2c9b72f…`, the non-fresh units were `petgraph@0.6.5`, `fixedbitset@0.4.2`, `rust-sugiyama@0.3.0`, and `aws-lc-sys@0.45.0` → `aws-lc-rs` → `rustls` → `hyper-rustls` → `reqwest@0.13.5`, all under `odis@2026.9.1`.
  - These are distinct versions, not feature variants. `odis`'s manifest declares a non-optional, target-gated `reqwest 0.13.4 (rustls)` dependency (`cargo tree -i aws-lc-sys` shows only the `odis` dev edge of `lctx-analytics`).
  - The workspace's own `petgraph 0.8.3` and `fixedbitset 0.5.7` reused one artifact hash (`411c5708…`, `95d548c3…`) across the model, analytics and providers steps, so workspace unification works.
  - The cost is a cold C/asm build of aws-lc-sys per profile directory for an oracle. That is recorded as a hold in `docs/pins.md`. Low priority, not a finding.
- **The lctx-model crate holds both declarations and kernels** (196k lines: `execution` 36.5k, `normalized` 22.7k, `synthesis` 12.3k, `analytics` 5.9k). Every crate depends on it, so any kernel-body edit recompiles every downstream crate, serving and the Python binding included (S10). Serving, the binding, `cpg-flow` and `lctx-surrealdb` never reference `execution`, `synthesis`, `structural` or `analytics`.
  - Why this is not a finding: `model()` needs every relation declaration; records (`#[derive(Domain…)]`, 135 in `execution` alone) are interleaved with kernels; and incremental compilation may absorb much of the downstream work. A separation of semantic authority from physical placement (H3) is credible but needs measurement (probe P-5).
- **Unit-test harnesses double-compile the largest crates.** Capture `20261009T003236` (O1, instrumented): cpg-core lib 95.9 s; its `--lib` unit-test harness 162.0 s, finishing last. cpg-core has 165 unit tests in 98 `cfg(test)` modules; lctx-model has 257. The remedy needs a case-by-case decision on private access, so this is not a finding.
- **Conservative root inputs to provenance.**
  - `Cargo.lock` is watched by three build scripts and `include_str!`'d into `bundle::build_digest`. Root `Cargo.toml`, `rust-toolchain.toml`, patch files, the `third_party` directory watch and `specs/` also feed the digests.
  - Since 2026-09-25, 76 of 1,541 commits touched `Cargo.lock`, and 40 touched specs, `third_party` or the toolchain. Each such change recompiles lctx-model, cpg-extract and cpg-core and everything downstream.
  - The conservative choice is recorded in ADR-0137, so this is an opportunity, not a finding.
  - Residual detail: cpg-extract's closure includes the `third_party/surrealdb` patch package, which is not a cpg-extract dependency.
- **sccache, settled at 0.17.0** (cargo-cache README, sccache Rust.md):
  - It caches only non-incremental, non-linker crate types: the imported O3 rlibs.
  - It caches nothing for workspace crates (all incremental in dev and release), test executables, binaries, proc-macros or the cdylib.
  - Its value is limited to imported units recompiled after fingerprint invalidation or into another profile directory.
  - Historical counters: 9 hits / 1,467 misses (2026-09-28). The wrapper cost is small. Adequate, not a finding.
- **Hakari.** Cargo's `resolver.feature-unification = "workspace"` already unifies features for every selection, as observed through identical unit hashes across steps. The CLI-only hack appears to have no consumer beyond `just deps` and `just turn-end` regeneration. Low-cost machinery (H27); confirm its intended consumer before removing it.
- **Unexplained third-party rebuilds.**
  - In run `20261011T071240`, providers step `f8002890…`, `pyrefly`, the `surrealdb*` crates, `tonic` and `ty_*` were non-fresh with no manifest change since 2026-10-10.
  - The release directory holds two `tonic` units (`a5145e…`, `d5f7b1…`) with identical features and rustflags but different `profile` fingerprints.
  - A second release-profile configuration (for example `cargo test --release` against `nextest --cargo-profile release`) may be maintaining parallel trees. Not established; see probe P-2.
- **`lctx-analytics`.** A re-export facade whose only workspace consumer is one cpg-extract test (`normalized_projections`). It keeps the oracle dev-dependencies out of lctx-model's test closure. Adequate.

## 5. Strengths, preservation constraints and adequate areas

- **Exact pins, and one feature set per profile across all selections** (observed identical artifact hashes). Keep workspace unification.
- **The F02 correction is real.** `producer_fingerprint::capture` is now owner-rooted. It walks normal and build path dependencies only and tracks membership and deletion through directory watches. A serving edit no longer touches cpg-extract's identity. Keep that tracking, relocation-independent relative paths, and the runtime-script declaration route (`runtime_scripts!`).
- **NativeCalls erasure.** `NativeCalls::call` boxes the request future before admission and Tokio scheduling (`call_boxed`). This is the F01 direction. Keep the charged-ownership semantics.
- **Grouped extraction harnesses.** `extraction_calls.rs` and related targets group 10 case modules per target, so the driver compiles once per binary.
- **Narrow verify selection.** Boundary filters (`PROVIDER_TARGET_FILTERS`) let a narrow `verify --select` avoid compiling unrelated targets, and verification resolves one profile for commands, artifacts and receipts.
- **Division of compilation work.** Incremental workspace crates plus O3 non-incremental imports under sccache is a sound split. Mold and clang with automatic frontend/backend workers is adequate.
- **Per-process fixtures.** These `OnceLock`s are correct per process. They just do not amortize under process-per-test (F1-01).
- **Independence to preserve.** Every test keeps its own expectations and fixture attempt. None of the remedies above may merge assertion logic into producer code.

## 6. Positions on earlier leads

| Lead | Position | Evidence |
|---|---|---|
| Compilation-cost F01 (record-specific futures) | **Residual** | `NativeCalls::call` boxes before scheduling. Plan §12–13 records C1/CU0–CU5 integrated. §9 keeps it open pending CU6 qualification. The latest capture (`20261009T003236`, O1) still shows cpg-core lib 95.9 s and harness 162 s. No current symbol evidence was read. |
| Compilation-cost F02 (whole-workspace provider provenance) | **Residual** | Owner-rooted closure implemented (`producer_fingerprint::capture`/`Collector::package`). Remaining: patch-package roots (surrealdb in cpg-extract), root lock, toolchain and specs sensitivity. A new manifestation: cpg-core's closure spans all of lctx-surrealdb, the serving reader included (F1-04). |
| Compilation-cost F03 (no qualified local loop) | **Confirmed still present** | `verify.py::DEFAULT_CARGO_PROFILE = "release"`; `local-test-candidate` not installed. Extended by F1-06. |
| Compilation-cost F04 (repeated drivers across targets) | **Residual** | cpg-extract grouped. cpg-core keeps 18 targets, 17 including `fixtures/native.rs` and 5 including the 284-line `catalog_runtime.rs`. lctx-model keeps 86 targets, several of 21–43 lines. Run `20261011T071240` model:rust: step 128.6 s against a 2.7 s suite with 12 units rebuilt, consistent with the coordinator's note. F1-01 is the runtime counterpart. |
| RC01 | **Resolved as a rule change** (operator-accepted, ADR-0137); implementation pending (BC3) | ADR-0137 Decision; DESIGN build paragraph. |
| RC02 | **Resolved as a rule change**; implementation residual (see F02 row) | ADR-0137 "Provider source attribution follows the owning crate's … closure". |
| Coroutine review F01 | **Residual** | Plan §13: CU0–CU5 integrated; CU6 and native qualification open. |
| P4 "~280 s per test = per-process preparation" | **Refuted as stated** (the duration is the SIGTERM instant: 32 `process aborted with signal 15`, recording status `cancelled`, 183 not run). The *structural* repetition is separately established (F1-01). | `junit.xml` and `nextest-recording-info.txt` in `f8002890…` |
| 2026-09-24 rust-build-performance | **Not examined** (it describes a much smaller workspace) | — |
| 2026-09-28 cargo-cache | **Confirmed** for the sccache and shared build-dir limits | README "Remaining limits" |

## 7. Candidate rule impacts

- **RC-F1-a (F1-02).**
  - *Rule and where recorded:* "do not impose job, worker or thread caps to make checks pass" (AGENTS.md, Testing and Cargo setup); ADR-0137 "tests use their ordinary worker defaults"; operator memory "Never constrain test parallelism".
  - *Proposed change:* allow nextest `threads-required` and test-group weighting that declares the real parallel demand of heavy fixture tests. This keeps the runner at default parallelism and production pools unchanged; it is not a cap for passing.
  - *If the rule is kept:* F1-01 still removes repeated work, but nested independent admission remains, and runs stay sensitive to host load.
- **RC-F1-b (F1-06).**
  - *Rule and where recorded:* ADR-0137's profile set (dev O2 plus a separate qualified test profile plus release); the AGENTS.md "Compile checks (`cargo check`/`cargo build`)" instruction; the DESIGN build paragraph.
  - *Proposed change:* define `dev`/`test` as the single qualified local profile, keep `release` for acceptance, and name `cargo check` as the default compile check.
  - *If the rule is kept:* edits validated through both bare cargo and verify keep compiling twice.
- **F1-03, F1-04, F1-05:** no rule changes. Crate moves preserve ADR-0118 isolation and the ADR-0137 provenance semantics. The DESIGN §15 crate descriptions would be updated with any split.

## 8. Proposed light probes

- **P-1** (light). *Question:* what share of `providers:extract` heavy-test time is the shared compile?
  - *Command:* `python3 -I` over existing `build/runs/*/verify-evidence/*/junit.xml` and `output.log` SLOW/PASS lines, correlating test duration with whether the test calls `typed_driver::run*` once per test or shares a helper.
  - *Discriminating outcome:* if the tests that share a helper have nearly identical durations, close to the run of a one-off compile test, the compile dominates.
  - *Limit:* existing receipts are sparse; only 3 invocations carry JUnit.
- **P-2** (light). *Question:* why do parallel release-profile units exist?
  - *Command:* `for c in ~/.cargo/build/library-context/release/build/*/; do n=$(ls $c | wc -l); [ $n -gt 1 ] && echo $c; done`, then for multi-unit crates `python3 -I -c` to diff `fingerprint/*.json` (features, profile, rustflags).
  - *Discriminating outcome:* many crates differing only in `profile` means a second release configuration is maintaining a parallel tree (an F1-06 extension). Differing `features` would contradict workspace unification.
- **P-3** (light). *Question:* what does one `operation_definition` hash pass cost?
  - *Command:* `b3sum` (if installed; otherwise skip) timed five times over `~/.cargo/build/library-context/release/build/lctx-model/5c97e1f3797c1c35/out/model-implementation.bin`.
  - *Discriminating outcome:* a per-pass time in milliseconds, which, multiplied by the static count of key constructions per serving request, bounds F1-05's per-call cost.
  - *Limit:* warm cache, as with P1.
- **P-4** (not light; the coordinator decides). Run the same `providers:extract` selection twice under matched host load, once with `threads-required` on the heavy filters and once without, recording per-test durations and the TIMEOUT count. This settles whether F1-02's bimodality comes from admission.
- **P-5** (not light). A compile-profile record of a one-line body edit in an `lctx-model::domain::execution` kernel, recording which workspace units rebuild and how long each takes. This settles the lctx-model opportunity.
