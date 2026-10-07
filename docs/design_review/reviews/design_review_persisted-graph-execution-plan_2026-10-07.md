# Persisted graph execution plan — independent design review

**Design / target · 2026-10-07 · Decision: Accept the Proposed architecture.**

The proposal supplies a coherent route from captured inputs through compilation in one private persisted graph to an admitted, sealed, unselected native snapshot. Its important change is the composition around immutable completed inputs: native selected access, exact contribution/view identity, carried validation, and direct sealing replace repeated rich scans, prefix reconstruction and self-import. Persistence has an explicit present consumer in compilation and dependency inspection, while the reusable operations also address the inspected serving, codec, original-transfer and response-encoding variants.

The design preserves a useful distinction between authority over meaning and physical execution. Model operations govern semantics; suitable native operations handle resident sets and adjacency; charged Rust kernels and spillable Arrow/DataFusion work remain available where they fit. The bounded native-to-Arrow bridge is a field conversion, not an assumed zero-copy protocol. The plan avoids a second compiler, parallel canonical row store, generic provider framework, resumability journal or speculative incremental scheduler.

No additional blocking architectural finding is established in this review. Acceptance is at **Proposed / static-inspected** strength: it authorizes the selected architectural direction and dependency sequence, and does not establish implementation, closure of the source findings, an operationally successful FastMCP compilation or a speedup. The named implementation investigations must resolve their actual contracts before dependent packages claim readiness.

## 1. Scope, outcome and coverage

| Field | Reviewed boundary |
|---|---|
| Subject | [Persisted graph compilation and shared efficient execution](../../plans/persisted-graph-execution-plan_2026-10-07.md), stable uncommitted proposal on main `ccbcae1f06610d00564d8b74078ae642a5c548e9`. Reviewed file SHA256: `06f05d787ec5d03d8d4f834806d300482eb7655b735b59249bbaf9423e0c1e7d`. |
| Standard | Core/template 3.3; Heuristics for Efficient Architecture 1.0; code-intelligence principles/review additions 1.5; library-context binding. |
| Reviewer | Independent delegated design reviewer; coordinator authors and publishes the proposal/review. |
| Outcome | Persisted graph compilation throughout core, same-database sealing, and singular common mechanisms for analogous inefficient code outside compilation. |
| Supported content | Captured/provider facts, all normalization stages, Catalog and Behavioral profiles, Facts/Normalized/Analysis/Catalog frontiers, selected projections and analytics, execution/summaries, C0/C1/C2/S0/E0/E1, admission, native loading/search, portable artifacts, backups/restores, pinned serving/PyO3/MCP and evaluator contract consumers. |
| Workload premise | One operator and a managed local persistent SurrealDB server; real pinned libraries with growing modules/fact kinds, shared or high-degree dependencies, overlapping evidence windows and scoped multi-call evidence journeys. Bulk memory, server work and stream finality matter independently of returned-result bounds. Concurrent publication must preserve existing reader pins. |
| Settled target choices | Native persisted compilation selected by the operator. Direct same-database sealing and exact internal contribution/view identity are approved; deterministic final content identity remains. Dependency foundations are wanted now; cross-run invalidation execution is deferred. |
| Exclusions | Implementation and production effects; probes, builds, tests, services and pilots; quantitative performance; automatic cross-run scheduling/resumption; another backend; historical runtime compatibility. Behavioral algorithms and provider quality are preserved rather than newly qualified. |
| Dirty-tree boundary | Existing AGENTS/STATUS/DESIGN, coordinator/plan/runbook edits and a test docstring were present and preserved. Adjacent future-boundary pointers may change during review; the reviewed new proposal was stable. Production sources were inspected in the current tree, not inferred from documentation receipts. |
| Method | Read the proposal, source review, current architectural owners, decisive executing sources and adjacent consumers. Static reasoning is sufficient for this proposal decision. No newly executed functional evidence is claimed. |

The interrupted Catalog run has no verdict. Its runtime dominance is unresolved and is unnecessary to establish the avoidable structural work addressed here. The review evaluates the proposal against the standard and functional target; agreement with old store-free rules is not an acceptance criterion.

