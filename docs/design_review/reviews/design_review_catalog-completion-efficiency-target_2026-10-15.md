# Catalog completion: efficiency and execution architecture

**Design / target review — Revise, 2026-10-15.**

The architecture has useful foundations for the pinned API/evidence Catalog: explicit provider
and model authority, immutable completed inputs, compact Catalog owners, independently earned
receiving admission, unselected publication, exact physical correspondence and evidence-linked
native reads. These are preservation requirements for the corrections below.

The assembled implementation nevertheless retains avoidable work in consequential places.
Native write batches repeat shared parent and incidence work inside the database; disjoint
producers meet an attempt-wide conflict address; retirement pages write a common backup gate;
and the Behavioral path repeats selection, hydration and correspondence preparation at finer
grains than their immutable dependencies require. Application resource admission also begins
after substantial transport allocations have already happened.

The development loop has a demonstrated cancellation defect. A completed delegated registration
can suppress cancellation of a live registration with the same owner. Exact run lookup prepares
the historical directory inventory before resolving a known full ID. Native recovery retries
create another liveness problem by discarding cancellation ownership and continuing to issue
transactions after conclusive conflicts.

These findings require revision. They do not establish that SurrealDB is the wrong store, that
database execution should replace every Rust kernel, or that a historical timeout had one
dominant cause. The strongest corrections are narrower: operation-shaped native lowering,
appropriate conflict domains, preparation shared under its actual immutable premises, and
cancellation/resource ownership carried through the complete operation.

## 1. Review contract and evidence boundary

| Field | Assessment |
|---|---|
| Subject | All implemented and inherited code serving the [Catalog completion coordinator](../../plans/catalog-completion-plan_2026-10-14.md) and its production, development and acceptance companions |
| Baseline | `24b0d08b6f1ed39a43b677ebc684806e07563aaf` |
| Standard | [Core/template 3.3, heuristics 1.0 H1–H28, code-intelligence 1.5 and repository binding](../design_principles/standard.toml) |
| Tier / purpose | Design / target |
| Reviewer | Fresh independent design reviewer, with coordinator-supplied source mapping, library research and historical observations; root reconciled and published the assessment |
| Functional target | Actual pinned providers → Facts → Normalized → Analysis/Catalog → independent admission → coherent unselected publication → evidence-linked native queries and original evidence |
| Additional supported behavior | Explicit Behavioral analysis, including finite and recursive Summary; it is not a Catalog prerequisite |
| Exclusions | Embedding inference, MCP-submitted runtime queries, operator activation and extensive environment qualification |
| Workload premise | Full pinned releases/corpora; many semantic families; shared and overlapping dependencies; high-degree relationships; concurrent producers/readers/cleanup; retained run histories; interrupted and uncertain operations |
| Resource premise | Existing operation/attempt budgets, bounded transfer, useful spill and normal concurrency; limits must protect a credible execution route |
| Method and effects | Source, exact library contracts and existing results first; one pure branch probe and historical capture extraction; no new native workload or performance campaign |
| Maturity | Diagnoses are Implemented/source-inspected unless marked Tested. Replacement designs remain Proposed or Interface-checked. No measured speedup or assembled qualification is claimed |

The criteria are the loaded standard and functional target. ADRs, pins, current policies,
retained D01–D13/C01–C11 decisions and previous reviews describe the current design; they may be
changed where the target warrants it. Rules governing conduct, preservation and publication
were respected. Concurrent diagnostics evidence, its index edits and the supplied
`docs/surrealdb_technical_reference.md` were preserved and are not adopted as authority.

### Coverage

| Responsibility | Decisive source examined |
|---|---|
| Captured providers and cancellation | `cpg-extract/src/{bundle,capture,pyrefly_stage,ty_flow}.rs`, `cpg-flow`, compiler ingress/native-call ownership |
| Scheduling and handoff | `cpg-core/src/{compilation,workspace}.rs`, `workspace/membership_demand.rs`, compiler-store composition/preparation |
| Compact Catalog and reuse | `catalog_core.rs`, `catalog_products.rs`, model Catalog scope/preparation/value dependencies |
| Behavioral Base/Summary and analytics | `semantic_execution.rs`, `base_scope.rs`, model evaluation/Summary/discovery/residual and analytics kernels |
| Native control and recovery | `control.rs`, authorization, coordination, retry, schema functions, ingress and retirement |
| Assurance and independent receiving | Installed-view audit/earned/receiving paths, completed-state earning, selectors, compiler-store reference/semantic admission |
| Transport and finality | Actual vendored SDK/RPC and pinned server source, application native readers |
| Publication and evidence consumers | Publisher entrypoints and selected backup; native query preparation, original attribution and continuations |
| Development execution | Delegated stop, run resolution/listing, fixture, capture/context/finalization, validation runner, beta adapter and build owners |

This is not a line-by-line correctness certification of every model family, an exhaustive
library audit or release qualification. The [evidence index](../evidence/2026-10-15_catalog-completion-efficiency/README.md)
contains source inquiry boundaries, commands, capture hashes, contrary evidence and citation
ownership. The [native capability inquiry](../evidence/2026-10-15_catalog-completion-efficiency/library-capabilities.md)
qualifies SDK/server 3.3.0 and SurrealKit beta.1 separately.

## 2. Responsibilities and semantic authority

The principal dependency direction is sound:

```text
Pinned acquisition and provider adapters
                 ↓
      Model-owned attributed facts
                 ↓
Exact completed selections → semantic kernels
                 ↓
     Independent artifact admission
                 ↓
      Immutable native publication
                 ↓
 Pinned operations → evidence → original bytes
```

Native physical access, ownership and transport implement this meaning. They do not define
provider fidelity, decide that unknown means absent, or turn an old receipt into receiving authority.

| Owner | Governing responsibility | Consumer boundary |
|---|---|---|
| `lctx-model::domain` | Identities, facts, roles, coverage, operations, invariants, projections/outcomes | Adapters/orchestration consume declared meaning |
| `cpg-extract` / `cpg-flow` | Pinned observations and exact source/role association | Independent provider meanings and cancellation remain visible |
| `cpg-core` | Selected compilation, kernels and coherent current/frozen handoff | Readiness depends on actual completed inputs |
| `lctx-compiler-store` | Attempt-owned compilation and independent admission | Pending, completed and admitted authority stay distinct |
| `lctx-native` | Physical selection, guards, pins, transport, receiving/retirement | Serialized correspondence is data; actual ownership grants capability |
| Publisher / serving | Manifest publication, fixed-realization operations and evidence packets | Publication does not select an operator snapshot; originals/cursors retain their realization |
| Run/fixture/capture owners | Host execution and evidence-writer lifetime | Host cleanup, capture health and native finality stay independent |

