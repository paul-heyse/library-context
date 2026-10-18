# Catalog execution speed: coordinating plan

**Implementation plan authored2026-10-18; production implementation not started.** The operator
approved the authoring approach and each RC01–RC09 direction on2026-10-18. Approval is a design
direction, not implementation or verification. All replacement capabilities below remain **Proposed**.

## 1. Outcome, basis and boundary

The target removes unnecessary remote stage handoff, repeated rich preparation, broad exclusion
and fine-grained lifecycle work while retaining the complete pinned API/evidence product. Private
compilation uses immutable typed/spillable completed sources. Native materialization occurs at
requested Facts, Normalized, Analysis and Catalog outputs and at justified independent retained
products. SurrealDB remains the durable canonical graph/publication/reader realization.

Authoring baseline:`9c457a0c3c0bf50b480f8d5c2a471ffac54cf115`. The source review inspected
`b0dd321e`; the intervening commit adds that review/evidence/status only, with no production delta.
The two supplied external reviews were untracked operator inputs at authoring and are unchanged.
The [principal review](../design_review/reviews/design_review_catalog-execution-speed-comparative-target_2026-10-18.md)
owns dated findings/evidence; its [76-group comparison](../design_review/evidence/2026-10-18_catalog-execution-speed-comparative/recommendations.md)
retains original recommendations, corrections and supporting details from both external inputs.
This plan's §6 alone owns current disposition for the transferred F01–F13/I01–I05 obligations.

Core/template3.3, heuristics1.0 and code-intelligence1.5 apply through the
[standard](../design_review/design_principles/standard.toml). The functional owners remain the
[semantic model](../design/sections/semantic-model.md), [API/evidence product](../design/sections/api-and-evidence-product.md),
[extraction](../design/sections/acquisition-and-extraction.md) and
[storage/publication](../design/sections/storage-and-publication.md). Approved changes may replace
incidental current rules; semantic fidelity and honest effects remain required.

This is a **new standalone track**. No existing plan is edited, retired, superseded or declared
complete. Earlier unresolved native/publication controls are relevant baseline limits, not a
backlog automatically imported in full. New consumers receive the checks needed for this target;
old-plan reconciliation remains later work. No operator adoption is implied.

Workload premise: one operator on this machine, regenerable pinned inputs, many small/empty fact
families, shared dependencies, overlapping public roots, sparse public selection beside large
private corpora, high-degree/cyclic graphs, optional techniques and concurrent independent work.
Preserve currently supported captured fact families, provider fidelity, unknown/NotRequested,
relationship identity and exact same-generation evidence. Useful output limits do not bound all
examined work. No30s goal, fixed savings estimate or synthetic per-row coefficient is an acceptance gate.

| Document | Owned design and implementation packages |
|---|---|
| This coordinator | SP00 approved decision/consumer slices; SP90 functional acceptance; SP95 later qualification/benefit; SP99 cutover/handoff and sole disposition |
| [Compiler](catalog-execution-speed-compiler-plan_2026-10-18.md) | SC00–SC04: private completed sources/spill, actual producer/consumer migration, demand/properties, coherent visibility/scheduling and per-key preparation |
| [Native](catalog-execution-speed-native-plan_2026-10-18.md) | SN00–SN04: assurance boundary, bounded packets/hazards, native persistence/receiving, ordinary publication/frontiers and owned regeneration |
| [Analysis](catalog-execution-speed-analysis-plan_2026-10-18.md) | AA00–AA04: contextual/reuse decisions, graph handoff, selective closure, borrowed roots and justified retained products/kernel work |
| [Extraction](catalog-execution-speed-extraction-plan_2026-10-18.md) | AX00–AX03: provider foundations, correspondence/dispatch, export/type semantics and protected capture/completion |
| [Diagnostics](catalog-execution-speed-diagnostics-plan_2026-10-18.md) | AD00–AD04: typed observations, actual native success/stats, schedule/handoffs, optional timeline/reporting and bounded diagnostic work |

## 2. Focused foundation assessment and combined target