## 2. Responsibilities, dependencies and semantic ownership

| Owner | Governing responsibility and consumer boundary | Reason for change |
|---|---|---|
| `lctx-model` | Typed records, nominal keys, graph lowering, stage outcomes, dependency/completion/view semantics, invariant applicability, projection and wire meaning. Pure finite operations remain explicit-value contracts. | A new domain concept, invariant, view distinction or wire meaning changes this authority and its mechanical derivations. |
| `cpg-core` | Attempt orchestration, producer-to-predecessor binding, completion ordering, demand resolution and operation adapters. | A workflow composition changes orchestration, without redefining records or native storage rules. |
| `lctx-surrealdb` | Generated mechanical body/schema/key mappings; bounded native reads/writes, stream finality and realization mechanics. | Native representation, index or SDK mechanism changes remain at this boundary. |
| `lctx-publisher` | External visibility, import/export/restore admission, executable realization sealing and unselected handles. | An external trust or publication lifecycle change affects this boundary. |
| Model/analytics/behavioral kernels | Named set, topology, transfer/BDD/SCC/proof and analytic operations over declared universes. | A new algorithm or semantic method has an owner and explicit dependencies. |
| Serving/bridge/evaluator | Pinned packet selection, response delivery and finite independent evaluation meanings. | Another rendering, transport or evaluation observation consumes shared meaning instead of creating another interpreter. |

The selected compile-time direction `cpg-core → lctx-surrealdb → lctx-model` is feasible: the inspected [native manifest](../../../crates/lctx-surrealdb/Cargo.toml) currently depends on the model, not core. The publisher currently consumes `cpg-core::artifact::VerifiedExport` ([publisher](../../../crates/lctx-publisher/src/lib.rs), lines 1–23); PG5 must replace the ordinary publication input with owned native completed state while retaining external import. No proposed core-to-publisher dependency is needed to reuse the lifecycle.

The additional domain is small and consequential: typed row identity, producer contribution, exact completed view, explicit dependency selection and completed state. Pending versus complete, available versus unavailable/not requested, data cycles versus derivation cycles, semantic versus physical identity, and graph versus non-graph backing remain distinct. This is an adequate Proposed model of the new behavior, rather than only new metadata columns.

The fact/fidelity table remains grounded in the retained authorities:

| Family or relation | Provider/revision and fidelity | Coverage and unknowns | Identity and consumers |
|---|---|---|---|
| Captured originals and native observations | Pinned capture and explicitly configured analyzer families; original bytes and provider-attributed extracted/resolved assertions. | Existing per-profile/per-family outcomes and unresolved boundaries survive migration. | Nominal typed keys, original content and run context; normalization/catalog/behavior and evidence consumers. |
| Normalized entities, callables and relations | Model-owned derivation/correspondence; candidate and unavailable distinctions retained. | Exact predecessor views govern applicability, including negative premises. | Typed row and canonical graph mappings; selected catalog, topology and serving packets. |
| Projections and analytics | Declared projection/method/settings; exact topology and heuristic outputs stay separate. | Total NotRequested/partial outcomes remain explicit. | Snapshot/specification and nominal source membership; kernels and qualified synthesis. |
| Execution, summaries and proofs | Retained model-relative transfer/fixpoint operations; conservative or unknown outcomes retain their assumptions. | Legal data recursion does not permit derivation cycles. | Owned execution/summary/proof keys and exact support; behavioral answers and synthesis. |
| Findings, assertions and retrieval values | Programmatic derivation; exact contextual evidence and deliberate search projection. | Missing or incomplete evidence does not become absence or proof. | Same-snapshot support, complete values/specifications and returned references. |
| Completed state and non-graph backing | Compiler-owned completed contributions under model contracts; operational dependency metadata, not program assertions. | Empty completion and actual availability distinct from omission/pending. | Separate deterministic state digest plus aggregate artifact identity; compiler views, inspection and transport. |