### Fact and fidelity assessment

| Family | Established meaning | Coverage/identity and consumers |
|---|---|---|
| Ruff syntax/lexical | Extracted independent pinned Ruff observations | Captured source/provider/run, canonical occurrence and role → normalization/Catalog/evidence |
| Pyrefly typing | Native typing/resolution assertions in its own family | Qualified symbols/types, unresolved/competing assertions → signatures/callables/Catalog |
| ty runtime flow | Declared flow observations with source/role correspondence | Explicit provider/profile/model and coverage → explicit Behavioral operations |
| Normalized entities/callables | Derived interpretations over exact inputs | Model-owned identity, ambiguity and qualification → Catalog/analysis/serving |
| Catalog/options/originals | Source/effective contract and evidence inventory | Exact invocation/context and original attribution → Catalog operations/evidence |
| Summary/Behavioral | Bounded results relative to a runtime model | Proof, residual, exhaustion and verdict distinctions → explicit enrichment |
| Analytics | Governed communities, centrality and FCA/RCA | Projection/method/settings and partial outcomes → ranking/facets, not behavioral truth |

No new provider-relabeling, unknown-as-absence or heuristic-as-fact defect was established in
the inspected paths. This positive fidelity judgment does not settle execution fit.

## 3. Preserved contracts and strengths

**Completed state is an owned capability.** Specifications distinguish producer, profile, model,
implementation, configuration and ordered input roles. Exact contributor sets/counts remain
separate from compound physical copy/incarnation selectors. See
[model completed contracts](../../../crates/lctx-model/src/domain/completed.rs), lines 45–179,
and [earned views](../../../crates/lctx-native/src/installed_view/earned.rs), lines 15–42.

**Shared payload is distinct from shared authority.** The selector's `Arc` does not replace pins,
session, pool or lifetime ownership. The protected, operation-local charged audit can share direct
ingredients while each root earns its mapping and terminal obligations. See
[installed views](../../../crates/lctx-native/src/installed_view.rs), lines 32–114,
[audit](../../../crates/lctx-native/src/installed_view/audit.rs), lines 33–150, and
[shared recovery ingredients](../../../crates/lctx-model/src/domain/recovery_closure/shared.rs), lines 40–207.

**Preparation demand has a real owner.** Binary contributor partitions reuse unchanged branches.
Registry retention follows current/frozen and running/remaining consumers; borrowed runs and
accepted flights retain independent lifetimes. See
[compiler preparation](../../../crates/lctx-compiler-store/src/compiler.rs), lines 7884–8125,
and [membership demand](../../../crates/cpg-core/src/workspace/membership_demand.rs), lines 19–174.
This is stronger than arbitrary cache sizing.

**Optional retention can decline before effects.** Singleton completion/private retention precede
mutable composition; the gate protects handoff rather than unrelated private work. See
[production](../../../crates/lctx-compiler-store/src/compiler/production.rs), lines 57–155,
and [Workspace](../../../crates/cpg-core/src/workspace.rs).

**Independent admission is substantive.** It preserves frozen inventory and original provider/session
ownership, checks references and applicable semantics, and retains exact private executed evidence.
Portable success or diagnostic replay grants no authority. See
[admission](../../../crates/lctx-compiler-store/src/admission.rs), lines 1199–1408 and 2578–2653.

**Finality is stronger than application End.** The patched SDK drains physical transport,
preserves Rows/Value/EmptyValue and joins execution/delivery. Readers check ordered statement
terminals and retain uncertain owners. Stock buffered queries are not equivalent replacements.

**Sparse selection and indexed retirement are deliberate.** Exact membership precedes hydration.
The primary-key range cast selects the pinned planner route carrying LIMIT into storage scanning.
Retirement uses bounded exact-state heads, atomic child nomination and recoverable progress. See
[control](../../../crates/lctx-native/src/control.rs), lines 94–126, and
[retirement](../../../crates/lctx-native/src/control/retirement.rs), lines 947–1193.

**Catalog avoids an unjustified Behavioral dependency.** Compact owners preserve total NotRequested
domains. C0 uses32-public-name windows, not a release-wide rich cross-product. Neither should be reversed.

**Completion preserves independent axes.** Operation failure, local terminality, remote certainty,
committed effects and storage disposition remain distinct. Accepted ingress retains charged batches
through acknowledgement/drainage. See [Completion](../../../crates/lctx-model/src/domain/completion.rs),
lines 43–142, and [ingress](../../../crates/lctx-compiler-store/src/compiler/ingress.rs), lines 1–145.

## 4. Composition and execution assessment

| Operation | Semantic/projection contract | Physical assessment |
|---|---|---|
| Extraction/normalization | Attributed pinned observations, qualified resolution, explicit coverage/ambiguity | Distinct provider States and role mapping are justified; whole-capture verification has a separate ownership question |
| Completed union | Exact selected universe; equal-content coalescing and unequal-content refusal | Binary ingredients reduce leaf preparation; changed roots still merge |
| Catalog C0/C1 | Selected public declarations, exact source/effective contracts and contextual evidence | Bounded windows/keyed preparation are strengths; granular cache preparation follows rich hydration |
| Base | Contextual root dependency closure; bounded conservative evaluation and root-specific proofs | Repeated per-expression closure/hydration/every-input planning: F05 |
| Summary | Qualified calls/ports, callee-first SCCs, attempt/pair outcomes and residuals | Existing SCC/worklists are useful; first-use correspondence still scans broad immutable collections: F06 |
| Analytics | Named graph projections, weights/settings/seeds; heuristic output, explicit partial/exhaustion | Pair/FCA products have real consumers; representation alternatives need equivalent outputs: I05 |
| Synthesis/retrieval | Programmatic evidence linkage; selected corpus and eligibility; ranking is not proof | Preserve same-realization witnesses/originals; assess query-shaped scoring access: I07 |
| Native ingress | Full-value conflicts, ownership and atomic bounded effects | Transport batching hides repeated parent/scalar work: F02 |
| Concurrent producers | Independent ready work plus complete close/freeze | Shared attempt conflict address: F03 |
| Retirement | Incarnation/hold exclusion and recoverable progress | Disjoint pages also write a global backup gate: F04 |
| Native reads | Exact selection and complete application/physical terminals | Pre-delivery decode/queue allocations lack operation-budget authority: F07 |
| Recovery/development | Original request reconciliation and truthful protected cleanup | Unbounded recovery re-admission, cancellation-state defect and unrelated history preparation: F01/F08/F09 |

