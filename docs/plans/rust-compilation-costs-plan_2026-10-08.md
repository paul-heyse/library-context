# Rust compilation costs: shared work, scoped provenance and the local test loop

**Accepted target / implementation in progress · 2026-10-08.** ADR-0137 records confirmed choices. Current verifier defaults remain release until BC3 qualification; no complete product acceptance is established.

## 1. Outcome, basis and ownership

Make ordinary Rust edits and focused/full test compilation do materially less avoidable work while preserving optimized production behavior. The [source review](../design_review/reviews/design_review_rust-compilation-costs_2026-10-08.md) establishes four structural causes, F01–F04; it did not identify the dominant pass in the unfinished LLVM module or measure a speedup. The interrupted diagnostic capture in §12 now identifies a dominant source-attributed LLVM hotspot; it still does not measure a speedup. This plan develops all four corrections, their interacting preservation requirements and the review's six remaining uncertainties.

Inspected baseline: main `8511eb5fe27498815ce4433fd2d0029aa2e6016e`, plus the dirty integrated PC1–PC5 implementation, 2026-10-08. No clean-tree assumption or rollback to an earlier coordinator checkpoint is made. Existing compiler/native/library evidence is retained with its original date and boundary. Source inspection and pinned-tool research establish a credible target; implementation must still establish correctness and runtime adequacy.