The plan's one fixed generated `compiler_record` family is justified by actual required non-graph records, including `QualityStep`. Complete registry reconciliation is a prerequisite, not an assumption that graph registries cover every typed input. `ArtifactChunk` uses the original-byte owner instead of duplicating full bytes. The graph and completed-state families are explicitly excluded from one another's entity/assertion counts.

## 3. Contracts, constraints and testing boundaries

The exact-view contract is the crucial foundation. Producers bind predecessors before work; pending writes do not enter seeds, closure, negative answers or global scans; a short completion transition publishes only drained, fully handled output sets. Frozen views select contribution identities and cannot widen when later overlapping contributions arrive. Full nominal keys and payload equality govern duplicate/conflict handling; digest equality cannot authorize a conflicting value.

Current [SourceSnapshot](../../../crates/lctx-model/src/domain/analysis/sources.rs), lines 8–77 and 117–125, encodes relation, producer, model, implementation, content and rows into internal input identity. The proposal explicitly migrates that meaning and its observation/cache/manifest consumers. It does not disguise a membership identity as the old canonical prefix hash. Logical identities exclude physical database names, clocks, batch divisions and delivery order; restore preserves the aggregate artifact identity while deriving a new physical realization.

Forward internal references may precede endpoint completion. The proposed pending-link and endpoint-first enforced-edge route preserves that legitimate case without turning an internal missing key into external uncertainty. Final admission retains endpoint kind/subtype, actual reference closure and derivation-DAG obligations. Compiler access must continue to use full typed references while membership gates what can be read; pending operational links cannot become an alternate semantic source.

In particular, completing a source row does not permit its declared reference to disappear merely because the physical enforced edge awaits an endpoint. The batched closure operation must consume those full retained references/pending links, or an equivalent mechanical representation, and preserve required-missing-premise errors. Later edge installation cannot enlarge the target universe selected by an earlier frozen view. The contract is specified; choosing its indexed layout is PG1/PG2 work. Likewise, a view's row statistics count distinct complete typed keys over its selected memberships, never the sum of overlapping contributions; implementing those counts must not relocate rich-prefix rebuilding into another index builder.

The complete body migration addresses a real boundary: [codec](../../../crates/lctx-surrealdb/src/codec.rs), lines 50–81, currently omits opaque binary query fields. A compiler body must preserve all declared fields, exact widths, nullable/sum arms, opaque bytes and logical text. SCHEMAFULL/coercions alone are not semantic validation. The plan requires generated mappings and independent stored-body comparison, avoiding manually maintained per-record codecs.

Pure value and algorithm controls remain server-free. Effectful completed-state, membership, stream, transport and sealing controls need an actual authenticated persistent native fixture. Changing the compiler's infrastructure does not justify making every semantic test start a server.

## 4. Composition and execution

| Operation | Question, semantic universe and method | Physical route, reuse and effects | Limits and evidence |
|---|---|---|---|
| Acquisition/facts/completion | What did the pinned provider produce, with what coverage? Full declared outputs and actual outcomes. | Bounded native writes into pending contributions; one completed transition. | Shape/conflict checks and terminal/drain success; partial outputs never imply complete. |
| Normalization/C0/C1/C2 | What correspondence, public callable or evidence classification follows under the owned policy? Required cross-module premises retained; requested output is not the universe. | Indexed selected memberships/keys, shared batched closure and late rich hydration. | Full relation-qualified visited keys, missing-premise behavior and independent expected rows. |
| A0/A1/projection/behavior | What structural, heuristic or model-relative result follows? Projection direction, multiplicity and simplification/settings remain declared. | Resolve method-demand union first; reuse selected topology; model-owned kernels, native finite operations or spillable DataFusion as suitable. | All-disabled A1 preserves total outcomes; required A0 work remains; operational partial outcomes stay explicit. |
| S0 | Which programmatic claims are supported? Complete documentary support and global literal/embedding requests. | First documentary pass supplies compact completed spool; later phase adds actually required later inputs. | Attempt-owned bounded/spillable spool; second build removed without retaining every rich grain. |
| E0/E1/original access | What exact contextual ranges and compatible full/projected values answer the request? Ordered logical ranges remain distinct. | Union physical chunk demand; one verification/fetch per bounded scope; existing exact-input inference/cache batching. | Original hashes and exact byte order; selected policy/specification/winner identity. |
| Admission/seal/import | Are declared domains, references, state and executable definitions valid? Exact applicable premises. | Carry unchanged validation; check changed/new premises; reconcile actual stored realization; direct same-database seal. External import independently admits. | No flag/digest substitutes for semantic admission. All writers drain before viewer capability and unselected handle. |
| Serving/restore/wire | Can a pinned consumer obtain exact evidence and bounded final delivery? Same realization throughout references/cursors. | Request-local typed indexes; coalesced verified range transfer; one charged final encoding per actual wire shape. | Missing/foreign keys remain errors or owned unknowns; changed envelopes and delivery stabilization still checked. |

