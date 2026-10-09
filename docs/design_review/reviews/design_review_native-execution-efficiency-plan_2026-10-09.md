# Native execution efficiency plan — independent target review

**2026-10-09 · design / target · decision: Accept scoped at Proposed / Interface-checked strength**

The proposed target addresses the source review's eight structural causes without replacing semantic admission with cache trust or converting every relationship into a scheduler dependency. Its useful organizing boundary is one immutable resolved input inventory, shared physical preparation and typed values, independently owned checks, and explicit effect/lifetime owners. The native access proposal also accounts for an important library limitation: a bounded `IN` request can still create result-sized native distinct state.

No blocking target finding was identified in the stable coordinated documentation. This accepts the proposed responsibility split, migration prerequisites and qualification route. It does not accept the current implementation as corrected, establish the generated native query's chosen plan, qualify the SDK patch, close source findings, or establish a latency/capacity improvement. The enclosing implemented native architecture still needs the scheduled corrections and matching-source acceptance.

## 1. Scope, baseline and method

| Field | Reviewed boundary |
|---|---|
| Subject | [Native-efficiency plan](../../plans/native-execution-efficiency-plan_2026-10-09.md), its source-review coverage, and coordinated seams in persisted coordinator §7.2/§8, GK, GR, PC4, PJ, BC and CU |
| Baseline | Main `d2341cc4`, plus the frozen large dirty GK/GR/native/compiler/serving tree. Final coordinated documentation inspected after the author declared it stable on 2026-10-09. Pending STATUS/publication receipts are outside the architectural judgment |
| Standard | Core principles/template 3.3, Efficient Architecture Heuristics 1.0, code-intelligence principles/review 1.5, repository binding; loaded through [standard.toml](../design_principles/standard.toml) |
| Reviewer | Fresh independent design reviewer; did not author the plan or production correction and did not delegate further |
| Functional intent | Pinned catalog and explicitly selected behavioral compilation, exact completed-input selection, support/invariant admission, optional products, native finality, pinned viewer preparation and purposeful extraction observation |
| Workload | Many producer owners and overlapping roots; sparse keys, reconvergence/high degree, full negative domains, cold/cache-off and warm execution, independent readers, cancellation and retirement on the existing resource-owned host |
| Method | Read-only inspection of the plan, source review, relevant semantic/storage/product owners, decisive current sources and locked library implementations; no repeated whole-codebase audit |
| Exclusions | Actual selected-query/session/runtime qualification, complete served-evidence journeys, whole-library/provider breadth, measured benefit, operator adoption and protected evaluation. The plan assigns their relevant implementation/acceptance obligations rather than claiming them |

The source review remains the authority for its dated diagnoses and historical receipts. This review independently inspected the consequential mechanisms rather than treating that review's verdict as proof. Product testing remains paused during documentation authoring. No builds, tests, probes, fixture operations, environment synchronization or repository writes were performed by this reviewer.

## 2. Responsibilities, domain meaning and fidelity

| Owner | Meaning or responsibility | Proposed consumer boundary |
|---|---|---|
| Model scope/stage/ownership declarations | Roles, ordered selectors, epoch visibility, support and invariant semantics | NE1 resolves declarations into immutable exact bindings; mechanisms consume them rather than deriving meaning from aliases |
| Core attempt/input preparation | Resolve sources, group compatible physical demands, preserve partitions | Registration, logical templates, product dependencies and native resolution consume the same inventory |
| Native access | Physical membership/index selection, ordering, hydration, transport terminals | Exact view and nominal demand remain authoritative; derived selection is private and rebuildable |
| Admission owners | Applicable independent rejection semantics, including Full and orphan domains | Borrow shared typed/sorted inputs through separate validator objects; no shared validity bit |
| Product owner | Canonical portable values, complete dependencies, current semantic acceptance | Decode once, check before effects, transfer into fresh ingress; readiness is private and attempt-bound |
| Producer/native lifecycle | Producing descendants, acknowledged effects, local completion and final drainage | Relevant terminal work gates local visibility; global failure/freeze/seal obligations survive |
| Pinned viewer | Exact preparation identity, stable flights, optional retention and close | Flight lifetime survives Moka generation replacement; requests may cancel independently |
| Extraction harness | Selected versus complete observation | Observation demand follows the control's guarantee, with independent complete-universe controls retained |

