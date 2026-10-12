# Reconciled efficiency alignment — implementation plan

**Authorized implementation track / paused at operator-requested closeout, 2026-10-12.** The operator approved creation of this new plan series
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
| [RC12](../design_review/reviews/design_review_efficiency-alignment-codebase_2026-10-11.md#rc12) | **Retain** current concurrency/resource rules; causal evidence required for a later policy change | No caps/weights/default change in this track; M01 follow-up, AGENTS/ADR-0162 remain authoritative |
| [RC13](../design_review/reviews/design_review_efficiency-alignment-codebase_2026-10-11.md#rc13) | **Retain** shared store and logical isolation; no per-attempt databases | Native/fixture owners preserve current rule; no new isolation service |
| [RC14](../design_review/reviews/design_review_efficiency-alignment-codebase_2026-10-11.md#rc14) | **Change:** this track qualifies one shared ordinary dev/test profile; release acceptance remains distinct | D03; Cargo/verify/rerun/README/AGENTS and ADR-0162 profile decision; no old BC3 dependency |
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

The original-revision upper fixture was cancelled through `just runs cancel` at07:32:57UTC,
exit143; process cleanup confirmed with no survivors. Its exact original run identity and outcome
are embedded in the retained local attachment `fcad900d-aa6b-4e98-aed0-56154d4216dc` disposition. It
produced no accepted upper receipt. Its native outcome remains Unknown. The explicitly owned
campaign discard/regeneration subsequently completed (`131431…4f6e67`), initially permitting current
fixture borrowing without promoting the interrupted work to migration or semantic acceptance. The later
current-native failure is now separately owned by attachment244447dc-d09d-4e20-ac7f-9cabdacac1d7;
its named recovery initially stopped at the immutable older installer's drainage. Current-driver
main/validation drains passed (`135037…4d76c9`), then the original named recovery completed
(`135100…ecdbae`); the original Unknown remains unchanged. Both databases passed readiness and
that recovery completed. Its temporary repair driver is retired. A later13-pass/two-fixture-failure run (`140232…4a34ff`) retained attachmentaf6e8d06-492d-4bb1-b216-6dcf899b41b0; named recovery (`151841…6a7f02`) joined its daemon but old-CLI validation drain failed. Separate current-driver main/validation drains passed (`20261012T152219.471Z-e2a755`, cleanup15:22:25UTC/no survivors), then original named recovery passed (`20261012T152248.377Z-a9672b`, cleanup15:22:51UTC/no survivors; receipt SHA256fda0fb8983f4086b442805209fa87de3b4a81ee91b1a834d55d82b9e33291d90). Admission is open; original Unknown and immutable proof bytes remain unchanged. The completed one-off driver is retired; separate evidence persists under its run.
The compact completed local disposition is retained. Explicit owner retirement removed the four
exact obsolete input/producer/recovery directory lifetimes (`133451…228bb7`), including the obsolete
campaign cache. Unrelated warm caches are untouched. Released old local run logs/receipts retired through the existing storage owner
(`134455…4fd618`), after the authenticated previous pc3-scope removal's stale host registration
was reconciled through targeted unregister. The integrated source5 checkout and task-owned obsolete
helpers then retired through worktree-remove (`134511…b09f1e`); its exact own build root is released
under lifecycle grace, with no cache purge. The task's exact launcher/exporter obligations are released.
Current compact-owner model controls passed14 (`20261012T072908.486Z-64775d`, nextest
`e6f70d23-dfbd-4fc0-a75c-fac9889c85da`, exit0/cleanup confirmed07:31:06UTC):
`cargo nextest run --locked --offline --release -p lctx-model --lib -E
'test(compact_analysis_) or test(compact_structural_) or test(compact_synthesis_) or
test(compact_catalog_parent_checks_) or test(catalog_evidence_accepts_protected_compact_) or
test(catalog_selection_rechecks_compact_) or test(compact_library_facts_coverage_)'`.
Actual current producer/native/publication/serving acceptance remains open.

**Current affected execution, 2026-10-12:** compiler/analysis/serving/publication run
`20261012T155136.796Z-9d7886` failed100 (nextest5d5cfc8b-8f64-4d3c-b220-24f32733f943):
18 cases, zero passed,12 failed and6 timed out; cleanup confirmed16:01:28UTC/no survivors.
The vacancy guard rejects an equal immutable row created by another isolated attempt between
observation and effect; the corrective contract is an explicit hash-bound exact planned successor,
not refreshed retries or weakened incarnation checks. Completion timeouts have a separate, still
open cause. Source inspection found live and frozen installed-view construction issuing one
remote effect per member; N01/N04 now removes that material serial work through charged bounded
member windows with unchanged owner fences, exact incidence, EOF audit and seal. This source
finding is not proof that it uniquely caused the observed timeouts. Serving fixtures lack exact derivation/shared budget or original chunks; publisher
stream fixtures require actual installed-reader scope. These fixtures are corrected in source,
with the final rerun outcomes below. The durable premise fixture now uses canonical InputRevision and its
existing relation-owned invariant, preserving native TypeId/schema authority; its corrected control passed in`76d9cb`.
Attachment`ad992027-52bc-4b6d-a30c-64a76dd21c89` retains native Unknown. Named recovery
`20261012T160917.199Z-df08bd` joined the exact daemon then failed old-CLI validation drainage
(exit1/cleanup16:09:21UTC). Separate current-driver run`20261012T161314.162Z-9cd84c`
passed main drainage but refused seven live validation reader pins (exit1/cleanup16:13:44UTC).
No pin was released by that failed drain. Exact named-pin reconciliation subsequently passed
(`20261012T162229.360Z-73f6f3`, exit0/cleanup16:22:52UTC/no survivors): full closed-scope
inventory and provenance were frozen/rechecked, only the seven explicitly named pins released,
exact released-row/no-hold EOF readbacks passed, and both current drains acknowledged SDK session
closure. Current CLI build`20261012T161749.948Z-a5ed81` passed0/cleanup16:21:49UTC; SHA256
4f0821bc390c78a805f1514f53dbba55349cffbe36e51ab2e2088ca9c77f7605. The session-finality
repair is committed as`1615a6e7`. Original named recovery then passed
(`20261012T162301.728Z-f25799`, exit0/cleanup16:23:04UTC/no survivors; recovery receipt
SHA25631f0e36e811c13787a20524c6df9a7b20e7f8217cb7297b3a84295f6cea10d20). Original
Unknown/plan/join/native bytes remain unchanged. Both temporary drivers are retired after their
confirmed cleanup, with retirement receipt in73f6f3/current-pin-reconciliation.
ADR-0163 installer handoff source passed24 host controls (`20261012T161308.315Z-a37db2`,
exit0/cleanup16:13:11UTC/no survivors); independent implementation review and actual handoff wait
for completion of the specialized recovery. Independent review caught and corrected missing
ordinary-credential readiness;23 affected host controls passed (`161745…5f41c7`). The handoff
source and ADR are committed as`f4ff9c31`. Actual handoff passed
(`20261012T162328.790Z-0efd83`, exit0/cleanup16:23:49UTC/no survivors): unchanged main/validation
generation/schema and both actual closed markers, candidate drainage/checks, exact two-field
descriptor publication, candidate admission reopening and ordinary-credential readiness all
passed. Journal identity1a20393b0d982e9496f6d87d8bb057247ff8b56108e21b853b1cd9bee703de78
retains exact predecessor/successor bytes. The live installer now names immutable4f0821bc…;
original recovery consumers validate their embedded original proof. Exact obsolete executable
retirement still requires its owner-directed current-consumer check. Retained-serving
cleanup inspection also found missing exact unselection/normal same-job retirement continuation
and unowned retirement-session finality. N09/S01 repairs those actual consumers before retaining
and subsequently retiring the numerical fixture; one retirement pass with remaining work is not
completion. No original recovery bytes or outcome are rewritten.

Focused creation/member/durable controls first failed at compilation
(`20261012T162828.293Z-a03270`, exit101/cleanup16:31:50UTC/no survivors), before running tests:
the new frozen-member fixture mixed local lib-test native types with its compiler-store
dependency and wrapped an already shared client. The fixture now opens corresponding local
and compiler-store configurations explicitly; the corrected rerun passed7 below. Independent retirement review
also identified cancellation after selection-lock wait, actual installed-generation validation
and durable pointer-absence acknowledgement on retry; all are required before native retirement
acceptance. Missing optional maintenance-marker observation is repaired (`cf524fc0`); two focused
host controls passed (`20261012T162818.463Z-cdbde4`, exit0/cleanup16:28:19UTC/no survivors),
including refusal of malformed present data. This is host dependency observation, not native
qualification or storage-collection authority.

Corrected native/compiler-store controls passed7
(`20261012T163252.561Z-76d9cb`, nextest0f66e8d5-d036-4802-be66-c76c86ce98f0,
exit0/cleanup16:36:28UTC/no survivors): deterministic immutable planned-successor
join/collision/incarnation refusal, charged member construction, actual frozen300 and live
overlapping200/400/450 membership windows, pinned SQL parsing, canonical durable five-child
reuse/miss and exact guarded selection clearance. These are functional controls, not proof of
the earlier completion-timeout cause. The affected compiler/analysis/publication/serving rerun
and later retirement durability/generation changes remain outstanding.
Exact obsolete-installer retirement run`20261012T163209.767Z-f8fd0c` failed before maintenance
or deletion (exit1/cleanup16:32:10UTC/no survivors): three same-UID host daemons deny executable
identity observation; a privileged read also requires unavailable authentication. The old
executable remains protected pending actual process-consumer proof. No daemon is stopped and no
name-based absence inference is made. The temporary driver is retired after cleanup, retaining
source and separate retirement receipt in that run; native content and original evidence remain
unchanged. This narrow external observation blocker does not prevent independent functional work.
Immutable creation/member fusion is committed as`57114aed`; the canonical durable fixture is
committed as`1ff99967`. Affected functional rerun`20261012T163837.666Z-ac22b6` finished with
normal parallelism and unchanged deadlines. Independent retirement review repaired the
three Rust lifetime/durability/generation defects and a host selection/scratch alias: only the
producer's exact `viewer.selected.json` path is admissible before any retirement lock/effect.
Fourteen focused host controls passed (`20261012T164218.543Z-c10f33`, exit0/cleanup16:42:20UTC/no
survivors), including exact producer-obligation release while unrelated and retained obligations survive;
independent source review accepts that host boundary. Actual retained-serving retirement and the newly
added stale actual-generation refusal control remain not_run. The live ordinary default remains release.

**Operator-requested closeout, 2026-10-12:** finish immediate tasks, integrate auxiliary work and
update owners, then stop. This is a paused partial implementation checkpoint, not track completion.
Retirement source/host consumers are committed as`598fa6ff`; current fixture/grouped-observation
controls as`2a9eb193`; protected numerical replay/profile wiring as`69b8695d`; complete22-table
history inventory selection as`610f549c`. No originals correction or ordinary-default switch was made
during closeout. The draft default-switch patch is retired; D03's companion retains its exact scope.

The last auxiliary checkout,`efficiency-decisions` at`439f3d2d`, had no unique commits versus main.
All dirty probe content survived in canonical main evidence; missing failed-setup observations/log
were copied byte-for-byte and committed as`f3565170`. Exact completed citation holds were released,
then storage owners retired only objects`93524993-6cbe-40b6-9909-47501f630172` and
`fcb7f904-29d8-4152-8282-37b700d740f9`. Authenticated failed engine01 disposal retains its original
failure/cleanup and immutable receipt in run`20261011T130336.743Z-0acd2e`, SHA256
`f99720a74a55b70c3f45de689397de696dcb204d24abba2b235e98db05b3f12c`.
Clean nonforce `just worktree-remove efficiency-decisions` passed in
`20261012T165546.314Z-8c3a30`, exit0/cleanup confirmed16:55:46UTC/no survivors; its integrated branch
was removed. Only main remains. No unrelated cache, operator data or paused-session asset was removed.

Affected functional command is retained verbatim in managed run`20261012T163837.666Z-ac22b6`:
`just fixture -- env INSTA_UPDATE=no cargo nextest run --locked --offline --cargo-profile test`
over cpg-core/cpg-extract/lctx-serving/lctx-publisher/lctx-native and the explicit23-case expression.
Nextest`ce83f0f4-8349-43a9-9724-780eccb2f27a` ran23: **6 passed,3 failed,14 timed out,508 skipped**;
exit100, host cleanup confirmed16:49:33UTC/no surviving child processes. The six passes are the actual
named retirement continuation/exact roots/terminal replay control, cancellation during selection-lock
wait, durable missing-pointer retry, indexed occurrence driver, analyzer/fulltext equivalence and frozen
BM25 isolation. These do not close the surrounding publication/serving or profile packages.
Fixture attachment`9938df36-38ee-4bdc-8c7e-75d229dd4099` retains native Unknown independently of
host cleanup. Its named closeout recovery is recorded below; no original failure/outcome is rewritten.

Named recovery`20261012T165029.263Z-634eef` ran
`just service maintenance --recover-native-attachment 9938df36-38ee-4bdc-8c7e-75d229dd4099`.
It joined/replaced the exact daemon, then failed validation drainage because native readers or backup
holds remain live (exit1/host cleanup confirmed16:50:54UTC/no survivors). The dedicated installation
remains **closed in maintenance**, with that exact protected borrower; ordinary borrowing must not
resume first. Reconcile only the actual current pin/hold owners under the existing joined-invocation
proof and named recovery, then retry that same journal. No new one-off repair, broad deletion, pointer
rewrite or generic acknowledgement was attempted at closeout. The shared paused service is untouched.

Two failures at`catalog_runtime.rs:127` report `original blob audit incarnation changed`.
Source inspection identifies a legal conflicting interleaving: `complete_blob` captures general guard
revision before full EOF validation, while exact peer chunk/header replays also advance that revision.
The log does not identify which guard field changed in these actual requests. N10 must distinguish
actual chunk-set mutation from guard/hold churn, retaining exact bytes/EOF, incarnation, tombstone and
holds; insertion-only existing per-blob predicate is a proposed mechanism pending all-path inspection
and revealing equal-replay versus mutation controls. No blind retry or dropped CAS is accepted.

The member-hydration case returned[] instead of260 ordered members. Source inspection finds four
`selection::members` queries still using obsolete `compiler_view_member.view` with logical IDs, while
current installed rows use `realization`. S04 must migrate production discovery through protected exact
realization/alias membership; preserve foreign-member exclusion and ordering/accounting assertions.
Fourteen timeouts remain unresolved and are not attributed solely to either source defect. Neither
concurrency caps nor longer deadlines nor weaker assertions were introduced.

**Resumption order:** resolve exact attachment recovery/maintenance ownership first; repair N10 and
S04 with minimal revealing controls, then rerun failed/timed-out affected boundaries. Preserve the six
passing receipts where subsequent changes leave their premises intact. Build current CLI/evaluator/
replay and perform checked installer handoff before actual new retirement flags. Execute fresh retained
S01 producer→independent numerical consumer→exact retirement; only then remove ANN declaration,
retire installed DDL under explicit closed maintenance in main/validation, and rebuild the minimal current
fixture. Acquire/retain/review the complete22-table history inventory under its actual maintenance owner,
qualify that exact manifest and run the eight individually selected history controls. Finish D03's
coherent ordinary-profile switch only after affected functional qualification; X01/X02 remain open.
The obsolete installer executable remains externally blocked on actual process-identity observation,
with its existing protected owner. Repo quality/full-suite, real-library/inference/operator campaigns
remain excluded. Shared paused/operator state and warm caches remain untouched.

**Execution started, 2026-10-11, baseline `ebcf423cc16e6cacea6455500a0cdfc9795d08ac`.**
The operator approved the detailed execution sequence, one dedicated campaign installation and
all affected functional boundaries, enabled optional paths and representative integrated journeys.
Repository quality campaigns and complete ordinary-suite execution remain excluded. The shared
service and paused qualification retain their own state. The existing plans and source reviews are
unchanged. Package/finding closure still requires its complete corrective obligations.

| Implementation slice | Current evidence and remaining boundary |
|---|---|
| Current compilation and scope | **Passed2026-10-12:** current seven-crate CLI-inclusive `cargo check --locked --offline --tests -p lctx-native -p lctx-compiler-store -p cpg-core -p cpg-extract -p lctx-publisher -p lctx-serving -p lctx` passed (`20261012T104943.020Z-c2639e`, cleanup10:51:42UTC). Current `cargo build --locked --offline --release -p lctx -p lctx-eval` passed (`20261012T130911.360Z-a3dcf7`, cleanup13:13:58UTC). Both cover latest originals, analytics, indexed-ID declarations and independent cache/setup lifetimes. Earlier resolved compilation failures remain in their managed receipts. No source finding closes through compilation alone. No whole-suite, qualify, lint, formatting or docs quality campaign runs. |
| Native physical/admission/publication integration | **Source integrated / current native acceptance pending:** exact64-stripe predicates, content-keyed original blobs, bounded ingress/effects, exact completed views/six-family identity, private executed ArtifactAdmission, staging-root publication, derivation namespaces and protected durable premises are retained. Current originals have one nonoptional blob owner; no old source-piece namespace survives. The dedicated campaign was initialized and checked at current format6 in main/validation (`131431…4f6e67`). Actual15-control execution (`20261012T132647.553Z-fc65c7`, cleanup13:28:13UTC/no survivors) passed the direct SIGINT/SIGTERM control and failed14 others; composed immutable row/hold SQL lacked a separator and pinned3.3 rejects field postfix locking targets. Current SQL now terminates fragments and binds exact record targets; the prepared-stream control now supplies its actual caller budget. Pure SQL/finality controls passed3 (`133242…db2b8b`). Named recovery (`133421…d0d5d2`) joined the exact daemon but old bound CLI drainage failed. Current ordinary dev CLI/evaluator build passed (`20261012T134342.899Z-70005c`, cleanup13:49:40UTC). Separately recorded current-driver drains passed for main/validation (`20261012T135037.325Z-4d76c9`, cleanup13:50:45UTC); original named recovery then passed (`20261012T135100.721Z-ecdbae`, cleanup13:51:03UTC, receipt SHA256c46e3f3471329f5552bad6447428fb5ae236c7ad06c1d2715ac1496dd22bdcb4). Original descriptor/plan/join/native bytes and Unknown are preserved. Both databases passed readiness at that earlier checkpoint; current borrowing is blocked by the later closeout recovery below. Temporary repair source is retired; separate immutable repair evidence remains. No historical translation, opaque store, preserved producer or old-format adapter remains a delivery prerequisite. |
| Current selection and lifecycle | **Source integrated / independently reviewed2026-10-12:** canonical reads use exact realization membership and protected completed owners, including zero demand; logical-only readers refuse canonical access. Cross-session receiving checks validate actual current generation, epoch, row/guard/incarnation and holds. Review found a live cap alone did not survive source abandonment; existing reader pins now independently protect exact completion closures through reader/query drain, using separate authenticated SDK sessions. Source review also found the independent product-cache SDK session missing from workspace drain and a cache-setup failure bypassing compiler abandonment; these affected lifetime fixes are integrated and independently source-accepted; actual native closure remains pending. Setup cancellation/unconfirmed reads preserve both pins and sessions; missing invalidation acknowledgement remains explicitly Unknown. Independent source review accepted the final reader/cache/setup lifetime changes at `a599c945`; actual source-abandon/read/drain control is pending. Completed-view readers now also own independent setup sessions; two focused setup controls passed in `083704…cc9f90`. Publication readback, audit, backup and CLI consumers are migrating onto guarded connected-reader session closure, without a second unconditional invalidation after failed close. Earlier finite selection/ownership controls passed22 (`20261012T082730.123Z-cf3e21`, exit0/cleanup confirmed08:31:41UTC). By-reference attachment, backup/restore and product release/reclaim still need current native acceptance. |
| Complete binding and retained products | **Source integrated / native acceptance pending:** ADR-0157's full immutable source/boundary/configuration/view descriptors coexist with mutable attempt slots. Warm unadmitted products remain protected until explicit exact-incarnation release. Terminal candidate admission executes current semantic gates before reuse; detached bytes and completion alone grant no artifact admission. Independent current-source review found mutable binding slots were neither held nor nominated, preventing full terminal attempt compaction; same-effect slot ownership and current revealing controls are integrated. Ordinary scoped admission also lacked durable cross-process premise reuse: protected factory-declared whole-input reuse now reaches the actual candidate route, with ineligible scopes retaining executed checks. Independent source review accepted its opaque executed proof, exact native backing observer, declared-input selection, current validity and portable-witness exclusion. Consuming source close atomically unwraps cap/pin owners before acknowledged release and session invalidation; the reviewed Weak-upgrade race is corrected (product recovery SHA256 `c478e89f0ef59f60506a3a89c8dc32ec5101f6f348dabbbc53c65fc01362eb2e`). Actual release, interrupted recovery, exact dependency-hold, premise invalidation and compaction controls remain open. |
| Compact owners and four consumers | **Focused model-Tested2026-10-12:** ADR-0160, nine whole analysis owners, Assembly Flow and Structural/Synthesis/CatalogEvidence/Selection consumers are integrated. Fourteen controls passed (`20261012T072908.486Z-64775d`, nextest `e6f70d23-dfbd-4fc0-a75c-fac9889c85da`, exit0/confirmed cleanup). Empty Python input correctly retains Summary1/Analytic5 input-level subjects and Embedding0; Uniform row count remains one per captured input/context. The current-only ordinary-profile repeat passed14 (`20261012T075708.753Z-d857a4`, nextest `b06a46f7-52bb-489f-803e-6332f891ad32`, exit0/confirmed cleanup08:02:17UTC). Independent focused source review accepted the current owner/consumer path and actual fixture (SHA256 `f3eb0b9ea2a670b9d3f25f4e98db8e6105f847b6f01ee62cf6ad329fb05ffd62`); actual current producer/compiler/publication/serving controls for nonempty and explicit-empty inputs remain pending; compact declarations are physically Complete with independently admitted semantic NotRequested domains. Ordinary complete-candidate reuse is the target for eight Rows owners; Embedding retains Fresh. Generic stage outcome4 remains ineligible. Per-owner actual reuse/domain-refusal controls are strengthened; final current compact journeys and cache matrix timed out in`ac22b6`, so acceptance remains open. Earlier failed fixture receipts retain their original failures. |
| Provider preparation/value/cache | **Partial focused-Tested:** captured request/context/output eligibility, source/value/config separation, event indexes, capture/correspondence and portable cache windows are in source. Earlier33 provider/cache controls passed (`d4113494-aaa4-4bb5-804e-ec9269b47159`); four core window controls passed (`20261012T034328.650Z-597a45`, exit0/confirmed cleanup). These do not qualify later native attachment/compaction changes. Current actual provider hits/misses, bounded delivery and native quota/lease integration remain open. |
| Analysis and synthesis | **Source integrated / current integrated acceptance pending:** shared condition/frame preparation, exact adjacency, Summary/synthesis indexes, exact optional analytics and cross-root embedding batching retain their declared semantics. Current ordinary O2 finite controls passed23 (`20261012T084120.467Z-ea87cc`, exit0/cleanup confirmed08:46:02UTC), covering shared condition/graph/Summary/synthesis preparation and refusal. Current HTTP client controls passed7 contract cases (`20261012T085306.639Z-b4106b`, exit0/cleanup confirmed08:54:18UTC); two environment-dependent live tests did no external work and are not external-service qualification. The additional A04 exact community-count/membership preparation is integrated; ordinary O2 `cargo nextest run --locked --offline --cargo-profile test -p lctx-model --test domain_analytics` passed4 (`20261012T103926.958Z-c5582a`, exit0/cleanup confirmed10:41:22UTC). Its prior invocation selected a nonexistent Nextest profile and ran no tests (`102421…633aaa`, exit96). Earlier actual-native A07 results remain unknown after timeouts; they are not passes. Use current finite and minimal revealing production cases; historical artifacts, real-library campaigns and benchmarks are not exit gates. |
| Serving/ranking | **Source integrated / current acceptance pending:** exact selected full-value ranking, contextual eligibility before candidate caps, compact pinned preparation, page hydration and small in-memory sorting remain the target. The exact obsolete-index predicate passed within `20261012T073120.201Z-23a76a` (eight selected controls, exit0/confirmed cleanup; most other selected historical controls are now retired and confer no current acceptance). The finite numerical-reference case passed in `20261012T080642.901Z-af59c9`; its actual-cohort case failed for missing `LCTX_NATIVE_SERVING_CONFIG` (exit1/cleanup confirmed), requiring the current retained-serving producer rather than plain fixture attachment. Current inspection also found that the private Python adapter built an obsolete RuntimeConfig and passed raw SQL to the relation-only snapshot query command. Its exact-SQL comparison omitted actual served candidate/rescore output. Replace it with a bounded read-only current serving replay over a fixed protected handle, then independently compare stored F32 values/projections and actual production candidate scores/order, without inference. The current private Rust replay now observes protected selected occurrences, canonical F32 full values/projections and actual production candidate/rescore kernels; Python independently computes scalar F64 scores, witness ties and the1024-target frontier. Seven host controls passed. Initial current Rust build (`154123…d9c5e9`, exit101/cleanup15:45:48UTC) exposed two example API mismatches; corrected build (`20261012T154606.804Z-168d9c`, exit0/cleanup15:47:27UTC/no survivors) passed for current CLI/evaluator/replay. Actual retained-cohort execution remains pending. Shared occurrence eligibility is a declared common-mode limit; the separate known foreign-cohort native control supplies bounded independent inclusion/exclusion expectations. Selected-profile replay build/environment wiring passed four host controls (`20261012T153231.120Z-a6f66a`, exit0/cleanup15:32:31UTC/no survivors). Actual independent numerical/current serving cases must pass before removing the current HNSW declaration and explicitly retiring its installed catalog entry. |
| Finite query ownership | **Focused-Tested within prior scope:** SDK raw query shape controls passed4 (`025058`); native query/audit controls passed10 (`030306`); corrected campaign wire controls passed9 (`20261012T033523.004Z-40c84c`, exit0/confirmed cleanup), including physical EOF, late errors and cancelled delivery. SDK shape and original session semantics are retained. Current reader-pin changes still require their own native control; these passes do not qualify current cutover or the whole track. |
| Signals and request ownership | **Implemented / partial focused-Tested:** request admission precedes embeddings, HTTP lifetime is joined, and process signals retain local/native cleanup independently of child exit. Earlier affected Python controls passed50. Actual direct-native SIGINT/SIGTERM passed in the15-control native execution (`132647…fc65c7`);14 other native cases failed as recorded above. Current source abandonment now separates historical operation failures from actual join/cleanup finality, serializes acknowledged attempt/session close, and skips repeated remote work only after its real acknowledgement. Immutable signal finality is published only after acknowledged session closure. A cancelled listener join retains its actual JoinHandle for retry. Independent source review accepts these bounded fixes; six focused pure controls passed (`20261012T135600.892Z-4ce91d`, nextestfb2877ef-bcf9-4d40-af1c-b9ef1be0d30f, cleanup14:01:59UTC/no survivors). Current15 native controls (`20261012T140232.596Z-4a34ff`, nextest5a11343a-3ff4-4d09-bbf0-e856abe854d0, cleanup14:04:40UTC/no survivors) passed13/failed2: original-backing fixture omitted its caller budget; retention fixture requested a bound view before binding. Both fixtures are corrected without changing assertions; actual rerun remains pending. Attachmentaf6e8d06-492d-4bb1-b216-6dcf899b41b0 preserves native Unknown. Its named recovery (`20261012T151841.606Z-6a7f02`, cleanup15:18:46UTC/no survivors) joined the exact daemon but the immutable older installer failed validation drainage on field-postfix locking targets. Separate current-driver drains (`152219…e2a755`) and original recovery resume (`152248…a9672b`) passed, with confirmed cleanup/no survivors, original bytes and Unknown preserved. Corrected fixture/ordinary retirement execution (`20261012T152259.744Z-01fe34`, nextest04cd422d-7ebd-4eb6-a397-4d8181158669, cleanup15:26:22UTC/no survivors) passed9/failed3 of12. Indexed EXPLAIN lacks its caller budget; completed original piece mutation now reaches the stronger exact blob-piece guard before generic immutable-address collision; retirement must resume pending child nominations behind its current forward cursor before requiring a retained receipt. Source review confirms these fixture-contract corrections, preserving actual wire acknowledgement/no-effect and exact protected contributor assertions. Attachment83d08eeb-f68d-4dee-a94c-cffb15bdc634 recovered through separate current-driver drains (`153139…df800a`, cleanup15:31:50UTC/no survivors) and original resume (`153154…bf12f8`, cleanup15:31:57UTC/no survivors; receipt SHA256107195cbb1a2c183859bbbf0e7f6984d293dcfc5d4dde05759ea00bbb990d04c); original Unknown remains and completed one-off source is retired. Current20-control native/cache/admission execution (`20261012T153258.415Z-8e2688`, nextest18b5c37a-8dd8-4432-882b-62210a14301b, cleanup15:34:19UTC/no survivors) passed17/failed3. All three corrected budget/guard/queue cases passed, as did both hydration controls and12 native cache cases. Failures exposed: imported dependency views lacked exact view→contributor incidence before installed traversal (production fix now establishes it from local imported contributors); the old cache fixture no longer crossed the binary chunk boundary (now CHUNK+1 with actual physical chunk assertions); the durable counted invariant was registered but not selected by its relation (test-local relation now explicitly references it and asserts selection). Targeted rerun (`20261012T154902.013Z-58cd41`, nextestf54ea27d-fbc6-467d-b814-240a92d911c2, cleanup15:50:32UTC/no survivors) passed imported-view round trip and physical chunk assertions; the durable fixture failed exact native relation authority because its test-local type was not the registered native record. The canonical InputRevision fixture with its existing relation-owned invariant preserves the TypeId/schema gate and passed in`76d9cb`. At that earlier checkpoint actual terminal cleanup succeeded and campaign borrowers were empty; caller failure is retained, not promoted. Attachment1d6f63ad-ab87-4980-978a-86fdea170692 retains Unknown; named recovery (`20261012T153645.238Z-3b1713`, exit75/cleanup15:36:46UTC/no survivors) refused an owner outside its bounded registered recovery classes. Exact inspection found registered clean cache/product-source/admission siblings and one Started cache session outside the finite owner classifier. A bounded classifier repair passed40 host controls (`20261012T154205.962Z-43ec97`, exit0/cleanup15:42:15UTC/no survivors), retaining all original bound bytes and rejecting unsupported failed/Started classes. Actual named retry (`20261012T154259.947Z-169c47`, exit1/cleanup15:43:03UTC/no survivors) joined the exact daemon, then the immutable older CLI failed validation drain on field-postfix locking targets. Separate current-driver main/validation drains passed (`20261012T154802.309Z-0db26b`, cleanup15:48:13UTC/no survivors; candidate SHA2564b7dca569fb35e020e6f858e4b8c448bdf9fa3c33f2a87b2d2271cfac48ebd4a), then the original named recovery passed (`20261012T154836.216Z-fbb84a`, cleanup15:48:40UTC/no survivors; receipt SHA25654a160559f9f551e47f733b7f98ca5c973c3f15f14547b75bd0031a45b487a34). Original outcome and immutable proof remain unchanged; completed temporary source is retired with its receipt in the separate repair run. Compatible installer handoff (ADR-0163) subsequently passed under D04 in`162328…0efd83`; the obsolete executable remains protected by the external observation blocker above. The import missing-contributor refusal passed (`20261012T153953.436Z-60140a`, nextest0510fee8-eb97-4ea1-b7c6-f42d9c2b49af, exit0/cleanup15:41:09UTC/no survivors). No lifecycle or recovery gate was relaxed. No mocked/native outcome is promoted. |
| Exact failed-owner recovery | **Focused host-Tested2026-10-12:** native-host receipt controls passed36 (`052921`); exact failed-wire attachment recovery passed (`053059`, receipt SHA256 `f188bf67adc2450be84ae93f331804f215ac5eb89f4c2cda10f0de25282e6227`), preserving its original IAM failure/Unknown; PD02 recovery controls passed12 (`053145`). The separately interrupted upper attachment is explicitly disposed by completed campaign regeneration, with its original Unknown preserved. Registered terminal store/workspace and clean signal sibling recovery classes passed41 focused host controls (`133321…d556f6`). Current named attachment244447dc-d09d-4e20-ac7f-9cabdacac1d7 recovered after separate current-driver drains (`135037…4d76c9`, `135100…ecdbae`), preserving original failures, Unknown and immutable inputs. Retained server handoff dependency observation now follows the exact executable chain across native regeneration while active resume retains exact native-generation guards;14 focused host controls passed (`135621…72d311`) and actual current status/dependency observation passed. Prior server/drain receipts are scoped evidence, not whole-track acceptance. |
| Design-phase cutover | **Current campaign cutover passed2026-10-12:** exact old daemon joined; only approved main/validation scopes discarded and regenerated, initialized and checked before reopening admission (`20261012T131431.314Z-4f6e67`, exit0/cleanup13:14:45UTC/no survivors). Original plan/join/removal bytes and both independent initializer revision/removal chains remain immutable. The local attachment disposition validates released=true and preserves original native outcome Unknown. Earlier initialization failures exposed pinned3.3 boolean syntax and six undeclared indexed IDs; both are corrected in current source. Repeated revision/self-contained receipt host controls passed (`104805…de338a`). The completed one-off regeneration module, tests, action flags and service/storage hooks are removed; generic completed receipt/current recovery consumers remain, with35 focused host controls passed (`20261012T131755.808Z-138c17`, cleanup13:17:57UTC). Actual completed receipt still validates after producer deletion. That cutover permitted earlier borrowing; current borrowing is blocked by closeout recovery`634eef` above. Shared paused/operator state remains untouched. Exact source5 owner retirement removed four obsolete input/producer/recovery roots (`133451…228bb7`); its compact disposition and unrelated warm caches remain protected. Old local run consumers released (`134124…6e0281`); authenticated prior pc3-scope removal reconciled through targeted unregister. Exact released old run logs/receipts retired (`134455…4fd618`), followed by integrated source5 checkout and obsolete helper removal (`134511…b09f1e`), both exit0/cleanup confirmed/no survivors. Its task build root is released under lifecycle grace; no cache purge. |
| Ordinary profile and final acceptance | **Pending:** O2 dev/test keeps assertions/overflow, imported O3 and normal parallelism; release remains default until this track's current affected functional controls qualify ordinary tests. X01 current functional acceptance and X02 resumption handoff remain open. Existing plans are not superseded or credited with closure. No real-library/operator activation, gold/heldout access, benchmark or broad storage purge is implied. |

Earlier detailed implementation receipts remain attributable in their existing managed run/evidence owners and Git; they are not an additional execution queue or current historical-migration requirement. The current table and finding dispositions below own the remaining work.

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
| Complete composed state, physical validity and migration | N06 logical six-family composition separated from N04 physical completion; N11 explicit factory/implementation/physical-epoch premises, cold invalidate/recheck. Current-only cutover under ADR-0161: practical view migration or explicit pinned-input regeneration | All closure-family mutations, bindings/aliases and incarnation/restart/import controls; regenerate current fixture views without a historical converter or old-revision acceptance prerequisite |
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

**Implementation in progress, 2026-10-12:** [§4](#4-package-sequence-and-sole-execution-checkpoint)
is the sole current checkpoint. ADR-0161 replaces historical converter/fixture prerequisites with
current view migration or pinned-input regeneration. No authentic old producer, old-format reader,
opaque legacy store or historical benchmark gates delivery. The original interrupted native outcome
remains Unknown; its exact joined daemon shutdown and approved scope-removal receipts are retained
under the existing attachment owner. Current initialization and functional acceptance must establish
the delivered contracts. Shared paused/operator state and old-plan qualification remain separate.

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