There is no single universal lifecycle defect. Removing scalar native work does not fix cancellation;
shortening a completion lock does not admit SDK allocations or eliminate Summary scans.

## 5. Findings

The following stable IDs are **required for target alignment and scheduled in the separate
Catalog efficiency track**. The
[Catalog efficiency coordinator §6](../../plans/catalog-collection-efficiency-plan_2026-10-17.md#6-sole-disposition-and-current-checkpoint)
owns their current disposition; this remains a dated assessment. Existing findings keep their source IDs
and historical closure boundaries. Proposed corrections below are not implemented by this review.

<a id="F01"></a>
### F01 — Completed registrations can suppress cancellation of live work

**High priority · FP-04/05, DP-02/19/20 · A2, G2/G5. Diagnosis: source-established and Tested by the branch probe.**

[request_stop](../../../scripts/run_delegation.py), lines 336–415, creates request state in
`owner_requests` before deciding whether that registration has a live owner lock. A completed
registration can supply the apparent prior request for a live registration with the same owner.
The live registration skips first SIGTERM. Without a persisted request, each poll manufactures
a new timestamp and restarts grace.

This is legitimate composition: an owner finishes setup and continues supervising a fixture or
capture writer. Cancellation must not depend on registration-ID lexical order. The coordinator's
pure probe exercised the actual function with substituted liveness/signals: completed-first issued
no signal over three polls and persisted no live request; reversed order issued SIGTERM, then SIGKILL.

**Corrective contract, Proposed.** Group actual participants by authenticated owner; distinguish
terminal registrations from live obligations; persist real owner-level request issuance before
using it to suppress another request or age grace. Keep child-specific cleanup and observer
finalization separate. Enumeration order cannot establish that a request was issued.

**Countercase and closure.** Preserve independent finalizing observers and shared daemons. Exercise
completed-before-live, reversed/shuffled order, multiple live registrations, repeated polls, owner
death and failed request persistence. The pure probe establishes the branch defect, not real host
drainage or native finality.

<a id="F02"></a>
### F02 — Native batches repeat shared parent and incidence work per row

**Medium priority, broadly enabling · FP-07, DP-10/14/16 · A4. Diagnosis: source-established.**

[immutable_row_plan](../../../crates/lctx-native/src/control.rs), lines 355–420, batches transport
but loops over rows, invokes incidence functions, reads existing values and inserts individually.
For compiler memberships, [schema functions](../../../crates/lctx-native/src/control/schema.surql),
lines 162–200, touch the contribution twice and reread the same parent FOR UPDATE. The
[effect wrapper](../../../crates/lctx-native/src/control/authorization.rs), lines 737–803, separately
owns intent, payload and outcome-reference release transactions.

A many-membership batch repeats parent eligibility/invalidation stable within its transaction.
Native bulk syntax cannot remove necessary record/index work, but this lowering adds avoidable
shared-control and interpreter work above it.

**Corrective contract, Proposed.** Consume a bounded typed window; prepare distinct parents/objects;
validate shared eligibility once; compare full existing values under exact required guards; then
group new ordinary/relation rows for native bulk insertion. Produce one atomic window effect outcome.
Preserve full-value collisions, original-blob completion exclusion, table-family rules, incarnation/
era checks, rollback and independently necessary outcome discoverability.

**Countercase and closure.** INSERT IGNORE or unconditional duplicate updates are not equivalents.
Deduplicated touches must preserve exact observed-revision-plus-one consumers in
[completed control](../../../crates/lctx-native/src/control/completed.rs), line 300, and
[live views](../../../crates/lctx-native/src/control/live_view.rs), line 268. Do not collapse durable
intent solely to remove a round trip. Test repeated parents, equal existing rows, late conflicts,
mixed relations, originals and interruption. Inspect revision meaning; measure only for a speed claim.

<a id="F03"></a>
### F03 — Disjoint producer effects share an attempt-wide conflict address

**High priority for compilation throughput · FP-07, DP-19/20 · A4. Diagnosis: shared conflict domain source-established; replacement protocol Proposed.**

Both intent and payload touch the attempt predicate; authorization reads the common owner FOR UPDATE.
`native_touch` updates one predicate record per target. Effects for disjoint ready producers therefore
write the same address. See [authorization](../../../crates/lctx-native/src/control/authorization.rs),
lines 690–715, [schema](../../../crates/lctx-native/src/control/schema.surql), lines 157–168, and
[coordination](../../../crates/lctx-native/src/control/coordination.rs), lines 31–34.

This limits concurrency delivered by dependency-ready scheduling. More futures cannot remove the
shared transactional key; conflicts repeat work. Historical logs contain actual conflicts, including
a payload reaching at least 64 retries, but neither their exact key nor causal time share is established.

**Corrective contract, Proposed.** Attempt admission and close/freeze operate over explicit admitted
participants and their exact durable effects, at conflict granularity appropriate to independent
producer/window work. Closing admission establishes a complete cut before relying on effect or
contribution inventories. Cover delayed RPCs, arriving intents, references, reconciliation and terminal
outcomes. Existing striped installation admission is a relevant mechanism to adapt, not a reason
for another global lock or worker cap.

**Countercase and closure.** Merely replacing attempt touches with read-only FOR UPDATE is incomplete:
a close/freeze transaction could miss a new intent committed outside its older predicate snapshot.
Specify the complete finite participant/admission cut, then check disjoint writes, close/freeze,
delayed intent/payload, added/missing inventory entries, uncertain commit, reconciliation and ABA.
Demonstrate useful disjoint overlap without weakening drainage. Protect the current protocol until
the replacement is qualified.

<a id="F04"></a>
### F04 — Disjoint retirement pages write a common backup gate

**Medium priority · FP-07, DP-10/20 · A4. Diagnosis: source-established; narrow replacement Interface-checked.**

Every claim, outgoing-hold and final-deletion page calls `native_backup_clear`, which checks and
increments `native_backup_gate:current`. Disjoint jobs share this written key. See
[schema](../../../crates/lctx-native/src/control/schema.surql), lines 176–181, and
[retirement](../../../crates/lctx-native/src/control/retirement.rs), lines 519/771/838.

The pinned RocksDB `getu` path uses snapshot-relative `get_for_update_opt`; record-ID FOR UPDATE
registers commit validation. A checked retirement read can plausibly preserve exclusion while
backup acquisition/release remain writers. Exact server paths/version are in the library inquiry.

**Corrective contract, Proposed.** Preserve active-backup refusal and the checked gate observation,
but remove retirement-side revision mutation if pinned interleavings establish equivalent exclusion.
Keep object guards, hold predicates, incarnations and atomic nomination.

**Countercase and closure.** Plain snapshot reading is insufficient. Check two disjoint retirements
and backup acquisition/release in both commit orders on the exact RocksDB realization. Reconcile
revision exhaustion and observer contracts. Separately establish whether the public global backup-hold
API still has a supported consumer: selected [backup](../../../crates/lctx-publisher/src/backup.rs),
lines 44–118, already uses a publication pin and external snapshot. Search results alone do not
authorize deleting a public contract or runtime obligation.

<a id="F05"></a>
### F05 — Base repeats whole-inventory preparation for each expression root

**High priority within explicit Behavioral support · FP-07, DP-10/16/20, CI-07 · A4. Diagnosis: source-established, with historical frequency observations.**

[Base execution](../../../crates/cpg-core/src/semantic_execution.rs), lines 437–476, selects/hydrates/
evaluates each expression separately. [BaseScopes](../../../crates/cpg-core/src/base_scope.rs),
lines 150–176 and 818–857, walks every declared input and constructs its reader/query for each grain.
[PreparedExecution](../../../crates/lctx-model/src/domain/execution/evaluation.rs), lines 300–390
and 1145–1177, rebuilds syntax preparation despite borrowed frame metadata.

Prepared edges, frontier selection and shared metadata are real reductions; this is not whole-graph
reconstruction per expression. Empty native selections also have shortcuts. Remaining root × family
setup and overlapping hydration/index preparation are nevertheless explicit. The historical recursive
capture records148 selected grains and 15,188 selected-input terminals; these are not RPC counts.

**Corrective contract, Proposed.** Prepare bounded compatible root cohorts under exact frame/input
selection; share immutable acquisition and syntax correspondence where dependencies overlap; retain
root partitions for semantics, errors, work limits and proofs. Known-empty selections avoid unnecessary
execution setup while retaining owner/schema/emptiness obligations. Existing `PreparedEdges::batch`,
partitioned hydration and `PreparedPositions` are credible foundations.

**Countercase and closure.** Release-wide eager hydration can enlarge live state and expose malformed
unreachable data. Preserve lazy errors, frames, per-root bounds and independent proof state. Exercise
overlapping/disjoint roots, shared dependencies, unreachable malformed companions, mixed frames,
shuffled roots, refusal and cancellation. Cohort grain follows admitted useful work, not a thread cap.

<a id="F06"></a>
### F06 — Summary memoizes attempts after broad correspondence scans

**Medium priority within explicit Behavioral support · FP-07, DP-10/16 · A4. Diagnosis: source-established.**

`call_infos` scans all headers for each binding attempt; first-use attempt preparation scans declarations,
parameters, links and arguments. Residual production searches events/seeds again. See
[Summary production](../../../crates/lctx-model/src/domain/execution/summary_production.rs),
lines 928–979,1033–1160 and 1640–1725. Its lazy attempt cache and Discovery indexes at 986–1010 and 1289–1350
already remove other work, as do SCC ordering, Pareto decisions and attempted-pair queues.

The residual problem is first-distinct-attempt preparation over broad immutable collections,
especially many calls sharing signatures/callees.

**Corrective contract, Proposed.** Build charged borrowed correspondence buckets once per exact
immutable Summary invocation, preserving order and competing rows; lazily realize consumed attempts
from them. Existing owners retain caller/callee/frame bindings, ambiguity and qualified source identity.

**Countercase and closure.** A one-row map cannot silently select a declaration/header. Do not prepare
unconsumed suffixes or replace per-attempt outcomes with callee success. Check shared signatures,
competing declarations, mixed contexts, unconsumed attempts, recursive/reconvergent calls and exhausted
pair work. Preserve known answers, PairOutcomes, residuals and fresh proofs. This is not an attribution
of the recursive timeout to SCC iteration.

<a id="F07"></a>
### F07 — Application budgets begin after transport decode and queue allocation

**High priority for resource composition · FP-05/07, DP-20, CI-08 · G5, A4. Diagnosis: source-established.**

Authentication supplies no caller gRPC Config. The SDK adopts the advertised decode limit and sends
`max_batch_records=0`; the server chooses batches within its wire budget. The raw channel holds two
batches, already decoded before application charging. See [reader](../../../crates/lctx-native/src/reader.rs),
lines 540–546,3242–3250 and 3542–3557; vendored [gRPC](../../../third_party/surrealdb/src/engine/remote/grpc.rs),
lines 190–227 and 1142–1158; [finite query](../../../third_party/surrealdb/src/method/query.rs), lines 122–138;
and the exact server/RPC paths in the capability inquiry.

The reader honestly documents this boundary. It is not evidence of OOM or measured RSS. It does mean
that request reservation does not govern the complete ingress working set, especially with concurrent
streams, decoded expansion and retained values alive together.

**Corrective contract, Proposed.** The transport owner admits decode/queue workspace and negotiates
query framing suitable for actual row widths/resources. Consider caller decode ceilings and query-specific
record/byte control together; refuse before uncontrolled expansion or retain an independently admitted
decode owner. The protocol has a record cap; the present SDK entry point does not expose it.

**Countercase and closure.** A lower decode limit alone rejects valid wide values/batches without asking
for smaller frames. Preserve adaptive throughput, SINGLE/Rows/EmptyValue, physical EOF and cancellation
drainage. Buffered results and concurrency caps are not substitutes. Exercise wide rows, small budgets,
simultaneous streams, queue coexistence, early refusal and late errors. Show admission before the actual
allocation it claims to cover; measurement is needed for quantitative memory claims.

<a id="F08"></a>
### F08 — Recovery drops cancellation ownership and reissues conflicts indefinitely

**High priority for failure containment · FP-05/07, DP-19/20 · G5, A4. Diagnosis: source-established.**

For `recover=true`, [transaction retry](../../../crates/lctx-native/src/control/retry.rs), lines 143–224,
uses an empty cancellation context and no owner-controlled stop. After conclusive TransactionConflict,
it continues issuing the same transaction. Consumers include reconciliation and outcome-reference
release in [authorization](../../../crates/lctx-native/src/control/authorization.rs), lines 788–803 and 895–919.

Awaiting a real terminal for an unknown in-flight operation is necessary. Reissuing after checked
uncommitted conflicts is a different admission decision. Historical captures establish some recovery
conflicts, not an infinite recovery loop in those particular runs.

**Corrective contract, Proposed.** Recovery owns admission and protected handback. After an issued
transaction reaches a conclusive terminal, cancellation or exhausted recovery admission can stop
further issuance while retaining exact durable requests, unresolved obligations, pins and actual
native ownership. Report Outstanding/Unknown where appropriate and expose the existing repair route.
Do not invent a universal retry framework or timeout that pretends an in-flight operation ended.

**Countercase and closure.** Lost response can conceal commit. Handback cannot release original
references/session/storage merely because a caller stopped waiting. Submitted futures still reach
terminality. Check conclusive conflicts, stop between attempts, response lost after commit, delayed
original requests, repeated recovery and eventual success. No false cleanup or further issuance after handback.

<a id="F09"></a>
### F09 — Selected run discovery prepares unrelated history

**Medium priority for the developer loop · FP-06/07, DP-10/16 · A4. Diagnosis: source-established.**

[runs.resolve](../../../scripts/runs.py), lines 213–243, enumerates/sorts all run directories before
checking an exact full ID. [Listing](../../../scripts/run_listing.py), lines 74–160, builds complete
paths/names/hash, then reenumerates/sorts for stability. Selected summaries also repeat metadata/
owner/group observations, although process snapshots are shared.

Bounded detailed hydration and explicit complete safety inventory are improvements worth keeping.
The remaining defect is preparation scope rather than response length.

**Corrective contract, Proposed.** Resolve a validated exact child path directly under the run root,
with identity/symlink/ownership checks, before history-dependent fallback. For pages, choose an explicit
consistency contract and prepare its necessary inventory once. Anchored continuation can avoid
whole-history invalidation where appropriate; a derived index must earn its maintenance cost.

**Countercase and closure.** Refuse traversal/foreign paths; keep complete safety coverage and
corrupt/unresolved records. Preserve fallback ambiguity/labels. Check exact IDs avoiding unrelated
enumeration, insert/delete pagination races, malformed receipts and active/unresolved/complete selections.
Changed cursor semantics require RC02; direct exact lookup does not.

## 6. Material investigations, without predetermined replacements

These are explicit open design questions. Their trigger is planning the related correction or
encountering the stated workload; unexamined magnitude is not itself a new defect.

<a id="I01"></a>
### I01 — Physical selector factorization

Full root-qualified mappings remain in completion and transport; shared ingredients do not remove
flat emission/root traversal. See [compiler](../../../crates/lctx-compiler-store/src/compiler.rs),
lines 2337–2485/2639–2733, [earning](../../../crates/lctx-native/src/completed_state/earning.rs),
lines 445–515, and model completed transport.

Compare immutable qualified fragments across reconvergent suffixes with the current flat route.
Preserve root-specific copies/incarnations, complete conflict/absence checks, ordered identity,
independent receiving, originals, backup and cold import. No actual root/selector/dependency counts
establish flat output as the current bottleneck. Functional meaning does not mandate codec1; repeated
flat data alone does not justify a new codec. Settle representative consumer demand and full lifecycle first.

<a id="I02"></a>
### I02 — Eager unions versus demand assembly and broader readiness

Binary leaves/partitions reduce repeated read/decode/sort; changed eager roots still merge under handoff.
Compare private checked merge, incremental current union and demand assembly against earlier selected
ports and atomic conflict/count visibility. Do not substitute metadata for real nominal validity.

The driver also awaits facts, then normalization/admission, before upper scheduling
([compilation](../../../crates/cpg-core/src/compilation.rs), lines 1153/1365). Within-phase overlap is real.
Before fusing phases, identify a consumer whose exact epoch and input authority can be earned earlier;
normalization's Facts epoch and shared admitted authority may justify some barriers. A complete legal
early-consumer route, not the presence of a barrier, decides this question.

<a id="I03"></a>
### I03 — Capture integrity and auxiliary observation grain

Opening/post-provider captured-closure verification protects writable files; readonly permissions are
not immutable ownership. See [bundle](../../../crates/cpg-extract/src/bundle.rs), lines 98–118/509–518.
Removing repeated checks needs an actual immutable capture owner or equally complete consumed-scope contract.

Separately, each fresh [SourceObserver](../../../scripts/verify_evidence/context.py), lines 50–110,
hashes allowed tracked/untracked files including docs/vendor source; nested CaptureOwners have separate
caches. Settle the consumer for full-tree observation before narrowing it. Compare cheap invocation/build
observations with explicitly requested full-tree context and sharing compatible observations. Preserve
argv/cwd/configuration, concurrent-change limits and “source-to-binary unknown.” Establish what each
observation can prove rather than adding provenance machinery. This is not a historical latency attribution.

<a id="I04"></a>
### I04 — Catalog granular-cache economics and cross-window preparation

C0 selects32-public-name windows but hydrates rich scope before constructing granular value-dependency
requests, reconstructing requests on fresh retention. See [Catalog core](../../../crates/cpg-core/src/catalog_core.rs),
lines 223–281, [products](../../../crates/cpg-core/src/catalog_products.rs), lines 11–67, and model Catalog build.
Overlapping aliases/signatures across windows may repeat useful immutable ingredients.

Whole-stage reuse already precedes execution. Compare carrying one admitted request into retention,
sharing exact dependency ingredients across actual demand, and omitting a granular cache when its lookup/
retention costs approach fresh computation. Preserve complete keys and root membership; add no speculative cache.

<a id="I05"></a>
### I05 — Analytic expansion and representation density

Co-use expands target pairs after bounded work admission; exhausted layers withhold a complete combined
partition. FCA keeps row/column bitsets and finite limits. See model
[analytics build](../../../crates/lctx-model/src/domain/analytics/build.rs), lines 675–755/838–929, and
[concepts](../../../crates/lctx-model/src/domain/analytics/concepts.rs), lines 18–105.
Compare sparse incidence, suitable bitmap/library forms or streamed aggregation only against named
pair/evidence/lattice consumers. Preserve isolates, multiplicity, settings and partial semantics. No
equivalent pair elision was established; limits alone do not establish feasibility for ordinary supported demand.

<a id="I06"></a>
### I06 — Independent reference-admission preparation

[Reference admission](../../../crates/lctx-compiler-store/src/admission.rs), lines 2299–2490, correctly
brackets current owners and preserves empty/subtype/memo obligations. It compares same-family selections
pairwise and repeatedly filters targets, constructs unions and plans field-level joins. Investigate one
charged per-call correspondence inventory and exact target-set preparation shared across compatible fields.

Determine which fields reuse the same selection/columns; whether grouped conflict checks preserve full
fields/nulls/competing copies; and what planning/scans disappear after existing emptiness/premise reuse.
Preserve independent receiving, original context, missing-target and subtype schemas, complete EOF and
failure-atomic memo publication. Shared preparation is not shared admission success. Historical100-second
admission intervals motivate the inquiry but establish no causal share.

<a id="I07"></a>
### I07 — Query-shaped lexical scoring access

[Native scoring](../../../crates/lctx-serving/src/search.rs), lines 669–678, looks up membership per
eligible document and filters its stored complete term array for query terms. The
[lexical statistics owner](../../../crates/lctx-native/src/lexical_stats.rs), lines 197–355, correctly
establishes frozen selected-view/family statistics.

Compare indexed postings or another representation reaching candidate/query-term intersections with
the compact arrays and bounded candidate windows. Include added rows/indexes, construction, reconciliation,
backup and retirement. Determine whether unrelated terms justify that cost and whether native access
actually constrains both dimensions. Preserve selected-corpus N/df/lengths, analyzer identity, zero-IDF
matches, eligibility before caps, deduplication, ties, fusion and winning witnesses. Table-wide BM25 is
not an equivalent replacement. Use independent known answers for empty/zero/duplicate/witness cases.

<a id="I08"></a>
### I08 — Diagnostic coverage and observation preparation

The concurrent [operation-diagnostics review](design_review_operation-diagnostics-target_2026-10-15.md)
has a separately stated live/post-run operational target. Its evidence informs this review; its
verdicts and proposed always-on signals are not inherited automatically. Independent source checks
leave several narrower questions, and some material disagreement:

- [The text adapter](../../../scripts/verify_evidence/runtime_adapter.py), lines 37–48, does retain
  matching flat producer/attempt fields. Its JSON path preserves ordered span context. Flat text
  loses parentage and repeated field names can overwrite context, but “no association recovered”
  would be too broad.
- [Projection](../../../scripts/verify_evidence/projection.py), lines 181–189, prepares observations
  from both native envelopes and native.log. [CLI reporting](../../../scripts/verify_evidence/cli.py)
  explicitly labels supplemental phase/attempt observations, not unique executions. Duplicate
  preparation/storage can be inefficient without establishing false test counts or a G6 failure.
- [Nextest coverage](../../../scripts/verify_evidence/nextest.py), lines 319/428/502, reports truncation
  unknown. Establish which consumed payload contains a known truncation marker; expose that loss
  and its locator there. An unmatched begin is incomplete observation, not evidence of a stall.
- [Phase](../../../crates/lctx-native/src/phase.rs), lines 149–195, captures its actual span/dispatcher
  and emits through that owner. Drop produces no invented terminal. Propagation before begin and
  omitted event kinds remain questions, rather than absence of all observation ownership.

Determine which consumers need structured parentage, which channels have known loss, whether one
attributed preparation can serve overlapping channels without losing original provenance, and which
operational questions require new signals. Structured DataFusion metrics and server statistics are
eligible library capabilities; availability alone does not justify periodic sampling or another
platform. This review leaves source-specific coverage unresolved and does not certify the parallel
review's full diagnostic target. F01's demonstrated cancellation defect is independent of observing it.

## 7. Complete alternatives and library fit

| Alternative | Whole-operation benefit | Integration burden and judgment |
|---|---|---|
| Corrected shared-native route | Persist once; selected Rust/DataFusion kernels; independent native admission/publication | Preferred immediate direction; correct concrete costs while retaining working authority |
| Operation-shaped native sets | Distinct parent/guard preparation, exact checks and grouped insertion; selective joins/projection near data | Strong narrow opportunity. Bulk syntax still performs record/index work; preserve delayed intents, atomicity, full bytes and outcomes |
| Native selection/validation | Reduce movement by lowering suitable model-owned relational checks | Choose per operation. Selected universe, multiplicity, absence and bounded work survive; traversal alone is not program analysis |
| Local spillable compiler preparation then native ingestion | Remove intermediate native control/read crossings while retaining bounded final native publication/querying | Credible larger alternative; compare local backing, final independent admission, uncertainty, originals, reuse and recovery together |
| Factored native selectors | Share qualified immutable suffix fragments where physical choices agree | Adds identity/lifecycle/receiving/cold consumers; I01 remains open |
| Database-hosted WASM kernels | Potentially remove transfer for a suitable bounded computation | No general benefit established; executable trust, deployment, cancellation/state/epochs remain real costs |
| Files/indexes or another store | Replace storage/search/graph/lifecycle as a complete operation | Eligible on technical merit. No complete simpler replacement for evidence, originals, independent admission, concurrent pins and recovery was established |

The native research used the supplied reference, Context7, the offline skill and exact source.
Current documentation is a discovery source, not automatic proof at the pin. The
[official documentation index](https://surrealdb.com/docs/llms.txt) exposes focused primary routes;
availability and consumer fit were checked separately.

Specific conclusions, qualified in the library inquiry:

- FOR UPDATE is optimistic snapshot-relative validation, not blocking or predicate locking. Bound
  arrays are not equivalent locked multi-get at this pin; scalar RecordId targets matter.
- INSERT/INSERT RELATION are available; INSERT IGNORE does not preserve full-value collision semantics.
- Native references/constraints offer local integrity, not complete incarnation/admission/outcome ownership.
- Arrow encoding is declared in the protocol but rejected by the inspected server; it is not an available fast path.
- Physical-EOF and retained external-transaction patches are substantive. Configuring transport must preserve them.
- SurrealKit beta cleanup is not native finality. Generated authority, containment and whole-target disposal are justified.
- Search built-ins must preserve selected-corpus/ranking meaning or propose an explicit policy change; native graph
  operations must preserve the declared universe/attribution/multiplicity and do not replace transfer semantics.
- Events/live/changefeeds/timers, buckets, generic DB MCP, Cloud and new graph front ends have no demonstrated missing
  consumer here. Their added writes, retention and recovery are costs to assess, not free simplifications.

The existing RocksDB/native route has no demonstrated engine-replacement requirement. No reviewed source
establishes engine I/O as the dominant cost; switching engines cannot itself remove repeated parent checks,
preparation or shared conflict addresses. This is a bounded judgment, not a claim that other engines are inferior.

### Shared corrective structure

The recurring fixes can compose through existing owners: operation-shaped bounded native lowering;
charged immutable preparation with per-consumer semantic checks; an ingress owner spanning decode,
delivery and terminality; and explicit request/recovery admission state. Native `OwnerEffectPlan`,
prepared edges/positions, resource pools, finite BatchStream and registered host owners are foundations
to refine. There is no need for a universal lifecycle enum, scheduler, proof cache, retry framework or
persistent run database. Shared preparation must never become portable validity or independently redefined policy.

## 8. Change, growth and failure scenarios

| Scenario | Required owner/contract behavior | Assessment |
|---|---|---|
| New fact family | One model declaration, new provider behavior where needed, derived adapters and meaningful controls | Existing schema derivation is a strength; F05 makes family growth multiply per-root setup |
| Analyzer upgrade | Adapter/captured contract changes; fidelity, role correspondence and disagreements remain explicit | Distinct Ruff/Pyrefly/ty contexts are appropriate; do not merge resolver States solely for reuse |
| New Catalog rendering | Consume canonical interpretation/evidence rather than add a classifier | Existing product ownership is supported |
| Mechanism substitution | Preserve exact universe, ambiguity, partial/work outcomes and evidence | Native sets/local spill/another engine remain eligible; replace complete operations, not only bytes |
| Shared expression/call dependencies | Borrow immutable ingredients; keep independent roots/attempts and proofs | F05/F06 remove repeated work without requiring a new fixpoint model |
| Disjoint ready producers | Independent effects with complete close/freeze cut | F03; read-only attempt checks alone fail the concurrent-intent countercase |
| Retirement with backup | Object/hold exclusion plus checked backup observation | F04; full gate deletion remains unselected |
| Wide/concurrent responses | Admit decode/queue and retained values; preserve physical EOF | F07 |
| Cancellation after setup | Request actual authenticated live owner while retaining observer/native obligations | F01, independent of native performance |
| Sustained recovery conflicts | Await each submitted terminal, govern reissuance and protected handback | F08 |
| Known run in long history | Resolve selected identity before unrelated inventory | F09; complete safety scans remain complete |
| Reader follows evidence during publication | Pin complete realization and originals across cursors/resources | Existing contracts supported; full current linked-Catalog acceptance remains open |

## 9. Independent judgments and gates

| Judgment | Verdict | Evidence and limit |
|---|---|---|
| A1 Localize change | **Satisfied at inspected ownership boundaries** | Fact-family and mechanism scenarios have identifiable owners; no new duplicated product classifier established |
| A2 Governing domain model | **Violated at delegated cancellation; product model substantially satisfied** | F01's manufactured request state, rather than actual issuance, governs cancellation; product identities/coverage/completion/receiving have governing contracts |
| A3 Composition | **Satisfied at principal product boundaries** | Exact inputs, optional preparation, independent admission/publication/evidence compose; physical amplification and other gate failures remain |
| A4 Execution fit | **Violated** | F02–F09 establish repeated work, contention, resource-boundary and failure-amplification problems for supported workloads |

FP-01/02/03 and product FP-04 are supported by inspected owners, exact contracts and independent
receiving. FP-04/05 fail at F01; FP-07 fails at the identified mechanisms. FP-06 is a strength in
pure kernels but F09 expands selected developer work unnecessarily. DP-01/04/05/07/08/09/11/12/18/21/22/24
have identified mechanisms; DP-02/19/20 have scoped failures; DP-10/14/16 have material execution-fit
violations. This is qualitative applicability, not a score or an exhaustive heuristic verdict table.

| Gate | Verdict | Independent basis |
|---|---|---|
| G1 Authority | Pass, source scope | Model-owned meaning; portable correspondence grants no admission |
| G2 Fidelity | **Fail at F01** | Unissued cancellation state treated as prior request; provider fidelity separately supported |
| G3 Validity | Pass at inspected boundaries | Selected references, validators, collisions and semantic admission precede publication; runtime acceptance separate |
| G4 Hidden behavior | Pass, source scope | Explicit captured inputs/effects; no new ambient/evaluation ingress established |
| G5 Consistency/recovery | **Fail** | F01 cancellation, F07 ingress resource composition, F08 recovery re-admission |
| G6 Transformation/reuse | Pass, source scope | Exact keys, independent receivers and complete EOF; remedies must preserve them |
| G7 Truthful claims | **Unresolved for source-specific diagnostic coverage; product reporting supported** | Historical/source/focused/enclosing evidence and supplemental counts are disclosed; I08 must settle known-loss coverage without inheriting blanket claims |
| G8 Library leverage | Pass at qualified boundary | Suitable graph/program/set libraries considered; F02/F04 need better composition, while incomplete substitutes receive no credit |
| CI-G1 Fidelity | Pass, source scope | Attributed assertions, typed unknown/partial/NotRequested and governed heuristics |
| CI-G2 Evidence closure | Pass at inspected query/original boundaries | Fixed realization and exact attribution/continuations; full linked-Catalog runtime open |
| CI-G3 Evaluation integrity | Pass at inspected production boundary | No private-reference ingress established; sealed evaluation not qualified |

Source-scope passes are not fresh runtime test passes. No positive result compensates for failed
architectural judgments or another failed gate.

## 10. Verification and evidence calibration

**Passed 2026-10-15, exit 0:** `uv run --no-sync python
docs/design_review/evidence/2026-10-15_catalog-completion-efficiency/delegated-stop-probe.py`.
Actual stop-loop branch, substituted liveness/signals; real host/native drainage not_run.

**Passed 2026-10-15, exit 0:** `uv run --no-sync python
docs/design_review/evidence/2026-10-15_catalog-completion-efficiency/phase-observations.py`.
Historical extraction, not fresh product execution. Exact hashes, commands, original line numbers
and durable consumers are in the evidence index.

| Existing receipt | Observation and limit |
|---|---|
| Recursive `8a3347` | Base approximately 323s;148 selected grains/15,188 selected-input terminals. Summary later fails with `native transaction retry cancelled`; no SCC-dominance claim |
| Document/lexical `7c15e3` | Genuine Catalog compiles approximately 260/265s; subsequent admission approximately 100/95s before cancellation, before mutation assertions |
| Recursive cancellation | Requested18:26:36UTC, ended18:29:10; Nextest timeout also recorded. Same-owner receipt ordering matches F01; no attribution of every delay interval |
| Native conflicts | Actual intent/payload/recovery conflict observations, one payload reaching at least 64 retries; power-of-two logging is not complete retry counts, exact key attribution or causal share |
| Earlier native controls | Keep their exact revision/scope at coordinator §6; not rerun here |

Phase events are not RPC or unique-test counts. Nested/concurrent durations are not additive.
The earlier finite replay truncated Base; newer logs have no literal truncation notice, but capture
coverage and producer ceilings still limit inference. Source-to-executable closure remains unknown.

Fresh builds/native controls, performance measurements, full journeys, operator activation and assembled
qualification: **not_run**. Static evidence sufficed for structural findings. The narrow F04 races and
F03 cut interleavings belong after their correction contracts are specified; no benchmark campaign is
required to establish the repeated work. Documentation/probe checks do not close V01–V05.

## 11. Priorities, dependencies and rule impacts

F01/F07/F08 affect cancellation, resources and failure containment. F03 affects compilation concurrency
and needs a complete cut contract before replacement. F05 is high within explicit Behavioral support,
without becoming a Catalog prerequisite. F02/F04/F06/F09 are independent bounded corrections where ready.

F02 can group transaction-local work while retaining the current attempt fence. F04 has a narrower race
contract than F03. F07 must preserve the current finality owner. F05/F06 retain independent semantic
checks; shared preparation cannot substitute success. Priority and prerequisite order therefore differ.

<a id="RC01"></a>
<a id="RC02"></a>
<a id="RC03"></a>
<a id="RC04"></a>
### Required and conditional rule impacts

| ID | Current rule / owner | Proposed change and dependency | If kept |
|---|---|---|---|
| RC01 | D02; [ADR-0145](../../adr/0145-holistic-state-ownership.md) conflicting-guard-write direction, storage §6.3/schema8 | F03 may replace attempt-wide mutations with a qualified finite participant/admission cut; preserve permanent delayed fencing, outcomes, Unknown and ABA; supersede only affected clauses | F02 still proceeds; F03 contention remains unresolved or requires an explicitly bounded supported workload |
| RC02 | README run-list membership invalidation; `run_listing.py`; [ADR-0172](../../adr/0172-composed-delegated-command-ownership.md) discovery contract | F09 may select anchored continuation/coverage instead; migrate help/response/consumers | Exact-ID fast lookup still proceeds; retain work necessary for the stronger cursor contract and remove redundant preparation |
| RC03 | Patched gRPC profile/provenance and SDK pin owner | F07 adds request/decode/batch admission; a qualified adapter/protocol extension updates capabilities/provenance | Keep honest ingress-boundary limitation; no complete request-budget claim or concurrency-cap compensation |
| RC04 | Recovery loop policy within ADR-0145/[ADR-0165](../../adr/0165-prepared-completion-and-owned-lifecycle.md) lifecycle direction | F08 adds owned recovery admission/handback after checked terminals, retaining obligations; record actual owner policy | Cancellation cannot bound further recovery issuance; F08 remains unresolved |
| Conditional only | Flat codec1 versus semantic-v4 and eager visibility: ADR-0166/storage §6.1 and retained I01/I02 directions | I01/I02 may change physical representation or visibility after complete consumer comparison | No such migration selected here |

F01/F02/F05/F06 and F04's minimum locked-observation correction refine existing guarantees and need
no ADR solely because code changes. Deleting the global backup API/gate is a separate decision after
consumer analysis. No rule is changed by this review; plan creation presents directions for confirmation.

### Existing findings and retained choices

| Existing scope | Comparative judgment and new obligation |
|---|---|
| P01/P02/P06; frictionF01/F02/F08 | Separation of operation failure/finality and optional pre-effect decline remains right. F08 is a separate recovery-issuance problem; no reason to flatten Completion or weaken poisoning |
| P03/P04; frictionF03–F05 | Pre-admitted growth and shared charged selectors remain right. F07 concerns allocations before the application's admission, not removal of those protections |
| P05/I01/I02; frictionF06/F07 | Binary ingredients and protected direct assurance remove real work. Remaining flat emission/eager merge questions are I01/I02; not a claim that the delivered fixes did nothing |
| T01; frictionF09 | Registered delegated ownership is appropriate. F01 exposes a composed same-owner case its current request selection mishandles; F09 is residual preparation beyond bounded hydration |
| Creation analysis / D03/D04/D06 | Per-seed witnesses, published flat semantics and pair/proof coverage remain meaningful. F05/F06 change preparation grain, not required answer meaning or SCC policy |
| D11/D12 / retirement closeout | Indexed discovery and bounded claim/finalization are improvements. F04 concerns the remaining global gate write, not rollback to scalar effect dispatch |
| V01–V05 and prior partial review | Local source/focused acceptance remains dated and bounded. This broader target review identifies new changes; it does not rewrite earlier verdicts or close unfinished exits |

D01 and D07–D09 retain real provider/source-context reasons. D05's governed analytic products have
no demonstrated equivalent elision; D10's useful exact alias preparation remains sound; D13 remains
justified while captures are writable. These are reasoned present judgments, not immunity from revision.
C01–C11 are capability choices, not an adoption backlog. Operation placement/storage remain open to a
complete better alternative; F02/F04 show substantial improvement is possible within SurrealDB.

## 12. Decision and enclosing status

**Revise.** Preserve the substantial domain/fidelity foundations while correcting the nine causal
findings. A1/A3 are supported at the inspected principal boundaries; A2 fails at delegated cancellation
and A4 at preparation, native control, transport and developer-loop mechanisms. G2/G5 have independent
scoped failures; G7 has the bounded diagnostic-coverage uncertainty in I08. Eight investigations identify consequential alternatives without pretending a replacement
has been qualified.

The next design work is to incorporate these findings into an execution plan, settle the operation/
admission contracts and implement without weakening independent receiving, exact identity/conflicts,
physical finality, original attribution or protected cleanup. Larger alternatives remain available
where complete comparison shows an advantage.

This review closes no V01–V05 exit, pinned pilot, current cold/linked-Catalog or older assembled
qualification boundary. Interrupted runs, source integration, reservation observations and historical
body passes remain their original evidence. Architectural judgment, implementation and runtime
qualification are separate.
