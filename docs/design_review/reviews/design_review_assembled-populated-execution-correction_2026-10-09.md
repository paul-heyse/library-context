# Assembled populated execution correction — independent review

**Current decision: Accept scoped at static / Implemented strength, 2026-10-09.**
The initial review and first reassessment found material integration defects. Sections 10–11
record their independent source reassessment after correction; no remaining material finding
was identified in that final scope. Runtime controls and the populated boundary remain separately
owned by the coordinator. None of these reviewer judgments claims Tested or Measured benefit.

Sections 1–9 describe the intermediate implementation examined before those corrections.
References to its now-retired native publication module are historical source locations;
publication now belongs to publisher's private admitted workflow.

**Initial decision: Revise.** The selected physical correction has a credible implementation: native writes use declaration-selected lowering, and final canonical readers reuse immutable pointer preparation without replacing hydration or independent admission. Five material issues remain: a valid-export verification regression, publication APIs that bypass admission, a final-read closure race, loss of acknowledged publication identity on cancellation, and incomplete failure-phase reporting.

These findings concern the assembled source and its public contracts. They do not establish runtime test failures or measured performance.

## 1. Review contract and scope

| Field | Assessment |
|---|---|
| Baseline | `e98fb4a994a4b49050ccc3c851a98e8e8ca66b9f`, plus the present uncommitted production/documentation diff and new native modules |
| Reviewer/date | Independent reviewer; 2026-10-09; no authorship of the examined changes |
| Tier/purpose | **Design / target**, with an independent correctness assessment |
| Standard | Core/template 3.3; efficient-architecture heuristics 1.0; code-intelligence profile 1.5; library-context binding |
| Functional target | Populated-journey execution plan; ADR-0138; semantic-model §15.11; storage/publication §6 |
| Selected completion | PJ0–PJ3, materially triggered PJ4, focused controls, and one populated native/MCP/evaluator boundary |
| Method | Read-only inspection of current Git changes, decisive source, and adjacent consumers |
| Evidence strength | **Implemented, source-inspected, 2026-10-09**. Remedies below remain **Proposed** |
| Verification | Builds, tests, generators, managed runs, and performance measurements: **not_run** |
| Intended publication path | `docs/design_review/reviews/design_review_assembled-populated-execution-correction_2026-10-09.md` |

The workload considered is populated pinned-library compilation followed by artifact admission, export or direct publication, repeated canonical reads, SQL backup/fresh restore, and connected evidence tools. Relevant growth dimensions are record volume, semantic vocabulary, completed contributors, aliases, and independently progressing final readers.

The deferred timeout campaign, assembled `qualify`, operator activation, real-library qualification, and larger storage/runtime redesigns are outside this decision. Native frontier placement and SQL staging restore were assessed as retained design choices, rather than treated as acceptance criteria.

The pending schema snapshot and integration results do not constitute evidence of either success or failure.

## 2. Responsibilities and semantic ownership

The intended dependency direction remains coherent:

| Owner | Responsibility | Inspected realization |
|---|---|---|
| `lctx-model` | Typed records, nominal identities, sums, invariants, attribution and completed-state meaning | `Relation::canonical` dispatches to the declared typed callback |
| Native adapter/codec/schema | Mechanical physical lowering and reconstruction | Selected adapter lookup; fixed envelopes; supplied scopes; complete expected rows |
| Native compiler | Contribution/view persistence, mutation admission, final preparation and operation lifetime | Mutation-only closure; final inventory; shared preparation; retained readers/workers |
| Workspace/core | Completion handoff, compilation attestation and semantic admission | Completion gate; retained owner descriptors; exact owner/binding comparison; distinct restored path |
| Publisher | Derived publication work and coherent visibility | Native graph reads, reference/search construction, final seal |
| Transport/restore | Independent external validation and fresh reconstruction | Detached loading and SQL staging remain separate from ordinary direct sealing |
| Serving/evaluation | Pinned evidence journeys and independent evaluation | Existing serving semantics; scoped journey diagnostics and representative runtime fixtures |

The principal boundary weakness is that **physical freeze and semantic admission are distinct in the owning workflows, but insufficiently distinct in the exported publication API**. F02 addresses that distinction.

### Fact and fidelity table

