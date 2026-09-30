# Phase 4 assembled plan review

**Design / target · Accept scoped · 2026-09-30**

This independently reviews the proposed Phase 4 architecture and execution plan. Acceptance is
at **Proposed** design level. It does not establish Phase 4 implementation, correctness of future
store transitions, live embedding behavior, performance or serving quality.

## 1. Scope, outcome and coverage

| Field | Scope |
|---|---|
| Subject | [Phase 4 detailed plan](../../plans/semantic-model-phase4-detailed-plan_2026-09-30.md), proposed [ADR-0105](../../adr/0105-analysis-vocabulary-epochs.md), [ADR-0106](../../adr/0106-typed-analysis-and-catalog.md), and affected architectural owners |
| Baseline | main 73187700fff553a0ca74abecdcfb7e8429c06617 plus the proposed documents inspected on 2026-09-30; concurrent hook/tooling changes excluded |
| Standard | Core/template 3.2, code-intelligence profile 1.3, repository binding declared by [standard.toml](../design_principles/standard.toml); design-review and design-review-code-intelligence skills |
| Reviewer | Independent assembled-plan reviewer; separate from document author |
| Intended outcome | A decision-complete design from which Phase 4 implementation can proceed after D0 adoption |
| Included | Cumulative vocabulary publication, normalized prerequisites, graph access, finite behavior/proofs, retained analytics, mandatory catalog, pure selection, synthesis, canonical retrieval inputs and P5 handoff |
| Excluded | Production implementation certification; full-library comparisons, pilot/RSS measurements, embedding-service qualification, P5 native/MCP journeys and PR6 evaluation |
| Method | Static document, source and type inspection; targeted dependency and failure walkthroughs. No probes were needed to settle this document-level judgment |
| Execution receipts | Product tests/probes: not_run, no command issued for this documentation review. Automatic hygiene/ADR/docs/catalog checks: not_run by this reviewer; hook-owned, with no anticipated result |

The parent [cutover plan §§4–6](../../plans/semantic-model-cutover-plan_2026-09-29.md)
retains analysis capabilities while permitting their implementation to change. The detailed plan
preserves this obligation instead of treating removal of dormant source as capability retirement.
Phase 3 qualification remains bounded as documented in its existing receipts.

## 2. Responsibilities, dependencies and semantic ownership

| Owner | Contract and reason for change | Downstream direction |
|---|---|---|
| lctx-model | Meaning of invocation, evidence, coverage, transfer, obligation, selection, synthesis and projection; one owned operation per semantic rule | Mechanisms consume types, operations, declarations and validation |
| lctx-analytics | Pure, bounded kernels over admitted semantic inputs | Reads borrowed graph/model inputs; returns typed results |
| cpg-core | Scheduled loading, joins, effect orchestration, kernel invocation and result publication | Depends on model, analytics, storage and embedding client |
| lctx-postgres | Atomic group completion, prefix-bound reads, generated lowering, receipts and generation lifecycle | Enforces model-declared invariants without classifying semantics |
| cpg-extract/cpg-flow | Attributed native evidence in a captured context | Does not impersonate analysis or synthesize future native coverage |
| P5 native/Python serving | Query loading, transport and response assembly over one leased generation | Consumes P4 meaning; no reverse dependency from the model to wire types |

The model is more than a set of result records. The plan assigns governing verbs: derive entry
identity, admit a call, compose an alternative, close coverage, discharge an obligation, classify
a requirement, emit a grounded assertion and realize an embedding value. This addresses A2's
behavioral-authority requirement.

**CI fact and fidelity assessment:**