The completed source review and independent assessment settle the broad direction; authoring adds
focused contract inspection, not another formal review. Existing model semantics, dependency-ready
scheduler, typed RowsView, scoped assurance, native set readers/operation pages, immutable blocks,
publication pins and run/evidence ownership are useful foundations. They are not all reusable by
merely substituting Arrow for a native handle.

Current `CompletedRelation` owns a NativeCompilerStore, CompletedView, bridge and execution
lifetime (`cpg-core/src/workspace.rs:1881–1903`); input resolution earns native providers.
Current native `ExecutedCheck` retains installed views and actual operation identity. These are
**native capabilities**, not generic immutable-data tokens. The model already supplies a pure opaque
`CheckedValidationPremise` (`lctx-model/src/domain/checked_admission.rs`), explicitly granting no
persistence/selected-view authority. Reuse that semantic owner and split native assurance wrappers
rather than introduce a second certificate framework or relabel native capabilities as private ones.

Current CLI setup allocates a remote native attempt before acquisition (`lctx/src/compile.rs:48–93`,
`compiler.rs:4310–4352`). Private compilation needs its own owned identity/resource/cancellation
scope; creating it must no longer require that native constructor. Native target allocation occurs
when required materialization or a selected durable product actually starts. Early readiness can
observe prerequisites without minting native eligibility. Real cache/import/native reads retain
their independent current session, pin, recovery and outcome ownership.

### Shared contracts and owning boundaries

These labels describe behavior, not a requirement for one new type/service per row.

| Contract / owner | Required meaning and migration consequence |
|---|---|
| Private completed source / compiler, SC00 | Immutable typed rows/batches or indexed spill; exact model/schema, producer/profile/configuration, ordered input roles/epochs/dependencies, output domain and outcome. A source owns all backing and admitted charges through its last borrower. Pending/refused output is not completed input. |
| Typed input selection / model + compiler, SC00/SC01 | Consumers resolve exact roles/epochs and full declared universes independently of physical native views. Borrowed selected views preserve empty versus absent and coalescing versus conflicting duplicates. No generic latest/name-only fallback. |
| Earned semantic property / model + compiler, SC02 | Actual successful owned execution over exact premises/EOF earns opaque private evidence; failure/Unknown/partial acquisition publishes none. Serialization or matching digest cannot recreate that authority. Native storage eligibility stays separate. |
| Requested-frontier lowering / native, SN00/SN02 | A sealed complete private source is translated once into actual attempt-owned native content/mappings with bounded staging. Existing semantic properties can travel only under a qualified preservation map; actual installed content and receiving establish their own assurance. |
| Coherent output visibility / compiler + native, SC03/SN01 | Prepare complete output sets outside exclusion; expose no partial producer. Predecessor changes rebase/refuse appropriately. Native page order, replay/refusal/Unknown and final manifest commit retain their actual authorities. |
| Capture validity / extraction, AX03 | Exact membership/bytes remain immutable or read-verified under the capture owner through provider use. Protect against same-user path mutation; a permission bit alone is not assurance. |
| Reuse / analysis + compiler, AA00/AA04 | Retain only named useful products with complete semantic/value keys, explicit population policy and actual native ownership. Private compute need not become a durable stage cache. Accepted effects remain joined regardless of optional intent. |
| Diagnostic projection / AD00 | Observations carry actual identity, timing presence, relevant work and true terminals. Observation completion/drop never grants semantic validity, cleanup or remote finality. |

The normal route is capture→private fact output→private normalized/upper owners→requested-frontier
freeze/semantic completion→bounded native materialization/reconciliation→admitted native artifact→
ordinary publication or explicit export→joined finalization. Pure semantic checks run before their
premises are relied upon; final native validation is not replaced by a producer-generated hash.
Independent import/restore enters through fresh incoming admission, never private authority recovery.

Compare three complete alternatives: improved persistence throughout; private typed/spillable
execution with only requested-frontier outputs; hybrid with selected justified products. Choose
hybrid while AA00 inventories genuine reuse consumers. If no product warrants its cost, use the
simpler no-intermediate-product route for that scope. Both private routes preserve native final
meaning. No universal backend framework, all-RAM requirement, giant transaction or compatibility
runtime is part of this target.