| Family | Provider/revision authority | Fidelity and coverage | Identity | Consumers |
|---|---|---|---|---|
| Source/provider facts | Retained captures, contribution specifications, provider runs and exact model identity | Provider-attributed; explicit availability and coverage remain | Nominal typed keys and exact completed views | Semantic admission, normalization, catalog |
| Graph entities/assertions | Model graph declarations and canonical codecs | Typed canonical representation; qualification and relationship meaning retained | Full entity/assertion identity and canonical content | Artifact admission, publication, serving |
| Nongraph compiler products | Declaration-derived compiler relation inventory | Typed intermediate products; not promoted to graph claims | Relation-qualified nominal key and reconstructed physical identity | Compiler scans, cold state validation, transport |
| Originals | Source metadata, ArtifactChunk and physical original chunks | Exact opaque bytes with independent range/digest checking | Source identity, ordinal/range and content | Export, restore, evidence retrieval |
| Search structures | Existing canonical search/window/vector authorities | Derived retrieval structures; eligibility and ranking remain separate | Canonical occurrence/projection identities | Connected tools |
| Phase observations | Actual operation/worker owner | Operational evidence, not semantic validity or performance proof | Phase ID plus scoped journey/span | Runtime diagnosis |

No changed path inspected relabels provider relations as another semantic relation or substitutes graph reachability for behavioral transfer.

## 3. What the implementation establishes

### Declaration-selected lowering removes the identified vocabulary coupling

[adapter.rs](../../../crates/lctx-surrealdb/src/adapter.rs) constructs declaration-derived adapters once and selects by relation name. The compiler checks the selected relation’s type and schema, then invokes its canonical callback before lowering: [compiler.rs](../../../crates/lctx-surrealdb/src/compiler.rs).

The fixed DDL removes vocabulary-wide body unions and scope `VALUE` programs. Adding an unrelated declaration changes lookup construction and semantic declarations; it does not expand an existing row’s database body-validation program. Local sum validation remains with the typed record callback rather than moving into another broad Rust union interpreter.

Independent reconstruction remains substantive:

- Graph reconciliation compares complete actual rows with mechanically regenerated envelopes: [reconciliation.rs](../../../crates/lctx-surrealdb/src/reconciliation.rs).
- Compiler backing reconstructs canonical bodies, nominal keys and complete physical envelopes.
- Scope values are regenerated from declared fields.
- Opaque original metadata checks multiplication and end-range arithmetic before the compiler begins the window’s effects.

Sharing mechanical envelope construction is appropriate. The independent boundary obtains actual stored content and reconstructs it; it does not accept a prepared pointer or matching digest as semantic proof.

### Final pointer preparation preserves hydration

The prepared runs contain compact physical pointers. [GraphRows](../../../crates/lctx-surrealdb/src/compiler.rs) still hydrates bounded pointer windows and checks missing, unexpected and incorrectly ordered backing rows.

[ordered_rows.rs](../../../crates/lctx-surrealdb/src/ordered_rows.rs) separates immutable scratch ownership from mutable cursors. Each cursor reopens the file and reserves reader/decode work. [acknowledged_candidates.rs](../../../crates/lctx-surrealdb/src/acknowledged_candidates.rs) drains the preparation worker before returning the immutable run. Consequently, an idle prepared handle does not retain its creator’s native scan lease.

The ordinary same-Workspace path has explicit charges for completed-owner descriptors, final handoff copies, scratch ownership, cursor work and transfer windows. I found no material new population-sized retained collection without a charge in that path. This is a static lifetime/accounting assessment, not measured memory qualification.

### Workspace finalization has the correct handoff shape

[Workspace::freeze_for_admission](../../../crates/cpg-core/src/workspace.rs) holds the completion gate, fences further Workspace mutation, drains the native content lane, compares actual completed owners and current/frozen bindings, and checks the normal compilation attestation.

Normal admission requires exact captures/frontier/profile/settings. Restored admission uses the restored manifest and independently reconstructed producer/facts/outcome contracts; it does not invent a normal compilation completion receipt.

### Direct and detached publication avoid canonical reload

The publisher derives references and search structures from native graph readers. Its previous post-admission portable canonical/original reload is removed: [publisher lib.rs](../../../crates/lctx-publisher/src/lib.rs).

Detached admission still loads external canonical/original/state content before freezing. SQL restore retains private staging, fresh canonical/state reconstruction, restored semantic admission, derived reconstruction and final actual reconciliation: [backup.rs](../../../crates/lctx-publisher/src/backup.rs).

The moved search implementation differs principally in ownership/import paths; its retrieval semantics are retained.

## 4. Material findings

