# Reconciled efficiency alignment — implementation plan

**Authorized implementation track / execution in progress, 2026-10-11.** The operator approved creation of this new plan series
and all 18 reconciled rule dispositions below. The subsequently authorized design-decision
investigation resolves the mechanisms and migrations in §6, using static evidence and bounded
probes. It does not implement the product changes. The execution checkpoint in §4 owns implementation progress;
the investigation's controls are distinct from product acceptance. Functional completion is the track
exit. Performance measurements and real-library campaigns remain explicit external follow-ups.

This document owns the combined target, shared decisions, dependencies, current progress and source
finding disposition. The five companions develop local contracts and acceptance; they do not maintain
competing status tables. Enduring architecture remains in the existing model/design owners and ADRs.

| Read for | Plan |
|---|---|
| Physical effects, completion, admission, derivations and release | [Native persistence and lifecycle](efficiency-alignment-native-plan_2026-10-11.md) |
| Capture, provider reuse, selected reads, value/provenance and cache | [Compiler, capture and reuse](efficiency-alignment-compiler-plan_2026-10-11.md) |
| Condition/graph preparation, Summary, analytics, synthesis and embedding batches | [Model, analysis and synthesis](efficiency-alignment-analysis-plan_2026-10-11.md) |
| Exact ranking, preparation, pages, sorting, transport and cancellation | [Serving, selection and transport](efficiency-alignment-serving-plan_2026-10-11.md) |
| Provider controls, dependencies/profiles, fixtures, reruns, evidence and status | [Development loop and evidence tooling](efficiency-alignment-development-plan_2026-10-11.md) |

## 1. Scope, basis and current state

The basis is the [original codebase review](../design_review/reviews/design_review_efficiency-alignment-codebase_2026-10-11.md)
and the [independent comparative audit](../design_review/reviews/design_review_efficiency-alignment-audit_2026-10-11.md),
both Revise. Original IDs are written **O-F01–O-F50**, audit IDs **A-F01–A-F20**; the prefixes only
identify the source and do not renumber findings. Audit F14–F19 are the six additional findings.
RC01–RC18 retain the original rule-impact IDs, with the audit's corrections and operator decisions.
Original review/evidence and audit/evidence remain unchanged and are current consumers of this plan.

The review read `88f33bc0` and was published at `2b05c6c2`. The audit read `2b05c6c2` with a captured
concurrent overlay and was committed in `a2fe359b9e10560e4502c59fce9933a822c1bebf`. Plan authoring began
at that commit and followed relevant concurrent source changes through
`d220a213166766bc47d446626c43b209e3b7f907`. The source/evidence routes are the
[original evidence index](../design_review/evidence/2026-10-11_efficiency-alignment-codebase/README.md)
and [audit evidence index](../design_review/evidence/2026-10-11_efficiency-alignment-audit/README.md).
Their inquiry briefs, operation map, V01–V15/R1–R26 and probes remain attributable historical evidence,
not fresh measurements or verified closure of this track.

Current source contains stronger current-context eligibility for serving/cold comparison/selected
backup, first-binding and support/anchor checks, charged root batching, recovery traversal improvements
and phase controls. Use these foundations. Their separate session's focused qualification is neither
repeated nor promoted into this track's acceptance. Refresh only source directly affected by a package
when execution starts; preserve concurrent work and record any changed finding premise here.

**Separate track:** at the operator's request, no existing plan document was reviewed. No existing
plan is superseded, retired or credited with closure here. This series owns its own functional scope,
including ordinary-profile qualification; it does not depend on old BC3 or other old-plan completion.
After this track is fully implemented, prepare the resumption handoff in X02. The operator will then
resume existing plans with any necessary updates. Existing plan state and separately paused service
qualification remain unchanged during authoring.

Scope is product plus development loop, assessed against FP-07/A4 and the efficient-architecture
heuristics, with the applicable code-intelligence fidelity constraints. The reconciled architectural
judgment retains the confirmed A1/A4 mismatches and the strengths in A3 composition; A2 needs the named
contract decisions. Narrow validity/provenance and reuse issues keep their own corrective contracts.
Efficiency defects do not automatically become semantic gate failures, and sampled adequate behavior
does not certify the whole product. Docs publication tooling is excluded. Protected gold/heldout data, operator stores/client registrations, service maintenance,
activation and real-library compiles are not authorized by plan creation. No builds, tests, services,
fixtures, database queries, repo-level quality checks or measurements ran in authoring, as requested.
Static source/document assessment and focused independent design advice supplied the plan.

**Decision investigation, 2026-10-11:** pinned main and an owned probe checkout at `439f3d2d`.
The [decision evidence](../design_review/evidence/2026-10-11_efficiency-design-decisions/README.md)
records exact source, independently authored finite controls, actual native-provider observations,
Cargo/profile/Hakari controls and a fresh dedicated RocksDB/gRPC campaign. The shared service and
its paused qualification were untouched; the fixed-name systemd installer and ordinary fixture
owner were not invoked. Only decision documents/evidence integrate from the probe checkout.
The observed gRPC transaction-drop failure strengthens PD02 from an inquiry lead into an
evidenced owner defect; it does not authorize operational remediation of the shared installation.
Repo-level quality checks and real-library/performance campaigns remain not_run by instruction.

## 2. Combined target and preservation constraints

Make ordinary work follow the requested semantic unit and the lifetime of what it consumes. Native
windows combine useful effects; a completed immutable view carries enough local protection to support
cheap attachment and exact access. Provider reuse starts from a complete captured request. Value-only
reuse stops unnecessary propagation without rewriting exact evidence. Preparation lives with an
invocation, reader pin, executable or service generation; physical storage and development dependencies
follow their actual owners. The target removes identified repeated work; no numerical speedup is claimed.

| Authority | Owns and supplies | Does not transfer to consumers |
|---|---|---|
| Executable model | Relation meaning, identities, selectors, coverage, outcomes, invariants and provenance | No physical layout imposed merely by typed record organization |
| Capture/provider contracts | Complete result-affecting input description, output inventory and eligible outcomes | Native lifetime does not establish semantic equivalence |
| Native persistence | Exact incarnations, complete installation, held content, effects and recovery/release | A digest alone is not backing integrity or admission |
| Admission/publisher | Checked semantic premises and coherent current derivation realization | A manifest/marker alone does not attest to a different realization |
| Analyses/synthesis | Invocation preparation, exact methods, proof alternatives and complete decisions | Bounded output does not justify truncated semantic input |
| Serving | Exact policy, contextual eligibility, compact pinned preparation, page hydration | A warm positive cache entry does not attest to new absence/context facts |
| Transport/runtime | Admitted demand and actual worker/effect lifetime through drain or explicit recovery | Coroutine/child exit is not remote terminal proof |
| Development owners | Dependency/profile selection, protected observations, exact receipts and object lifecycle | Later mutable paths do not rewrite earlier evidence |

