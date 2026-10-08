# Rust compilation costs: typed boundaries, shared compiler work and the local test loop

**Principal design/target review · 2026-10-08**

**Decision: Revise.** The workspace has useful semantic ownership, established library composition and correctly separated production provenance and semantic compatibility. Its Rust compilation realization nevertheless amplifies work in ways that do not follow those semantic boundaries. Record-specific asynchronous requests carry their concrete future types into Tokio’s task machinery; extraction provenance watches downstream and unrelated workspace sources; substantial test drivers are compiled into many separate integration executables; and ordinary test compilation uses substantial optimization without a separately qualified local-loop policy.

These causes interact, but they are not one defect. Narrowing invalidation reduces how often compilation occurs. Moving shared async mechanics behind checked typed boundaries reduces what each compilation must produce. Changing local test settings reduces optimizer work for a deliberately different developer workload. Consolidating test build units removes repeated compilation and linking of common test orchestration. None of these establishes a quantitative speedup without measurement.

The reported transition to one busy CPU has a credible compiler explanation: backend parallelism schedules modules, while an expensive function or module can remain on the critical path after other work finishes. Automatic worker settings cannot divide every function into independently schedulable optimization work. Existing completed object files corroborate substantial specialization of native-call task machinery. They do **not** identify the unfinished module’s body or establish which optimization pass dominates the observed tail.

The recommended direction preserves production runtime performance as the governing constraint. Keep typed records, semantic keys, shared validation, exact completed inputs, charged batching and cancellation/drain ownership. Compile common SQL, submission and task-management mechanics once where their inputs have already become runtime descriptors and Arrow batches. Introduce a separately qualified local test policy while retaining optimized production acceptance. Do not introduce worker caps, another scheduler, blanket per-row dynamic dispatch or an arbitrary collection of new crates.

## 1. Review contract and evidence boundary

| Field | Scope |
|---|---|
| Subject | Rust build configuration, `cpg-core`, adjacent model/extraction/native boundaries, provenance build scripts and integration-test build structure |
| Baseline | HEAD `f9714734` plus the identified dirty main tree, including integrated PC1–PC5 source changes |
| Standard | Core/template 3.3; Heuristics for Efficient Architecture 1.0; Code Intelligence 1.5; library-context binding |
| Tier and purpose | Design / target |
| Reviewer | Independent delegated design reviewer |
| Date | 2026-10-08 |
| Functional intent | Compile and exercise the evolving Rust workspace without disproportionate repeated compiler work, while preserving optimized production behavior and independent functional assurance |
| Workload premise | A large native Rust dependency graph, many nominal record types, multiple asynchronous compiler stages, numerous integration executables, repeated local edits and focused/full workspace test compilation |
| Expected changes | Add a record or stage; change coordinator or serving implementation; run a focused contract control; substitute the physical native-request submission mechanism; select different local test optimization |
| Method | Static source/configuration review; exact pinned compiler/Cargo/sccache source qualification supplied by the library researcher; current completed-object inspection supplied by the coordinator |
| Excluded | Product catalog execution performance, operator databases, acquisition of the real FastMCP library, running-build intervention, new builds/probes, test execution and implementation |
| Maturity | Structural diagnoses are **Implemented evidence, inspected 2026-10-08**. Remedies are **Proposed** unless identified as an inspected existing capability. No speedup is claimed |

The root independently examined live build artifacts; this reviewer independently read the decisive application sources. Artifact figures below are attributed to that inspection rather than represented as a new reviewer-run measurement. Existing plan receipts retain their original scope and date.

The review judges the build architecture against the functional target and loaded standard. ADR-0136, ADR-0135 and current agent rules describe the subject’s selected choices; agreement with them does not establish architectural fitness.

## 2. Responsibilities and consequential distinctions

| Owner | Coherent responsibility | Current realization and expected change |
|---|---|---|
| Root `Cargo.toml` and `.cargo/config.toml` | Compilation policy, dependency resolution, feature unification, output/cache locations and selected compiler/linker settings | Changes when development/production compilation policy or toolchain capabilities change |
| Cargo and rustc | Dependency scheduling, compiler units, incremental state, code generation and backend scheduling | Their established mechanisms should remain the owners of scheduling |
| `lctx-model` and its macros | Nominal records, identities, constraints, codecs, semantic declarations and producer compatibility | New semantic kinds legitimately add model declarations and typed code |
| `cpg-extract` | Provider-local observations and captured producer implementation identity | Analyzer/provider implementation changes belong here; unrelated downstream rendering changes do not |
| `cpg-core::workspace` | Attempt-owned outputs, batching, immutable completed inputs and native submission coordination | The typed and physical portions of an operation need not share the same generic compilation boundary |
| `NativeCalls` and `NativeBridge` | Submitted-call ownership, acknowledgement, cancellation and drainage | Shared scheduling semantics should not specialize on every record-specific future |
| `consumed_rows` and stage owners | Checked completed-input dispatch, read-only streaming and stage-owned batch interpretation | Exact source/epoch meaning must survive a shared physical stream loop |
| Integration harnesses | Independent expectations against actual production capabilities | Logical test organization need not require a separate compiled copy of substantial driver code per source file |
| Verification tooling | Explicit selection, readiness observation and evidence reporting | Existing named-target narrowing is valuable; a test-name filter alone does not necessarily narrow Cargo compilation |

The source inventory provides context, rather than compile-time attribution.

The workspace map counted 201 top-level integration sources (core 31, extraction 61,
model 86). The 263-line extraction `typed_driver` is included by 43 targets; the
287-line core `catalog_runtime` by 21. The model contains roughly 300 source files,
including an approximately 17,860-line graph declaration and hundreds of derives.
Model derives expand primarily in the model crate; downstream users do not automatically
re-run those derives. Large nominal/codec inventories are legitimate model work,
not by themselves proof of avoidable compile cost. F01 distinguishes that work from
the repeated shared task/control machinery instantiated in consumers.

Consequential distinctions:

- A source change is different from a change affecting a particular compiled unit.
- Semantic compatibility is different from executable provenance.
- A checked typed row is different from its encoded transfer representation.
- A submitted native request is different from a caller awaiting its result.
- Compilation of a test executable is different from execution of selected tests.
- A local development test profile is different from production qualification.
- Available worker capacity is different from the remaining dependency or module critical path.
- Incremental reuse, Cargo freshness and sccache reuse are different mechanisms.

Those distinctions already have credible authorities. The problems are primarily the physical placement and granularity of work, rather than a missing universal build model.

### Code-intelligence profile boundary

No fact family, analysis model or served answer is being redesigned here. Relevant profile obligations constrain remedies rather than justify a new product audit.

| Concern | Inspected authority | Preservation requirement |
|---|---|---|
| Provider implementation attribution | `cpg-extract::bundle::build_digest`; captured producer bindings | Record the complete relevant implementation/configuration/source premises |
| Semantic compatibility | `lctx-model` declarations and explicit revisions; DESIGN §15 | Do not substitute broad source hashes for semantic contracts |
| Completed-input provenance | `CompletedInput<R>` / `SourceSnapshot`; exact workspace input selection | Erasing physical streaming mechanics must retain the selected producer, model, implementation, view and relation |
| Typed fidelity and validation | `Record`, generated codecs and `Batch<R>` | Do not bypass nominal matching, identity validation or coverage interpretation |
| Product evidence closure and evaluation | Existing product owners | No changes or qualification claims in this review |

## 3. What the compiler symptom establishes

The checked-in Linux flags select automatic frontend/backend workers:

```text
-Z unstable-options --jobs-frontend=0 --jobs-backend=0
```

They do not establish that every phase or final item can occupy every CPU. The exact pinned rustc source qualifies the following distinctions:

1. **Frontend and backend workers are separate mechanisms.** More backend capacity does not parallelize an indivisible frontend operation, and more frontend capacity does not subdivide an LLVM function.
2. **Codegen partitioning assigns whole function mono-items.** The partitioner distributes and merges units; it does not split one function’s body into independently optimized Rust codegen units.
3. **A codegen-unit limit is not a promise of balanced units.** Incremental compilation’s default partitioning can still leave skewed units.
4. **Backend work is scheduled per module.** A large remaining module can produce a serial tail after the other modules finish.
5. **Local ThinLTO is different from whole-program cross-crate LTO.** The inspected compiler path can perform local ThinLTO without importing upstream rlibs. “LTO worker” therefore does not imply that the workspace enabled global fat LTO.
6. **Optimization and generic sharing differ by effective profile.** The inspected pinned configuration defaults generic sharing differently at O2/O3 than at lower optimization levels. This is a condition to account for, not proof that changing optimization alone eliminates the application’s duplication.
7. **Not all apparent inlining is MIR inlining.** The inspected compiler source disables the ordinary default MIR inliner in the relevant incremental configuration. LLVM optimization remains material; blaming MIR inlining merely because the workspace uses O2 would be unsupported.

The coordinator inspected completed objects under:

```text
/home/paul/.cargo/build/library-context/debug/build/cpg-core/5ae03f0d95fe1f1a/out
```

The command was:

Each listed object uses the filename prefix `cpg_core-5ae03f0d95fe1f1a.`.

```text
llvm-nm --defined-only --print-size --size-sort --radix=d --demangle FILE
```

| Completed object suffix | File bytes | Defined symbols | Symbols containing `native_calls::NativeCalls` |
|---|---:|---:|---:|
| `dpvrryfqkmtvjwp4qh4a1kues.145up70.rcgu.o` | 119,070,872 | 96,881 | 49,905 |
| `8v8meqxnzjydqn2crqfhh4a25.145up70.rcgu.o` | 77,515,176 | 63,283 | 31,720 |
| `8ikqejbsz5g1ezrdrhbv1g707.145up70.rcgu.o` | 62,936,800 | 60,115 | 46,578 |

In the first object, the matching defined symbols were text symbols whose reported sizes totaled 17,008,647 bytes. Inspected names included Tokio harness/stage/drop machinery containing record-specific `Writer<…>::close` and `declare_async<…>` futures.

These are **per-object counts**, not globally unique instantiations, compilation times or a whole-build size estimate. Rust can copy generic/inline/drop bodies across codegen units. The corresponding unfinished object for the current worker was empty when observed, so the currently slow worker’s body was not established by its symbols.

The appropriate conclusion is narrower and useful: the source’s record-specific request types are visibly propagating into substantial emitted task machinery. It is not necessary to infer a compiler regression to explain why this architecture presents expensive work to the optimizer.

### Scheduling, locks and caches

These causes require different remedies:

| Cause | Meaning | What this review establishes |
|---|---|---|
| LLVM/module tail | Useful compiler work remains in a skewed unit after other units finish | Exact compiler scheduling permits it; the unfinished unit’s dominant body/pass remains unconfirmed |
| Cargo scheduling | Dependents wait for required predecessor work | A legitimate dependency critical path; worker flags cannot remove it |
| Shared-output locks | Independent processes contend for protected mutable build state | Fine-grain locking does not eliminate every lock; this review does not diagnose a current lock as the active CPU-bound tail |
| Incremental invalidation | Changed premises cause affected queries/codegen units to be recomputed | Broad provenance dependencies make additional invalidation certain |
| sccache miss or non-cacheability | A compilation cannot be satisfied by a compatible stored artifact | Pinned sccache does not cache incremental Rust compilations or linker outputs |
| Repeated test targets | Several executable units independently compile their harness code and link | Established by the integration source structure |

Keeping incremental workspace compilation and sccache for non-incremental imported dependencies is a coherent division of responsibilities. sccache should not be expected to erase record-specific optimized workspace compilation on an edit. Conversely, disabling incremental compilation globally to pursue sccache hits would exchange one workload policy for another without settling this review’s structural findings.

### Exact compiler and library references

