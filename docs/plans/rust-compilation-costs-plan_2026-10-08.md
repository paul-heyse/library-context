# Rust compilation costs: shared work, scoped provenance and the local test loop

**Accepted target / implementation in progress · 2026-10-08.** ADR-0137 records confirmed choices. Current verifier defaults remain release until BC3 qualification; no complete product acceptance is established.

## 1. Outcome, basis and ownership

Make ordinary Rust edits and focused/full test compilation do materially less avoidable work while preserving optimized production behavior. The [source review](../design_review/reviews/design_review_rust-compilation-costs_2026-10-08.md) establishes four structural causes, F01–F04; it does not identify the dominant pass in the unfinished LLVM module or measure a speedup. This plan develops all four corrections, their interacting preservation requirements and the review's six remaining uncertainties.

Inspected baseline: main `8511eb5fe27498815ce4433fd2d0029aa2e6016e`, plus the dirty integrated PC1–PC5 implementation, 2026-10-08. No clean-tree assumption or rollback to an earlier coordinator checkpoint is made. Existing compiler/native/library evidence is retained with its original date and boundary. Source inspection and pinned-tool research establish a credible target; implementation must still establish correctness and runtime adequacy.

This document owns compilation-cost sequencing and the sole scheduled disposition of source-review F01–F04 in §9. Architectural owners retain enduring contracts: [DESIGN §1.2](../design/DESIGN.md#section-1-2), [acquisition/extraction §4](../design/sections/acquisition-and-extraction.md) and [semantic model §15](../design/sections/semantic-model.md). The [persisted-execution coordinator](persisted-graph-execution-plan_2026-10-07.md) continues to own PG/PC completion, its source findings and native/CLI/MCP acceptance. Its [correction plan](persisted-execution-corrections-plan_2026-10-07.md) supplies captured bindings and complete operation outcomes; accepting this plan does not close PC6.

Plan authoring changes documentation only. During implementation, preserve current source repairs, exact pins, shared caches/intermediates, available compiler/test parallelism, disposable fixture ownership, operator state and protected evaluation inputs. Do not routinely clean Cargo caches, add job/thread caps, synchronize a live native environment, or interrupt another running build. A performance campaign, toolchain upgrade and real-library pilot are not prerequisites to correcting the demonstrated architecture.

## 2. Confirmed rule changes and target defaults

The operator explicitly confirmed these choices in the plan-creation conversation on **2026-10-08**. The similarly numbered rule impacts in older catalog/persisted reviews are different decisions.

| Source item | Confirmed choice | Route before dependent implementation |
|---|---|---|
| Compilation-cost review RC01 | **Accepted:** qualify local-test compilation separately from optimized production acceptance. | BC0 records [ADR-0137](../adr/0137-qualified-local-compilation.md), superseding ADR-0136, and updates DESIGN §1.2, AGENTS testing guidance and the [review binding §4](../design_review/design_principles/binding/library-context.md#4-where-findings-land) release-profile instruction with the transition rule. Carry forward all unaffected build, pin, fixture and available-parallelism decisions; never rewrite the accepted ADR body. BC3 changes the actual default only after qualification. |
| Compilation-cost review RC02 | **Accepted:** provider provenance covers its relevant implementation closure; composition provenance belongs at its actual owner. | BC0 records the complementary §4/§15 provenance decision in ADR-0137. BC2 narrows provider compilation inputs and captures native execution composition at the workspace owner. Preserve ADR-0135's semantic/captured/executable distinctions. |
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

BC0 decision/owner updates are implemented; BC1–BC5 remain open for their actual implementation and acceptance. The root owns shared declarations, manifests, verification commands, plan disposition and integration. Parallel reasoning/work is useful where mutable ownership does not conflict; logical independence is not permission for competing edits to workspace/native/model files.

| Package | Working prerequisite | Delivered capability and completion boundary |
|---|---|---|
| **BC0 — decisions and architecture** | Confirmed RC01/RC02 and default choice; independent target-plan review resolved | Successor/complementary ADR routes and affected owner/instruction updates. Declare the qualification-gated local target without changing the active default prematurely. Preserve all unaffected ADR-0135/0136 obligations. |
| **BC1 — shared checked async mechanics** | BC0; current PC1 completion/PC2 binding contracts integrated | Erase requests before scheduling; own charged synchronous/asynchronous writes; shared registration and read-only batch streaming; migrate all callers/selected stages. Focused failure, lifetime and output controls plus structural specialization evidence. |
| **BC2 — scoped implementation provenance** | BC0; captured supplier/spec/inventory contracts integrated | Rooted dependency/asset fingerprints, raw provider identity and composed workspace execution identity; actual production and cold-transport consumers migrate together. Mutation, relocation/deletion and ingestion-only controls. |
| **BC3 — qualify and install local tests** | BC1's large/request future boundaries; working single-profile selection | Candidate O1/LTO-off controls on normal stacks and parallelism; default-test installation, verifier artifact/reporting migration and candidate retirement. Release production route independently retained. |
| **BC4 — grouped harness compilation** | Settled verification/module mapping; BC1 interfaces for affected test calls | All listed driver consumers regrouped, one driver per group, selectors migrated and fixture independence retained. Discovery/source-inclusion comparison and affected grouped controls. |
| **BC5 — integrated completion** | BC1–BC4 working on one current tree | Affected optimized native/CLI/cold/MCP controls and applicable leaves; assembled release qualification where required by shared transport/provenance contracts. Reconcile PC6 receipts at its owner; update findings/architectural claims only with their closure evidence. |

Recommended route: BC0 → BC1, with BC2 independently ready after BC0; then BC3/BC4 on their concrete prerequisites; BC5 integrates. BC4 mapping can be prepared earlier. BC3 candidate configuration/selection can be designed early, but its stack/runtime qualification cannot substitute for BC1 or silently change the default.

Do not wait for unrelated PC6 acceptance to remove known compiler amplification. Consume its already integrated completion/binding contracts, refresh tests affected by BC changes and preserve its open restored-coverage and native journey obligations. BC5 and PC6 can share a valid final-tree receipt without duplicating executions, but each plan closes only its own obligations. An old-tree or differently profiled receipt is not interchangeable.

## 6. Migration and deliberate limits

This is a direct source cutover within the current design. Owned batch signatures, shared mechanics and grouped targets replace their prior paths completely. Existing public artifacts retain their supported fields; composition identity is carried by the existing execution-implementation hash. Supplier/cold-admission semantics remain ADR-0135's contracts. Do not add an old borrowing API, legacy target aliases or a dual provenance writer for convenience.

The candidate profile is temporary qualification state, not a standing second default. After passing, transfer its overrides to test, remove it and check the resulting actual default selection. An implementation failure keeps the current default intact until corrected; release availability is not evidence that the candidate passed.

Keep shared/final artifact directory ownership and sccache unchanged. Initial profile/fingerprint changes can require dependency rebuilding and do not warrant cache cleanup. Source membership remains conservative at crate granularity and root-lockfile scope; further granularity requires a concrete invalidation consumer and complete dependency evidence. No automatic cross-run incremental executor or cache accounting system is introduced.

## 7. Verification and acceptance

Commands below are **planned / not_run** for implementation. During each package, use compile checks and minimal revealing controls, with explicit targets/module filters. Pure model controls need no native server; native checks use `just fixture -- …` or the owning `just verify` boundary. Readiness observes; any explicit repair follows `just sync tools|native` only with the appropriate ownership. Do not run full families or `qualify` after every slice.

| Package / route | Revealing evidence required |
|---|---|
| BC1: core library/unit controls; selected `compiler:producer` and native store controls | Acknowledged success, primary failure, receiver disappearance/late failure, cancellation before/after submit, failed join retained, interrupted/retried drainage; reservation stays held while a cancelled sync/async write is still running and releases at terminality; declared/empty/duplicate outputs and exact input epochs/read-only refusal. Use independent expected rows and an explicitly observed reservation lifetime. |
| BC1 structural/runtime scope | Inspect representative emitted symbols from the first necessary build: concrete record/closure types no longer determine Tokio task/submission machinery. Shared stream/setup source is monomorphic. Qualify request/batch allocation and dispatch over revealing actual production operations; claim no unchanged runtime performance from source alone. Reuse ordinary acceptance runs, not a separate broad benchmark prerequisite. |
| BC2: fingerprint/model/provider controls, selected compiler/store transport | Serving/core-only versus provider/model/analyzer/asset mutations affect proper scopes; optional/target/path/build roots handled conservatively; added/deleted assets and relocation stable; no recursive Cargo invocation; ingestion-only changes alter execution identity while supplier code stays scoped. Cold restore/inventory retains the captured composite hash under a different current build. |
| BC3: candidate normal libraries, tests, CLI main and finite/native controls | Resolved effective profiles, candidate-built CLI path, normal test/main stacks, assertions/overflow semantics, test discovery/filtering and runtime adequacy. Preserve and separately exercise release-only oracle/extension controls. After install, verify bare nextest and verifier resolve test, and explicit production selection resolves release. |
| BC4: extraction/core grouped targets and verifier harness controls | Each old test appears exactly once under its topic module; selected provider case set remains equivalent; driver compiled once per group; user filters intersect; fixture namespace/environment state independent under normal parallelism. Run all affected grouped cases once at this package's functional completion, not just a discovery mock. |
| BC5: final optimized acceptance | Affected Catalog/Behavioral frontiers, compile/admit/direct-seal, complete backup/restore/cold inspection and actual native/MCP expectations on current source. Because shared request/identity contracts change, schedule `just qualify` with release-resolved Rust artifacts once at functional scope completion, plus applicable leaves. Keep continuing PC6 failures open at its coordinator and rerun failed boundaries rather than claiming whole-plan acceptance from focused passes. |

Current available command forms include `just verify --print --select compiler:producer --nextest-args='--test workspace_inputs'`, `just verify --select compiler:producer --nextest-args='--test facts_admission'`, `just verify --select store:rust --nextest-args='--test compiler_views'`, and `just verify --rerun RUN`. Resolve current names with `--print` before execution; grouped targets and the new profile option become valid only after BC3/BC4 deliver them. For the candidate, bare nextest uses `--cargo-profile local-test-candidate`; accompanying Cargo binaries use `--profile local-test-candidate`. After installation, ordinary bare nextest needs no profile argument and optimized acceptance uses `--release`.

`summary.json`/existing run handles own raw receipts. Record source revision/tree, actual profile, selected cases, date and passed/failed/blocked/not_run. A source-inspected artifact pattern, accepted ADR or mocked command is not an actual product pass. Timing work, if later authorized, compares explicit equivalent workloads/conditions; no numerical speedup is part of this plan's current acceptance claim.

## 8. Remaining investigations and decision consequences

| Question from the source review | Disposition and settling evidence |
|---|---|
| Dominant current backend body/pass | **Deferred.** No new compile probe for authoring. Inspect residual ordinary-build evidence if BC1/BC3 leave an excessive tail; uncertainty does not block removing demonstrated specialization. |
| Runtime overhead of erasure | **Scheduled in BC1/BC5.** Owned batch-level requests and actual operational controls; compare timing only under separately named conditions before claiming equivalent performance. Correct a material regression before closure. |
| O1 versus O0 | **O1 selected; qualification scheduled in BC3.** Known O0 stack failures prevent automatic fallback. Any failure drives coherent future/stack ownership correction. |
| Exact provider closure | **Selected at conservative normal/build crate/asset scope in BC2.** Mutation/deletion/relocation and capture consumers establish completeness. Finer lockfile/source granularity is deferred until repeated relevant invalidation exposes a concrete need. |
| Best harness grouping | **Selected in §3.4; qualified in BC4.** Discovery, source inclusion, selected-case equivalence and fixture independence settle the physical migration. Residual skew can justify changing groups; it cannot justify dropping controls. |
| Pinned compiler regression | **Deferred.** Revisit only for a residual symptom matched to the actual pinned phase/settings after structural corrections, or a deliberately selected toolchain change. Existing reports are leads, not diagnoses. |
| Host dependency optimization | **Deferred.** Revisit if remaining macro/build-script compilation or execution is a demonstrated bottleneck; qualify both sides before changing existing O2/O3 overrides. |

Deferred investigations have triggers, not assumed fixes. Do not compensate with worker caps, a new scheduler, arbitrary crate splitting, blanket row erasure or weakened semantic admission.

## 9. Sole compilation-cost finding disposition

The source review owns original diagnoses/evidence; this table owns current scheduled status. No finding is closed by authoring or operator policy confirmation. Subsequent reviews link here and retain source IDs.

| Source finding | Disposition | Responsible component / scheduled work | Closure evidence |
|---|---|---|---|
| [Compilation-cost F01](../design_review/reviews/design_review_rust-compilation-costs_2026-10-08.md#F01) | **open** | NativeCalls/workspace/consumed-row/stage owners; BC1/BC5 | Shared emitted task path and setup/stream source, complete caller migration, charged cancellation/drain/input controls and operational runtime qualification. |
| [Compilation-cost F02](../design_review/reviews/design_review_rust-compilation-costs_2026-10-08.md#F02) | **open** | Extraction fingerprint and workspace execution-identity owners; BC2/BC5 | Relevant production closure, unrelated downstream stability, relevant mutation/deletion/relocation effects and captured composite cold transport. |
| [Compilation-cost F03](../design_review/reviews/design_review_rust-compilation-costs_2026-10-08.md#F03) | **open** | Cargo/verification/CLI artifact owners; BC3/BC5 | Qualified O1/LTO-off normal-stack local loop, installed default selection, truthful profile receipts and retained release acceptance. |
| [Compilation-cost F04](../design_review/reviews/design_review_rust-compilation-costs_2026-10-08.md#F04) | **open** | Extraction/core harness and verification selection owners; BC4/BC5 | Grouped target/source inventory, one shared driver per group, preserved discovery/selection/assertions and independently owned concurrent fixtures. |

## 10. Authoring checkpoint and next action

**2026-10-08: authoring complete; BC0 implemented and execution in progress.** Source, current callers, provenance consumers and pinned profile/nextest contracts were inspected. RC01/RC02 and the default-local choice are operator-confirmed. No production/configuration change, Rust build, product test, compile probe, environment sync, cache cleanup or running-process intervention was performed for authoring.

The [independent design/target review](../design_review/reviews/design_review_rust-compilation-costs-plan_2026-10-08.md) **accepts the Proposed target within scope**, with A1–A4 satisfied and no new blocking findings. Its binding §4 clarification is reconciled in RC01's update route; it introduces no additional operator choice. Source F01–F04 remain open at §9. The exact source membership map was checked read-only: 42 extraction and 19 core driver consumers occur once in the proposed groups, with none missing or added.

**Authoring checks, 2026-10-08: passed** `just docs-check` (ADR metadata, agent instructions, documentation publication/search and offline links: 343 canonical pages, zero link errors). The read-only source inventory check passed; 104 pre-existing dirty files outside authorized document edits remained byte-identical. **not_run:** product checks, compile probes and resumed PC acceptance, because this scope creates the plan documents. These checks do not establish product acceptance.

The next implementation action is BC0's decision/owner routing, followed by the actual BC1/BC2 contract corrections. Existing PC1–PC5 work remains dirty and PC6 acceptance stays open. STATUS links to this plan for compilation-cost scope and to the persisted coordinator for its distinct execution boundary.

**Execution checkpoint, 2026-10-08:** ADR-0137 carries forward the unaffected ADR-0136 decisions and installs the confirmed local/provenance target. Architectural owners, agent instructions and the review binding name the qualification-gated transition. BC1–BC5 checks remain pending; the dirty PC1–PC5 baseline is preserved.