This document owns compilation-cost sequencing and the sole scheduled disposition of source-review F01–F04 in §9. Architectural owners retain enduring contracts: [DESIGN §1.2](../design/DESIGN.md#section-1-2), [acquisition/extraction §4](../design/sections/acquisition-and-extraction.md) and [semantic model §15](../design/sections/semantic-model.md). The [persisted-execution coordinator](persisted-graph-execution-plan_2026-10-07.md) continues to own PG/PC completion, its source findings and native/CLI/MCP acceptance. Its [correction plan](persisted-execution-corrections-plan_2026-10-07.md) supplies captured bindings and complete operation outcomes; accepting this plan does not close PC6.

The [coroutine companion](rust-coroutine-compilation-amplification-plan_2026-10-08.md) develops
the subsequent coroutine review's F01 correction within BC1: structural first, then mandatory
assessment and correction of confirmed recurrence across named workspace owners. Its CU0–CU6
packages consume these contracts without another finding-status ledger. This coordinator retains
the combined completion boundary; no new default, compiler/toolchain change or native acceptance
is implied by authoring the companion.

Plan authoring changes documentation only. During implementation, preserve current source repairs, exact pins, shared caches/intermediates, available compiler/test parallelism, disposable fixture ownership, operator state and protected evaluation inputs. Do not routinely clean Cargo caches, add job/thread caps, synchronize a live native environment, or interrupt another running build. A performance campaign, toolchain upgrade and real-library pilot are not prerequisites to correcting the demonstrated architecture.

## 2. Confirmed rule changes and target defaults

The operator explicitly confirmed these choices in the plan-creation conversation on **2026-10-08**. The similarly numbered rule impacts in older catalog/persisted reviews are different decisions.

| Source item | Confirmed choice | Route before dependent implementation |
|---|---|---|
| Compilation-cost review RC01 | **Accepted:** qualify local-test compilation separately from optimized production acceptance. | BC0 records [ADR-0137](../adr/0137-qualified-local-compilation.md), superseding ADR-0136, and updates DESIGN §1.2, AGENTS testing guidance and the [review binding §4](../design_review/design_principles/binding/library-context.md#4-where-findings-land) release-profile instruction with the transition rule. Carry forward all unaffected build, pin, fixture and available-parallelism decisions; never rewrite the accepted ADR body. BC3 changes the actual default only after qualification. |
| Compilation-cost review RC02 | **Accepted:** provider provenance covers its relevant implementation closure; composition provenance belongs at its actual owner. | BC0 records the complementary §4/§15 provenance decision in ADR-0137. BC2 narrows provider compilation inputs and captures native execution composition at the workspace owner. Preserve ADR-0138's semantic/captured/executable distinctions. |
| Ordinary local-test selection | **Selected:** qualified local settings become the bare Cargo/nextest default. | BC3 initially uses a temporary named candidate, then installs the qualified settings in `[profile.test]`, migrates verification selection/reporting and retires the candidate. Release acceptance stays explicit. |

The selected first candidate is **workspace O1, incremental, debug information disabled, LTO explicitly off, debug assertions and overflow checks enabled**. Imported O3/non-incremental and existing build overrides remain initially unchanged. O0 is not the selected fallback: historical O0 test-thread and CLI-main stack failures make an automatic switch unjustified. An O1 failure requires correcting the exposed implementation issue and rerunning its revealing controls before changing the default.

Policy acceptance does not establish qualified settings. BC0 may declare the accepted target while clearly labeling implementation as Proposed; it must preserve the current release route until BC3 completes.

## 3. Foundation assessment and combined design

Assessment uses core/template3.3, heuristics1.0, code-intelligence1.5 and the repository binding. Typed model ownership, exact completed views, shared validators, independent expectations and explicit native drainage remain useful foundations. Physical compilation units need not mirror every record distinction. Relevant patterns are shared execution after typed admission, relevant dependency propagation and compiled reuse of stable test orchestration.

The corrections address different kinds of work. Scoped fingerprints avoid upstream recompilation on unrelated changes. Shared request/stream mechanics reduce the code instantiated when compilation is necessary. The local profile avoids optimizer work inappropriate to frequent revealing controls. Harness groups reuse common orchestration within a compiled unit. None replaces Cargo/rustc scheduling, and none promises that every remaining function can occupy every CPU.

### 3.1 Shared native requests and checked streaming — F01

`NativeCalls::call` currently carries each concrete future into `tokio::spawn`, then boxes its join future afterward. `NativeBridge` already erases requests before `JoinSet::spawn`; reuse that composition pattern while retaining each owner's distinct lifecycle. Do not merge their runtimes or add another executor.

**Selected boundary:** keep a thin typed result adapter and one monomorphic submitted-request path. The adapter accepts the owned future, creates its typed acknowledgement, boxes the complete request before scheduler entry and returns a boxed caller-await future. The shared submission path owns registration, admission, submitted-call guards, spawn/join bookkeeping and drainage. Its request result uses the existing typed/shared error representation. A failed operation must mark the native attempt/cancellation before acknowledgement; a failure whose receiver disappeared remains visible to drainage. Successful acknowledged calls can be retired; failed or unfinished calls cannot disappear merely because their finished flag is set.

Keep small result-specific adapters where typed channels require them. Their generic work must not include the full Tokio task, submission or drainage implementation. Boxing `Pin<Box<F>>` without erasing `F`, or boxing only the join handle, does not deliver this boundary.

**Charged arguments:** move the complete `Batch<R>` into every submitted write future, then erase that future. Do not clone only Arrow and leave the reservation with the waiting caller. Static authoring inspection found that `Writer::write_arrow` currently clones an Arrow batch while `NativeBridge::call` can return on cancellation; provider sink adapters also turn an owned batch into a borrowed `ProducerOutput` call. This is an additional revealing ownership case within F01's correction scope, not a reason to remove the PC completion repairs.

Change `ProducerOutput::write` and `contribute` to consume an owned checked batch. `ProviderSink` already supplies ownership; preserve it instead of borrowing. Migrate direct Rust callers and tests, synchronous flush, asynchronous push and final close together. Inspect rows before transfer where an assertion needs them; do not add a compatibility borrowing path or duplicate charge to hide the ownership issue. No new public erased-row model or transfer wire schema is needed.

Extract record-independent contribution descriptor construction and registration into one implementation used by synchronous and asynchronous registration and empty-output completion. Retain single registration, declared output enforcement, writer insertion synchronization and refusal behavior. A thin typed adapter establishes `R`'s model/allowed-output contract and invokes that common operation. Preserve ordering between registration and typed writer visibility.

For completed-input streaming, keep exact typed permit/declaration matching and owner-selected SQL at the typed boundary. Share SQL ordering, read-only planning, stream execution, batch iteration, terminal errors and yielding in a monomorphic physical loop with a borrowed batch-level callback. That callback retains the typed permit and stage-owned coverage/decoding policy. No per-row dynamic dispatch or independently authored decoder registry is introduced.

Migration covers every `NativeCalls` caller, workspace registration/write/completion path and `stream_at`/`stream_where_at`/`stream_query_at` consumer, including artifact admission, C0/C1/C2/S0/E0, structural/analytic/behavioral stages and their generated inventories. Genuinely record-specific codecs and row algorithms remain concrete. Where a large stage future remains after sharing these mechanics, split coherent load/compute/emit operations using boxed stage boundaries; do not mechanically split every macro or impose blanket `inline(never)`.

The allocation/dispatch change occurs per request/batch, not per row. Native transport already has asynchronous ownership and crossings, but production runtime preservation still requires the focused operational qualification in §7.

**Proposed coroutine extension, 2026-10-08:** sharing the physical stream does not contain an
enclosing typed await inventory. The [companion §3/§4](rust-coroutine-compilation-amplification-plan_2026-10-08.md#3-target-operations-and-internal-interfaces)
selects borrowed coherent phases and inventory-derived ordered loaders, including nested scoped
readers, children and drops. Structural is first; confirmed summary/model, selection/synthesis,
retrieval, analytic/semantic-execution and normalization patterns follow with their actual consumers.
Retain stage-owned selection/cardinality policy and typed leaves. Bulk output is conditional on
residual evidence and buffering/cancellation contract design, not a prerequisite API replacement.

### 3.2 Provider and native execution provenance — F02

**Provider closure:** root source enumeration at `cpg-extract`, following declared normal/build local dependencies. Current local members are extraction, `cpg-flow`, `lctx-model` and `lctx-model-macros`; core/store/analytics are extraction dev-dependencies and do not become provider production roots. Preserve declared runtime scripts, their membership declaration, analyzer patch bytes, toolchain and locked external revisions.

Generalize the existing fingerprint helper to accept an owning crate root. Parse Cargo manifests with the already pinned `toml` crate, adding that same workspace pin as a build dependency where needed. Follow normal/build path dependencies, inherited workspace declarations and target-specific tables conservatively; exclude dev/test roots. Include relevant local patched-source roots conservatively. Do not spawn Cargo recursively from a build script or implement a second feature/version resolver. Cargo and the lockfile retain resolution authority.

Walk owned production source/assets with deterministic workspace-relative paths, membership and deletion watches. Retain whole root manifest/lockfile sensitivity initially: unrelated dependency-policy edits may still invalidate provenance, a deliberate conservative limit. Stop walking all workspace crate sources and all unrelated sibling assets. Reuse the existing runtime-script declaration rather than creating another maintained important-files inventory. Pin/patch changes remain deliberate; no wholesale re-resolution is required.

**Composition closure:** add a compiler-owned build fingerprint rooted at `cpg-core` and its normal/build local dependency/asset closure. This covers native ingestion and model realization dependencies without depending on serving/publisher-only sources. Define one domain-separated execution hash from the supplied stage implementation and this composition digest at `Workspace::output`. All ordinary workspace-produced contributions, including stage, normalization, semantic and embedding outputs, use it because they share native ingestion. `Workspace::producer` continues to consume the declaration and its captured binding.

Retain raw scoped provider hashes in `Provider.build_digest`, supplier IDs and `Stage.code`. Store the composed execution identity in the existing `ContributionSpec.implementation`; do not relabel it as semantic compatibility. The descriptor, artifact producer inventory and cold comparison already consume that opaque hash. Contribution/view identities propagate it to exact completed dependencies; `SourceSnapshot` attribution remains with its documented contribution/view owner.

Copied/imported completed specifications retain their captured identity verbatim. Never replace them with the running compiler's composition. The existing fields can carry this distinction without adding redundant provenance columns or bumping the transport format solely for this hash. New ordinary attempts produce new identities; rebuild owned disposable acceptance artifacts from pinned inputs and retire replaced scratch under the existing cutover policy. Operator databases and historical failure receipts are not rewritten.

A serving-only source edit leaves provider identity and extraction build inputs stable. A native-ingestion-only edit leaves raw supplier identity stable but changes produced contribution execution identity. Relevant provider/model/analyzer/runtime-asset edits still affect their proper scopes. Native schema/definition realization identity remains separately owned; it is not evidence of compiler-ingestion provenance.

### 3.3 Qualified local-test compilation — F03

First expose `local-test-candidate`, inheriting `test`, with the following overrides. After its qualification, put these values in `[profile.test]` and remove the temporary profile:

```toml
opt-level = 1
lto = "off"
incremental = true
debug = 0
debug-assertions = true
overflow-checks = true
```

| Compilation unit | Selected local policy |
|---|---|
| Workspace normal libraries, binaries and test harnesses | O1, incremental, LTO off |
| Imported dependencies, including nonmember path dependencies | Existing O3, non-incremental, LTO off |
| Workspace proc macros/build scripts | Existing build-override O2; inherited incremental setting; LTO off |
| Imported proc macros/build dependencies | Existing wildcard O3, non-incremental, LTO off |

Do not duplicate inherited override tables or change dev/release, CGU counts, compiler workers, Cargo jobs or nextest threading. Lower host/dependency optimization is a separately triggered investigation, not part of this first candidate. LTO/profile changes can cause an initial rebuild of imported artifacts; preserving caches does not guarantee reuse across incompatible flags.

One resolved Cargo-profile value per selected verification boundary must drive run/list/build/doc commands, executable paths and summary metadata. Add `--cargo-profile NAME` to the existing selection interface, applying to the preceding selection like its other arguments. Bare nextest's default maps to Cargo `test`; explicit nextest compilation uses `--cargo-profile`, whereas nextest `--profile` remains runner configuration. CLI binaries paired with local tests use `cargo build --profile test`, not plain dev builds. Resolve artifact directories from that same owner, including an inherited intentional target override; file presence alone cannot prove profile freshness because test/dev share `target/debug`.

Until candidate qualification, verifier defaults remain release. After installation, ordinary local Rust verification defaults to test and explicit production controls use release. `just qualify` resolves optimized acceptance to release, including its representative native/CLI/MCP journeys and applicable Clippy/doc/conformance leaves. Do not allow a local override to silently substitute for required release evidence. Receipts identify actual profiles; preparation/readiness remains with ADR-0134 and never silently syncs.

Keep the existing CPython flow oracle explicitly release-backed: `tests/scripts/test_flow_soundness.py` independently builds `target/release/lctx` to challenge the integrated release producer. Likewise retain optimized extension preparation where its native journey contract requires it. Declare these production-only steps in selection/receipts instead of claiming every Python or extension control exercises O1. A candidate main-thread control must actually invoke the candidate-built CLI.

Pinned semantics: [Cargo profiles](https://github.com/rust-lang/cargo/blob/3d7cf6e937d6127d0f49881bf689c560b36d35c4/doc/book/src/reference/profiles.md), [override precedence](https://github.com/rust-lang/cargo/blob/3d7cf6e937d6127d0f49881bf689c560b36d35c4/src/workspace/profiles.rs#L489), [rustc LTO selection](https://github.com/rust-lang/rust/blob/c1070d69382b8d2f2eb65119c738a77d9e324c9e/compiler/rustc_session/src/session.rs#L979) and [nextest 0.9.146 Cargo forwarding](https://github.com/nextest-rs/nextest/blob/cargo-nextest-0.9.146/cargo-nextest/src/cargo_cli.rs#L436). The source review owns the broader compiler/cache evidence. These are Interface-checked facts, not a candidate run.

### 3.4 Coherent compiled harness reuse — F04

Move topic files beneath non-autodiscovered case directories and introduce the grouped top-level targets below. Each target includes its substantial common driver **once**; case modules import that driver instead of retaining their own `#[path]` copy. Keep existing assertions and topic module names. Lightweight non-driver controls and pure `cpg-flow` targets remain separate.

Current source mapping found 42 top-level extraction driver consumers and 19 core catalog-runtime consumers. This is the current target membership, distinct from the earlier review's broader 43/21 inclusion counts; neither is a timing estimate.

| New extraction target | Existing topic modules |
|---|---|
| `extraction_contracts` | attachment_oracle, determinism, harness, python_reference_oracle, typed_conformance |
| `extraction_syntax` | native_diagnostics, native_exports, native_lexical, typed_captures, typed_class_metadata, typed_comprehension_routes, typed_lexical, typed_ruff_context, typed_symbols, typed_syntax_shapes |
| `extraction_types` | native_class_traits, native_generics, native_model_context, typed_flow, typed_protocols, typed_types |
| `extraction_calls` | context_constructors, direct_usage, entry_value_witnesses, execution_channels, local_semantics, native_callable_deprecation, native_callable_variants, native_overload_origins, native_usage, typed_calls |
| `extraction_normalized` | normalized_bindings, normalized_callables, normalized_entities, normalized_events, normalized_imports, normalized_projections, normalized_recovery, normalized_relations |
| `extraction_evidence` | synthesis_documentary_templates, typed_deployment, typed_documents |

| New core target | Existing topic modules |
|---|---|
| `compiler_catalog` | catalog_core, catalog_evidence, catalog_selection, normalized_generation, normalized_relations |
| `compiler_analysis` | analysis_preparation, analytic, analytic_embedding, retrieval_preparation, structural |
| `compiler_behavior` | base_execution, behavioral_frontiers, local_semantics |
| `compiler_synthesis` | synthesis_documentary, synthesis_refutation |
| `compiler_artifacts` | entry_value_publication, graph_artifact, model_publication, summary_publication |

Reuse normal Cargo automatic target discovery; do not introduce a registration framework or one giant workspace harness. Generic assertion/decoder adapters remain small and local. A separate test-support crate is unnecessary for this selected grouping.

Update `scripts/verify.py` target lists/narrowing and live documented invocations. `providers:extract` retains its existing case scope: extraction library tests, separate acquisition/bundle targets, and harness/typed_conformance/typed_flow/typed_calls/native_overload_origins/typed_ruff_context modules in their new targets. Express these module restrictions through the existing nextest filter mechanism; compose user filters by intersection. Do not accidentally broaden this boundary to every case in every newly selected group. `compiler:producer` continues to cover its declared full target set, while focused selections name the new target and module filter.

Record an old-target/test-to-new-target/module discovery comparison. Remove obsolete top-level copies, their per-module driver inclusions and replaced current command references; historical receipts stay historical. Preserve disposable server/attempt ownership. Grouping merges a process, not test state: any exposed shared fixture namespace, environment mutation or static interference must be corrected in test/production ownership, never by a test-thread cap or whole-suite serial lock.

## 4. Scenarios and preservation

| Revealing change/failure | Required resulting behavior |
|---|---|
| Add another nominal record to a stage | New codec/typed adapter; shared task and SQL loop do not specialize on its request closure. Model inventories remain the authority. |
| Cancel synchronous or asynchronous caller after native submission | Request retains the charged batch until terminality; late error and interrupted/retried drain remain observable. No partial publication. |
| Change serving-only source | Extraction/provider production closure stays stable; ordinary serving compilation remains Cargo-owned. |
| Change native ingestion with unchanged emitted provider rows | Supplier identity remains scoped; workspace-produced contribution execution identity changes and survives cold transport. |
| Import a supported artifact from another compiler build | Captured execution/supplier identities remain intact; admission uses supported semantics and exact bindings, not the current executable hash. |
| Run candidate CLI or test-thread control | The recorded candidate-built artifacts are used, with normal stack/parallel settings; no enlarged-stack workaround establishes qualification. |
| Edit common driver versus one topic test | Driver rebuild affects its coherent groups; a topic edit affects its group. Assertions, discovery and independent fixture state remain intact. |

Preserve typed identity validation, exact immutable views/epochs, read-only SQL options, append-only codebooks, generated codecs, coverage policy, batch accounting and structured primary/secondary outcomes. Preserve optimized production settings and scope; no per-row erasure, alternate scheduler, second store, compatibility reader or runtime decoder authority is added.

## 5. Execution packages and dependencies

BC0 decision/owner updates are implemented. BC1/BC2 source and BC4 grouping are integrated on the preserved PC baseline; their functional acceptance and BC3/BC5 remain open. The root owns shared declarations, manifests, verification commands, plan disposition and integration. Parallel reasoning/work is useful where mutable ownership does not conflict; logical independence is not permission for competing edits to workspace/native/model files.

| Package | Working prerequisite | Delivered capability and completion boundary |
|---|---|---|
| **BC0 — decisions and architecture** | Confirmed RC01/RC02 and default choice; independent target-plan review resolved | Successor/complementary ADR routes and affected owner/instruction updates. Declare the qualification-gated local target without changing the active default prematurely. Preserve all unaffected ADR-0138/0136 obligations. |
| **BC1 — shared checked async mechanics** | BC0; current PC1 completion/PC2 binding contracts integrated | Erase requests before scheduling; own charged writes; shared registration/streaming; migrate all callers and confirmed enclosing coroutine patterns through companion CU0–CU6. Focused failure, lifetime, output and whole-operation compiler evidence. |
| **BC2 — scoped implementation provenance** | BC0; captured supplier/spec/inventory contracts integrated | Rooted dependency/asset fingerprints, raw provider identity and composed workspace execution identity; actual production and cold-transport consumers migrate together. Mutation, relocation/deletion and ingestion-only controls. |
| **BC3 — qualify and install local tests** | BC1's large/request future boundaries; working single-profile selection | Candidate O1/LTO-off controls on normal stacks and parallelism; default-test installation, verifier artifact/reporting migration and candidate retirement. Release production route independently retained. |
| **BC4 — grouped harness compilation** | Settled verification/module mapping; BC1 interfaces for affected test calls | All listed driver consumers regrouped, one driver per group, selectors migrated and fixture independence retained. Discovery/source-inclusion comparison and affected grouped controls. |
| **BC5 — integrated completion** | BC1–BC4 working on one current tree | Affected optimized native/CLI/cold/MCP controls and applicable leaves; assembled release qualification where required by shared transport/provenance contracts. Reconcile PC6 receipts at its owner; update findings/architectural claims only with their closure evidence. |

Recommended route: BC0 → BC1, with BC2 independently ready after BC0; then BC3/BC4 on their concrete prerequisites; BC5 integrates. BC4 mapping can be prepared earlier. BC3 candidate configuration/selection can be designed early, but its stack/runtime qualification cannot substitute for BC1 or silently change the default.

Do not wait for unrelated PC6 acceptance to remove known compiler amplification. Consume its already integrated completion/binding contracts, refresh tests affected by BC changes and preserve its open restored-coverage and native journey obligations. BC5 and PC6 can share a valid final-tree receipt without duplicating executions, but each plan closes only its own obligations. An old-tree or differently profiled receipt is not interchangeable.

The companion orders CU1 structural containment before further stage migrations, then permits
CU2–CU4 on working shared contracts with distinct editing ownership. CU5 resolves the conditional
output decision; CU6 integrates whole-operation evidence. BC3 must qualify its exercised paths
with working containment, not merely a smaller parent function. Full F01 closure additionally
requires all confirmed migrations and justified exclusions. Existing BC2/BC4 work is not reopened.

## 6. Migration and deliberate limits

This is a direct source cutover within the current design. Owned batch signatures, shared mechanics and grouped targets replace their prior paths completely. Existing public artifacts retain their supported fields; composition identity is carried by the existing execution-implementation hash. Supplier/cold-admission semantics remain ADR-0138's contracts. Do not add an old borrowing API, legacy target aliases or a dual provenance writer for convenience.

The candidate profile is temporary qualification state, not a standing second default. After passing, transfer its overrides to test, remove it and check the resulting actual default selection. An implementation failure keeps the current default intact until corrected; release availability is not evidence that the candidate passed.

Keep shared/final artifact directory ownership and sccache unchanged. Initial profile/fingerprint changes can require dependency rebuilding and do not warrant cache cleanup. Source membership remains conservative at crate granularity and root-lockfile scope; further granularity requires a concrete invalidation consumer and complete dependency evidence. No automatic cross-run incremental executor or cache accounting system is introduced.

## 7. Verification and acceptance

Commands below are **planned / not_run** for implementation. During each package, use compile checks and minimal revealing controls, with explicit targets/module filters. Pure model controls need no native server; native checks use `just fixture -- …` or the owning `just verify` boundary. Readiness observes; any explicit repair follows `just sync tools|native` only with the appropriate ownership. Do not run full families or `qualify` after every slice.

| Package / route | Revealing evidence required |
|---|---|
| BC1: core library/unit controls; selected `compiler:producer` and native store controls | Acknowledged success, primary failure, receiver disappearance/late failure, cancellation before/after submit, failed join retained, interrupted/retried drainage; reservation stays held while a cancelled sync/async write is still running and releases at terminality; declared/empty/duplicate outputs and exact input epochs/read-only refusal. Use independent expected rows and an explicitly observed reservation lifetime. |
| BC1 structural/runtime scope | Inspect representative emitted symbols from the first necessary build: concrete record/closure types no longer determine Tokio task/submission machinery. Shared stream/setup source is monomorphic. Qualify request/batch allocation and dispatch over revealing actual production operations; claim no unchanged runtime performance from source alone. Reuse ordinary acceptance runs, not a separate broad benchmark prerequisite. |
| BC1 coroutine extension, companion CU1–CU6 | Inspect parent, typed loader/emitter children and drops across CGUs; exact-prefix overlap, first-error order, unpolled-operation laziness, stage-specific selection and per-grain charge lifetime. Compare actual pinned normal/harness compiler evidence; a moved oversized child or parent-only shrinkage does not establish closure. CU5 adds bulk-output controls only if its conditional design is selected. |
| BC2: fingerprint/model/provider controls, selected compiler/store transport | Serving/core-only versus provider/model/analyzer/asset mutations affect proper scopes; optional/target/path/build roots handled conservatively; added/deleted assets and relocation stable; no recursive Cargo invocation; ingestion-only changes alter execution identity while supplier code stays scoped. Cold restore/inventory retains the captured composite hash under a different current build. |
| BC3: candidate normal libraries, tests, CLI main and finite/native controls | Resolved effective profiles, candidate-built CLI path, normal test/main stacks, assertions/overflow semantics, test discovery/filtering and runtime adequacy. Preserve and separately exercise release-only oracle/extension controls. After install, verify bare nextest and verifier resolve test, and explicit production selection resolves release. |
| BC4: extraction/core grouped targets and verifier harness controls | Each old test appears exactly once under its topic module; selected provider case set remains equivalent; driver compiled once per group; user filters intersect; fixture namespace/environment state independent under normal parallelism. Run all affected grouped cases once at this package's functional completion, not just a discovery mock. |
| BC5: final optimized acceptance | Affected Catalog/Behavioral frontiers, compile/admit/direct-seal, complete backup/restore/cold inspection and actual native/MCP expectations on current source. Because shared request/identity contracts change, schedule `just qualify` with release-resolved Rust artifacts once at functional scope completion, plus applicable leaves. Keep continuing PC6 failures open at its coordinator and rerun failed boundaries rather than claiming whole-plan acceptance from focused passes. |

Current available command forms include `just verify --print --select compiler:producer --nextest-args='--test workspace_inputs'`, `just verify --select compiler:producer --nextest-args='--test facts_admission'`, `just verify --select store:rust --nextest-args='--test compiler_views'`, and `just verify --rerun RUN`. Resolve current names with `--print` before execution; grouped targets and the new profile option become valid only after BC3/BC4 deliver them. For the candidate, bare nextest uses `--cargo-profile local-test-candidate`; accompanying Cargo binaries use `--profile local-test-candidate`. After installation, ordinary bare nextest needs no profile argument and optimized acceptance uses `--release`.

`summary.json`/existing run handles own raw receipts. Record source revision/tree, actual profile, selected cases, date and passed/failed/blocked/not_run. A source-inspected artifact pattern, accepted ADR or mocked command is not an actual product pass. Timing work, if later authorized, compares explicit equivalent workloads/conditions; no numerical speedup is part of this plan's current acceptance claim.

## 8. Remaining investigations and decision consequences

| Question from the source review | Disposition and settling evidence |
|---|---|
| Dominant current backend body/pass | **Measured partial diagnostic / source-reviewed, 2026-10-08 (§12/§13).** Retained C1-related completed InstCombine events fall sharply after the ordered boxed-loader correction. Matched pre-optimization IR and a simplified pinned-LLVM reproducer attribute a related PHI-heavy path to the structural producer. The exact active pass/optimized PHI shape in the actual compiler remains unresolved; the broader coroutine review supports coherent phase/loader containment without claiming a compiler defect or full-build speedup. |
| Runtime overhead of erasure | **Scheduled in BC1/BC5 and companion CU1–CU6.** Owned batch-level requests, coarse borrowed adapters/phases and actual operational controls; compare timing only under separately named conditions before claiming equivalent performance. Correct a material regression before closure. |
| Whole-operation containment, recurrence and output refinement | **Scheduled in companion CU0–CU6.** Mandatory assessment of every named candidate; correct confirmed patterns with their consumers and justify exclusions. Preserve specialized selection/cardinality, inspect children/drop bodies, and retain current output API unless residual evidence plus preserved buffering/error/cancellation semantics justify CU5. |
| O1 versus O0 | **O1 selected; qualification scheduled in BC3.** Known O0 stack failures prevent automatic fallback. Any failure drives coherent future/stack ownership correction. |
| Exact provider closure | **Selected at conservative normal/build crate/asset scope in BC2.** Mutation/deletion/relocation and capture consumers establish completeness. Finer lockfile/source granularity is deferred until repeated relevant invalidation exposes a concrete need. |
| Best harness grouping | **Selected in §3.4; qualified in BC4.** Discovery, source inclusion, selected-case equivalence and fixture independence settle the physical migration. Residual skew can justify changing groups; it cannot justify dropping controls. |
| Pinned compiler regression | **Deferred.** Revisit only for a residual symptom matched to the actual pinned phase/settings after structural corrections, or a deliberately selected toolchain change. Existing reports are leads, not diagnoses. |
| Host dependency optimization | **Deferred.** Revisit if remaining macro/build-script compilation or execution is a demonstrated bottleneck; qualify both sides before changing existing O2/O3 overrides. |

Deferred investigations have triggers, not assumed fixes. Do not compensate with worker caps, a new scheduler, arbitrary crate splitting, blanket row erasure or weakened semantic admission.

## 9. Sole compilation-cost finding disposition

The source reviews own their dated diagnoses/evidence; this table owns current scheduled status. No finding is closed by authoring or operator policy confirmation. Subsequent reviews link here and retain source IDs. The coroutine review's F01 extends compilation-cost F01 through the same structural cause; its distinct closure obligations remain visible in §13.

| Source finding | Disposition | Responsible component / scheduled work | Closure evidence |
|---|---|---|---|
| [Compilation-cost F01](../design_review/reviews/design_review_rust-compilation-costs_2026-10-08.md#F01); [coroutine review F01](../design_review/reviews/design_review_rust-coroutine-compilation-amplification_2026-10-08.md#F01) | **open** | NativeCalls/workspace/consumed-row/stage and scoped-loader owners; BC1/BC5, [companion CU0–CU6](rust-coroutine-compilation-amplification-plan_2026-10-08.md#4-execution-packages-and-concrete-dependencies) | Shared emitted task/setup paths; residual generic/C1 correction (§12); structural and confirmed recurring boundaries containing children/drops; complete caller migration and justified exclusions; actual input/output, charged cancellation/drain and completion controls; actual-settings compiler fit and operational qualification. CU5 resolves bulk append conditionally before adoption. |
| [Compilation-cost F02](../design_review/reviews/design_review_rust-compilation-costs_2026-10-08.md#F02) | **open** | Extraction fingerprint and workspace execution-identity owners; BC2/BC5 | Relevant production closure, unrelated downstream stability, relevant mutation/deletion/relocation effects and captured composite cold transport. |
| [Compilation-cost F03](../design_review/reviews/design_review_rust-compilation-costs_2026-10-08.md#F03) | **open** | Cargo/verification/CLI artifact owners; BC3/BC5 | Qualified O1/LTO-off normal-stack local loop, installed default selection, truthful profile receipts and retained release acceptance. |
| [Compilation-cost F04](../design_review/reviews/design_review_rust-compilation-costs_2026-10-08.md#F04) | **open** | Extraction/core harness and verification selection owners; BC4/BC5 | Grouped target/source inventory, one shared driver per group, preserved discovery/selection/assertions and independently owned concurrent fixtures. |
| [Coroutine target-review F01](../design_review/reviews/design_review_rust-coroutine-compilation-amplification-plan_2026-10-08.md#F01) | **resolved in Proposed target, 2026-10-08** | Companion author and model producer/scoped-loader owner; CU2/CU6 | Explicit `semantic_models::apply`/`publish_records` scope, roots/merge/evidence preservation and independent acceptance cases added to companion §2/§3.1/CU2/§6; independent follow-up accepts the revised target. This closes the authoring coverage gap only; source F01 and actual model-production correction/acceptance remain open. |

## 10. Authoring checkpoint and next action

**2026-10-08: authoring complete; BC0 implemented and execution in progress.** Source, current callers, provenance consumers and pinned profile/nextest contracts were inspected. RC01/RC02 and the default-local choice are operator-confirmed. No production/configuration change, Rust build, product test, compile probe, environment sync, cache cleanup or running-process intervention was performed for authoring.

The [independent design/target review](../design_review/reviews/design_review_rust-compilation-costs-plan_2026-10-08.md) **accepts the Proposed target within scope**, with A1–A4 satisfied and no new blocking findings. Its binding §4 clarification is reconciled in RC01's update route; it introduces no additional operator choice. Source F01–F04 remain open at §9. The exact source membership map was checked read-only: 42 extraction and 19 core driver consumers occur once in the proposed groups, with none missing or added.

**Authoring checks, 2026-10-08: passed** `just docs-check` (ADR metadata, agent instructions, documentation publication/search and offline links: 343 canonical pages, zero link errors). The read-only source inventory check passed; 104 pre-existing dirty files outside authorized document edits remained byte-identical. **not_run:** product checks, compile probes and resumed PC acceptance, because this scope creates the plan documents. These checks do not establish product acceptance.

The current action is candidate qualification of the integrated BC1/BC2 contracts and BC4 groups, followed by BC3 default installation and BC5 release acceptance. Existing PC1–PC5 work remains dirty and PC6 acceptance stays open. STATUS links to this plan for compilation-cost scope and to the persisted coordinator for its distinct execution boundary.

**Execution checkpoint, 2026-10-08:** ADR-0137 carries forward the unaffected ADR-0136 decisions and installs the confirmed local/provenance target. Architectural owners, agent instructions and the review binding name the qualification-gated transition. BC1 erases requests before scheduling, owns complete charged batches, shares registration and checked batch streaming, and migrates its callers. BC2 roots fingerprints at production owners and composes execution identity at ordinary output creation. BC4 moves 42 extraction and 19 core topics into the approved 6/5 groups; directly annotated test functions remain unchanged (226), with nested fixture controls retained. `cargo check --locked -p cpg-core -p cpg-extract --tests` **passed** after the grouped macro-import repair. Independent source review found patch membership and snapshot-path defects; a conservative `third_party` watch and expectation relocation correct them. Snapshot payload is unchanged. Candidate compilation and tooling/profile controls are in progress; functional BC1–BC5 acceptance remains pending. The dirty PC1–PC5 baseline is preserved.

**Final-source integration, 2026-10-08:** a focused followup identified residual generic/inventory DataFusion loops in native analysis, frontier coverage, embedding/retrieval metadata, graph preparation and normalization admission. These now use the same physical stream loop, preserving exact completed-input gates or the detached validator's invariant/table binding. Owner-authored Expr predicates remain typed, and specialized root/join/chunk loops remain concrete. Independent source review found no material defect in this migration. `cargo check --locked -p cpg-core -p cpg-extract --tests` **passed** on the integrated source, including the new native provenance controls. The root interrupted its own preceding candidate compilation before test execution because it omitted this required migration; no candidate test verdict was established. Dependency/intermediate caches remain intact. Final-source candidate qualification was interrupted for the operator-authorized diagnostics in §11; the active default remains release.

## 11. Opt-in compilation diagnostics

**Implemented / focused tested, 2026-10-08.** The operator authorized stopping the excessive
candidate compilation and implementing diagnostics before its restart. The owned nextest group
3814270 was interrupted with SIGINT; its fixture cleaned up and no test had started. Exit254
is interrupted compilation, not a test verdict. The old log remains at
`/tmp/library-context-final-candidate_2026-10-08.log`; completed artifacts and imported caches
are preserved. BC3 default installation and BC5 optimized acceptance remain pending.

`just compile-profile` reuses the existing run directories and cancellation owner. Its `record`
command accepts the existing Cargo command after `--`, defaulting to focus `cpg-core`; another
workspace package or `workspace` broadens selection explicitly. `doctor` observes prerequisites
and never installs or elevates. Recordings retain requested/effective argv, source/configuration
identities, unique compiler-unit directories, product outcome and independent telemetry outcome.
Fixtures, evaluation gold/heldout and environments are excluded from source capture.

Direct Cargo build/test/rustc and nextest run/list commands add pinned Cargo build analysis,
section timings and HTML timings. Their own JSON output supplies exact `build-started.run_id`
values; only matching `CARGO_HOME/log/<ID>.jsonl` and stamped timing files are retained. A missing
or incomplete session is explicit; a concurrent session's newest log is never substituted.
Wrapped commands retain selected-unit telemetry without pretending to enable whole-Cargo
analysis. Existing Cargo home, shared intermediates, target selection and profiles remain intact.

The command-local rustc wrapper delegates unselected units and compiler probes to the existing
wrapper (normally sccache). Selected units bypass that cache and add only
`self-profile-events=default,args,llvm`, JSON phase timings and JSON monomorphization statistics.
Normal libraries, unit-test harnesses and integration units own distinct sidecar directories.
No global RUSTFLAGS, compiler/job/thread limits, cache cleaning or forced rebuild is introduced.
A fresh Cargo unit can therefore produce no compiler profile; this is reported explicitly.
Unsupported shell response files are refused rather than silently reinterpreted.

Each active selected compiler has an independently owned perf observer: user-space cpu-clock
samples at99Hz with DWARF16384 call chains, rotated every45s. Live status reports only closed
chunks, process/thread CPU and memory observations, and sampled hotspots. It does not guess
the compiler phase from CPU use. The8GiB free-disk floor stops perf telemetry, not compilation.
Collector startup/storage failures must preserve the compiler command and its exit status.
Run cancellation stops the owned build and its observers; stopping a sampler alone leaves its
compiler running. Raw evidence is retained, including incomplete captures. Profiling runs use
the existing retained marker, protecting them from ordinary run pruning; explicit
`just runs retain RUN --release` relinquishes that protection. Stop with `just runs cancel RUN`,
then use `just compile-profile report RUN` to decode what is available. Interrupted or failed
units remain partial even when their observer finalized its own receipt; an unreadable compiler
self-profile never prevents reporting its completed CPU samples and retained phase timings.

The operator-installed `/opt/compile-profiling/perf` is root-owned, group-accessible to paul,
mode0750 with CAP_PERFMON. `kernel.perf_event_paranoid=2` persists through
`/etc/sysctl.d/99-local-perf.conf`; no standing sudo access is required for recording. The
permission control produced usable call stacks and zero lost samples on2026-10-08. Debugger
snapshots and syscall traces remain conditional investigations because ptrace briefly pauses
execution; ordinary busy compilation uses CPU sampling and compiler self-profiling.

`just compile-profile-tools sync` builds measureme12.0.3 at exact revision
`5ac839c602b59eee9c908b3b35b6d6c0cd1c42f7` under an independent tools root with its retained
[lockfile](../../tools/compile-profile/measureme.Cargo.lock). The pinned newer compiler requires
the retained [two-call CPUID patch](../../tools/compile-profile/measureme-nightly-cpuid.patch),
which removes obsolete unsafe blocks without weakening warnings. The installation receipt
binds source, lock, patch, toolchain, format9 and executable hashes; incompatible readers are
blocked. Product Cargo dependencies and lockfile are unaffected by tool acquisition.

`report RUN` converts terminated compiler self-profiles with summarize/crox and generates native
sample reports while preserving raw data. Reader execution has no timeout: the actual stopped
captures took69s/76s to summarize. Crox defaults to a1000µs minimum event duration for the
visualization; `LCTX_COMPILE_PROFILE_TIMELINE_MIN_US=0` requests full detail. The threshold is
recorded and invalidates cached conversions; raw captures and summaries remain unfiltered. `view RUN --kind compiler` identifies local Perfetto
timelines; `sampled` imports closed perf files through samply on localhost; `hotspot` reports
sampled costs or explicitly annotates a named native symbol. Query arguments and LLVM pass/IR
labels offer source attribution where rustc emits it. Sampled symbols do not establish exact
generic source items; monomorphization size/counts are estimates, not elapsed time; nested event
durations must not be summed. This tooling establishes attribution, not a compilation speedup.

**Verification, 2026-10-08:** **passed**
`uv run --no-sync pytest tests/scripts/test_compile_profile.py tests/scripts/test_compile_profile_capture.py tests/scripts/test_compile_profile_tools.py tests/scripts/test_build_measurements.py -q`
(83 controls after partial-report corrections), scoped Ruff, Pyrefly (zero errors), system-Python3.12 wrapper compilation and
`just compile-profile doctor`. Independent source review's CP-R1–CP-R5 corrections cover actual
rustc timing prefixes, observer-storage failure isolation, pipe draining, owner-death cleanup
and nextest runner-versus-Cargo profiles. All five corrections passed focused controls and the
followup source review found no further material defect. Historical `bench-builds` is now
report-only; its obsolete build/capture/preflight routes refuse before touching artifacts.

Actual isolated Cargo and nextest controls retain receipts
`20261008T180929.729Z-bd46bb` and `20261008T180930.521Z-9f2cd9`: products, exact Cargo sessions,
stamped HTML, format9 summarize/crox timelines and74/79 phase records **passed**. These are
tooling controls, not product qualification. The short bare-build sampler could not attach
before target exit; that telemetry failure remains recorded separately from its successful
product. The45s supervisor control read a closed chunk while sampling continued, then killed
the sampler owner: perf finalized two retained chunks and the target remained alive. Native
reports and samply import **passed**; its first window reported zero lost samples.

`just docs-check` **passed after correction** (343 pages, zero link errors); its first run
exposed the BC4-moved typed-conformance source link, now corrected at the review's current
case path. Scoped `turn-end` preserves paused Rust source and skips unrelated generators.
The profiling baseline check retained216 pre-existing files byte-for-byte outside this scope.

**Stopped diagnostic candidate, 2026-10-08:** run `20261008T181813.329Z-ddd158` owns the same
candidate selection on a disposable fixture, launched through `just run --background --label
compile-profile-candidate -- just fixture -- just compile-profile record --focus cpg-core --
cargo nextest run --locked --cargo-profile local-test-candidate … --no-fail-fast`.
At the operator's request, sampling continued for another ten minutes, then
`just runs cancel 20261008T181813.329Z-ddd158 --json` stopped this owned run at18:33:51UTC;
fixture cleanup completed at18:33:52UTC. The run is cancelled and retained. No tests started;
this is interrupted compilation, not a failed-test verdict. Both compiler identities terminated.
Both retained format9 self-profiles decoded successfully with the pinned summarize/crox readers;
all42 closed native sample chunks remain readable. Raw normal/harness profiles are12.24GB/12.76GB.
`just compile-profile report 20261008T181813.329Z-ddd158` **passed** through managed report
run `20261008T184632.122Z-dd0142`: both compiler conversions and all42 sample reports passed;
the canonical `report.json` retains `partial=true`, cancelled state and the retained marker.

The Cargo session's final62 bytes were an incomplete JSONL record. Reanalysis **passed** for
its exact validated1878-record prefix, with `partial=true`, `analysis_scope=complete_prefix`
and raw bytes/hash unchanged. The original stop receipt retains its initial blocked parser
outcome; `compile-profile/assessment/cargo-prefix-reanalysis.json` carries the corrected derived
receipt. Only a malformed final unterminated line can be omitted from analysis; malformed
middle or newline-terminated final records remain blocked. No newer shared Cargo log is substituted.

The report fixes passed ten additional focused controls and independent static review.
Detailed assessment and remaining remediation are in §12. BC3 default installation, candidate
runtime qualification and BC5 optimized acceptance remain pending; the active default is release.
Operator stores, protected data, source baseline and shared caches remain preserved.

## 12. Interrupted-capture assessment and remaining correction

**Measured partial diagnostics / source-inspected interpretation, 2026-10-08; C1 and residual adapter corrections Implemented, focused verification below.**
The retained run above was stopped before compiler completion. Its normal library and unit-test
harness both used `local-test-candidate` (O1, LTO off), with available compiler parallelism.
This evidence establishes a compilation hotspot on the captured dirty source; it does not
establish product correctness, full build duration or improvement against an uninstrumented baseline.

| Retained observation | Normal library | Unit-test harness | Interpretation |
|---|---:|---:|---|
| Completed object emissions / codegen modules |255/256|255/256|Almost all codegen units finished; backend work can collapse to one expensive remaining unit.|
| C1 `catalog_evidence::produce::{closure#0}` completed InstCombine durations |104.18s,102.11s,72.00s,71.95s|107.27s,101.69s,73.28s,70.82s|Same generated async body dominates the completed source-attributed InstCombine events in both products.|
| C1 aggregate completed InstCombine time, five events |350.27s|353.11s|Sequential completed passes on this function, not a sum of nested scopes or the entire unfinished tail.|
| C0 `catalog_core::produce::{closure#0}` aggregate InstCombine time |20.10s|17.80s|A useful nearby comparison; C1 is substantially more expensive.|
| Final sampled `InstCombinerImpl::visitPHINode` self CPU |about98.5%|about98.7%|Sustained busy LLVM optimization, roughly one CPU-second per second per compiler, rather than an idle Cargo lock.|
| Generated instruction count |13,628,512|13,879,886|A large compiled surface; an instruction count, not elapsed time or executable size.|

The busiest normal-library function is
[`catalog_evidence::produce`](../../crates/cpg-core/src/catalog_evidence.rs).
Before correction, its two `decoder_inputs!` expansions each contained143 entries:54 catalog inputs,17 catalog
outputs,55 evidence inputs,6 runtime inputs,9 expected-domain inputs and2 authored
metadata types. The inventory and artifact loops therefore place286 specialized await sites
inside one producer coroutine; overlapping entries are not unique types or runtime query counts.
The producer's captured MIR size estimate is25,100 statements/terminators versus11,664 for C0.
C1 additionally expands38 output declarations and26 typed output-writing loops.

**Source-supported cause:** typed macro expansion accumulates suspension, stored child-future,
cleanup and borrowing paths in one very large generated async function. Shared lower streaming
loops have reduced physical duplication, but do not remove these enclosing specialized await
sites. The source shape and completed pass attribution strongly support this as the primary
remediation target. The interrupted active pass has no completed duration or source label;
its exact function and full eventual cost are not established by the completed event list.

The exact pinned LLVM
[`visitPHINode`](https://github.com/rust-lang/llvm-project/blob/1b9c0d5ff9bbe7634aead059efe6b11a7eeba145/llvm/lib/Transforms/InstCombine/InstCombinePHI.cpp#L1555)
contains repeated incoming-block lookup and sibling-PHI comparison. Large joins/PHI sets can
amplify those scans. This is a credible mechanism for the sampled hotspot, not proof of which
inner loop dominates or an LLVM defect: the capture does not retain the relevant optimized IR.
No debugger suspension or additional compiler run is needed to establish the current priority.

**Implemented C1 correction within F01/BC1:** generate ordered typed loader adapters mechanically
from the existing decoder inventory, and await their uniform boxed futures in one monomorphic
loop. Erase the concrete future before the loop owns it. Keep exact record/prefix permits,
owner-selected SQL, artifact admission, coverage checks and `ConsumedInputs` duplicate handling
inside the small typed adapter. Preserve the existing macro order; iterating a sorted declaration
list may change behavior. Do not introduce an independently authored decoder registry or per-row
dynamic dispatch. Box coherent inventory, artifact-load and emission phase boundaries where
needed; moving source files or extracting another143-await concrete async helper is insufficient.

Preserve charged data and drop order: captured sources/admission survive through coverage and
link production; frames and authored definition/parameter rows drop before scope preparation;
the artifact scope drops before blocking computation; charged evidence moves into its worker;
output rows/links drop per artifact while invocation rows remain through linking. Completion,
cancellation and original/cleanup error ownership remain with the existing producers.

**Remaining generic amplification:** captured mono statistics group codegen-unit placements by
DefId, so these are neither distinct type counts nor elapsed costs. The shared `stream_batches`
and `NativeCalls::submit` coroutine each have one placement. However `stream_query_filter_at`
has6350/6354 normal/harness placements, `NativeCalls::call`1786/1809, `Writer<R>::close`888/888
and `FrontierIndex::visit_with_check`458/458. Complete F01 by erasing callbacks/futures before
owning shared async/control-flow implementations, retaining thin record-specific adapters.
In particular, expected-input validation must stay conditional on a handled relation; hoisting
it unconditionally would change the existing skipped-input contract. Typed writer preparation
can remain concrete while shared completion awaiting is separated.

**Authorized completion boundary:** this execution delivers the C1/residual adapter correction,
focused controls and retained diagnosis. BC3 default installation and BC5 release acceptance remain
pending. Available compiler/test parallelism, production optimization and runtime limits remain
unchanged. The current capture does not justify blanket LLVM-pass disabling or a toolchain upgrade.
A residual PHI tail required retained IR and function attribution; the subsequent retained-IR
assessment below distinguishes the simplified reproducer from the actual compiler.

Raw evidence and derived artifacts remain under
`build/runs/20261008T181813.329Z-ddd158/compile-profile/`: `units/` owns raw captures and reports;
`assessment/compiler-attribution.json` owns source-attributed pass comparisons,
`assessment/sampled-windows.json` owns the repeated native-window observations, and
`assessment/decoder-receipts.json` owns the successful interrupted-profile conversion receipts.
Summaries omit unfinished event scopes. Instrumentation itself has overhead, including86.31s
of completed query-string allocation in the normal summary; the measured durations cannot be
presented as an uninstrumented compile baseline. The following correction is now integrated; F01–F04 remain open at §9 because broader operational
qualification, default installation and optimized acceptance have not been completed.


### Correction implementation and comparison checkpoint, 2026-10-08

**Implemented / independently source reviewed:** ordered C1 loader groups are generated directly
from the existing semantic inventories. Thin typed adapters retain exact record/prefix permits,
coverage admission and duplicate handling; one shared loop awaits their boxed futures. Inventory,
declaration, frame publication, artifact loading, blocking computation and emission own separate
boxed phases with the original charged owners and drop order. Typed stream callbacks are erased
before shared async setup, conditional expected-input guards use borrowed dynamic dispatch,
and native calls/writer completion await shared implementations. Final close remains lazy;
submitted final writes retain both the complete charged batch and erased writer through native
terminality. Independent combined source review found no material defect in these changes.

**passed:** `cargo check --locked -p lctx-model --tests`
(run `20261008T200128.659Z-ab2679`) and
`cargo check --locked -p cpg-core -p lctx-model --tests`
(run `20261008T200616.190Z-40632c`).
**passed:** nine model controls through `just verify --select model:rust --cargo-profile
local-test-candidate --nextest-args="--lib --test analysis_expected -E 'test(conditional_) |
test(artifact_labels_are_transient_and_failed_permits_preserve_prepared_state)'"`
(run `20261008T200306.501Z-07705c`). These cover conditional guard laziness and refusal before
prepared-state consumption; they do not establish native producer acceptance.

The comparable retained capture is `20261008T200802.033Z-6761de`. It uses the preceding package,
profile, library and grouped-target selection; runner-only changes remove `--no-fail-fast` and
add `--no-run` to compile without executing tests. The first attempt was refused by nextest's
conflicting flags before compilation (`20261008T200747.177Z-a47384`); the corrected command
was then recorded. The watchdog requested cancellation after600 seconds from normal compiler
start. Nextest's separate Cargo process group survived the outer cancellation; after verifying
both recorded compiler identities, that owned group was stopped102 seconds later. The raw
capture and this discrepancy are retained in `assessment/comparison-watch.json` and
`assessment/descendant-stop.json`. This is an interrupted diagnostic, not a compiler/test pass
or an exact ten-minute recording. `just compile-profile report 20261008T200802.033Z-6761de`
**passed** decoding both partial compiler captures and all32 retained native sample chunks
(report run `20261008T202036.272Z-0c35a1`).

**Cancellation repair checkpoint:** the diagnostic tool now records verified descendant ownership
and stops the owned escaped Cargo/compiler groups without signaling reused or unrelated processes.
Independent static review found no material defect; the implementing worker reports50 focused
tool controls passed and Pyrefly zero errors/three warnings. These are attributed worker receipts,
not a newly executed review-scope gate or proof of a repeated live compiler-stop scenario.
Tool source remains dirty; its complete scope-end leaves and actual stop acceptance remain pending.

| Comparable retained observation | Before normal / harness | After normal / harness |
|---|---:|---:|
| C1 parent coroutine MIR estimate |25,100 /25,100|591 /591|
| C1-related completed InstCombine events retained at ≥1000µs |350.33s /353.16s|0.185s /0.266s|
| Generated instruction count |13,628,512 /13,879,886|8,261,487 /9,578,756|
| Completed object emissions / codegen modules |255/256 /255/256|255/256 /255/256|

The comparison includes new C1 helper names and related concrete adapters rather than only its
renamed parent. Timelines share the1000µs visualization threshold: submillisecond events are
omitted, so these are retained-event totals, not the unfiltered sum of every C1 pass. Nested
scopes are not added. Raw profiles and unfiltered compiler summaries remain available. MIR
estimates and codegen-unit placements are not elapsed time, distinct type counts or runtime
performance. The shared streaming, guarded submission and close coroutines each have one
placement. Concrete record callback/close adapters remain small; they have not been mistaken
for repeated shared coroutines.

**Remaining diagnosis, updated 2026-10-08:** both corrected units stopped at255 completed object
emissions. Their final retained native windows are98.77%/98.80% self CPU in `visitPHINode`.
The subsequent owned IR capture `20261008T205144.595Z-adb189` retains the unfinished normal
codegen unit's pre-optimization bitcode under `core-ir/`, using the actual compiler arguments
with save-temps and isolated output/incremental directories. Completed dependency artifacts
were reused; the live native controls were left untouched. The structural coroutine has7,949
blocks and37,772 instructions before optimization. A read-only LLVM C API inspection found
no PHIs in that pre-optimization body; it cannot establish the shape of the later hot PHIs.

The retained simplified reproducer `20261008T205953.268Z-ada851` applies the pinned LLVM
`default<O1>` pipeline with a null target machine to this exact bitcode. Its named structural
function expands beyond2.25 million instructions; a five-second sample records99.08% self CPU
in `visitPHINode`. The reproducer was stopped and retained. This establishes a related
structural-function amplification path, not the exact rustc pass sequence, target configuration,
inner loop or a proved compiler defect. Raw bitcode/LLVM hashes, commands, scripts, last named
function and evidence limits are retained in `core-ir/assessment/structural-reproducer-assessment.json`.
The actual native control remains compiling with the sampled related native hot path.

Read-only pre-optimization inspection also identifies large summary, model, synthesis, retrieval
and selection loader/producer bodies (`core-ir/review-ir-stats.json`). Their instruction counts
are candidate shapes, not per-function elapsed costs or a ranking of optimization priorities.
The [coroutine design review](../design_review/reviews/design_review_rust-coroutine-compilation-amplification_2026-10-08.md)
reconciles these observations with domain ownership and library fit. It requires revision of
the enclosing physical composition for A4; typed record/codec leaves and synchronous kernels
remain useful boundaries. No full compile-time or runtime improvement is claimed.

Derived receipts and the attribution script live under the comparison's `compile-profile/assessment/`:
`correction-comparison.json`, `correction_attribution.py`, `remaining-tail-inference.json` and
normal/harness final self reports. The source capture binds the preserved dirty BC/PC tree.
**In progress:** selected actual native/core/C1 controls, run `20261008T202337.170Z-114602`.
**not_run:** BC3 default installation, BC5 release acceptance and real-library/operator actions;
they are outside this correction-and-diagnosis completion boundary.

## 13. Coroutine architecture review checkpoint, 2026-10-08

**Runtime follow-up, Proposed 2026-10-09:** the [populated companion](populated-journey-execution-amplification-plan_2026-10-09.md) owns native write/preparation improvements and coarse runtime phase evidence. Its PJ2 dispatch retains BC1/CU's shared future boundaries; runtime preparation does not undo the compiler-cost correction or justify per-row erasure/tasks. The persisted coordinator owns new populated findings and PJ5's native/CLI/cold/MCP acceptance, which may supply BC5/CU6 only with matching source/profile and the actual required controls. BC3 default installation remains independently qualification-gated; release remains active, and operator-deferred timeout/assembled obligations stay open. Existing compiler captures are not runtime phase attribution or new-schema acceptance.

The [independent coroutine design/target review](../design_review/reviews/design_review_rust-coroutine-compilation-amplification_2026-10-08.md)
is complete. **Decision: Revise for A4; A1–A3 and semantic/lifecycle gates hold within the
inspected scope.** Its F01 is grouped with existing F01 at §9. It assesses structural execution
deeply, recurring workspace coroutine shapes, coupled ownership/runtime consequences and exact
existing-library capabilities. This is neither whole-workspace certification nor native acceptance.
Rule impacts are **none**; no dependency, profile, toolchain or policy change is inferred.

**Proposed correction direction:** retain model-owned nominal records, codecs and pure kernels;
move coherent structural loading/publication operations behind borrowed opaque phase boundaries;
derive thin typed loaders mechanically from existing inventories and share their ordered traversal.
Containing the parent alone is insufficient: child futures, nested scoped-read loops and generated
drop paths are separate closure obligations. Preserve exact type/prefix/view permits, profile
selection, conditional coverage, first-error order, poll-time laziness, selected grain lifetime,
charged batch/writer ownership and terminal native drainage. No new scheduler or per-row task/box
is needed. Summary/model scopes, synthesis, retrieval preparation, selection and analytic producers
are recurrence candidates for directed inspection, not automatically proven hotspots.

The existing futures/Tokio/DataFusion/Arrow/petgraph interfaces support this direction. Borrowed
`BoxFuture` boundaries do not require spawning or `'static` ownership; synchronous graph kernels
already provide a positive composition example. A shared checked bulk append is **conditional**:
consider it only if phase/loader containment leaves material output composition costs, and first
compare cross-flush ordering, thresholds, deduplication, poisoning, backpressure, row-byte admission
and cancellation semantics. Arrow structural validation must not replace nominal `Batch<R>` checks.

**Remaining consequential work:** complete CU6’s native/runtime release qualification and retain the normal/harness child/drop assessment with exact source, settings and incremental-workload limits. Compiler samples do not establish transformed PHI shape or a compiler defect; exact pipeline investigation remains conditional on a material residual that could change the remedy. Whole-workspace and runtime performance remain unmeasured. BC3 installation, BC5 acceptance and current PC acceptance retain their separate owners.

**Coroutine plan authoring, 2026-10-08:** the [companion target](rust-coroutine-compilation-amplification-plan_2026-10-08.md)
now develops those boundaries and CU0–CU6. The operator selected structural first plus mandatory
assessment/correction of confirmed named workspace recurrence. Its accepted target is now implemented within CU0–CU5; F01 remains open until the coordinator’s operational obligations are met. The companion's
§5 carries all five source-review investigations and conditional output/cardinality questions.

The [independent companion target review](../design_review/reviews/design_review_rust-coroutine-compilation-amplification-plan_2026-10-08.md)
accepts the revised Proposed target within scope, with A1–A4 satisfied and no remaining blocking
target finding or rule impact. Its stable F01 identified omitted model production; CU2 now covers
the actual apply/publication consumer and independent catalog/root/merge evidence cases. §9 owns
that resolved authoring disposition separately from the still-open source implementation finding.
The review's document hashes identify the inspected target snapshot before publication/checkpoint
links were added; those later additions do not change the reviewed target or package contracts.

**Coroutine execution checkpoint, 2026-10-08:** the companion’s CU0–CU4 migrations are integrated and independently source-reviewed, including source-confirmed C0, documentary, semantic execution/normalization child loaders, local analysis and native inventory recurrence. The 24-file reviewed composite is `05664db988cac05cc6da26565a21864d4a77fc829ae07457429ca58d01938769`; no material findings. §9 remains the sole disposition owner. CU5 retains current checked/buffered `push`, with a material attributed output-cost plus preserved-contract trigger for reconsideration. The companion §7 owns detailed consumer exclusions, source identities and execution receipts.

**passed (2026-10-08):** final core/model all-test compile check and 13 actual focused candidate controls (run `20261008T225434.109Z-fe57d0`). The final actual-settings compiler capture `20261008T225117.998Z-e84d9b` completed normal/harness and seven integration targets, exit0 in2m51s, without changing available parallelism or profiles. Parent/child/drop assessment is complete with29 explicit match rules per product and no attributed relocated runaway; native release qualification remains open. These are bounded diagnostic/runtime receipts, not BC3 installation, F01 closure or whole-workspace speed measurements.

The prior native run `20261008T202337.170Z-114602` was cancelled with operator authorization; it produced no test verdict. First CU capture `20261008T224204.565Z-5a9f8e` completed normal codegen, then filesystem exhaustion interrupted remaining products. Raw closed evidence is retained; the operator freed storage before the replacement. Its run-local assessment distinguishes source versions, partial harness, filtered timelines and overlapping pass durations. Current native/PC acceptance consumes only matching final-source/profile receipts.

**Integrated-source follow-up, 2026-10-08:** companion §7 records the64GiB/automatic-partition ordinary defaults, selected-read allocation/order and definite-conflict repairs,757 model plus17 compiler/12 store/3 codec-cancellation passing controls, and the confirmed native SDK-value conversion correction (both actual serving controls reran passed). Capture `20261009T003236.093Z-dfb25d` passed all nine products, exact readers and14 closed perf reports; normal/harness95.578s/161.839s are instrumented diagnostics with changed source/CGU workloads. Four required Behavioral stage controls still timed out at300s; the assembled release run completed with failures. CU6/BC3/BC5 and F01 remain open. No matching-source assembled acceptance, whole-build speed claim or default installation follows from the focused repairs.

**Prior review/authoring receipts (2026-10-08):** document checks passed344/346 pages respectively, no link errors; independent target review accepts the revised Proposed target including explicit model application/publication. Those historical checks are not newly executed product acceptance. BC3 default installation, BC5 release qualification and PC6 remain open; release remains the active verification default.