Finding IDs are stable within this review. The coordinator’s [persisted-execution §8](../../../docs/plans/persisted-graph-execution-plan_2026-10-07.md) is the existing route for scheduling their disposition. This review does not close or duplicate that mutable ledger.

<a id="f01"></a>

### F01 — Known-export inspection now requires an unrelated frozen publication authority

**Priority: High — deterministic valid-input regression.**
**Principles/judgments:** FP-01, FP-02, FP-06; DP-08; A1/A3; G6.

[AdmittedArtifact::verify_export](../../../crates/cpg-core/src/artifact.rs) verifies a transported copy against its already admitted manifest using a supplied resource Workspace. It returns `Result<(), ModelError>` and does not perform detached readmission.

However, [verify_transport](../../../crates/cpg-core/src/artifact.rs) now constructs `VerifiedExport` with `workspace.native().frozen()?` for **both** branches. When `expected` is present, readmission is skipped, so an otherwise valid resource Workspace remains unfrozen.

The existing legitimate consumer makes the regression explicit: [transported_graph_refuses_missing_and_tampered_originals](../../../crates/cpg-core/tests/cases/graph_artifact.rs) exports admitted content, creates a separate fresh resource Workspace, and expects verification of the intact export to succeed.

**Consequence.** Correct transport inspection fails with the native “content is not frozen” refusal after completing its integrity work. Inspection has become coupled to publication readiness in a different database.

**Proposed correction.** Separate transport checking from construction of a publication-capable `VerifiedExport`. Known-manifest verification should return its inspection result without minting a native capability. Detached verification should construct `VerifiedExport` only after loading and independent admission.

Do not fix this by freezing the resource Workspace or importing content merely to inspect it; both introduce effects and lifecycle requirements unrelated to this operation.

**Closure evidence.** Trace both branches, then run the existing intact/tampered-original control. Verify that known-manifest inspection neither freezes nor populates the resource Workspace. Execution is currently **not_run**.

<a id="f02"></a>

### F02 — Public publication entry points can bypass the semantic admission boundary

**Priority: High — trust/capability boundary defect.**
**Principles/judgments:** FP-02, FP-04, FP-05; DP-02, DP-03, DP-19; A2; G3/G5.

Two exposed paths share this structural cause.

First, [NativeCompilerStore::frozen](../../../crates/lctx-surrealdb/src/compiler.rs) is public. It checks physical content readiness and mutation terminality, but not successful Workspace owner/binding agreement, compilation attestation or semantic admission. A caller can obtain `FrozenNative` after native `freeze_content` without traversing the admission owned by core.

The production wrappers traverse admission correctly. The exported capability contract does not enforce that route.

Second, `native publication.rs` (intermediate source) is a new public raw-Loader sealing function. It fingerprints definitions, creates VIEWER credentials and inserts the publication marker without performing manifest validation, graph/state/search reconciliation, native closure or one-shot seal admission. The publisher retains a forwarding [private helper](../../../crates/lctx-publisher/src/lib.rs), although the changed restore route uses `FrozenNative::seal`.

**Consequence.** The API admits “physically frozen” as sufficient publication authority and exposes a marker-writing route outside the private final seal. Callers can bypass guarantees that the ordinary and restored wrappers carefully establish.

This finding does not claim that current ordinary publication takes those shortcuts. It identifies an accidental API escape in the assembled boundary.

**Proposed correction.** Keep the marker-writing leaf private and remove the obsolete forwarding route. Preserve a distinction between physical freeze and successful semantic admission: either retain final sealing behind admitted-artifact/restored-admission owners, or require an opaque admission authority whose construction those owners control. Bind that authority to the exact frozen store and manifest.

Do not add a second semantic validator in the native layer. Preserve core’s semantic ownership and native’s physical lifecycle ownership.

**Closure evidence.** Inspect all capability constructors and marker writers. Demonstrate that physical freeze alone grants immutable read/preparation access but cannot grant publication, and that no public raw-Loader function bypasses the final seal. No execution was performed.

<a id="f03"></a>

### F03 — Global closure can reject a canonical read that was already admitted

**Priority: Medium — scheduling-dependent lifecycle regression.**
**Principles/judgments:** FP-02, FP-05; DP-08, DP-19; A3; G6.

[retain_read_setup](../../../crates/lctx-surrealdb/src/compiler.rs) admits the read before spawning its setup driver. The final seal closes general admission and waits for those admitted operations.