## 3. Approved rule directions and decision routes

**Each row explicitly accepted by operator2026-10-18.** The approval came from the nine-item
RC01–RC09 disposition question, not inferred from approval of a review. These are approved target
changes; actual architectural/source migrations and verification remain not started. SP00 prepares
the relevant ADR/design-owner slices before dependent code. Accepted ADRs remain immutable except
lifecycle metadata; preserve unaffected semantics when superseding an affected decision.

| Review impact | Approved change | Decision/architecture route and prerequisite slice |
|---|---|---|
| RC01 | Compiler-private typed/spillable completion; native authority at requested outputs | New/superseding decision for affected ADR-0138 §B2/§B7/§B12 and semantic §15; before SC00/SC01 authority migration |
| RC02 | Private compute may rerun; durable recovery covers real native effects/independent consumers | Scope affected ADR-0138/0178/0190 and storage §6.1–6.2; before SN00 and private/native lifetime integration |
| RC03 | Bounded whole-operation packets and grouped control work under complete hazard coverage | Native realization decision/schema owner, affected ADR-0190; before SN01/SN04 format consumers |
| RC04 | Transport earned properties only under exact preserved premises; independent receiving remains | Assurance decision/validation owner, affected ADR-0186/0187; before SC02/SN00 evidence migration |
| RC05 | Demand-specific declarations before binding/preparation, same facts/domains/config/coverage | Model/profile owner, affected ADR-0160/0186; before SC02/AX02 demand changes |
| RC06 | Justified retained products/population timing; decline before effects and join accepted work | Reuse/lifetime decision, affected ADR-0190; before AA04/SC01 optional native integration |
| RC07 | Equivalent frontier contextual execution alongside justified bulk preparation | Scope-program/model decision and selected execution contract; before AA02 strategy migration |
| RC08 | Carry genuinely protected capture validity across providers | Acquisition/extraction owner and affected ADR-0182/0184; before AX03 removing repeated checks |
| RC09 | Extend typed observation/statistic/terminal vocabulary without product authority | Evidence/diagnostic owner, affected ADR-0147/0148; before AD00/AD01 protocol consumer changes |

No additional dependency version or telemetry service is selected by this approval. Existing pinned
libraries are evaluated at their actual workspace versions/features. Routine local refactoring
needs no ADR solely for movement; newly discovered rule changes require their own disposition.

## 4. Work packages and actual prerequisite order

### SP00 — Settle decision slices, durable consumers and common contracts

**Consumes:** approved directions, current code and source-review obligations. **Delivers:** the
smallest coherent authority/lifetime/assurance/demand/observation contracts and their decision routes,
with an explicit consumer table distinguishing current compilation, requested artifact, incoming
transport, cross-attempt product and pinned serving. Include all current construction/entry points,
not just `lctx compile fastmcp`, and actual optional cache/external-effect paths.

Record source identity versus native physical mapping, private evidence versus native admission,
complete producer visibility, capture integrity and real failure/recovery scope. Assign each meaning
to its existing owner. Specify any deliberate logical/wire identity changes before migration;
regeneration is allowed, silent reinterpretation is not. A private source cannot masquerade as a
CompletedView or imported native owner. Derive actual type/API changes only after capability fit
is understood. SP00 is delivered in independently usable decision slices; all documents need not
wait for one giant architecture batch.

**Closure:** source-backed consumer/entrypoint coverage and a coherent route for all four frontiers,
cache hit/miss/import, missing source/property and cancellation. No new production test-only API,
export fixture or broad gate is introduced. A selected contract is not yet an implemented prerequisite.