Native-to-Arrow projects fields into generated builders once and carries counts/statistics to the spillable engine. [CanonicalBatches](../../../crates/lctx-surrealdb/src/batches.rs), lines 22–103, currently reconstructs selected canonical entities/assertions and encodes grouped typed rows; it is an adjacent conversion boundary to migrate when used by the new common mechanism. The plan correctly avoids making that rich reconstruction a mandatory precursor to compiler Arrow work or keeping a whole-store Vec of batches.

This physical route remains credible under corpus growth and skew: selected access does not repeatedly scan unrelated rich families; batched frontier expansion handles shared/high-degree dependencies without per-edge RPC; global algorithms retain their real universes and spill/compact where appropriate. Index maintenance, duplicate checks and final content/state hashing remain necessary work. The benefit is removal of repeated work, not proof that every native query is automatically selective or faster.

## 5. Change and failure scenarios

| Scenario and kind | Owner and expected propagation | Judgment basis / meaningful settling case |
|---|---|---|
| Add a fact family or configuration witness / domain concept | Model declaration, provenance/coverage and genuinely new operation semantics; generated native body, registry/backing route and consumers follow. | No table/task/transaction per semantic kind. PG1 registration must reject an undeclared required record. |
| Add an analytic over existing facts / composition | Analytic owner declares its projection, dependencies and outcome; common access/preparation composes the selected union. | One-selected versus all-disabled tests, with A0 unchanged; no additional cache or independent method classifier. |
| Substitute native selected access for IPC scans / mechanism | Native owner absorbs indexes/SDK mechanics; model identity and closure contracts remain. | Frozen lower view, reconvergence, inverse ownership, parallel occurrences, isolates and foreign/missing keys. |
| Grow unrelated modules or share one dependency widely / instances and skew | Existing operations and policies reused. Physical selected demand and late hydration remain scoped. | Source inspection must show no per-grain whole-source scan or singleton remote loop; a bounded result alone is insufficient. |
| Extend shared vocabulary after lower admission / membership change | Completion/view owner supplies a new exact view; validators whose premises changed recheck. | A missing-to-present lookup invalidates absence-sensitive validity; unrelated frozen-premise checks stay valid. |
| Compile while another snapshot is served / lifecycle | New attempt remains private; published reader pin and supervisor lifetime stay independent. | Earlier references/continuations survive another publication and never resolve under its content/definitions. |
| Fail after provisional stream rows or lose COMMIT acknowledgment / failure | Attempt owner drains and abandons its owned database; no resumability journal or generic retry stack. | Late terminal/transport error rejects dependent outputs; unknown acknowledgment fails the attempt; cleanup failure reports an unselected orphan. |
| Export and restore under another database name / transport | Publisher/import owner validates complete graph plus state/backing and rebuilds definitions/indexes. | Same aggregate content identity; missing/extra/tampered state or backing rejects, including a complete-empty contribution. |
| New library release or analyzer revision / binding | New pinned run/context and explicit semantic contract changes; earlier selected realization remains immutable. | Persisted dependencies are useful inspection/future-invalidation foundations; no automatic cross-run reuse is claimed. |
| Evaluation observes a changed transport / adjacent consumer | Evaluation owner changes actual observation integration while retaining independent tasks/oracles and frozen comparison meaning. | Protected answers never become compiler/retrieval inputs or tuning data. |

