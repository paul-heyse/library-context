# Phase 5 generation-bound serving — target design review

**Decision: Accept scoped, at Proposed target-design maturity, 2026-10-01.**

The proposed design provides a credible reconstruction of the ten retained tools over canonical Catalog generations. Its strongest choices are separating a live process guard from short-lived query connections, preserving one model classifier and ranking owner, admitting prepared classification through canonical content receipts, and reconstructing exact-input inspection through current proof owners. The disposable vector artifact has a legitimate physical purpose and remains subordinate to canonical embedding uses.

The focused foundation enhancements are warranted. F1 supplies a serving prerequisite; F2 and F3 improve existing adapter ownership and composition without requiring a wider semantic rebuild. No new blocking finding was established within this review. Acceptance supports proceeding to contract integration and implementation planning; it does not qualify an implemented serving service or certify the enclosing Phase 0–4 architecture.

## 1. Review contract and evidence boundary

| Field | Scope |
|---|---|
| Subject | [Phase 5 detailed plan](../../plans/semantic-model-phase5-detailed-plan_2026-10-01.md), [foundation enhancements](../../plans/semantic-model-foundation-enhancements_2026-10-01.md), proposed [ADR-0114](../../adr/0114-generation-serving-contracts.md), changed DESIGN §15.12 and §11 |
| Standard | Core/template 3.2; code-intelligence profile 1.3; library-context binding |
| Tier · purpose | Design · target |
| Reviewer · date | Independent delegated design reviewer, 2026-10-01 |
| Baseline | Main `16ffb9e4`, with preserved dirty source and the identified proposed documentation |
| Method | Static inspection of owning documents, decisive production sources, retained consumers, selected pinned library interfaces and capability references |
| Supported target | Ten retained routes; Catalog minimum frontier; generation-bound classification, packets, original evidence, exact-profile retrieval and optional finite native/brief enrichment |
| Exclusions | Full Phase 0–4 audit, PR6/new tools, ANN, arbitrary predicates/SQL, general heap identity, broader R4 admission, retrieval-quality and performance qualification |

Reviewed document digests:

- Detailed plan: `88771d881bcf909180effbb291eeff0d0155301be20c75715e712580c87818fc`.
- Foundation companion: `e7135bd97276e13119f11112ae0b36be1cde38533e72159c5f0ad2d79582edf7`.
- ADR-0114: `2649fd0b8570f418463e1d5215e07a29caab175ead6e842affbbde5b632177ad`.

**Coordinator disclosure, 2026-10-01:** after this review, ADR-0114's design-reference metadata removed §B13/§B14 because no binding decision changed, and product §14.7/§14.8/§14.9 gained explicit links to the proposal. The coordinator also linked this completed review and replaced future-review wording with its Accept scoped status in the main/parent plans and STATUS. These metadata, owner-link, navigation and status corrections change no reviewed target contract; the reviewed digests and judgment above are retained.

The detailed plan digest incorporates the coordinator’s disclosed prose-only replacement of `Vec<f32>` with “f32 arrays”; this changes no reviewed contract.

Documented architecture remains **Proposed**. Current-interface observations below are **Interface-checked, 2026-10-01**. Existing implementation is cited to establish the starting boundary, not as freshly qualified behavior.

## 2. Responsibilities and semantic ownership

| Owner | Responsibility and hidden decisions | Consumers and dependency direction |
|---|---|---|
| `lctx-model` | Canonical meanings; finite classification; exact-input assessment; ranking/fusion policy; serving mappings and wire contracts | Store lowering, repository preparation, native bridge and Python consume owned contracts |
| `lctx-postgres` | Published-generation admission, guard/query connection lifecycle, checked hydration, physical lowering and disposable vector effects | Depends on current model declarations; retains infrastructure failures and cleanup obligations |
| Prepared repository/runtime | Admitted owned metadata, private indexes, request orchestration and shared resource admission | Composes model operations over one leased generation |
| Python/FastMCP | Transport, lifecycle integration, numerical BM25 adapter and presentation | Receives declared schemas/settings and validated bridge values; does not interpret applicability or fuse independently |
| Operator CLI | Explicit vector preparation and activation | Pins canonical inputs; preparation/publication does not select automatically |