| Family | Authority and identity | Fidelity/coverage | Consumer |
|---|---|---|---|
| Native facts | Existing provider/run/context and nominal source identities | Native observations remain attributed; Flow NotRequested differs from Partial | Normalization and analysis |
| Dispatch and receiver assessments | Normalized event, raw premises, MRO and coverage | Derived membership and symbolic ClassOf do not manufacture raw provider assertions | Bindings and prebuilt invocation projection |
| Transfers and finite witnesses | Normalized callable ownership, qualified alternatives and evidence occurrences | Exact finite propositions remain distinct from residual uncertainty | Summaries, discharge and behavior |
| Metrics/communities/concepts | Analysis invocation, declared universe and method parameters | Heuristic versus exact-under-context remains explicit | Ranking/grouping and navigation |
| Public catalog/scenarios | Public slots and original artifact/span references | Candidate, unresolved and actually checked evidence remain distinct | Pure selection, retrieval and P5 packets |
| Embedding uses | Shared spec/value semantics, consumer-specific use identities | Exact consumed bytes and availability; not evidence that a requirement holds | Analytic kNN or later retrieval |

The graph remains a materialized computational projection of canonical relations. The plan keeps
the user's full-collection lifecycle and constructs it once after complete normalized inputs.

## 3. Contracts, constraints and testing boundaries

The central new lifecycle is sufficiently specified for implementation design:

- Twelve named vocabulary families have one assembly owner and ordered immutable epochs.
  Ordinary result families retain one writer.
- Producer computation yields sealed delta receipts without read authority. An atomic publication
  group merges vocabulary and loads dependent ordinary rows before shared validation, then records
  completion. Missing or empty planned outputs cannot silently disappear.
- Canonical bases are not importer write targets; growing vocabulary bases are not importer read
  targets. Literal-bound security-barrier views and private source permits enforce prefix visibility.
- Old receipts validate their original prefixes. Generation/prefix reference checks reject a future
  row even if it physically exists in a later base.
- Failed or unconfirmed close poisons the attempt. Final seal rejects unclosed groups and remaining
  ingestion privileges. The existing terminal lifecycle remains the recovery owner.

This is necessary, rather than a speculative multiwriter framework. In
crates/lctx-model/src/domain/stages.rs:450, a vocabulary writer depends on every contributor;
its earlier checkpoint is required by downstream normalization. In
crates/lctx-postgres/src/generations/receipts.rs:31 and :205, current completion freezes whole
outputs and later validation compares whole-table receipts. Reopening those tables or merely
registering late contributors cannot satisfy both contracts.

The plan correctly retains load-then-generated-reference validation rather than claiming existing
FKs are deferrable: crates/lctx-postgres/src/generations/ddl.rs:299 generates ordinary reference
constraints. R0 must prove new prefix isolation, grant refusal and atomic failure against the real
store. That future evidence is an implementation obligation, not a missing design probe.

Composition admission remains private. The existing token at
crates/lctx-model/src/domain/normalized/binding_normalization.rs:513 is the migration target for
the caller-provided booleans at crates/lctx-model/src/domain/composition.rs:55. A successful shape
binding under an unknown wrapper is insufficient; the new domain enumerates unavailable and open
members as well as successes.

## 4. Composition and execution

| Stage/question | Representation and owned semantics | Limits, publication and evidence |
|---|---|---|
| Normalized dispatch | Raw target plus derived member; complete MRO/provider context; known conservative members retained | Runs before graph construction; open-world uncertainty blocks completeness |
| Local behavior | Entry-value witnesses, typed conditions and normalized owner/formal/place identity | Provider/context-qualified reaches; mutation/rebinding twins challenge stability |
| Execution/completion | Base expressions and completion → source-call preparation → enrichment | Acyclic evidence pipeline; a call never supplies its own base body/default premise |
| Recursive composition | Callee-first stored-graph SCCs; qualified semantic state separate from witness cost and aggregate TransferKey | Depth 8/proof-step 64 plus existing versioned work limits; explicit stable residuals |
| Structural/optional analytics | Declared universe, multiplicity, weights and output selector | Shared reservations; optional techniques off; convergence or its absence recorded |
| Catalog and pure selection | Public slots, canonical callable contracts, contextual domain/closure and exact witnesses | Catalog independent of flow/seeds/briefs; whole-conjunction context intersection |
| Synthesis/retrieval | One qualified emitter, original anchors, rendered units and consumed vectors | Same-generation closure; early analytic embedding inputs never depend on late briefs |