These boundaries refine [semantic-model §15.10–§15.12](../../design/sections/semantic-model.md) and [storage/publication §6.2](../../design/sections/storage-and-publication.md). Existing completed views, nominal roles, stage declarations and independent validators supply an adequate domain model for this bounded correction. NE6 deliberately adds an operational distinction between local contribution completion and final attempt certainty; it does not reinterpret a semantic relation edge as a work dependency.

| Fact or relation family | Provider/revision and fidelity | Coverage, identity and retained consumers |
|---|---|---|
| Syntax, typing and declared runtime views | Existing pinned provider contracts; extracted/provider assertions retain their different meanings | Exact producer/profile/model/configuration/source bindings; disagreement, unknown and NotRequested stay explicit; compiler and evidence consumers retain them |
| Normalized support and qualifications | Model-owned resolution/derivation | Exact roots/roles/epochs; physical absence differs from virtual or empty support; independent actual-row/negative admission remains |
| Behavioral and analytical results | Existing named finite model/projection/method | Five verdicts and declared heuristic status remain; complete universes, directional relationships, multiplicity and provenance are unchanged |
| Products and served preparation | Derived values, never additional fact authority | Missing/deleted dependencies remain key material; fresh native ingress and one immutable viewer realization preserve interpretation and evidence references |

The plan changes execution mechanisms and ownership granularity. It does not add a fact category, alter evaluator meaning, collapse relationship identity or claim more provider coverage.

## 3. Physical composition, contracts and source coverage

Direct inspection found Cartesian owner/key enumeration and owner-driven payload windows in `compiler.rs:3286–3433`; selected token querying at `:2869–2888`; per-grain invariant execution in `scoped_admission.rs:152–196,243–293`; repeated session registration in `workspace.rs:2572–2619`; multiple portable candidate decoders in `workspace_products.rs:62–149` and `normalize/reuse.rs:25–49`; polling in `native_bridge.rs:103–133`; global scan waiting at `compiler.rs:1881`; replaceable initialization generations in `preparation.rs:67–72,108–146`; and whole-inventory observation in `typed_driver/mod.rs:65–95`. These support the plan's chosen correction boundaries, without attributing historical timeout shares.

| Source obligation | Target and independent challenge | Assessment |
|---|---|---|
| F01 / RC01 | NE2 unary key equality prefix plus exact-owner residual; one-owner direct lookup and verified-pointer paths; compact exact-view preparation for repeated demand | Credible conditional route. It avoids absent owner/key construction and prohibits native union-seen replacement. Foreign owners sharing a key, conflicts, empty validation, projection/order and terminal errors remain explicit |
| F02 / RC03 | NE1/NE3 captured providers/logical inputs, compatible union hydration, borrowed partitioned checks | Preserves independent check objects and complete OwnershipRows::Full inputs. Actual stored-output/orphan roots and invalid rows outside producer positives cannot be dropped |
| F03 / RC03 | NE4 one union token inventory per exact input/root group | Separate role-associated root domains retain missing/empty and complete membership discovery. Enabled lookup is correctly separated from cache-off failures |
| F04 / RC03 | NE4 one charged typed candidate through canonical and current semantic checks, then ingress | Wrong-but-canonical rows must refuse before registration/mutation. Optional pre-effect fallback does not cover committed/uncertain effects. Cold capture has its own actual-completion comparison, with a legitimate single-read fallback |
| F05 / RC04 | NE5 event-driven handoff, coarse async completion, registration reuse and moved charged batches | Synchronous callbacks retain a suitable blocking owner; Tokio blocking receive is explicitly excluded on async workers. Queue pressure, cancellation and late acknowledgement keep ownership |
| F06 / RC02 | NE6 relevant producing descendants gate local completion | ProducerOutput/drain ownership, a private scoped terminal handle and global failure propagation prevent simple deletion of `wait_scans` from becoming premature validity/publication |
| F07 / RC04 | NE7 stable viewer-local flights composed with optional Moka retention | Installation precedes spawn; waiter cancellation, completion/removal identity, miss/completion race, close and external value leases are specified. Waiting requests cannot hold the loader's last permit |
| F08 / RC05 | NE8 explicit selected/full observation and bounded/coarse reads | Narrow observation no longer requires unrelated rich relations. Complete inventory/determinism/orphan controls remain explicit and native-backed; attempt-private databases are isolated |