This direction is appropriate. Database effects remain outside pure classification/native operations, while Python retains a real numerical capability rather than acquiring a second semantic engine.

The domain distinctions are sufficient for this target: generation/model, mapping/physical realization, serving policy, wire representation, prepared consumer and request continuation have separate identities. Requirement witnesses remain distinct from ranking witnesses. Restricted request diagrams remain distinct from stored conditions. Unknown, unavailable, unrequested, corrupt and complete-empty outcomes are not interchangeable.

### Fact and fidelity boundary

| Served material | Authority and fidelity | Coverage/identity constraints |
|---|---|---|
| Public catalog, signatures/options | Canonical extracted/normalized/derived records and C0/C2 operations | Complete admitted domains; source/effective signatures and incompatible contexts remain distinct |
| Documentary/scenario/deployment evidence | Canonical C1 association and original artifact/chunk records | Association basis and source qualification retained; missing association does not erase addressable evidence |
| Briefs/assertions | Canonical S0 conclusions and support closure | Optional brief selection cannot shrink the catalog; Markdown and structured packets share closure |
| Retrieval ranking | Model policy over canonical units, library numerical scores and canonical embedding uses | Heuristic ranking nominates candidates; it supplies no behavioral support |
| Native inspection | Current conditions, Entry/rebase and finite proof owners plus the proposed exact-input operation | Path-local refutation and may-model compatibility remain bounded claims, never concrete execution or universal absence |

The provider reconstruction itself is outside this review. The target consumes its existing provenance and qualification rather than relabelling it.

## 3. Contracts, admission and failure behavior