The finite recursion choice is coherent. crates/lctx-analytics/src/summaries/worklist.rs:16
already distinguishes nondominated depth/cost representatives and equal-cost evidence; :70
distinguishes an admitted alternative from a limited witness or an open sibling. The proposed
plan preserves that useful contract while explicitly accounting for invocation-distinct guards.

The important separation is between qualified worklist states, condition-independent transfer
aggregation, finite proof occurrences and final publication. The existing global validator at
crates/lctx-model/src/domain/derivation.rs:90 checks conclusion → proof → premise edges.
Having a witness depend on its final aggregate could create a cycle even when the computation
terminates. The proposed witness ranking and post-SCC aggregate publication avoid that structure.

The plan does not promise that all recursive opaque conditions converge exactly. It chooses
bounded exact witnesses plus honest residual uncertainty, retains multiple-lap controls and
forbids widening distinct evaluations into an Exact proposition. An all-alternatives result
requires its own complete membership; a finite positive does not erase an unresolved sibling.

## 5. Change and failure scenarios

| Scenario and change kind | Owning change and propagation | Assessment |
|---|---|---|
| Add a model in existing channels: instance/domain extension | One model definition and actual new semantics; declared context requirements and digest flow into native capture and analysis | Credible local route; no late ambient imports or duplicate Python classifier |
| Add a new analysis: composition | Declare input, capability, method/limits, outputs and kernel; generated storage/receipts follow | No new provider framework or facts-family repurposing |
| Substitute a graph/numeric kernel: mechanism | Preserve universe, direction, multiplicity, weighting, determinism, budgets and diagnostics | Explicit adaptation; library availability alone cannot redefine ranking |
| Add a requirement predicate: domain extension | One predicate/domain/witness/closure contract; P5 wire and loading consume it | Pure classifier remains independently testable without server/database setup |
| Recollect changed source/model: instance | Fresh full generation, corrected dispatch before one graph construction, immutable publication | No reuse-hash authority or incremental repair path |
| Cancel vocabulary close or lose commit acknowledgement: failure | Store-owned poison/abort protocol; no read handle from unconfirmed completion | Clear terminal semantics; R0 real-store controls still required |
| Identical input used for early analytics and late retrieval: composition | One spec/codec/cache-winner operation; distinct one-shot consumption records | Resolves the schedule cycle without a shared mutable artifact relation |
| Cross two hydrated graph views: invalid binding | Generative graph brand must reject mixing tokens before private indices are interpreted | F01 design correction accepted; implementation compile-fail evidence pending |

## 6. Correctness and fidelity gates

These are **document-target** gate results, not runtime test results.

| Gate | Verdict | Evidence and boundary |
|---|---|---|
| G1 Authority | pass | Explicit model owners; canonical facts/projections; derived support avoids fake ProviderRun; separate consumer use records do not redefine vector meaning |
| G2 Semantic fidelity | pass | Unknown/open/residual/check-intent states preserved; raw and derived call evidence separated; F01 corrected at design level |
| G3 Validity | pass | Admission tokens, shared prefix/group validation, branded graph access and adversarial controls have named enforcement points |
| G4 Hidden behavior | pass | Model-context capture precedes providers; embedding is declared effectful; pure kernels/selection do not acquire ambient evidence |
| G5 Consistency and recovery | pass | Atomic epoch groups, private receipts, immutable lower reads, terminal failure and explicit work-bound outcomes |
| G6 Transformation and reuse | pass | Once-built native graph, bounded proof semantics, exact consumed vectors and explicit algorithm conversion policies |
| G7 Truthful capability claims | pass | New design remains Proposed; NotRequested, Partial and lexical-only are explicit; P5/product/performance are excluded |
| G8 Library leverage | pass | Existing library mechanisms retained; specialized worklist/weighted rank/FCA justify their required domain contracts |
| CI-G1 Fidelity | pass | No candidate-to-execution, heuristic-to-proof or unknown-to-absence promotion is authorized |
| CI-G2 Evidence closure | pass for P4 contract | Same-generation evidence roots and proof closure required; actual served hydration remains P5 |
| CI-G3 Evaluation integrity | pass | Independent expected answers move before deletion; gold/held-out data excluded from compiler input |