The new frozen setup path then calls [prepared_canonical](../../../crates/lctx-surrealdb/src/compiler.rs), which unconditionally invokes `check_frozen`. That calls the general `check`, rejecting a globally closed authority—even when the preparation already exists.

A revealing schedule is:

1. A canonical/header read receives its setup lease.
2. Seal begins closure before the setup driver progresses.
3. The driver enters the prepared-run path.
4. `check_frozen` rejects closure initiated by the seal that is waiting for this admitted read.

**Consequence.** A valid accepted read fails because final drainage starts, rather than completing under its retained ownership. The behavior depends on scheduling. This is within final-reader/seal composition, not the deferred pre-final overlap investigation.

**Proposed correction.** Separate admission of a new external read from execution of an already admitted read’s descendants. Under a retained parent lease, check failure and frozen-content readiness while permitting final closure to wait for the operation. Keep new external admission closed.

**Closure evidence.** Arrange a focused case that admits canonical setup, pauses before prepared-run access, starts sealing, then releases setup. The read should complete and retain its cursor/worker ownership until terminality; sealing should wait. Cover existing preparation and preparation initiated by an admitted parent. **not_run**.

<a id="f04"></a>

### F04 — Cancellation after acknowledged marking loses the committed handle

**Priority: High — retained completion-state gap.**
**Principles/judgments:** FP-05; DP-05, DP-19, DP-21; A2; G5.

`native publication.rs` (intermediate source) acknowledges the marker and records the committed handle in a **local** `Completion`, then awaits session invalidation and VIEWER readback.

Returned failures preserve that committed effect correctly. Cancellation during either later await drops the local completion. The private [OperationLease drop path](../../../crates/lctx-surrealdb/src/compiler.rs) retains an unconfirmed-operation failure, but the native operation state has no retained committed publication identity.

**Consequence.** A later drain can report uncertainty/orphan disposition, but cannot report the exact publication handle already known to have committed. One-shot sealing prevents retry; the known committed identity is nevertheless lost.

Cleanup remains conservatively refused, so this is **not** a demonstrated rollback or deletion. It is a failure to preserve the distinction between a known committed publication and an unconfirmed attempt. This is a carried-forward gap in the now-consolidated final seal, rather than necessarily a newly introduced regression.

**Proposed correction.** Record marker acknowledgement and the exact committed identity in the retained seal/completion owner before the next await. Later drainage and failure reporting must preserve it after caller cancellation. A small seal-owned state is sufficient; no event ledger or general retry machinery is needed.

**Closure evidence.** Cancel after successful marker acknowledgement and before readback completes. A subsequent drain must retain the exact committed handle, refuse destructive cleanup and refuse another seal. Separately retain unknown status when marker acknowledgement itself was interrupted. **not_run**.

<a id="f05"></a>

### F05 — Acknowledged phase failures are reported as incomplete work

**Priority: Medium — selected diagnostic contract incomplete.**
**Principles/judgments:** FP-06; DP-21; PJ1 operational observability.

The `Phase` primitive appropriately emits no terminal on drop. That preserves the distinction between interrupted work and actual terminal work.

Several production callers, however, finish only successful results:

- `native publication.rs` (intermediate source);
- publication search/reconciliation/seal;
- [canonical preparation](../../../crates/lctx-surrealdb/src/compiler.rs);
- artifact/semantic admission paths that return through `?`.

For example, an acknowledged statement error finishes its operation lease and returns `Err`, but the phase is dropped without a `failed` terminal.

**Consequence.** Logs cannot distinguish an acknowledged failure from interruption at the same phase. The top-level failure may remain visible, but the selected phase-attribution target is incomplete.

**Proposed correction.** Finish phase observations from the actual owned terminal result, including acknowledged failures. Keep drop silent and emit cancellation only after actual drainage. Avoid a drop guard that guesses the outcome.

**Closure evidence.** Exercise a production phase with an acknowledged failure and verify one `begin` and one `failed` terminal. Retain the interrupted-future case with only `begin`. The synthetic `Phase` tests alone do not establish production caller behavior. **not_run**.

## 5. Composition, execution and change scenarios

