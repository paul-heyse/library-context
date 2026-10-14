# SurrealDB lifecycle completion: integrated design review

**Independent design/target review · 2026-10-12**

**Current frozen-source decision: Accept scoped, at Implemented / Interface-checked strength; see §14.** The initial 2026-10-12 acceptance missed overlapping-root reconstruction, and §13 amended it to Revise for IF10. Independent reinspection of the replacement source now accepts the bounded correction. The initially inspected lifecycle candidate provides a coherent route from exact completed native contributions through protected selection and evidence access. Its completion change shares independent contributor/blob observations across the complete singleton/union installation batch while preserving each view’s own validity checks and effects. Allocation accounting distinguishes temporary construction from retained typed and native state. Inspection preserves independent outcomes and reports its provenance without acquiring native release authority.

The initial assessment identified no new material finding, but missed repeated full logical-normalization, dependency-view and alias reconstruction across overlapping outputs. Section 14 accepts the implemented replacement at source strength; IF10's current disposition and required runtime evidence remain solely with the lifecycle coordinator. No rule change is required. This review does **not** establish completed LC01 functional acceptance. Actual generated Catalog admission, unselected publication and populated linked native queries remain pending. A passing test body with an uncertain or failed enclosing fixture remains insufficient.

## 1. Scope, baseline and method

| Field | Assessment |
|---|---|
| Functional intent | A version-pinned API and evidence catalog that preserves provider fidelity, exact selected content and traceable evidence while supporting repeated compilation, publication and native querying. |
| Subject | Integrated lifecycle foundations and the final remaining-functional candidate: native/compiler completion, protected selected access, original-byte observation, typed failure diagnostics, passive inspection/provenance, development-only beta integration and owned target cleanup. |
| Standard | Core principles/template 3.3; efficient-architecture heuristics 1.0; code-intelligence principles/review additions 1.5; library-context binding. |
| Tier and purpose | Design / target. Existing decisions and operator rules describe the subject; the standard and functional outcome determine this judgment. |
| Reviewer | Independent delegated design reviewer, read-only. |
| Baseline | Main `9a98aaa051bd6588f6955044cd1f46731100d3b2` plus the frozen dirty production candidate identified below. |
| Accepted implementation foundations inspected | Native `71f49c73637bec7d75c73e40f1078fab63361937`; development tooling `72d58d2e649c74a26cb079cbdb5387d6be402f30`; integrated lifecycle handoff through baseline main. These commits identify source, rather than establishing acceptance by themselves. |
| Method | Independent examination of current owners, four lifecycle plans, production source, affected consumers, control definitions, exact dependency manifests and selected pinned library implementation. No builds, probes, service connections, mutations or delegation. |
| Explicit limits | Embedding execution and MCP-submitted query execution are outside this runtime review. Their broader correctness is not certified. No real-library, whole-product, backup/restore campaign, operator activation, measured performance or historical qualification claim. |

The workload premise is a personal, locally operated catalog compiling pinned code and documentary evidence into multiple exact native views, with overlapping contributor dependencies and original-byte ranges. Growth includes more contributors, output relations, original chunks, large supported rows, high-degree evidence relationships and concurrent equal compilation. Readers may remain pinned while another publication is produced. Resources must follow actual live allocations; complete observation and physical terminality remain required.

The review examined the four current lifecycle plans and the relevant semantic-model §15, storage/publication §6 and synthesis/serving §11 owners. Source coverage includes `lctx-native` installed-view audit/live installation, originals, selected access, reader, control/retry and diagnostics; compiler completion/installation; serving member discovery and evidence hydration; publication backup as an adjacent contract; and development worker/service/run integration. Search covered affected selected-access call sites and logical view bindings; a remaining binding was inspected as query metadata feeding protected candidate access, rather than presumed to be a bypass from its name.

### Frozen candidate identities

These are Git blob hashes of the inspected working files, obtained with `git hash-object`; they are not commits.

| Path | Blob |
|---|---|
| `crates/lctx-compiler-store/src/compiler.rs` | `1b06a4dfe4afd3b8529c1e2f3ec421896503346a` |
| `crates/lctx-native/src/installed_view/live.rs` | `93acd5942cee24b15f144a8911b1d3ab26f71661` |
| `crates/lctx-native/src/control/authorization.rs` | `38c359865c9895f6792dbf7b1320c4d83ef8de7a` |
| `crates/lctx-native/src/control/retry.rs` | `f8c5450a21f10626befebead618a2e77b6620bad` |
| `scripts/surrealdb_development.py` | `f2c2eabaf5639551dfc78277bca34a464d7d98e0` |
| `scripts/surrealdb_service.py` | `ae70df58137b72e16cd0f48ceb5f91013e4b41d7` |
| `tools/lctx-surreal-dev/build.rs` | `38abeb4c9dea5b522110e9ea4796aea8f4860014` |
| `tools/lctx-surreal-dev/src/lib.rs` | `5e7dc4434ddd8d8aef824a9fa5216b28f05a8b26` |
| `tools/lctx-surreal-dev/src/main.rs` | `5cb56dfd75c5b8b1366104999987d55bbe649e66` |
| `tools/lctx-surreal-dev/Cargo.toml` | `0589d1ff3b47cceabbbf4836df8001207576284c` |
| `crates/cpg-core/tests/facts_admission.rs` | `925b708aaad8fb983a386bbff5614ef919456825` |
| `tests/scripts/test_surrealdb_development.py` | `7a817ac631d0f4456d4c3cd7bc9a4981b8b6e8e0` |
| `tools/lctx-surreal-dev/tests/fixtures/token-cancellation/tests/config.toml` | `0a106de86ec8272a3392f0e50ea7f4a470e8d539` |
| `tools/lctx-surreal-dev/tests/fixtures/token-cancellation/tests/suites/cancel.toml` | `a65d86c3a84f0ee5ff6407a8fd85fa8588265ba2` |