For support and normalization, the projection universe is the model-declared exact inputs and ownership program, not the displayed root subset. Root direction/roles/partitions and complete negative universes stay meaningful. Method/settings and exact-under-model claims remain with their existing owners; no analytic algorithm changes. Prepared native/provider streams continue to fail on incomplete terminality; resource refusal is not relabelled as complete absence. Cached selected domains are derived from freshly discovered membership before reuse, not used as evidence that discovery was complete.

The lifecycle target separates local output visibility, content freeze, semantic admission, publication and final seal/abandon. An unrelated reader may finish or fail after a contribution is locally complete; final admission must still observe a compromising failure. This is a useful composition improvement only if the relevant descendant contract is actually enforced, which NE0/NE6 require before migration.

## 4. Library fit, alternatives and growth scenarios

The relevant library skills were loaded, and decisive API/planner claims were checked against locked registry source. No dependency change or cross-version API transfer is needed.

| Capability | Resolved-version evidence and composed choice | Tradeoff / condition |
|---|---|---|
| Native membership | SurrealDB core 3.3.0 `idx/planner/plan.rs:323–365` builds equality combinations; `:605–606` requires distinct for Union; planner `mod.rs:315–316` enables it; `dbs/distinct.rs:8–31` retains processed IDs in memory | Unary equality avoids that Union mechanism in inspected code. The actual generated query/driver must still qualify. Residual owner filtering can examine unrelated owners; NE2 charges it and provides actual-owner prepared selection when repeated demand justifies startup |
| Relational preparation | DataFusion 55.1.0 `session_state.rs:1013–1033,2352–2357` retains logical prepared plans; `dataframe/mod.rs:1601–1604` creates physical execution for a demand | Typed providers/logical templates fit existing consumers. Provider-bearing TableScans must be bound correctly; a mutable shared catalog or parameter substitution cannot silently rebind a grain. The effect-disallowing SQL boundary need not expose PREPARE |
| Retention/coalescing | Moka 0.12.16 `future/cache.rs:635,666,843,1221–1315` puts initialization ownership in each cache object and coalesces same-key fallible calls there | Moka remains suitable for optional completed values. A small stable lifecycle map supplies the cross-generation flight contract; it must not grow into a generic cache/workflow reimplementation |
| Domain/dependency graph | Existing model schedule and petgraph dependency metadata; `compilation.rs:479–579`, `compilation/reuse.rs` | Resolve exact bindings for current consumers. A reverse closure may serve a named consumer, but reachability metadata does not certify cache validity or effect completion; no universal execution graph is needed |
| Compact ordering/typed transfer | Existing charged spillable native ordering and Arrow/DataFusion representations | Reuse them without imposing rich whole-graph residency or a second nested runtime. Sorted runs/cursors and typed candidates retain their charges and terminal owners |

The alternatives are considered at the complete operation boundary. Retaining Cartesian point enumeration leaves the demonstrated absent-combination work. Bulk native `IN` relocates expansion and adds distinct state, so it is not a suitable default. Unary access preserves simple native lowering at the cost of per-key crossings and unrelated-owner matches; compact exact-view selection amortizes repeated broad reads while preserving a sparse path that avoids mandatory whole-view startup. Typed/spilled stages followed by final native materialization remain a credible alternative if qualified native preparation is still disproportionate; the plan names that trigger and an ADR/owner route rather than ruling it out because of current placement decisions. Salsa/whole-graph state adds no demonstrated consumer benefit here.

| Realistic change or failure | Owning edits and propagation | Revealing boundary |
|---|---|---|
| New fact family/domain extension | Model meaning, provider behavior and invariant additions; mechanically derived storage/input registration | Existing observer/cache/dispatcher need not enumerate unrelated families. New negative obligations still belong to their validator |
| Native access mechanism substitution | Native adapter and selected-plan evidence; preserve exact view/root meaning | Equality, direct pointers or derived selection must agree on foreign/missing/conflicting owners and ordered projection |
| New invariant/composition | New independent check semantics over compatible prepared inputs | Full/negative domains and input order remain; sharing bytes does not inherit another check's success |
| New epoch/settings/source | New immutable binding/product identity | Equal aliases or row contents cannot merge explicit epochs or preserve absent/deleted dependencies |
| High overlap/high degree/repeated broad demand | Shared compact discovery, union hydration and spillable complete input | Input reuse avoids repeated rich decoding; actual match work and preparation startup remain charged and justified |
| Queue-full cancellation or late failure | Bridge/native submitted owners retain arguments, session and acknowledgements | Wakeups must not deadlock an async worker or erase failed effects after caller disappearance |
| Concurrent reader/producer completion | Producing descendant scope versus final global drainage | Unrelated immutable read does not extend local wait; its compromising late failure still prevents final success |
| Retention pressure during initialization/close | Stable viewer flight identity, waiters, permits and value leases | No second same-key loader, stale removal, stranded waiter or premature reader invalidation |