| Scenario | Owning route and assessment |
|---|---|
| Add an unrelated declaration | Model declaration and adapter construction change; fixed existing-row DDL does not expand. **Satisfied at static strength.** |
| Change a local sum | Typed canonical callback and reconstruction govern the changed meaning; flexible storage is not relied on for sum validity. **Satisfied.** |
| Corrupt complete physical envelopes | Actual rows are reconstructed and compared, including scope/body/key fields. **Satisfied for inspected validators.** |
| Later invalid original range in a window | All selected metadata/ranges are lowered before original persistence begins. **Satisfied.** |
| Extra owner, changed binding or pending contribution | Workspace/native final inventory comparison and pending rejection cover these distinctions. **Satisfied in owning admission workflows.** |
| Empty or alias-only canonical contribution | Completed owners drive pointer discovery; entity aliases expand one hop; empty scratch has an explicit empty cursor. **Credible inspected route.** |
| Two independent cursors; cancel one | Separate reopened readers and charges share scratch; worker observers retain terminality. **Satisfied structurally.** |
| Admitted final read overlaps seal | New prepared access rechecks global closure. **Violated: F03.** |
| Known-export inspection using separate resources | Unconditional capability construction rejects valid transport. **Violated: F01.** |
| Detached import or SQL fresh restore | Initial loading and independent admission remain; post-freeze canonical reload is removed. **Satisfied in owning workflows.** |
| Failure after marker acknowledgement | Returned failures preserve committed effects; cancelled callers lose retained identity. **Violated: F04.** |
| Coherent publication through public APIs | Exposed capability/sealing paths bypass admission. **Violated: F02.** |

### Analysis/projection record

| Question | Universe and method | Fidelity/model | Limits and evidence linkage |
|---|---|---|---|
| Which canonical rows belong to final content? | Complete completed-owner membership plus declared one-hop entity aliases; deterministic external ordering | Exact derived selection | Bounded windows and spill; no ready partial inventory; pointers require hydration |
| Do stored rows realize canonical content? | Actual graph/backing rows reconstructed through model-selected codecs | Exact physical equality, distinct from global semantic validity | Full-envelope comparison and original checks |
| Does restored content satisfy the frontier? | Actual restored completed bindings, producer inventory, facts premises and model publication checks | Exact under retained profile/model contracts | Independent admission; preparation supplies no validity authority |
| Do retrieval structures realize canonical search content? | Existing window/occurrence/vector lowering and reconciliation | Derived retrieval structures | Exact canonical inputs; no changed heuristic promoted into fact |

## 6. Library fit, alternatives and execution judgment

The selected composition is proportionate. SurrealDB 3.3 retains envelope types, uniqueness indexes and enforced role edges; typed Rust callbacks own semantic-body validity. Tokio/futures own asynchronous and shared preparation lifetimes, while tempfile and standard file readers own spill storage. The specialized ordering kernel preserves SDK value equality and conflict semantics instead of introducing a second query engine or canonical store.

The alternatives remain meaningful:

| Alternative | Judgment |
|---|---|
| Retain broad body unions and scope `VALUE` programs | Preserves immediate private SQL rejection, but retains unrelated-vocabulary amplification |
| Fixed envelopes plus selected ingress and reconstruction | Best inspected route for this target; requires the admission boundary correction in F02 |
| Per-kind tables or generated discriminator assertions | Adds vocabulary-dependent database machinery without a demonstrated current access consumer |
| Frozen pointer runs | Reuses stable final selection without mutable invalidation; appropriate here |
| Mutable generation cache | Adds invalidation ownership unnecessary for this final immutable lifetime |
| Different intermediate placement/data-only restore | Could reduce crossings, but introduces different recovery/transport obligations; neither is required to resolve these findings |

**A4 is satisfied for the selected correction at static strength.** Ordinary write enforcement no longer grows with unrelated vocabulary, and repeated final canonical readers share pointer discovery/order preparation. Spill storage provides a credible volume-growth route without a resident union set.

This does not settle candidate-by-contributor amplification, total batch live state, native frontier crossings or SQL staging/copy cost for larger workloads. Those remain their existing triggered investigations. No latency, throughput or capacity improvement is claimed.

## 7. Independent judgments and gates

Architectural verdicts and correctness gates are separate. “Pass” below means a scoped source-based gate judgment, not a passing test result.

| Architectural judgment | Verdict | Basis |
|---|---|---|
| **A1 — Localize change** | **Violated** | Adapter locality improves, but F01 couples transport inspection to unrelated publication readiness |
| **A2 — Encode domain meaning explicitly** | **Violated** | F02 fails to enforce admitted-versus-frozen authority; F04 loses a consequential committed-state distinction |
| **A3 — Extend through composition** | **Violated** | F01 and F03 break legitimate inspection and final-reader compositions |
| **A4 — Fit execution to workload** | **Satisfied, scoped/static** | Selected lowering and reusable pointer preparation remove identified structural amplification |