Consequential compiler claims above use the installed Rust revision
`c1070d69382b8d2f2eb65119c738a77d9e324c9e` and Cargo revision
`3d7cf6e937d6127d0f49881bf689c560b36d35c4`, rather than assumptions about floating nightly behavior.

| Claim | Primary source |
|---|---|
| Concrete instance collection; lazy selection unless link-dead-code | [rustc collector](https://github.com/rust-lang/rust/blob/c1070d69382b8d2f2eb65119c738a77d9e324c9e/compiler/rustc_monomorphize/src/collector.rs#L1916), [collection selection](https://github.com/rust-lang/rust/blob/c1070d69382b8d2f2eb65119c738a77d9e324c9e/compiler/rustc_monomorphize/src/partitioning.rs#L1169) |
| Whole-function placement and codegen-unit merging | [placement](https://github.com/rust-lang/rust/blob/c1070d69382b8d2f2eb65119c738a77d9e324c9e/compiler/rustc_monomorphize/src/partitioning.rs#L201), [merging](https://github.com/rust-lang/rust/blob/c1070d69382b8d2f2eb65119c738a77d9e324c9e/compiler/rustc_monomorphize/src/partitioning.rs#L317) |
| Generic/inline/drop copies and MIR versus LLVM inlining | [instantiation mode](https://github.com/rust-lang/rust/blob/c1070d69382b8d2f2eb65119c738a77d9e324c9e/compiler/rustc_middle/src/mono.rs#L132), [copies](https://github.com/rust-lang/rust/blob/c1070d69382b8d2f2eb65119c738a77d9e324c9e/compiler/rustc_monomorphize/src/partitioning.rs#L262), [MIR inliner policy](https://github.com/rust-lang/rust/blob/c1070d69382b8d2f2eb65119c738a77d9e324c9e/compiler/rustc_mir_transform/src/inline.rs#L46) |
| Coroutine poll/drop bodies and per-module local ThinLTO | [coroutine lowering](https://github.com/rust-lang/rust/blob/c1070d69382b8d2f2eb65119c738a77d9e324c9e/compiler/rustc_mir_transform/src/coroutine/mod.rs#L14), [ThinLocal scope](https://github.com/rust-lang/rust/blob/c1070d69382b8d2f2eb65119c738a77d9e324c9e/compiler/rustc_codegen_ssa/src/back/write.rs#L1749), [backend scheduling](https://github.com/rust-lang/rust/blob/c1070d69382b8d2f2eb65119c738a77d9e324c9e/compiler/rustc_codegen_ssa/src/back/write.rs#L1026) |
| Generic-sharing defaults and conditions for upstream reuse | [defaults](https://github.com/rust-lang/rust/blob/c1070d69382b8d2f2eb65119c738a77d9e324c9e/compiler/rustc_session/src/config.rs#L1550), [reuse selection](https://github.com/rust-lang/rust/blob/c1070d69382b8d2f2eb65119c738a77d9e324c9e/compiler/rustc_middle/src/ty/instance.rs#L221) |
| Cargo profile inheritance, local LTO and override restrictions | [pinned Cargo profiles](https://github.com/rust-lang/cargo/blob/3d7cf6e937d6127d0f49881bf689c560b36d35c4/doc/book/src/reference/profiles.md), [override precedence](https://github.com/rust-lang/cargo/blob/3d7cf6e937d6127d0f49881bf689c560b36d35c4/src/cargo/core/profiles.rs#L489) |
| Incremental Rust and linker-output cache exclusions | [sccache 0.17 Rust support](https://github.com/mozilla/sccache/blob/v0.17.0/docs/Rust.md) |
| Automatic boxing of a large concrete future still retains its concrete type | [Tokio 1.53.2 spawn implementation](https://github.com/tokio-rs/tokio/blob/tokio-1.53.2/tokio/src/task/spawn.rs#L171) |

Tokio's automatic `Box::pin(future)` for a large `F` is distinct from erasure to
`dyn Future`. It still specializes scheduling on the concrete `F`; the inspected
artifact names include `Pin<Box<…NativeCalls::call<…>>>`. Existing automatic boxing
therefore does not already discharge F01.

Compiler reports [#145527](https://github.com/rust-lang/rust/issues/145527),
[#140004](https://github.com/rust-lang/rust/issues/140004),
[#159943](https://github.com/rust-lang/rust/issues/159943) and
[#159944](https://github.com/rust-lang/rust/issues/159944) are investigation leads only.
They describe other optimization settings or earlier compiler phases; no exact-pin
regression diagnosis or toolchain replacement is established here.

## 4. Findings

Current findings remain in this source review until an implementation plan explicitly adopts them. They are **Deferred pending the requested follow-up design/implementation decision**; the revisit trigger is authorization to implement the Rust compilation-cost corrections. The existing persisted-execution coordinator continues to own PC acceptance. This review does not copy or close its findings.

### <a id="F01"></a>F01 — Record-specific future types specialize shared native task and streaming mechanics

**Priority: high.**

**Principles:** FP-01, FP-06, FP-07; DP-10, DP-16, DP-17, DP-20; **A4 violated**.

The application deliberately needs nominal record types and typed codecs. It does not need a separate implementation of native-call ownership and task scheduling for each record-specific future.

In [`native_calls.rs`](../../../crates/cpg-core/src/native_calls.rs), lines 25–54, `NativeCalls::call` is generic over both its result and its input future. That concrete future is captured by the async block passed to `tokio::spawn` at line 41. Boxing of the join future occurs later, at line 49. At that point the record-specific request has already entered Tokio’s generic task machinery.

The following callers expose this multiplication:

- [`workspace.rs`](../../../crates/cpg-core/src/workspace.rs), lines 1979–1997: `declare_async<R>` performs shared contribution registration, descriptor construction and native submission inside a record-generic async function.
- The same file, lines 1895–1926: `Writer<R>` combines typed encoding with synchronous/asynchronous native transfer.
- The same file, lines 2083–2108: `push<R>` carries a typed batch into an awaited native request.
- [`catalog_selection.rs`](../../../crates/cpg-core/src/catalog_selection.rs), lines 18–48 and 404–425: decoder inventories expand record-specific `load<R>` calls with distinct closures and repeated awaits; output inventories expand record-specific async registration.
- [`catalog_core.rs`](../../../crates/cpg-core/src/catalog_core.rs), lines 18–35 and 62–84, and [`catalog_evidence.rs`](../../../crates/cpg-core/src/catalog_evidence.rs), lines 52–70 and 93–112, repeat the same structural pattern.
- [`consumed_rows.rs`](../../../crates/cpg-core/src/consumed_rows.rs), lines 92–159: record and callback parameters remain attached through SQL construction, DataFusion planning, stream execution and iteration.

Distinct closure expressions add specialization dimensions beyond the record type. Multiple awaited specializations also expand the containing coroutine’s state and control flow. Generated source is therefore materially relevant even when its handwritten macro invocation is short.

The object evidence in §3 corroborates native task specialization. The source establishes the streaming and containing-future expansion route; it does not establish their individual share of compilation time.

The concrete-future submission shape already exists at HEAD `f9714734`; it predates
the dirty PC correctness repairs. Those repairs strengthen late-error retention and
typed completion. They are preservation obligations, not the cause to remove.

**Correction — Proposed.** Keep a thin typed boundary for applicability, exact input selection, writer construction, row validation and encoding. Move common contribution setup, native submission/task ownership and SQL stream iteration into shared implementations whose arguments already express the necessary runtime meaning.

The first narrow correction should erase the request future **before** it enters Tokio’s scheduler. A small generic adapter can convert the request to the established boxed-future representation and delegate to the shared submission implementation. Specialization by a small number of genuinely different result types can remain; specialization by every request closure should not determine the full scheduler implementation.

`NativeBridge` provides an inspected precedent: its `Request` is a `BoxFuture<'static, Result<(), ModelError>>` before `JoinSet::spawn`, at [`native_bridge.rs`](../../../crates/cpg-core/src/native_bridge.rs), lines 12 and 37–44. Reuse that library composition where it meets the asynchronous caller’s ownership contract; do not introduce another executor framework.

For streaming, retain `sql::query`’s read-only options and exact completed-input/epoch checks. A shared physical batch loop may receive a checked binding and a batch-level interpretation callback. The stage continues to own coverage and semantic interpretation. Do not move those meanings into a generic transport utility or replace typed model declarations with a second authored decoder registry.

**Critical preservation boundary.** `Batch<R>` owns typed rows, Arrow buffers and a reservation covering both representations; see [`record.rs`](../../../crates/lctx-model/src/domain/record.rs), lines 605–720. Current submitted futures retain that ownership. A correction that clones only the Arrow batch while leaving its charge with a cancellable caller is invalid: the caller could release the accounting reservation while submitted work remains live.

A shared request must own its charged arguments through acknowledgement and drainage. Feasible directions are boxing the complete charged request before scheduling, or a model-owned checked transfer conversion with an erased keepalive and correct reservation ownership. Select the simpler qualified direction. This review does not prescribe a new public transfer schema.

**Alternatives and limits.**

- Extract generic-independent setup into ordinary monomorphic helpers before adding any new type-erasure boundary.
- Box at batch/submission boundaries rather than adding per-row dynamic dispatch.
- Retain concrete generic row algorithms where specialization purchases production runtime performance.
- A crate split alone does not remove specialization if generic bodies remain instantiated by each caller.
- Merely moving macro expansions to another module, or adding `inline(never)` indiscriminately, does not settle the ownership or containing-coroutine problem.

The remaining large stage futures may need coherent function boundaries after shared submission work is removed. Do not claim that the first transport correction necessarily eliminates every serial backend tail.

**Closure evidence.** Inspect the revised source and representative emitted symbols to establish that record-specific futures no longer instantiate the complete native task-management path. Run focused actual cancellation/late-failure, charged-transfer, output-completion and completed-input controls. Preserve read-only SQL, exact epochs and independent semantic expectations. Qualify any consequential change to runtime allocation/dispatch over representative production operations before claiming production performance preservation. Compilation/runtime timings, if later requested, must keep source, profile and conditions explicit.

### <a id="F02"></a>F02 — Extraction provenance reverses rebuild locality by watching the whole workspace

**Priority: high.**

**Principles:** FP-01, FP-06, FP-07; DP-09, DP-17, DP-21; **A1 and A4 violated**.

[`scripts/producer_fingerprint.rs`](../../../scripts/producer_fingerprint.rs), lines 38–65, walks every workspace crate manifest, source directory and build script, plus root manifests/lockfile/toolchain, `third_party` and `specs`. Line 53 explicitly states that unused siblings remain included.

[`cpg-extract/build.rs`](../../../crates/cpg-extract/build.rs), lines 8–12, turns this whole-workspace source closure into a `cargo:rustc-env` value. [`bundle.rs`](../../../crates/cpg-extract/src/bundle.rs), lines 29–49, incorporates that value into provider build identity.

Consequently, changing serving or compiler-coordinator source can rerun the extraction build script and change the extraction crate’s embedded environment value even when extraction’s Rust implementation dependencies have not changed. Normal Cargo dependency direction does not contain this watch relationship. An upstream extraction rebuild can then propagate to its ordinary downstream consumers.

This is more than hashing overhead. Provenance membership causes compiler invalidation across a boundary whose semantic responsibility did not change.

Complete producer provenance remains necessary. The defect is using every workspace production source as the producer’s closure and embedding that closure at an upstream owner.

**Correction — Proposed.** Define the relevant producer implementation closure at the provider/extraction boundary, including the model, analyzer fork/dependency revisions and explicitly used runtime assets that can affect that producer. Keep source membership/deletion tracking, deterministic relative paths and exact captured configuration.

If complete compiler-composition or executable provenance is required, construct that identity at the composition/executable owner. It may include downstream processing and native ingestion semantics, but it should not require injecting all downstream source changes into the extraction crate’s own compiled identity.

The declaration of embedded runtime scripts already supplies a useful ownership route. Use that and actual dependency/source ownership rather than an independent hand-maintained “all important files” list.

**Revealing legitimate case.** A native ingestion change could alter a completed contribution while provider-emitted typed rows remain unchanged. Narrowing provider provenance must not erase the compiler composition responsible for that effect. Preserve separately scoped implementation identities and their existing captured bindings; do not equate a narrow provider digest with the identity of every operation performed on its output.

**Alternatives and limits.**

- A conservative relevant closure is preferable while a dependency’s effect is uncertain.
- Retaining root lockfile sensitivity can be an interim conservative choice; unrelated dependency changes would continue to rebuild affected provenance consumers.
- Cargo dependency resolution is a useful source of closure membership, but runtime assets and patched sources require their declared owners.
- Removing provenance or substituting semantic compatibility for build identity is unacceptable.
- Keeping a whole-executable identity as separately captured provenance is viable if it has a real consumer and does not redefine provider identity.

The existing model fingerprint is a different case. [`lctx-model/build.rs`](../../../crates/lctx-model/build.rs) watches the model, its macros and dependency manifests/lockfile; [`model.rs`](../../../crates/lctx-model/src/domain/model.rs), lines 286–292, explicitly describes implementation provenance separate from compatibility. Own-model implementation changes are expected to affect that provenance. This review does not report broad own-model identity as the same defect merely because it is expensive.

**Closure evidence.** Mutation controls should show that an unrelated serving implementation edit leaves the extraction/provider identity and extraction compilation inputs unchanged, while provider source, model dependency, analyzer revision, runtime asset membership and relevant composition changes affect their proper identities. Preserve relocation stability and source-deletion detection. The existing [`producer_fingerprint.rs` test](../../../crates/cpg-extract/tests/producer_fingerprint.rs) is the starting control, but its all-workspace membership expectations must change with the contract.

### <a id="F03"></a>F03 — Ordinary test compilation has no independently qualified local-loop policy

**Priority: medium, with an early configuration decision.**

**Principles:** FP-05, FP-06, FP-07; DP-10, DP-22, DP-23; **A4 violated**.

[`Cargo.toml`](../../../Cargo.toml), lines 139–164, selects workspace O2 and imported O3 for dev/release. `[profile.test]` only sets `debug = 0`; the user’s bare `cargo nextest run --workspace --no-fail-fast` therefore does not select an inexpensive unoptimized workspace test loop. Repository verification explicitly selects `--release` through [`scripts/verify.py`](../../../scripts/verify.py), line 46 and lines 505–513.

These settings buy optimized runtime execution, which is useful for production acceptance and potentially for long native journeys. They also apply optimizer work to frequent local compilation of the generic structure in F01. The selected policy does not distinguish “learn quickly from this revealing contract control” from “exercise optimized production behavior.”

The operator has explicitly allowed different local test settings while preserving production runtime performance. Keeping one optimization policy for both workloads is therefore not justified solely by the current release-test rule.

**Correction — Proposed.** Introduce a named local-test policy or an explicit default-test override, with production release settings and optimized acceptance retained. The simplest direct candidate is a lower workspace test optimization level while preserving optimized stable imported dependencies. A named Cargo profile is preferable if it makes selection and evidence provenance clearer without adding a new launcher framework.

O1 and O0 are candidates, not delivered fixes. O1 can retain useful optimization while avoiding part of the O2 optimizer workload; O0 is the simpler compilation candidate. Exact compiler behavior and runtime consequences must decide between them.

**Material counterevidence.** The existing [persisted-execution coordinator §9.1](../../plans/persisted-graph-execution-plan_2026-10-07.md) records temporary core-only O0 controls with stack aborts, including test-thread and CLI-main failures, and later child-only enlarged-stack reruns. Relevant rows are at lines 505, 518 and 522; lines 533–539 explicitly distinguish the temporary core O0 configuration from committed production settings.

These receipts do not prove O1 fails. They do disprove a presumption that lowering optimization is automatically operationally interchangeable for the current large futures. `RUST_MIN_STACK` also does not enlarge a CLI main thread’s OS stack.

Do not hide this with a permanent oversized-stack default and call the profile qualified. F01’s structural correction may be a prerequisite for a useful lower-optimization loop where large coroutine entry frames remain.

**Alternatives and limits.**

- Keep optimized dependencies and reduce optimization only for volatile workspace crates.
- A separate local profile with `lto = "off"` can remove local ThinLTO while retaining ordinary LLVM optimization. `false` and `"off"` have different semantics. Package overrides cannot select LTO. This candidate requires the same runtime/stack qualification and does not remove generic collection or all optimizer work.
- Inspect effective proc-macro/build-script overrides: wildcard imported-package O3 can outrank the O2 build override. Lower optimization may reduce their compilation cost while increasing macro execution time; the net benefit is unresolved.
- Use lower optimization for focused finite controls, with optimized native journeys retained.
- Keep release as the default until the local policy’s revealing stack/runtime cases pass, while exposing the proposed policy explicitly for qualification.
- Do not change overflow checks or debug assertions unintentionally when selecting inheritance.
- Do not treat nextest’s execution profile as Cargo’s compilation profile.
- Do not infer runtime performance improvement from faster compilation.

**Closure evidence.** Show the resolved Cargo profile for normal workspace libraries, test harnesses, imported dependencies and proc-macro/build-script units. Run a minimal revealing set containing the known stack-sensitive driver/main cases and affected finite controls. Preserve optimized production acceptance separately. Report the local profile with every receipt; a successful local control does not establish the production runtime boundary.

### <a id="F04"></a>F04 — Integration-file boundaries repeatedly compile substantial common drivers

**Priority: medium.**

**Principles:** FP-01, FP-06, FP-07; DP-10, DP-17, DP-23; **A1 and A4 violated**.

The extraction integration suites repeatedly include [`tests/typed_driver/mod.rs`](../../../crates/cpg-extract/tests/typed_driver/mod.rs) through `#[path]`. That driver contains capture, native provider composition, workspace creation, actual facts compilation and generic inspector entry points; see lines 141–254.

Core integration suites similarly include [`tests/fixtures/catalog_runtime.rs`](../../../crates/cpg-core/tests/fixtures/catalog_runtime.rs). Its 287 lines contain actual compiler setup, asynchronous compilation and generic SQL result decoding. Examples include `catalog_core.rs`, `catalog_evidence.rs`, `catalog_selection.rs`, `structural.rs`, `analytic.rs` and `behavioral_frontiers.rs`.

These modules are shared as source, not as a compiled library boundary. Each top-level integration file is a separate executable compilation unit. Common non-generic driver code, used generic wrappers, coroutine bodies and linking are consequently repeated across targets. Unused generic helpers need not all be emitted; the defect does not depend on claiming they are.

A driver edit also invalidates many integration targets because the same source is incorporated into each. Logical test-topic separation is useful, but it does not require one substantial compiled driver copy per topic.

**Correction — Proposed.** Group integration targets around coherent verification boundaries, with topic tests retained as modules, or introduce a small compiled test-support boundary for substantial monomorphic orchestration where its dependency direction is workable.

The simplest candidate is grouped harnesses in the existing test tree. Keep small typed assertion/decoding adapters local when they genuinely differ. A test-support crate is justified only if it creates a useful compiled reuse boundary and avoids new public production APIs or problematic dependency cycles.

Nextest can still execute independently selected tests from a grouped binary. Grouping should not merge test state, weaken fixture ownership or remove independent expectations.

**Alternatives and limits.**

- Retain small separate targets for genuinely isolated controls that do not need the full driver.
- Share monomorphic orchestration while leaving lightweight generic assertion helpers in callers.
- Consolidating everything into one enormous harness would sacrifice compilation locality and can create another skewed build unit. Select coherent groups.
- Test-name filtering selects runtime tests; use explicit Cargo target selection to avoid compiling unrelated executables where appropriate.
- Existing `verify.py::narrow`, lines 398–438, already supports named-target narrowing. Preserve and update that route rather than creating another selection authority.

**Closure evidence.** Compare target inventories and dependency/source inclusion before and after the correction. Trace a driver edit and a test-only edit to their intended compilation units. Check nextest discovery/filter mappings, verification selections, independent fixture lifetimes and unchanged assertions. Full suite behavior remains an acceptance obligation; changing physical harness boundaries does not justify deleting semantic controls.

## 5. Change and failure scenarios

| Scenario and kind | Expected owner | Current consequence | Corrected direction and preservation |
|---|---|---|---|
| Add a nominal record consumed by an existing stage — domain extension | Model declaration, genuinely new codec/domain behavior, stage input/output declaration | Macro-expanded async calls can pull shared task/stream machinery into additional specializations | Typed codec and small adapter grow; common native scheduler and stream mechanics remain shared |
| Change serving implementation — mechanism change | Serving and its actual consumers | Whole-workspace extraction fingerprint also changes | Relevant provider identity stays stable; composition identity changes only where required |
| Change native ingestion without changing provider rows — mechanism change | Native ingestion/compiler composition | Current broad closure notices it, but attributes it through upstream provider compilation | Preserve the responsible composition identity separately; do not erase actual output-affecting provenance |
| Run a focused finite control — test-instance selection | Existing verification boundary | Runtime filtering can still compile many executables and O2 workspace code | Explicit Cargo-target narrowing and qualified local profile |
| Substitute concrete submission with batch-level erased request — execution mechanism | `NativeCalls` / checked transfer owner | Current request type reaches Tokio task internals | Preserve result, cancellation, late-error reporting, reservation lifetime and drain semantics |
| Cancel caller after native submission — failure | Submitted-call owner | Current future retains charged batch and task ownership | Shared/erased request must retain both charge and arguments until terminality |
| Lower workspace test optimization — compilation policy | Root profile and verification reporting | Known O0 entry-frame failures challenge interchangeability | Qualify stack-sensitive controls; keep production settings and evidence distinct |
| Edit common test driver — test mechanism | Test harness/support boundary | Many independent executables recompile included source | Compile common orchestration once per coherent reuse boundary |

A native compiler-request substitution is credible here because physical transport already accepts runtime relation descriptors and Arrow batches. It does not require weakening the typed model or supporting another store.

## 6. Libraries, alternatives and preservation

The workspace already uses the right established categories of capability: Cargo/rustc for builds, sccache for compatible artifacts, Clang/mold for linking, Tokio/futures for asynchronous execution, Arrow/DataFusion for transfer/compute, and generated typed model declarations for correctness. No clearly suitable library removes the application’s responsibility for charged native-call ownership or producer attribution.

The library-first correction is to expose the appropriate physical inputs to those libraries without specializing their full machinery unnecessarily.

| Alternative | Benefit mechanism | Burden and limit | Judgment |
|---|---|---|---|
| Current baseline | Concrete typed futures maximize specialization; one optimized loop resembles runtime acceptance | Emitted task duplication, broader invalidation, repeated harness compilation and expensive local optimization | Revise |
| Thin typed adapters plus shared async mechanics | Removes record/closure specialization from common scheduling and stream operations | Requires correct checked transfer and cancellation ownership; batch-level allocation/dispatch effects need qualification | Preferred structural direction |
| Qualified lower-optimization local test policy | Avoids optimizer work in the frequent local loop | Separate artifacts, evidence labeling, possible stack/runtime differences; production acceptance still required | Preferred complementary policy, candidate settings unresolved |
| Grouped integration harnesses | Reuses compiled driver code and reduces separate executable/link work | Updates target/filter mapping; overly broad groups can hurt locality | Preferred test-structure direction |
| Compiled test-support crate | Reuses monomorphic orchestration across targets | Adds dependency and API boundary; must avoid moving production concerns into test utilities | Viable when grouping is insufficient |
| More worker flags or codegen units | May expose remaining independent work | Does not remove generic expansion or split a giant function; cache identity changes | Tuning candidate only after structural causes |
| Global non-incremental workspace plus sccache | Enables caching for eligible repeat compilations | Loses incremental edit reuse; does not help changed-source misses or linking | No blanket recommendation |
| Arbitrary crate splitting | Can create separate compilation scheduling/reuse boundaries | Can merely relocate specialization, expose internals and increase linking | Require a concrete responsibility/rebuild boundary |
| Manual scheduler or worker caps | Changes admission/scheduling | Adds policy ownership and does not remove the diagnosed work | Excluded; no demonstrated requirement |
| Blanket dynamic rows or removal of typed codecs | Reduces some specialization | Weakens hot-path performance and model guarantees | Reject |

Preserve:

- production O2 workspace/O3 imported settings until a deliberate replacement is qualified;
- exact dependency/toolchain pins and native analyzer source-family boundaries;
- existing incremental/shared build state, benchmark captures and compatible sccache reuse;
- model-owned semantic identity, append-only codebooks, shared validators and generated codecs;
- exact immutable completed views and read-only SQL;
- typed row/Arrow reservation accounting;
- submitted-call acknowledgement, late-failure reporting and drain ownership;
- independently owned disposable fixtures and independent test expectations;
- existing readiness/selection/reporting routes;
- dirty PC1–PC5 work and its separate acceptance obligations.

## 7. Independent judgments and gates

### Architectural judgments

| Judgment | Verdict | Evidence and boundary |
|---|---|---|
| A1 — Localize change | **Violated** | F02 causes downstream/unrelated source edits to invalidate upstream extraction; F04 repeats a common driver source across many executable units |
| A2 — Encode domain meaning explicitly | **Satisfied within scope** | Typed records, exact completed-source identities, separate semantic compatibility/provenance and charged submitted-call ownership govern the inspected operations |
| A3 — Extend through composition | **Satisfied within scope** | Stage and model inventories compose owned operations; the native libraries discharge the relevant physical capabilities. Their excessive specialization is independently an A4 defect |
| A4 — Fit execution to supported workload | **Violated** | F01’s actual task specialization, F02’s invalidation scope, F03’s undifferentiated local optimization and F04’s repeated build units amplify normal edit/test work |

Applicable foundation/supporting judgments:

- **FP-01 and FP-06 violated** at the provenance and repeated-harness boundaries.
- **FP-02, FP-03, FP-04 and FP-05 satisfied within the inspected typed/ownership contracts.** This is not an acceptance of every pending PC implementation path.
- **FP-07 violated** by the demonstrated compiler-work amplification.
- **DP-09, DP-10, DP-16 and DP-17 violated in the finding-specific architectural sense.** F02 is over-invalidation, not an established unsound cache hit.
- **DP-18, DP-19, DP-20 and DP-23 are preservation obligations for the proposed corrections.** The inspected read-only and submitted-ownership mechanisms support them; no new full runtime acceptance is claimed.
- **DP-22 satisfied by this bounded evidence treatment.** Diagnoses, candidate remedies and unmeasured benefits remain distinct.

### Correctness/fidelity gates

The verdicts concern the inspected build-cost boundary, not the whole product.

| Gate | Verdict | Scope evidence or reason |
|---|---|---|
| G1 — Authority | **Pass, scoped static** | Root configuration owns build settings; model declarations own record meaning. F02 is excessive dependency scope, not two competing semantic definitions |
| G2 — Semantic fidelity | **Pass, scoped static** | Inspected typed records/codecs and completed-input identities preserve consequential distinctions. Remedies must retain them |
| G3 — Validity | **Pass, scoped static** | Shared constructors/validators and typed decoding remain governing boundaries; no recommendation bypasses them |
| G4 — Hidden behavior | **Pass, scoped static** | SQL helper refuses writes; build-script effects are explicit. No operator state was inspected |
| G5 — Consistency/recovery | **Pass for inspected build ownership; product acceptance not assessed** | Cargo owns shared compilation state; native requests explicitly retain task/charge ownership. PC completion remains separately open |
| G6 — Transformation/reuse | **Pass for current build policy, scoped static** | Compatible compilation identity is tool-owned; over-invalidation does not establish incorrect reuse. Lower-profile and transport remedies remain Proposed and require their stated qualification |
| G7 — Truthful capability claims | **Pass, scoped** | No compile-speed or dominant-pass claim is inferred from worker counts/object sizes; no current-run tests are claimed |
| G8 — Library leverage | **Pass, scoped** | Established build/compiler/task/stream libraries are used. Excessive specialization does not establish that a library can replace the application’s semantic ownership |
| CI-G1 — Fidelity | **Not applicable to a new product behavior; preservation assessed** | No extracted relation or claim meaning is changed. Attribution remains required by F02 |
| CI-G2 — Evidence closure | **Not applicable** | No served claim/evidence path is changed or certified |
| CI-G3 — Evaluation integrity | **Not applicable** | No evaluator, gold/heldout input or comparison meaning is changed |

The architectural failure remains material despite those scoped correctness gates. Working outputs and strong semantic contracts do not offset avoidable compiler-work amplification.

## 8. Verification and remaining uncertainty

| Claim | Evidence and date | Outcome/boundary |
|---|---|---|
| Native request specialization reaches Tokio task machinery | Source inspection of `NativeCalls::call`, 2026-10-08 | **Implemented**; no new runtime check |
| Substantial emitted native task specialization exists | Coordinator’s completed-object `llvm-nm` inspection, 2026-10-08 | Observed per-object symbols; not timing or globally unique totals |
| Broad source closure invalidates extraction provenance | Fingerprint/build-script source and existing mutation-control design, inspected 2026-10-08 | **Implemented**; no new mutation run |
| Profile inheritance and pinned compiler/sccache behavior | Exact pinned source/document qualification by library researcher | **Interface-checked**; no candidate profile build |
| O0 can encounter stack failures | Dated coordinator §9.1 receipts | Historical evidence; not fresh qualification of O1/O0 |
| Proposed transport/stream correction preserves behavior and runtime performance | Proposed ownership argument and closure controls | **not_run**; implementation does not exist |
| Proposed local profile is usable | Candidate settings and known stack countercase | **not_run**; setting unresolved |
| Existing full product/PC acceptance | Current STATUS/coordinator | Separately pending; not certified by this review |

No Rust builds, product tests, compile probes, synchronization or running-process interventions were performed for this review. Documentation publication checks and scoped turn-end maintenance are recorded in STATUS separately.

Remaining uncertainties affect different decisions:

1. **Dominant current backend pass/body.** Current incomplete-object identity and phase evidence do not establish it. This uncertainty limits the immediate-tail attribution, not F01’s demonstrated specialization defect.
2. **Runtime overhead of the selected erasure boundary.** Batch-level erasure is credible; allocation/dispatch effects depend on the implementation and request workload. Qualify before claiming preserved performance.
3. **Local O1 versus O0.** Known O0 stack failures make this a real selection question. Structural future boundaries and revealing stack-sensitive cases should settle it.
4. **Exact producer closure granularity.** Preserve complete result-affecting producer/composition provenance before narrowing membership. Conservative relevant closure is acceptable during that determination.
5. **Best harness grouping.** Existing verification families provide candidate boundaries; target inventories and edit locality should select them. One giant harness is not the objective.
6. **Compiler regression.** No exact-pin regression was established. Superficially related reports from different optimization levels or earlier compiler phases remain investigation leads, not diagnoses.

No further probe is necessary to decide that the architecture needs revision. Targeted qualification is necessary to select and accept the remedies.

## 9. Rule impacts

These are required changes to current rules if the proposed design is selected. They are not applied by this review.

### <a id="RC01"></a>RC01 — Separate local test compilation from the universal release-test rule

**Current rule:** ADR-0136’s retained release-test workflow; DESIGN §1.2’s “existing release test workflow”; AGENTS.md’s release-test/release-profile verification instructions; `scripts/verify.py`’s explicit `--release` command.

**Proposed change:** Preserve optimized production/acceptance controls, while permitting an explicitly named and qualified local test compilation policy. Define its selection and reporting through existing verification/Cargo routes. If the default `[profile.test]` changes, record that the normal workspace library compilation used by tests changes too.

**Dependent recommendation:** F03; complements F01.

**If retained:** Implement the structural findings first and continue optimized focused compilation. A lower-optimization local loop remains unavailable as an ordinary accepted route.

### <a id="RC02"></a>RC02 — Replace all-workspace provider source membership with scoped provenance ownership

**Current rule:** `scripts/producer_fingerprint.rs`, lines 38–65, especially the explicit inclusion of unused siblings at line 53; `cpg-extract/build.rs`’s embedded source-closure digest; existing fingerprint mutation expectations.

**Proposed change:** Provider identity tracks its complete relevant implementation closure. Composition/executable identity is captured by the owner of that composition where it has a consumer. Retain complete configuration/source attribution, relocation stability and membership/deletion tracking.

**Dependent recommendation:** F02.

**If retained:** Serving/coordinator/unrelated workspace changes continue to invalidate upstream extraction provenance and may trigger its rebuild. F01/F03/F04 remain independently useful, but F02 remains unresolved.

**ADR-0135 impact:** No change to its separation of supplier semantics from executable provenance, captured binding obligations or cold admission. Narrowing provenance scope must honor those distinctions. A later design that instead changes their meaning would need a separate explicit rule impact; it is not this recommendation.

F01 and F04 are implementation/design corrections within the preserved semantic and lifecycle contracts. They do not inherently require changing the exact pins, worker-default rule, read-only SQL rule, batch charging or operation-completion contracts. Any new public transfer contract or substantial ownership move should be assessed through the existing ADR/design route rather than assumed by this review.

## 10. Ranked follow-up and decision

| Priority | Direction and owner | Findings | Closure obligation |
|---|---|---|---|
| 1 | Correct native request specialization at `NativeCalls`, then inspect typed registration/stream boundaries | F01 | Shared scheduling machinery; charged cancellation/drain behavior preserved; production runtime effects qualified |
| 2 | Align provider/composition provenance with actual ownership | F02 | Unrelated downstream edits stop changing extraction identity; relevant implementation changes remain captured |
| 3 | Select a qualified local-test policy through root Cargo/verification owners | F03 / RC01 | Resolved profile evidence, stack-sensitive controls and separately retained optimized acceptance |
| 4 | Group integration harnesses or share substantial monomorphic test support | F04 | Smaller appropriate build-unit inventory, stable test discovery/filtering and unchanged independent expectations |
| Later, conditional | Tune remaining codegen partitioning/toolchain settings using observed residual behavior | §3 uncertainty | A concrete remaining cause and compatible cache/profile evidence; no new scheduler or caps |

This is consequence ranking, not a rigid implementation sequence. The local-profile decision is cheap to formulate early, but a usable lower-optimization loop may depend on correcting large async boundaries. Provenance scope can progress independently. Harness changes should follow stable verification-boundary selection so that they do not create another oversized compiler unit.

**Bounded decision: Revise.** The inspected Rust compilation-cost architecture does not fit the ordinary edit/test workload because it repeatedly specializes shared asynchronous mechanics, invalidates upstream extraction for unrelated downstream changes and recompiles common test drivers across many executables. The optimized local loop adds a separate avoidable cost under the operator’s selected development requirements.

**Enclosing architectural status:** Product semantics, persistence, serving, PC1–PC6 completion and release qualification are not accepted or rejected by this compiler-cost review. Their current owners and evidence remain authoritative.

The next consequential action is a root-owned implementation plan that selects the charged native-request boundary, scoped provenance identity and local-profile policy, presents RC01–RC02, and retains the production/runtime and independent-assurance constraints above.