## 7. Findings and applicability

<a id="F01"></a>

### F01 — Graph token lifetime did not identify its graph

The initial §4.3 described opaque GraphNode/GraphEdge tokens carrying a borrow lifetime, but a
shared lifetime does not distinguish tokens from two concurrently borrowed graph objects.
Passing one graph's token to another graph's mapping/access method could reinterpret a private
index under the wrong projection. This is an FP-02/FP-05, DP-02/DP-03/DP-08, A2 and G2/G3 contract
gap; the existing MaterializedGraph API keeps those indices private
(crates/lctx-model/src/domain/projection/snapshot.rs:31 and :125).

**Correction inspected, 2026-09-30:** §4.3 now selects callback-based with_native_graph access
with a generative invariant brand separate from the graph borrow lifetime. Branded tokens cannot
escape or cross views; externally retained results are converted to EntityRef/ArcId within the
callback. This closes the ambiguity at **Proposed** design level without a new dependency.

**Owner and closure:** lctx-model projection access, package G0. Implementation must establish
the selected trait surface and compile-fail wrong-graph/escape controls before claiming enforcement.
Current execution disposition belongs in the parent
[cutover findings](../../plans/semantic-model-cutover-plan_2026-09-29.md#8-findings-disposition).
No production correction or test is claimed by this review.

The applicable foundations are satisfied for the target: FP-01/FP-06 separate semantic operations
from orchestration; FP-02 gives meaningful admission/publication/algorithm contracts; FP-03 provides
an acyclic composition route; FP-04 has both concepts and governing operations; FP-05 names
invalid-state rejection and lifecycle ownership. DP-01–05, 07–12, 18–24 are addressed by the
contracts and controls above. DP-06 and 13–17 are satisfied by generated lowerings, existing
libraries and bounded modules; no new universal rule/provider framework is required.
CI-01–13 are applicable within the P4/P5 boundary and supported as specified in §§2–6.

## 8. Library fit and total complexity

| Capability | Inspected fit | Choice and burden |
|---|---|---|
| Native graph | Cargo.toml:36–37 pins petgraph 0.8.3 serde-1 and Postcard 1.1.3 alloc; snapshot.rs:21–37 serializes the actual compact Graph with nominal weights | Borrow existing topology; no StableGraph mutation requirement, duplicate graph registry or persisted index identity |
| SCC and bounded recursion | Existing worklist separates semantic progress/cost; model derivation validator uses petgraph | Library topology plus domain-specific proof/residual operations; a generic Datalog engine would not remove those semantics |
| Ranking | ranking.rs:43–101 defines distinct-site fractional direct usage; its Params at :181 records weighted iteration tolerance/limit | Preserve weighting and diagnostics rather than selecting an unqualified similarly named routine |
| Communities | communities.rs imports GraphDataBuilder/Leiden and explicitly builds weighted inputs; Cargo.toml:45 disables default features | Thin charged conversion with declared layer semantics, not an ArcId-as-weight shortcut |
| Evidence/selection | selection.rs:38 and :203 own classify/joint; current wire dependency is visible at :3 | Move pure meaning into the model, leave transport in P5; no new solver/framework |
| Embedding realization | embedding.rs:11–47 owns exact f32 little-endian codec/digest; use records distinguish consumer purposes | Shared value operation and immutable service winner, with early/late one-shot canonical consumption records |

This review inspected these existing boundaries and the proposed alternatives; it did not
independently certify every upstream API listed by the plan's broader library survey. Pin changes
and first implementation of new visit traits remain interface-check triggers. No new generic
replacement was needed to make the architecture coherent.

## 9. Alternatives and tradeoffs

Delaying all vocabulary completion is the smallest storage change but leaves the scheduler cycle.
Splitting each phase into independent condition/place authorities replaces a lifecycle problem
with repeated translations and semantic decisions. Arbitrarily reopening tables breaks previous
receipts. The bounded epoch mechanism is justified by a present consumer, with additional
view/delta/receipt complexity contained in one store owner.

Unbounded recursive opaque conditions cannot be made a finite exact domain merely by choosing
Ascent/datafrog. A finite domain worklist preserves current exact witnesses and reports the
remaining uncertainty. This trades recursive completeness for a declared, inspectable result;
it is not described as an optimization preserving unbounded completeness.

Porting dormant tables unchanged preserves implementation shape but restores duplicate catalog,
identity and evidence rules. The selected typed migration removes those authorities while
retaining independent expectations. It does not force every algorithm into a declarative engine.

## 10. Verification and uncertainty

| Claim | Evidence on 2026-09-30 | Remaining evidence |
|---|---|---|
| Late vocabulary creates a real ordering/receipt problem | Interface-checked: stages.rs writer dependencies and receipts.rs freeze/verification paths | R0 scheduler, permission, prefix, reference and terminal failure controls |
| Base/enriched execution can be acyclic | Interface-checked: lctx-analytics/src/execution.rs:1–53 explicitly prepares base completion before source-call enrichment | B1 migrated independent semantic controls |
| Recursive proof DAG can remain finite and non-self-referential | Proposed design walkthrough against derivation.rs and worklist.rs | B3 bounded recursion, sibling, corruption and shuffle controls |
| Catalog can remain independent of behavior and briefs | Proposed, with legacy catalog.rs:474 inputs and selection.rs contracts inspected | C0–C2 no-flow/no-seed controls and exact source/effective twins |
| Embedding schedule can avoid late-input cycles | Proposed early analytic and late retrieval consumption design; existing codec inspected | E1/E0 real value admission, cache winner and service-free replay controls |
| F01 invalid graph-token mixing is excluded by the selected API | Proposed generative callback contract inspected | G0 trait/API realization and compile-fail controls |
| End-to-end Phase 4/5 capability or performance | Not established | Implementation packages/Q0, then P5; excluded pilot measurements need separate activation |

No test was added merely to mirror prose. The plan's independent controls target substantive
failure modes and preserve existing external/hand-expected answers before migration.

## 11. Authority changes and dispositions

ADR-0105 explicitly proposes replacing the affected one-shot completion clauses of accepted
ADR-0101; current implemented semantics remain in force until adoption and migration. Its
unaffected cumulative-generation, resource and terminal-state decisions must survive.

ADR-0106 explicitly proposes bounded recursive evidence and conditional operator review.
Accepted ADR-0005 still contains the unconditional manual-review target; the plan correctly
routes its replacement through D0 rather than pretending an existing workflow satisfies it.
Programmatic synthesis, grounding and heuristic restrictions survive the proposed change.

The owner sections distinguish current Phase 3 implementation, dormant legacy recovery semantics
and Proposed Phase 4 changes. Parent §8 owns implementation finding disposition; detailed packages
own execution order and closure controls. Accepting this document closes no historical
implementation finding.

## 12. Architectural judgment and decision

| Judgment | Verdict | Reason |
|---|---|---|
| A1 Localize change | satisfied at Proposed level | Models, analyses, predicates and kernel substitutions have bounded owners and explicit consumer contracts |
| A2 Encode domain meaning explicitly | satisfied at Proposed level | Coverage, invocation, proof, residual, catalog, selection and realization concepts govern named operations; F01 ambiguity corrected |
| A3 Extend through composition | satisfied at Proposed level | Epoch publication, normalized admission, pure kernels, classification and realization compose without reversed serving dependencies |

**Bounded decision: Accept scoped.** The assembled Phase 4 target is sufficiently specified to
proceed through D0 adoption and dependency-ordered implementation. No unresolved design/fidelity
gap remains in the claimed proposed envelope after F01's clarification.

**Enclosing architecture:** accepted as a Proposed Phase 4 target for the named retained
capabilities and change scenarios. Implementation, runtime lifecycle guarantees, full-library
qualification, P5 serving and product quality remain unqualified. An implementation review must
reassess actual governing paths and the required independent controls, especially R0/R1 and B3.

The next step is D0 authority adoption and preservation of independent expectations, followed by
the vocabulary-publication prerequisite. No production Phase 4 work was performed by this review.