| Package | Delivered capability | Required prerequisite, not whole-document barrier |
|---|---|---|
| SC00 | Private completed-source/working-set/spill owner | SP00 private identity, validity/lifetime and RC01/02 slices |
| SC01 | Actual extraction/normalization/upper/CLI consumers migrated to private handoff | Working SC00 portions; SN00 boundary contract; actual SN02/SN03 integration before switching public compile |
| SC02 | Early demand and qualified semantic-property preparation/reuse | SP00 RC04/05; working exact-source inputs; native qualification needs SN00 |
| SC03 | Atomic output-set visibility and dependency-ready launch preparation | SC00 exact predecessor/outcome contracts; SN01 only for actual native binding sets |
| SC04 | Exact-key pending preparation without I/O under global registry lock | Existing protected-source contract; adapt to SC00 when migrated |
| SN00 | Private semantic evidence versus actual native/incoming admission | SP00 RC01/02/04 and current opaque model premise owner |
| SN01 | Coarse bounded native packets, exact set preparation and real hazard exclusion | SP00 RC03/physical-contract slice; does not depend on all private migration |
| SN02 | Requested-frontier native materialization and independent receiving | Working SN00/SN01 plus implemented SC00 source reader |
| SN03 | Ordinary publication/export/reader integration for every supported frontier | SN02 admitted native artifact, current publisher pins/definitions/commit contract |
| SN04 | Current target regeneration and obsolete physical/protocol retirement | Affected SN02/SN03 consumers and focused controls, exact service owner drainage |
| AA00 | I03/I04 strategy/product decisions | Current source + SP00 consumer/demand slices; no full-library measurement prerequisite |
| AA01 | Exact charged graph materialization handoff | SC00 source ownership; normalization producer/upper consumer migration |
| AA02 | Equivalent selective contextual closure with dense bulk alternative | AA00 I03 decision + RC07; exact typed selected inputs |
| AA03 | Borrowed root partitions and early qualified cache lookup | AA00 value contract + working exact membership; SC00 for new backing |
| AA04 | Justified retained products and independent pure-root work | AA00 product decision, SC01/SC03 lifetimes and SN02 for accepted native retention |
| AX00 | Provider/capture/export contract and same-fact migration inventory | Current exact locked analyzer contracts; SP00 before selecting changed integrity/durability policy |
| AX01 | Module correspondence and typed writer dispatch | AX00 observations/ordering; no need to wait for all native packets |
| AX02 | Independent export/provider completion with preserved type-work semantics | AX00 sharing/ordering; existing safe sink can support local changes, working SC01 sink before private migration; RC05 if demand changes |
| AX03 | Capture validity carried through actual provider use | AX00/RC08 and implemented read protection; integrate SC00 lifetime when migrated; preserve current checks until protection works |
| AD00 | Minimal typed identity/terminal/statistics observation contract | SP00 RC09 slice; independent of full compiler pivot |
| AD01 | Actual native success/retry/read statistics and adapter consumers | AD00; observe current paths first, migrate with SN01/SN02 |
| AD02 | Actual schedule/handoff/work observations | AD00; SC03/AA/AX owners provide final actual transitions |
| AD03 | Optional timeline and compact factual report | AD00 plus actual AD01/AD02 observations; no mandatory collector |
| AD04 | Cheap disabled diagnostics and justified optional capture/profiling hooks | AD00 dispatcher/loss contract; I05 decides optional integration depth |
| SP90 | Focused assembled functional correctness and applicable hygiene | Actual migrated producers/consumers and required local controls |
| SP95 | Later selected full qualification/FastMCP benefit questions | Functional target delivered; separately selected runtime scope and ownership |
| SP99 | Accurate functional handoff and retired obsolete mechanisms | SP90, applicable SN04; explicit SP95 outcomes/deferrals, not invented whole-plan acceptance |

Ready branches may proceed independently. SC01/SN02 exchange early contracts, not a circular demand
that both be fully implemented first. An actual end-to-end compile switches only after its private
producer chain and native requested-boundary consumer work together. Do not leave a partly migrated
ordinary path in service or maintain two final canonical authorities as a permanent fallback.

Shared edit surfaces (`workspace`, `compilation`, compiler inputs/admission, native packets and
reader/event adapters) need one integrating owner. File contention is not a semantic dependency;
use coordinated commits or a worktree for independent conflicting revisions, not blanket serialization.

## 5. Verification, cutover and completion