The revealing case for compact S0 retention is a member whose first-pass documentary support triggers a global request but whose final emission requires later embedding/literal results. The spool must retain exact support and emitter identity without retaining every rich input. The plan explicitly assigns this check before dependent completion.

## 6. Independent correctness and fidelity gates

Verdicts below assess whether the **proposal specifies a conforming route**, not whether a future implementation passes its controls.

| Gate | Verdict | Evidence and preservation boundary |
|---|---|---|
| G1 Authority | pass | One model authority; canonical body/bytes are mechanical immutable forms; non-graph backing and completed metadata have distinct ownership. |
| G2 Semantic fidelity | pass | Complete fields, typed identity, frontier-dependent Place aliases, explicit availability and family separation are required. |
| G3 Validity | pass | Production validators, exact reference/subtype/derivation closure, external admission and independent stored reconciliation remain. |
| G4 Hidden behavior | pass | Native dependency applies explicitly to every compilation route; pure kernels retain explicit-value testing; acquisition remains pinned. |
| G5 Consistency and recovery | pass | Completed membership gates private reads; frozen views, terminal checks, bounded effect units, acknowledgment semantics, drain and sealing are explicit. |
| G6 Transformation and reuse | pass | Exact premise descriptors govern carried validity and preparation; complete-state identity/transport and actual value/specification meanings migrate deliberately. |
| G7 Truthful capability claims | pass | Plan is labelled Proposed; no speed, successful pilot, new test result or automatic incrementality is claimed. |
| G8 Library leverage | pass | Existing SDK sets/bulk operations, original-range verifier, spillable DataFusion, graph/kernel libraries and std-style indexes are composed. Exact full-key closure remains a justified gap. |
| CI-G1 Fidelity | pass | Provider attribution, coverage/unknowns, model-relative behavior and heuristic qualification are preservation requirements. |
| CI-G2 Evidence closure | pass | Same-snapshot facts/support, full reference closure, pinned reader journey and state-backed dependencies remain mandatory. |
| CI-G3 Evaluation integrity | pass | Production/evaluator roles, sealed protected populations and frozen comparison meanings retained. |

No passing architectural judgment offsets these gates, and these proposal-level passes are not current-tree functional qualification.

## 7. Findings, preservation obligations and applicability

