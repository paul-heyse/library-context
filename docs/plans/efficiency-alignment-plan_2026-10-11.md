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

**Execution started, 2026-10-11, baseline `ebcf423cc16e6cacea6455500a0cdfc9795d08ac`.**
The operator approved the detailed execution sequence, one dedicated campaign installation and
all affected functional boundaries, enabled optional paths and representative integrated journeys.
Repository quality campaigns and complete ordinary-suite execution remain excluded. The shared
service and paused qualification retain their own state. The existing plans and source reviews are
unchanged. Package/finding closure still requires its complete corrective obligations.

| Implementation slice | Current evidence and remaining boundary |
|---|---|
| Campaign prerequisite / G00 validation owner | **Implemented / focused-Tested 2026-10-11:** ADR-0149 and the existing installation owner add an explicit checked UUID-derived campaign unit; fixtures, daemon checks, recovery assets and restart/handoff paths use its identity. `just run --background --label efficiency-campaign-routing -- uv run --no-sync pytest -q tests/scripts/test_surrealdb_service.py tests/scripts/test_surrealdb_fixture.py` passed, run `20261011T161840.282Z-c6ffb5`, cleanup confirmed. Script controls are not native finality evidence. |
| Dedicated source-format installation | **Passed 2026-10-11:** `LCTX_SURREAL_SERVICE_CONFIG=/home/paul/.local/state/library-context/efficiency-alignment-2026-10-11/installation.json just run --background --label efficiency-campaign-install -- uv run --no-sync python scripts/surrealdb_service.py install --campaign --installer target/release/lctx`, run `20261011T161912.657Z-56a20d`, cleanup confirmed. Installation `0ec1d0c0-fbd0-4265-93e5-d9e186bab099`, schema5/eec4, owns its immutable installer and main/validation scopes. Actual source5 obligation populations, repeatable cold checkpoint and source→target conversion remain to be delivered. No shared installation was observed or modified. |
| Ready independent packages | S02/S08, D06–D09 and A01–A05 are being implemented; focused evidence is pending. All other product packages remain Planned. |


All package rows below are **Planned** at this checkpoint. No source finding is closed by plan approval.
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

**Current checkpoint, 2026-10-11:** implementation is authorized and underway at the §4 checkpoint.
The resolved design decisions and their bounded investigation evidence remain applicable within their
original scope. Campaign isolation is implemented and the dedicated schema5 installation is available;
source5 obligation populations and cold checkpoint are next native prerequisites. Ready independent
packages are in progress. S09/D10 still requires the selected server execution-owner handoff and
actual finality controls. No source finding is closed by installation or by an accepted decision.
Real-library/performance campaigns and repository quality checks remain not_run. Existing shared-service
and old-plan qualification state is unchanged.

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