The process-lifetime guard is a material correction to the retained `PinnedGeneration`, which currently contains legacy descriptors and artifacts rather than a canonical lock-holding lease ([repository.rs](https://github.com/paul-heyse/library-context/blob/0bc8ea11171d6fd96827a8e250964d3de5b3b4ce/crates/lctx-postgres/src/repository.rs), `PinnedGeneration` and `ServingStore::pin`).

The canonical protocol already supplies a useful foundation: [lease.rs](https://github.com/paul-heyse/library-context/blob/16ffb9e4ab3e8c5a5e7bec97668f7dfcfb307abe/crates/lctx-postgres/src/generations/lease.rs) checks publication state, model/physical compatibility and live columns before taking a session-level shared generation lock. [GenerationStore::cleanup_on](https://github.com/paul-heyse/library-context/blob/16ffb9e4ab3e8c5a5e7bec97668f7dfcfb307abe/crates/lctx-postgres/src/generations/mod.rs) requires exclusive generation access before deletion. The proposed serving-role route extends this ownership without requiring migration credentials.

The plan correctly avoids treating that lease as content validation. Published leases do not rehash all rows. Narrow classification therefore requires matching consumed relation/epoch receipts and the C2 declaration-closure validation receipt. [receipts.rs](https://github.com/paul-heyse/library-context/blob/16ffb9e4ab3e8c5a5e7bec97668f7dfcfb307abe/crates/lctx-postgres/src/generations/receipts.rs) contains both relation content receipts and validator receipts bound to canonical generation content/model/physical identity. Their exposure to the serving reader is new implementation work with a credible existing store owner.

Lifecycle expectations are explicit: resolve selection once; never repin after guard loss; reject new work and suppress success after detected loss; retain capacity and guard ownership until native completion; drain or discard failed query connections; do not transparently replay a request. Bounded supervision and final liveness confirmation do not promise instantaneous network-failure detection.

The shared memory allowance and CPU admission owner are meaningful contracts. The plan distinguishes reservations from total RSS and includes preparation, retained input, conversions and numerical state. A failed reservation is refusal, not empty output. Implementers must preserve that distinction across the Python numerical adapter.

## 4. Composition and execution

| Operation | Universe and method | Meaning, limits and evidence linkage |
|---|---|---|
| Classification | Complete admitted metadata domain; current finite classifier and contextual conjunction | Exact within the declared finite selection model; preserves completeness, conflicts and original requirement witnesses |
| Ranked discovery | Eligible canonical retrieval occurrences; BM25 and exact-vector channels; model-owned family aggregation/RRF | Heuristic ordering with declared ranked extent; filtering precedes final limits; original winning units remain addressable |
| Packet hydration | Sets of nominal IDs and declared mappings | Same-generation membership/roles; indivisible mandatory signatures; optional sections continue explicitly |
| Explanation lookup | Relation-qualified derivation DAG | Bounded depth/nodes/edges; truncation does not become semantic uncertainty or complete absence |
| Exact-input assessment | Admitted finite path/proof and justified request-scalar-to-atom links | Path-local refutation or compatibility under stated assumptions; unsupported links and exhausted work remain explicit |

The plan separates stable preparation from request interpretation and defers original bodies/full proof expansion. It avoids precomputing the meaning of future requirements.

Continuation identity includes actual channel state and query-vector identity. Rejecting changed channels is a reasonable first implementation without retained request-result sessions. Its cost is potentially recomputing unchanged deterministic channels; that is an explicit tradeoff rather than an undeclared cache.

## 5. Revealing change and failure scenarios

| Scenario | Change owner and propagation | Assessment |
|---|---|---|
| Add a supported predicate | Model predicate/evaluator and classification inventory; loader follows declared inputs | Credible domain extension. Inventory must include negative lookups and completeness context, as required by companion §2.2 |
| Add an optional packet field | Owned DTO/mapping and hydration dependency; schemas and physical realization follow | Localized representation change; optional addition does not fragment mandatory signature meaning |
| Replace BM25 numerical implementation | Numerical adapter and selected policy/settings; unchanged admitted occurrence/score contract | Credible mechanism substitution without changing classification or fusion |
| Change ranking/display policy | Service preparation and cursor identities | No source re-extraction required unless unit/spec meaning changes |
| Exact scalar with no valid bridge | Pure native operation returns explicit unknown/unsupported basis | Preserves fidelity; source/stub/decorated/default/namespace ambiguity cannot manufacture atom assignments |
| Caller cancels during native work | Runtime retains worker reservation and generation guard until completion | Lifecycle owner remains responsible after transport cancellation |
| A validly encoded consumed row changes | Receipt/content admission refuses | Prevents schema validity from being mistaken for semantic admission |
| Artifact contains a valid-width wrong vector | Canonical decode/digest plus numeric readback and explicit preparation controls | Derived physical artifact cannot become an independent value authority |

A full analyzer-upgrade journey is not examined: no provider ownership changes are proposed. Release changes are handled here through immutable generation/process identity, not a historical-ID bridge.

## 6. Correctness and fidelity gates

These are judgments about the specified **Proposed target**, not executed test outcomes.

| Gate | Verdict | Evidence |
|---|---|---|
| G1 Authority | pass | Model owns meanings/policy; views and vector artifact are derived; Python semantic duplication retires |
| G2 Semantic fidelity | pass | Context-compatible conjunction, typed availability, request/stored condition separation and native may-model limits |
| G3 Validity | pass | Admission, content receipts, closure/role checks, numerical validation and explicit rejection routes are specified |
| G4 Hidden behavior | pass | Read-only startup performs no acquisition/rebuild/selection; vector preparation is explicit; query path executes no library code |
| G5 Consistency and recovery | pass | Process guard, terminal loss, drain/completion ownership, atomic artifact publication and explicit resource/continuation outcomes |
| G6 Transformation and reuse | pass | Separate policy/wire/preparation identities; shared vector codec; channel-bound continuation; explicit numerical/readback controls |
| G7 Truthful capability claims | pass | Proposed contracts and not_run packages remain distinct from implementation and qualification |
| G8 Library leverage | pass | SQLx, SeaQuery, pgvector, BM25, Serde/Schemars, PyO3 and the current BDD kernel have bounded roles; no generic projection/framework replacement |
| CI-G1 Fidelity | pass | Unknown is not absent; rankings are heuristic; native refutation is path/model-local |
| CI-G2 Evidence closure | pass | Same-generation original sources, support membership and shared Markdown/structured closure |
| CI-G3 Evaluation integrity | pass within scope | Independent expected controls are preserved; no gold/reference input route or tuning is introduced |

Runtime realization of these gates remains **not_run**. That is the proper maturity boundary for this document review.

## 7. Findings, foundations and focused enhancement opportunities

**No new material finding requiring a target revision was established. No new F identifier is created.** Existing cutover findings retain their source-review IDs and current disposition owner.

| Foundation | Verdict | Reason |
|---|---|---|
| FP-01 Separation | satisfied | Model decisions, numerical adapters, physical effects and transport have coherent owners |
| FP-02 Stable contracts | satisfied | Finite inputs/outcomes, substitutions, identity and failure contracts constrain consumers |
| FP-03 Composition | satisfied | Serving composes classification, hydration, ranking and native operations; orchestration does not own their meanings |
| FP-04 Domain model/authority | satisfied | Consequential distinctions and governing operations are specified; lowerings and consumers follow them |
| FP-05 Explicit structure | satisfied | Leases, cancellation, availability, reservations and invalidation boundaries are explicit |
| FP-06 Local reasoning | satisfied | Pure classification/native controls need no unrelated service setup; actual store/transport controls cover their own boundaries |

Applicable DP-01–DP-24 are satisfied at target-design maturity by the mechanisms above. DP-10/16/17 particularly support prepared consumers, thin adapters and removal of replaced authorities. Relevant CI-01–CI-13 are satisfied within the consumed serving boundary; upstream extraction and evaluation-system implementation are not recertified.

The companion’s enhancements have concrete source support:

- **F1 is warranted and enabling.** [Prepared::new](../../../crates/lctx-model/src/domain/selection/evaluate.rs) rebuilds C2; `classify` repeatedly filters domain membership/evidence/closure. A narrower explicit inventory and private receipt-admitted owner improve the read boundary while strict replay preserves independent admission. This is more useful than merely moving `Prepared::new` outside the request loop.
- **F2 is warranted and independent.** [structural.rs](../../../crates/cpg-core/src/structural.rs) maintains expected-domain relation membership separately from [CoverageAdmission](../../../crates/lctx-model/src/domain/analysis/expected.rs). A handled/skipped operation can centralize that decision without weakening strict required-input checks.
- **F3 is warranted and bounded.** [local_semantics.rs](../../../crates/cpg-core/src/local_semantics.rs) repeats entry/theory/field loading; [semantic_summaries.rs](../../../crates/cpg-core/src/semantic_summaries.rs) repeatedly constructs consumed inventories and handwritten extras. Resolving one inventory and preserving `(relation, epoch)` is a useful composition improvement. A table-name-only deduplication would be incorrect.

These are the useful non-overhaul opportunities. F2/F3 should not delay activation merely because their broad cleanup is unfinished. Their inclusion in Q0 must follow the actual integrated tree.

Two implementation priorities deserve emphasis without adding scope or gates. Establish N0’s canonical request-scalar-to-atom bridge inventory before widening the native loader. Establish F1’s exact epoch/content-receipt route before declaring the production prepared owner complete. Both priorities are already required by the plans.

## 8. Library fit and integration burden

The selected mechanisms fit their assigned responsibilities.

The current embedding codec stores exact little-endian component bits and hashes those bits ([value.rs](../../../crates/lctx-model/src/domain/embedding/value.rs)). The inspected pgvector 0.4.2 SQLx adapter encodes a decoded `Vector` with its dimension/header and big-endian component representation. That interface is a credible numerical conversion route; it does not supply a canonical bytea-to-vector SQL cast. The proposed artifact resolves this actual seam while keeping canonical context/spec/text/evidence outside the artifact.

This inspection does not establish runtime preservation through the extension. Selected-profile shape, finiteness/norm, signed-zero bits, digest and readback controls remain V0 obligations. Scope stays at the selected 1,024-dimensional exact profile.

The SQLx/PostgreSQL, PyO3 and FastMCP capability skills were consulted, with pins checked against current repository declarations. Their contracts support the proposed effect, bridge and transport routes. They do not qualify this assembled service. In particular, native lifetime retention and final serialized MCP byte accounting remain application responsibilities; FastMCP response truncation cannot establish the hard packet limit.

## 9. Alternatives and tradeoffs

Direct canonical views with broad per-request replay would reduce new preparation code but retain unnecessary producer coupling and repeated work. Preparing broad data once is a viable simpler intermediate, but it does not deliver the intended narrower serving read contract.

The proposed narrow preparation adds inventory/index/admission work. That cost is justified by a real repeated consumer, and the plan preserves a strict route rather than inventing a forgeable trust flag.

Restoring copied bundles/projection tables would reintroduce semantic and lifecycle authority that the accepted cutover removes. A general projection language, dynamic row framework or independent SQL float decoder would add greater machinery without resolving a current need better.

The vector artifact adds publication, admission and cleanup work. Those effects are acceptable because preparation is explicit, the content is narrowly numerical, its identity covers canonical dependencies and cleanup remains with the generation owner. Revisit the choice if the pinned stack supplies a simpler faithfully qualified conversion, or if a new consumer requires a different vector envelope.

## 10. Verification and material uncertainty

| Claim | Evidence/maturity | Remaining settling evidence |
|---|---|---|
| Current selection replay/scans and adapter duplication | Interface-checked by source inspection, 2026-10-01 | Targeted implementation controls and migrated-consumer inventory |
| Canonical lease/receipt foundation | Interface-checked by source inspection, 2026-10-01 | Actual serving-role admission, content/prefix rejection and retirement/loss controls |
| Vector conversion route | Interface-checked shared codec and pgvector adapter | Actual PG18 preparation, numeric readback, corruption, cleanup and read-only startup |
| Native exact-input target | Proposed, grounded in retained primitive-theory and current condition/Entry owners | Explicit canonical links and independent literal/unsupported/context/budget controls |
| Packet/ranking/wire composition | Proposed with retained consumers inspected | Hand-expected ranking cases and actual native/MCP list/call/resource journeys |

`cargo check`, targeted tests, real PG18 probes, `just test-all`, `just hygiene` and library pilots: **not_run** in this review, as assigned. No evidence folder was necessary for the static target judgment.

The largest execution uncertainty is complete metadata preparation within the stated allowance. The plan gives it a legitimate stop-and-redesign trigger; it does not silently prune the universe or equate a larger budget with adequate architecture. Native exact-input links and cross-language cancellation are additional focused implementation risks already assigned to N0/N1 and L0/T0.

No latency, throughput, total RSS, retrieval-quality, live embedding or broad Python semantic claim is established.

## 11. Authority and disposition routes

ADR-0114 remains proposed. D0 should accept or revise it and update its owning architectural sections before production implementation. This review supports accepting its scoped architectural direction.

The parent cutover §8 remains the single current disposition owner for F05, F10, F12, F13 and the C13 remainder. The detailed plan supplies closure routes; acceptance of this review closes none of those implementation obligations.

F1–F3 remain package identifiers, not source-review findings. The companion owns their local completion; the main plan owns assembled Phase 5 acceptance. No additional register, readiness authority or review workflow is required.

## 12. Decision and enclosing limit

| Judgment | Verdict |
|---|---|
| A1 Localize change | satisfied |
| A2 Encode domain meaning explicitly | satisfied |
| A3 Extend through composition | satisfied |

**Accept scoped** for the Proposed Phase 5 target and focused foundation enhancements. Proceed with D0 and ready model/mapping/native work; integrate F2/F3 according to their actual independent scope.

This is architectural target acceptance, not serving release qualification. Phase 5 remains unavailable until implemented and qualified. Q0 requires the assembled serving review and its named functional/hygiene, real-generation, native and MCP evidence. The enclosing Phase 0–4 architecture, broader native semantics and independent performance/quality campaigns remain outside this review’s certification.
