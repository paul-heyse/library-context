# Rust compilation-cost plan: shared execution, scoped identities and qualified local tests

**Principal independent design/target review · 2026-10-08**

**Decision: Accept the Proposed target within the reviewed scope.** The plan selects compatible corrections for all four source-review causes. It separates provider attribution from compiler execution composition, moves native scheduling and streaming behind checked typed boundaries, qualifies the local test policy before making it the default, and groups substantial test drivers without broadening the selected verification contract. These are credible mechanisms for removing avoidable compilation work while preserving semantic and lifecycle guarantees. They do not establish a numerical speedup or unchanged production performance.

The most consequential improvement over a purely mechanical erasure plan is complete argument ownership. The current synchronous writer clones an Arrow batch while its accounting reservation remains with the caller; the synchronous bridge can return on cancellation while submitted work continues. The plan correctly requires the whole checked `Batch<R>` to move into submitted work and migrates synchronous writes, provider sinks, flush, asynchronous push and close together. A smaller future that released its charge early would be an invalid correction.

Acceptance applies to the combined architecture and its specified qualification obligations. The compilation-cost findings retain their source IDs and their sole scheduled disposition at [plan §9](../../plans/rust-compilation-costs-plan_2026-10-08.md#9-sole-compilation-cost-finding-disposition). This review closes none of them. Persisted-execution PC6, product semantics outside the changed boundary, real-library acceptance and measured performance remain outside this decision.

## 1. Review contract, workload and evidence

| Field | Scope |
|---|---|
| Subject | [Rust compilation-cost plan](../../plans/rust-compilation-costs-plan_2026-10-08.md), with decisive native/model/provenance/verification/harness owners and adjacent cold consumers |
| Baseline | Main HEAD `8511eb5fe27498815ce4433fd2d0029aa2e6016e`, plus the identified dirty integrated PC1–PC5 source and new plan, inspected 2026-10-08 |
| Standard | Repository core/template 3.3, Heuristics for Efficient Architecture 1.0, Code Intelligence 1.5 and library-context binding; manifest `docs/design_review/design_principles/standard.toml` |
| Tier · purpose | Design · target |
| Reviewer | Fresh independent delegated design reviewer |
| Functional outcome | A usable ordinary Rust edit/test loop with less avoidable compilation work, preserving optimized production behavior and independent assurance |
| Workload premise | Large native dependency graph, many nominal records and stage futures, repeated source edits, focused controls and full affected integration targets; normal available compiler/test parallelism on one operator's development workspace |
| Growth/failure scenarios | Additional records/stages and closure types; unrelated serving edits; ingestion-only edits; batch cancellation and interrupted drainage; grouped concurrent fixtures; stack-sensitive test-thread and CLI-main entry points |
| Method | Static document, source, manifest and configuration inspection; source-review pinned-tool evidence retained with its stated version and date |
| Excluded effects | Repository implementation edits, builds, probes, tests, synchronization, cache cleanup, process intervention, operator databases and real-library execution |
| Maturity | Target **Proposed**; existing interfaces and realization **Interface-checked / Implemented**, inspected 2026-10-08. No target **Tested** or **Measured** claim |

The reviewer read the shared worker/reviewer contracts, the complete declared review standard and its profile skill, the source review, current STATUS and relevant architectural owners. Prior conclusions were leads: decisive current sources were inspected independently. No probe is needed to settle this document's architectural choice; the plan correctly schedules implementation qualification where behavior or artifact selection could still fail.

## 2. Responsibilities and governing distinctions

| Owner | Responsibility and dependency direction | Reason for change |
|---|---|---|
| Root Cargo configuration | Profile policy, exact resolution, compiler/cache/output settings; Cargo remains resolution and scheduling owner | Deliberate development-policy selection, without changing production optimization or worker policy |
| `cpg-extract` and shared fingerprint helper | Provider production source/dependency/asset attribution; implementation depends on model and analyzer families | Provider/model/analyzer or declared runtime asset changes |
| `cpg-core::workspace` | Attempt-owned output creation, contribution execution identity, native ingestion composition | Compiler/native composition or owned operation changes |
| `NativeCalls` / `NativeBridge` | Submitted request, acknowledgement, cancellation and retained join/drain ownership | Physical request mechanism and lifecycle defects |
| `lctx-model` | Record meaning, generated codec contracts, resource reservation and exact completed contribution/view identity | New semantic record, invariant, producer contract or supported semantic revision |
| `consumed_rows` and stage kernels | Shared read-only batch execution versus typed selected input and stage-owned interpretation | Physical streaming changes versus domain algorithm/coverage changes |
| Artifact inventory and native cold state | Capture and compare completed specifications; admit supported semantics independently of current executable identity | Transport realization and admitted-state changes |
| Verification and integration harnesses | Resolved artifact/profile/case selection, actual fixtures and independent expectations | Local test policy or coherent compiled-target organization |

The model is adequate for these operations. A provider build, its supported semantic contract, captured supplier/configuration binding, an attempt, a completed contribution, an exact dependency view and a physical native realization are distinct. The existing `ContributionSpec` records implementation separately from model/configuration/captured binding; its identity includes the implementation. Completed contribution and view identities carry the effect onward. This is an executable distinction, rather than merely a prose naming scheme.

### Profile fidelity boundary

| Fact/contract family | Provider/revision and fidelity | Coverage, identity and consumers |
|---|---|---|
| Provider observations and supplier assertions | Current pinned provider declarations and `Provider.build_digest`; extracted/resolved/inferred roles remain unchanged | Declared family/profile coverage and captured supplier bindings remain explicit; normal facts production and cold admission consume them |
| Completed compiler contribution | Stage code plus Proposed compiler-composition fingerprint; execution attribution, not a new semantic fact | Existing `ContributionSpec.implementation`, exact inputs/outputs, contribution/view identities and artifact producer inventory |
| Completed stage inputs | Model-owned typed permits and exact immutable views | Record/declaration and epoch matching remain before shared streaming; stage-owned coverage retains unknown/NotRequested distinctions |
| Analytical/behavioral/synthesis outputs | Existing model-owned methods, projections and verdict semantics | No method, projection universe, direction, multiplicity, approximation, evidence link or output interpretation changes are selected |

No new analysis record is introduced. The profile's projection/method/model/budget/evidence columns retain their current owners; erasing a physical request or loop is not permission to narrow their universe or relabel their results. Full extracted-family or served-answer qualification is not established by this review.

## 3. Contracts and composed execution

### Charged native operations

[`NativeCalls::call`](../../../crates/cpg-core/src/native_calls.rs) currently sends a concrete request future into `tokio::spawn`, then boxes its join future. [`NativeBridge`](../../../crates/cpg-core/src/native_bridge.rs) already accepts an erased `BoxFuture` before `JoinSet::spawn`. The plan reuses this composition pattern while preserving separate runtime owners. A thin result adapter can construct typed acknowledgement and caller-await state; the shared submitted path need not specialize its task registration and drainage on every record/closure.

The plan specifies the complete failure contract: stop new submissions on cancellation, retain already submitted work, mark attempt failure before acknowledging failed operations, report late failures whose receiver disappeared, retain failed joins, and preserve ownership when drainage is interrupted and retried. Successful acknowledged calls may retire; a finished flag alone cannot hide an unfinished or failed join.

[`Writer::write_arrow`, `flush`, `close`, `ProducerOutput::write` and provider adapters](../../../crates/cpg-core/src/workspace.rs) confirm why owned batch migration must accompany erasure. `write_arrow` clones `RecordBatch`; `write`/`contribute` borrow `Batch<R>`; the sink already receives an owned batch but borrows it again. On bridge cancellation the waiting call can return before its submitted work terminates. Moving `Batch<R>` into the erased submitted future keeps rows, Arrow storage and reservation together. Existing owned asynchronous push/close behavior is a preservation constraint, not a replacement target to weaken.

Contribution registration has one record-independent descriptor/registration operation, shared by synchronous/asynchronous and empty completion routes. The typed adapter still checks model/allowed output and constructs the typed writer. Single registration and writer visibility ordering remain explicit. The plan's registration, duplicate-output and empty-output controls address this real concurrency boundary.

For reads, [`consumed_rows`](../../../crates/cpg-core/src/consumed_rows.rs) confirms that SQL ordering, read-only planning, execution, iteration and yielding are currently repeated beneath generic record/callback types. A borrowed batch callback is sufficient after exact permit/declaration selection; typed decoding and coverage stay at their semantic owner. Batch-level dispatch adds bounded per-request/batch work instead of per-row dispatch. Large remaining stage futures can be split by coherent load/compute/emit responsibility rather than by indiscriminate boxing.

### Scoped provider and execution identity

[`producer_fingerprint.rs`](../../../scripts/producer_fingerprint.rs) currently enumerates all workspace crates and unrelated siblings. [`cpg-extract/build.rs`](../../../crates/cpg-extract/build.rs) embeds that result upstream. The plan's manifest-rooted normal/build closure removes this reversed invalidation relationship while retaining conservative optional/target roots, workspace inheritance, local patch sources, root lock/manifest sensitivity and declared assets. Manifest parsing with the existing TOML dependency is an appropriate bounded adapter; recursive Cargo invocation or a second feature/version resolver would add ownership and build-lock problems without a need.

The selected composition root is credible: [`cpg-core/Cargo.toml`](../../../crates/cpg-core/Cargo.toml) directly includes extraction, model and native store. It therefore covers ingestion changes that provider-emitted rows alone cannot attribute. Composing stage implementation and this digest at `Workspace::output` gives every ordinary workspace output one governing execution identity. Raw provider hashes and `Stage.code` remain supplier/stage attribution. Serving/publisher-only source changes need not affect either production root.

The existing hash field is sufficient. [`ContributionSpec::identity`](../../../crates/lctx-model/src/domain/completed.rs) includes `implementation`; [`producer_inventory`](../../../crates/cpg-core/src/artifact_manifest.rs) reads that field; [restored artifact admission](../../../crates/cpg-core/src/artifact.rs) compares the retained inventory. No consumer needs to parse components from the digest, and no new wire column follows from its new construction. Imported specifications must bypass current-build recomposition and retain their captured value exactly. Model semantic compatibility and native schema realization remain independently owned.

Whole root manifest/lockfile sensitivity still invalidates more than a minimal resolved dependency set. That conservative scope is deliberate, useful for complete provenance, and substantially narrower than watching unrelated downstream sources. Finer resolution needs a concrete consumer and complete evidence; a new resolver is not justified by this correction.

### Local profile and artifacts

[`Cargo.toml`](../../../Cargo.toml) currently has O2 incremental workspace and O3 non-incremental imported settings; test only overrides debug information. The candidate's O1, explicit LTO off, incremental, debug-zero, enabled assertions/overflow settings distinguish the frequent local learning loop from optimized production acceptance. Imported/build optimization initially remains stable. The plan accounts for override precedence and possible initial rebuilds rather than promising cache compatibility across changed flags.

The temporary candidate is qualification state. Actual test defaults change only after revealing normal-stack controls pass; then the candidate disappears and bare Cargo/nextest selection is checked again. Known O0 test-thread and CLI-main stack receipts justify excluding an automatic O0 fallback, but do not qualify O1. A child stack enlargement cannot stand in for normal test/main behavior.

[`verify.py`](../../../scripts/verify.py) currently hardcodes release in run/list/build/doc steps and release-directory environment expansion. The plan correctly gives one resolved profile authority responsibility for commands, paths and receipts, including intentional target-directory overrides. Test/dev directory sharing makes file presence insufficient evidence of freshness. Candidate CLI controls must build and invoke that actual candidate, and optimized acceptance must refuse a silent local override.

The explicitly release-backed [CPython flow oracle](../../../tests/scripts/test_flow_soundness.py) and optimized extension controls remain separately declared production steps. This avoids an inaccurate claim that a Python boundary selects O1 simply because neighboring Rust tests do. `just qualify` remains release-resolved for its required Rust artifacts and applicable leaves.

### Compiled test reuse

Source inclusion inspection matched the plan's 42 extraction and 19 core top-level driver consumers. The six extraction/five core groups place one substantial common driver in each binary while retaining topic modules and independent expectations. This is a credible middle ground between repeated executables and a single giant harness. Non-driver and pure flow controls stay separate.

The current provider boundary selects library tests, acquisition/bundle and six specific topic modules. Grouping those modules into larger binaries does not authorize running all neighboring topics: mandatory module filters and user-filter intersection must preserve that boundary. The plan explicitly requires discovery and selected-case equivalence. Cargo target selection limits compiled executables; nextest filters independently limit executed cases.

The shared native fixture helpers use `OnceLock` runtimes but create a fresh `NativeCompilerStore` for each attempt. Grouping can merge runtime/static context in ordinary Cargo test execution; the plan therefore retains independent namespace/state ownership under normal parallelism. Correcting any actual shared-state interference is preferable to caps, serial locks or weaker assertions.

## 4. Revealing change and failure scenarios

| Scenario and kind | Correct owner and propagation | Assessment and settling evidence |
|---|---|---|
| Add a record to an existing stage — domain extension | Model/codec/stage declaration and new domain behavior; shared scheduler and SQL mechanics unchanged | Credible Proposed route; BC1 source/symbol inspection plus independently expected rows and decoder/input controls |
| Change serving-only implementation — mechanism edit | Serving and actual downstream consumers | Provider/composition roots remain stable; BC2 mutation and build-input controls establish this |
| Change ingestion with identical emitted rows — composition change | Compiler/native root; new contribution execution identity | Supplier identity stays scoped; composed implementation changes and cold inventory retains it |
| Import a supported prior-build artifact — transport instance | Captured completed specification and supported semantic admission | Current executable cannot replace its captured hash; BC2/BC5 cold restore under a different build |
| Cancel a sync/async write after submission — failure | Submitted request and drain owner | Charge remains held until terminality; observed reservation lifetime and late-error/retried-drain controls required |
| Run candidate tests and CLI — policy substitution | Root profile and resolved verification artifacts | Both actual test and main entry paths use normal stacks; qualification precedes installed defaults |
| Edit a driver or one topic — test mechanism change | Coherent target group | Driver compiled once per group; topic changes affect their group, without changing case discovery/selection |
| Increase records/batches while requests overlap — workload growth | Typed encoding plus bounded shared submission/streaming | No per-row erasure or extra stage/object per kind; request-held charge and existing capacity controls govern live state |

These scenarios challenge the consequential remedies with valid work: cold artifacts from another build remain supported, cancelled callers do not uncharge live native writes, new types remain nominally checked, and provider selection does not broaden accidentally. Existing exact immutable views and read-only SQL are essential preservation constraints.

## 5. Library fit and alternatives

Cargo/rustc, Tokio/futures, Arrow/DataFusion and the generated typed model already supply the appropriate categories of capability. The plan improves their composition rather than introducing a scheduler, row registry, second store or general build framework. Exact pinned-tool facts are attributed to the [source review's library evidence](design_review_rust-compilation-costs_2026-10-08.md#exact-compiler-and-library-references) and the plan's pinned links; this review makes no fresh cross-version transfer or new timing claim.

| Alternative | Benefit and burden | Judgment |
|---|---|---|
| Retain present baseline | Avoids migration, but preserves demonstrated scheduler specialization, upstream invalidation and repeated drivers | Insufficient for the intended local workload |
| Selected checked erasure and scoped fingerprints | Removes shared generic task/loop expansion and unrelated provenance watches; introduces batch-level allocation/dispatch and bounded manifest parsing | Preferred, with charged ownership and complete dependency evidence |
| Purely monomorphic extraction without typed adapters | Could reduce generated work further, but transfers nominal matching/coverage into runtime interpretation | Unjustified semantic and hot-path burden |
| Keep release-only local tests | Coherent artifacts but keeps optimizer work in frequent local controls | Viable fallback until candidate passes; does not close F03 |
| Permanent separate local profile | Explicit selection but duplicate standing default/policy machinery | Temporary candidate followed by ordinary test default is simpler for the selected operator behavior |
| Compiled support crate | Can share code across targets, but adds a public/support dependency surface and possible extraction/core test cycle | Revisit only if coherent grouping leaves demonstrated duplication |
| One giant harness | Maximizes common source reuse but worsens topic rebuild locality and risks an oversized unit | Not required; selected coherent groups are preferable |
| Worker tuning, blanket nonincremental builds or arbitrary crate splits | Can move remaining work but does not remove the diagnosed amplification; changes cache/runtime obligations | Conditional residual investigation, not a substitute for the selected corrections |

A4 is supported qualitatively by removed work and bounded replacement work. One allocation/dynamic call per native request or batch is a credible trade for compiled shared mechanics; per-row codec/domain loops remain concrete. Operational qualification must still detect a material production regression. No per-package numerical cost model or separate benchmark campaign is required to accept this target.

## 6. Independent gates and foundations

Gate passes below apply to the specified Proposed contract and its inspected integration route, not to completed implementation.

| Gate | Verdict | Evidence and limit |
|---|---|---|
| G1 Authority | **Pass, scoped static** | Model owns semantic contracts; provider owns raw identity; workspace owns composed execution hash; one profile owner and one finding-disposition owner |
| G2 Semantic fidelity | **Pass, scoped static** | Typed permits/codecs/coverage stay governing; execution identity is not semantic compatibility; captured cold identities remain exact |
| G3 Validity | **Pass, scoped static** | Shared model validation, allowed outputs and read-only input matching preserved; revealing invalid/duplicate/empty controls scheduled |
| G4 Hidden behavior | **Pass, scoped static** | Build effects and profile/artifact selection explicit; manifest walker does not recursively invoke Cargo; no operator action or sync implied |
| G5 Consistency/recovery | **Pass for the Proposed boundary** | Owned complete batches, submission guards, retained joins and drainage remain required before cleanup/publication; current sync defect is explicitly scheduled |
| G6 Transformation/reuse | **Pass for the Proposed boundary** | Scoped closure retains conservative complete dependencies; imported specs are unchanged; local profiles are qualified separately; discovery/selection equivalence retained |
| G7 Truthful claims | **Pass** | Proposed implementation, historical evidence and future qualification remain distinct; no measured speedup or completed PC claim |
| G8 Library leverage | **Pass** | Established task/build/stream/parser capabilities remain owners; bespoke code only binds repository provenance/lifecycle contracts |
| CI-G1 Fidelity | **Pass for preservation and attribution scope** | Supplier/semantic/execution distinctions explicit; no row or verdict meaning changes; full product fidelity not assessed |
| CI-G2 Evidence closure | **Not applicable to a changed served-claim path** | No new served claim/evidence operation is selected or certified; exact snapshot identities remain a constraint |
| CI-G3 Evaluation integrity | **Pass for the selected scope** | No protected input, evaluator meaning or expectation derivation changes; independent semantic assertions preserved |

**FP-01–FP-07 satisfied within the Proposed target.** The scenario routes keep ownership coherent and local, typed domain meaning governing, compositions explicit, and physical execution units separate from semantic kinds. **DP-01–05, DP-08–10, DP-13–24 satisfied at this specification boundary**, insofar as relevant to identity, transformation, qualification, lifecycle and change. This includes the conservative dependency choice under DP-09 and batch-level erasure tradeoff under DP-10/20. **CI-01/02/04/08/10/12/13 preservation satisfied within scope.** Relationship/projection/algorithm rules not changed by the target retain their current owners; this review does not certify their enclosing implementation.

## 7. Findings, dependencies and qualification

**No new blocking target-plan finding was identified.** Original compilation-cost F01–F04 remain separate diagnoses, with current scheduled disposition only at [plan §9](../../plans/rust-compilation-costs-plan_2026-10-08.md#9-sole-compilation-cost-finding-disposition).

| Source finding | Target assessment | Closure evidence still required |
|---|---|---|
| [F01](design_review_rust-compilation-costs_2026-10-08.md#F01) | Selected shared request/registration/streaming boundary is coherent; owned synchronous writes correct a concrete preservation gap | All caller migration, source/emitted-symbol evidence, exact inputs/output controls, charge lifetime, late failures and retried drainage, operational runtime adequacy |
| [F02](design_review_rust-compilation-costs_2026-10-08.md#F02) | Relevant provider closure and workspace execution composition fit existing identity/cold consumers | Dependency/asset mutation, membership/deletion/relocation, unrelated stability, ingestion-only effects and different-build cold transport |
| [F03](design_review_rust-compilation-costs_2026-10-08.md#F03) | Qualified O1/LTO-off candidate followed by test default matches the selected local/production distinction | Effective profiles, actual candidate CLI, normal stacks/parallelism, default installation and retirement, truthful release-only steps and final production acceptance |
| [F04](design_review_rust-compilation-costs_2026-10-08.md#F04) | Explicit groups match current top-level driver membership and preserve case scope | Each old test discovered once, single driver inclusion, filter intersection, fixture independence and all affected grouped controls |

BC1 and BC2 remove independent kinds of work: request specialization versus invalidation frequency. BC3 runtime/stack qualification depends on BC1's coherent future boundaries; configuring candidate selection earlier does not discharge that dependency. BC4 mapping can proceed before write migration, but migrated test callers depend on BC1 interfaces. BC5 integrates their semantic/transport/profile effects on one tree. BC0 must carry the settled rule impacts to enduring owners before dependent changes become active.

The relationship to PC is appropriate. Current completion/captured-binding contracts are prerequisites, while unrelated pending PC6 acceptance is not a reason to preserve avoidable compiler work. A valid final-tree release receipt can serve both plans, but only its actual profile/cases/tree determine applicability. Compilation-cost acceptance cannot close restored-coverage or native/MCP obligations through an old or differently profiled receipt.

**Implementation checks: not_run.** Planned commands and selections are owned by [plan §7](../../plans/rust-compilation-costs-plan_2026-10-08.md#7-verification-and-acceptance), including focused `just verify --select …`, actual fixture-backed controls, candidate `--cargo-profile local-test-candidate` / Cargo `--profile local-test-candidate`, and final release-resolved `just qualify`. The root owns documentation checks and turn-end maintenance; none was run by this read-only reviewer.

Residual dominant-pass/toolchain/harness-skew questions have stated triggers and do not undermine the selected structural route. Runtime dispatch effects, exact closure membership, actual artifact selection and grouped fixture behavior are implementation qualification uncertainties, not silently accepted performance facts. A failed candidate must be repaired and rerun before default installation. A materially regressed production operation reopens the physical boundary instead of being offset by compilation savings.

## 8. Rule impacts and decision

This review introduces **no additional operator rule choice**. The accepted source-review rule impacts remain the applicable decisions; the identifiers below retain their source, rather than creating similarly numbered new decisions.

| Source impact | Rule surfaces and required change | Dependency / if retained |
|---|---|---|
| [Compilation-cost RC01](design_review_rust-compilation-costs_2026-10-08.md#RC01) | Successor/complementary route for ADR-0136, DESIGN §1.2, AGENTS testing guidance, binding §4's release-profile instruction and live verification/documented command surfaces. Permit the qualified local default while requiring explicit optimized acceptance. Preserve unaffected pins/cache/worker/fixture obligations | F03 and profile-selection changes depend on it. Retaining universal release-only local guidance leaves F03 unresolved; structural corrections still stand |
| [Compilation-cost RC02](design_review_rust-compilation-costs_2026-10-08.md#RC02) | Replace whole-workspace provider source membership through acquisition/extraction §4 and semantic model §15 ownership. Complement ADR-0135 without rewriting its accepted supplier/semantic/cold distinctions | F02 depends on it. Retaining the broad provider watch leaves unrelated invalidation unresolved; native composition cannot substitute for missing supplier attribution |

The owned `write`/`contribute` API and shared physical mechanisms are deliberate internal source cutovers. They need complete caller/deletion migration but no compatibility borrowing path, new row authority, transport version bump solely for the opaque hash, or automatic operator reconstruction. Changed field meaning must be documented at its enduring owner even when the wire shape stays the same.

| Judgment | Verdict | Scoped reason |
|---|---|---|
| A1 Localize change | **Satisfied** | Relevant source roots restore rebuild direction; coherent groups and shared physical mechanics bound ordinary extension/rebuild propagation |
| A2 Encode domain meaning explicitly | **Satisfied** | Existing typed records/permits, semantic contracts, captured suppliers, composed execution identity and charged terminality govern the Proposed operations |
| A3 Extend through composition | **Satisfied** | Model/stage additions reuse common request/stream machinery; Cargo/Tokio/DataFusion remain physical owners; cold specifications reuse captured identity |
| A4 Fit execution to the workload | **Satisfied, qualitative Proposed target** | The four mechanisms remove distinct demonstrated amplification without per-row dispatch, alternate scheduling or broadened test state; replacement work and production qualification are proportionate |

**Bounded decision: Accept the Proposed compilation-cost target within scope.** This enables BC0's root-owned decision/owner updates and implementation of the selected boundaries. It does not qualify O1, close F01–F04, establish unchanged runtime performance or certify the enclosing persisted/product architecture.

**Enclosing architectural status:** PC completion, all product semantics, persistence/serving journeys, real-library qualification and measured performance remain with their current owners and unresolved acceptance boundaries. Reopen this target judgment if implementation requires dropping charged ownership, narrowing necessary semantic input, redefining cold compatibility, retaining duplicate provenance authorities, broadening declared controls or accepting a material production regression.

**Publication:** `docs/design_review/reviews/design_review_rust-compilation-costs-plan_2026-10-08.md`. The coordinator owns reconciliation and publication while preserving this independent judgment.