## 5. Package prerequisites and coordinated consumers

NE0 provides the governed access/local-completion decision and qualifies the existing SDK session patch before native dependent controls. NE1 provides a working immutable binding/compatibility contract. NE2 qualifies physical access before consumers rely on it; NE3 consumes that access for shared native admission. NE4 depends on the working semantic predicate/input slice and native token route, and therefore cannot delay cold access/admission correction. NE5/NE7 can progress on independent ready contracts; NE6 needs only the actual delivered owner/acknowledgement slice. This ordering distinguishes logical readiness from conflicting file ownership and does not introduce another task scheduler.

The final coordinated seams are coherent: GK §3.3 adopts shared bindings and independent physical admission; GR §2/§4 adopts union tokens, typed candidates and stable flights; PC4 replaces mandatory multi-owner point enumeration while retaining order/residual/terminal and actual-plan requirements; PJ preserves final freeze/global closure; BC/CU preserve shared erased orchestration and small typed leaves. The new target does not reopen vocabulary-sized coroutines or expand unrelated provenance fingerprints.

[Persisted coordinator §7.2/§8](../../plans/persisted-graph-execution-plan_2026-10-07.md#72-native-efficiency-replacement-and-common-acceptance-2026-10-09) owns sequencing, all eight source-qualified Open rows and closure evidence. The source review now links to that transfer, retaining its dated judgment. NE9/GK7/GR6 combine matching-final-source acceptance and retirement with surviving PC6/CU6/BC5 obligations; BC3 keeps its separate candidate-profile gate and release defaults. The DESIGN §B3 ADR-0140 reference repair records existing authority and does not adopt NE0.

The plan accounts for the source review's additional investigations: actual native planning, complete-grain compatibility, cold capture, session/authentication uncertainty, bridge/descendant ownership, flight pressure, setup/runtime sharing, useful graph integration, alternative placement and measurement/evidence breadth. Each has a package decision or a concrete reopening trigger. No consequential recommendation was found silently dropped or converted into an unexplained blanket deferral.

## 6. Independent judgments, gates and applicability

| Judgment | Verdict for the proposed target | Own evidence / scope |
|---|---|---|
| A1 Localize change | Satisfied | Exact bindings and coherent preparation/admission/lifecycle/observer owners constrain extension propagation; explicit finite transformations retain bounded test dependencies |
| A2 Encode domain meaning explicitly | Satisfied | Model roles/epochs/scope/ownership govern shared preparation; independent acceptance and typed effects remain. NE6 adds the relevant operational distinction through its actual producing-work owner |
| A3 Extend through composition | Satisfied | Union physical work composes separate roots/checks; local completion no longer requires unrelated readers; library primitives and ordinary scoped owners suffice |
| A4 Fit execution to the supported workload | Satisfied, scoped Proposed design | Credible access alternatives, charged compact preparation/spill, union hydration, one decode, wakeups and appropriate lifetime scopes remove identified amplification. Actual native query choices and runtime implementation are excluded pending NE0/NE2/NE9 evidence |

FP-01–FP-06 are satisfied for the proposed contracts and change scenarios. FP-07 is satisfied at the scoped target strength above, not as a claim about the uncorrected implementation. DP-01–09/11–12/18–24 retain one authority, exact identities, independent enforcement, declared effects and finality. DP-10/13–17 are satisfied by useful preparation, qualified library composition and bounded bespoke ownership gaps. CI-01–08/10/13 are preserved in the scoped identity/coverage/projection/pin contracts; CI-09 has no changed heuristic rule here, and CI-11's full served-claim breadth is outside this review. CI-12 is unchanged; no protected evaluator input or meaning is touched.

| Gate | Verdict | Independent basis / limit |
|---|---|---|
| G1 Authority | Pass, target scope | Declarations govern bindings and operations; prepared forms/products/flights are derived, private and rebuildable |
| G2 Semantic fidelity | Pass, target scope | Root roles/partitions, nominal identity, empty/missing and distinct epochs remain explicit; physical grouping cannot flatten them |
| G3 Validity | Pass for proposed enforcement contract | Separate actual-row/current semantic checks and Full/orphan domains remain; reject-before-ingress is explicit. Actual corrected enforcement remains not_run |
| G4 Hidden behavior | Pass, target scope | Effects stay owned; typed read-only preparation does not open SQL statements; local completion grants no publication |
| G5 Consistency/recovery | Pass for proposed lifecycle contract | Scoped producing terminality, global poison/finality, flight races, pins and close are specified. SDK/session and concurrent runtime qualification remain excluded until NE0/NE6/NE7 |
| G6 Transformation/reuse | Pass, target scope | Complete discovered domains, current checks, fresh ingress, exact bytes and pre-effect fallback preserve reuse meaning. Whole-stage/selected runtime equivalence remains an implementation obligation |
| G7 Truthful claims | Pass | Proposed, Implemented and historical Tested boundaries are distinguished; timeouts/cancellation, unqualified patch and unmeasured benefit are not relabelled as success |
| G8 Library leverage | Pass, scoped Interface-checked | Native Union limits, DataFusion logical preparation and Moka per-generation initialization are qualified; selected gaps are small owned compositions. Actual-query behavior is not assumed |
| CI-G1 Fidelity | Pass, target scope | No provider meaning, heuristic status, unknown/absence or finite model contract changes; independent negative controls remain |
| CI-G2 Evidence closure | Pass for changed pin/preparation contract; complete journey not assessed | Stable exact viewer realization and external leases remain. No complete resource/continuation/claim-to-fact certification is inferred; affected journeys remain NE9 obligations |
| CI-G3 Evaluation integrity | n.a. to this correction | No private truth, protected population, evaluator meaning or comparison baseline changes |

No new Fnn is assigned: no blocking architectural defect was identified in the reviewed stable target. The existing source F01–F08 remain Open in the coordinator; this result does not supersede them.

## 7. Evidence limits, rule impacts and decision

| Boundary | Evidence/date and outcome |
|---|---|
| Target composition and coordinated prerequisites | Proposed / source-inspected, 2026-10-09; static judgment above |
| Exact library mechanisms | Interface-checked / locked-source inspected, 2026-10-09; not a generated-query or runtime pass |
| Builds, focused controls, actual native plans, SDK qualification | **not_run** by this reviewer; delegated scope expressly prohibited execution. Commands and selectors belong to companion §7 / coordinator §9.1 |
| Documentation checks/publication | **not_run** by this reviewer; root owns `just docs-check` and scoped `just turn-end`, with receipts recorded after publication |
| Whole implementation, real-library readiness, speed/capacity | Unqualified / unmeasured; no new claim |

No additional rule relaxation is required by this review. NE0 must install RC01/RC02's complementary ADR and storage/lifecycle owner updates before dependent implementation, carrying forward ADR-0138's canonical authority and final admission obligations. RC03–RC05 retain independent acceptance, stable in-flight ownership and complete observation guarantees. A later better intermediate placement follows the existing supersession route if its named trigger is reached.

**Bounded target decision: Accept scoped at Proposed / Interface-checked strength. Enclosing implemented architecture: needs the scheduled native-efficiency corrections and remains unqualified.** The excluded actual native query/session/flight/descendant implementation scenarios return at their named package gates and final-source NE9 acceptance. If unary access still scans disproportionate unrelated ownership, uses native Union/distinct, or cannot preserve complete terminal semantics, the native lowering must change before migration or closure; the plan's derived exact-view route and placement reconsideration remain available. If compatible input sharing omits a negative universe or flight/lifetime changes weaken finality, that is a required correction rather than an acceptance exception.

The next consequential work belongs to the root's detailed execution planning for NE0/NE1 and the actual query/lifetime contract slices. Correct the cold membership/admission/coordination route before attributing cache-off failures to warm reuse, and retain source-specific retirement/evidence before closing any finding. Production and qualification remain paused by this documentation-only review.