FP-01/02/03/04/05/06 are violated at the finding boundaries above, despite substantial supporting mechanisms elsewhere. FP-07 is satisfied for the selected physical correction. DP-01 and the inspected typed identity/reconstruction rules remain satisfied; DP-08/19/21 have the identified gaps.

| Gate | Verdict | Evidence/limit |
|---|---|---|
| G1 Authority | **Pass, scoped** | Model declarations remain semantic authority; physical forms are derived |
| G2 Semantic fidelity | **Fail** | Frozen and admitted authority are insufficiently distinguished in the exposed publication contract |
| G3 Validity | **Fail** | F02 exposes publication without the admission/reconciliation boundary |
| G4 Hidden behavior | **Pass, scoped** | Effects are explicit; known-export verification must remain inspection-only |
| G5 Consistency/recovery | **Fail** | F02 bypasses finalization; F04 loses acknowledged committed identity |
| G6 Transformation/reuse | **Fail** | F01 changes valid verification behavior; F03 breaks admitted-read lifetime |
| G7 Truthful capability claims | **Pass at Implemented strength** | No pending test or performance result is accepted as evidence |
| G8 Library leverage | **Pass, scoped** | Existing libraries realize the selected capabilities with bounded specialized ownership |
| CI-G1 Fidelity | **Unresolved for bypass APIs** | Owning wrappers retain fidelity admission; F02 bypasses the premise needed to extend that judgment |
| CI-G2 Evidence closure | **Unresolved for bypass APIs** | Normal/restored workflows retain closure, but raw sealing does not establish it |
| CI-G3 Evaluation integrity | **Pass, scoped** | Inspected changes introduce no gold/heldout path into production; evaluator execution remains pending |

## 8. Verification, uncertainty and disposition

| Obligation | Outcome |
|---|---|
| Compile checks for affected crates | **not_run** |
| F01 existing intact/tampered-export control | **not_run** |
| Focused native envelope, completion, preparation and lifecycle controls | **not_run** |
| Cancellation/committed-effect and production failure-phase controls | **not_run** |
| One populated native/MCP/evaluator boundary: `just verify --select serving:mcp` | **not_run** |
| New schema snapshot generation/acceptance | **not_run by this reviewer**; coordinator reports generation in progress |
| Applicable non-functional leaves | **not_run** |
| Deferred timeout campaign and assembled `just qualify` | **not_run; outside selected completion** |
| Performance measurements | **not_run; no performance claim** |

A suitable existing focused F01 route is:

```text
just verify --select compiler:producer --nextest-args "-E test(transported_graph_refuses_missing_and_tampered_originals)"
```

The coordinator should select the remaining affected controls and revealing lifecycle cases within the authorized boundary. No whole-family replay or additional qualification campaign is required merely because this review identified source defects.

**Rule impacts: none.** The remedies realize existing admission, immutable-read, one-shot seal, completion and diagnostic contracts. They do not require changing native placement, SQL staging transport, dependency policy or the selected acceptance scope.

The findings have distinct closure obligations. Removing the raw seal escape does not preserve committed effects; fixing transport inspection does not fix retained-read closure; improving phase terminals does not prove lifecycle correctness.

## 9. Scoped decision

**Revise the assembled implementation.** Preserve declaration-selected lowering, complete physical reconstruction, Workspace owner/binding fencing, immutable shared scratch, independent cursors, and the direct/detached publication separation.

The coordinator owns resolution of F01–F05 and the pending focused controls plus one populated native/MCP/evaluator boundary. Source correction and independent reassessment can settle architectural findings; execution receipts must establish their named runtime boundaries separately.

The enclosing persisted execution architecture remains **unqualified beyond this review’s scope**. This decision neither resumes deferred timeouts nor requires assembled `qualify`, and it makes no performance claim.

## 10. Independent reassessment of integration corrections

**Verdict: Revise, bounded to one new lifecycle finding and a residual F05 diagnostic gap.** F01–F04’s original defects are corrected in the inspected source. This is an independent static assessment dated **2026-10-09**, against `e98fb4a994a4b49050ccc3c851a98e8e8ca66b9f` plus the current dirty tree.

AGENTS, design-review skills, coordination routing, core/template 3.3, heuristics 1.0, profile 1.5 and relevant owners were read. No files were changed; builds, tests, generators and delegation were **not_run**. Endpoint binding changed during inspection; the final assessment includes its updated implementation.

**F01–F05 reassessment**