### SP90 — Focused functional implementation endpoint

During implementation run touched-crate compile checks and minimal revealing controls with the
currently qualified profile, normal available parallelism and existing fixtures/owners. At functional
completion run full workspace compile and strict keep-going Clippy, resolving all warnings/errors,
plus applicable Python Ruff/Pyrefly, dependency/schema/ADR/docs leaves for changed surfaces. Do not
introduce job/thread caps, reset lifecycle state, broaden timeouts or clean warm caches to pass.

Focused combined scenarios include all requested frontiers/profiles; same attributed facts and
unknown domains; captured/derived source identity; overlapping roots and contextual negative joins;
empty semantic families without physical expansion; spilling/backing last-borrower lifetime; stale
predecessor/disjoint completion; failed/Unknown property reuse; actual native collision/collection;
independent receiving; fresh definitions/roles/search; commit-before-acknowledgement and real drain.
Tests challenge model meaning independently where a shared producer/replay can agree incorrectly.

Publication controls use genuine completed admissions and actual mutation/restoration. Failed seals
are one-shot; restoring rows does not reset eligibility. Full export/import retains its product
purpose and dedicated receiving tests, never a fixture shortcut for ordinary publication.

### SP95 — Explicit later qualification and benefit scope

**Deferred until separately selected after functional delivery.** Retain I01: an equivalent genuine
FastMCP run must identify roots/dependencies/docs/examples, selected frontier/profile/methods,
embedder/cache dimensions, executable/schema and whole-command completion. Full qualification,
recursive/publication journeys and benefit/overhead comparisons use existing verification/run owners;
no new benchmark framework or minimum repeat count is implied. Any required shared-contract assembled
acceptance follows repository policy at its final appropriate boundary, not after every package.

No20–35s forecast, 3ms/row scaling coefficient,95% attribution target,3% overhead target or fixed
transactions-per-stage threshold is accepted. Explain what work is removed and what remains unmeasured.
Embedding inference, submitted MCP runtime and operator adoption remain outside this series unless
separately selected. Environmental/provenance campaigns disproportionate to design-phase correctness
are excluded. SP95 not_run prevents an end-to-end qualification/Measured claim, not functional progress.

### SP99 — Regeneration, deletion and honest handoff

Complete native cutover through SN04 after actual target readers/writers pass. Rebuild project-owned
stores/projections from pinned inputs and remove obsolete generations/compatibility-only paths under
existing owners. No data preservation, old-format reader or historical benchmark is required solely
for this design phase. Preserve named current/cited/Unknown owners and operator stores; authenticated
whole-target discard is distinct from a successful logical cleanup record.

End the functional slice only after every required consumer has migrated and accepted effects are
joined. Record implementation, focused evidence, unresolved uncertainty and later SP95 boundary
separately. Full-series completion is not claimed while selected required acceptance remains open.
Update durable architecture/decision owners when actual meaning changes; later reconciliation of
older plan documents is separate. No generic final inventory phase may defer an actual consumer
migration required to make an earlier package correct.

## 6. Sole disposition and current checkpoint

This section alone owns current execution state for this source review's scheduled obligations.
Companions own design/package detail; the review remains dated evidence. All statuses below are
**Open / planned, not implemented**, except explicit conditional deferrals and source exclusions.