The pinned server source examined independently resolves to upstream `238bfeb11f5725bebed370167656748df8067595`, beneath the source root identified in [library-contract evidence](../evidence/2026-10-12_surrealdb-lifecycle-target/library-contracts.md#source-identities). Local SDK/RPC patches and server execution-drain changes remain distinct from that upstream identity and from adoption by a running executor.

## 2. Domain model and authority

The model adequately separates the phenomena that matter to these operations. A logical contribution or view is not its physical realization, an independently observed completion is not semantic admission, publication is not selection, and local command termination is not native certainty.

| Owner | Responsibility and consumer contract |
|---|---|
| `lctx-model::domain` | Meaning, canonical identity, completed descriptors, provider/profile/coverage semantics, validation and structured completion/failure vocabulary. |
| Native control/runtime | Issuance, exact request identity, incarnation/phase, holds, pins, reconciliation and physical terminality. |
| Installed-view audit/live installation | Independently observe actual selected backing; construct and retain exact physical completion under current native authority. |
| Compiler store | Compose singleton/cumulative views and exact physical mappings; establish compiler admission through its distinct owned checks. |
| Publisher | Commit an immutable manifest over admitted content and executable definitions; preserve reader and recovery obligations. |
| Serving | Interpret library/member/path requests and evidence operations using protected native selection. |
| Service/run/storage owners | Authenticate host processes and targets, retain work and evidence, drain exact children and dispose wholly owned targets. |
| Development worker | Apply generated schema and run supported beta operations against an issued campaign binding. It cannot grant product admission or host disposal. |

Dependency direction remains appropriate: native wrappers enrich the model’s diagnostic projection; model code does not depend on native implementation errors. Compiler and serving consumers invoke selected-access and completion owners rather than reconstructing native lifecycle permission from identifiers.

### Fact and fidelity assessment

| Information | Authority and identity | Fidelity and uncertainty | Consumers |
|---|---|---|---|
| Provider facts and coverage | Captured provider/revision/profile/configuration and exact completed input views | Extracted/resolved distinctions remain owned upstream; incomplete coverage is not absence. | Compiler admission, Catalog and selected operation preparation |
| Physical contribution/view completion | Full descriptor, selected mapping, incarnation, installation validity and actual backing | Independently observed physical completeness; does not certify domain admission. | Compiler predecessor access, exact readers and publication |
| Original source evidence | Source/layout association, full blob identity, range and digest | Exact captured bytes; shared bytes do not merge source identities or range obligations. | Evidence hydration and independent admission |
| Derived evidence assertions | Model-owned derivation and selected assertion identity | Derived claims retain their own premises and qualification. | Evidence packets |
| Failure diagnostics | Model-owned class/code/stage with native-owned optional identity enrichment | Known typed causes, explicitly opaque foreign causes and explicit truncation. | Persisted native receipts and host inspection |
| Worker provenance | Physical worker hash plus compiled dependency/profile/schema response | Recorded build inputs and compiled schema correspondence; server remains separately observed or uninspected. | Development ledger and passive inspection |

The source does not promote an allocation reservation, diagnostic projection, build-info response, beta report or host disposal receipt into content authority.

## 3. Contracts, lifecycle and independent outcomes

`InstallationAudit` owns a real native pin, exact contributor observations and blob-incarnation observations. Its checks require the same native session and resource pool. Reuse validates current expected rows/incarnations; a stored observation cannot become portable admission. Its drop path retains uncertainty when explicit closure has not succeeded (`installed_view/audit.rs:28–86`).

The final compiler batch operation:

1. Acquires an owned execution lease.
2. Deduplicates exact view identities while rejecting conflicting descriptors.
3. Creates one audit when fresh live installation requires it.
4. Passes that audit through singleton and union installations.
5. Explicitly finalizes the audit and composes closure failure with the primary result.

This is an appropriate operation boundary (`compiler.rs:1207`, `compiler.rs:4764`). Cached, physically completed and published branches retain their existing exact held-completion routes (`compiler.rs:1410`). Each fresh view still validates its descriptor, mappings, aliases, root members, completion metadata and final effect. Sharing contributor/blob observation does not share a different root’s permission to exist.

`complete_blob` retains the original incarnation across recapture. Only a conclusively returned typed `PremiseChanged` result permits another capture; retirement, replacement and uncertain issued effects refuse or reconcile through their existing owner (`originals.rs:436–526`). Equal replay can conservatively invalidate the captured revision. This supports bounded peer convergence, while retaining the separately identified IF04 limitation under continuous replay.

The outcome axes remain independent:

| Axis | Meaning and enforcement |
|---|---|
| Command/body outcome | Whether the requested worker or test body succeeded. |
| Local execution cleanup | Whether actual admitted children and execution owners drained. |
| Native certainty | Whether issued effects and finalizers are acknowledged; local exit cannot establish it. |
| Committed effects | Acknowledged committed work survives later cleanup failure. |
| Report/capture health | Whether expected diagnostic/report material exists and is usable. |
| Credential restoration | Whether the exact owned temporary token input was restored. |
| Whole-target disposal | Whether the exact owned executor stopped and its disposable state was discarded. |

The model’s `Completion` and `OperationFailure` preserve primary and finalization failures, local/remote state, committed effects and storage obligations. Typed failure projection bounds cause count and stage size, omits arbitrary error display text from machine classification and retains truncation (`domain/diagnostic.rs:110`). Native enrichment uses actual wrappers and issuance identity (`lctx-native/src/diagnostic.rs:11`); the signal writer persists that projection beside existing outcome fields (`signal.rs:257`).

The corrected facts controls compose workspace execution drainage with actual native attempt/session abandonment (`facts_admission.rs:50`). That is the required distinction between finishing compiler execution and acknowledging its fixture ownership.

## 4. Composition and execution fit

The initial frozen-source assessment considered the physical route credible because independent membership/blob observations follow unique protected contributors and blobs within the operation. That execution-fit conclusion was incomplete: it did not account adequately for repeated full logical-root and alias reconstruction. Section 13 amends A4 and the affected principles; IF10 requires correction before accepting this workload route.

Membership auditing streams the complete contribution-indexed inventory once per first observed contributor, checks monotonic ordering and declared output counts/digests, and validates backing/guards/holds/endpoints in bounded windows. Stream drainage precedes publishing an observation into the audit cache (`installed_view/audit.rs:557–725`). This avoids a scalar network request for each membership without replacing independent correspondence checks with count equality.

Original handling shares the full-blob completeness observation under exact pins/incarnations. Each typed original still checks its source/layout, coverage and range digest (`installed_view/audit.rs:1088`). More ranges therefore retain their necessary association work without repeatedly rescanning every piece for full-blob completeness.

The batch fixes one composition seam: singleton installation and cumulative unions formerly entered independent audit scopes. All those consumers now share one protected audit. However, additional output relations still multiply complete membership/backing reconstruction in `logical_roots_many`, repeated dependency-view reads and complete alias reconstruction. Sharing the existing contributor/blob checks alone does not settle the overlapping-output execution-fit obligation.

Allocation reservations now distinguish serialization scratch from retained typed descriptor capacity and independently retained native expected rows (`installed_view/live.rs:76–113`, `live.rs:294–340`). Settlement occurs after checked effects and retains mapping capacity, relation capacity and conservative tree allowance. This is allocation admission, not allocator measurement or a measured capacity claim.

Selected serving access preserves a complete selected universe. Library and adjacency queries nominate physical identities; native access intersects them with held selected membership before canonical hydration (`selection.rs:278`, `evidence.rs:566`, `reader.rs:956`). Foreign same-nominal content cannot acquire eligibility merely by appearing in a nomination. Candidate ordering can spill; bounded output does not substitute for examined-work ownership.

### Analysis and projection record

| Question | Projection and settings | Fidelity/model | Budget and outcome | Evidence linkage |
|---|---|---|---|---|
| Is this completed physical namespace valid? | Exact declared view/dependency mappings; full contributor outputs and originals; deterministic canonical comparison | Independent exact physical observation under current native authority | Charged windows, retained observations and checked EOF; refusal/uncertainty is not completion | Held completion binds exact descriptor, roots, mappings and incarnations |
| Which members/evidence are eligible? | Operation-specific indexed nominations intersected with the exact held publication inventory; relation roles and directions preserved | Exact selected eligibility; no new heuristic interpretation | Caller budget, owned streams/sorting and explicit errors | Returned canonical payloads remain under selected reader ownership |
| What happened to this operation? | Typed bounded cause projection and separate host/run/native axes | Observation, not semantic admission or release | Explicit opaque/missing/uninspected/truncated states | Exact recorded identities and inspect routes |
| Which worker/schema produced this run? | Compiled lock/patch-profile metadata and embedded schema identity | Build provenance, not running-server attestation | Read-only build-info response; invalid/missing correspondence refuses or reports unknown | Physical worker hash retained in the development ledger |

No quantitative performance conclusion follows. Concurrent host activity and timeouts cannot establish a speed ratio. A4 here rests on the changed work structure, retained guarantees and a credible bounded execution route.

## 5. Change and failure scenarios

| Scenario and change kind | Owning route and affected consumers | Assessment |
|---|---|---|
| Add a deployment-evidence family — domain extension | Model declarations and genuinely new provider/derivation semantics; existing completed-view, selection and evidence contracts carry its identity and uncertainty. | Semantic changes belong to domain owners. No automatic new lifecycle queue, service or proof registry is required. |
| Grow overlapping outputs/original ranges — workload growth | Compiler batch composes per-view installation with shared contributor/blob observations. | Initial assessment covered those observations and allocation lifetimes, but missed repeated logical-root/dependency-view/alias reconstruction; IF10 and §13 amend execution-fit acceptance. |
| Add a pinned release — instance/binding | Acquisition/provider context produces new exact views and publication identity. | Existing reader/reference meaning remains pinned. Same nominal names cannot silently select another physical copy. |
| Publish B while A has readers — composition/concurrency | Publisher commits B; A’s reader and recovery owners retain A’s exact realization. | Publication does not reinterpret existing references. Retirement belongs to native reachability and actual owners. |
| Substitute native references for holds — mechanism substitution | Requires complete incarnation, admission/retirement exclusion, reconciliation and release semantics. | Built-in references provide useful reverse relationships, but do not by themselves discharge this complete lifecycle contract. Retaining current holds avoids duplicate retention authority. |
| Equal peers and replay — concurrency | Typed completion outcome permits same-incarnation recapture under existing cancellation/contention policy. | Bounded peer completion is supported. Guaranteed progress under indefinitely sustained replay is excluded; IF04 owns the refinement trigger. |
| Cancel after token substitution — failure | Worker journals exact original input; host restores through the bound development ledger after authenticated child drainage. | Source route preserves exact scope and caller source. The static fixture supplies an observable query window; its existence is not an actual beta cancellation pass. |
| Logical drain fails during owned-target disposal — failure | Managed bounded logical actions retain their actual run owners; exact executor shutdown/discard settles whole-target retention. | Failed logical drain and original native uncertainty remain recorded. Unknown local drainage or cancellation prevents destructive continuation. |
| Inspect a closed or missing target — presentation/diagnosis | Passive ledger projection reports missing/uninspected state; explicit native diagnostics require the existing admission route. | Inspection neither opens native admission nor manufactures cleanup success. |

These scenarios support local change and composition without promising inexpensive replacement of every engine or analyzer.

## 6. Independent gates

All pass judgments below are **Interface-checked/source judgments dated 2026-10-12**, limited to the inspected supported behavior.

| Gate | Verdict | Independent basis |
|---|---|---|
| G1 Authority | Pass | Model meaning, native lifecycle authority, compiler admission, publication and host cleanup have distinct owners. Generated assets and passive projections retain their source. |
| G2 Semantic fidelity | Pass | Logical/physical identities, completion/admission, absent/unknown/failed and independent outcome axes remain distinguishable. |
| G3 Validity | Pass | Exact descriptor collisions, selected mappings/incarnations, backing/hold correspondence, complete output digests and final checked effects retain enforcing boundaries. |
| G4 Hidden behavior | Pass | Passive inspection and compiled build-info do not connect to a service. Native plan/index inspection is explicit. Schema/test effects require an issued campaign. |
| G5 Consistency and recovery | Pass, scoped | Checked stream terminals, exact pins, original request reconciliation and authenticated disposal prevent partial work becoming success. Sustained-replay progress is excluded. |
| G6 Transformation and reuse | Pass | Audit reuse is operation-local and premise-bound; root/layout/range checks remain independent. Selected hydration preserves physical eligibility and evidence identity. |
| G7 Truthful capability claims | Pass | Compiled worker/schema provenance remains separate from server observation and runtime qualification. Missing and unsupported paths do not silently fall back. Pending acceptance remains explicit. |
| G8 Library leverage | Pass | Native indexed/streaming operations, library parsing/serialization, existing run/storage ownership and beta schema/test capabilities supply generic mechanisms; custom code closes domain lifecycle gaps. |
| CI-G1 Fidelity | Pass | This change does not promote extraction or structural evidence into runtime proof, heuristics into facts, or incomplete coverage into absence. |
| CI-G2 Evidence closure | Pass, source scope | Selected evidence nomination is resolved through the held exact inventory; derivation hydration cannot independently widen it. Actual generated publication/query acceptance remains pending. |
| CI-G3 Evaluation integrity | Not applicable | No evaluator, protected population or production expectation source changes. The token fixture is a lifecycle input, not catalog truth. |

## 7. Findings and principles

**Initial frozen-source assessment identified no new material finding; amended by §13 for IF10.** Current disposition remains in coordinator §6; this review is not a second disposition ledger.

The frozen candidate addresses the shared audit-lifetime portion of original [F03](design_review_surrealdb-lifecycle-target_2026-10-12.md#F03) at source strength. It does not settle full overlapping-output reconstruction or close required runtime evidence. F03/IF10, original [F01](design_review_surrealdb-lifecycle-target_2026-10-12.md#F01), IF05/IF07 enclosing outcomes and current acceptance obligations retain their disposition solely in [coordinator §6](../../plans/catalog-completion-plan_2026-10-14.md#6-sole-disposition-and-current-checkpoint).

| Principles | Verdict and reason |
|---|---|
| FP-01, FP-02; DP-17, DP-24 | Satisfied: coherent owners and explicit contracts localize completion, selection, diagnosis and host cleanup changes. |
| FP-03; DP-06, DP-08, DP-14, DP-16 | Satisfied: batch completion composes existing capabilities; schema tooling and inspection reuse their existing owners. |
| FP-04, FP-05; DP-01–DP-05, DP-07 | Satisfied: consequential domain and lifecycle distinctions govern actual operations rather than merely naming output fields. |
| FP-06; DP-18, DP-21, DP-23 | Satisfied: pure projection/accounting contracts can be examined independently; native behavior uses explicit owned fixtures. |
| FP-07; DP-09, DP-10, DP-19, DP-20 | Initial satisfied judgment amended for IF10: allocation lifetime and protected observation remain supported, but repeated full reconstruction across overlapping roots requires correction under FP-07/DP-09/DP-10/DP-20. |
| DP-13, DP-15, DP-22 | Satisfied: resolved-library boundaries and limitations are explicit; source inspection and historical runtime receipts are not promoted to measured or assembled acceptance. |
| CI-01–CI-06, CI-08, CI-11, CI-13 | Satisfied within the affected boundary: attribution, exact selected universe, evidence identity, uncertainty and pin lifetime remain preserved. |
| CI-07, CI-09, CI-10 | Preserved at the inspected consumers: no analysis relocation, heuristic promotion or new ambient extraction input. |
| DP-11, DP-12; CI-12 | No changed numerical/fixpoint/evaluation semantics. Typed recapture still has explicit cancellation/contention outcomes. Wider algorithms are outside this review. |

IF04 remains a concrete architectural refinement opportunity. Equal immutable replay can advance the generic guard revision and force fresh full readback. The current conservative route buys safety without introducing another mutation-fence protocol. This review accepts that tradeoff for bounded contention, rather than asserting that replay-driven amplification disappeared. A named workload requiring progress despite sustained replay must reopen the fence design.

## 8. Library fit and alternatives

The worker uses exact SurrealKit `1.0.0-beta.1` and patched SDK/RPC `3.3.0`; the macro is separately pinned to beta.1. The development dependency closure avoids unused embedded/JWKS execution. Actual remote authentication remains its own acceptance obligation. Generated schema is checked against native authority; installed definitions are parsed and compared while normalizing only application modifiers (`tools/lctx-surreal-dev/src/catalog.rs:14`).

The compiled build-info response records resolved versions, declared manifest/documentation patch-profile hashes and compiled schema correspondence. It explicitly leaves the server uninspected. The host binds it to physical worker bytes before effects and projects only allowed fields (`surrealdb_development.py:26`, `surrealdb_service.py:1053`). These are useful provenance facts, not a full source attestation for every transitive dependency.

The C01–C11 decisions were examined as alternatives serving concrete consumers, rather than accepted because they were recorded:

| Alternatives | Independent assessment |
|---|---|
| C01 references; C02 events/changefeeds/live history; C03 timers | Useful mechanisms for narrower relationships or proactive consumers. They do not replace exact incarnation, native certainty, host drainage or owner-directed release. Pinned reference insertion and event exhaustion source supports retaining the complete current route. |
| C04 GQL/inline/count/materialized views | Additional physical/query forms need a complete selected-universe and multiplicity contract. Existing indexed participant/reference access already serves the affected evidence paths. The pinned inline-cache coherence/cap-change conditions make unconditional substitution inappropriate. |
| C05 direct lexical/vector/fusion substitution | Current search composes native nomination with frozen selected-corpus statistics and family/channel witness ranking (`search.rs:291`, model `ranking.rs:435`). A generic scorer or ANN changes more than execution placement. Approximate retrieval remains a separate semantic choice. |
| C06 database API; C07 WASM modules | Neither removes application pin, evidence, cancellation and executable-lifecycle responsibilities automatically. Pinned API invocation disables internal permission checks after endpoint authorization. Moving computation requires a concrete complete operation whose composition becomes simpler. |
| C08 buckets | Separate byte and control operations add object admission, selected backup and disposal duties. Pinned bucket removal/export source does not supply the current originals contract. Native blob correction is the simpler complete route for current consumers. |
| C09 diagnostics/parser tooling | Appropriate positive adoption: actual native plans/index observations and library parsing answer current developer questions without a second catalog model. |
| C10 generic DB MCP/telemetry/cloud tooling | No demonstrated current consumer requires their deployment. Domain MCP and owned receipts retain narrower evidence and lifecycle contracts. |
| C11 alternative storage/backup | Files plus an index simplify immutable byte storage, but require a new selected graph/search, hold, publication and recovery composition. A second specialized service adds identity/readiness/disposal coordination. No inspected complete operation is materially simpler through those replacements now. |

Selected logical backup was inspected only as an adjacent preservation contract: it retains an exact publication pin and dedicated transaction-bound exporter, then explicitly cancels/invalidates the original owner before publishing staged output (`lctx-publisher/src/backup.rs:13–92`). This does not qualify its wider paused runtime campaign.

LN05’s decision to retain existing held completions plus operation-local observations is supported. A second persistent witness would add validity binding, invalidation, acquisition/release and cold-import obligations. No examined current consumer requires that additional lifetime after the compiler batch seam is fixed. Reopen when a named consumer repeatedly reconstructs the same namespace after its valid observation owners have ended and existing held completion cannot satisfy its contract.

Compared with the former per-view audit baseline, the candidate reduces repeated full observation without weakening root assurance. Compared with unchecked flags or producer-derived success, it preserves independent protection. Compared with a durable proof registry, global compilation lock or reduced concurrency, it introduces less machinery and preserves independent work.

## 9. Verification and uncertainty

| Claim | Evidence and outcome |
|---|---|
| Ownership, operation contracts, selected access and candidate structure | Interface-checked by this independent review, 2026-10-12. |
| New accounting, batch observation and inspection controls exist | Implemented; control source inspected. Execution **not_run** by this reviewer. |
| Actual generated Catalog/admission/publication/populated query journey | Pending with coordinator. No pass inferred from provider progress, tiny precursor body or selected-access component controls. |
| Original-budget IF05/IF07 enclosing acceptance | Pending with coordinator. Passing body and failed/Unknown enclosing fixture remain separate. |
| Actual beta token cancellation/restoration | Fixture and recovery source inspected; actual execution **not_run** by this reviewer. |
| Worker build-info and native compilation/readiness | Coordinator’s current validation owns results; this source judgment does not predict them. |
| Broad qualification, embedding/MCP execution, activation and performance | **not_run** here; excluded from this bounded acceptance claim. |

Read-only commands used included `git status --short`, `git rev-parse HEAD`, `git log`, `git diff`, `git hash-object`, `rg` and source reads. No test/build/service command was run.

No additional probe is needed to settle the architectural direction. Existing focused acceptance must establish that the implementation completes the intended real generated journey and its actual cleanup. A newly exposed source defect or contradictory native outcome requires targeted follow-up; a timeout alone does not identify its cause or establish a performance regression.

## 10. Rule impacts and disposition

**Rule impacts: none.** No `RCnn` item is introduced. This recommendation requires no change to a dependency policy, operator boundary, accepted ADR or other repository rule.

Current dispositions remain exclusively in [Catalog completion coordinator §6](../../plans/catalog-completion-plan_2026-10-14.md#6-sole-disposition-and-current-checkpoint). This review does not reconcile older paused plans or declare LC02/LC03 complete.

## 11. Architectural judgments and bounded decision

| Judgment | Verdict | Basis |
|---|---|---|
| A1 Localize change | Satisfied | Completion reuse, selected access, domain diagnostics and host cleanup have coherent owners; new family and mechanism changes follow those boundaries. |
| A2 Encode domain meaning explicitly | Satisfied | The model represents consequential distinctions, and executing operations enforce exact completion/admission, logical/physical, selected/foreign and outcome meanings. |
| A3 Extend through composition | Satisfied | One batch audit composes existing installation operations; diagnostic/provenance projection and beta integration reuse actual owner capabilities. |
| A4 Fit execution to the supported workload | Initial satisfied judgment superseded by §13; replacement source accepted in §14 | Shared contributor/blob observation did not remove repeated complete logical-root/dependency-view/alias reconstruction across overlapping outputs. Section 14 accepts the implemented scoped correction and its credible physical route; runtime acceptance remains pending. IF04's separate sustained-replay exclusion remains. |

**Initial bounded decision: Accept scoped at Interface-checked/source strength; §13 amended it to Revise for IF10. Current replacement source: Accept scoped under §14.** The initial no-new-material-finding conclusion missed the reconstruction amplification described in §13; source reinspection does not rewrite that omission or the failed runtime outcome.

**Enclosing architecture:** initial source acceptance was amended for the overlapping-output execution-fit scenario. Section 14 independently accepts the correction's frozen source; required runtime evidence remains with IF10's coordinator owner. Functional LC01 acceptance remains incomplete; the wider product, excluded execution paths, paused queues and measured performance are not certified.

The next consequential work belongs to the coordinator: finish current candidate validation, establish the actual generated Catalog/admission/unselected-publication/populated-native-query journey with truthful enclosing cleanup, and apply LC02/LC03 only after their prerequisites are satisfied. Runtime evidence may strengthen this review’s implementation maturity or expose a focused follow-up; it must not rewrite an earlier uncertain operation into success.

## 12. Bounded source follow-up: typed owner accounting and fixture resource contract

**Independent source follow-up · 2026-10-12 · Accept scoped, Interface-checked.** The following final corrections preserve this review’s architectural judgment. No new finding or rule impact is introduced. This follow-up examines the changed accounting owner and affected fixture contracts; it does not establish new runtime acceptance.

The examined working-file blobs are:

| Path | Git blob |
|---|---|
| `crates/lctx-native/src/installed_view/audit.rs` | `802f8c8a4a82e2d804874019d5438f8fd6f81a46` |
| `crates/lctx-compiler-store/src/compiler.rs` | `044c4ae7d3422a2202e7743cab5f761d520dcec0` |
| `crates/cpg-core/tests/facts_admission.rs` | `931f7c77398df4bc4eb09ac950a559d30b2f7c9f` |

### Typed logical-normalization owners

`installed_view/audit.rs::logical_roots_many` retains a map of physical owner keys to typed `CompletedContribution` clones. Its former sizing path serialized a descriptor solely to estimate that retained allocation and additionally admitted a large native-row sizing allowance. The changed operation promotes the existing `inspect_closure` heap helper without changing its definition, then reserves the map entry, inspected fixed-format physical key and typed descriptor heap before cloning the unique owner. An identical physical key still requires exact descriptor equality; a collision still refuses. The native `ExpectedRecord` keeps its own independently owned row reservation.

This correction matches the allocation being retained. The inspected `CompletedContribution` contains its contribution specification, fixed-size assurance/outcome and output map; the helper accounts for producer string capacity, dependency inputs, declared outputs, optional captured binding and output entries. The current assurance variant has no heap payload. Removing a sizing-only serialization therefore removes temporary work and overlapping admission without removing canonical identity validation, actual backing observation or native expected-row ownership. The unique-owner map remains charged for its lifetime, and reservation failure precedes insertion. FP-04/FP-05, FP-07, DP-03, DP-09 and DP-20 remain satisfied for this changed boundary; G3 and G6 remain pass at source strength.

### Full-compilation semantic fixture

`facts_admission.rs::availability_reuses_exact_admission_and_refuses_changed_invalid_membership` retains Catalog and Behavioral workspaces together on one shared pool. Its purpose is to establish exact facts-admission authority, profile separation, reuse and rejection of changed invalid membership. It is not a low-memory acceptance control.

Static inspection independently counted **141** declared Pyrefly outputs and **29** contributed vocabulary relations in `domain/producer_contract/facts.rs`, giving **170** singleton views per profile. Compiler completion constructs these singleton views even for empty output relations, and the installed-view cache retains their held completion owners. Each live held completion retains an `ExpectedRecord`; `control/held.rs::inspect_expected_read` retains a reservation of native row size multiplied by its allowance plus **65,536 bytes**. Both profile-specific completions remain live while their workspaces share the fixture pool. Their reservation floors alone therefore require:

`2 × 170 × 65,536 = 22,282,240 bytes > 16,777,216 bytes`.

This is a lower bound on the existing admission contract, not measured physical memory. It excludes other retained state and scratch. The old 16 MiB allowance cannot admit the fixture’s required simultaneously retained owners under that contract. Using this file’s existing `budget()` helper, which supplies 1 GiB to full-compilation fixtures, is consequently an acceptable fixture-contract correction. It changes neither production bounds nor the ExpectedRecord reservation policy, and preserves the independent smaller invalid-attempt case and semantic assertions. It does not establish that a complete two-profile compilation fits within 16 MiB, settle whether the conservative per-record allowance could later be improved, or turn the original refusal into a pass.

The compiler batch control’s private reader now explicitly receives its already declared `native_operation_budget` of 256 MiB. This binds the reader and operation to the intended existing resource owner instead of relying on an implicit scratch budget. It changes no production allowance or expected query result.

### Evidence and enclosing decision

The earlier 16 MiB facts refusal remains **failed** for its original candidate and contract. The coordinator’s reported 64 MiB receiver body pass remains distinct from its enclosing fixture cleanup/certainty. The coordinator also reports actual beta SIGTERM exit 143 with confirmed child cleanup, exact placeholder restoration and credential-consumer release while report failure and remote Unknown remain preserved; this source follow-up has not independently authenticated or rerun that receipt. Exact runtime reconciliation belongs in coordinator §6, separately from this review.

Source reads, diffs, `git hash-object` and a read-only count of the declared relation lists supplied this assessment. Builds, tests, probes and service operations are **not_run** by this reviewer. Current candidate functional acceptance and the complete generated Catalog/admission/unselected-publication/populated-native-query journey remain pending. This follow-up retained A1–A4 for its then-inspected boundary; §13 subsequently amends A4 for missed reconstruction amplification. The independent fidelity gate judgments and separate IF04 exclusion are unchanged by that execution-fit correction.

## 13. Review correction: overlapping-output reconstruction and IF10 target

**Independent amendment · 2026-10-12 local date / 2026-10-13 UTC · Revise scoped for IF10.** The initial review and accounting follow-up missed a material execution-fit seam. `logical_roots_many` independently rescans complete contributor memberships and backing records and rereads declared dependency views for each root, despite unchanged protected inputs. `audit_aliases` also reconstructs the same complete contributor-set aliases and reachability across overlapping outputs. The accepted audit-batch lifetime did not by itself remove this amplification. A4 and the affected FP-07/DP-09/DP-10/DP-20 judgments require correction; no new fidelity failure or architectural rule change is established. [Coordinator §6, IF10](../../plans/catalog-completion-plan_2026-10-14.md#6-sole-disposition-and-current-checkpoint) owns required implementation and current disposition, without a second finding ledger here.

The coordinator reports actual Catalog control `20261013T001826.068Z-60ae0b` timed out at its existing 360-second allowance after forward progress: Pyrefly completed, while documents completion repeatedly reconstructed logical roots and aliases across overlapping views. Its local trace records roughly 2.4–3.5 seconds for logical-root reconstruction and 0.4–0.5 seconds for alias auditing per observed view; reused membership checks and closure inspection continued. These observations expose repeated work, not a comparative speed claim or proof of the entire timeout's cause. The decisive final operations report `cancelled_before_issue`, retries zero, following harness cancellation. Pin-release failure is a separate finalization consequence; these records do not demonstrate deadlock or sustained transaction conflict. This reviewer inspected the source seam and the coordinator's attribution, but did not run or independently authenticate the campaign.

### Accepted correction target — Proposed / Interface-checked

Retain charged prepared logical-row fragments within the existing `InstallationAudit`, keyed by exact physical contributor, positive incarnation and full inspected row. Prepare families 0/1/3 once from independently observed contribution, membership and backing rows. First-observation validation, output EOF/digest checks, complete transport drainage and a final contributor premise comparison must succeed before publishing a reusable fragment. Reuse checks the live audit/session/resource pool and the current closure's exact contributor premise. The same pin protects observations through consuming checked effects; failure, release or uncertainty cannot create reusable authority.

Every root retains independent closure/mapping inspection and selects only its own fragments. Compose them through the existing six globally sorted families and full family hashers, retaining equal-row deduplication and conflicting-row refusal. Bindings, aliases, membership/realization completion, holds and final native effects remain independently earned. No whole-root shortcut, concatenated fragment digest or cross-operation witness is accepted. Existing `PreparedRows` supplies retained/spilled runs and budgeted cursors; cache keys, descriptors, runs and simultaneous assembly/cursor work remain charged for their actual lifetimes. Capturing fragments during the existing successful membership audit avoids an additional initial scan; lazy preparation once after that audit is a smaller acceptable implementation route.

Declared logical-view rows may be normalized directly from the exact independently validated typed `CompletedView` retained by `inspect_closure`. That inspection must continue to read actual native views, validate descriptors, dependency holds and complete mapping correspondence per root. Existing normalization consumes only the validated descriptor and canonical identity, so another native read in logical normalization supplies no additional required property. Reuse the existing normalization authority or a shared helper to preserve identical canonical bytes. A separate descriptor cache is unnecessary while fresh closure inspection remains.

A complete alias-result cache is acceptable for an exact deduplicated contributor set. Its key includes each physical ID, positive incarnation and full row, with complete equality rather than hash-only agreement. Initial alias/backing/endpoint/hold validation, all stream terminals, complete-set reachability and final contributor comparisons precede publication. Reuse validates every contributor against its protected observation and current root; the same live pin protects completed contributors and immutable outgoing content. The alias function derives seeds from all contributor memberships and aliases from their outgoing holds, so the complete set determines its result independently of the root relation. A successful subset cannot discharge a larger or smaller set's reachability, and per-contributor fragments alone cannot discharge whole-set reachability. Charge set keys and retained runs; a last-exact-set entry is a sufficient simple starting point where that is the actual repeated consumer.

This amendment accepts those bounded targets, not a replacement implementation. At publication of this amendment, no new frozen candidate has been independently reinspected. Revealing controls should compare all six roots with the uncached route for overlapping singleton/union closures; exclude unrelated cached contributors; reject changed incarnations/full rows; preserve conflicting-row refusal and empty output; leave no reusable fragment or alias entry after a late terminal failure; distinguish equal/reordered contributor sets from subsets/supersets; and preserve pin/budget cleanup. The actual generated Catalog/admission/unselected-publication/linked-query journey must then establish its own truthful enclosing outcome under unchanged concurrency, deadline and semantic assertions. F01/F03 and whole-track completion remain pending with the coordinator.

## 14. Independent IF10 replacement-source reinspection

**2026-10-12 local date / 2026-10-13 UTC · Accept scoped, Implemented / Interface-checked.** Independent source reinspection accepts the implemented operation-local contributor fragments, typed declared-view lowering and last-exact-contributor-set alias preparation for the overlapping-output scenario. No new material source finding or rule impact was identified in this bounded replacement. This supersedes §13's source-level Revise requirement for the identified seam; it does not close IF10/F03, F01 or LC01 runtime acceptance. Current dispositions remain exclusively in [coordinator §6](../../plans/catalog-completion-plan_2026-10-14.md#6-sole-disposition-and-current-checkpoint).

The coordinator's supplied SHA256 values were checked before inspection and again before publication; they match these examined Git blobs:

| Path | Git blob | SHA256 |
|---|---|---|
| `crates/lctx-native/src/installed_view/audit.rs` | `32e2655401e75addb364d89fa00a7c0304b04096` | `b1cc2cdb4bb9911bd8ab7cc89fc7a5241022877a94500150c85b5954cfa1e1ff` |
| `crates/lctx-native/src/installed_view/live.rs` | `30369e9754b5d7ab34c1e441dd990b1646f038e9` | `65f4140af5e2286fedaccde648532f1f86a70382f0d29e28cdf54259774b5d48` |
| `crates/lctx-compiler-store/src/compiler.rs` | `e7ce484fb4b7670b0a4cfac8464ccbdad05c63df` | `aa0fa4f6e03c34d8253eddb3bf2c6115be61d67205d19938f96097441f201549` |

### Validity and exact composition

`InstallationAudit` retains prepared contributor runs for families 0/1/3 and one complete alias preparation (`audit.rs:28–108`). Fragment publication requires the existing complete protected membership observation, exact incarnation/full-row agreement, successful independent contributor/membership/backing preparation, complete membership transport drainage and another native contributor comparison (`audit.rs:2040–2105`, `audit.rs:2168–2196`). No provisional fragment enters the map after a late stream failure. The contributor cache's original full row is never replaced on a changed premise; a mismatch refuses. Keys are exact physical records, and continued live pin/session/resource ownership is required.

The production caller first performs fresh per-root `inspect_closure` and `audit_memberships_scoped`, then invokes the scoped alias and logical-root routes (`live.rs:140–154`). Consequently each reuse compares the retained premise with a freshly inspected root's `ExpectedRecord`; the new preparation does not replace physical mappings, dependency holds or descriptor inspection. Each root iterates only its own closure mappings. Prepared rows are fed to that root's six `SortedRows` families and full `CompletedFamilyHasher` sequence; equal-key deduplication and conflicting-row refusal remain global within the root. There is no fragment-digest concatenation, aggregate-root reuse or admission shortcut.

The alias entry's ordered physical/incarnation vector is backed by the audit's separately retained immutable full rows; every selected row must still equal the current closure's inspected row (`audit.rs:1728–1768`). Thus omitting another copied full row from the vector does not weaken the full-row premise. Exact complete vector equality allows reuse; a subset, superset or different incarnation cannot match. On a miss, the original full alias/backing/endpoint/hold checks, both stream drains and complete-set reachability run before a final contributor comparison and cache publication. Only the last successful set is retained. Reachability is neither inferred per contributor nor reused across a changed set.

Scoped declared-view lowering uses the exact typed view already independently read, physically validated and compared during `inspect_closure` (`audit.rs:2238–2255`). It constructs descriptor input for the existing logical normalizer; canonical descriptor serialization and identity remain owned by `completed_logical_rows::normalize`. The root's native view inspection, dependency-edge checks and exact mapping correspondence remain. Cold recovery, aggregate completed-state earning and other unscoped consumers continue through the original native-read route; the new preparation is confined to the live protected installation scope.

### Ownership, execution fit and remaining evidence

The implementation takes the accepted lazy-prepare-once route: it retains one initial extra physical reconstruction per unique contributor, then avoids repeating those native membership/backing scans and normalizations for each overlapping root. Each root still incurs complete local merge/sort/hash work and its necessary closure, member and final-effect checks. This is a credible narrower physical route for multi-output compilation; it is not a measured speedup or a guarantee that the complete Catalog control fits its deadline. A later ordered merge or preparation during the first membership audit remains an optional improvement if an actual consumer exposes that cost.

Prepared runs reuse the existing budgeted/spilled `PreparedRows` mechanism, whose last run/cursor owns scratch and retained reservations. The audit charges fragment-map entries and physical keys; the alias entry owns its charged exact-set vector. Root assembly and decoded cursors retain their existing independent charges. Replacing the one alias entry drops obsolete key ownership while any live row cursor retains its own run. No new private pool, global cache, durable witness, thread limit or deadline increase is introduced. Compiler batch finalization still releases the shared pin through `Completion`, preserves the primary/finalization outcomes and retains Unknown when required (`compiler.rs:1207–1263`); per-view holds and checked installation effects remain unchanged.

**Final metadata-bound reinspection:** the final audit blob above replaces the first IF10 candidate's `df8a8fbfb0583a4a204c65fe42fdcefac5601bc9` / SHA256 `246fd15f582f45a2d6c8601368053956749763ba3eefc22a45121c5f138f6e04`. Before each logical-fragment insertion it now admits an entire worst-case B-tree internal node: eleven inline key/value slots, twelve pointer slots, 256 bytes of header/alignment allowance, and that entry's separately allocated physical `RecordId` key (`audit.rs:2183–2194`). Independent inspection of the pinned `nightly-2026-09-29` alloc source's `collections/btree/node.rs` confirms `B = 6`, capacity `2 * B - 1` and internal edge capacity `2 * B`. One full node allowance per retained entry conservatively covers sparse nodes and insertion/split allocations; the empty map allocates no retained node, and each nonempty retained node has a key. Admission precedes insertion and remains in `audit.charge` until audit drop. Prepared run payloads and decoded cursors retain their independent reservations. This corrects sparse-node metadata under-admission without changing cache validity, row meaning, hash composition or the scoped source acceptance. Live/compiler hashes are unchanged; no runtime result is inferred.

A1–A3 remain satisfied for the affected boundary. A4 and FP-07/DP-09/DP-10/DP-20 are satisfied at scoped source strength for this replacement: preparation follows protected immutable input lifetime, root-specific meaning remains independently composed, and combined retained state uses existing resource ownership. G1–G3 and G5–G6 pass at source strength on the cache validity, exact composition and terminality evidence above. G4/G7/G8 and applicable CI-G1/CI-G2 remain preserved; CI-G3 remains not applicable. IF04's separate sustained-replay exclusion is unchanged. No enclosing numerical, library, operator or whole-product qualification is implied.

Source inspection covered both scoped helpers and their production caller, compiler batch ownership/finalization, the existing normalizer and ordered-run implementation, adjacent cold consumers and the changed revealing control definitions. The late-terminal control now asserts no fragment publication, cold/scoped root equality and one contributor/alias observation; the actual compiler batch control asserts unique contributor preparation and one exact alias-set observation across two roots. Those are inspected test definitions, not executed evidence. Builds, tests, probes and service operations are **not_run** by this reviewer. Current compilation and affected controls remain pending with the coordinator, followed by actual generated Catalog, independent admission, unselected publication and linked native queries with truthful cleanup. The failed `60ae0b` receipt and its original cancellation/finalization outcomes remain unchanged.