| Finding | Independent source assessment |
|---|---|
| **F01 — Transport inspection regression** | **Source-resolved.** Known-manifest inspection returns `CheckedTransport` without importing or requiring frozen native content. Detached verification separately loads and admits before constructing `VerifiedExport`. See [artifact.rs](../../../crates/cpg-core/src/artifact.rs). The intact/tampered-original control remains present; it was not executed. |
| **F02 — Publication admission bypass** | **Source-resolved.** Private `Admission` accepts only nominal core-owned artifacts, exports or restored admissions and obtains their paired native store/manifest. The marker writer is private; lifecycle guards expose no client or publication method. Session creation and sealing check retained endpoint/namespace identity; cold sessions lack publication location authority. See [native_publication.rs](../../../crates/lctx-publisher/src/native_publication.rs) and [target check](../../../crates/lctx-surrealdb/src/compiler.rs). Existing privileged Loader access is outside this supported publication route. |
| **F03 — Admitted canonical read rejected during closure** | **Original defect source-resolved.** Public preparation admits first; private descendant preparation checks failure/frozen readiness without reapplying global admission closure. Preparation and cursor workers retain scan ownership. See [preparation](../../../crates/lctx-surrealdb/src/compiler.rs). The cached-preparation control exercises that distinction; the first-owner control exercises its constituent ownership operations rather than the complete native discovery branch. New F06 below concerns a separate public exception. |
| **F04 — Acknowledged handle lost on cancellation** | **Source-resolved.** Checked marker acknowledgement records the pre-encoded exact handle in retained operation state before another await. Drain/refusal preserve it; abandon requires no committed effects. See [marker acknowledgement](../../../crates/lctx-publisher/src/native_publication.rs) and [abandon](../../../crates/lctx-surrealdb/src/compiler.rs). Record-then-drop and unknown-drop controls exist, but do not execute cancellation inside production marker/readback. |
| **F05 — Failure phases appear incomplete** | **Substantially corrected, still open.** Admission, preparation, publication, compiler and CLI result owners now finish failure results; `Phase` drop remains silent. Native setup still has an early-return gap described below. |

**F06 — Public post-closure reads lack a retained parent**
**Medium; FP-02/05, DP-08/19/20; A2/A3, G3/G5/G6.**

[completed_state_after_closure](../../../crates/lctx-surrealdb/src/compiler.rs) is public and checks only the permanent `seal_started` flag. It neither requires a live finalization guard nor registers parent ownership before its first await. Its descendants use [track_rows](../../../crates/lctx-surrealdb/src/compiler.rs), which deliberately permits closed-state execution.

After a seal attempt ends, a fresh caller can therefore start this read despite global closure. On an acknowledged pre-marker failure, cleanup can observe zero active operations and proceed while this unregistered caller subsequently starts descendants. This defeats drainage’s closed-admission premise. The production publisher holds its finalization lease correctly; the defect is the exposed exception.

**Proposed correction:** require a live, same-store finalization parent covering the entire operation, with no unguarded public route. Closure evidence should establish refusal without that parent and retained ownership across the read’s awaits.

**Residual F05 — Native setup authentication failure has no terminal**
**Medium; FP-06, DP-21.**

[native setup](../../../crates/lctx-surrealdb/src/compiler.rs) emits `begin`, then authenticates through `await?` before entering the result block that emits terminals. An authentication rejection returns `Err` and silently drops the phase, making a completed failure look interrupted.

**Proposed correction:** encompass the complete setup result in terminal reporting, preserving existing cleanup and silent cancellation behavior.

**Adjacent authority and execution assessment**

Content freeze drains mutators and compares actual owners/bindings under Workspace’s completion gate. Before freeze, canonical readers discover pointers per read; afterward, headers and payloads share immutable preparation with independently charged cursors. Pointer reuse does not replace hydration, core semantic admission, or final complete physical graph/state/original/search reconciliation.

Declaration-selected lowering and immutable spill preparation remain a credible execution correction at static strength. Core semantic admission, native physical lifecycle and publisher visibility remain coherent responsibilities; neither finding requires another semantic validator or hostile-root SQL protection.

**Bounded judgments:** A1 satisfied; A2/A3 violated at F06; A4 satisfied for the selected correction at static strength. G3/G5/G6 fail at the identified boundaries. Other inspected gates and CI-G1–CI-G3 retain scoped source support, without certifying broader fidelity, evaluation or serving.

