# Persisted execution corrections plan — independent target review

**DESIGN / TARGET · 2026-10-07 · Proposed architecture accepted within the scope below.**

The [supporting correction plan](../../plans/persisted-execution-corrections-plan_2026-10-07.md) and revised [persisted execution coordinator](../../plans/persisted-graph-execution-plan_2026-10-07.md) specify a coherent corrective architecture. They integrate correction-causes F01–F06, both confirmed rule impacts, the secondary library recommendations and the consequential uncertainties. Completion, captured attribution, native selection and restore transport now have complete responsibility boundaries rather than six isolated patches.

**Decision: Accept scoped, as a Proposed implementation target.** This accepts the specified contracts and credible execution routes for the reviewed scenarios. It does not accept the current implementation, close either source review's findings, authorize resumed production work, certify the enclosing persisted pivot or establish performance. The three named native-plan, failed-request and cold-admission investigations remain required implementation acceptance; they are not missing target decisions disguised as a research backlog.

## 1. Scope, standard and baseline

The subject is PC0–PC6 and their integration with PG0–PG9: failed operation completion; facts supplier binding and reporting; completed-state identity/transport; table-specific native scope layout; shared prepared demand and bounded candidate ordering; DataFusion fetch composition; restore unit batching; adjacent compiler, publisher, CLI and serving consumers. Both profiles, affected frontiers, exact originals and selected evidence remain supported. General behavioral semantics, evaluator algorithms and the complete retrieval implementation are not re-reviewed.

Workload premises are many small dump statements, new covering providers/families, high-degree scope matches, overlapping contributions, arbitrary supported residual predicates and cancellation on the managed local native host. Required complete domains remain complete. No latency, throughput or capacity claim is inferred from the route.

The applied standard is [core 3.3](../design_principles/core/design-principles.md), [template 3.3](../design_principles/core/design-review-template.md), [heuristics 1.0](../design_principles/core/efficient-architecture-heuristics.md), [code-intelligence 1.5](../design_principles/profiles/code-intelligence/principles.md) and its [review additions](../design_principles/profiles/code-intelligence/review.md), selected by [standard.toml](../design_principles/standard.toml). The core skill's target-purpose rule governs architectural judgment; the binding's legacy change/conformance wording does not affect this design/target review.

