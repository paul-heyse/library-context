# Design review: implementation friction, production execution and Codex effectiveness

**Decision: Revise — production outcome/admission contracts and composed host cleanup need correction; several related execution-fit improvements are also warranted.** The recent work repaired real defects. It preserved exact semantic selections, independent native admission, coherent publication and conservative recovery rather than making failures disappear by weakening checks. However, closely related problems remain at adjacent boundaries. An acknowledged operation refusal can become failed finality through a dispatcher; optional reuse capture can cancel required compilation; allocation admission sometimes follows allocation; and temporary accounting or overlapping preparation can survive far beyond its useful work.

The development loop exposes the same general weakness at a different semantic boundary. A managed run and a fixture each understand their own process group, but the enclosing run does not compose the fixture's delegated host cleanup. Its direct-group cleanup can be confirmed while a nested compiler still runs and writes the run log. Separately, run discovery expands retained history into full live observations when the agent needs only a few recent commands.

These are several structural causes, not one universal lifecycle defect. The recommended response is a small set of owned contracts, using the existing pools, ordered runs, dispatchers, process identities and evidence adapters. A generic workflow engine, new memory pool, automatic cleanup policy or pervasive telemetry system would add unnecessary machinery.

## 1. Scope, evidence and functional premise

| Field | Assessment |
|---|---|
| Baseline | Clean main at 5ffdca70439ac7474f90ebc639453dfc55dd2536; relevant predecessor checkpoints e54f03cd and 426f3892 |
| Window | Approximately 2026-10-14 17:00–20:14 UTC, including causal antecedents where needed |
| Standard | Core/template3.3, heuristics1.0, code-intelligence1.5 and library-context binding |
| Tier / purpose | Design / target; code, development operations and their composition |
| Reviewers | Root coordinator plus three fresh same-model Codex reviewers for production execution, working set and tooling; root inspected decisive evidence and reconciled the combined judgment |
| Evidence strength | Implemented/source-inspected diagnoses, with explicitly dated historical operational observations; remedies Proposed |
| Execution in this review | Source, retained logs, session metadata and installed documentation inspection. No builds, product tests, process probes, database queries/campaigns, full Catalog or FastMCP journey |
| Product premise | A version-pinned API/evidence Catalog from library release/corpus/import inputs; exact attributed facts and linked original evidence; optional Behavioral/analytic products remain distinct |
| Workload | One operator and Codex/model family, concurrent producer and agent work, overlapping dependency closures, shared relations, large selectors, skew and cancellation; development data can be regenerated |