| Source finding | Responsible packages | Closure obligation |
|---|---|---|
| F01 | SP00, SC00/SC01, AA01, SN00/SN02/SN03 | Private hot handoff; every requested frontier; exact backing/evidence and actual native/incoming authority |
| F02 | SN01/SN02/SN04, SC03 | Bounded whole packet/control effects; original-block set reads; full hazards/identity/receiving/retirement |
| F03 | SC02, SN00/SN02 | Reuse earned semantic properties only across preserved premises; genuine physical/cold checks |
| F04 | SC02, AX00/AX02 | Demand before preparation; all supported facts, config/domains/NotRequested and authentic availability |
| F05 | SC03, SN01 | Coherent set visibility; private preparation; true dependencies and correct overlapping predecessors |
| F06 | AA00/AA04, SC01 | Justified cache/population with early keys and accepted-child/native finality |
| F07 | AX00–AX02, AA04 | Exact correspondence/type/export semantics and independent work; same provider facts |
| F08 | AA00/AA02 | Exact frontier contextual route or justified bulk, sparse/dense/negative/degree semantics |
| F09 | AA03/AA04 | Borrowed shared rich partitions; neutral value keys before unnecessary copy/hash/hydration |
| F10 | AX00/AX03 | Genuine capture validity protection without repeated whole-corpus provider scans |
| F11 | SC04 | Exact-key owned initialization outside global I/O lock; no failure/Unknown replay |
| F12 | AD00–AD03 | Actual native success/stats and truthful structured identity/terminal consumers |
| F13 | AD04 | Dispatcher-correct disabled path; bounded structured capture and no fabricated durability/CPU |

| Investigation | Current disposition / owner | Decision product and constraint |
|---|---|---|
| I01 FastMCP distribution/benefit | Deferred / SP95 | Equivalent later requested operation and accurate milestone/workload comparison; blocks measured speed claims only |
| I02 conflicts/realized blocking | Open / SN01 + AD01/AD02 | Identify actual protected conflict keys/causal resource where needed; remove structural hotspots without assuming mutual livelock or subtracting overlapping totals |
| I03 contextual strategy | Open / AA00→AA02 | Exact selective versus bulk consumer contract and revealing sparse/dense cases; constrains route, never universe semantics |
| I04 products/materialization | Open / SP00 + AA00→AA04/SC00 | Named useful durable consumers and retained-buffer/spill/value economics; no mandatory intermediate crash-resume absent consumer |
| I05 diagnostics depth | Open / AD00/AD04 | Minimal useful factual trace/report selected; deeper server/CPU/what-if extensions conditional on a concrete residual question |

### Recommendation-group execution disposition

IDs retain the [source crosswalk](../design_review/evidence/2026-10-18_catalog-execution-speed-comparative/recommendations.md)
meaning. A corrected/excluded quantitative claim is not an implementation task. Conditional
extensions have the trigger below; none silently becomes a mandatory platform or campaign.