Read-only review of source and proposed documents, 2026-10-07: source inspected at `43cd6501`, refreshed against HEAD `593aebf335159d0d1644758a3bbae03e6aa05f8c`. The four dirty source/owner hashes in [correction-causes review §10](design_review_persisted-execution-correction-causes_2026-10-07.md#10-source-anchors-and-baseline-reproducibility) were checked and are unchanged. Final substantive plan inputs examined have SHA-256:

| Document | SHA-256 |
|---|---|
| Supporting correction plan | `54b62a3c5676fd67d8db0ec427a4ed3e47247cdd44fa0ca13b89eb33a3a8e0bc` |
| Persisted execution coordinator | `6a25affb738e43a4fafbc35264e81c49e24d2d2831cb598fcfadcfd3fbe38a77` |

These identify review inputs, not correctness evidence. Subsequent links/status publication do not strengthen the judgment; changes to contracts require affected reassessment.

## 2. Responsibilities, authority and fidelity

| Owner | Authoritative responsibility | Consumers and dependency boundary |
|---|---|---|
| `lctx-model` | Completion composition; supported producer contract concepts; exact contribution/view identities and invariants | Workflows compose outcomes without moving effects into the model. Existing typed graph/fidelity authorities remain. |
| `cpg-extract` provider modules | Declared producer roles, analyzer/adapter semantic revision and configuration binding | One composition serves extraction scheduling and cold admission; executable construction is separate. |
| `cpg-core` | Captured binding admission, availability/reporting and compiler orchestration | Consumes declared obligations plus exact recorded premises; does not reconstruct a prototype supplier list. |
| `lctx-surrealdb` | Table-specific mechanical lowering, prepared-result/terminal contract, compact ordering and membership mapping | Compiler, provider, reader, projection and serving share mechanics; domain policy remains with callers. |
| Publisher/CLI/native attempt owners | Effects, drainage, private staging, seal/backup/restore outcomes | Feed explicit finalization outcomes into shared composition. Publication and selection remain separate. |
| Serving/search owners | Context/path eligibility, ranking and evidence semantics under one pin | Eligibility reaches channels before limits; full-match key/origin arrays are retired. |
| Coordinator §8 | Sole current scheduled finding disposition | Supporting design/packages and source reviews link here; no second task ledger. |

The existing direction `cpg-core → lctx-surrealdb → lctx-model` remains useful. Provider declarations own binding knowledge while the model owns its meaning; ordinary functions/data suffice. No reverse core dependency, universal cleanup executor or generic retry service is required.

| Fact/relation family | Provider/revision and fidelity | Coverage/unknowns | Identity and consumers |
|---|---|---|---|
| Extraction and attribution facts | Independent Ruff, Pyrefly/embedded Ruff, ty and other suppliers retain declared analyzer/adapter revisions; existing fidelity tags remain | Independently expected family/profile/scope obligations; missing, extra, ambiguous, partial and NotRequested remain distinct | Captured role/provider/configuration/source binding enters ContributionSpec and exact views; admission and manifest reporting share the result |
| Completed membership/backing state | Compiler-derived, exact under declared model/profile | Pending state cannot establish results or absence; exact empty input remains a legitimate completed state | Full relation-qualified nominal keys, contribution identity, dependencies and physical mapping remain distinct |
| Selected canonical evidence/originals | Existing attributed facts and programmatic derivations | No new inference or expanded behavioral claim | One complete pinned realization; original bytes, signature sums and incoming S0 naming evidence retain their owners |

This is an adequate operation model for the reviewed scope: captured compatibility differs from current executable identity; completion differs from local task terminality; membership differs from payload presence; eligibility differs from channel output limits; execution units differ from HTTP requests. Consumers are explicitly required to use those distinctions.

## 3. Complete contracts and counterexamples

PC1 correctly separates the primary typed cause, all relevant finalization failures, local terminality, remote effect certainty and private-resource disposition. Cleanup-only failure cannot become success. Primary plus cleanup failure cannot replace the primary through `?` order. A sealed unselected database or published backup with a later cleanup/durability failure is recorded as an already committed effect, not implied rollback. The plan includes initialization before Workspace construction, interrupted/retried drains and sanitized serving projections. This addresses the siblings inspected in [CLI compile](../../../crates/lctx/src/compile.rs), [publisher](../../../crates/lctx-publisher/src/lib.rs), [facts inspection](../../../crates/cpg-core/src/facts.rs) and [restore](../../../crates/lctx-publisher/src/backup.rs).

PC2 binds an independently selected supported producer contract to recorded supplier/configuration/source metadata. Coverage rows cannot nominate their own expected supplier. The legitimate parser-only rebuild survives; an analyzer semantic change does not gain compatibility by matching a tool name. The Pyrefly stage's Ruff supplier and NotRequested reporting ownership prevent a one-stage/one-provider shortcut. The shared admitted result removes manifest's independent fallback classification while keeping all nine exact premises for reuse.

The migration is explicit: spec digest v2, completed-state format2/content v2, artifact format3, header validation before reconstruction, propagated native/artifact/backup/inspection consumers and intentional old-format refusal. Producer-contract identity joins schedule/admission compatibility because the current graph semantic-contract digest does not automatically encode descriptor metadata. Contribution-dependent views already change through the new spec identity; gratuitous changes to unrelated hash algorithms or codebooks are avoided.

PC3/PC4 preserve result statement positions and every terminal, rather than simply prepending LET to readers that consume statement0 or0/1. Missing/null, context, mandatory Eq/IN, OR residuals, exact ownership and ordering are explicit. A compiler-only reference cannot expand unrelated backing tables; retained graph scope_context and separate search projections keep their actual consumers. DDL, construction, expected imported values, executable definitions and realization identity change together.

The demanding overlap case is covered: a physical row matching several values and several contributors is compactly ordered/deduplicated, then intersected with verified exact owners before hydration; no pre-intersection LIMIT may erase legitimate rows. General predicates may require complete exact-input examination but do not require a resident node array. Canonical physical-ID order and typed nominal-key order use separate adapters; frontier aliases remain bounded and exact. Cancelled partial membership windows and spill scratch remain owned through drainage.

PC5 retains grammar-owned whole transactions and import-mode admission while aggregating contiguous units. Its8MiB target,4096 payload-statement cap, transaction control counts, prefix/separator bytes and64MiB complete-request ceiling describe enforceable bounds. An oversized indivisible unit travels alone only within the hard ceiling. Accepted RC02 deliberately permits later effects inside the failed request; no later request is sent, staging remains private, and cleanup uncertainty is retained. The SDK's first returned error is accurately distinguished from an aggregate of statement errors. Successful import still requires independent cold/state/body reconciliation.

## 4. Composition and physical execution

| Operation/question | Universe, selector and method | Exactness, budgets and failure | Output/evidence linkage |
|---|---|---|---|
| Can captured facts support availability? | Independently declared contract + exact nine recorded views; no executable provider construction | Exact supported semantic revision; incompatible/skewed bindings refuse; pure declaration/admission controls remain local | One admitted availability/reporting result, bound to contribution identity |
| Which typed records satisfy demand? | Single-equality scope streams, or exact contributor membership streams; compact external order, dedup, exact intersection and projected hydration | Full nominal keys; one active branch, charged buffers/run metadata, private spill, residuals before semantic fetch; provisional until terminal success | Exact selected view or published pin; no universe widening |
| Which members/capabilities are eligible? | Shared streamed demand and owned context/path/origin predicate; predicate before channel caps | No complete key/origin Vec; bounded/indexed references; unchanged ranking/eligibility meaning | Selected evidence stays within the pinned realization |
| Does native state match canonical state? | Existing full physical-row external ordering adapter plus independent expected reconstruction | Physical-ID order and exact bytes/types retained; imported field values remain untrusted | Complete state/realization reconciliation, separate from supplier replay |
| Can a dump be restored? | Parser execution units grouped into ordered bounded HTTP requests | Transactions remain whole; private failed staging; complete response checked before next request | Fresh new-format cold admission, exact originals/evidence and same artifact identity under a new physical name |

The correction removes avoidable resident match state and scalar request crossings; it does not remove semantically required complete scans or promise immediate first-row delivery. Sorting all necessary compact candidates before ordered output is a stated tradeoff. PC1 is a genuine prerequisite for PC4's blocking/spill ownership and PC5's failure composition. PC2/PC3 can develop independently; whole restore consumes both. The coordinator correctly replaces its former target array route while preserving §9.1's historical observations.

## 5. Realistic change and failure scenarios

| Scenario/kind | Expected propagation and local reasoning | Review result |
|---|---|---|
| Add a covering supplier/family; domain extension | One owned declaration plus genuinely new provider behavior; scheduling/admission/reporting follow it | Credible; no central prototype/configuration exception list remains in target |
| Unrelated executable rebuild; implementation binding | Preserve supported captured semantics, source/configuration and provenance | Explicit compatibility boundary; changed analyzer semantics require a new supported declaration |
| Add compiler-only nominal reference; domain/layout extension | Relevant table inventory and derived scope fields change; unrelated tables do not | Complete inventories derive from registries/lowerings, not fixture samples |
| >32 scopes, >128 memberships, high degree and overlap; growth/skew | Complete branch iteration, external runs and bounded membership/hydration | Credible bounded resident state; no demand truncation or union seen-set substitution |
| Substitute native ordered merge/DataFusion sort; mechanism change | Preserve exact keys/order, membership, budgets and lifetime | Existing adapter seam is sufficient; specializations need qualified semantics and a concrete benefit |
| Failure after seal/backup, or cancellation during merge/drain; lifecycle | Preserve committed effects and all causes; do not release scratch with live work | Explicit PC1 contract and focused combinations |
| Many small statements and early/late import failure; transport growth/failure | Batch whole units; effects stay private; no subsequent request | Explicit accepted behavior and independent disposable-import acceptance |

The supported source-to-evidence journey is preserved through captured admission, direct sealing, transport and pinned selected reads. Both profiles/frontiers and actual native/CLI/PyO3/MCP/evaluator consumers appear in PC6. Behavioral analysis algorithms and protected evaluation population decisions remain unchanged; this does not certify their entire implementation.

## 6. Correctness and fidelity gates

Verdicts below judge the **specified Proposed contracts**, not executed implementation. Runtime closure remains pending at coordinator §8/§9.

| Gate | Verdict for target | Independent reason/limit |
|---|---|---|
| G1 Authority | Pass | Shared completion, provider binding, lowering and reporting owners replace competing interpretations; one disposition owner |
| G2 Semantic fidelity | Pass | Typed causes, supplier roles, uncertainty, nominal identities, null/missing and signature/S0 distinctions are preserved |
| G3 Validity | Pass | Supported-contract admission, exact membership and independent imported-state checks remain enforceable; changed realization must be qualified |
| G4 Hidden behavior | Pass | Cold declarations perform no acquisition/provider construction; spill/import/cleanup effects and operator exclusions are explicit |
| G5 Consistency/recovery | Pass | Private failure state, complete terminality/effect outcomes, committed-effect reporting and whole-transaction boundaries are specified |
| G6 Transformation/reuse | Pass | Exact nine premises, versioned bindings, ordering/alias contracts and post-filter fetch constrain substitutions |
| G7 Truthful claims | Pass | Proposed/Interface-checked and historical Tested claims remain separate; no speed inference or falsely completed restore |
| G8 Library leverage | Pass | Parser, native indexes/streams/point reads and DataFusion fetch are composed; bounded reasons support retained compact ordering, Arrow buffers and shared joins |
| CI-G1 Fidelity | Pass for touched contracts | Attribution and incomplete/NotRequested meanings remain explicit; no new program-analysis fidelity claim |
| CI-G2 Evidence closure | Pass for specified journey | Exact originals, complete transport and one serving pin remain requirements; actual new-format journey not executed |
| CI-G3 Evaluation integrity | Pass for preservation boundary | Protected data remain isolated; evaluator meanings do not change; evaluator algorithms/populations not re-assessed |

## 7. Findings, source obligations and applicability

**No new blocking finding remains in the final reviewed target.** A serving-array bypass ambiguity encountered during review was resolved in correction-plan §4.1: the member RETURN array and capability key/origin arrays are explicitly retired; eligibility reaches each channel before its limit; obsolete functions leave the realization inventory. This is clarification of source F03/F04 closure, not a new mutable finding ledger.

| Stable source ID | Integrated obligation | Current disposition owner |
|---|---|---|
| Correction-causes F01 | Complete typed outcome; all initialization/finalization siblings; committed effects and interrupted drains | [Coordinator §8](../../plans/persisted-graph-execution-plan_2026-10-07.md#8-sole-finding-disposition), PC1 |
| Correction-causes F02 | Declarative supported/captured binding, nine-premise admission/reporting and explicit state migration | Coordinator §8, PC2/PC6 |
| Correction-causes F03 | One prepared demand/result/terminal authority across private and published consumers; actual plans | Coordinator §8, PC3/PC4/PC6 |
| Correction-causes F04 | Single-prefix/contributor streams, compact external ordering, exact intersection, aliases and late failure | Coordinator §8, PC4 |
| Correction-causes F05 | Table-specific complete layout, real scalar/search consumers, independent reconciliation and schema identity | Coordinator §8, PC3/PC6 |
| Correction-causes F06 | Bounded whole-unit requests, selected failed-request semantics and complete restored admission | Coordinator §8, PC5/PC6 |

FP-01–FP-06 are satisfied by the proposed scoped operation contracts and composition. FP-07 is satisfied for the proposed reviewed growth/failure routes: table-specific physical work, compact spillable selection and coarse whole-unit import. Applicable DP-01–05/08–10/13–16/18–24 and CI-01/03–05/08/10/11/13 are satisfied within those contracts. No new FP/DP/CI violation is established. Unchanged general recurrence, behavioral precision and heuristic analytics are outside this plan review's assessment.

## 8. Library fit and total complexity

Exact pinned source was read where decisive. SurrealDB3.3 ordered union dedup retains `HashSet<RecordId>` (`exec/operators/scan/union_index.rs`); adopting it unchanged would relocate F04's growing state. Single-equality index branches plus bounded external ordering avoid that premise. The existing [ordered_rows kernel](../../../crates/lctx-surrealdb/src/ordered_rows.rs) already provides finite runs, binary-carry merging and adjacent conflict checks for full physical rows; compact nominal-key adaptation, charging and acknowledged blocking ownership are **Proposed**, not already delivered.

DataFusion55.1 `StreamingTableExec::with_fetch` supplies the library limit wrapper; [NativeExec](../../../crates/lctx-surrealdb/src/compiler_provider.rs) currently does not delegate it. Faithful post-membership/post-residual delegation is appropriate. Outer LIMIT already stops polling, so delegation does not diagnose an earlier complete scan. DataFusion SortExec remains a valid alternative in a caller that owns its runtime; installing a second nested runtime behind generic native reads would add configuration/lifecycle burden without a new consumer need.

The pinned parser/execution units and checked HTTP import remain preferable to bespoke lexical parsing or switching to `/sql`. HTTP SDK3.3 reads the complete response then returns the first statement error; PC5 does not claim it aggregates every error. Arrow59.3 generic FixedSizeBinary coalescing and Tokio1.53/shared-future lifetime behavior retain the source review's bounded reasons for custom compact buffers and durable join ownership. No blanket library replacement is warranted.

## 9. Alternatives and tradeoffs

| Alternative | Judgment and reason |
|---|---|
| Retain current prototype binding, match arrays, global fields and per-unit HTTP | Fails extension locality and execution fit already diagnosed by the source review; isolated success cannot close those causes |
| Proposed ordinary contracts + compact native external ordering + batched import | Best complete route among inspected alternatives; explicit owners and existing mechanisms avoid a new executor/store/runtime |
| Native ordered union | Not selected as common route: overlapping branches retain result-sized dedup state; a future qualified specialization has a concrete trigger |
| DataFusion sort everywhere | Sound candidate with an existing runtime; generic native scanner adoption adds a second ownership/configuration route, so it is deferred rather than declared incapable |
| Broader provenance compatibility or compatibility reader | Cannot infer supported semantics from build/name equality; required metadata instead migrates explicitly and old captures rebuild |
| Immediate fail-stop between every independent import unit | Preserves older internal timing but retains scalar crossings; accepted private failed-request behavior supplies the simpler batching contract |

The selected design trades disk work and delayed ordered delivery for bounded resident candidates, keeps full semantic examination where necessary, and trades immediate inter-unit stop for coarse transport under private staging. These benefits warrant the stated machinery without any quantitative speed claim.

## 10. Verification and consequential uncertainty

**Product verification: not_run.** No `cargo check/build/test`, `just verify-*`, `just qualify`, parser/native probe, real-library compile, operator command or live-model work was executed for this review. Static source/document inspection is **Interface-checked, 2026-10-07**; PC contracts remain **Proposed**. Historical receipts were read as attributed observations only.

The changed implementation must settle three named boundaries: actual single-prefix/contributor/published plans without hidden full-match state; disposable multi-unit failed-import privacy/cleanup; independent new-format cold admission across an unrelated executable rebuild and different database name. PC1 failure combinations, PC3 schema/import tamper and PC4 forced-run/overlap/residual/cancellation controls address concrete counterexamples. These are meaningful targeted controls; stopped compiler suites, legacy parity, assembled qualification, wheel packaging, Qwen/operator work and performance campaigns are not revived.

The plan's explicit requirement to revise a lowering rather than increase limits or truncate results is consequential: unexpected native materialization blocks F03/F04 closure. Likewise parser-only success cannot settle RC02's effect timing, and current-ID fixture acceptance cannot settle F02's compatibility contract. This acceptance supplies no runtime finding closure.

## 11. Authority changes and disposition

No additional rule reversal is proposed by this review. The two source-qualified impacts remain [correction-causes RC01/RC02](design_review_persisted-execution-correction-causes_2026-10-07.md#8-rule-impacts-and-disposition), operator-confirmed2026-10-07: table-specific scope projections and within-request later independent import effects. PC0 records the complementary ADR and owner changes before dependent production work; ADR-0133's placement/direct-sealing choice remains intact.

New captured-binding compatibility, format/schema cutover and complete operation outcomes require their stated decision/owner route; accepting this plan does not make those rules Implemented. Accepted ADR text is not amended casually. Scheduled findings stay open at coordinator §8 until named closure evidence is supplied; the source review preserves its dated assessment and links to that owner. Production execution remains paused.

## 12. Architectural judgment and decision

| Judgment | Verdict for Proposed target | Scope and reason |
|---|---|---|
| A1 Localize change | Satisfied | Producer declarations, completion semantics and native mechanics each have one owner; table inventories contain physical propagation |
| A2 Encode domain meaning explicitly | Satisfied | Supplier compatibility, outcome certainty, exact membership, eligibility and transport units have adequate representations and governing operations |
| A3 Extend through composition | Satisfied | Existing workflows consume those contracts and library primitives; no new generic framework or second semantic classifier is required |
| A4 Fit execution to supported workload | Satisfied | Compact spillable ordering, complete exact inputs, table-specific definitions and whole-unit batching provide credible growth/failure routes with stated tradeoffs |

**Accept scoped as a Proposed implementation target for PC0–PC6 integrated with PG0–PG9.** The enclosing implemented pivot remains unresolved and its source findings remain open. This judgment excludes implementation/runtime qualification, unrelated retrieval/behavioral algorithms, real-library/live-model/operator adoption and quantitative performance; those exclusions do not remove any promised PC contract or scenario.

The next architectural action belongs to the coordinator/operator: review the complete planning result and, if execution is separately resumed, record PC0's decision/owner changes before dependent implementation. Priority follows semantic/recovery consequence (PC1/PC2), while actual dependency order supplies shared layout/lowering before bounded access and complete outcome handling before restore batching. Final acceptance must use the new format/schema and the actual consumers, not promote earlier receipts.
