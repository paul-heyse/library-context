# Catalog execution — integrated target design review

**Decision: Accept scoped at Implemented / Interface-checked strength, 2026-10-13.** The reviewed source provides a coherent Catalog execution route through earned contributions, internal selection, frozen compound boundaries, independent admission, exact transport, publication and native evidence access. One material residual amplification was discovered during this review, `catalog-integrated#F01`. Its bounded replacement was independently reinspected and accepted at source strength; the finding and original diagnosis remain below.

**Subsequent source correction, 2026-10-14:** [§9](#F02) records `catalog-integrated#F02`, scalar detached-import effects missed at the initial CU03/A4 assessment, and independently accepts their bounded replacement at source strength. The original cutoff and its runtime limits remain dated; the follow-up does not establish functional acceptance.

This decision does **not** establish CE01 functional acceptance. Actual generated Catalog analysis, admission, publication and linked native queries remain unproven at this review’s cutoff. Focused controls, earlier cold restoration and source review retain separate scopes. The [Catalog coordinator §6](../../plans/catalog-execution-plan_2026-10-12.md#6-sole-disposition-and-execution-checkpoint) alone owns current dispositions, execution receipts and CE01–CE03 completion.

## 1. Review contract and evidence boundary

| Field | Assessment |
|---|---|
| Subject | Assembled CP00–CP04, CU01–CU04 and OP01–OP03, including adjacent admission, publication, native queried analysis, detached/cold receipts and owned cleanup |
| Tier · purpose | Design · target |
| Standard | Core/template 3.3; efficiency heuristics 1.0; code-intelligence profile/review additions 1.5; repository binding |
| Reviewer | Independent delegated design reviewer |
| Date | 2026-10-13 local; investigation continued into 2026-10-14 UTC |
| Baseline | Main `35053d52c99f622a09bfc66b7a5aa76311cad5ab`, plus the specifically inspected concurrent changes listed below |
| Evidence strength | Implemented and Interface-checked source assessment. Reviewer builds, tests, probes, service operations and runtime qualification: **not_run** |
| Disposition owner | [Catalog coordinator §6](../../plans/catalog-execution-plan_2026-10-12.md#6-sole-disposition-and-execution-checkpoint) |

The functional target is a version-pinned API/evidence catalog that supports capability discovery, exact public API/configuration selection, and traceable invocation/deployment evidence with explicit limitations. The [product target](../../design/sections/api-and-evidence-product.md#section-14) and code-intelligence profile govern this assessment. Accepted ADRs, current plans and operational rules explain the subject; agreement with them is not itself the acceptance criterion.

The workload premise is one operator with concurrent compiler/read operations over multi-output contributions, overlapping dependency closures, original documents, exact relation selectors, retained products and immutable publications. Content volume, relation count, overlap, original size and alias degree can grow independently. Complete analysis may require a larger universe than its output selector. Resident preparation, temporary conversions, native requests and retained backing must remain coherently owned; spill and explicit refusal are legitimate mechanisms, while refusal alone does not establish execution fit.

The review examined the relevant architectural owners, all three companion plans, original Catalog F01–F08, original lifecycle F01–F05 and relevant foundation/completion follow-ups. Decisive current source was inspected directly. C01–C11 were considered as the existing complete alternatives and reconsideration routes; unchanged platform research was not repeated without a changed premise.

External embeddings and MCP runtime were not executed. Their exclusion from this execution campaign does not certify those architectural boundaries or make the profile’s broader answering, behavioral-analysis and evaluation workloads irrelevant. This is acceptance of the named Catalog execution architecture and touched preservation contracts, not the entire product, research backlog or all serving journeys.

### Identified concurrent source

The inspected changes include:

- Compiler `views_inner` future erasure and bounded dependency-frontier observation windows, including named `PendingView::_charge`.
- Normalization stage future erasure and synthesis ordered membership/emission intersection.
- Native immutable-creation lookup using one borrowed wanted-ID index.
- The relation-selective preparation correction for this review’s F01.
- Frozen transport control lifetime corrections.
- Removal of the dormant legacy vector-index cleanup branch.

Decisive final Git blobs are:

| Source | Inspected blob |
|---|---|
| `lctx-native/src/installed_view/audit.rs` | `531e7a7ab871813d77a3e4825ce66b9888f4e6fe` |
| `lctx-compiler-store/src/compiler.rs` | `20d70c87d4c8649d834fd5f9b6d74d68b07dca57` |
| `lctx-native/tests/compiler_views.rs` | `205db464293c22e47476344208024aa2c753a5be` |
| `cpg-core/src/normalize/pipeline.rs` | `7dcb3f9f18607bf46fcd7f81176582b756ce01d0` |
| `cpg-core/src/synthesis.rs` | `94ba555e89cc8ef624bb06c949d3ce580fc74cf5` |
| `lctx-native/src/control/coordination.rs` | `f88faaa76e3fb31c788fae4212b30911560c8f62` |

F01’s final `audit.rs` SHA-256 is `dad227dfbd5b2897788dedb4fb17540bf693f53d4bb0f5be85415f3c2c972160`. Material subsequent changes require assessment of their affected evidence.

## 2. Domain meaning, responsibilities and dependencies

The central improvement is a separation between semantic view meaning and the physical authority needed by its consumer.

| Owner | Responsibility and hidden decisions | Consumer contract |
|---|---|---|
| `lctx-model::domain` | Contribution specifications/outcomes, declared relations, views, bindings, coverage, semantic identities, transport correspondence and eligibility | Typed meaning and owned validation; neither a digest nor a decoded descriptor grants native authority |
| `cpg-core` | Pinned producer orchestration, normalization, Catalog/analysis composition and programmatic synthesis | Consume declared completed inputs; preserve nominal roles, uncertainty and evidence |
| `lctx-compiler-store` | Attempt ownership, exact physical mappings, admission, freeze and artifact handoff | Earned internal selections and privately constructed admitted artifacts |
| `lctx-native` | Protected physical copies/incarnations, actual observations, ordered preparation, native effects and terminality | Exact selected content under a live owner, or independently earned retained/frozen authority |
| Publisher/restore | Immutable publication, exact export/backup and independent destination reconstruction | Retained expectations plus actual incoming/stored validation |
| Serving | Model-owned selection, ranking and evidence presentation | Nominate through indexes, then hydrate through exact protected selection |
| Host/run/service/storage owners | Process identity, target containment, current quiescence, disposal and diagnostic retention | Independent local, native, storage and committed-effect outcomes |

The source realizes three different operations:

1. Independently audit and protect an immutable contribution.
2. Select an exact internal relation view beneath its live preparation owner.
3. Earn an independently necessary retained or frozen compound boundary for admission, publication, retention or detached/cold consumption.

[Earned selection](../../../crates/lctx-native/src/installed_view/earned.rs) retains the actual audit pin, expected contributor rows, physical/incarnation correspondence and prepared members. Its construction executes closure, membership and composed alias checks before exposing the capability. The representation is not a serializable grant.

[Compound earning](../../../crates/lctx-native/src/completed_state/earning.rs) independently assures selectors and copies, retains complete contributor/selector inventories, and performs final completion after child inventory verification. Logical views remain explicit without reinstating a durable member namespace for every internal relation.

[Artifact admission](../../../crates/lctx-compiler-store/src/artifact.rs#L1737) freezes content, executes facts availability, semantic invariants and frontier checks, prepares canonical graph content, checks graph admission and constructs the actual manifest. This is an identifiable governing operation, rather than a “completed” label interpreted independently by publication.

### Fact and fidelity map

| Family | Provider/fidelity and identity | Coverage and consumers |
|---|---|---|
| Provider facts and original evidence | Pinned provider observations, with their declared extracted/resolved fidelity, producer/configuration/model and original source identity | Coverage, explicit empty outputs and unsupported/unrequested outcomes survive completion; individual extractors were not requalified here |
| Normalized and Catalog relations | Model-owned derived interpretation over attributed inputs; nominal relation/key identity remains explicit | Normalization, Catalog selection/evidence and admission |
| Analysis and synthesis | Exact, conservative and heuristic meanings remain distinct under their owning model/settings; assertions are programmatic | Selected analysis/frontier and S0 consumers retain evidence and qualification |
| Completion/view/binding/alias inventories | Exact state and correspondence metadata, not new facts or semantic proofs | Internal selection, retained/frozen completion, transport and publication |
| Native query/evidence results | Pinned publication and exact selected membership, with model-owned interpretation | Get/search/compare and original evidence; current generated Catalog runtime remains pending |

No inspected change relabels provider inference as runtime behavior, collapses relationship identity to endpoint pairs, or makes empty incomplete coverage mean absence.

## 3. Contracts and physical composition

### Canonical preparation and assurance

[PreparedRows](../../../crates/lctx-native/src/ordered_rows.rs#L1001) supplies immutable memory/disk backing, independent cursors and non-consuming ordered union. Equal keys with equal payloads deduplicate; unequal payloads refuse. Logical normalization precedes canonical ordering. Complete family identity is produced only after required EOF and final checks.

[Family preparation](../../../crates/lctx-native/src/installed_view/audit.rs) retains exact contributor runs and keys family reuse according to their actual dependencies. Membership/backing reuse includes protected physical incarnations; view and binding meaning remains explicit. A changed composed set still requires its proper flat canonical union and fold. The architecture does not claim that fragment digests concatenate into the v4 root.

[Expected-record windows](../../../crates/lctx-native/src/control/held.rs#L405) issue actual record-set and guard-set queries. Identity correlation rejects foreign/duplicate results and missing guards. Optional discovery absence remains distinct from a required-record refusal. Hold windows use exact owner/object incidences. These are observations: later effects still check their consumed premises atomically.

The current compiler dependency-frontier change preserves this distinction. Slots retain their independent logical/output/incarnation checks, while physical observations deduplicate within bounded windows and reuse the operation-local observed map. Pending descriptors keep their reservation owner; frontier capacity remains owned while borrowed slots are processed. Newly discovered dependencies enter the next frontier.

[AliasTopology](../../../crates/lctx-native/src/alias_topology.rs) prepares directed source access and processes newly reached nodes. Visited runs merge incrementally rather than reconstructing complete history each round. Selection returns its reachable subset; audit additionally refuses unreachable owned sources. Cross-contributor paths and disconnected cycles therefore retain different meanings.

### Consumer preparation

CU01’s [restore preparation](../../../crates/lctx-publisher/src/restore.rs#L986) collects demanded originals, then prepares layouts, source associations and required blobs through table passes. It preserves duplicate/foreign-row and missing-content refusal. SQL claims are compared against independently checked transport content; imported SQL is data, not executable authority.

CU02’s [PreparedPositions](../../../crates/lctx-compiler-store/src/scoped_positions.rs) binds exact backing, input ports, order, root namespaces and resource owner. It retains all matching positions and restores source batch/row order before Arrow gathering. Repeated nominal relation ports remain distinct. Borrowed lifetimes prevent applying positions to replacement backing.

Actual rich consumers have migrated beyond the primitive: Evidence, Selection and Synthesis use ordered selected keys and nominal row lookup; LocalSemantics deliberately merges its declared ports while deduplicating overlap. SourceCall and model windows use prepared column positions. Necessary root-specific builder/checker state remains independent.

CU03’s [compiler identity/export](../../../crates/lctx-compiler-store/src/compiler.rs#L6443) separates direct logical identity from portable preparation. Export requires frozen or retained read-only content, prepares emitted content, independently checks its logical identity against the protected boundary, and only then writes transport.

The [transport model](../../../crates/lctx-model/src/domain/completed_transport.rs) distinguishes semantic v4 identity, exact envelope correspondence and conservative selector eligibility. Root-qualified slots preserve distinct equal-descriptor copies. Eligibility excludes incidental physical labels/sharing while retaining relevant root-local alias meaning. Receipts bind expectations; destination import still earns fresh authority.

CU04’s [Spec](../../../python/lctx_mcp/src/lctx_mcp/embedder.py#L63) uses checked factory construction, recursively immutable parsed values and one retained Rust recipe response. Properties do not recreate the identity recipe. This is an adapter correction with a local owner, not a global cache or an embedding-service qualification.

### Analysis records

| Operation/question | Universe and method | Precision, limits and evidence |
|---|---|---|
| Complete logical identity | Exact selected contribution/view/binding/alias closure; canonical ordered family fold | Exact deterministic identity; resource/terminal refusal cannot yield completed roots |
| Alias reachability | Directed selected owned aliases and required seeds; prepared adjacency/frontier | Exact least reachable set; audit’s all-owned-source obligation remains separate |
| Root-scoped analysis/checking | Model-declared input closure and private root namespaces; prepared positions/ordered nominal lookup | Required intermediates remain in the input universe; independent checker state and model/settings survive |
| Catalog/S0 construction and native evidence | Declared Catalog/analysis parents, programmatic assertion/evidence linkage, exact selected hydration | Coverage and qualification remain explicit; heuristic nomination does not establish eligibility or truth |

### Lifetime and recovery

[WorkspaceExecutionOwner](../../../crates/cpg-core/src/workspace.rs#L40) retains spill files, bridge, provider joins, native calls/store, cache session and original signal across artifact handoff without retaining the entire read catalog. [AdmissionRuntime](../../../crates/lctx-compiler-store/src/admission.rs#L672) delegates drainage and signal completion to that actual owner. Generic borrowing does not itself finalize the lifetime.

Installation preparation workers retain their join ownership across cancelled drainage. Closing preparation drains accepted work before releasing its pin. Failed terminal work retains uncertainty rather than publishing success.

Original completion uses typed `PremiseChanged` recapture under its existing retry/cancellation owner. It preserves the original incarnation and independently rechecks bytes and layout protection. This is a credible bounded-peer convergence route. Guaranteed progress under indefinitely sustained equal replay remains excluded and retains IF04’s demand-triggered reconsideration; it is not claimed to have disappeared.

Typed native diagnostics preserve request/attempt/lifecycle identity, issuance and bounded secondary causes through the signal projection and host readers. Diagnostic routes do not grant release authority.

Owned campaign disposal uses fresh authenticated current participants and executor state before ownership transfer or physical effects. Confirmed whole-target discard can settle that target’s disposal obligation while preserving original failed/Unknown operations. This is appropriate for disposable targets and does not authorize shared/operator deletion.

## 4. Change, growth and failure scenarios

| Scenario and kind | Ownership and propagation | Judgment |
|---|---|---|
| Add a deployment-evidence or output relation, including an empty one — domain extension | Model/provider declaration owns new meaning, coverage and associations. Completion and selection consume those declarations | Credible after F01’s selective preparation correction. A new relation no longer requires decoding every unrelated membership for selector preparation |
| Add another root selector or binding — binding/composition | Exact view, dependency roles and selected physical mappings change; selector preparation and eventual frozen identity follow | Internal selection reuses protected preparation. Binding changes do not silently grant detached authority |
| Grow overlapping views and content — workload growth | Contributor preparation follows unique protected content; changed unions still perform necessary composition | Full-family reconstruction and durable realization are no longer automatic internal-view prerequisites; remaining selected projection work has an actual consumer |
| Cross-contributor chain, cycle or high-degree alias — topology/skew | Alias owner defines direction and completeness; prepared adjacency and visited frontier execute it | No output filter substitutes for the needed universe; disconnected owned aliases remain an audit refusal |
| Replace relation partitions with compact ranges/offsets — mechanism substitution | Native preparation owns the derived access layout; canonical identity and selector contracts remain | Credible future substitution. It must preserve exact source lifetime, ordering, equality and failure behavior; no backend framework is needed now |
| Replace native storage with immutable files/index — broader mechanism alternative | Publication, shared retention, exact selected queries and recovery would all need equivalent owners | Plausible for a smaller product, but not presently a simpler complete replacement for the examined connected Catalog route |
| End the producer Workspace before artifact use; then cancel final drainage — lifetime/failure | Retained execution owner and resumable join ownership preserve actual outstanding work | Source route is coherent; dropping a caller or observing process exit does not establish native completion |
| Freeze, export, import and then challenge mutable replay — transformation/lifetime | Frozen boundaries own identity/export; a fresh mutable attempt owns the mutation challenge | Corrected native control preserves both contracts without reopening frozen state |

The `pending_overlap_frozen_selection_and_state_transport` control originally requested completed-state authority while content was mutable. Its inspected correction preserves the unbound-root observation through `views`, explicitly asserts mutable completed-state refusal, and freezes source and receiver before completed identity use. Equal replay and corrupt-membership refusal remain in a separately owned mutable attempt, with unwind finalization and observer invalidation. This is source-correct; its fresh runtime result remains pending.

## 5. Findings and source correction

<a id="F01"></a>
### catalog-integrated#F01 — Relation selectors decoded complete contributor membership runs

**Original diagnosis: violated FP-07/A4 and DP-10; relevant H5/H9/H12/H15/H17.**

At the initial inspected baseline, `LogicalContributorRows` retained one complete all-relations membership run. Every first distinct selector opened that complete run in `selected_prepared_member_result`, decoded all its rows, tested the requested relation inside the scan, then rekeyed/sorted selected members. Production internal selection and compound boundary assurance consumed this route.

A contributor with populated relation A and declared-empty relation B therefore required another complete membership scan to select B. Increasing demanded relation kinds multiplied unrelated decoding despite removing the former full-six-family and durable per-view lifecycles. Caching identical selectors did not amortize distinct relation selections.

This established avoidable structural amplification. It did not establish that this operation dominated an earlier timeout or imply a measured speed ratio.

**Correction reinspected and accepted, 2026-10-13, Implemented / Interface-checked.**

[PreparedMembershipRelations](../../../crates/lctx-native/src/installed_view/audit.rs#L596) now makes one complete canonical pass and retains relation-specific prepared runs beneath the exact contributor preparation. The original canonical membership run remains unchanged for v4 family hashing. [Selector composition](../../../crates/lctx-native/src/installed_view/audit.rs#L4747) receives only the requested relation runs and preserves selector rekeying, equal deduplication, unequal collision refusal, exact count and terminal checks.

Missing relation preparation can mean empty only beneath the existing whole-output audit and final contributor/context checks. The correction does not infer completeness from an unaudited map miss.

The final source charges provisional and retained sparse B-tree nodes, relation keys, Arc overhead and run lifetimes. Partitions preserve `MAX_ROW_BYTES`. Cancellation or failure returns no completed preparation, and shared publication still follows the original worker’s final exact observation and context checks.

Copying canonical membership payloads once buys selective access through existing spillable run ownership. Compact pointers/ranges could reduce retained bytes, but would add another lookup or offset seam. The selected correction is proportionate to the actual consumer and removes the demonstrated repeated work. Its benefit warrants this bounded additional representation; speed and capacity remain unmeasured.

Three pure controls were inspected:

- `relation_preparation_selects_only_own_rows_preserves_full_hash_and_large_frames`
- `relation_selector_preserves_overlap_dedup_and_unequal_collision_refusal`
- `relation_preparation_refuses_late_cancellation_and_resource_without_publishing`

They exercise selective access, full-run equivalence, empties, last-borrower ownership, overlap/conflict and late refusal. Reviewer execution: **not_run**. The coordinator owns their results and the enclosing native/Catalog acceptance.

**Current disposition:** source correction accepted by this review; execution disposition and runtime closure belong solely in [coordinator §6](../../plans/catalog-execution-plan_2026-10-12.md#6-sole-disposition-and-execution-checkpoint). This residual manifestation relates to original `catalog#F01/F02`; it does not replace their stable IDs or close their distinct acceptance obligations by association.

### Original finding coverage

| Source findings | Current source assessment |
|---|---|
| Catalog F01/F02; lifecycle F03/IF10 | Earned internal selection, reusable canonical runs and compound retained/frozen boundaries address the original physical expansion. This review’s F01 identified and corrected the remaining relation-access amplification |
| Catalog F03; lifecycle F02 | Actual identity-correlated record/guard/hold windows and bounded backing/endpoint inspection replace the diagnosed scalar observation structure |
| Catalog F04 | Prepared directed adjacency/frontier preserves cross-contributor reachability and selection/audit distinctions |
| Catalog F05 | Demanded restore preparation removes original/source/blob lookup-by-repeated-whole-spool scanning |
| Catalog F06 | Prepared exact input-port positions and actual rich consumer lookup replace repeated union-mask discovery |
| Catalog F07 | Direct identity and frozen prepared export separate semantic hashing from unnecessary transport work |
| Catalog F08 | Immutable Spec retains one Rust-owned recipe response |
| Lifecycle F01 | Serving discovery/evidence delegates candidate payload hydration to native exact selected membership rather than reconstructing the removed member namespace |
| Lifecycle F04 | Typed durable failure projection retains recoverable causality |
| Lifecycle F05/IF04 | Typed same-incarnation recapture supports bounded peers; sustained replay’s separate limitation remains explicit |
| Relevant host foundation findings | Current-quiescence authentication, exact target containment, independent outcomes and protected cleanup remain coherent within inspected changed seams |

These are dated structural assessments. Current execution dispositions remain in the coordinator; earlier review acceptance and implementation existence do not establish new runtime passes.

## 6. Library fit, alternatives and execution economy

The resolved families inspected in `Cargo.lock` include DataFusion 55.1.0, Arrow selection 59.3.0, BLAKE3 1.8.6, patched SurrealDB 3.3.0 and development-only SurrealKit 1.0.0-beta.1.

| Capability | Comparison and choice |
|---|---|
| Exact ordered native union | Existing prepared runs fit native `RecordId` ordering, payload preservation, conflict refusal, spill and terminal ownership. A generic relational merge would still require those domain adaptations. Retain the bounded existing mechanism |
| Root partition gathering | Arrow’s `take_record_batch` supplies the physical gather. Domain code owns nominal ports, positions, universe and resource/lifetime binding |
| Relation-selective membership | One prepared partition layout removes repeated unrelated decoding. Compact offsets remain a credible refinement if retained bytes become the material constraint |
| Alias reachability | Existing indexed runs/frontier suit spillable exact closure; model/native code retains directed alias and audit semantics |
| Semantic identity | BLAKE3 supplies hashing; the model owns the canonical six-family recipe. Changed-set union remains necessary |
| Native selection and assurance | Indexed set operations preserve useful database capability. Atomic receiving effects and ownership checks remain distinct from bulk observations |
| Development schema tooling | Generated assets feed the selected beta; actual target/session containment and disposal remain repository-owned because the library does not discharge those host obligations |

The current route removes full-state identity and durable physical lifecycles that were imposed solely by internal semantic views. It also removes unnecessary transport detours and repeated lookup/discovery work. It retains full contributor audit, exact selected projection, independently installed destination validation and final native CAS because they protect different uncertainty or trust transitions.

The BoxFuture changes allocate at complete stage/operation boundaries and preserve lazy work and owned destruction. The creation-index change uses borrowed wanted identities and retains indices until validation, preserving declaration-order refusal and first equal payload selection. The synthesis intersection consumes the exact ordered namespace/key contracts. These are local execution choices, not new semantic authorities or workflow machinery.

The dormant legacy vector-index cleanup branch has been removed. The current materialization blueprint still declares the same `neighbor` HNSW index; removing the obsolete cleanup branch therefore preserves that capability. This is a CE02 deletion of compatibility machinery, not a vector architecture change, embedding qualification or reason to reopen C05.

No new scheduler, generic preparation framework, durable proof cache, global compile lock or optional platform rollout is needed by the examined corrections.

## 7. Independent judgments and gates

### Foundations and supporting rules

FP-01–FP-06 are **satisfied within the inspected scope**: semantic owners, derived physical layouts, capability construction, effects and lifetimes are identifiable; consumers compose owned operations rather than privately reconstructing their meaning.

FP-07 is **satisfied for the named scenarios after F01’s source correction**. Necessary contributor audit and selected projection remain, while demonstrated repeated full-input work has selective or reusable routes. This is a qualitative execution assessment, not measured performance or capacity qualification.

Relevant DP-01–DP-05, DP-07–DP-12 and DP-18–DP-24 are satisfied at the inspected boundaries. DP-06/DP-13–DP-17 are supported by the retained ordinary operation/library composition and deletion of replaced paths. CI-01–CI-11 and CI-13 preservation obligations are satisfied within the touched representation, selection and lifecycle boundaries. Evaluation integrity itself was not changed or requalified.

| Gate | Verdict | Evidence and limit |
|---|---|---|
| G1 Authority | Pass, scoped source | Model-owned semantic identity/eligibility; native physical authority; private admission construction |
| G2 Semantic fidelity | Pass, scoped source | Explicit empties/outcomes, exact root/copy correspondence, preserved nominal ports and aliases |
| G3 Validity | Pass, scoped source | Executed admission route, exact observation refusal, independent incoming/stored checks |
| G4 Hidden behavior | Pass, scoped source | Pure preparation and immutable recipe reuse remain explicit; diagnostics do not acquire effect authority |
| G5 Consistency/recovery | Pass, scoped source | Freeze, inventory/finality checks, retained execution owner, cancellable drainage ownership and independent cleanup axes |
| G6 Transformation/reuse | Pass, scoped source | Exact dependency keys, unchanged canonical run/order, selective position/partition mappings and fresh receiving authority |
| G7 Truthful claims | Pass, scoped source | Source maturity and runtime limits are explicit; no timeout, cold pass or cleanup result is promoted to Catalog qualification |
| G8 Library leverage | Pass, scoped source | Existing library/native mechanisms provide generic capability; bespoke code covers identified semantic/ownership gaps |
| CI-G1 Fidelity | Pass for touched boundaries | No relation relabeling, unknown-to-absence conversion or heuristic-to-proof promotion identified |
| CI-G2 Evidence closure | Pass for touched boundaries | Exact selected hydration, graph admission and preserved same-boundary evidence; full generated serving journey remains unqualified |
| CI-G3 Evaluation integrity | Not applicable to changed behavior | Evaluation/reference handling was not changed or comprehensively inspected; no evaluation qualification follows |

| Architectural judgment | Verdict | Basis |
|---|---|---|
| A1 Localize change | Satisfied | Domain declarations, physical preparation, orchestration and host ownership have coherent reasons for change |
| A2 Encode domain meaning | Satisfied | Operations govern contribution/view/binding/copy/eligibility/admission/lifetime distinctions; data-only descriptors do not grant authority |
| A3 Extend through composition | Satisfied | New selectors and consumers compose earned selection, frozen boundaries and independent admission without automatic durable per-relation lifecycle |
| A4 Fit execution | Satisfied after F01 source correction | Shared preparation, relation-selective access, native windows, frontier aliases and exact partition lookup provide a credible route for the stated growth/failure scenarios |

## 8. Verification, rule impacts and decision

Reviewer source inspection and baseline identification used read-only file reads, `rg`, `git diff`, `git hash-object` and `sha256sum`. Reviewer builds/tests/probes/services: **not_run**, as assigned.

The coordinator records earlier scoped cold `922690` and Clippy/pure-control evidence. Those are attributed dated receipts, not executions by this reviewer or qualification of the newly changed tree. The cancelled Catalog `4c15ff` remains interrupted evidence. At this review’s cutoff, new relation controls and corrected frozen transport controls were queued or compiling; current Facts/cold continuation and generated Catalog acceptance remained separately owned.

The useful remaining evidence is already defined by CE01: focused changed-boundary controls, independently specified canonical/copy/absence cases, genuine generated Catalog analysis through independent admission and unselected publication, populated native get/search/compare, exact original evidence and truthful enclosing cleanup. A numerical benchmark is not required to accept the source architecture. Quantitative speed/capacity claims would require their own suitable measurements.

**Rule impacts: none.** The recommendations and F01 correction fit the current functional target and accepted preparation/freeze/transport distinctions. No new RC identifier, policy exception, platform prohibition or ADR is required by this review. The frozen-control adjustment repairs an outdated consumer assumption; the vector cleanup deletion removes a dormant compatibility path while preserving current schema meaning.

**Bounded decision: Accept scoped, Implemented / Interface-checked.** F01’s initial architectural violation is retained as a dated finding, with its inspected source correction accepted. No further material architectural defect was established in the examined assembled route.

**Enclosing architecture:** accepted at source strength for the named Catalog execution, consumer preparation, admission/transport/publication and owned-lifetime scenarios. Current runtime functional acceptance, CE02 current-state cutover/disposal, CE03 reconciliation, external embedding/MCP execution, wider research/evaluation workloads and operator adoption remain outside this acceptance.

The coordinator’s next consequential action is to obtain and assess the queued focused results and actual generated Catalog journey on the integrated tree, preserve any failure’s phase and owned state, then complete CE02/CE03 only after their named prerequisites hold. A material failure or changed source premise reopens its affected boundary; source acceptance cannot convert an interrupted or uncertain operation into success.

## 9. Detached-import effect windows — source follow-up

<a id="F02"></a>
### catalog-integrated#F02 — Detached import lowered set-shaped inventories into scalar native effects

**Follow-up, 2026-10-14. Original diagnosis: violated FP-07/A4 and DP-10; related to original `catalog#F03` and CU03. Correction accepted at Implemented / Interface-checked strength.**

This is a correction of the principal review’s initial CU03 execution-fit assessment. The scalar effects were present in its examined source; they were not introduced by the replacement. The initial broad source acceptance missed this remaining amplification. Its affected A4 assessment required revision until the bounded correction below was independently reinspected. The original review and its cutoff remain recorded rather than silently rewritten.

### Source baseline and diagnosis

The original detached-import implementation is preserved in main `35053d52c99f622a09bfc66b7a5aa76311cad5ab`, `crates/lctx-compiler-store/src/compiler.rs`, Git blob `9554ad96d08d2149b548d9a6d0a40b4ef9bc2a0e`, SHA-256 `aea845caae3423fa3810a168d7a6edf6bfb6e4b581026195968ceb91160cd669`. Those import/register seams were unchanged in the principal’s identified dirty compiler blob `20d70c87d4c8649d834fd5f9b6d74d68b07dca57`; its other concurrent changes concerned dependency mapping and future erasure.

`import_state_inner` received complete copy and alias inventories, but `register_transport_copy` issued one `ensure_rows` and one attempt-to-copy `hold_many` effect per copy. The later `hold_contribution_inputs` loop repeated each attempt-to-copy hold while adding dependency-view holds. Alias receiving issued another creation effect and a three-incidence hold effect for each alias.

Those effects carry real creation, incarnation, incidence and uncertainty protections. The defect was per-item physical execution despite existing set-capable `ensure_rows` and `hold_many` owners. Increasing copy or alias count multiplied native effect coordination and crossings even though the import’s usable result still depended on final independent whole-state checks. A bounded window preserves those protections with fewer separately issued effect lifecycles.

The coordinator-reported cold `f8f754` timeout prompted this inspection. It is not evidence that these crossings caused the timeout, dominated its duration or explain a performance ratio. Source retirement/absence, receiving progress, local cleanup and native uncertainty retain their distinct recorded outcomes in [coordinator §6](../../plans/catalog-execution-plan_2026-10-12.md#6-sole-disposition-and-execution-checkpoint). No production retirement correction was examined or authorized through this finding.

### Independently accepted replacement

The reinspected candidate is `crates/lctx-compiler-store/src/compiler.rs`, Git blob `b91cd10a6f245c058014f0043a55984943257718`, SHA-256 `1fc9783388b478d9a33d9f2ec73de702826d2a85a5052be3e590290c51d66bdc`.

`register_transport_copies` prepares count-, byte- and resource-bounded windows through the existing native effect owners. Physical identity still derives from the destination attempt, expected transport envelope and copy label. Distinct equal-descriptor copies remain distinct. The original descriptor-encoding and retained-specification reservations cover their actual pending and registered lifetimes. Local `Registering` ownership and known-copy publication follow acknowledged creation and exact attempt holds; registration is not completion authority.

Dependency-view holds are collected across copies after all view declarations exist. The already acknowledged attempt-to-copy hold is no longer repeated. Membership and alias target endpoint preparation still precedes alias attachment. Alias windows retain each copy’s owner-to-alias, owner-to-target and alias-to-source incidences, including when different copy inventories share one global alias row. Neither logical-descriptor equality nor alias sharing collapses those physical obligations.

Count and conservative byte limits flush before adding another item that would exceed a window. Resource preflush and direct reservation-refusal handling release acknowledged pending work before admitting the next item. A previously supported large singleton still reaches the unchanged native wire preflight rather than being rejected solely by a batching target. The code retries local admission only; an uncertain issued effect is never replayed through this fallback.

Reinspection identified and corrected a further lifetime edge within this remedy: pending alias reservations can survive a copy boundary and compete with opening the next copy’s prepared cursor. The final candidate handles only a direct cursor-construction `Resource` refusal with nonempty pending aliases: it flushes the prior window, requires both native effects to acknowledge, releases its arenas and retries cursor construction once. Empty-window resource refusal and codec, IO, opaque or context errors propagate. The fallback does not retry an issued native effect.

The import error/poison path, final state verification, independently installed semantic identity, selector eligibility comparison and final ownership transition to `Completed` are unchanged. Partial or uncertain windows therefore cannot become an admitted imported product through local registration alone. Recovery remains with the original attempt/effect and enclosing cleanup owners; no new retry protocol or durable preparation mechanism is introduced.

### Judgment, limits and disposition

**Bounded source decision: Accept scoped, Implemented / Interface-checked.** The replacement corrects F02’s demonstrated amplification while preserving exact correspondence, required holds, endpoint ordering, resource ownership, failure semantics and independent final audits. A1–A3 and the inspected correctness/fidelity gate judgments remain unchanged; A4 is satisfied for this corrected receiving route. No rule impact, platform substitution or ADR is required.

The retained work has a concrete purpose: copy and alias rows still acquire their exact destination ownership, and independent destination verification still challenges incoming content. Bounded native windows remove avoidable coordination rather than weakening that assurance. No latency, speed or capacity claim follows from source inspection.

Reviewer builds, tests, probes and service operations: **not_run**. Focused native controls and the fresh cold/import composition remain the coordinator’s execution evidence, with their actual outcomes and enclosing cleanup assessed separately. Source acceptance does not close CE01 or reinterpret an earlier timeout/Unknown as success.

**Current disposition:** F02’s source correction is accepted here; current execution disposition, required runtime closure and any subsequent failure investigation belong solely to [coordinator §6](../../plans/catalog-execution-plan_2026-10-12.md#6-sole-disposition-and-execution-checkpoint). A later material change to these seams requires assessment of the affected evidence.

## 10. C1 compact predecessor composition — local integration correction

**Source assessment, 2026-10-14: Accept at Implemented / Interface-checked strength. No new architectural finding or rule impact.** The coordinator reports that actual Catalog `39eae3` failed after 689s: synthesis passed, then retrieval preparation refused with `C1 named lower native frame absent`, before enclosing artifact admission/publication and linked-query acceptance. This failed runtime outcome remains distinct from the following source correction.

The inspected candidate comprises:

| Source | Git blob |
|---|---|
| `crates/lctx-model/src/domain/retrieval/build.rs` | `643303575866ffd87cd1a1d85953c5ec8ae45bf4` |
| `crates/lctx-model/src/domain/catalog/evidence/frame_controls.rs` | `f6c2121eb37561e3f3923728c218f2ddeeb42c5a` |
| `crates/cpg-core/src/retrieval_preparation.rs` | `817b1b2fb63b64d901d01d2cfa45c9c270d984af` |

Retrieval had passed an empty compact predecessor map into strict C1 verification even though admitted Local/SourceCalls compact owners were available. Catalog and synthesis already composed these admitted domains. The new model-owned `Data::verify_evidence_frames` reconstructs a fresh charged map through `CoverageAdmission::uniform_analysis`, using each actual provider run’s exact input/context, selected definitions and protected owner. It invokes unchanged strict C1 invocation/source/input equality and compact/ordinary overlap checks, then installs the map only after successful verification. Ordinary requested frames retain their existing route. No physical invocation rows are fabricated and no guard is relaxed.

This is a local integration omission corrected through an existing adequate semantic owner and validation contract; it does not warrant `F03`. The inspected pure control reproduces the old missing-frame refusal, accepts exact protected domains, preserves missing-proof/wrong-context refusal and checks unchanged evidence payloads. Reviewer execution: **not_run**. Fresh control results and actual current Catalog acceptance remain pending with [coordinator §6](../../plans/catalog-execution-plan_2026-10-12.md#6-sole-disposition-and-execution-checkpoint); this source assessment does not reclassify `39eae3` or establish CE01 completion.

## 11. Protected source preparation during boundary construction — design amendment

<a id="F03"></a>
### catalog-integrated#F03 — Boundary construction reacquires its already protected source observations

**Follow-up, 2026-10-14: violated FP-07/A4 and DP-10; related to original `catalog#F03` and CP02/CP04. The bounded source-backed construction recommendation is accepted at design/source strength. Its implementation and runtime closure are not established by this amendment.**

This finding amends the principal's initial A4 assessment for repeated retained-product and final frozen boundary construction. The original inspection accepted the native windows but missed repeated source acquisition across those boundaries. A new boundary lifetime requires independently established receiving authority; that requirement does not make every unchanged protected source observation a fresh acquisition. The affected route requires revision until its correction is independently assessed. Other scoped judgments and the original dated cutoff remain recorded.

### Examined source and structural consequence

| Source | Git blob | SHA-256 |
|---|---|---|
| `crates/lctx-native/src/completed_state/earning.rs` | `944b3c62417fa74a5b5d577cdb13763afe37eaa0` | `7f6958a0b5bda0a3a5b4d86038f8e66a190614f1e45e34aada9dc7668d6dd762` |
| `crates/lctx-native/src/installed_view/audit.rs` | `531e7a7ab871813d77a3e4825ce66b9888f4e6fe` | `dad227dfbd5b2897788dedb4fb17540bf693f53d4bb0f5be85415f3c2c972160` |

`complete_boundary_content` creates `ClosureInspectionScope::new` for each boundary even when the caller supplies its existing operation-owned `InstallationAudit`. That scope starts without the audit's successful contributor, view and required-hold observations. `observe_fresh_requests` therefore rereads and decodes the selected contributor/view records, then `transfer_expected_many` compares and protects them into the same audit pin. Required incidence pairs are read again. Repeated product boundaries and final freeze over overlapping inputs multiply these source metadata reads, decoding and acquisition effects by the number of boundaries.

The actual Cpg artifact route preserves the operation owner: `artifact::admit` freezes the Workspace, and `Workspace::admission_engine` borrows its existing native store and resource budget. The compiler store supplies that same audit to scoped retention and physical completion when the pools correspond. This establishes a concrete live-source consumer rather than a hypothetical process-global cache opportunity. A foreign-pool finalization still requires its own admitted preparation.

The amplification is narrower than repeated whole-content auditing. Contributor membership, backing, logical runs and family preparation already reuse `InstallationAudit` caches where their exact dependencies agree. Each changed selector still needs its domain traversal, exact mapping correspondence and composed alias/absence assurance. Flat v4 still requires the demanded complete union fold. Those operations are not removed by this recommendation.

The coordinator's later Catalog interruption prompted the investigation. Its phase durations do not establish that this source repetition caused the timeout, dominated admission or explains a speed ratio. No query/publication pass, timeout resolution or runtime capacity claim follows from this source finding.

### Accepted bounded recommendation and preserved trust exposure

Provide an explicit source-backed construction route that borrows the actual protected operation preparation under its native session/client, pin, installation generation/validity epoch, resource pool and exact selected mapping correspondence. Existing operation inspection already provides an appropriate local mechanism: missing observations are earned through owned preparation workers, with exact record/incarnation comparison, terminal reader completion, current context and live-pin checks before successful publication. Reuse must derive from that real owner; serialized descriptors, logical identity alone, unrelated pins or map presence without completed preparation cannot select this route. No generic proof registry or new cross-operation cache is required.

Keep the source lease/drain owner through construction. Sharing an `Arc<ReaderPin>` alone does not prevent explicit audit closure from releasing the pin. Source closure or uncertainty must refuse further borrowing; cancelled or failed preparation must not publish reusable success.

The new receiver must still earn its own authority. Preserve the existing new-parent acquisition that compares every selected contributor's full expected bytes and positive active incarnation before installing parent-to-copy holds. Preserve fresh complete child inventory inspection, exact selector/copy/binding correspondence, required incidences, EOF reconciliation and the final native parent revision/context CAS. Preserve the independently acquired retained-parent pin and its acknowledged transfer. No prefix, cached source success or partial child installation grants completed boundary authority.

Data-only, unscoped, foreign-owner/resource and independent cold/import acquisition remain fresh. A completed receiver may outlive its source only after its own boundary and pin are successfully earned. This recommendation changes source preparation during construction, not independent receiving validation, semantic admission, publication or cold trust.

### Rule impacts, closure and bounded decision

**ADR assessment:** this bounded route does not require superseding [ADR-0165](../../adr/0165-prepared-completion-and-owned-lifecycle.md). Its selected option validates immutable contribution content once under its real owner; its decision permits protected observation reuse under exact incarnation/validity owners while requiring receiving effects to compare their premises atomically. [ADR-0152](../../adr/0152-native-completion-and-freeze.md) likewise retains complete selected validation and physical finality, rather than prescribing repeated source acquisition into the same pin. The recommendation preserves both decisions' independent retained-boundary and cold-admission obligations. A broader route that skipped actual receiving validation or carried operation-local authority into a foreign lifetime would change that assessment.

**Owner wording impact:** narrow the completion plan's current statements that a fresh physical-boundary inspection starts empty and retained/frozen inspection bypasses cached-capability discovery. Distinguish explicitly borrowed, still-protected source construction from independently fresh receiving inventory and finality. The root owns this clarification; this review changes neither the plan nor an accepted ADR.

Closure requires independent inspection of the concrete replacement, including repeated overlapping boundaries reusing exact source observations, first-miss earning, source closure/uncertainty and foreign pin/context/pool refusal, plus unchanged receiving corruption, missing/extra incidence, partial-installation, cancellation/drainage and finality refusal. Independent cold acquisition must retain its separate route and controls. Structural evidence can establish removal of the demonstrated repetition without a numerical benchmark; focused runtime results and actual generated Catalog acceptance remain separate coordinator obligations.

**Bounded decision: revise the affected current source route; accept the explicit protected-source construction recommendation.** No implementation correction is accepted in advance. Reviewer edits are limited to this amendment; builds, tests, probes, formatting and service operations are **not_run**. [Coordinator §6](../../plans/catalog-execution-plan_2026-10-12.md#6-sole-disposition-and-execution-checkpoint) alone owns current F03 disposition, implementation evidence and enclosing CE01 acceptance.

### Partial implementation assessment — source-backed admitted hold

**2026-10-14: Accept scoped, Implemented / Interface-checked.** The root's stable `hold_completed_state_from_source_with_model` route preserves the already earned physical inventory when private `ArtifactAdmission` still owns its actual `NativeCompletedState`. This is a bounded F03 consumer correction. It does not establish acceptance of the concurrent source-construction or inventory-window replacements, F03 runtime closure or enclosing Catalog acceptance.

The inspected snapshot comprises `completed_state.rs` blob `11d8711525f6c42bef66dd704da4ea8adcb38fd4` (SHA-256 `6fa8b38721d55c88004dc255a9f8fb24f8644cfa2532ea0f700e7b8af1f2f335`), `completed_state/earning.rs` blob `e38a0ebaf20a1c4ed81618a82273e0e4e442c1ca` (SHA-256 `d32e456fa252fd100d42ff7525efc82dd8c5292fd105ca3b3cd095cee27396b2`) and compiler `compiler.rs` blob `55b48a3937ac55bc88096c87b5230d3589737cda` (SHA-256 `469d8da5290af3b6785009aa0bcfbf4eed7b8f5425125f01270c10eb3a4df74f`). These identify concurrent source snapshots; this verdict covers only the new admitted-hold helper/private inner factory, actual client/pool provenance fields at all three physical-state constructors and the `admit_completed_state` caller switch. Concurrent inventory helpers and source-construction changes require their separate assessment.

The source route refuses foreign client Arcs or resource pools before reader/effect creation. It independently reads and validates the canonical completed parent, compares the complete `FrozenCompletion` and original source incarnation, then executes the unchanged frozen owner/generation/validity/full-byte/incarnation hold CAS. It clones the source's inventory Arc only after that successful acquisition. Inventory reservations stay with the shared inventory. The actual semantic-admission pointer, full persisted envelope, model/evidence validation, required admission-to-parent incidence and fixed admission hold CAS remain fresh. The final parent read/incarnation comparison and full-byte frozen hold CAS, reader closure and error composition are unchanged. Identity-only public factories still call the private inner factory without a source and inspect the complete inventory freshly.

The reuse premise has an actual protection chain. Completed-parent incidence rules refuse changed child inventory or parent-owned holds; retirement cannot remove outgoing protection while incoming holds remain. Terminal attempt cleanup or fencing defeats the current frozen-owner predicate, and validity maintenance requires live attempts/pins to drain. An Arc or `HeldRecord` is not itself a new live pin: the fresh frozen-owner checks and enclosing operation drainage remain necessary. No new retry or cancellation owner was introduced. First fresh inventory/finality during physical parent construction remains required; this correction avoids repeating it immediately during admission over the same protected physical owner.

Reviewer execution is **not_run**. Affected compilation and existing pure controls remain coordinator-owned. A real opaque-source positive/foreign-owner refusal and closed/fenced or changed-incarnation refusal control remains explicitly pending; pure envelope controls alone cannot establish native lifetime/CAS behavior. No mandatory broad suite or numerical benchmark is added by this source acceptance. Current F03 disposition and actual functional acceptance remain solely with the coordinator.

### Integrated source correction and inventory preparation assessment

**2026-10-14: Accept scoped, Implemented / Interface-checked. F03's bounded source correction is accepted; runtime closure remains separate.** Independent reinspection now also covers the executor's explicit protected-source construction route and the adjacent inventory preparation replacement. This supersedes the preceding implementation exclusions for the source listed here, while retaining the original finding, initial missed assessment and intermediate snapshot.

| Final inspected source | Git blob | SHA-256 |
|---|---|---|
| `crates/lctx-native/src/installed_view/audit.rs` | `6afcdc517f38d73c8c4763a64c87a1bd6fa918d3` | `fdf74f380b5f15d996012ea638f6c2a43e36a369e02944ce8c01291c4b4cd632` |
| `crates/lctx-native/src/installed_view.rs` | `2e687ea598057630d6d56abc67e86172d217a3bf` | `5b948bcfe50fd8f975ab1f987054cd6eb3f06f82cf34463a2bf2b20420921932` |
| `crates/lctx-native/src/completed_state/earning.rs` | `e38a0ebaf20a1c4ed81618a82273e0e4e442c1ca` | `d32e456fa252fd100d42ff7525efc82dd8c5292fd105ca3b3cd095cee27396b2` |
| `crates/lctx-native/src/completed_state.rs` | `ce87b894038686d37a2c585d89b353335599305d` | `1b1737d5b693a143ebba2315805174276aa067dfed993a73f80807ce9c763b00` |
| `crates/lctx-compiler-store/src/compiler.rs` | `55b48a3937ac55bc88096c87b5230d3589737cda` | `469d8da5290af3b6785009aa0bcfbf4eed7b8f5425125f01270c10eb3a4df74f` |

`ProtectedInspectionSource` borrows the actual audit rather than reconstructing authority from selection metadata. Capture checks the real client, shared resource pool, current installation context, live pin and source accepting/closed state; capture-reader uncertainty retains the audit's Unknown obligation. Source scopes retain exact pin/context correspondence and source-liveness checks while borrowing successful operation observations and required incidences. Missing observations still use the existing independently owned preparation workers and their terminal publication rules. The two explicit source construction factories are selected by compiler retention/finalization only under the corresponding operation pool. Existing scoped and unscoped fresh factories remain fresh; foreign-pool finalization does not borrow another pool's preparation.

Every selector still performs its exact traversal, mapping, membership and composed alias assurance. New-parent full-byte/incarnation contributor acquisition, complete receiving inventory, final native parent revision/context CAS and independent retained-parent pin remain in the inspected route. Source closure can refuse construction rather than extend operation authority into a detached lifetime. The already assessed admitted-hold helper separately borrows only the completed physical inventory under its original incarnation and fresh frozen/semantic checks. Together these corrections preserve preparation through the actual live operation without weakening new receiving authority. A1–A3 and the inspected fidelity gates remain satisfied; A4 is satisfied for the corrected F03 route. No speed, capacity or timeout-resolution claim follows.

The adjacent inventory replacement is also source-correct within its bounded scope. Initial copy/selector streams now project fixed-size identity pointers rather than transfer and decode full child descriptors before their guarded readback. `InventoryPointer` requires the exact two-field envelope, declared table, canonical lowercase 64-character hexadecimal identity/key, strict key order and full guarded child correspondence. Each child is decoded once from its guarded window. Full source contributor descriptor/incarnation and semantic view equality remain checked; hashes still consume children in the original canonical order. Exact selected-copy equality, missing/extra refusal, complete inventory hashes, EOF and transport drainage remain prerequisites to returned inventory authority. Pointer discovery grants no completion authority.

Parent-hold inspection still prepares the exact expected canonical inventory. It now compares each streamed canonical hold frame directly with that expected cursor, preserving strict record-ID order, envelope/owner equality and EOF. It removes the actual-row sorter and subsequent actual-run reread. Extra, missing, equal-count substitution, duplicate, reversed and foreign-owner rows still refuse. Pointer windows, guarded rows, retained inventory and canonical comparison workspace remain charged to their actual owners. Partial preparation on failure is not published as completed inventory.

Two protected-source controls, the pointer correlation/order control and expanded parent-hold refusal controls were inspected in source. They distinguish the fresh route from borrowed operation holds, exercise first-miss worker routing and reject foreign session/pool/pin/context, closed/uncertain sources and malformed or incomplete inventories. Their pure premises do not establish actual native transport, maintenance or receiving-CAS behavior. Reviewer builds, tests, probes and services remain **not_run**; coordinator compilation/pure results, the real native source/refusal probe and actual generated Catalog admission/publication/query acceptance retain their separate outcomes and limits. The operator's bounded closeout does not turn these pending obligations into passes.

**Final bounded source decision: Accept the inspected F03 correction and inventory preparation replacement.** No additional architectural finding or ADR supersession is required. The previously identified completion-plan wording clarification remains root-owned. Current disposition, runtime closure and the distinct Catalog-generation review remain with the coordinator; this amendment makes no whole-product or CE01 completion claim.