Focused native/core/publisher controls and `just verify --select serving:mcp` remain **not_run** here. No runtime, Tested or Measured conclusion follows. Rule impacts: none; the enclosing architecture remains unqualified beyond this review’s scope.

## 11. Final independent reassessment

**Accept scoped at static / Implemented strength, 2026-10-09.** Against `e98fb4a9` plus the inspected dirty tree, F06 and residual F05 are source-resolved. **No remaining material findings or regressions were identified within this reassessment.**

- **F06:** [completed_state_after_closure](../../../crates/lctx-surrealdb/src/compiler.rs) requires a borrowed `&OperationLease` and rejects non-finalization, finished or different-store parents before its first await. Finalization origin is a private flag set by `begin_finalization`; an operation label cannot supply it. The borrow prevents consuming the parent while the read remains live, and its active ownership blocks drainage across the read’s awaits. [Publisher sealing](../../../crates/lctx-publisher/src/native_publication.rs) passes its own retained lease and finishes it after the complete result. The [source control](../../../crates/lctx-surrealdb/src/compiler.rs) rejects label spoofing and another-store ownership, observes pending read/drain, and confirms drain remains pending after read cancellation until the parent finishes. It does not exercise a completed native query.
- **Residual F05:** [Native begin](../../../crates/lctx-surrealdb/src/compiler.rs) wraps the entire private `begin_inner` result, including authentication refusal and setup cleanup, before emitting its terminal. The [fixture-backed control](../../../crates/lctx-surrealdb/tests/compiler_lifecycle.rs) supplies an incorrect password and asserts begin/failed observations without password disclosure; it was not executed here.
- **Adjacent restore diagnostics:** [Restore](../../../crates/lctx-publisher/src/backup.rs) owns an outer result through cleanup, with local result terminals for staging admission, canonical copy, derived references and staging cleanup. Early errors reach the outer terminal. [Phase](../../../crates/lctx-surrealdb/src/phase.rs) still emits no terminal on drop, preserving interruption as incomplete.

F01–F04 retain their earlier **bounded source-resolved verdicts and control limitations by reference** to [followup.md](#10-independent-reassessment-of-integration-corrections); they were not re-reviewed.

Under core/template 3.3, heuristics 1.0 and profile 1.5: **A1–A4 satisfied for these corrections**, with constant parent validation and coarse observations preserving existing execution responsibilities. The identified G3/G5/G6 failures are resolved at source strength. Other gates retain the referenced review’s bounds. Rule impacts: **none**.

Edits, builds, tests, generation and delegation: **not_run**. No runtime, Tested or Measured claim follows; enclosing architecture and release qualification remain outside this verdict.

## 12. Independent reassessment of captured phase emission

**Accept scoped at static / Implemented strength, 2026-10-09.** No material defects found in the final diagnostics correction on dirty main at `e98fb4a994a4b49050ccc3c851a98e8e8ca66b9f`, against plan §3.5 and assembled-review F05.

- [phase.rs:48](../../../crates/lctx-surrealdb/src/phase.rs) registers metadata with the captured dispatcher, checks `enabled`, and sends through `Dispatch::event`. Pinned tracing-core **0.1.36** confirms that this also checks `event_enabled`. The four declared fields match their values; begins omit elapsed duration.
- Captured dispatcher and retained span provide consistent subscriber routing and explicit parent attribution. [native.rs:12](../../../crates/cpg-core/tests/fixtures/native.rs) restores both around the thread’s `block_on`; adjacent task handoffs retain poll-scoped attribution.
- [artifact.rs:1050](../../../crates/cpg-core/src/artifact.rs) finishes facts, invariants and frontier observations before propagating errors. Enclosing observations also receive failures. Consuming finish prevents duplicate terminals; silent drop preserves incomplete work. `Returned` remains distinct from semantic success, and cancellation is never fabricated on drop.
- Observations remain coarse, outside row loops, with no added payload or credential fields. Existing stderr/run-log routing remains intact. [compiler_provider.rs:703](../../../crates/lctx-surrealdb/src/compiler_provider.rs) is a behavior-preserving private formatter rename.

The reported binary suppression is an observed symptom; its underlying tracing/library/compiler cause remains **unproven**.

Runtime acceptance remains separate: the four source-probe passes and earlier native authentication-control pass are supplied evidence, with the latter predating this emission fix. The rebuilding cross-crate core control is pending. Builds, tests, generators, edits and delegation were **not_run** here. This conclusion establishes neither current runtime closure of F05 nor performance or broader qualification.