Enduring contract routes are the [semantic model](../design/sections/semantic-model.md),
[facts and identity](../design/sections/facts-and-identity.md),
[acquisition and extraction](../design/sections/acquisition-and-extraction.md),
[storage and publication](../design/sections/storage-and-publication.md),
[analytics](../design/sections/analytics.md), [behavioral analysis](../design/sections/behavioral-analysis.md),
[synthesis and serving](../design/sections/synthesis-and-serving.md),
[product target](../design/sections/api-and-evidence-product.md) and
[validation/evaluation](../design/sections/validation-and-evaluation.md). Development contracts retain
[README test semantics](../../README.md#tests), [native service ownership](../surrealdb.md) and
[workspace storage ownership](../workspace-storage.md). Follow those owners to their accepted decisions;
reading an owner does not require reviewing the pre-existing plans it links during this task.

Three identities must remain distinct: **value identity** permits explicitly declared substitution;
**evidence identity** says exactly which sources and invocation justify a claim; **physical identity**
says which protected installed incarnation exists. Current derivation identity adds which implementation
realizes retained logical content. A reusable artifact can preserve values while requiring new evidence
or fresh destination admission. None of these distinctions requires a new service or register.

Preserve bounded effects, full-value collision checks, exact request reconciliation, committed outcomes
through fallible cleanup, logical shared-store isolation, selected coherent recovery and independent
cold admission. Preserve exact frozen ranking, explicit Partial/NotRequested/Refused/unknown states,
negative/context witnesses, deterministic ties and exact final-byte admission. Preserve charged live
state, ownership through cancellation/drain, bounded retirement, protected/cited/warm content and normal
available concurrency. These are implementation constraints, not optional verification examples.

Keep demonstrated library strengths: independent provider adapters, deterministic SCC scheduling,
Summary semantic-progress/Pareto separation, bounded BDD operations, justified exact ranking/FCA kernels,
independent public-byte evaluation, native-thread drain and owner-directed storage. Do not replace them
with a framework merely because a general library offers an operation with a similar name. Exact pins
and the selected capability skills remain the library contract route at implementation time.

### Physical choices and alternatives

Use fused windows and bounded paging before assuming a new fencing protocol is necessary for every
improvement. N03 permits finite guards/stripes where read-only ordinary transactions cannot protect
all predicates. Use the existing exact view-member address instead of owner-by-key Cartesian lookup.
Use current lower provider interfaces instead of a new pipeline. Keep selected-only exact vector
scoring as the default; cohort-first scoring has a real sparse-domain losing case. Repair the current
sorter first, with a bounded composed-library comparison before wholesale replacement. Group immutable
provider test observations before introducing persistent fixture infrastructure.

The costs of these choices are explicit: durable completion must cover every identity-bearing family;
by-reference attachment migrates enumeration/recovery consumers; value cutoff needs complete dependency
and receipt contracts; compact NotRequested migrates scoped queries and assessments; shared blobs and
current derivations require real cutover/release logic. Those migrations belong beside their producer,
not in a final inventory-only phase. Removing duplicate checks is safe only where the actual premise
remains protected; cold/foreign ingress still introduces uncertainty.

A new fact family should add its declaration/provider and consumer union without re-forming unrelated
unions. A provider upgrade should invalidate its responsible outputs without pulling analyzer code into
serving builds. A patch release should reuse unchanged values but preserve changed origin/role/span
meaning. A new retained derivation must not change a live reader's realization. A cancelled operation
must leave an owned terminal or recoverable outcome. These scenarios explain the boundaries and recur
in package acceptance without expanding the product target.

## 3. Operator-confirmed rule dispositions

**All rows explicitly approved by the operator, 2026-10-11.** “Change” approves the direction subject to
the stated safety and migration conditions; it does not establish implementation. “Retain” keeps the
standing rule. The linked source impact provides its original quotation/location. G00 publishes the
relevant owning changes before dependent behavior changes, using the ADR skill where a decision changes.
Accepted ADRs remain immutable: supersede where necessary, amend the owning design in the same slice,
and update actual binding/agent/tooling consumers. Do not invent one ADR per table row.

| Impact | Approved disposition | Route before dependent implementation |
|---|---|---|
| [RC01](../design_review/reviews/design_review_efficiency-alignment-codebase_2026-10-11.md#rc01) | **Change:** narrow ordinary fencing only after complete predicate/recovery accounting; finite guards/stripes allowed | N03; control/storage owner and superseding decision for affected ADR-0145 coordination rules |
| [RC02](../design_review/reviews/design_review_efficiency-alignment-codebase_2026-10-11.md#rc02) | **Change:** captured-input provider reuse with complete context keys and explicit outcome eligibility | C02/C03; acquisition/extraction and product reuse owner; supersede pure-stage restriction where held in a decision |
| [RC03](../design_review/reviews/design_review_efficiency-alignment-codebase_2026-10-11.md#rc03) | **Change:** admission-for-reuse of eligible terminal previously-unadmitted results under recovery ownership | N09/C03; recovery/admission owner and affected ADR-0145 decision |
| [RC04](../design_review/reviews/design_review_efficiency-alignment-codebase_2026-10-11.md#rc04) | **Change:** separate reusable value identity and exact provenance; preserve/regenerate receipt-bearing outputs | C05; model source/completed-view and product reuse contracts, decision and validators |
| [RC05](../design_review/reviews/design_review_efficiency-alignment-codebase_2026-10-11.md#rc05) | **Change:** omit contribution profile only with complete dependency/output neutrality | C02/C05; product §14.10/profile, provider and model contracts |
| [RC06](../design_review/reviews/design_review_efficiency-alignment-codebase_2026-10-11.md#rc06) | **Change:** remove duplicate same-process reconstruction/carry protected validity; retain meaningful stored/cold checks | N07/S03; publication/storage/serving owners and affected ADR-0145 assurance decision |
| [RC07](../design_review/reviews/design_review_efficiency-alignment-codebase_2026-10-11.md#rc07) | **Change:** versioned compositional completed-state identity over the complete validated closure | N06; executable state format, storage/model consumers and migration decision |
| [RC08](../design_review/reviews/design_review_efficiency-alignment-codebase_2026-10-11.md#rc08) | **Change:** compact uniform NotRequested with frontier/assessment/scoped-query migration | C06; semantic coverage/frontier and product consumers, model migration decision |
| [RC09](../design_review/reviews/design_review_efficiency-alignment-codebase_2026-10-11.md#rc09) | **Change:** current derivation identity/eligibility for retained manifests with safe generation transitions | N08; publication handle, storage/serving owners, decision and actual format transition |
| [RC10](../design_review/reviews/design_review_efficiency-alignment-codebase_2026-10-11.md#rc10) | **Change:** exact frozen serving; migrate ANN numerical consumers to independent exact/full-value controls before retirement | S01; serving §11.2/product §14.8/runbook, supersede affected ADR-0131 ranking text, numerical controls and DDL |
| [RC11](../design_review/reviews/design_review_efficiency-alignment-codebase_2026-10-11.md#rc11) | **Change:** eligibility precedes admission/caps/tier counts; reorder scoring only with equal result and failure semantics | S05; product/serving ranking owner and its decision/controls |
| [RC12](../design_review/reviews/design_review_efficiency-alignment-codebase_2026-10-11.md#rc12) | **Retain** current concurrency/resource rules; causal evidence required for a later policy change | No caps/weights/default change in this track; M01 follow-up, AGENTS/ADR-0137 remain authoritative |
| [RC13](../design_review/reviews/design_review_efficiency-alignment-codebase_2026-10-11.md#rc13) | **Retain** shared store and logical isolation; no per-attempt databases | Native/fixture owners preserve current rule; no new isolation service |
| [RC14](../design_review/reviews/design_review_efficiency-alignment-codebase_2026-10-11.md#rc14) | **Change:** this track qualifies one shared ordinary dev/test profile; release acceptance remains distinct | D03; Cargo/verify/rerun/README/AGENTS and superseding ADR-0137 profile decision; no old BC3 dependency |
| [RC15](../design_review/reviews/design_review_efficiency-alignment-codebase_2026-10-11.md#rc15) | **Retain** completion-time executable observation; protected-generation reuse only, no later mutable-path substitution | D05; preserve evidence/README meaning, document actual protection at its owner |
| [RC16](../design_review/reviews/design_review_efficiency-alignment-codebase_2026-10-11.md#rc16) | **Change:** content-keyed originals with artifact references and full recovery/retirement integration | N10; executable storage/schema, source retrieval, recovery and migration decision |
| [RC17](../design_review/reviews/design_review_efficiency-alignment-codebase_2026-10-11.md#rc17) | **Change:** durable admission premises need validator identity and protected physical integrity/incarnation; missing/stale rechecks | N11; admission/storage owner and trust decision; no portable trust authority |
| [RC18](../design_review/reviews/design_review_efficiency-alignment-codebase_2026-10-11.md#rc18) | **Retain** portable cache enabled by default; repair physical costs first | C08; runtime/reuse policy retained; original F20 “RC19” typo means this RC18 |

No memory or standing skill edit is required. If implementation uncovers a genuinely new rule change,
present its concrete consequence to the operator; these approvals need not be requested again.

## 4. Package sequence and sole execution checkpoint

**Operator cutover direction, 2026-10-12:** this is an early design-phase system. Migrate views
into the current realization model where practical; otherwise record their regeneration from
pinned inputs. Retain no legacy runtime code or data after cutover. Opaque historical stores,
compatibility readers and frozen old-revision examples are not delivery requirements. Assess
the current contracts from first principles with minimal revealing functional cases; historical
campaigns and benchmarks are not prerequisites for future assessments. This direction replaces
the opaque-preservation approach and any historical-fixture prerequisite described in earlier
partial receipts below. Unknown live effects still require owned drain before removal. The shared
paused installation and operator state remain outside this campaign's effects.

The original-revision upper fixture `20261012T071909.321Z-e5da48` was cancelled through
`just runs cancel` at07:32:57UTC, exit143; process cleanup confirmed with no survivors. It
produced no accepted upper receipt. Native cleanup is unresolved (launcher hold
`launcher:5deaf905af854d50a2ddc9af521f17cc`); exact campaign recovery is required before
another native operation. Its interrupted work establishes no migration or semantic acceptance.
Current compact-owner model controls passed14 (`20261012T072908.486Z-64775d`, nextest
`e6f70d23-dfbd-4fc0-a75c-fac9889c85da`, exit0/cleanup confirmed07:31:06UTC):
`cargo nextest run --locked --offline --release -p lctx-model --lib -E
'test(compact_analysis_) or test(compact_structural_) or test(compact_synthesis_) or
test(compact_catalog_parent_checks_) or test(catalog_evidence_accepts_protected_compact_) or
test(catalog_selection_rechecks_compact_) or test(compact_library_facts_coverage_)'`.
Actual current producer/native/publication/serving acceptance remains open.

**Execution started, 2026-10-11, baseline `ebcf423cc16e6cacea6455500a0cdfc9795d08ac`.**
The operator approved the detailed execution sequence, one dedicated campaign installation and
all affected functional boundaries, enabled optional paths and representative integrated journeys.
Repository quality campaigns and complete ordinary-suite execution remain excluded. The shared
service and paused qualification retain their own state. The existing plans and source reviews are
unchanged. Package/finding closure still requires its complete corrective obligations.

| Implementation slice | Current evidence and remaining boundary |
|---|---|
| Current capacity / evidence boundary | **In progress, 2026-10-11 (receipts through 2026-10-12 UTC):** operator freed disk space, including this checkout's Cargo outputs. Capacity no longer blocks compilation. Cold compilation exposed real type/API integration errors, repaired within the affected owners; failed receipts remain failures. `cargo check --locked --offline --release -p lctx-native --tests` passed (`20261012T030041.117Z-ab7ec0`, confirmed cleanup), before the subsequent maintenance-completion/product-release changes. Compiler-store/core/publisher libraries compiled (`20261012T025759.583Z-043a5b`), but that command's test-target phase failed on a publisher exhaustive-match omission, since repaired. `cargo check --locked --offline --release -p lctx-native -p lctx-compiler-store -p cpg-core -p lctx-publisher --tests` passed (`20261012T033936.839Z-fd65be`, exit0/cleanup confirmed), before the later migration component edits. No whole-suite, qualify, lint, formatting or documentation quality campaign runs in this track. No source finding is closed by type checking. |
| N03/N04/N05/N06/N08/N10/N11 and C05/C06 source integration | **In progress / partial compile evidence, not activated, 2026-10-12:** target bounded stripes/exact predicate tombstones, original blobs/pending-original ownership, exact held view/completed-state foundations, answer-affecting native/publisher source identities, derived namespace consumers and staging-root publication are being integrated. Source5 remains untouched at format5; target format6 is not installed. Native physical completion earning, selected physical mappings, durable semantic-admission premises, authenticated full typed5→6 conversion and all migration/retirement/namespace/shortcut controls remain open. C05 now separates actual configuration/value payloads from fresh receipts and preserves ordered input roles; C06 now emits Assembly compact Flow NotRequested per exact captured input, including explicit empty domains; compiler expected/frontier/publication validation and serving resolve actual selected completed owners. This is source integration awaiting focused production/consumer controls. Whole-output NotRequested reuse remains disabled; analysis-owner per-kind compaction is still open. No source finding is closed by this unverified source work. |
| Exact publication/selection and migration seams | **Source-only / not_run, 2026-10-11:** ADR-0156's staging implementation reconciles the full direct derived-child inventory before a root-predicate CAS and retains root/completion/admission incarnations in the marker; committed reporting precedes fallible finalizers. An intermediate independent source check found no concrete defect in those bounded hunks; it is not functional acceptance. Exact-reader/selection foundations now use completed realization namespaces and selected contributor aliases. Selected-backup consumers now use per-view physical mappings and source-only snapshot-checked published capability/transaction factories, preserving distinct equivalent selected physical copies; native acceptance and the complete artifact admission gate remain pending. Restore derived comparison uses an exact completed reader and explicit composite closure. A fixed model-validator driver computes ordered typed fingerprints and requires explicit EOF; source inspection caught and corrected storage canonicalization sorting before order validation. Its four controls are written, not run. Maintenance-only source5 original translation validates exact old envelopes, continuity and complete byte EOF before producing a correspondence; target native readback/admission and aggregate conversion are still required. Historical source5 producer assurance and its original identity algorithms are now integrated source; original facts/normalization declarations are registered. Authentic upper-stage factory metadata and selected-record controls now pass; the dependency-ordered typed conversion and full native migration remain open. No deserialized witness, matching hash, historical provider binding or partial stream is accepted as current native/semantic authority. |
| N09 complete binding meaning and migration source review | **Source-only / not_run, 2026-10-11:** ADR-0157 separates the mutable attempt slot from immutable full source/boundary/configuration/view meaning. Publication binding inventory is now v2; strict native canonical descriptor codecs/schema and selected backup's exact descriptor lookup plus existing data-only portable projection are integrated. Native completion must still install and retain completion→descriptor→view ownership and independently check it in published snapshots; actual guarded compaction remains open. The source5→6 foundation requires all eight components in Preflight/Declarations/Transform/Verify order and a real host maintenance lease, with an immutable original journal and atomic source-fence/progress updates. Root source review found an acquisition path that could suppress a failed recheck by observing unchanged old progress; the corrected draft preserves that primary error and reconciles only an exact newly committed bootstrap/transfer with independent source/journal readback. The complete runnable registry still requires typed DAG/binding conversion, aggregate activation and CLI wiring; completed admission, publications and retained obligations now have source implementations awaiting integrated acceptance. The old4→5 runtime contract now explicitly refuses in a format6 executable, preventing accidental4→6 activation. Original format5 binding identity is retained only in maintenance historical authentication; conversion must independently recompute targetv2. Finite source5 declarations recovered from original source include six providers and nine normalization stages in both profiles; catalog input/output role inventories were compared to the retained artifact. Focused metadata controls and partial compile evidence establish no native migration acceptance or finding closure. |
| Physical/semantic capability factory integration | **Implemented source / partial focused evidence, 2026-10-11:** native physical completion independently audits exact dependency-view mappings, backing/alias/original closure, member EOF and all six logical families. Retained singleton product namespaces stay in the physical inventory without changing portable artifact meaning. Compiler storage privately assembles ArtifactAdmission only after actual facts/producer/reference checks, executed whole/scoped invariant kernels, actual frontier kernels, canonical graph EOF and manifest verification. Fresh admission bypasses hash-only semantic memoization. Persisted version2 evidence records actual ordered selected inputs, complete definitions/factories and exact bindings; native later re-holds it against the current model, epoch, incarnation and full inventory. Mechanical native CAS plans do not manufacture the compiler semantic capability; this is the existing trusted-writer boundary. Ordinary/restored production paths now call this factory; the legacy marker-only `mark_attempt_admitted` continues to refuse. Full completion addresses, portable contribution/membership normalization and restore owner maps distinguish different outcomes of one spec. Model controls initially passed35/failed3 (`20261012T025314.888Z-8d534e`); adding the omitted canonical UniformNotRequested relation repaired the actual model inventory. The seven affected follow-up controls passed (`20261012T030205.627Z-0381c1`, cleanup confirmed). Artifact/native admission acceptance, historical-assurance migration and durable reuse corruption matrices remain not_run; fresh fine-grained premise reuse is not claimed. |
| C04 demand and N09 cleanup nomination integration | **Implemented source / partial focused evidence, 2026-10-11:** table creation eagerly earns its exact held view before optimizer false/NULL/zero elimination. Actual coerced equality/positive IN under mandatory AND supplies charged finite demand; unsupported representations/disjunction/residuals retain DataFusion evaluation and limit ownership. Source-owned held candidates use the source's exact per-view mapping closure; receiver preflight covers all output/dependency completions before one atomic incoming-reference transaction. Equivalent dependency copies stay independently selected per view. Twelve selected compiler controls now pass: eleven in `20261012T030053.095Z-c2c6a1`; the metadata control initially failed because its test omitted Tokio, then passed with the runtime (`20261012T031509.404Z-ca0b92`), cleanup confirmed. Exact spec mismatch under an equal request is corruption; preserved source5 assurance remains ineligible for current captured reuse. Terminal cleanup nominates exact outgoing roots before releasing the attempt's holds. Independent source review found the old unadmitted branch directly deleted warm products; this branch is removed. Product roots remain warm regardless of artifact admission until explicit release atomically registers an exact-incarnation retirement job/item, then marks released; collection and other holds remain separate. Cleanup refuses product owners; collector retains unreleased products and history protects referenced release jobs. Genuine format6 attachment, release/reclaim, terminal-unadmitted recovery admission and binding compaction remain not_run. |
| C06 compact native domain and typed reconstruction controls | **Source integrated / focused model-Tested, 2026-10-12:** Assembly emits exact captured Catalog Flow declarations, including zero-member domains; requested Behavioral Flow cannot declare NotRequested. Compiler availability, expected/frontier publication checks and published serving resolve selected completed semantic owners; serialized declarations create no native grant. Owner output counts account for every unique captured input/context domain, including empty domains. `cargo nextest run --locked --offline --release -p lctx-model --lib --test uniform_outcomes --test record_construction --test analysis_expected -E 'binary(uniform_outcomes) or binary(record_construction) or test(compact_flow_frontier_requires_selected_owner_and_exact_captured_domain) or test(compact_flow_survives_frontier_finish_without_fabricated_scoped_coverage)'` passed8 (`20261012T050926.818Z-d87822`, nextest `48951dae-cf05-4148-ad7e-c41e1599beca`, exit0/cleanup confirmed): four semantic/scoped/serving-domain controls, two typed construction controls, frontier enumeration and independent FrontierCheck.finish replay. Missing owner/descriptor, mixed scoped/compact authority, wrong domain/count/request and budget ownership remain explicit. Earlier `050423` failed before bodies on three stale missing-import diagnostics, cleanup confirmed. Actual Assembly provider and integrated native/compiler/published serving controls remain pending. Analysis-owner per-kind compaction and whole-output NotRequested reuse remain open; no finding closure. |
| Buffered query ownership and shape | **Focused-Tested, 2026-10-11 (2026-10-12 UTC receipts):** exact SDK3.3.0 raw query batches retain empty SINGLE versus empty rows without a wire change; the execution future and receiver are joined through physical EOF and terminal/result_count checks. Native finite queries preserve NONE/NULL/object/scalar/array/nested/ID/bytes, validate every statement and late error, and retain the original client/scope/lease through cancellation or private-scope Drop. Visible payload/container capacities, accumulation and transient relocation are charged through conversion; abandoned delivery drains without accumulating discarded output. Opaque transport queues/allocator/RSS/shared backing and arbitrary decoder T remain outside this accounting claim. SDK isolated source harness passed4 (`20261012T025058.961Z-c9a24b`). Native accepted-query controls plus selected audit controls passed10 (`20261012T030306.764Z-568d8e`, cleanup confirmed). Initial dedicated wire builds failed before query execution (`031525`, `032217`). Run `20261012T032812.673Z-2daca0` then failed because a test SLEEP required root permissions; its attachment `1bee9b72-0438-4481-a30c-7506bfe33833` remains protected. Removing that timing dependency and repairing confirmed-logical-error close cleanup produced **passed9** in `20261012T033523.004Z-40c84c`: eight pure accepted-query controls and one actual campaign33385 wire control, exit0/cleanup confirmed. Attachment `e41e9249-4db2-4510-8b35-e1d07394b7c8` records native finality confirmed and released=true. This tests shape/parity, cancellation/drop, preparation/late errors, physical EOF and original session identity; it does not qualify migration or the whole track. |
| Preserved integration drafts | **Source integration in progress, 2026-10-12:** [execution evidence](../design_review/evidence/2026-10-11_efficiency-execution-integration/README.md) retains original packet checksums and historical anchors. Strict original metadata, live inherited host authority, originals/blob conversion, executable staging and immutable correspondences are source-ready. Authentic original factory exporters passed2 (`20261012T034619.656Z-223380`, exit0/cleanup confirmed): 126 declarations cover all21 upper recipes across actual profiles/configurations, plus all888 original typed record fields. These establish metadata, not historical completion qualification. CompletedAdmission now invokes actual current physical EOF plus the same private whole-artifact gates; warm targets remain unadmitted. Retained obligations preserve actual original guards/claims, protect replacement products, then journal exact-incarnation retirement nominations before old hold release; publication incidence uses independent target seal/guard/hold readback. Publication conversion now reuses production canonical/search lowerers and complete independent seal validators under the same live page/host Arc, with mandatory drainage before checkpoint advance. Old original chunks remain solely for authenticated retirement/resume consumers; no blanket deletion or old runtime reader is added. Four-crate test-target compilation passed `033936`; compiler-store tests compiled `040256`. Later checks `040405`, `041252`, `042025` failed before bodies on concurrent historical API or new staging/receipt API integration; repairs are in source and need fresh verification. Six-crate test-target check `20261012T045457.111Z-1d7fda` failed before bodies on six candidate/translation compilation errors, cleanup confirmed; repairs subsequently passed `052212`. Registering typed handlers exposed an owner-shadowing error in `052342`; the correction passed the six-crate test-target check `20261012T053049.136Z-1a4279`, exit0/cleanup confirmed, including registered typed handlers and the aggregate. Later bounded-writer and analysis-owner changes need fresh verification. Authentic upper registry controls passed3 (`20261012T041811.126Z-c09cb1`) and upper-record collector controls passed3 (`20261012T042152.470Z-4c6bb1`), both cleanup confirmed. Typed DAG/binding conversion, complete registry/aggregate/CLI, native migration and acceptance remain open. All authenticated completed original output, including unselected/unadmitted contributions, receives physical-only target singleton/dependency inventories when no semantic artifact exists; this grants neither artifact admission nor a reusable product. Journal-owned staging protection transfers only through bounded actual target holds before aggregate EOF, with unknown work retained. No target activation or shared-service/operator change occurred. |

| Campaign prerequisite / G00 validation owner | **Implemented / focused-Tested 2026-10-11:** ADR-0149 and the existing installation owner add an explicit checked UUID-derived campaign unit; fixtures, daemon checks, recovery assets and restart/handoff paths use its identity. `just run --background --label efficiency-campaign-routing -- uv run --no-sync pytest -q tests/scripts/test_surrealdb_service.py tests/scripts/test_surrealdb_fixture.py` passed, run `20261011T161840.282Z-c6ffb5`, cleanup confirmed. Script controls are not native finality evidence. |
| Dedicated source-format installation | **Passed 2026-10-11:** `LCTX_SURREAL_SERVICE_CONFIG=/home/paul/.local/state/library-context/efficiency-alignment-2026-10-11/installation.json just run --background --label efficiency-campaign-install -- uv run --no-sync python scripts/surrealdb_service.py install --campaign --installer target/release/lctx`, run `20261011T161912.657Z-56a20d`, cleanup confirmed. Installation `0ec1d0c0-fbd0-4265-93e5-d9e186bab099`, schema5/eec4, owns its immutable installer and main/validation scopes. A genuine baseline-pinned producer subsequently populated terminal-unadmitted, admitted artifact-only, detached-import and unselected-publication obligations; exact receipts are in the protected source5 checkout. The full migration population matrix and source→target conversion remain incomplete. No shared installation was observed or modified. |
| Cold recovery hash integration | **Implemented / focused-Tested 2026-10-11:** the existing cold backup owner hashes accepted archive writes and exact asset reads during serialization, checks stable file identities and directory inventory, finalizes and syncs before publishing the recovery pointer. Independent restore hashing remains intact. `just run --background --label efficiency-cold-hash-final -- uv run --no-sync pytest -q tests/scripts/test_surrealdb_service.py -k 'backup or restore or recovery or archive_hash_writer'` passed, run `20261011T163527.865Z-6e6dba`, cleanup confirmed. Controls cover short writes, mutation and write/finalization/sync failure without pointer publication or loss of restart ownership. Populated campaign backup (`20261011T170317.898Z-173597`) and actual `restore … --apply` (`20261011T170447.644Z-275b7e`) passed with confirmed cleanup; independent restore verified the newly serialized archive. This is one actual roundtrip, not complete migration/recovery acceptance. The immutable baseline-only originals/warm producer subsequently passed `20261011T193444.056Z-c2d415`: genuine repeated bytes across distinct revisions/paths, explicit empty source, completed/unadmitted content and an acknowledged old-codec warm product are retained in `.campaign/source5/originals-warm.json`. Its first driver run (`20261011T193259.345Z-8e5ebb`) refused storage-owner reassignment before any fixture/native command; the correction retains the existing evidence owner through its consumer API. Final populated schema5/eec4 cold checkpoint `20261011T193605.126Z-921fbf` passed with confirmed cleanup, preserving `source5-complete.tar` under protected recovery object `466254a2-5ace-46ee-b559-60f5f33f6e53`. Target migration, live/unresolved refusal and complete lineage/history matrices remain open; this releases code preparation, not target-format publication acceptance. |
| D04/D05 protected fixture and executable observations | **Implemented / focused-Tested 2026-10-11:** borrow acquires admission and replacement protection before initial proof, then carries an in-process, creator-PID-bound checked generation through readiness, attachment and child setup; changed runtime/descriptor/admission/daemon/executable refuses. Mutable Cargo outputs are descriptor-hashed at every completion; reports/exports preserve original policy, time, digest and binding, with executed bytes/source closure explicitly unknown. No immutable Cargo producer lease was found, so no digest-reuse branch is claimed. Passed `uv run --no-sync pytest -q tests/scripts/test_surrealdb_service.py tests/scripts/test_surrealdb_fixture.py tests/scripts/test_verify.py tests/scripts/test_executable_observation.py tests/scripts/test_verify_evidence_cli.py`, followed by the same modules with `-k 'checked_generation or borrow_protects or failed_borrow or attachment_child_setup or native_prerequisites or original_completion or completion or check_daemon'` for final hardening. Integrated campaign/verify exercise remains part of combined acceptance. |
| D06–D09 development observation grain | **Implemented / focused-Tested 2026-10-11:** exact rerun retains Cargo package/target/features/profile and narrows compilation before its case filter; run completion resolves only indexed enrolled components; status reads report metadata without decoding; explicit report invalidates by reader identity; per-root detail requires debug while terminals remain visible. Hakari captures declarations and target shape, with a conservative full-source fallback qualified by actual Cargo metadata equivalence for unsupported symlink layouts. Passed `uv run --no-sync pytest -q tests/scripts/test_freshness.py tests/scripts/test_verify_evidence_cli.py tests/scripts/test_compile_profile.py tests/scripts/test_runs.py tests/scripts/test_verify_evidence_nextest.py` plus changed focused controls, including actual six-kind Cargo/nextest experiments with an invalid unselected sibling. Passed `cargo nextest run --release -p cpg-core --lib -E 'test(=stage_runtime::tests::root_detail_requires_debug_while_coarse_terminals_remain_visible)'` (one selected control). Actual checkout freshness safely reported existing workspace-hack `percent-encoding` drift without regeneration. No elapsed-time claim or repository quality closure. |
| S06 bounded sorter | **Implemented / focused-Tested 2026-10-11:** fitting rows stay in charged memory with rewind and point lookup; temporary directories appear only on spill; bounded fan-in merges a retained tail directly with disk runs and preserves full-value collision refusal. `cargo nextest run --locked --offline --release -p lctx-surrealdb --lib -E 'test(ordered_rows::tests::)'` passed17 controls,166 skipped (run `a3b6ce52-1784-4569-bbdf-8e3377b01f3e`). Consumer/native integration remains in combined acceptance. |
| S01 exact policy and reference consumer | **Implemented / unit-Tested 2026-10-11, physical cutover pending:** ADR-0150 supersedes the old approximate-retrieval clause while carrying all evidence/value/context/deployment obligations; current architecture/runbook consumers now state exact ranking. Private numerical replay consumes `native_exact` observations and compares their order/scores with independent worker scalar results. `uv run --no-sync pytest -q tests/scripts/test_programmatic_native_numeric.py -k exact_reference` passed the tie/order/missing/duplicate/nonfinite/incorrect-score refusal control. Actual native numerical replay, served-candidate assertion and HNSW/index-identity retirement remain not_run/pending; no finding closure. |
| C01/C02/S02/S08 lower contracts and captured requests | **Implemented / focused-Tested before D02 physical move, 2026-10-11:** model owns fact/embedding contracts, verified tokenizer bytes feed construction once, executable constants are process-prepared, and captured requests bind the actual ordered/multiplicity-preserving acquisition inventory. `cargo nextest run --locked --offline --release -p cpg-core -p cpg-extract -p lctx-serving --lib --test fake_embedding --test bundle --test producer_fingerprint --test executable_identity -E 'test(service_error_adapts_only_typed_unavailability_at_the_compiler_boundary) or test(contract_only_freezes_one_value_admits_tokens_first_and_releases_budget) or test(model_facts_inventory_binds_every_executable_without_analyzer_execution) or test(executable_constant_controls) or test(pagination::tests) or binary(fake_embedding) or binary(producer_fingerprint) or binary(executable_identity)'` passed15 controls, run `47c9b4d3-268b-42b2-9bea-79a2306ed25b`. Model/embed/tokenizer passed11 (`83c967e8-f23f-4221-af14-91f0ef70d36f`), captured-request passed4 (`c665153a-eb17-455e-a9c6-70ae7f8db6ce`), fork controls passed9. Native held attachment, origin-sensitive provider attribution and NotRequested reuse remain pending; complete-only eligibility remains conservative. |
| A01–A06 and D01 | **Implemented / focused-Tested before D02 physical move, 2026-10-11:** shared checked conditions, prepared structural adjacency, indexed complete Summary discovery, exact bounded analytics, indexed synthesis and charged cross-root embedding queues preserve semantic/refusal boundaries. Focused model passed49 (`c08fa85e-a50f-487e-a846-8d6104805ba3`), analytics passed6 (`8d5d8145-7403-4a90-97fa-e0a29d19e06a`), pure Session passed10 (`57e8bf2d-506e-406f-8261-2c37e11f7b24`), retained-capacity passed1 (`31d55968-d28f-414d-9f4b-2902bd2ef25b`). An earlier Session filter failed its additional native case because no fixture configuration was supplied (`a3ac8a19-e197-40b7-ae27-62914b207c8c`); A07/D01 actual integration subsequently failed: `LCTX_SURREAL_SERVICE_CONFIG=/home/paul/.local/state/library-context/efficiency-alignment-2026-10-11/installation.json just fixture -- cargo nextest run --release -p cpg-core --lib --test compiler_analysis --test compiler_behavior --test compiler_artifacts` with the explicitly selected analysis/execution/retrieval filter ran19 cases (`3f329f0c-6994-4d90-84a7-92548cfb2b79`): four pure cases passed and all15 native bodies timed out at the unchanged360s deadline amid transaction conflicts. No native analysis pass is claimed. Command process cleanup is confirmed, but attachment `76b7677b-5fe7-4b20-a70e-dc6d03760833` retains32 started native owners and remote completion unknown. Raw receipt/JUnit/attachment snapshot are retained under `build/efficiency-alignment/a07-3f329f0c-6994-4d90-84a7-92548cfb2b79`; source request identities remain evidence anchors, not inferred attempt ownership. Four further serving integration controls and old-format extras/checkpoint wait for authenticated recovery. D01 grouped controls compile; distinct fixture/profile observations stay separate. |
| N01/N02 ingress foundation | **Implemented / partial focused evidence, 2026-10-11:** fused immutable rows/guards/holds, one internal request digest, small intent bindings, cooperative retry backoff, and model-owned checked canonical ingress. Campaign run `20261011T170602.864Z-e36d7a` selected six controls: five passed including actual late-collision atomic rollback; budget assertion failed because it expected `Limit` instead of the pool's `Resource`. The assertion is corrected and construction now reserves before guard/hold expansion, retaining pending-window charge through acknowledgement. Current pure controls passed11 (`525146ac-3bbe-450e-a202-3459625fa2e2`): retry4, original effect receipt1, immutable construction3, ingress acceptance/drain2 and actual OS SIGINT1. Command: `cargo nextest run --locked --offline --release -p lctx-native -p lctx-compiler-store -p lctx-model --lib -E 'test(control::retry::tests) or test(control::receipts::tests) or test(control::immutable_window::tests::windows_) or test(control::immutable_window::tests::budget_) or test(control::immutable_window::tests::construction_) or test(compiler::ingress::tests) or test(checked_ingress_moves_) or test(actual_direct_process_sigint_)'`. That checked-ingress model filter selected no test. A later revealing `cargo nextest run --locked --offline --release -p lctx-model --lib --test domain_resources -E 'test(completed_identity::tests) or test(checked_ingress_moves_)'` passed3 (`07889093-9de4-4c4f-8d99-a6b4ef211f64`), proving canonical Arrow buffer transfer and separate retained charges plus six-family identity sensitivity. Its initial fixture used a non-normalized package name and failed; the fixture was corrected before the pass. Initial compile failed three cfg-test nominal type mismatches introduced by the split; explicit lower fixture connections repaired them. Actual campaign held-incarnation/changed-byte refusal and collision rollback passed2 (`920d4cb4-6d05-4408-b0b0-e4583803c120`), through `just fixture -- cargo nextest run --locked --offline --release -p lctx-native --lib -E 'test(control::held::tests::stale_expected_) or test(control::immutable_window::tests::native_late_collision_)'`, managed run `20261011T183118.536Z-fea4b8`, exit0 and cleanup confirmed. A charged sequenced native sink retains original request/disposition receipts, acceptance/drain ownership and final acknowledgement; providers share it across typed writers. Physical typed lowering reserves before JSON/graph/member/alias construction; originals flatten across typed chunks with pre-copy reservations, and import membership transfers use fully delivered bounded keyset pages. Full provider accepted-work/error and integrated native cancellation controls remain pending. |
| D02 physical admission split | **In progress, 2026-10-11:** ADR-0151 selects `lctx-native` shared mechanics and `lctx-compiler-store` validated admission/completed storage. Actual native/compiler-store/core/publisher files and normal dependencies are split; lower validated restored admission reuses the semantic gate without analyzers, and its owner is retained through publication cleanup. Scoped native/store/core/extract/publisher test compilation passed (`cargo check --offline --release -p lctx-compiler-store -p cpg-core -p cpg-extract -p lctx-publisher --tests`). Actual fingerprint and normal dependency closure controls passed in C07/C08. The root membership-page type mismatch and explicit IntoFuture binding mismatch are repaired. Serving Send lifetime was isolated to a generic borrowed iterator across native awaits; explicit Iterator+Send input repaired it, and `cargo check --release -p lctx-serving --lib --tests` passed7.94s. Full Python binding/CLI test compilation with the delivery ledger passed after repairing two moved `NativeProductCache` CLI consumer paths: `cargo check --locked --offline --release -p lctx-semantics -p lctx --tests` (2026-10-11). Three pure delivery-ledger controls passed with two inherited-span controls (`1b7ab4db-5a51-4964-b9dc-280e24e0fb60`); actual Python/native delivery remains not_run. Parquet remains supported. No dependency/fingerprint closure or current integrated acceptance is claimed yet. |
| S09 server execution owner | **Implemented patch / partial focused evidence, 2026-10-11:** actual pinned server registry/shutdown controls pass22 Python controls including six compiled native lifecycle cases. Delivery-loss and healthy-EOF ownership retain the ephemeral session, LIVE cleanup and accepted permit. First actual campaign successor build (`20261011T172811.033Z-e9ad62`) failed at locked metadata: enabling the existing tokio-util runtime feature adds one locked `futures-util` edge. The exact reviewed feature-edge lock patch leaves every version/checksum/source unchanged. Full actual build passed (`20261011T173545.829Z-659e65`, cleanup confirmed), recipe `635c2302b89a3a9995c3200ff79c373c37c619eddebbc6aee6a0d1ebd73b13f8`, binary SHA256 `4b662edebfb8d522807894ced9e8d7abeec2c65cf7acb28f7f79b3bb2a8d2864`. Actual pinned server gRPC/registry library controls passed52 gRPC plus4 registry cases (`20261011T175515.503Z-3810e5`, cleanup confirmed). Checked campaign maintenance handoff passed (`20261011T180546.422Z-9109b0`, cleanup confirmed), installing server generation `ced8e696fc3faae87651d090cf50fc33ea6ac8584096fa138af7784fa95afb0f` with schema5/eec4 preserved and admission reopened. Actual wire finality remains not_run. Campaign host recovery isolated controls passed292, then20 focused native-host controls after full-process replay hardening. ADR-0158 carries the explicit finite joint scope from ADR-0154 while preserving original Unknown/not_observed status. First actual recovery `20261011T191354.907Z-84a3c6` failed before effects because historical released attachment `53417b67-b9fc-48e4-98e0-35c2dd8ed1f8` also remains protected under the new observation policy; the named joint route passed298 affected host controls plus8 focused interruption controls. Actual joint recovery `20261011T192413.834Z-8b1daa` exited75 at successor startup readiness after publishing the authenticated old-daemon `joined.json`; no recovery receipt or admission reopening is claimed. The dedicated campaign remains in maintenance with both attachments protected. Direct CLI execution and helper import created distinct owner exception/context identities, bypassing the existing40s startup retry on the first refusal; canonical module identity is being repaired without changing deadlines. The CLI/helper identity correction passed302 affected host controls through `uv run --no-sync pytest -q tests/scripts/test_surrealdb_service.py tests/scripts/test_surrealdb_fixture.py tests/scripts/test_storage_owners.py`, including actual CLI/import identity and unchanged40s startup retry/refusal. Exact joint resume `20261011T193227.869Z-871bab` passed, exit0/confirmed process cleanup. Authenticated recovery receipt SHA256 `a2d4301eb51890f96524502f104635d80c7897fc2db9b81185f36e213cb69ea9` records old-registry join, replacement invocation and installation-wide persisted-owner recovery; both named attachments released and maintenance removed. Original32 Started receipts and the legacy command's unqualified transport finality remain unchanged; original request outcomes remain unknown. No historical Cargo-lock log is promoted into prerequest/native finality evidence. Shared server source and installation remain untouched. |
| C07/C08 current-tree focused controls | **Passed 2026-10-11:** `cargo nextest run --locked --offline --release -p lctx-compiler-store -p cpg-extract --lib --test capture --test bundle --test captured_request --test producer_fingerprint --test ruff_context -E 'test(product_codec::controls) or test(verification_controls) or test(event_index_controls) or binary(capture) or binary(bundle) or binary(captured_request) or binary(producer_fingerprint) or binary(ruff_context)'`, run `d4113494-aaa4-4bb5-804e-ec9269b47159`, passed33 across7 binaries,138 skipped. Covers preallocation/corruption/charge, exact closed captured tree, fused SHA copy, Ruff event-index equivalence, captured request/origin and actual normal dependency/fingerprint closure. Two later inherited-field span/path preparation controls passed alongside three pure delivery-ledger controls (`1b7ab4db-5a51-4964-b9dc-280e24e0fb60`), using `cargo nextest run --locked --offline --release -p cpg-extract -p lctx-semantics --lib -E 'test(inherited_span_controls) or test(session::delivery_tests)'`. The later C08 bulk-window implementation and root completion data controls compiled with `cargo check --locked --offline --release -p lctx-compiler-store -p cpg-core --tests`. Combined13-control nextest build failed LLVM ENOSPC before test bodies. Lower-only `cargo nextest run --locked --offline --release -p lctx-compiler-store -p lctx-native -p lctx-model --lib -E 'test(window_controls) or test(product_codec::controls) or test(installed_view::tests) or test(recovery_closure::tests::unbound_)'` passed9 (`053d9f0e-348e-49b6-8a11-af1131d0fab6`),517 skipped: three cache-window, three codec, two installation-identity and one unbound-view recovery control. The four core window controls subsequently passed (`20261012T034328.650Z-597a45`, nextest `dddbca8a-a6fa-4642-ac19-a1875bd33885`,4 passed/104 skipped, exit0/cleanup confirmed): `cargo nextest run --locked --offline --release -p cpg-core --lib -E 'test(normalize::kernel_products::window_controls)'`. These cover aligned request multiplicity/absence, optional-hit headroom release, effect-free resource retry and releasing optional encodings before output; no shared cache purge or parallelism cap was applied. Native provider driver, restored signal finality, ordinary provider hits, cache quota/leases and batched consumer integration remain open. |
| S07/D10 request and process owners | **Implemented / partial focused-Tested 2026-10-11:** Rust request admission now precedes embedding; preparation/execution share a scoped budget, native accepted tasks survive delivery cancellation, and Python retains one HTTP client through joined close. Per-process SIGINT/SIGTERM owners fence admissions and retain native/read/session cleanup independently from child exit. Fixture receipt controls preserve unknown or failed native finality despite child0 and confirmed process cleanup. Affected Python controls passed50; native-extension compile and actual direct-process/native transport checks remain pending. A new fixture-backed subprocess control now retains the real first-acknowledged native stream across actual SIGINT and SIGTERM, then checks joined child, logical/physical query drain, session invalidation/detach and typed native receipt; compilation and execution are not_run. |

| S09 exact failed-owner recovery | **Implemented / focused host-Tested, 2026-10-12:** ADR-0158 carries the prior named joint recovery contract and permits the fixed failed finite-query wire owner only with complete local-terminal/remote-confirmed receipt, no storage/committed obligations and its explicit failure inventory. A dead plain ordinary maintenance marker transfers only after immutable original-byte preservation; specialized/live/foreign/malformed owners refuse. Recovery cannot close native admission before acquiring its own maintenance marker. `uv run --no-sync pytest -q tests/scripts/test_surrealdb_fixture.py -k native_host` passed36 (`20261012T052921.017Z-20d950`, exit0/cleanup confirmed), including interrupted preservation-before-transfer replay and unchanged original failed receipts. Initial `052822` failed4 because the old exception path attempted close-admission under an unowned rejected marker; repaired `052848` passed34 before adding foreign-owner and preservation-interruption cases. Actual exact recovery of IAM-failed attachment `1bee9b72-0438-4481-a30c-7506bfe33833` passed (`20261012T053059.934Z-db6fd8`, exit0/cleanup confirmed): `LCTX_SURREAL_SERVICE_CONFIG=/home/paul/.local/state/library-context/efficiency-alignment-2026-10-11/installation.json just service maintenance --recover-native-attachment 1bee9b72-0438-4481-a30c-7506bfe33833`. Authenticated host receipt SHA256 `f188bf67adc2450be84ae93f331804f215ac5eb89f4c2cda10f0de25282e6227` supplements unchanged original failure/Unknown; the exact attachment is released and ordinary maintenance finalized. The original production semantic exporter then passed under closed/drained maintenance (`20261012T053134.763Z-b70ad2`, 1 test, exit0/cleanup confirmed). Its create-new 1,860,408-byte packet SHA256 `9101da30a83463ea700bb06d41379feb99a17f8c134362c96bf461ef127354be` authenticates three actual artifact associations, including the committed publication marker, with consumer `efficiency-source5-semantic-migration`; original source content was unchanged. Existing PD02 recovery regression controls passed12 (`20261012T053145.613Z-f3dddb`), command `uv run --no-sync pytest -q tests/scripts/test_surrealdb_fixture.py -k 'pd02_host_recovery or pd02_interrupted_recovery or pd02_recovery_refuses'`, exit0/cleanup confirmed. No original outcome is promoted. |

| Native capability focused controls | **Passed within focused scope, 2026-10-12:** `20261012T053312.168Z-617a4f`, nextest `cb3785d5-2b74-4a68-b75c-59a08cb4510b`, passed12/exit0/cleanup confirmed across retained-product, pending-product, detached-completion, immutable-seal, candidate, supplier and cancellation controls. The intended borrowed-reader-pin case was omitted by a wrong sibling-module filter; its exact follow-up passed1 (`20261012T053946.144Z-571035`, nextest `83fc1ad8-4cc4-8d1a-e58c4b427798`, exit0/cleanup confirmed), using the actual `reader::streaming_tests` filter. These pure controls establish neither live format6 native acceptance nor full-track closure. |
| Design-phase view cutover | **Regeneration selected / implementation in progress, 2026-10-12:** ADR-0161 replaces opaque preservation. The dedicated campaign's main/validation fixture views are explicitly regenerated by current producer/compiler/publication/serving controls; no claim of historical semantic migration. Interrupted old upper work remains Unknown. Remove preservation-only converters/adapters and old runtime namespaces; exact campaign child/daemon quiescence and current installation readback precede opening. Shared paused/operator state remains untouched. Current functional acceptance is pending; historical campaigns and benchmarks are not gates. |
| C06 analysis-owner integration | **Source integrated / focused controls under repair, 2026-10-12:** ADR-0160 records whole-owner compaction and exact protected predecessor premises. Model test-target compilation passed (`20261012T062412.719Z-25ea66`, exit0/cleanup confirmed); the later seven-crate pass is recorded below. Earlier six owner/shared-boundary/historical-assurance controls passed (`20261012T055331.657Z-836b19`, exit0/cleanup confirmed), before the four later consumers. The revealing model selection (`20261012T065225.369Z-ac77e5`, nextest `4521ad31-dd16-4c28-9723-b5c67732ab8e`) passed3 and failed9, exit100/cleanup confirmed: its shared fixture constructed invalid contributor-free CompletedViews before reaching the intended invariants. Repairs use genuine explicit-empty terminal supplier descriptors. Invariant Uniform inventory is canonical/unprefixed, while current producers still capture and validate exact real checkpoint roles; historical empty bridges grant no current compact authority. Nine compact whole-owner deltas and Structural/Synthesis/CatalogEvidence/Selection consumers are in source. Actual producer/native/four-consumer acceptance remains open; whole-output NotRequested reuse remains disabled. |
| Current migration integration diagnostics | **Compilation passed / focused functional repair in progress, 2026-10-12:** CLI-inclusive seven-crate `cargo check --locked --offline --release -p lctx-native -p lctx-compiler-store -p cpg-core -p lctx-publisher -p cpg-extract -p lctx-serving -p lctx --tests` passed (`20261012T063401.664Z-bce665`, exit0/cleanup confirmed), including forensic writers and direct-signal controls. Earlier `060833`, `060926` and `062636` compile failures retain their original receipts; this pass follows their repairs. The later pure migration selection (`20261012T064046.905Z-c49d67`, nextest `73922d38-decb-40b7-89ae-54ef2938e7d6`) passed18 and failed3, exit100/cleanup confirmed: two failures exposed current graph decoding of historical records, and one control incorrectly rejected legitimate `FOR UPDATE` locking. The old-record codec repair subsequently failed compilation (`20261012T065718`, exit101/cleanup confirmed) on missing Serialize implementations, under repair. Source inspection also exposed the need for an actual new complete-empty Uniform binding derived from authenticated original selected contributors; no historical epoch or compact authority may be invented. Host original-export/live-owner controls passed48 (`20261012T062406.703Z-9bf818`, exit0/cleanup confirmed), including prepared executable identity and exact source5 step routing. Earlier overlapping host selections are not added into a qualification count. Actual upgrade/native functional effects remain not_run. |

**Latest compact-consumer control, 2026-10-12:** `cargo nextest run --locked --offline --release -p lctx-model --lib -E 'test(compact_analysis_) or test(compact_structural_) or test(compact_synthesis_) or test(compact_catalog_parent_checks_) or test(catalog_evidence_accepts_protected_compact_) or test(catalog_selection_rechecks_compact_)'` (`20261012T070153.851Z-fa695c`, nextest `679ce125-9fb2-463c-b040-d648016e93fd`) passed10 and failed3, exit100/cleanup confirmed. Structural and both CatalogEvidence/Selection actual invariants passed, including missing/overlapping proof and exact parent membership refusal. The remaining shared-fixture assertion incorrectly expected every owner domain to have zero members for empty Python input; embedding and analytics retain real input-level subjects. Their actual domain meaning must remain unchanged. Synthesis and nine-owner control acceptance remain open pending this repair.

Package rows below describe the approved complete delivery contract; the evidence table above owns current partial execution. No source finding is closed by plan approval or a partial prerequisite.
Update this section with actual delivered prerequisites and dated receipt links as execution progresses;
keep companion design prose current without duplicating progress logs.

| Packages / responsible component | Delivery | Required working prerequisite |
|---|---|---|
| G00 — coordinator and affected architectural owners | Publish approved rule/contract changes with responsible implementation slices | Operator decisions above are available; exact owner/consumer source inventory refreshed per slice |
| [N01–N02](efficiency-alignment-native-plan_2026-10-11.md#N01) — native effects/ingress | Combined bounded effects, recoverable retry, canonical ingress, bounded pages/acknowledgement scheduling | Existing request recovery/ownership; no N03 redesign required |
| [N03](efficiency-alignment-native-plan_2026-10-11.md#N03) — native control | Narrow fencing with complete safety inventory and controls | G00 affected decision; pinned engine/predicate/recovery inquiry resolved |
| [N04](efficiency-alignment-native-plan_2026-10-11.md#N04) — native compiler store | Held candidate, exact completed installation and view-local contribution mapping | Complete physical contract and migrated enumeration/recovery consumers |
| [N05](efficiency-alignment-native-plan_2026-10-11.md#N05) — publisher/lifecycle | Root ownership transfer and safe abandonment | Existing resumable retirement; marker/cleanup semantics preserved |
| [N06–N07](efficiency-alignment-native-plan_2026-10-11.md#N06) — state identity/seal | Complete composed identity, then one ordinary lowering/reconciliation | N06 frozen/drained complete witness before N07 check removal |
| [N08](efficiency-alignment-native-plan_2026-10-11.md#N08) — publisher/serving realization | Current derivations and guarded unchanged publication | Working derivation identity/address/eligibility and actual migration, not handle-only design |
| [N09](efficiency-alignment-native-plan_2026-10-11.md#N09) — recovery/product/binding owners | Four separate terminal reuse/reclaim/product release/binding compaction obligations | Candidate admission for reuse; actual retained consumers released before their incidence retires |
| [N10–N11](efficiency-alignment-native-plan_2026-10-11.md#N10) — originals/admission | Content blobs; protected durable premises | N10 actual blob consumer transition; N11 N04/N06 physical validity and validator identity |
| [C01–C02](efficiency-alignment-compiler-plan_2026-10-11.md#C01) — capture/provider contracts | Lower contracts, complete keys/output eligibility | Actual acquisition metadata and provider inventory; G00 affected reuse decisions |
| [C03–C04](efficiency-alignment-compiler-plan_2026-10-11.md#C03) — compiler/native interface | By-reference hits and exact member access | C02 semantic keys + N04 physical completion/protection; N09 for unadmitted recovery hits |
| [C05](efficiency-alignment-compiler-plan_2026-10-11.md#C05) — model/reuse consumers | Value cutoff with exact provenance and profile neutrality | Split actual dependencies/config outputs; receipt-bearing consumers migrated |
| [C06–C08](efficiency-alignment-compiler-plan_2026-10-11.md#C06) — compiler/providers/cache | Demanded unions/non-request; shared preparation/capture integrity; portable-cache repair | C06 migrated model/frontier/assessment/scoped consumers; C07 N02 for overlap; C08 existing cache ownership |
| [A01–A03](efficiency-alignment-analysis-plan_2026-10-11.md#A01) — model/Structural/Summary | Shared checked conditions, immutable graph prep, indexed summary discovery | Existing exact semantic/proof contracts; A01 where consumers need shared lowering |
| [A04–A06](efficiency-alignment-analysis-plan_2026-10-11.md#A04) — analytics/synthesis/retrieval | Exact bounded analytics preparation, synthesis joins, cross-root ready embeddings | Exact method policy; existing dedup/refusal; S09 drain contract for in-flight integration |
| [A07](efficiency-alignment-analysis-plan_2026-10-11.md#A07) — model/analysis consumers | Integrated semantic/refusal boundary | Working changed A packages and C05/C06 receipt/coverage consumers |
| [S01–S02](efficiency-alignment-serving-plan_2026-10-11.md#S01) — ranking/process prep | Exact policy/reference migration and executable constants | S01 actual numerical consumers before ANN removal; S02 immutable executable epoch |
| [S03–S05](efficiency-alignment-serving-plan_2026-10-11.md#S03) — serving/native selected access | Context-valid compact prep, page hydration, exact bounded ranking | S03 witness before S04; N08 before retained shortcut; S01 policy/reference before changed ranking |
| [S06–S08](efficiency-alignment-serving-plan_2026-10-11.md#S06) — sorter/MCP/tokenizer | In-memory small sort, separate demand/HTTP lifetime, verified-byte construction | Existing resource/transport owners; all sorter and resource entrypoints migrate |
| [S09](efficiency-alignment-serving-plan_2026-10-11.md#S09) — CLI/native transport | Signals reach owned drain; truthful remote finality | Pinned remote drop contract or actual functional evidence; unknown remains owned |
| [D01–D02](efficiency-alignment-development-plan_2026-10-11.md#D01) — provider controls/build owners | Grouped immutable observations and narrowed dependency/fingerprint closure | C01 semantic boundary for D02; complete fixture identity for D01 |
| [D03](efficiency-alignment-development-plan_2026-10-11.md#D03) — Cargo/verification | Qualified shared ordinary dev/test defaults, distinct release | This track's affected functional controls, final D02 closure before default switch |
| [D04–D07](efficiency-alignment-development-plan_2026-10-11.md#D04) — fixture/evidence/rerun/storage | Protected generation, original observations, exact build targets, local completion | Existing actual lifetime/metadata owners; no global cache or registry redesign |
| [D08–D10](efficiency-alignment-development-plan_2026-10-11.md#D08) — profiling/diagnostics/direct tests | Metadata status, selected diagnostics/freshness inquiry, direct-process cancellation | D10 S09 common primitives + separate actual test entrypoint/drain; D09 equivalence before capture narrowing |
| X01 — coordinator and integrated consumers | Functional acceptance and real migration/retirement closure | All changed contracts integrated; unresolved findings/limits dispositioned without false closure |
| X02 — coordinator | Handoff for later resumption of old plans | X01 functional completion; no automatic old-plan edits or supersession |

Priority is consequence, not a global barrier. Address tokenizer selected-byte validity, honest corpus
origin, exact ranking statements and retained-derivation identity early. In parallel, ready independent
work removal includes N01/N02, C07 preparation, A05, S02/S08 and D06–D09. Implement N03's selected
protocol with its complete mutation partners before removing the existing fence; its native controls
constrain that package rather than independent N01/N02 work. C02+N04 make
reuse both sound and inexpensive. C05/C06 and N06/N08/N10/N11 are larger semantic/migration changes and
need their consumers alongside them. Optional analytics need not gate delivery of catalog improvements,
but their declared functional work still belongs to this track's exit.

An agreed interface permits design work; it is not a working prerequisite. A smaller once-v3 identity
improvement can land before N06, but cannot close composed-identity work. Keeping safe fencing while its
replacement remains unresolved is safe interim behavior, not N03 closure. A by-reference contract cannot
be credited while recovery still relies on copied per-attempt rows. Shared source files require one edit
owner even for logically independent packages: compiler/workspace, native control/lifecycle, serving
preparation and verify/rerun receipts are the main coordination surfaces.

## 5. Reconciled source findings — sole current disposition

Every row is **scheduled / Planned** unless explicitly marked **retained limitation / external**.
The route names the responsible package and its closure contract, not a finding already resolved.
A source finding with multiple obligations closes only when each has its own evidence or an explicit
accepted residual disposition. Audit groups do not merge those obligations. Source anchors below retain
original IDs; audit mapping follows in the next table.

| Original finding | Reconciled diagnosis, correction and closure route |
|---|---|
| [O-F01](../design_review/reviews/design_review_efficiency-alignment-codebase_2026-10-11.md#f01) | Shared effect conflict remains; **N03** complete predicate/recovery safety, not an unconditional read-only fence mandate. N01 independent. |
| [O-F02](../design_review/reviews/design_review_efficiency-alignment-codebase_2026-10-11.md#f02) | Retry behavior needs recoverable backoff/observation; **N01**, never infer rollback from stop/lost acknowledgement. |
| [O-F03](../design_review/reviews/design_review_efficiency-alignment-codebase_2026-10-11.md#f03) | Small separately fenced effects; **N01** fused bounded windows and collision/retirement controls. |
| [O-F04](../design_review/reviews/design_review_efficiency-alignment-codebase_2026-10-11.md#f04) | Originals window per chunk; **N02** cross-chunk windows now, **N10** content sharing separately. |
| [O-F05](../design_review/reviews/design_review_efficiency-alignment-codebase_2026-10-11.md#f05) | **N01** separates unused intent bindings, duplicate request hashing and payload representation; retain durable intent/outcome protocol. Its wholesale elimination is not required. |
| [O-F06](../design_review/reviews/design_review_efficiency-alignment-codebase_2026-10-11.md#f06) | Growing stream lifetimes; **N02** immutable keyset pages/owned bounded spill and truthful cancellation, without deadline increases. |
| [O-F07](../design_review/reviews/design_review_efficiency-alignment-codebase_2026-10-11.md#f07) | Sequential acknowledgement/scheduling; **N02/C07** bounded accepted work through drain, no hidden unbounded queue. |
| [O-F08](../design_review/reviews/design_review_efficiency-alignment-codebase_2026-10-11.md#f08) | Re-canonicalization of checked Batch; **N02** trusted canonical ingress, retain untrusted checks. |
| [O-F09](../design_review/reviews/design_review_efficiency-alignment-codebase_2026-10-11.md#f09) | **N06/N07** complete identity/freeze and ordinary reuse; backing reads are already batched, not native read-per-row. |
| [O-F10](../design_review/reviews/design_review_efficiency-alignment-codebase_2026-10-11.md#f10) | **N07** lower once/reconcile stored once; independent cold checks remain, same lowering replay is not an oracle. |
| [O-F11](../design_review/reviews/design_review_efficiency-alignment-codebase_2026-10-11.md#f11) | **N08** current derivation for retained manifests + guarded unchanged publication. Ordinary fresh compile already fingerprints native implementation. |
| [O-F12](../design_review/reviews/design_review_efficiency-alignment-codebase_2026-10-11.md#f12) | **N05** root ownership transfer; existing retirement already charges/resumes high degree, preserve it. |
| [O-F13](../design_review/reviews/design_review_efficiency-alignment-codebase_2026-10-11.md#f13) | **N09** separate terminal-result admission/reuse and abandoned-root reclamation; neither implies the other. |
| [O-F14](../design_review/reviews/design_review_efficiency-alignment-codebase_2026-10-11.md#f14) | **N10** content blobs under artifact identity, all retrieval/recovery/release consumers. |
| [O-F15](../design_review/reviews/design_review_efficiency-alignment-codebase_2026-10-11.md#f15) | **N11** durable protected premises; missing validator/incarnation proof rechecks. Portability is not trust. |
| [O-F16](../design_review/reviews/design_review_efficiency-alignment-codebase_2026-10-11.md#f16) | **C02/C03** complete captured-provider keys/outcomes then held reuse; include PD01 metadata and determinism controls. |
| [O-F17](../design_review/reviews/design_review_efficiency-alignment-codebase_2026-10-11.md#f17) | **N04/C03/N09** by-reference attach, installed-view completion, binding/history lifecycle. Recovery already follows logical dependency closure. |
| [O-F18](../design_review/reviews/design_review_efficiency-alignment-codebase_2026-10-11.md#f18) | **C05** split config siblings/value cutoff; exact receipts retained/regenerated; remove debug-only cutoff graph while preserving binding checks. |
| [O-F19](../design_review/reviews/design_review_efficiency-alignment-codebase_2026-10-11.md#f19) | **C02/C05** explicit neutrality over all actual dependencies and availability; not a hash-field deletion. |
| [O-F20](../design_review/reviews/design_review_efficiency-alignment-codebase_2026-10-11.md#f20) | **C08** separate crossing/encoding/eviction-planning costs; SUM and fenced admission are distinct roles, no proven G1 defect; retain default-on RC18. |
| [O-F21](../design_review/reviews/design_review_efficiency-alignment-codebase_2026-10-11.md#f21) | **C04** mandatory key-only direct demand plus N04 completion; no full payload scan for sparse keys. |
| [O-F22](../design_review/reviews/design_review_efficiency-alignment-codebase_2026-10-11.md#f22) | **C04/N04** existing exact member address resolves the old owner-count alternative; preserve view-local authority. |
| [O-F23](../design_review/reviews/design_review_efficiency-alignment-codebase_2026-10-11.md#f23) | **C06** consumer-demanded unions, complete output inventory including explicit empties. |
| [O-F24](../design_review/reviews/design_review_efficiency-alignment-codebase_2026-10-11.md#f24) | **C06** domain-total compact non-request with frontier/assessment/scoped-query migration. |
| [O-F25](../design_review/reviews/design_review_efficiency-alignment-codebase_2026-10-11.md#f25) | **C06** invocation preparation; admission already uses compact projection, avoid overstating reconstruction. |
| [O-F26](../design_review/reviews/design_review_efficiency-alignment-codebase_2026-10-11.md#f26) | **C07** reusable Pyrefly preparation and conditional Ruff walk fusion; diagnostic/noqa/correspondence parity. Reparse evidence is narrower than every class-containing module. |
| [O-F27](../design_review/reviews/design_review_efficiency-alignment-codebase_2026-10-11.md#f27) | **C07** retain the existing OnceLock AllThreads pool; amortize actual wave preparation/dispatch and preserve joined completion/serial extraction. A new pool is not required. |
| [O-F28](../design_review/reviews/design_review_efficiency-alignment-codebase_2026-10-11.md#f28) | **C07** once-built multi-owner path/use/derivation indexes. |
| [O-F29](../design_review/reviews/design_review_efficiency-alignment-codebase_2026-10-11.md#f29) | **C07** hash while copying; keep point-of-use integrity where ambient mutation remains possible. |
| [O-F30](../design_review/reviews/design_review_efficiency-alignment-codebase_2026-10-11.md#f30) | **C07** selected captured-tree revision/modified identity, not checkout HEAD or global cleanliness. |
| [O-F31](../design_review/reviews/design_review_efficiency-alignment-codebase_2026-10-11.md#f31) | **A01** checked condition preparation across real consumers; keep reduction/effective support and bounds. |
| [O-F32](../design_review/reviews/design_review_efficiency-alignment-codebase_2026-10-11.md#f32) | **A02** immutable Structural indexes/ordered adjacency and bounded high-degree consumption, same witnesses/Stops. |
| [O-F33](../design_review/reviews/design_review_efficiency-alignment-codebase_2026-10-11.md#f33) | **A04** exact preparation/bounded kernels and explicit necessary pair-work residual. No transparent shortlist/cap/bipartite semantic substitution; performance/method expansion **external M04**. |
| [O-F34](../design_review/reviews/design_review_efficiency-alignment-codebase_2026-10-11.md#f34) | **A06** cross-root ready-text batches; existing dedup/batching/refusal retained, not a missing batching API claim. |
| [O-F35](../design_review/reviews/design_review_efficiency-alignment-codebase_2026-10-11.md#f35) | Separate **S02** executable constants, **S03** context/pin preparation, **S04** page hydration; each lifetime needs its own evidence. |
| [O-F36](../design_review/reviews/design_review_efficiency-alignment-codebase_2026-10-11.md#f36) | **S05** selected exact scoring and bounded kernels; cohort-first is conditional and can lose for sparse eligibility. |
| [O-F37](../design_review/reviews/design_review_efficiency-alignment-codebase_2026-10-11.md#f37) | **S01** exact policy plus actual ANN reference migration before DDL removal; “no consumer” was false. |
| [O-F38](../design_review/reviews/design_review_efficiency-alignment-codebase_2026-10-11.md#f38) | **S03/S05** owner-correct selected preparation and bounded query inquiry; catalog alias counterexample/G1 failure not established. |
| [O-F39](../design_review/reviews/design_review_efficiency-alignment-codebase_2026-10-11.md#f39) | **S07** pending-demand separation for tools/resources; Python slots are not native CPU thread counts. |
| [O-F40](../design_review/reviews/design_review_efficiency-alignment-codebase_2026-10-11.md#f40) | **S06** no-spill small path, reservation-sized runs, bounded fan-in; all-runs merge can lose boundedness. |
| [O-F41](../design_review/reviews/design_review_efficiency-alignment-codebase_2026-10-11.md#f41) | **Retained limitation / external M01:** per-process bounds do not establish host bounds; timeout causes unapportioned. RC12 retains policy; repeated-work remedies are separately scheduled. |
| [O-F42](../design_review/reviews/design_review_efficiency-alignment-codebase_2026-10-11.md#f42) | **S09/D10** separate CLI and direct-test integration and actual remote drain/recovery. PD02 is an evidenced execution-owner defect; implementing/qualifying its selected correction is a prerequisite, not another open inquiry. |
| [O-F43](../design_review/reviews/design_review_efficiency-alignment-codebase_2026-10-11.md#f43) | **D09** selected per-root diagnostics, preserve coarse terminal/drain and current phase controls. |
| [O-F44](../design_review/reviews/design_review_efficiency-alignment-codebase_2026-10-11.md#f44) | **S08** construct from selected verified buffers; narrow G3 validity issue, not claimed unchanged loaded-byte identity/G4 defect. |
| [O-F45](../design_review/reviews/design_review_efficiency-alignment-codebase_2026-10-11.md#f45) | **D01** grouped immutable observations, fresh semantic variants and truthful case grain. |
| [O-F46](../design_review/reviews/design_review_efficiency-alignment-codebase_2026-10-11.md#f46) | **C01/D02** real dependency and fingerprint boundaries; feature gates alone insufficient. |
| [O-F47](../design_review/reviews/design_review_efficiency-alignment-codebase_2026-10-11.md#f47) | **D03** independently qualify shared dev/test here; `cargo check` is not another optimized codegen tree; no old-plan dependency. |
| [O-F48](../design_review/reviews/design_review_efficiency-alignment-codebase_2026-10-11.md#f48) | **D04** protected service generation and **D05** original completion observation are separate contracts; cache lifetime may span invocations. |
| [O-F49](../design_review/reviews/design_review_efficiency-alignment-codebase_2026-10-11.md#f49) | **D06** recorded Cargo kind/name/package/feature/profile, not always `--test` for `package::name`. |
| [O-F50](../design_review/reviews/design_review_efficiency-alignment-codebase_2026-10-11.md#f50) | **D07** exact known-owner sidecar completion; main receipt already precedes global scan. |

### Audit traceability

Each audit row retains its own corrective/preservation contract even when an original finding shares
its package. The six additional findings have explicit owners; audit F20 strengthens original F30.

| Audit finding | Package routes and distinct obligation |
|---|---|
| [A-F01](../design_review/reviews/design_review_efficiency-alignment-audit_2026-10-11.md#F01) | N01/N02/N03/N05/N10; effect grain, safe fencing, root transfer and byte sharing are separate closures |
| [A-F02](../design_review/reviews/design_review_efficiency-alignment-audit_2026-10-11.md#F02) | C04/N04; selected keys and installed-view authority |
| [A-F03](../design_review/reviews/design_review_efficiency-alignment-audit_2026-10-11.md#F03) | N02/N06/N07/N11; complete physical witnesses versus cold assurance |
| [A-F04](../design_review/reviews/design_review_efficiency-alignment-audit_2026-10-11.md#F04) | C02/C03/N04/N09; complete eligibility, protection, preflight and terminal recovery |
| [A-F05](../design_review/reviews/design_review_efficiency-alignment-audit_2026-10-11.md#F05) | C05/C06/C08; identity/profile, coverage and portable-cache costs remain separate |
| [A-F06](../design_review/reviews/design_review_efficiency-alignment-audit_2026-10-11.md#F06) | N08; current retained-manifest derivation and real generation cutover |
| [A-F07](../design_review/reviews/design_review_efficiency-alignment-audit_2026-10-11.md#F07) | C06/C07/A01/A02/A04/A06; actual owner lifetimes, exact methods and bounded ready work |
| [A-F08](../design_review/reviews/design_review_efficiency-alignment-audit_2026-10-11.md#F08) | S01–S05; reference migration, preparation lifetimes, page and score semantics |
| [A-F09](../design_review/reviews/design_review_efficiency-alignment-audit_2026-10-11.md#F09) | S07/S09/D10; native demand versus embedding and two external cancellation entrypoints |
| [A-F10](../design_review/reviews/design_review_efficiency-alignment-audit_2026-10-11.md#F10) | S08; selected validation premise survives construction |
| [A-F11](../design_review/reviews/design_review_efficiency-alignment-audit_2026-10-11.md#F11) | S06; bounded sorter; host aggregate policy remains RC12/M01 |
| [A-F12](../design_review/reviews/design_review_efficiency-alignment-audit_2026-10-11.md#F12) | C01/D01/D02/D03/D09; fixture, dependency/profile and diagnostic obligations |
| [A-F13](../design_review/reviews/design_review_efficiency-alignment-audit_2026-10-11.md#F13) | D04–D07; distinguish service proof, executable evidence, exact builds and owner completion |
| [A-F14](../design_review/reviews/design_review_efficiency-alignment-audit_2026-10-11.md#F14) | A03; Summary discovery indexes preserving Pareto proof alternatives and SCC progress |
| [A-F15](../design_review/reviews/design_review_efficiency-alignment-audit_2026-10-11.md#F15) | A05; synthesis joins and complete decision population before brief selection |
| [A-F16](../design_review/reviews/design_review_efficiency-alignment-audit_2026-10-11.md#F16) | C07; indexed Ruff correspondence preserving ambiguity/multiplicity/fallback |
| [A-F17](../design_review/reviews/design_review_efficiency-alignment-audit_2026-10-11.md#F17) | D08; metadata-only status and explicitly owned exact-input analysis cache |
| [A-F18](../design_review/reviews/design_review_efficiency-alignment-audit_2026-10-11.md#F18) | N09 product release; named consumer/retention ownership, no deletion by age/size |
| [A-F19](../design_review/reviews/design_review_efficiency-alignment-audit_2026-10-11.md#F19) | N09 binding incidence/compaction; complete binding semantics and selected-backup consumer migration |
| [A-F20](../design_review/reviews/design_review_efficiency-alignment-audit_2026-10-11.md#F20) | C07; exact captured source origin versus base-plus-modified provenance |

## 6. Investigations, smaller observations and coverage limits

### Resolved design decisions and remaining implementation evidence

The authorized investigation selected the mechanisms below on 2026-10-11. The companion plans carry
their concrete interfaces, rejected alternatives, consumer migrations and preservation constraints;
the [evidence index](../design_review/evidence/2026-10-11_efficiency-design-decisions/README.md)
records decisive sources/probes and limits. These selections are **Proposed designs**, with bounded
Tested premises where stated. They do not close findings or qualify the future implementation.
The remaining controls in the last column verify a selected contract; they are not unresolved
architectural choices. If implementation contradicts a premise, reopen that specific decision with
its evidence and affected consumers rather than silently retaining an undelivered accepted change.

| Question / source lead | Selected mechanism and evidence owner | Remaining implementation evidence |
|---|---|---|
| Complete phantom/predicate partners and recovery/maintenance authority, original R1 | N03 versioned finite write stripes plus exact object/reference/history/backup guards; all-stripe era/admission transition. Tested RocksDB conflicts, absent guards, phantom negative and rollback | Actual 64-stripe layout/admission budget, every mutation partner, recovery/successor/history and destructive-drain interleavings; current global guard stays until cutover |
| Complete provider inputs, outcome determinism and PD01 | C02 one canonical captured request plus acquisition attribution/RECORD/roles/origin; whole-provider Complete and valid uniform NotRequested eligibility. C07 retains existing joined AllThreads pool and serial extraction | Complete production key construction, actual hit/miss/admission and metadata mutations; Partial output stays ineligible even if repeatable |
| Exact view lookup and completion, original R5 | N04 view-scoped immutable six-family completion and exact physical realization mapping; C04 mandatory typed key demand with residuals and direct member addresses | Interrupted/late installation, foreign/missing/virtual/empty/member demand and receiver preflight; no whole-attempt freeze before later compiler frontiers |
| Scoring policy, original R3 | Exact frozen S01; S05 selected-first remains default, cohort-first rejected for this track's sparse/failure-contract costs | Actual ANN reference migration, selected exact results/failures/ties and caps after eligibility |
| View-local predicate cost, original E-06/V06/P-E1 | S03 compact pin/context preparation and bounded selected-root access; preserve current validated query route without an unmeasured wholesale query rewrite | Cold/warm contextual invalidation and native query/refusal controls; no new latency or catalog-alias failure claim |
| Complete composed state, physical validity and migration | N06 logical six-family composition separated from N04 physical completion; N11 explicit factory/implementation/physical-epoch premises, cold invalidate/recheck. Adjacent conversion for non-reconstructible retained obligations | All closure-family mutations, bindings/aliases, incarnation/restart/import controls and actual typed transition; pinned-input rebuild only for proven reconstructible obligations |
| Profile neutrality, sibling cutoff and receipt separation | C05 relation-local value tokens separate from held ActualSource, five configuration groups/per-method requests, separated CatalogCore/Retrieval payloads and explicit per-provider profile policy | Techniques-off analytics changes preserve unrelated compute requests while current exact receipts regenerate; changed consumed values/contracts/coverage invalidate. Source-bearing rows cannot be relabeled |
| Optional analytics primitive substitution | A02 invocation-sorted charged adjacency; A04 retain exact ordered F32→F64 scalar reduction, bounded selection and necessary pair work; A06 existing Session queue with compact checked window-use expectations | Actual graph witnesses/Stops and refusal equivalence; exact analytics ties/winners; cross-root batching attribution, byte ownership and drain |
| Remote stream drop and physical terminal ownership, PD02 | S09/D10 server execution-owner handoff with actual terminal cleanup; retain NativeRows/local drain and recovery-owned unknown on lost connection. Actual disconnect reproduced an unfinalized-transaction error | Implement/qualify the owner fix and its shutdown path; separate CLI and direct-test signals, transport loss and journal/recovery evidence. Healthy service/absent row is not terminal proof |
| Cargo/Hakari metadata-only freshness | D09 declarations/configuration bytes plus complete file/directory shape and placeholders, stable recapture; conservative full route for unsupported symlink/escaping semantics. Actual Cargo/Hakari outputs matched on 988 current inputs | Product capture/discovery integration, ambient-config isolation, concurrent changes and fail-closed fallback; no second independently maintained target authority |
| Cold-backup hash-while-write | Exact asset-reader and sequential finalized archive-writer hashes, metadata appended by name; six finite tar/failure controls passed | Actual restore staging, private source coherence, fsync faults, recovery/restart and dedicated-service cold round trip; independent destination checks remain |

C01/D02 selects existing lower semantic-contract owners and an independently consumed native
contract boundary with responsible fingerprints; Parquet remains explicitly supported through the
public SessionContext surface. D03 selects O2 incremental ordinary dev/test with assertions/overflow
checks, imported O3 non-incremental and distinct release; tiny exact-profile controls passed, while
product qualification/default switching remains its future package. D04/D05 use actual protected
generation capabilities and retain completion-time hashing when protection is absent. C06's compact
NotRequested is exact-domain total at all four consumers. C08 selects the locked Postcard envelope
over unchanged canonical section bodies, with allocation admission and the existing cache lifecycle.
C07 retains the existing joined pool and separate Ruff walks, with complete captured-tree opening
and per-reader closing checks; only Acquisition's independently redundant byte pass is removed.
No optional replacement, delayed executable digest, disabled cache or resource cap is an escape from
the accepted target.

Original V13/P-E1-style inquiry searches supply partial engine knowledge, not an executed plan. V14's
upstream thread behavior is not this producer's set determinism. V15's conflict log matches include
intentional controls and do not apportion production contention. P4's approximately 280-second values
were signal instants, not completion times. The known operation-map V02 error is corrected: TyFlow is
behavioral-only, not a sixth catalog facts provider. Do not revive these as quantitative premises.

### Original non-findings and audit's smaller opportunities

The inventory below preserves original §7.10 and the audit's two smaller opportunities. “Retain” is an
explicit disposition, not omission. Revisit triggers are narrower than a new blanket review.

| Item | Disposition and route / trigger |
|---|---|
| U01 — server-side `ensure_rows_owned` per-row statements | N01/N03 preserve full collision semantics; not per-row network crossings. Measure window latency **after fusion** (M02) before broader set-based replacement |
| U02 — serial post-joined Pyrefly root extraction | C07 prepares/indexes safely; retain serial merge until Natives/input/lifetime parity is established. Provider phase-share measurement M03 before extra overlap |
| U03 — admission Lookup growth (charged estimate 192 B/entry in source review) | Retain explicit charged refusal. M03 representative graph sizes/peak live state trigger a new physical design if needed; estimate is not measured allocation |
| U04 — provider cross-batch seen index (source estimate about 130 B/row) | Retain early typed conflicts. Reopen only if an equivalent early-failure contract and actual memory pressure justify removal |
| U05 — DataFusion Parquet feature | D02 inquiry completed: retain the public unrestricted SessionContext capability; spill itself uses Arrow IPC. Revisit only with an explicit supported-surface decision and consumer migration |
| U06 — process per stdio MCP client | Retain deployment shape. M04 actual multi-client demand/capacity need precedes any shared daemon/admission owner |
| U07 — sequential serving families/channels | S03/S05 remove repeated work first. M04 latency evidence precedes added concurrent streams; overlap is not work reduction |
| U08 — per-call HTTP client | S07 lifespan-owned client/drain; already acknowledged by the original, not an additional audit finding |
| U09 — long-lived Pyrefly compiler worker | Not selected. C02/C03 captured reuse uses existing owners; revisit residency only for a demonstrated residual workload |
| U10 — model declaration causes full rebuild | Retain conservative model cutover and responsible identity; no broad semantic incremental migration introduced |
| U11 — native EXPORT/IMPORT replacing owned recovery | Not selected: selected closure and independent destination admission remain required. N04/N08/N10 migrate the actual recovery owner |
| U12 — experimental nextest failed-only `-R` | Not required. D06 uses actual target metadata and exact existing routes; reassess only for a concrete unmet rerun consumer |
| U13 — model mixes declarations and kernels / line count | D02 follows actual consumer coupling. No crate split solely for size; M03 build attribution can justify later boundaries |
| U14 — cold effect Bytes/JSON expansion | N01/C08 correct active redundant serialization. Broader cold format rewrite awaits M02/M03 significance; no generic new codec framework |
| U15 — four/five gRPC sessions per compile | Retain bounded owner-specific sessions and truthful drain; no shared session pool without an actual lifecycle-compatible need |
| U16 — freshness copies source bodies | D09 selects declarations/configuration bytes plus complete file/directory shape and placeholders, supported by actual Cargo/Hakari equivalence controls; preserve conservative fallback and unknown states |
| U17 — cold-backup repeated hashing | Native companion selects exact hash-while-copy/write with trailing metadata; finite failure controls passed. Preserve stopped-service coherence/independent restore checks; actual backup/restore acceptance and M02 remain future work |

The original's “Positions on prior leads” remains a provenance/reconciliation table in that review.
Its confirmed/residual items flow through the stable O-F routes above. Its refuted/adequate/Not examined
rows are not promoted into new defects or claims of completed old-plan work. This plan neither rereads
those old plans nor inherits their status. The targeted breadth pass below covers selected uncertainty;
remaining unexamined scope is explicitly unqualified.

### Targeted breadth pass during authoring

The operator chose a targeted pass over the audit's named unexamined breadth. This was static advice
and source inspection, not another codebase-wide certification. No protected evaluation data were opened.

| Area sampled | Design consequence and remaining limit |
|---|---|
| Model transfers/consequences and Summary | A01 includes actual transfer consumers and effective support; A03 preserves call/Entry witnesses, finite proof alternatives, SCC progress and residuals. No universal consequence-semantics certification |
| Evaluation internals | Preserve independent public-byte/native numerical observers, duplicate-field/finite-bound handling and no production-derived oracle. A07/S01 integrate affected boundaries; protected gold/heldout remain sealed |
| Acquisition metadata and lexical correspondence | **PD01** adds origin/role/ownership/distribution metadata to C02 keys; C07 preserves Ruff ambiguity and owned/inherited provider meaning. Byte identity alone is insufficient |
| Lexical/native search lowering | S03/S05 retain exact selected context, family/view term statistics and lexical semantics; vector reorder assumptions do not transfer automatically. Remaining engine costs need focused evidence |
| Python middleware and resources | S07 explicitly includes capability resources and final JSON-RPC envelope admission, not just tool calls/inner result bytes. This is not full middleware qualification |
| Server gRPC scheduling/drop | **PD02** source plus dedicated native disconnect establish the missing execution-finality owner; S09/D10 selects a server handoff and truthful recovery-owned unknown. Fix qualification remains Planned; no assertion that every owned NativeRows operation leaks |
| Upgrade/recovery/backup | N04/N08/N09/N10 migrate selected closure and first-binding consumers. Current source4→history-receipt-v5 upgrade is history-specific, not a generic new-format migrator |
| Development owner interactions | D04/D05/D07/D09 separate protected service generation, original executable observation, local completion and metadata discovery. No global mutable-path cache or cleanup authority added |
| vLLM/custom service boundary | Retain audit's sampled exact-custom-version limit; no runtime fairness/performance claim or service upgrade. Actual embedding performance is external M03/M04 |

PD01 is an additional completeness requirement discovered during planning, assigned to C02 rather than
a new standing rule. PD02 became an evidenced source/native defect during the decision investigation;
its selected correction and future acceptance belong to S09/D10. The remaining breadth
is not silently declared sound: investigate a newly exposed concrete dependency within its owning
package, without turning functional work into an unrequested exhaustive review.

## 7. Migration, acceptance and evidence

### G00 and coordinated cutover

Publish affected architectural meaning with the implementation slice. The approved direction is
available now; implemented contracts and consumer migrations are not. Group coupled decisions under
their actual owners, supersede accepted ADRs where needed and retain links back to these findings.
Avoid a new central rule register or runtime framework.

For format/identity changes, enumerate writer, reader, backup/export/import, recovery, validation,
serving and retirement consumers in the same package. Implement the actual supported transition under
the existing upgrade owner, or an explicit quiesced project-store rebuild from pinned inputs. A format
constant bump is insufficient. Do not add old-format compatibility readers, keep obsolete generations
without a current consumer, or retire a still-pinned realization. Within a current format, genuinely
live named consumers can hold their exact derivation until released. Cross-format cutover follows the
existing design-phase policy; interrupted work keeps its recovery owner and truthful disposition.

No authoring action touches a database. Future implementation can use non-disruptive logical validation
scopes under the existing fixture owner when authorized. Disruptive maintenance, actual project-store
rebuilds, operator adoption and deletion require their own concrete execution authority; they are not
inferred from a successful test or a proposed migration design. X01 cannot claim a migration complete
if necessary authorized operational steps remain undone; record that boundary explicitly.

### Local and integrated functional evidence

Each companion names revealing success/refusal/failure cases. During implementation use compile checks
for touched crates and focused affected controls through existing `just verify --select ...`/bare-tool
owners, with actual fixture ownership where persistence matters. Exact selections come from the then-
current control inventory; do not encode stale test names or create a second runner in this plan.
Pure finite controls need no database. Fake delays can show scheduling, but cannot prove persistence,
remote drainage or independent model fidelity. Do not multiply identical reconstruction in production
and tests and call it independent evidence.

The coordinator owns **X01**, one integrated functional acceptance after working consumer migrations.
It must cover the boundaries below with actual changed components, reusing focused evidence where valid
and rerunning only changed/failed/uncertain controls. Package completion does not automatically trigger
a whole-product campaign. Apply the repository's execution policy for shared-contract acceptance when
implementation occurs; repo-level hygiene is not part of this plan-authoring task.

| Boundary | Revealing evidence required for functional completion |
|---|---|
| Native write/completion/recovery | Late collision, retry/unknown acknowledgement, complete state families, blocked late mutation, retirement races, interrupted root transfer and actual destination admission |
| Reuse/identity/coverage | Same-input provider hit, changed metadata/context miss, exact receipt versus value cutoff, complete/empty/NotRequested/partial scopes, held by-reference recovery and sparse exact keys |
| Analysis/synthesis | Independent finite meaning/refusal cases, deterministic Structural/summary proof alternatives, exact analytics winners/communities, complete synthesis decisions and embedding refusal/drain |
| Publication/serving | Retained manifest under current derivations, exact numerical reference, current-context/negative invalidation, page and cursor semantics, off-page failure behavior and final-byte refusal |
| Lifetimes and transitions | Actual native cancellation/drain or explicit recovery, CLI and direct-test entrypoints, selected backup/restore, old-consumer release, preserved shared objects and truthful cleanup |
| Development loop | Actual dependency/fingerprint boundaries, grouped fixture controls, this track's ordinary-profile qualification, exact target reruns, original completion observations, local lifecycle and metadata status |

Record each outcome as passed, failed, blocked (with prerequisite) or not_run and link the command/receipt,
source baseline, date and scope. Keep Implemented, Interface-checked, Tested and Measured distinct.
Closure cannot be inferred from an accepted ADR, a package's code presence, or a controlled mock.
Functional tests establish changed behavior; they do not establish comprehensive prior-system quality.
If a fidelity or lifetime question remains material to the change, resolve it before closing that change.

The track can finish with explicit accepted limits: necessary exact pair work under the current analytics
method, process-local resource envelopes, no new global daemon and no measured benefit claim. It cannot
finish by silently recategorizing an unimplemented accepted contract as a limitation. O-F41 remains a
known external policy/measurement item; O-F33's exact-work residual remains explicit even when its
preparation/bounded-kernel package completes. Neither requires an unapproved approximation or cap.

## 8. External follow-ups — not functional exit gates

These remain outside the approved functional-completion track. They have named purposes and triggers;
none is reported passed or converted into a quantitative benefit claim by static inspection. A future
campaign needs its own authorized workload/service envelope and must preserve concurrent work.

| ID / owner | Purpose and trigger | Source impact / present disposition |
|---|---|---|
| M01 — resource/runtime and development owners | After repeated-work repairs, attribute representative concurrent failures/slowdowns to retry conflicts, CPU, memory and test duplication under matched conditions; observe actual peak use, not reservation totals | O-F01/O-F41/O-F45, original R12; RC12 retained, no thread weights/caps or host admission policy changed here |
| M02 — native/serving owners | After fused effects/selective access, measure actual window/query behavior and distributions; obtain EXPLAIN/plan evidence where needed, including sparse/dense shared cohorts and cold backup | O-F03/O-F38/U01/U17; no per-row-query or latency claim from lexical search alone |
| M03 — compiler/product/build owners | Separately authorized real-library cold/warm/edit/profile campaigns: row volumes, phase splits, peak live state, compile/link and cache costs, provider determinism at representative scale | O-F16/O-F20/O-F26/O-F33/O-F46/O-F47, U02–U04/U13/U14; no FastMCP/pilot run or measured speedup in this track |
| M04 — product/analytics/serving owners | Actual large exact analytics or multi-client workload exposes unacceptable remaining work/latency; compare explicit method/deployment changes and transport/embedding behavior | O-F33/O-F36/O-F39, U06/U07/U09; new approximation, global scheduler/daemon or method policy needs a concrete design decision |

Existing historical probes remain useful clues with their original samples, nested phases and load
limits. They do not form a matched baseline. No numerical cost model, generic instrumentation scheme
or obligatory benchmark matrix is introduced; collect only evidence that answers the concrete question.

## 9. Functional closeout and later existing-plan resumption

**Current checkpoint, 2026-10-11 (receipts through 2026-10-12 UTC):** implementation remains
in progress at §4. The dedicated schema5 campaign retains its authentic original producer,
populated protected cold checkpoint and completed exact joint host recovery; original timed-out
native outcomes remain unknown. Disk capacity no longer blocks compilation. Model, compiler
metadata, native query/audit, isolated SDK and four core cache-window controls passed within their
stated scope. Four-crate test-target compilation and the nine-control dedicated wire run passed;
subsequent migration edits have exposed and repaired compile defects; fresh compilation and native acceptance remain required. Full physical/semantic
admission and selected attachment are integrated in source, with no integrated acceptance yet.
The typed source5→6 converter, genuine native acceptance, C05/C06 consumers and remaining package
contracts still precede X01. Warm retention now requires explicit product release rather than
unadmitted attempt cleanup. No format6 activation has occurred. Preserve the original receipts,
warm products, recovery assets and shared caches. No source finding is closed by these prerequisites.
Real-library/performance and repository quality campaigns remain not_run; shared paused service,
operator databases and all old-plan qualification state remain unchanged.

At X01 completion, this sole disposition table must distinguish delivered contracts, verified scope,
accepted residual limits and external follow-ups. Preserve source reviews/evidence while they have
current obligations. Move enduring design into its established owners as slices land; do not replace
architecture with a growing progress log.

**X02 handoff** follows full functional implementation of this track. Produce a concise handoff here
listing changed architectural owners/ADRs, schemas and required operational transitions, available
interfaces, exact acceptance receipts, retained limits and M01–M04. Explain which contracts an existing
plan must rebase its assumptions against. Do not pre-emptively mark any old finding closed or create a
cross-plan status mirror. The operator then resumes the existing plans and commissions their review/
updates as needed. This track remains separate and does not supersede them.
