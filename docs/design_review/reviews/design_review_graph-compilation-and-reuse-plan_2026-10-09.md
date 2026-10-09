# Graph compilation and selective reuse: implementation-target review

**Design / target review · 2026-10-09 · Decision: Accept scoped at Proposed strength.**

The two companions specify a coherent replacement for repeated per-root selection and independently interpreted body/coverage membership. They place complete scope meaning in the model, compile shared physical demand without merging logical validity, and distinguish reusable programs, prepared topology and pure results. Their reuse contract includes negative domains and changing membership; equal computation does not preserve stale contribution provenance or grant admission. The persisted coordinator preserves the independent completion, native reconciliation, publication, transport and compilation-cost obligations across this replacement.

This is an independent assessment of a document target. Existing decisions, the source review and the accepted RC01/RC02 were examined as subject and motivation, not treated as proof that the replacement is correct. The bounded Moka addition exposed one retirement-order gap, F01 below; the final target resolves it at Proposed strength. No blocking architectural finding remains in that revised target. The source review's F01/F02 remain open at the coordinator until implementation and their closure evidence exist. This acceptance does not qualify the production compiler, measured reuse benefit, cache capacity, real-library behavior or operator adoption.

## 1. Scope, outcome and coverage

| Field | Reviewed boundary |
|---|---|
| Principal reviewer | Fresh delegated design reviewer, independent of the plan author |
| Date and baseline | 2026-10-09; main `8f11d9a0`, with root-owned documentation edits and two new plans. Production sources were read in place; the supplied untracked `docs/graph_hashing_reference/` was preserved. |
| Principal subjects | [Compiler companion](../../plans/graph-compilation-kernels-and-hashing-plan_2026-10-09.md), [reuse companion](../../plans/graph-compilation-reuse-and-invalidation-plan_2026-10-09.md), and [persisted coordinator §7.1/§8/§9](../../plans/persisted-graph-execution-plan_2026-10-07.md#71-replacement-execution-and-cross-plan-contracts-2026-10-09) |
| Standard | Core/template3.3, heuristics1.0, code-intelligence principles/review1.5 and library-context binding, loaded from [standard.toml](../design_principles/standard.toml); `design-review` and its code-intelligence companion |
| Tier and purpose | Design / target: best architecture for the functional outcome, including credible extension and mechanism substitution |
| Functional intent | Version-pinned API/evidence discovery and correct use, with attributed extraction, declared graph/program analyses, changed-release recomputation and same-realization evidence journeys. General behavioral completion and product differentiation remain separate qualification questions. |
| Workload premise | One local operator, overlapping callable/class/field roots, relation-rich library/corpus inputs, large classes and high-degree outliers, optional upper analyses, repeated immutable-view requests and concurrent consumers under a shared resource budget. |
| Included | Model scope operations; native/relational/compact graph execution; normalization and support admission; Structural/Local/Model/Summary/Analytic and C0–C2/S0/E0 preparation/results; selective cross-run products; pinned serving preparation; hashing, publication boundaries and Rust build consequences. |
| Excluded from acceptance | Implemented behavior of the new compiler/cache, general live incremental deletion, arbitrary program interpretation, distributed scheduling, retained old-format compatibility, real-library/Qwen qualification, protected evaluation activation and operator databases. |
| Method | Static inspection of target contracts, current owners and decisive source; resolved lockfile and selected pinned library interfaces, including the final Moka retirement correction. No builds, probes, environment preparation, databases or benchmarks. |

Final reviewed compiler/reuse document SHA-256 values are respectively `ec586872be40275cc1a61e7a0bc2b1967cff3bff7e5a4dc27edd33c7f34a9dfc` and `90ead772771e4b678c24b0bfd84a23161eef51b93301f67659b49cb3411919f5`. Later mechanical publication/link changes do not expand this assessment.

Adjacent inspection included semantic-model §15.1/§15.10–§15.12, storage/publication §5–§6, product §14, correction PC contracts, populated PJ foundations, BC/CU containment and qualification, and the graph-native/evidence companions' supersession boundaries. This was not an exhaustive audit of every provider, invariant or packet caller. The family-wide migration inventory is a target obligation; current code coverage is not inferred from that inventory.

## 2. Responsibilities, dependencies and domain fidelity

The design preserves three graphs with different meanings. Semantic entities and identity-bearing arcs describe program/evidence phenomena. The producer graph controls exact completed dependencies and readiness. The operation graph describes selections, memberships, projections and kernel application. A cycle in program meaning belongs to its fixed-point/refusal contract; it does not become a producer scheduling error. An operation node does not automatically create a database table, task or transaction.

| Owner | Responsibility and consumer contract | Dependency and reason for change |
|---|---|---|
| `lctx-model::domain` | Typed scope meaning, applicability, roots/admission roots, directional membership, coverage/absence, outcomes, canonical program/product encoding and substitutability | Mechanisms consume model meaning. A changed body, coverage or result-equivalence rule changes this owner. |
| `cpg-core` | Bind exact sources and output demand; compile physical operations; share preparation; invoke retained kernels; coordinate declared product dependencies and current admission | Depends on model and native access. A physical batching/lowering change is distinct from a semantic operation revision. |
| `lctx-surrealdb` | Mechanical fields/indexing, bounded projected transport, terminality, complete derived cache insertion/lookup and retention | Depends on model contracts; cannot mint core admission or publisher authority. Native layout/transport changes belong here. |
| Existing model/analytics kernels | BDD/transfer/SCC/FCA and other owned numerical or finite algorithms | Consume selected inputs and retain their own assumptions, convergence, ordering and refusal. New domain algorithms may remain ordinary functions. |
| Publisher and pinned viewer | External visibility and coherent realization; viewer-local preparation under the exact snapshot lifetime | Publication effects and reader credentials are never reusable pure products. Snapshot replacement constructs another owner. |

The decisive current duplication is visible independently of the earlier review: [scoped_aspects.rs:106](../../../crates/cpg-core/src/scoped_aspects.rs#L106) authors source/span/path containment, initializer expansion and context coverage joins; [symbolic_fields.rs:355](../../../crates/lctx-model/src/domain/normalized/symbolic_fields.rs#L355) and its `ClassInventory` repeat those meanings; [AspectScope](../../../crates/lctx-model/src/domain/normalized/callable_aspects.rs#L1169) currently declares roots and memberships without the complete operation. Compiler §3.2/GK1 moves that missing meaning to the normalized model owner and requires both inventory computation and physical selection to consume it. This addresses the operation-model gap rather than simply renaming output records.

| Family | Attribution/revision and fidelity | Coverage, identity and consumers preserved by the target |
|---|---|---|
| Source/syntax/provider observations | Captured original source and exact provider/settings contracts; extracted or provider-attributed facts | Complete versus unavailable families remain explicit; source/span/structural role remains nominal; normalization/catalog consume them. |
| Normalized callable/class/field properties | Owned resolution/derivation operations over exact completed inputs | Body/context/qualification domains and missing advertised owners are explicit; supporting references do not become new body roots. |
| Containment/calls/projections | Declared relation roles, direction and independent vertex inventory | Isolates, parallel identity-bearing arcs, unresolved targets and complete intermediate universes survive; physical indices remain local. |
| Execution/Model/Summary | Owned model, assumptions, invocation and proof premises | Five verdicts, negative domains, transfer semantics, ordered application and SCC policy remain distinct from reachability. |
| Analytics/ranking/vectors | Selected methods, settings, seed, projection and actual value/spec identity | Heuristic output remains heuristic; NotRequested and non-convergence/refusal stay explicit; embedding reuse retains its existing owner. |
| Catalog/evidence/selection | C0/C1/C2, S0 and original-context/witness owners | Matching requirements, joint context, conflict, unknowns and original bytes retain their meanings and same-realization references. |

This table assesses preservation contracts at Proposed strength. It is not new analyzer, analysis-quality or evidence-service qualification.

## 3. Complete operations and reusable contracts

The proposed scope operation is sufficient for the selected extension/substitution scenarios because it consumes typed roots, exact named input bindings and policy, decides body/initializer and directional membership, joins qualifications to context-sensitive coverage, and produces separate root outcomes plus owner views. It declares missing namespaces, advertised ownerless rows and global/negative obligations. Selection cannot replace the actual support/reference predicate. Consumers can stop independently interpreting containment and coverage while keeping their specialized normalizers and validators.

Shared physical membership is intentionally weaker than shared validity. A class root and an initializer root may share decoded premises but still produce separate outcomes and inventories. Deduplication follows declared relation identity and ordering, not endpoint equality or the fact that two records share a physical fetch. Scalar/list, forward/reverse, source role, repeated declarations and evidence multiplicity remain visible. Independent known answers are necessary because deriving two lowerings from one declaration can reproduce one shared semantic mistake.

Reuse §3 separates:

- canonical operation intent, independent of SQL aliases, code pointers and arena indices;
- prepared topology/layout, dependent on typed membership, ordering/multiplicity and physical revision;
- pure typed result values, dependent on all result-affecting inputs and an explicit eligibility/substitution contract.

The durable request key includes role-associated dependencies, model/vocabulary, operation/kernel/code/settings, parameters, policy/profile and result contract. Whole-view tokens are the conservative first realization. Selected domains additionally cover full matching membership, coverage and missing/empty outcomes. New matching rows and failed-to-resolved lookups therefore invalidate a negative answer even if all previously returned rows remain unchanged. A cached selector cannot validate its own domain token: the current bound completed source supplies or independently validates it.

[ContributionSpec and CompletedContribution](../../../crates/lctx-model/src/domain/completed.rs#L13) already separate captured binding, profile, model, implementation, inputs and output content. [CompletedView](../../../crates/lctx-model/src/domain/completed.rs#L41) derives exact logical contribution membership without a physical database name, and [SourceSnapshot](../../../crates/lctx-model/src/domain/analysis/sources.rs#L10) describes a completed input rather than a read capability. These interfaces support portability. They do not implement the proposed cache or justify weakening view dependencies automatically.

Default result equality includes the canonical output's identity, attribution and evidence fields. A deliberately computation-only product may omit source coordinates only when its owner regenerates every affected occurrence/support/output identity from current inputs. That case is not permission to relabel old evidence. Equal value cuts off only value-dependent consumers; changed contribution/source provenance still refreshes. Consumers observing exact `SourceSnapshot` remain exact-view-dependent until a finer complete contract exists.

## 4. Composition, execution fit and hashing

The compiler's physical route has a credible purpose beyond a selector-only refactor: one inspectable scope contract serves normalization, admission and serving; root parameterization shares semantic compilation; compatible roots share discovery/decoding; and resulting products can participate in selective reuse. The bounded language excludes arbitrary SQL and unrestricted request code. It does not attempt to encode every specialized algorithm or create a plugin/task framework.

| Operation/question | Universe and semantics | Physical realization and settings | Resource/outcome/evidence boundary |
|---|---|---|---|
| Complete callable/class/field scope | Exact typed roots, containment/initializer membership and contextual coverage; separate partitions | Indexed native/DataFusion or compact graph realization of the same model operation | Bounded hydration, spillable compact membership, explicit missing/unsupported/refused outcomes; support predicates still execute. |
| Structural/topological preparation | Independent vertices and typed directional ArcIds; full required intermediates | Shared nominal/dense mapping and existing graph/SCC kernels under declared projection policy | Charge preparation/visits, preserve isolates/parallel evidence, hydrate rich witnesses separately. |
| Execution and Summary | Full owned transfer/model premises and negative domains | Existing BDD/transfer/fixpoint algorithms and settings; topology may schedule but cannot decide the transfer result | Five verdicts, deterministic ordering, convergence/refusal and exact proof linkage. |
| Catalog/evidence/retrieval | Artifact/member/context partitions, actual original ranges and witness policies | Shared metadata/closure demand; retained compact S0 and existing embedding/value subsystem | Original bytes, supported/unknown/conflict and winning-witness contracts remain. |
| Selective cross-run computation | Declared complete dependency domains plus result contract | Native immutable pure-product store; fresh affected snapshot computation after deletions/negative changes | Validate current domains; reconstruct current ownership; never reuse publication effects or checked capabilities. |
| Repeated pinned request | Exact realization, root/program/policy/binding and evidence contract | Viewer-owned prepared metadata/layout or qualified exact closure | Pin and independent consumer lease; bounded retained state; another request's cancellation cannot invalidate a surviving consumer. |

Sparse demand retains native indexes and projected streams. Repeated traversal over a reusable selected universe can justify compact adjacency and dense visits. A high-degree root can remain individually streamed against the shared program rather than forcing all roots into a huge union. Shared hydration is bounded or handle-based; avoiding decode does not justify retaining every rich row. The plans account for cold conversion, repeated preparation, transfer, admission and retained state separately. Whole-view reuse can avoid pure computation cheaply; selected-domain reuse still pays fresh compact selection when completeness requires it. Neither a digest nor a cache entry eliminates that necessary work.

The current [native provider](../../../crates/lctx-surrealdb/src/compiler_provider.rs#L179) exposes Exact pushdown for supported expressions, projection and charged selected-key owners; [grain_roots](../../../crates/cpg-core/src/consumed_rows.rs#L960) still prepares root queries and traversal state per invocation. The target preserves useful native/DataFusion capabilities instead of replacing every join with Rust loops. Its native residual/fetch/order/late-error obligations prevent fusion from changing semantics. Bounded compact state can spill; an oversized graph has indexed access rather than compulsory full in-memory construction.

Hashing has two separate jobs. XXH3-128 locates process-local program candidates and requires full structural equality. BLAKE3 retains durable program, dependency and content identity. Canonical framed encoding, typed roles and ordered versus set semantics precede hashing. Topology fingerprints include independent vertices and exact typed/directional arcs, including internal SCC edges; node membership or child-hash multisets cannot stand in for topology. Forced XXH collisions must remain ordinary unequal buckets, and no fast-hash key grants validity.

The build design preserves one bounded non-generic physical execution loop, coarse async request/stream boundaries and synchronous typed leaves. A newly declared operation must not produce a large generic async branch per row, relation or lowering. This is credible preservation of BC/CU's structural remedy; its emitted-code/runtime evidence and BC3's candidate/default qualification remain required. A native machine-code compiler, e-graph or another expression interner has no necessary role in the selected scope. Existing BDD canonicalization is retained.

## 5. Change, growth and failure scenarios

| Scenario and change kind | Owner and expected propagation | Reasoned target result and revealing evidence |
|---|---|---|
| Add another supported body/context membership rule — domain operation/policy extension | Normalized model operation changes once; finite evaluator and physical lowerings consume it; relevant result/program revision changes | No independent SQL/ClassInventory semantic edits. A nested class, equal span with different structural path and foreign context distinguish the rule. |
| Add an analytic over existing facts — composition plus genuinely new algorithm | Analytic owner declares exact projection, settings/outcomes and consumer; common selection/preparation and product contracts compose | No provider algorithm or publication interpreter is added. Selected-only dependencies and NotRequested remain exact. |
| Substitute compact graph for native selection — mechanism change | Physical compiler/native adapters change; operation meaning, root partitions and admission stay owned | Known-answer cases preserve isolates, parallel arcs, missing namespaces, order and context. Existing native residual/finality protections remain wherever native execution is retained. |
| New library release introduces a target of an unresolved lookup — instance/input change | Current source/domain tokens change; dependent selector/product recomputes | Empty/unknown becomes resolved only under current coverage; unrelated qualified domains can reuse. Clean rebuild and independent expected evidence challenge invalidation. |
| Equal computation with changed source/span/provider provenance — binding/input change | Result contract decides substitution; current contribution/support/source owners refresh | Full canonical products default to a miss. Computation-only products regenerate current identities; exact-view observers do not cut off. |
| SCC merge/split or internal/parallel edge change — topology change | Full topology token and retained kernel schedule change | Same members are insufficient. Fresh affected snapshot computation preserves cycle semantics and witnesses; no general deletion algorithm is claimed. |
| Many overlapping roots plus a high-degree outlier — growth/skew | Compiler chooses compatible bounded batches or individual streamed demand against common preparation | Rich live state is bounded; logical owner outcomes remain independent; compact/spill/indexed options avoid compulsory global expansion. |
| Concurrent same-key writers, cancelled consumer or retirement — concurrency/failure | Native cache/store and viewer own terminal visibility and leases; runtime owns exact charges | Fully accounted equal products converge; valid unequal products fail determinism/contract checks; eviction cannot release still-live charges or interrupt another consumer. |
| Publish a new realization during an evidence journey — lifecycle change | Publisher constructs/seals a new owner; existing viewer/cache keeps old pin | References, resources and continuations stay in their originating realization; an equal digest does not transfer a live reader grant. |

The local reference evaluator permits scope/kernel controls without extraction, a database or serving. Native lowerings and cache completion/retirement need owned disposable fixtures, because those controls must exercise the actual persistence/finality boundary. This division is necessary isolation, not a promise that physical behavior can be established by the finite oracle alone.

## 6. Independent correctness and fidelity gates

Verdicts here concern whether the Proposed target specifies an adequate route and obligations. They are not executed test outcomes.

| Gate | Verdict | Independent evidence and scope |
|---|---|---|
| G1 Authority | Pass at Proposed strength | One model operation owns scope meaning; native layouts/cache are derived; actual source duplication is explicitly scheduled for removal. |
| G2 Semantic fidelity | Pass at Proposed strength | Separate roots, advertised obligations, source/context/coverage, list/order/multiplicity and uncertainty survive lowerings and products. |
| G3 Validity | Pass at Proposed strength | Exact bindings, real predicates, current domain validation and typed ingress precede fresh completion/admission. Cache metadata and physical freeze mint no admission capability. |
| G4 Hidden behavior | Pass at Proposed strength | Pure eligibility is explicit; external acquisition/runtime/publication effects remain effectful; arbitrary request code and ambient/latest reads have no route. |
| G5 Consistency/recovery | Pass at Proposed strength after F01 correction | Only terminal complete products become visible; late errors/conflicts remain explicit. Viewer retirement fences and drains initialization/insertion before final cache release and external-lease waiting. |
| G6 Transformation/reuse | Pass at Proposed strength | Canonical encoding, full keys, complete positive/negative/topology dependencies and value/provenance separation govern substitution. Independent clean-rebuild and known-answer controls remain required. |
| G7 Truthful capability claims | Pass at Proposed/interface strength | Target and benefit remain Proposed/unmeasured; dependency/features and selected interfaces are version-qualified; no authoring or historical pass claims new production readiness. |
| G8 Library leverage | Pass at Proposed/interface strength | Native store/transport, DataFusion, petgraph/bitset and retained kernels provide generic mechanics; the bounded bespoke portion is scope/dependency/substitution/admission meaning. Moka supplies coalesced fallible viewer initialization/eviction; Salsa and snapshot relation engines are considered with explicit lifecycle gaps and revisit triggers. |
| CI-G1 Fidelity | Pass at Proposed strength | Attributed providers, typed relation meanings, five verdicts, complete coverage and governed heuristics remain independent of execution placement/cache equality. |
| CI-G2 Evidence closure | Pass at Proposed strength | Current evidence/support identities and complete same-realization pin remain mandatory; cache equality cannot relabel stale source or move references across snapshots. |
| CI-G3 Evaluation integrity | Pass for the preserved boundary | No gold/heldout activation or production truth input is introduced; small independent expected graphs and clean recomputation do not replace protected quality evaluation. |

## 7. Findings, applicability and relationships

<a id="F01"></a>
### F01 — Viewer retirement must fence and finish insertions before releasing cache-owned leases

**Source/interface-inspected diagnosis, 2026-10-09; revised remedy Proposed.** DP-19/20, FP-05/07, G5 and A4 apply. The first Moka-backed GR5 draft said retirement cancels preparation, invalidates entries, drains work/maintenance and waits for retained value leases. Moka0.12.16 [future/cache.rs:1374–1388](https://github.com/moka-rs/moka/blob/v0.12.16/src/future/cache.rs#L1374) implements `invalidate_all` through an invalidation timestamp; entries inserted later are not covered. [future/value_initializer.rs:239–247](https://github.com/moka-rs/moka/blob/v0.12.16/src/future/value_initializer.rs#L239) inserts a successful initializer result after evaluation. An initializer already completing can therefore insert after the earlier invalidation, retaining the retired viewer's cached value/pin/charge while retirement waits for value leases. Generic invalidation does not fence owned initialization/insertion.

The correction belongs to the viewer lifecycle owner: fence new request/preparation admission, cancel/drain every admitted initializer and insertion owner, then perform final invalidation plus applicable maintenance or drop cache references before waiting for external reader/value leases. Remaining waiters cannot restart preparation after closure. Request cancellation remains independent of shared preparation, and external submitted work retains its ordinary terminal/drain ownership. A finite retirement-versus-completing-initializer control must show no post-retirement entry remains reachable and that an external borrower retains its own pin/charge until release.

**Dated target reassessment, 2026-10-09:** the revised GR5 contract specifies that ordering. Static inspection resolves the Proposed ordering gap; production behavior is not_run. This is a separate plan-target finding, not source graph/hash F01 and not source-finding closure. Its disposition route is [persisted coordinator §8](../../plans/persisted-graph-execution-plan_2026-10-07.md#8-sole-finding-disposition).

No additional blocking `Fnn` is established in the final reviewed target. This does not erase the earlier defects or close their production disposition. Source-qualified [graph/hash F01](design_review_graph-compilation-kernels-and-hashing_2026-10-09.md#F01) and [F02](design_review_graph-compilation-kernels-and-hashing_2026-10-09.md#F02) remain at [persisted coordinator §8](../../plans/persisted-graph-execution-plan_2026-10-07.md#8-sole-finding-disposition). F02's complete scope authority is a prerequisite for sound aggressive sharing and finer domain reuse; F01's work-granularity correction then preserves separate results and validity. The plans preserve that relationship rather than combining the findings into one hashing defect.

FP-01/02/03/06 are satisfied at Proposed strength by coherent semantic/mechanism/effect owners, the finite evaluator and substitution route. FP-04/05 are satisfied at Proposed strength by complete root/domain/dependency/outcome/lifecycle definitions governing selection and reuse. FP-07 is satisfied for the stated qualitative workload by shared preparation, indexed sparse execution, bounded hydration and compact/spill alternatives. Relevant DP-01–DP-12 and DP-18–DP-24 are supported by those contracts; DP-13–DP-17 are supported by the bounded library composition and retirement obligations. This is grouped applicability, not a claim of exhaustive implementation compliance.

Important preservation constraints are actual support/reference predicates, exact source and role attachment, independent cold reconciliation, pin lifetime, current provenance, coarse asynchronous containment and all final-source acceptance. An implementation that shares membership by merging validity, caches admitted handles, or reconstructs a second independent scope policy would reopen these judgments even if selected output rows matched.

## 8. Library fit and total complexity

| Capability/candidates | Scoped interface evidence and composed tradeoff | Judgment |
|---|---|---|
| DataFusion55.1.0 plus SurrealDB3.3.0 native selection | Current provider shows optimizer-visible Exact filters, projection and charged key owners. Native scope/indexed access and checked statement terminals remain retained mechanisms. Compact graph execution complements sparse native work. | Suitable composed lowerings; native query count alone is not a performance claim. |
| petgraph0.8.3/fixedbitset0.5.7 | Lockfile and pinned references checked. Exact `Csr` source permits self-loops but excludes parallel edges; selected quotient needs exact ArcId/evidence mapping and suitable reverse/algorithm support. Bitset state owns universe length/generation. | Keep canonical multigraph and existing shared views; use dense/CSR forms only under declared projection contracts. |
| Native canonical product persistence and explicit dependency graph | Existing immutable completed descriptors and typed codecs are portable; separate acceleration state uses the same server/transport rather than another primary backend. The model supplies complete domain and equality semantics the store cannot infer. | Proportionate for batch immutable products; avoid a generic scheduler/cache framework. |
| Salsa0.28.5 | Exact locked `database.rs:167–238` supplies persistence and exclusive serialization; `function/backdate.rs` checks equality subject to provisional/cycle/durability restrictions; `storage.rs:153–191` cancels/waits for clones. The shared skill is0.28.4 and its test receipts do not qualify0.28.5 integration. | Viable alternative, not rejected for absent persistence. Native reads/absence, source identity, portable output/admission and retained ingredient lifetimes still need integration. Existing batch product shape favors the proposed narrow store; revisit a fine-grained mutable-query workload. |
| Ascent0.8.1/Datafrog2.0.1 | Reasoning reference supplies snapshot indexed/fixpoint and monotone-set candidates, not a qualified incremental deletion/negation integration. Neither is a current product dependency. | Optional lowering when a named relation fixpoint needs it; cannot substitute topology for transfer or assume retractions. |
| Differential/DBSP | No version-qualified project integration was examined in this review; signed-delta capability is a discovery candidate. | Not necessary for the selected snapshot-recomputation contract. Revisit dominating repeated insert/delete workloads; replacing the dependency owner is preferable to adding a competing graph. |
| Moka0.12.16 future cache | Exact registry `Cargo.toml.orig`, `future/cache.rs` and `future/value_initializer.rs` checked: future feature, `try_get_with` uncached errors, same-error-type coalescing, abort retry, timestamp invalidation and best-effort weighted bounds. Not currently a lockfile dependency. | Suitable replaceable viewer-cache mechanics; charged `Arc` payload/pin, admission and retirement remain owned contracts. Final insertion fencing is required by F01; approximate weights are not a hard memory bound. |
| XXH3-128, xxhash-rust0.8.19 versus twox-hash2.1.5 | Lockfile pins checked; exact `xxh3.rs` exposes `xxh3_128` and inline `Xxh3Default` streaming state. Feature addition is explicit. Hash-consing compares canonical structure and preserves durable BLAKE3 domains. | A narrow justified primitive selection; no global map-hasher change or measured throughput claim. |

The native reuse store, viewer-local cache and embedding value cache have different product scopes and lifetimes. They must not duplicate dependency policy or turn approximate native counts into current-domain proof. Moka weight scaling/bypass cannot substitute for exact retained-payload charging; eviction removes only the cache reference and does not revoke external borrowers. The canonical program interner remains non-evicting within its model lifetime so eviction cannot create duplicate live dense/pointer identities. Cache bookkeeping and encoding cost must be included when evaluating reuse. The target expressly keeps conservative whole-view tokens when a finer domain is not qualified; that avoids unsound reuse without making coarse-only invalidation the final design.

## 9. Alternatives and tradeoffs

The present prepared-edge design remains a useful foundation but does not meet the overlapping-root and operation-extension scenarios: root query/traversal/hydration still repeats and body/coverage meaning remains separately authored. Faster hashing or a blanket map replacement does not remove those causes.

A selector-only repair is the simplest viable alternative. It could move complete scope meaning to ordinary model functions and directly improve `PreparedEdges` batching. It has less program machinery. The proposed compiler earns its additional bounded operation representation through multiple real consumers, contrasting lowerings, program interning and persisted products. The comparison would change if implementation turned ordinary kernel callbacks into an unrestricted interpreter or automatically emitted physical stages per logical node; neither is part of the accepted target.

A Salsa-centered design can provide tracked dependency/backdating machinery, but the native immutable source-domain and fresh-admission contracts remain project work, and ingredient persistence adds serialization/exclusion and lifecycle integration. A Datalog/delta engine can provide named relation fixpoints, but does not by itself encode provider fidelity, negative coverage, evidence provenance or publication. The current target chooses immutable products plus fresh affected snapshot computation deliberately rather than claiming general incremental graph updates. This is an execution/mechanism choice, not a claim that those libraries lack the capability.

## 10. Verification and consequential uncertainty

**Static target inspection: passed, 2026-10-09**, using the document/source reads identified above and resolved lockfile/selected exact registry interfaces. This means the selected contracts and qualitative routes were inspected. **Product compile/tests/probes/benchmarks: not_run**; no corresponding product command was issued. Root documentation checks are separate authoring receipts and are not reviewer-run product evidence.

| Claim to establish during implementation | Required settling evidence already routed by the plans |
|---|---|
| Owned scope governs all applicable callers | GK1/GK3 extension/substitution inspection and independent nested/foreign/missing cases; old interpretation removed, not merely wrapped. |
| Shared work preserves logical validity | GK2 overlap/skew controls with actual owner partitions, list/order/evidence and complete negative domains; source shows bounded shared demand. |
| Cross-run portability and fresh ownership | GR2 two physical attempts and reload; ordinary ingress, current specs/bindings, completion, semantic/global admission and final freeze/drain. |
| Finer invalidation is complete | GR4 insertion/deletion, absent-to-present, context/coverage and SCC merge/split/internal-edge cases; clean rebuild plus independent known answers. |
| Cached data cannot bypass trust boundaries | Corrupt/stale/partial/coherently native-tampered entries, valid same-key unequal outputs and late statement errors; publication/cold import protections independent. |
| Capacity and cancellation are sound | Concurrent writer/reader, pinned request, eviction and cancellation controls; live payload and scratch remain charged until actual last consumer/worker terminality. |
| Build/runtime preservation survives composition | GK7/GR6 consume CU6/BC3/BC5 final-source profiles, cardinality/order/first-error and structural/runtime evidence; normal available parallelism remains. |
| Enclosing functional scope is qualified | Final both-profile/frontier, native/PyO3/MCP/evaluator, backup/restore/cold controls and applicable leaves, including remaining failed timeouts and assembled acceptance where authorized. |
| Quantitative benefit | Equivalent cache-off/cold/no-hit/warm/reload complete-operation measurements, including encoding, lookup, selection, transfer, admission and retained memory. No numerical claim is established now. |

The plan's specified entry visibility and lifetime contract is sufficient for target review. The revised target orders viewer admission fencing, initializer/insertion drainage, final cache-reference release and external-lease waiting. Source-level synchronization and the retirement race control still require implementation evidence. No mandatory probe or benchmark is needed to settle the current architectural direction. A demonstrated material implementation mismatch, an unbounded retained representation or key validation repeating the avoided rich work would require correction before declaring that package ready.

## 11. Rule impacts, supersession and acceptance ownership

The accepted source-qualified [graph/hash RC01](design_review_graph-compilation-kernels-and-hashing_2026-10-09.md#RC01) and [RC02](design_review_graph-compilation-kernels-and-hashing_2026-10-09.md#RC02) remain the rule-impact identifiers. This review creates no additional rule decision and does not apply their implementation changes.

| Rule impact | Required route and boundary |
|---|---|
| RC01 — graph/native/relational operation compilation | GK0/GK1 update semantic-model §15.10's topology-only/SQL-only restriction and establish complete operation authority; complementary ADR/owner changes preserve existing pure kernels and §B3's compatible seam. |
| RC02 — selective cross-run reuse | GK0/GR0 replace semantic-model §15.1/product §14.2's deferral, define portable product/dependency/substitution/store contracts and preserve one canonical authority and independent admission. |
| Overlapping open preparation routes | Coordinator §7.1 gives GK/GR precedence over PG2/PG4/PG6, PC3/PC4 and PJ normalization/validation/browse deferrals; existing completion, native access, ordering, freeze/pointer preparation and actual-state reconciliation remain useful foundations or obligations. |
| Surviving acceptance | PC6/CU6/BC3/BC5/PG9/PJ5 join final-source GK7/GR6. Failed timeouts remain failed/deferred until matching reruns; obsolete routes retire only with a replacement behavior/control map. |
| Original graph-native and ER/EV scope | Adjacent companions link to the new execution target while retaining their original source findings, semantic/evidence/quality obligations and dated receipts. Product or real-library closure is not transferred by implication. |

These amendments are subjects of the review, not constraints on its recommendation. Their direction is justified independently by the selected scenarios. Older unfinished plans are not a reason to defer the better target; their surviving guarantees remain acceptance requirements rather than mandatory completion barriers before new design. The coordinator's §8 owns graph/hash disposition; the companions contain packages/contracts rather than another mutable findings ledger.

## 12. Architectural judgment and bounded decision

| Judgment | Verdict | Independent reason and support limit |
|---|---|---|
| A1 Localize change | Satisfied at Proposed strength | Scope meaning changes in one model owner; backend mechanics and retained algorithms have separate reasons to change; finite tests need bounded inputs, physical controls need the native boundary they assess. |
| A2 Encode domain meaning explicitly | Satisfied at Proposed strength | Operations include applicability, roots, body/initializer/context/coverage, negative domains, outcomes, effects, dependency roles and substitution. Consumers are required to use and retire old interpretations; implemented authority remains unqualified. |
| A3 Extend through composition | Satisfied at Proposed strength | New membership/policy and analytic scenarios compose retained algorithms with complete operation contracts; native/compact substitution preserves meaning and admits genuine algorithm-specific code. No universal engine is required. |
| A4 Fit execution to the workload | Satisfied at Proposed strength | Shared stable preparation, indexed sparse access, compact repeated traversal, bounded hydration/spill, exact product reuse and current independent admission have credible routes for overlap, skew, growth and failure. Additional program/cache machinery serves named consumers; speed/capacity remain unmeasured. |

**Accept scoped at Proposed strength** for the combined graph-compilation/shared-root and selective-reuse architecture and its integrated preservation/acceptance route. No failed or unresolved target gate prevents that bounded decision. The acceptance is not conditional on finishing older plans first, and it does not waive their surviving behavior controls.

**Enclosing architecture:** the proposed replacement is acceptable for the reviewed document scenarios; production integration, all-caller retirement, runtime correctness, performance, real-library evidence and operator adoption remain outside this acceptance. The current source defects F01/F02 and inherited open completion/qualification obligations retain their existing owners.

The next architectural work belongs to GK0/GR0/GK1: encode the agreed scope/product/dependency contracts and their owner/decision changes before dependent implementation. Scope authority precedes aggressive sharing; safe portable products precede finer propagation. Final-source GK7/GR6 acceptance must assess their interactions, not sum local positive judgments or substitute this review for execution evidence.