The [evidence index](../evidence/2026-10-14_implementation-friction/README.md) records run identities, source limits and actual observations. The [previous integrated review](design_review_catalog-creation-completion-integrated_2026-10-14.md) and [Catalog coordinator §6](../../plans/catalog-completion-plan_2026-10-14.md#6-sole-disposition-and-current-checkpoint) are leads and comparison material, not substitutes for source inspection.

This review does not resume Catalog execution, change existing findings' dispositions, or certify whole-product acceptance. Embedding execution, MCP-submitted queries, operator adoption, protected evaluation and older paused campaigns remain outside the executed scope. Adjacent retained/cold consumers were inspected where they share the reviewed factories; the full serving/query architecture was not recertified.

No timing benchmark is needed to establish a dead allowance, repeated prefix reconstruction, allocation-before-admission or a missing cleanup relationship. Existing durations describe particular failed or successful runs; they do not establish speed ratios, RSS or the dominant cause of every timeout.

## 2. Responsibilities and distinctions that should survive

| Owner | Meaning and consumer contract |
|---|---|
| Model stages, completed views and charged containers | Exact prerequisites, requestedness, contributor sets, nominal union validity and resource admission |
| Workspace / compiler-store | Producer completion, selected-input composition, private reuse, actual accepted task ownership and coherent handoff |
| Native installed/completed factories | Exact physical correspondence, current protection, independently earned inventories and retained backing |
| Native authorization / retirement | Recorded effects, transactional locks and fences, actual outcomes, fresh discovery and bounded progress |
| Harness / managed runs | Authenticated host process ownership, command result, capture and direct cleanup |
| Fixture / service | Delegated command ownership, native uncertainty, maintenance, exact executor shutdown and campaign disposal |
| Verification evidence | Native result/capture/context observations with selective access and explicit provenance limits |

The content model is a strength: logical views do not become physical authority, equal logical dependencies may have different root-qualified physical mappings, and NotRequested differs from requested empty or failed analysis. Independent cold admission and fresh final guarded reconciliation cover uncertainties that same-operation reuse cannot remove.

Outcome and lifecycle distinctions are also already represented, but are not consistently realized across composition. A failed operation, a joined task, a closed session, removed scratch, a confirmed remote effect and a discarded owned executor are different facts. Likewise, retained payload, borrowed immutable backing and temporary decode/validation workspace have different resource lifetimes.

The reviewed changes do not alter provider fact fidelity, graph universe, relation multiplicity, Summary transfer semantics, verdicts or evaluation inputs. Those are preservation constraints; this operational review is not a fresh validation of all corresponding algorithms.

| In-scope content | Provider/revision and fidelity | Coverage/identity and consumer boundary |
|---|---|---|
| Source syntax and typed assertions | Independent providers and captured inputs at the baseline's exact pins; observed/resolved distinctions unchanged | Source/role/provider attribution remains in completed contributions; normalization cannot treat unobserved facts as absent |
| Selected memberships and canonical families | Derived exact unions over declared immutable contributors; actual nominal validity still earned | Logical view identity is distinct from physical copy/incarnation and root-qualified correspondence; native admission/publication consume them |
| Summary and compact declarations | Derived finite results under the declared Behavioral model, or explicit Catalog NotRequested | Model/profile, exact inputs and partial/verdict meanings remain authoritative; shared container growth must not redefine them |

No new graph projection or transfer algorithm is selected here, so a per-analysis method inventory and a full served-claim trace would exceed the assessed boundary. The operational corrections must preserve those model-owned contracts.

## 3. What the last implementation got right

| Earlier finding / friction | Comparative judgment |
|---|---|
| completion-integrated#F01: retirement discovery | The two exact-state indexed head queries and bounded checked merge address the observed job-only scan/filter/sort. Fresh candidate/refetch, nomination invalidation and allowance stopping remain. No new discovery defect was established. |
| completion-integrated#F02: selected union | The repaired path earns actual nominal conflict/count validity before replacing the cumulative view. Do not return to a metadata-only count shortcut. F06 below concerns the physical cost of this correct check. |
| completion-integrated#F03: refusal versus finality | Store-level finality-only drainage and sticky operation diagnostics are the right distinction. F01 below finds the same problem still present in dispatchers. |
| completion-integrated#F04: retirement page | The scalar locked-read lowering respects the pinned engine restriction and keeps complete prechecks before batched mutations. Actual six-control and cold/original/cleanup receipts bab4b8 and 1069dd support its bounded correction. |
| completion-integrated#F05: coordinator progress | Co-polling pending unspawned producers during launch/freeze waits repairs the identified holder/FIFO progress problem. It does not make the completion gate's whole scope necessary. |
| completion-integrated#F06: overlapping retained closures | Streaming one inspected closure into unique canonical ingredients removes the simultaneous sum-of-closures retention. It preserves independently assured selectors and different physical realizations. |
| completion-integrated#F07: inventory lifetime | One ordered selector representation, scoped row decode/validation and charged retained typed state are substantive corrections. The final refinement passed compile/Clippy; its functional rerun was interrupted. F04/F05 below identify adjacent factories not covered by that control. |
| completion-integrated#F08: compact async read | Awaiting NativeBatches.next_async is appropriate inside the async producer. Genuine synchronous provider callbacks still need their separate blocking bridge. A successful enclosing Catalog exit remains unestablished. |
| Native Unknown and exact disposal | Local recovery correctly refused native Unknown; later whole-target disposal settled ownership without rewriting the original failed/interrupted results. R7 disposal324b00 passed. |
| Library diagnostics | Retaining DataFusion's existing top-consumer error made attribution more useful. The 507.1MiB inventory observation is a reservation, not measured heap/RSS. |

The earlier source acceptance was bounded. This review expands the examined failure/growth scenarios; it does not retroactively turn those focused corrections or receipts into failures.

## 4. Findings

Priority is consequence-based: P1 addresses invalid operational contracts or predictable supported-workload amplification; P2 addresses independent change/efficiency burdens. All corrections remain Proposed. This review owns these new, unscheduled findings until an implementation plan explicitly takes disposition; older IDs remain with their existing coordinator.

| ID | Cause | Priority / owner |
|---|---|---|
| [F01](#F01) | Dispatcher task results reintroduce operation/finality conflation | P1 — compiler-store dispatchers / Workspace |
| [F02](#F02) | Optional product capture uses mandatory poisoning admission | P1 — retained-product owner |
| [F03](#F03) | Shared charged mutation admits growth after mutation | P1 — model charged operations |
| [F04](#F04) | Construction/decode allowances outlive their backing | P1 — native completed factories |
| [F05](#F05) | Retained capabilities recreate full selector copies | P1 — installed-view / compiler mapping owners |
| [F06](#F06) | Changed cumulative unions rebuild and retain overlapping prefixes | P1 — native membership preparation |
| [F07](#F07) | Each selector reconstructs unchanged transitive assurance ingredients | P1 for overlapping growth — selected closure preparation |
| [F08](#F08) | Completion exclusion includes unrelated private/optional work | P2 — Workspace handoff |
| [F09](#F09) | Enclosing run lacks delegated host cleanup/log ownership | P1 — harness/run/fixture composition |
| [F10](#F10) | Selective discovery expands all historical runs into live views | P2 — managed-run inspection |

<a id="F01"></a>
### F01 — a joined dispatcher operation can still be reported as unconfirmed finality

**Diagnosis: source-established,2026-10-14. FP-02–05; DP-02/08/19; A2/A3 and G2.**

When a caller drops its acknowledgement receiver, a subsequently acknowledged operation refusal becomes the registered task's error in [native_calls.rs:74](../../../crates/lctx-compiler-store/src/native_calls.rs). Drainage records it as a native-call join failure and retains it for repeated drains at line164. The bridge has the same adaptation at [native_bridge.rs:124](../../../crates/lctx-compiler-store/src/native_bridge.rs), and folds the operation error together with the task join at line70.

[Workspace finality:133](../../../crates/cpg-core/src/workspace.rs) inserts both aggregate dispatcher results into Completion.failures before merging the store's actual finality. Its immutable signal reaches [fixture classification:355](../../../scripts/surrealdb_fixture.py), which correctly protects a completion containing failures. Thus whether the caller was still receiving determines whether an acknowledged rejection is confused with unsuccessful drainage.

**Corrective contract.** Dispatchers own two results: actual task/thread terminality and retained operation diagnostics. Join accepted work and report genuine panic/join/remote uncertainty through finality. Preserve late refusals as the caller's sticky operation outcome, appended after signal settlement, following the store's existing drain_finality_report/append_operation_failures separation. The fixture's conservative classification should remain.

Preserve nested Completion facts when splitting reports: actual local Outstanding, remote Unknown, storage outcomes and committed effects remain attached to their authorities. A joined dispatcher task does not establish cleanup of every owner inside its operation. Only acknowledged operation diagnostics cease impersonating join failure; a wrapped completion must not be flattened into a harmless error.

A single small dispatcher completion carrier or ordinary paired reporting operations can serve NativeCalls and NativeBridge. Merely filtering errors in Workspace relocates interpretation and risks dropping real failed joins. Ignoring wrapped Resource errors is also unsound because a resource primary can coexist with uncertain cleanup.

**Closure.** Cancel/drop the receiver, then return an acknowledged refusal and complete actual drainage: the operation remains failed, immutable finality is clean, and no task is lost. Pair it with a panic/failed join and genuine Unknown that remain protected. The existing cancelled-caller test preserves the error but does not establish this separation. This is a residual of completion-integrated#F03, not a reason to weaken it.

<a id="F02"></a>
### F02 — optional product capture can cancel otherwise valid required compilation

**Diagnosis: source-established,2026-10-14. FP-02–04; DP-02/08/18/19; A2/A3 and G2.**

[Workspace.retain_product:370](../../../crates/cpg-core/src/workspace_products.rs) promises to decline optional capture on Resource/Limit while keeping the completed producer. It first submits retain_product_identity through mandatory native_call. [That dispatcher:74](../../../crates/lctx-compiler-store/src/native_calls.rs) fails/cancels on every error; [retention admission:2739](../../../crates/lctx-compiler-store/src/compiler.rs) also uses a poison-on-error mutation lease.

Even refusal of the local request-gate reservation can therefore poison admission and be wrapped as SharedCause. The fallback's direct error match misses that wrapper; cancellation would invalidate handoff even if the match succeeded. Historical Summary1e738a failed within native_product_retention with a resource primary. That localizes the observed failure, without proving that every retention failure is safely optional.

**Corrective contract.** Required producer completion and optional reuse capture are distinct operations. Prefer explicit retained/declined preparation before entering mandatory effects. A decline must leave required views usable and account for all submitted ownership. Once native effects begin, uncertain or unsettled failure stays fatal/protected unless a separate optional owner actually settles it. A successful empty result is not a decline.

Keeping retention mandatory is a simpler alternative only if the product explicitly requires it and the optional fallback is removed. That is a less suitable target for disposable reuse preparation. A broadly nonpoisoning mutation flag is not sufficient: it must specify which effects may already exist and who settles them.

**Closure.** After successful required production, force optional preparation refusal and observe usable current views plus clean drainage. Pair with post-effect/Unknown refusal that still prevents publication. The current translation-only test does not exercise the real fallback. F01's outcome separation helps report this correctly but cannot by itself make retention optional.

<a id="F03"></a>
### F03 — the shared charged-map operation grows state before it asks for admission

**Diagnosis: source-established,2026-10-14. DP-03/08/20; FP-04/05; resource-admission contract violation.**

[ChargedMap:79](../../../crates/lctx-model/src/domain/charged.rs) promises reserve-before-growth. Its update operation, at lines113–137, executes an unrestricted mutation closure, measures the resulting heap and only then grows StateCharge. Production normalization pushes sequence members through this path at [relation_normalization.rs:459](../../../crates/lctx-model/src/domain/normalized/relation_normalization.rs); [Summary:1277](../../../crates/lctx-model/src/domain/execution/summary_production.rs) inserts branch-set entries.

A push at vector capacity reallocates before the shared pool can refuse it. On refusal the container has already changed until its owner discards it. Final heap delta also cannot bound an arbitrary closure's temporary allocations. This establishes an admission-contract defect, not publication of an invalid semantic graph or measured process OOM.

**Corrective contract.** Put pre-admitted nested growth with the shared charged owner. Use capacity-aware append/set-insert operations or explicit admitted mutation allowances; retain useful no-growth replacement, duplicate insertion, removal and shrinking without charging a full replacement. An unconstrained mutation callback with after-the-fact measurement cannot provide the current guarantee.

Do not clone values for rollback or make every normalization/kernel caller independently reproduce container accounting. Those approaches create another allocation and distribute the same policy.

**Closure.** Use a refusing pool and a mutation/allocation witness to show refusal precedes growth, old state and charge remain coherent, and no-growth updates remain usable. Inspect representative high-degree vector/set callers after migration. No database or full library run is required to establish this ordering.

<a id="F04"></a>
### F04 — temporary native construction and decode allowances remain reserved after the work ends

**Diagnosis: source-established,2026-10-14. FP-07; DP-10/19/20; H17; A4.**

[Boundary construction:395](../../../crates/lctx-native/src/completed_state/earning.rs) charges builder sets, copied contributors and selector references. It independently reconstructs InstalledViewInventory, then transfers the builder charge into NativeCompletedState at line692 while those local containers leave scope. The portion covering dead builder backing remains reserved; the retained inventory has separate backing/charges. Builder copies and receiving copies also coexist at the transition.

The closest completion-integrated#F07 analogue is [audit_attempt_bindings:728](../../../crates/lctx-native/src/completed_state/earning.rs): it accumulates native-row decode allowance for every binding while retaining typed bindings. Prior rows' decode workspace is not still needed. Frozen completion, lifecycle physical completion and published completion/admission factories show related retained decode-style allowances at [completed_state.rs:456,1404,1624](../../../crates/lctx-native/src/completed_state.rs).

The builder's fixed per-selection allowance also precedes deep mapping clones without following their length. An allowance covering the caller's original mapping backing does not automatically cover its clone.

**Corrective contract.** Native factories distinguish current scratch, retained owned backing and already charged borrowed backing. Admit before allocation, retain through the actual borrower, and transfer only retained-state allowance into returned capabilities. Reuse the existing StateCharge release mechanism and scoped reservation idiom; no second pool is needed.

The independent receiving pass must remain. Either consume its checked typed result without retaining redundant builder copies, or scope builder backing explicitly until all required comparisons/effects finish, then release it. Releasing everything at EOF would be wrong if an expected row or prepared run remains borrowed.

**Closure.** Keep a completed owner alive after construction and demonstrate that builder/per-row decode scratch is gone while retained payload remains charged until its last borrower. Cover frozen binding audit and each retained factory, preserving late malformed/missing-child refusal, full correspondence, holds and final CAS. completion-integrated#F07's inventory-only control cannot close those routes. Magnitude and historical dominant-cause attribution remain unmeasured.

<a id="F05"></a>
### F05 — retained view capabilities recreate the selector duplication removed from inventory

**Diagnosis: source-established,2026-10-14. FP-07; DP-10/16/20; A4 and pre-admission contract gaps.**

[HeldInstalledView:147](../../../crates/lctx-native/src/installed_view.rs) stores both a retained InstalledViewCompletion and a SelectedViewDescriptor. [from_retained:21](../../../crates/lctx-native/src/installed_view/earned.rs) clones the full view and mapping array. Published/frozen/physical factories first clone a selector from the protected inventory, then [retained:279](../../../crates/lctx-native/src/installed_view/live.rs) stores both forms. Inventory, completion and descriptor therefore coexist as typed copies.

The same factory validates and constructs completion.value before its new charge. Compiler-store additionally copies mapping arrays into its registry when registering completed inventories; [remember_view_mapping:2055](../../../crates/lctx-compiler-store/src/compiler.rs) owns that copy. Some live mutable registry is needed, but copying an already retained immutable inventory into it needs a distinct consumer purpose.

**Corrective contract.** Expose a borrowed selector view or share one immutable typed payload under the actual capability/inventory owner. Keep expected-envelope bytes and native pins: those provide distinct integrity/protection, unlike another typed mapping clone. Resolve mappings from retained inventory where that is the active source; preserve the distinct live-selection path.

Sharing by logical view alone is invalid. Two roots may select different physical suffix copies/incarnations. A shared payload must bind the actual inventory/capability and exact selector. Independent receivers still earn their own authority.

**Closure.** Exercise all retained factory variants and the internal earned-selector case. Demonstrate one typed payload per actual source/capability, admitted scratch for genuine serialization or clones, unchanged missing/current/ABA refusal and backing retained after receiver/last-owner changes. This in-memory correction does not require changing flat semantic v4 or a persisted codec.

<a id="F06"></a>
### F06 — exact cumulative unions retain overlapping prefixes without demand retirement

**Diagnosis: source-established,2026-10-14. FP-07; DP-09/10/19/20; H5/12/14/17/19; A4.**

[Contribution completion:5777](../../../crates/lctx-compiler-store/src/compiler.rs) calls count_completed_memberships for each changed multi-contributor aggregate. [Preparation:7797](../../../crates/lctx-compiler-store/src/compiler.rs) reads the selected contributors, builds a sorted run and retains its completed shared future under the exact view identity. Successful entries have no inspected demand-retirement path. Finality clears several other registries but not this membership registry.

[PreparedCandidates:1676](../../../crates/lctx-native/src/ordered_rows.rs) owns immutable memory/spill backing through Arc. Consequently producers adding a shared vocabulary create overlapping prepared prefixes. Exact-set caching avoids repeating one identical set; it does not remove scanning/sorting and retention across changing sets. Earlier consumers may legitimately need an older prefix, so simple latest-only eviction is not equivalent.

**Corrective contract.** Keep actual union conflict/count validation, but share preparation at a useful immutable grain and retire registry ownership by real remaining/running demand. Existing borrowers keep their backing. Per-contributor ordered fragments with bounded merge are a suitable first direction; an incrementally checked current union or demand-only assembly are alternatives.

Releasing obsolete registry entries addresses live backing but not repeated scans/sorts. Both obligations need attention. Demand-only assembly is a larger choice because it changes the eager cumulative handoff seam; it is not silently selected by this finding. No portable proof cache or generic incremental framework is required.

**Closure.** Several shared-relation producers with disjoint, equal-overlap and conflicting nominal content must retain exact demanded counts/rows, earlier frozen views and clean failure drainage. Observe ingredient preparation and obsolete-run release structurally. Preserve distinct physical mappings. Correctness of completion-integrated#F02's full union check remains a strength.

<a id="F07"></a>
### F07 — each selected root reconstructs unchanged transitive assurance ingredients

**Diagnosis: source-established repeated preparation,2026-10-14; representation remedy remains a separate choice. FP-07; DP-10; A4.**

[Physical completion:2519](../../../crates/lctx-compiler-store/src/compiler.rs) discovers and retains a full mapping vector for each selected root and discovered dependency. Product retention repeats this route. Each mapping traversal has its own seen set. [Boundary assurance:413](../../../crates/lctx-native/src/completed_state/earning.rs) then creates an InspectedClosure for each selector; [audit traversal:2161](../../../crates/lctx-native/src/installed_view/audit.rs) reconstructs dependency maps and cloned descriptors.

Native observations and some typed contributor data are already shared. This is not a claim that every repeated element causes a new database query. A legitimate chain or reconvergent diamond still repeatedly traverses and constructs local suffix state.

Under the current physical codec, the full ordered mapping array is a required selector output. Its necessary emission/hash cost must be distinguished from repeated assurance preparation. completion-integrated#F06's streaming correction removes simultaneous closures, not this repeated work.

**Corrective contract.** First share operation-local direct dependency/assurance ingredients under the exact protected source, physical mapping context and resource pool. Consume borrowed checked descriptors instead of rebuilding owned recovery closures merely to inspect them again. Each root retains its mapping consistency, membership content and final-state obligations.

Dependency identity, relation and cardinality semantics stay with model-owned RecoveryTraversal or its owned successor. Shared native observations must not create another interpreter of those semantics.

If required flat arrays remain materially burdensome, physical mapping fragments/factorization are a separate codec/consumer decision. They do not inherently require a Merkle/DAG semantic identity or changing flat six-family v4. Equal logical suffixes with different physical copies must remain separate. The simpler flat-output/shared-preparation alternative is preferred before that larger migration.

**Closure.** Chain/diamond inputs preserve exact flat outputs and roots, differing physical suffix selections and late unbound corruption refusal while compatible immutable ingredients are prepared once per operation. Native receiving/cold authority remains independent. The actual best physical factorization, if any, is further investigation rather than an adopted design.

<a id="F08"></a>
### F08 — the completion gate includes another producer's optional private closure work

**Diagnosis: source-established,2026-10-14. FP-01/03/06/07; H21–23; A1/A3/A4.**

[Workspace:2989](../../../crates/cpg-core/src/workspace.rs) holds one completion gate through contribution completion, descriptor acquisition, optional retained-product construction, output binding and visible handoff. The retained boundary includes dependency discovery and full content preparation. Binding and prefix freezing acquire the same gate.

Co-polling now lets the holder make progress. It does not remove head-of-line waiting by disjoint ready producers or selected-input consumers behind another producer's optional retained closure. F06's repeated aggregate work increases that serialized interval.

**Corrective contract.** Prepare private immutable singleton/retention work outside exclusion where its premises permit. Serialize the actual multi-output handoff/current-set commit, or use affected-relation ownership and checked current-set comparison for work depending on mutable cumulative state. Preserve previous views on refusal, coherent multi-output visibility and no acceptance after cancellation/failure.

Simply dropping the gate is not equivalent. Its mutable-current-set protection is real; the question is which work must inhabit that interval. F02's optional-outcome boundary should be settled before moving optional capture into independent work.

**Closure.** Hold one optional preparation pending while a disjoint producer completes/binds; also demonstrate overlapping-contributor exclusion, atomic visible results and cancellation drainage. Existing holder/FIFO controls establish progress, not this locality. No new output-fidelity failure is asserted by this finding.

<a id="F09"></a>
### F09 — a managed run can release capture ownership while a delegated fixture compiler is alive

**Diagnosis: source-established with historical operational evidence,2026-10-14. FP-03–06; DP-19/20; A2/A3 and G5.**

[spawn_group:106](../../../scripts/harness.py) starts a new session. The run owns its direct group; [fixture launch:1905](../../../scripts/surrealdb_fixture.py) creates another. [Run cancellation:560](../../../scripts/runs.py) signals the direct group, and status/protection at line205 observes that group. [Run storage completion:393](../../../scripts/runs.py) does not carry the fixture's delegated host obligation.

Run bd7117 was cancelled with direct cleanup Confirmed at19:10:58UTC. At19:12:17 the session still observed Cargo/sccache in fixture group1084760. Exact recovery initially refused unresolved command cleanup; a separate authenticated group observation reported unavailable leader identity. The group drained naturally and b847e8 recovery passed at19:13:40. Final cancellation repeated the pattern: three local recovery attempts failed before later retries succeeded. Native-Unknown refusal in01b376 was separate and correct.

The outer receipt is scoped truth, but the enclosing workflow still has a compiler and inherited log writer. Completing its log/receipt lifecycle then creates an actual retention boundary gap, not merely a confusing display.

**Corrective contract.** Before delegated work becomes usable, bind its existing owner to the parent run. Parent cancellation requests that owner to stop; aggregate host cleanup and run/log release require all registered delegated host commands to settle. Publish direct-group cleanup, delegated host cleanup and native completion separately. Preserve authenticated identities, one-writer receipts, retryable recovery and original outcomes. Persistent native service executors are not disposable run children.

Prefer a small owner-reference/request composition through existing harness/run/fixture owners. A single supported supervisor arrangement is another alternative. Linux cgroup containment is worth qualifying on this machine, but cannot replace native finality or the semantic owner relationship; §6 compares it.

The exact signal-level cause is not established. Repeated TERM, interruption during guard cleanup and grace-period interaction are hypotheses, not findings. Longer grace alone does not repair the missing composition.

**Closure.** Inert nested commands cover cancellation with a live leader, leader exit with descendants alive, repeated TERM and receipt-publication failure. Parent cleanup/log release remains protected until actual delegated drainage; reused/foreign identities are never signalled. A later focused fixture case preserves separate native Unknown. Existing exact recovery/disposal remains the safe fallback.

<a id="F10"></a>
### F10 — run discovery performs history-wide live expansion for a small lookup

**Diagnosis: source-established with historical output friction,2026-10-14. FP-07; DP-10/16/21; H15/26; A4.**

[runs list:733](../../../scripts/runs.py) builds a detailed view for every historical run. [view:205](../../../scripts/runs.py) independently computes group observation, protection and survivors; applicable routes rescan processes. The parser has no recent limit, active/unresolved selector, label filter or compact continuation.

The agent's18:17UTC list invocation requested the broadest output and received an enormous truncated history. That choice was poor, but there was no supported bounded discovery route. Deleting history would remove evidence instead of correcting the access pattern.

**Corrective contract.** Make ordinary discovery a compact bounded recent page, with explicit active/unresolved and label selection plus completeness/continuation information. Select cheaply before live hydration where the predicate allows; reuse one request-scoped process observation. Keep selected detailed status and an explicit complete-history route. No second persistent run database is required initially.

An active-state search may need to examine all metadata records. State that scope honestly: bounded output does not guarantee bounded examined work. Unknown/corrupt records must remain discoverable.

**Closure.** Filesystem-backed controls show recent discovery avoids unrelated old live observations, continuation obeys declared consistency semantics, malformed/unresolved records remain reachable, and detailed status retains its supported facts. Access counts are sufficient to establish removed work; no speed ratio is claimed.

## 5. Further Codex improvements and things that are not tool defects

These recommendations are Proposed. They have concrete consumers but are not additional proven production failures.

<a id="O01"></a>
**O01 — opt-in evidence capture for an exact ad hoc command.** Several actual controls used managed run → fixture → bare Nextest, without verification-native sidecars. [CaptureOwner](../../../scripts/verify_evidence/capture.py) already provides the adapter boundary. Expose a thin optional arbitrary-command capture route over those adapters, preserving exact arguments, fixture ownership and native outcomes. This would make phase/case evidence available without forcing a permanent family declaration. Keep ordinary bare tools first-class. Validate one exact Nextest command and interrupted capture; do not invent another runner or reinterpret a passing body as successful fixture cleanup.

<a id="O02"></a>
**O02 — explain waits and resource refusals at existing owner boundaries.** Show compilation/lock waiting separately from test execution, and associate coarse phases with the actual producer/owner and invocation. Preserve phase IDs through async wrappers and correlate begin/terminal within the same process/test invocation; log order alone misattributed compact-owner progress in the investigated stall. A dropped phase observation remains incomplete, never a fabricated terminal result. A known pending native batch, completion gate or producer join can report that wait without claiming the historical closed wait cycle was observed. Use existing phase/progress and verification-event mechanisms; no per-row event stream or universal wait graph.

DataFusion55.1's existing top-consumer report was decisive. Preserve its diagnostic once at the boundary with typed owner/requested/used/limit values, while keeping reservation totals distinct from actual heap. Its public report_top returns formatted text; do not advertise a structured consumer-vector API that this pin does not expose. Phase logging added to mutation fixtures is Implemented, but its effectiveness is still untested because the final reruns were cancelled.

<a id="O03"></a>
**O03 — keep current work discoverable without reconstructing a session chronology.** The sole Catalog coordinator contains many successive “current/latest” narratives. They preserve useful evidence but make the next action and applicable revision hard to locate. Keep its current checkpoint short: current source, active owners, exact open obligations, source-qualified receipts and next action. Put dated causal explanation in the review/evidence that consumes it, and use Git for superseded narrative after surviving obligations transfer. Preserve the same disposition owner rather than adding another task ledger.

Batch coherent source changes before rebuilding their consumers; distinguish superseded commands, build completion and test execution. Existing [source observations:93](../../../scripts/verify_evidence/context.py) and [post-command executable observation:143](../../../scripts/verify_evidence/capture.py) correctly disclaim exact build closure. An opt-in execution handoff may record the selected executable and matching observations; do not turn a dirty-tree digest or later file hash into proof of executed source. Cold compilation after cargo clean is expected, not evidence of broken caching. The ENOSPC receipt does not justify a new purge policy.

<a id="O04"></a>
**O04 — align role configuration with the operator's one-model Codex workflow.** Repository configuration still names different models for coordinator, worker roles and code mapping. The active parent and all three reviewers in this review were observed as gpt-6.1-sol; the project coordinator default names gpt-6-astra. A config declaration is therefore not the current runtime observation.

Keep functional role briefs and independent judgment, but remove model-routing policy that competes with the operator's selected model. Prefer one operator-owned model choice, inherited by roles, rather than duplicated role pins; verify the effective setting in a fresh session. Official documentation establishes the model/default/role override order, not the model actually used by a particular thread. [Codex subagent configuration](https://learn.chatgpt.com/docs/agent-configuration/subagents) supports inheritance when competing defaults/overrides are absent. This is a configuration fit recommendation, not a comparative model-quality claim.

The earlier send_message/followup_task confusion is agent workflow misuse: the supplied runtime distinguishes a message from activation of an idle agent. Encrypted message payloads and counts cannot establish the exact delay. Use the existing task-activation operation and inspect returned status; add only a concise route in the existing role contract if recurring. No repository scheduler is warranted.

<a id="O05"></a>
**O05 — use existing command discovery and narrowly improve error hints.** service dev and service status --json were my incorrect spellings; parser/help routes already existed and refused before effects. Prefer the actual owning help/preview at the point of use. A targeted parser hint pointing service dev users to surreal-dev is reasonable; an alias forest or new CLI registry is not supported by these incidents. The pre-existing ADR reference/index failures are documentation defects to carry forward, not reasons to run unrelated quality remediation during this review.

## 6. Shared corrective approach and library fit

The common principle is that an established property or obligation travels with its actual owner. The necessary constructs remain different because their meanings differ:

| Category | Small shared capability | Existing mechanism / retained boundary |
|---|---|---|
| Operation versus finality | Joined ownership report separate from sticky operation diagnostics | Compose the store's distinction while retaining nested Outstanding/Unknown, storage and committed-effect metadata |
| Resource lifetime | Pre-admitted owned payload, borrowed payload and scoped scratch | StateCharge/ResourceBudget and DataFusion MemoryReservation; release/transfer at the actual backing lifetime |
| Repeated immutable preparation | Exact ordered ingredients and demand-owned borrowers | Existing PreparedRows/PreparedCandidates, budgeted cursors, spill and selected-input declarations |
| Optional work | Explicit decline before mandatory effect admission, or separately settled optional owner | Existing poisoning/uncertainty rules remain for accepted mandatory effects |
| Host execution | Parent run references delegated command owner and composes cleanup/capture release | Harness identities, run/fixture receipts and service-owned native finality |
| Inspection | Cheap compact selection followed by explicit detailed hydration | Existing run records and verification evidence adapters; one request-scoped process snapshot |

A shared struct is useful only where it removes an independently reinterpreted contract. For example, one result carrier can serve both dispatchers; one charged nested append operation can serve normalization and Summary. Resource scratch and host cleanup should not share a universal lifecycle enum merely because both eventually release something.

**DataFusion/Arrow.** Cargo.lock and the selected skill match DataFusion55.1.0/Arrow59.3.0. Inspected MemoryReservation supports fallible growth, shrinking and splitting; StateCharge already supports release. Native immutable ordered runs retain backing through Arc and budgeted cursors. These mechanisms fit the proposed local corrections. DataFrame caching or moving native descriptors into Arrow solely to obtain buffer accounting would materialize another representation without solving its ownership. Arrow buffer accounting does not automatically cover native Value or typed B-tree metadata.

Allocator-level accounting is a different alternative to explicit owned allowances. It would need to cover nested native/Rust payloads, concurrent task ownership and borrowed backing, rather than only Arrow buffers or one thread's allocations. This review establishes no requirement for a hard whole-process allocator limit. Applying shared pre-admitted growth operations and fixing backing lifetimes is the smaller correction; an allocation profiler may later settle a genuine heap-attribution question without becoming the runtime admission mechanism.

**SurrealDB.** The inspected code and historical actual planner/page controls support indexed discovery, full transactional prechecks and grouped mutations. The failed array FOR UPDATE attempt was a real qualification mistake; the current scalar locked lowering is appropriate for the selected patched3.3 engine. No further removal of locks, higher deadline or general backend replacement follows. This review ran no engine queries.

**Linux/systemd.** Installed systemd is255.4, with unified cgroups and existing repository user-service integration. Its version255 documentation provides [ExitType=cgroup](https://raw.githubusercontent.com/systemd/systemd/v255/man/systemd.service.xml), which observes surviving unit processes, and [KillMode=control-group](https://raw.githubusercontent.com/systemd/systemd/v255/man/systemd.kill.xml), which stops the unit's remaining processes. Local installed manuals agree. Context7's261.3 material was discovery only; it was not silently treated as the installed contract.

A transient user service could contain nested host descendants despite new process sessions. This is **Interface-checked, not Tested** for the proposed run integration. It must preserve exact exits, environment, log ownership and cancellation, and must exclude separately owned native services and shared cache daemons. An empty cgroup cannot establish native request finality. Qualify an inert nested-process case before choosing containment over the smaller explicit-owner composition. No transient unit was created by this review.

## 7. Change scenarios, priorities and uncertainty

| Scenario / change kind | Owning contract and expected locality | What must remain true |
|---|---|---|
| Add a producer sharing a vocabulary — domain extension/composition | Stage declarations and exact union preparation, F06/F08 | No nominal conflict shortcut; independent outputs need not wait for optional capture |
| Add a large binding/selector — new instance/growth | Native retained/scratch factories, F03–F05 | Complete descriptor/mapping/incarnation and independent receiving; only live backing stays charged |
| Reconvergent chain with differing physical suffix copies — contextual binding | Selected assurance ingredients, F07 | Root-qualified choices remain distinct; flat outputs retain current meaning |
| Caller cancellation followed by acknowledged refusal — failure | Dispatcher outcome/finality, F01/F02 | Operation remains failed; actually joined ownership is not relabelled Unknown |
| Run delegates a new supervised command — operational domain extension | Parent/child owner composition, F09 | Host and native outcomes independent; capture protected through all writers |
| Replace direct PGID containment with a transient unit — mechanism substitution | Harness adapter and existing owner receipts | No foreign/shared service signalling or loss of exit/capture/native obligations |
| Retain more historical runs — growth | Selective discovery, F10 | Compact inspection, explicit completeness and recoverable malformed/unknown records |

Correct F01/F02/F03 and F09's operational contracts first in consequence priority. F04/F05 and F06/F07 address supported-workload amplification independently of whether one fixture happens to pass. F08 is important after the optional and mutable handoff premises are explicit. F10 and O01–O05 reduce development burden without becoming prerequisites to every production fix.

Prerequisite order is not the priority list: the shared charged-growth contract enables safe consumer migration; the optional preparation/effect split enables narrower exclusion; delegated-host ownership enables aggregate status and storage-release changes. Native finality must not be relaxed to compensate for missing host composition.

The following questions remain genuinely open:

- Which exact signal/cleanup interruption produced each escaped fixture group? Existing receipts establish the composition gap, not that low-level trigger.
- For overlapping selectors, how much can shared direct assurance remove while keeping required flat outputs, and would physical factorization justify a codec migration?
- Which complete demanded unions benefit from retained backing after their current consumer ends? Consumer demand, not an arbitrary cache size, must settle release.
- Does a transient user execution unit integrate more simply than explicit delegated-owner references on this machine, including cache-daemon and direct fixture behavior?
- What functional evidence do final-source Summary/Catalog/cold and the pinned pilot establish after remediation? Their current incomplete state remains with the Catalog coordinator.

Source inspection establishes the listed ordering/lifetime/reconstruction defects. Local probes were permitted but not needed to assert those diagnoses; the unresolved alternative mechanisms are explicitly not reported as executed. Remedy controls above are follow-up work, not checks performed by this review.

**Disposition transferred 2026-10-14:** the [Catalog completion coordinator §6](../../plans/catalog-completion-plan_2026-10-14.md#6-sole-disposition-and-current-checkpoint)
now owns scheduled F01–F10/O01–O05 and their closure obligations alongside inherited Catalog scope.
Its §3 records the operator-confirmed RC01/RC02/RC05 directions and conditional RC03/RC04 permissions.
This navigation/disposition transfer changes no dated diagnosis, evidence or verdict and establishes
no remediation or runtime closure.

## 8. Rule impacts

These are proposed changes for subsequent plan creation; this review changes no rule or older disposition.

| ID | Current rule / owner | Proposed change and dependent recommendation | If retained |
|---|---|---|---|
| <a id="RC01"></a>RC01 | DESIGN §1.2 / ADR-0142 managed direct command cleanup and storage completion | Extend composed run ownership to registered delegated host commands; preserve direct/native axes, F09 | Keep separate owner receipts but expose an enclosing command owner that cannot complete before delegated host cleanup; status-only wording cannot fix log release |
| <a id="RC02"></a>RC02 | Managed-run CLI help/README: list renders full historical views | Make default discovery compact/bounded with explicit complete history and selective routes, F10 | Add an explicit bounded discovery route; the current all-history operation remains unsuitable for routine lookup |
| <a id="RC03"></a>RC03, conditional | ADR-0166 and semantic/storage owners' physical selector codec/envelope | Only if F07 chooses persisted mapping factorization: deliberately migrate codec/schema and all receiving/cold consumers; preserve flat semantic v4 | Keep flat output; share compatible assurance ingredients and accept necessary emission/hash work |
| <a id="RC04"></a>RC04, conditional | Semantic §15.1 and Catalog compiler/native plans' eager cumulative handoff | Only if F06/F08 chooses demand-only aggregate assembly rather than factoring current preparation: redefine demanded selection/visibility timing explicitly | Preserve eager current views, but factor immutable preparation and narrow only private/optional work outside commit exclusion |
| <a id="RC05"></a>RC05 | .codex/config.toml, .codex/agents and ADR-0109/shared role routing | Replace competing per-role/model defaults with the operator's single selected Codex model inheritance, O04; retain role responsibilities | Explicitly override competing models at every supported spawn and disclose configuration divergence; do not claim automatic inheritance |

No rule change is needed to separate dispatcher finality, make the advertised optional fallback real, correct charged mutation ordering, scope scratch, or share an in-memory immutable selector. Those implement the current contracts more faithfully. Independent cold admission, actual pins/guards, complete EOF/inventory and conservative Unknown classification are preservation constraints, not rules this review proposes weakening.

## 9. Architectural judgments and decision

| Judgment | Verdict and evidence |
|---|---|
| A1 Localize change | Violated in independent completion composition: optional private work occupies the shared handoff gate, F08. Most owners remain identifiable and suitable for local correction. |
| A2 Encode domain meaning | Violated in operational outcome/admission and enclosing host ownership, F01–F03/F09; exact content/physical identity distinctions are a strength. |
| A3 Compose capabilities | Violated where dispatchers, optional capture and nested cleanup lose the contracts supplied by their individual owners, F01/F02/F08/F09. |
| A4 Fit execution | Violated for dead/duplicate backing, repeated prefixes/transitive preparation, broad exclusion and history-wide discovery, F04–F08/F10. No measured performance claim. |

| Gate | Scoped verdict |
|---|---|
| G1 Authority | Pass for inspected semantic and individual receipt owners; retain their one-writer and identity boundaries |
| G2 Semantic fidelity | Fail for operation/refusal/finality and optional-outcome composition, F01/F02; no new product-fact relabelling established |
| G3 Validity | Pass for inspected actual selected-union/current native enforcement; full semantic validation was not recertified |
| G4 Hidden behavior | Pass within inspected explicit effect routes; optional capture's misleading contract is judged under G2/A2/A3 |
| G5 Consistency/recovery | Fail for enclosing delegated-host/capture lifetime, F09; F03/F05 independently violate admission contracts without an established inconsistent-commit/retry/truncation failure. Genuine native Unknown protection remains correct |
| G6 Transformation/reuse | Pass within inspected exact protected correspondence and independent-admission boundaries; no enclosing content requalification |
| G7 Truthful claims | Pass at the explicitly bounded evidence strength; no whole-source executed-binary, RSS, speed or enclosing acceptance claim |
| G8 Library leverage | No demonstrated reimplementation violation; existing library/native carriers support the bounded remedies. Proposed OS containment is not claimed integrated |
| CI-G1 Fidelity | n.a. to the new operational findings; provider/analysis fidelity recertification excluded, with no reinterpretation recommended |
| CI-G2 Evidence closure | n.a. to the new operational findings; preserved as a constraint, full serving/evidence journey not_run |
| CI-G3 Evaluation integrity | n.a. to the operational inquiry; no evaluation/reference inputs used or changed, protected evaluation not run |

FP-01/03/04/05/06 are violated in the specific composition/local-reasoning scenarios above, not by every component. FP-02 fails at the advertised optional-outcome/dispatcher contracts. FP-07 fails at the concrete amplification paths. Applicable supporting rules are attached to findings; no exhaustive heuristic score or trade-off permits a fidelity/ownership gap.

**Production decision: Revise. Tooling decision: Revise at F09/F10; pursue the bounded opportunities without turning them into a mandatory workflow. Overall decision: Revise.**

This is a broader target judgment than the previous integrated source assessment. It credits its repairs while identifying additional obligations. Existing Catalog runtime exits, original failures/interrupted Unknowns, and older SA-F02/BC3/SM8 boundaries remain unchanged. Review publication authorizes no remediation, new native campaign, model/configuration change, data deletion or operator adoption.