**No new blocking Fnn finding.** The review does not reopen the source findings as unscheduled alternatives or close them on plan acceptance. Their current execution owner is [plan §8](../../plans/persisted-graph-execution-plan_2026-10-07.md#8-sole-finding-disposition).

| Stable source obligation | Independently inspected evidence | Proposal route and why it is complete |
|---|---|---|
| [Catalog-speed F01](design_review_catalog-compilation-speed_2026-10-07.md#f01) | `normalize/mod.rs:81–147,204–222,501–547`; `consumed_rows.rs:884–894`; C0 `catalog_core.rs:154–232`; C1 `catalog_evidence.rs:191–259`; callable scope forward-plus-ownership requests; retrieval chunks per logical range; S0 two documentary builds. | PG1–PG4 selected memberships/keys, batched closure, projected Arrow/statistics, deduplicated physical edge/chunk requests and compact S0 spool. Global semantic premises and distinct logical ranges retained. |
| [Catalog-speed F02](design_review_catalog-compilation-speed_2026-10-07.md#f02) | `workspace.rs:1362–1389` includes prior completed relation; `ordered_stream.rs:310–354` reindexes/sorts; `215–263` gathers singletons. | Contribution-local digest and exact view membership eliminate rich prefix reconstruction; bounded type-window codec replaces scalar packing. Old selected views, exact equality/conflicts and deterministic final content remain. |
| [Catalog-speed F03](design_review_catalog-compilation-speed_2026-10-07.md#f03) | `compilation.rs:610–615`, `artifact.rs:1255–1260`; repeated admission and current publisher export boundary. | PG1/PG4/PG5/PG7 carry exact validity, admit changed premises, seal same realization and independently admit external transport; completed state/backing survive restore. No removal of distinct protections. |
| [Catalog-speed F04](design_review_catalog-compilation-speed_2026-10-07.md#f04) | `analytic.rs:39–111` builds expected inputs and FrameScopes before selected-method execution. | Demand union precedes kernel preparation; total outcomes and required A0 topology retained. |
| [Catalog-speed F05](design_review_catalog-compilation-speed_2026-10-07.md#f05) | `loader.rs:125–197` performs per-occurrence external writes within later bulk role loading. | Bounded full-key endpoint set precedes enforced edges; distinct relationship occurrences remain. |
| Plan D01–D04 | `source_evidence.rs:82–117` repeatedly filters typed families; nested `source_usage` callers; `backup.rs:296–330` scalar original transfer versus existing batch API; `service.rs:124–143,236–250` repeated final encoding; `codec.rs:50–81` singleton encode. | Shared lifetime-bound selected-row indexes, verified physical range unions, charged final encoder and type-slice codec. These mechanisms have actual compiler and non-compiler consumers. |

FP-01–FP-07 are **satisfied at Proposed strength** for the reviewed scenarios. DP-01–09, DP-11–13, DP-15, DP-18–24 and CI-01–06/08–13 are satisfied by the explicit preservation and lifecycle contracts. DP-10/14/16/17 and CI-07 are satisfied by operation-shaped batching, scoped preparation, library delegation and coherent owners. Those judgments apply to the proposed architecture; the executing baseline retains the source review's violations until corrected.

Two practical preservation points constrain implementation rather than create new findings. First, a frozen view must govern **every** access path, including empty/bulk queries, adjacency and dependency inspection, not only typed ID reads. Second, sharing the codec must not make reconciliation trust the producer's stored body: expected bodies still regenerate from actual canonical bytes and compare against actual stored values. Both obligations are already explicit in §§4.3/5.4 and §9.

## 8. Library fit and total complexity

The selected local capability skill is `neo4j-surrealdb`, whose SurrealDB source profile is 3.3.0; the inspected Cargo dependency is exactly `=3.3.0` with grpc/http/rustls and no embedded backend. This review checked repository SDK usage and capability limits; it did not re-run skill probes or certify every server patch accepted by current readiness.

| Capability | Fit and integration judgment |
|---|---|
| Remote typed sets, keys, projection and adjacency | Existing `RecordSelection`/reader and bulk loader supply credible seams. Adding view predicates and generated atomic nominal keys is narrower than a backend-neutral query framework. Index selectivity must be settled for actual uncertain query shapes, as PG2 requires. |
| Terminal-aware streaming | Existing [NativeRows](../../../crates/lctx-surrealdb/src/reader.rs), lines 210–280, enforces row/statement ordering and terminal/outer completion. Compiler reuse must preserve provisional outputs and private discard on late failure. Small batches do not establish server RSS safety. |
| Arrow/DataFusion bridge | Native projected Values require generated field conversion. Spillable joins/sorts remain useful for operations with relational/global structure. Reusing Arrow APIs does not require persisting one IPC source per selected grain. |
| Graph/program-analysis kernels | Full-key nominal visited sets and model transfer/BDD/SCC/proof operations preserve meaning that generic reachability cannot establish. This is a specialized contract gap, not failure to delegate native capabilities. |
| Bulk codec and enforced edges | Existing array inserts can be retained and widened with type-slice conversion/external endpoint batches. Endpoint readiness and distinct role occurrence identity remain genuine dependencies. |
| Original range access | Existing verified 32-range/256KiB API gives a concrete reusable basis. Physical demand union must preserve every ordered logical range and complete original admission. |
| Index timing | Early identity/compiler indexes and checked pre-seal secondary builds preserve required guarantees. A new table per record, table-wide adjacency cache or concurrent-build protocol is unnecessary. |
| Response encoding | A bounded budget-aware writer closes an actual allocation/output-contract gap around existing serialization. Final bytes are reused; distinct MCP envelopes and delivery stabilization remain meaningful work. |

Canonical bytes plus complete queryable bodies add stored width, but they serve compiler field access and independent exact export/reconciliation under one immutable authority. The fixed non-graph family avoids a second copy of graph-eligible records. These benefits justify the representations; retaining another writable full typed canonical store would not.

## 9. Alternatives and tradeoffs

| Alternative | Complete-route judgment |
|---|---|
| Current store-free compiler then publication | Strong pure boundaries, but actual execution repeats rich scans, prefixes, admission and self-import. It lacks the selected persisted compiler/dependency-inspection outcome. |
| Indexed in-process immutable contributions | A simpler viable remedy for many speed-review findings in isolation. It avoids intermediate RPC/index lifecycle, but does not supply the operator-selected persisted graph compilation. It remains a useful comparison for kernel preparation, not a second supported compiler. |
| Persist every typed relation and also maintain canonical graph tables | Easier direct row addressing, but duplicates graph-eligible payload ownership and requires another reconstruction/synchronization boundary. The proposal's canonical-first complete body plus narrow non-graph backing is preferable. |
| Put every computation in SurrealQL | Reduces some movement, but would transfer complex program semantics and specialized algorithms into another interpretation surface. The mixed native/kernels/Arrow placement is simpler and more faithful. |
| Selected proposal | One private persisted authority and direct seal; exact memberships; common operations with actual consumers. Adds native runtime dependence to artifact-only compilation, persisted membership/index work and complete-state transport, all explicitly declared. |
| Embedded compiler or raw Arrow transport | A second lifecycle or unstable/new protocol burden without a current required benefit. No such expansion is needed to implement the selected route. |

The simplest **complete** alternative within the selected persisted outcome is essentially the proposal: existing native families/lifecycle, compact membership metadata, one fixed backing family and finite common helpers. It does not need a scheduler, event history, generic storage abstraction or per-query proof system. Future cross-run invalidation would revisit dependency granularity and absent/deleted membership, not justify implementing that executor now.

## 10. Verification and uncertainty

**Review controls: not_run** — no `cargo check/build/test`, `just verify-*`, `just qualify`, product services, performance probe or real-library restart. The assignment was read-only plan assessment, with only the temporary review text written. Static evidence above is dated 2026-10-07. Existing coordinator receipts are historical bounded evidence and do not qualify the proposed contracts.

The plan's targeted functional scope is meaningful: it challenges semantics and exposures rather than mirroring implementation or comparing counts. Small independent graphs should include isolates, parallel occurrences, reconvergence, legal data cycles and invalid proof cycles, cross-module premises, missing/foreign rows, complete-empty and shuffled contributions. Native cases must challenge late terminal errors, unknown acknowledgment, actual persistent restart, read-only sealing and pin continuity. Transport cases must remove/alter/add state membership and required backing, and verify identity across physical names. Wire cases must hit exact cap boundaries with escaping, Unicode and actual final delivery maps.

Remaining investigations are bounded and correctly sequenced: PG1 registry/body/key coverage; PG2 uncertain actual selectivity/ordering; PG4 selected dependency union and compact S0 support lifetime; PG8 allocation/retention ordering. These are not grounds to claim implementation readiness now. Their closure can use static interface/source inspection and targeted functional controls appropriate to the uncertainty; no performance campaign, numerical budget model or compulsory EXPLAIN/proof artifact is required.

The important unmeasured premise is absolute end-to-end performance. The reviewed route removes named repetition while adding native persistence/index work. Static evidence supports architectural fit and selection, but a completed representative real-library run is needed for a Measured speed/capacity claim. That later work remains separately authorized.

## 11. Rule impacts and disposition

No **additional** operator rule change is recommended beyond the already selected proposal. For discoverability, the consequential impacts are listed here with stable review identifiers; this review applies none of them.

| ID | Rule and location | Selected change / dependent obligations | If the old rule were retained |
|---|---|---|---|
| RC01 | Storage §6.1 and ADR-0128 ordinary portable export/verify/readmission before publication. | Approved direct sealing of owned admitted native state; external transport admission and actual stored reconciliation retained. Source F03; PG0/PG5/PG7. | Ordinary compilation retains its round trip and conflicts with the selected same-database route. |
| RC02 | `SourceSnapshot`, completed-input digest and observation/cache/manifest bindings. | Approved exact internal contribution/view identity; deterministic final content identity and exact membership retained. Source F02/F03; PG0/PG1. | Full-prefix identity cost remains; a segmented implementation alone cannot claim its removal. |
| RC03 | DESIGN §B3/§B7/§B12, semantic-model §15.1/§15.11 and storage §§5–6.2/ADR-0128 store-free/post-compile-only placement. | Explicitly selected persisted core compilation; native dependency applies to artifact-only; pure kernels remain explicit-value operations. PG0 and all compiler migrations. | The requested persisted compiler outcome is unavailable. |
| RC04 | Existing manifest/backup semantics and instructions saying complete snapshots transport graph/original families only. | New completed-state format/digest and necessary backing enter complete artifact/handle identity and transport; obsolete formats retired. PG1/PG5/PG7/PG9. | Dependency/view state would be lost or misidentified on export/restore; retaining the old meaning would require withdrawing complete-state transport. |

RC03/RC04 are consequences of the operator-selected native/foundation outcome, not newly requested alternatives. PG0 must supersede the affected ADR rules and update their owning sections with Proposed/implemented distinctions; accepted decision text is not implementation closure. Surviving identity, attribution, programmatic synthesis/evaluation, complete values/projection policy, pinned readers and explicit selection remain constraints.

The scheduled F01–F05 and D01–D04 disposition remains solely in plan §8. This review creates no second ledger and no new standing validation/document type.

## 12. Architectural judgment and decision

| Judgment | Verdict | Scenario basis |
|---|---|---|
| A1 Localize change | satisfied | Model semantics, native mechanics, orchestration, publication and serving have coherent boundaries. A new record or analytic changes its owner and derivations; pure kernel tests stay isolated. |
| A2 Encode domain meaning explicitly | satisfied | Contributions/views/dependencies/completion distinguish all relevant new states, identities and outcomes, and proposed behavior must consume those authorities. Graph/backing/state distinctions and frontier lowering are explicit. |
| A3 Extend through composition | satisfied | Finite common selected-access, range, codec, preparation and encoding mechanisms have actual shared consumers; new methods compose their dependency union without another cache/provider framework. |
| A4 Fit supported workload | satisfied at Proposed strength | Indexed membership and batched access remove per-grain rich scans; local contribution identity removes prefix work; demand/lifetimes reduce unused preparation; direct seal removes self-import. Spillable/global kernels and bounded effect units retain necessary whole-universe and failure work. Native persistence/index costs buy the selected useful graph/dependency substrate. No latency or capacity claim follows. |

**Decision: Accept the Proposed persisted graph execution architecture and package dependency sequence.** This is a design decision, not acceptance of an unimplemented migration or closure of the executing baseline's catalog-speed findings. The enclosing implementation continues to need the planned corrections and targeted native/transport/serving integration; broader provider/behavioral quality, operator adoption, real-library completion and measured performance are not certified.

The highest-consequence implementation risks are exact membership/identity and complete-state transport, followed by selected access and validation preservation. Prerequisite order therefore starts with PG0 and PG1's real small producer/consumer foundation before PG2 and dependent compiler migration. PG6/PG8 can proceed when their actual shared contracts are ready; PG7's complete transport must land before PG5 claims complete publication/restore behavior. Logical parallelism does not permit multiple writers of shared model/schema/codec declarations.

The coordinator's next step is to publish this assessment, retain the source findings as Open/Proposed, and execute PG0–PG2 within the subsequently authorized implementation boundary. No additional approval gate, pilot restart, benchmark campaign or implementation action is introduced by this review.