| Source groups | Planned route / disposition |
|---|---|
| E1-01 | AX00/AX02 + AD02: distinguish analyzer/export/queue/completion |
| E1-02 | SP00/SC02/SN03: actual profiles/methods/inputs and every frontier |
| E1-03 | SP00 foundation preservation; implement remaining defects, not generic rewrites |
| E1-04, E1-05 | SN01/SN02: bounded graph packets, exact set correspondence/read paths |
| E1-06 | AD01/AD02: selective actual physical/work observations |
| E1-07, E1-08 | SC02/AX02: early demand with same supported captured facts; reduction excluded |
| E1-09 | SC02/SN00 + AD01/AD02: property-specific shared acquisition and truthful metrics |
| E1-10, E1-11, E1-12 | AA00/AA03/AA04: consumer-led cache policy, early multi-lookup, useful window retention |
| E1-13, E1-14 | AA01/SC00: graph handoff; projection meanings retained |
| E1-15, E1-16, E1-17 | AX01/AX02/AA04: correspondence, exact type-work and independent pure work |
| E1-18 | AX03: protected capture validity, no silent integrity-check removal |
| E1-19 | AD00–AD04: diagnostics subrecommendations D01–D26 below |
| E1-20 | SP95 deferred equivalent workload; initial SP00 truthful baseline, no mandatory pilot |
| E1-21, E1-22 | AD00/AD01 alongside functional packages; hypothesis-specific investigations, no diagnostics-first whole barrier |
| E1-23, E1-24 | SP95 deferred comparisons; unsupported forecasts/p95/gates excluded |
| E1-25 | SP00 contracts early, AD01 and SC02 parallel; no complete telemetry prerequisite |
| E2-01, E2-08, E2-24 | SP00/SC00/SC01/SN00/SN02: selected hybrid and deliberate decision/deletion |
| E2-02, E2-09, E2-14 | SN01/SN02: current operation pages and complete canonical output, not one giant transaction |
| E2-03, E2-17, E2-18 | SC03/AX02: independent completion/ingress only under explicit order/backpressure; authentic descriptors |
| E2-04 | SN01/AD01/AD02 I02: contention real, keys/livelock/all-write concurrency unproved |
| E2-05, E2-16 | SC03/SC04: actual set visibility/per-key preparation; no partial-producer locks |
| E2-06, E2-15 | SC02/SN00: preserve existing qualified reuse; no blanket verify-once or bare digest authority |
| E2-07, E2-11 | AA00/AA04: correct async/foreground distinction and justified population |
| E2-10, E2-20 | Excluded as forecasts/necessity claims; I01/SP95 owns later measurements |
| E2-12, E2-13 | SN01: exact hazard/correspondence/retry semantics; unconditional guard removal excluded |
| E2-19 | AX00/AA03/AD04: do not infer cheap CPU/hash/planning from incomplete fixture logs |
| E2-21 | SP95 deferred genuine FastMCP; no benchmark-before-functional barrier |
| E2-22, E2-23 | Numerical gates excluded; valid structural directions mapped to SN01/SC02/SC03/AA04 |
| E2-25 | AD00–AD04 and conditional I05; not one indivisible diagnostics system |
| D01, D02, D03 | AD00/AD01: actual success, typed page/retry/read identity and timing presence |
| D04, D05, D06 | AD00/AD02: scoped context, actual lifecycle/schedule/handoff and aggregate work |
| D07, D08, D09 | AD00/AD02/AD03: phase event pairing, async overlap, genuine causality; CPU inference excluded |
| D10, D11 | AD03/AD04: library timeline/export; bounded optional recorder with truthful loss/error |
| D12, D13 | AD04 conditional: actual server-correlation question; no mandatory collector/fixed scrape or slow-log cadence |
| D14 | AD00/AD01 report semantics: unexplained client remainder remains unknown |
| D15, D16 | AX00/AD02 conditional native telemetry/snapshot; timing-mode baseline equivalence excluded |
| D17, D18 | AD02: exact executed-plan metrics; bridge only if native metrics leave a concrete gap |
| D19, D20, D21 | AD04 conditional application/server CPU or scheduler investigation; no environmental campaign |
| D22 | AD03 compact factual timeline/report; critical-path claims only with actual recorded edges |
| D23 | Deferred conditional AD04/I05: unresolved prioritization decision requiring a sensitivity tool; otherwise omit simulator |
| D24, D25 | SP95 deferred selected equivalent experiments/overhead; no generic bench command or lower-bound extrapolation |
| D26 | Arbitrary thresholds omitted; AD00/AD01 run alongside source-established functional changes |

### Authoring checkpoint,2026-10-18

Approved target directions and this six-document implementation proposal are delivered; no
production package is implemented by plan authoring. Focused foundation advice and source inspection
were used; no product tests/builds, database operations or service changes were performed. Initial
implementation entry is SP00's private/native authority and consumer slices, with AA00, AX00 and
AD00 independently ready for their bounded design work. Whole-doc preparation is not a runtime barrier.

**Authoring checks,2026-10-18:** scoped source-document check `20261018T064251.082Z-a9167a`
passed six new plans/eight owned Markdown documents,66 local targets, source/explicit fragments,
all76 unique recommendation dispositions,13 findings,5 investigations,9 approved rule rows and28
defined work packages. This is source consistency, not rendered HTML or functional acceptance.
`just docs-check` (`20261018T064206.899Z-317070`) failed on the untouched supplied
`docs/external_review_execution_speed_and_diagnostics.md` missing title/H1; its preceding ADR lint
and agent lint passed. Both managed commands have confirmed host cleanup; their logs/receipts are
retained for `citation:catalog-execution-speed-plans-2026-10-18`. Product builds/tests and runtime
operations are not_run for this documentation-only scope. Independent bounded document/contract
assessment found no integration blockers. No old-plan verification claim is promoted. Current default
schema9/protected Unknowns and prior publication acceptance stay under their existing owners until
the actual future service-owned cutover.
