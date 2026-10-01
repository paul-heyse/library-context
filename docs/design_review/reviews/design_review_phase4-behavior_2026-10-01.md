# Phase 4 behavioral foundations and finite summaries — design review

## 1. Scope, outcome and coverage

| Field | Value |
|---|---|
| Subject | B0–B3 with normalized admission, guard/vocabulary/store prerequisites and adjacent C1/C2/S0 consumption contracts |
| Baseline | Frozen main `bc6428a`, including Summary `de78f51`, coverage `34dbf1d` and field-screen restoration `bc6428a` |
| Standard | Core/template 3.2; code-intelligence profile 1.3; library-context binding |
| Tier · purpose | Design · target |
| Reviewer · date | Independent delegated design reviewer · 2026-10-01 |
| Method | Read-only inspection of frozen sources, current authorities, retained dormant expectations and identified raw receipts; no reviewer probes or edits |
| Outcome | **Revise**: restore the retained symbolic field-store/reader composition contract and qualify actual Summary publication |
| Excluded qualification | Full Phase 4 F0/X0/Q0 acceptance, actual-library pilots, live embeddings, performance measurements, Phase 5 serving and PR6 evaluation |

The reviewed behavioral architecture has substantial, identifiable semantic ownership. Exact entry proofs, independently established execution evidence, model applicability, finite witnesses, aggregate alternatives and complete-set claims are separate contracts. That is materially stronger than treating a call graph or a final summary table as behavioral authority.

The remaining architectural gap concerns a retained, narrower capability: symbolic association between an admitted unchanged field store and its qualified readers. It is distinct from proving temporal heap value identity. The frozen implementation preserves receiver locations and conservative read screens, but does not yet restore this composition route.

Actual Summary publication is also unqualified. The inspected receipt contains three passing native controls and two failing PostgreSQL controls. The behavioral contribution validator has a statically inspected correction in the frozen baseline. A subsequent closed-reader test declaration correction is visible outside that baseline, but its actual PostgreSQL rerun was still pending when this review closed.

**Evidence strength:** source conclusions below are **Implemented / Interface-checked, 2026-10-01** unless a specific dated receipt establishes **Tested**. No **Measured** claim is made.

## 2. Responsibilities, dependencies and semantic ownership

| Owner | Responsibility and hidden decisions | Consumer contract |
|---|---|---|
| Normalized binding/dispatch | Complete event alternatives, binding variants, source/effective distinction and private invocation/composition admission | Consumers receive replayed authority; counts and producer booleans cannot manufacture it |
| Entry and stability | Prove an exact access reads a parameter entry value; determine eligible binding-stable predicates | Use, Value and Guard proof domains remain distinct; entry identity does not prove mutable state |
| Local | Native value contributions, guards, finite type reasoning and receiver locations | Qualified local evidence and explicit refusal outcomes |
| BaseEvaluation/BaseCompletion | Independently entered expression and ordered statement evidence, outcomes, cleanup and source read inventory | Evidence under entry is separate from invocation or caller continuation |
| SourceCall/EnrichedExecution | Fresh binding/default/body premises and later expression/completion enrichment | Source execution cannot establish its own earlier prerequisites |
| Model | Early pinned model applicability, channel rules, independent invocation actions, normal postconditions and context lifecycle | Authored applicability does not substitute for independently replayed execution |
| Summary | Finite witness construction, SCC work, residuals, aggregate publication and typed claims | Finite-path positives do not establish complete alternative sets |
| Obligation | Priority, admissible discharge and five behavioral verdicts | Producers and consumers share the same operation |
| PostgreSQL/stage runtime | Immutable completed outputs, source receipts, read grants, vocabulary prefixes and coherent publication | A reader has authority only for acknowledged declared inputs |
| C1/C2/S0 | Catalog associations, selection domains and programmatic synthesis | Consume precise evidence under its original meaning; no uncertainty upgrade |

The implemented dependency direction is:

```text
facts → normalized admission
             ↓
Local → BaseEvaluation → BaseCompletion → SourceCall
                                      → EnrichedExecution → Model → Summary
                                                                   ↓
                                                    final S0 consumption
```

Summary proofs use Local/Model alternatives or earlier finite witness occurrences. Final Summary aggregates are publication results, not recursive proof premises.

The domain model governs these operations rather than merely naming their output rows. The principal adequacy exception is P4B3-F01: field locations and universal heap-state boundaries do not represent all retained symbolic store/reader distinctions.

### Fact and fidelity table

| Relation family | Provider/source and fidelity | Coverage and unknowns | Identity and consumers |
|---|---|---|---|
| Flow uses/values/reaching/regions | Pinned native provider assertions and exact assertion/support pairs | Provider/context-qualified coverage; rebinding, nested and loop-carried reaches can refuse entry proof | Nominal native observations; Entry, Local and execution consumers |
| Binding attempts/admission | Derived normalized authority retaining native event/variant premises | Incompatible, open, unavailable and unsupported members remain distinguishable | Event, target, variant and attempt identities; SourceCall, Model, Summary |
| Local/Model transfers | Derived qualified evidence under their owning methods | Captured state, unsupported projections and runtime applicability remain explicit | Owner, places, context, kind and qualification; Summary |
| Finite Summary witnesses | Exact finite derivations under bounded policy | Separate depth/work/proof residuals | Witness occurrence and earlier nominal premises; aggregate publication and claims |
| Aggregate alternatives | Owned OR of finite members | OR refusal preserves exact members plus a separate boundary | Transfer key and aggregate qualification; downstream consumers |
| Call-closure claims | Complete-member question over actual binding attempts | Open siblings and incomplete membership block complete discharge | Exact event, ports, qualification and member inventory |
| Field/formal negative screens | Derived native source-model assessments | Missing coverage, dynamic access, incomplete hierarchy and execution uncertainty prevent negatives | Exact invocation and source universe; not temporal heap proofs |
| Catalog field links | Source candidates and earlier receiver-location evidence | Currently uniformly unavailable heap state | Field/access/location references; C2 and future S0 consumption |

## 3. Contracts, constraints and testing boundaries

**Entry and guards.** `conditions/entry.rs` reconstructs owner, formal, access, provider run, complete reaching membership and the selected execution-domain source. Value and Guard sources retain their own region premises. `conditions/stability.rs:8–35` restricts binding-only stability to the retained identity predicates and requires a Guard entry source. A valid Use proof or arbitrary Value proof cannot establish guard substitution.

`CheckedGuardBinding` and substitution replay retain exact normalized binding, event, source actual, source atom and actual place. The influence/selection relationship remains separate from value transfer.

**Execution.** Source binding, source-body completion and caller continuation are different propositions. Ordered completion records retain abrupt outcomes and cleanup. Model invocation actions and normal postconditions cite different evidence sources (`model_production.rs:29–36, 81–87`). Context execution consumes early protocol contracts, independently entered body proofs and explicit outcome transitions; later Model output is not a prerequisite for its own execution.

**Finite proofs.** `summary_proof.rs:19–23` recomputes depth, step and rank equations from earlier proof occurrences. The global derivation validator checks cross-family cycles (`derivation.rs:89–115`). A malformed proof does not become a budget boundary.

**Discharge.** `summary_consequences.rs:45–53` requires the exact owning invocation and matching coverage before admitting discharge. A false finite condition requires explicit complete native Flow membership and cannot mint positive discharge (`80–91`).

**Publication.** `semantic_summaries.rs:21–37` constructs admitted invocations from captured sources, assesses actual coverage, finalizes discharge and publishes nominal records. `summary_replay.rs:35–52` reconstructs the expected complete Summary inventory from immutable predecessors and stored topology. Coupled deletion of an invocation and its output cannot simply shrink the native frame universe.

These pure operations permit focused semantic tests. Actual ownership/read-grant/publication behavior additionally requires disposable PostgreSQL. Whole replay protects persisted realization, but agreement with the producer alone is not independent evidence of domain adequacy.

## 4. Composition and execution

| Question/stage | Universe and method | Qualification, limits and evidence |
|---|---|---|
| Local entry/value/guard | Exact native source/support pairs in an input/context | Shared entry/stability operations; explicit refusals and retained reservations |
| Base/source/enriched execution | Independently admitted expression, statement and call frames | Ordered outcomes, defaults, body and release premises; no circular source-call proof |
| Models/context | Selected digest-bound authored catalog and captured dependency context | Exact applicability, action phase, independent normal call and resource/value identity |
| SCC schedule | Stored conservative CallableInvocation projection, including known parallel relationships and isolates | Borrowed petgraph SCC computation; canonical callee-first order; topology completeness is not inferred |
| Summary work | Origin-qualified semantic states and admissible cost representatives | Conditions remain part of work-state equality; named depth/proof/work/member bounds |
| Aggregate publication | Closed component’s finite witness members grouped by transfer key | Owned OR; exact members retained if bounded aggregation refuses |
| Consequences | Finite alternatives and actual call-attempt inventories | Finite positive, complete-set closure and refutation remain separate propositions |
| Negative reads | Admitted native source universe, lexical/dynamic inspection and coverage | CompleteNoReadUnderModel cannot follow from empty output; no heap-state inference |
| S0 boundary | Exact Summary invocation/outcome joined to catalog/structural/analytic parents | Parent identity is implemented; final behavioral finding consumption remains unqualified |

Decisive Summary implementation points:

- `summary_production.rs:87` excludes witness identity, arrival order and cost from semantic state while retaining origin, ports, kind and canonical condition.
- `summary_worklist.rs:34–48` retains nondominated representatives. An exhausted redundant derivation does not erase an existing admissible state.
- `summary_production.rs:153–160` gives residuals stable component/origin/event/channel/phase/port identities with an Over qualification.
- `summary_production.rs:171–197` publishes owned OR aggregates and retains finite member support.
- `summary_consequences.rs:103–114` reconstructs complete call-member claims from actual attempts and preserves open siblings.

The correction in `composition.rs:1272–1291` is appropriate: a member condition must imply its OR aggregate, not equal it. Exact aggregate union and complete witness membership remain the responsibilities of generic derivation validation and whole Summary replay. Implication alone would be insufficient.

## 5. Change and failure scenarios

| Scenario and kind | Owning change and affected consumers | Assessment |
|---|---|---|
| Add a model using existing channels — instance | Authored declaration/applicability and genuinely new domain behavior; configuration digest and Model consumers follow existing contracts | **Satisfied, Interface-checked:** no second classifier is required |
| Add supported symbolic field-store/reader association — restored domain composition | Local owns store/read premises; Summary owns finite association and uncertainty; C1/C2/S0 consume its precise basis | **Violated:** the frozen route has locations and heap boundaries but no retained composition contract; F01 |
| Replace SCC mechanism — execution mechanism | Preserve invocation universe, direction, canonical component order, opaque graph tokens and reservations | **Satisfied, Interface-checked:** replacement is behind the existing operation; no new semantic graph authority needed |
| Replace BDD engine — execution mechanism | Condition kernel must preserve nominal atom alignment, exact operations, refusals and canonical identity | **Satisfied, Interface-checked:** consumers use Diagram operations, not BDD-local variable indices |
| Recurse through one/two opaque guard sites — composition | Every invocation retains exact guard lineage; finite depths survive; exhaustion becomes a stable residual | **Implemented / focused-Tested:** native controls exercise depth 0/1/2 and repeated invocations |
| Known positive plus unresolved sibling — failure | Finite claim remains positive; complete-set claim remains open | **Implemented / focused-Tested:** actual call members and native sibling controls retain this distinction |
| Remove coverage or add exec/eval/dynamic receiver — failure | Negative screens become Unknown; source inventory cannot shrink through output deletion | **Implemented / focused-Tested:** native and actual PostgreSQL field-screen controls |
| Read completed Summary through an incompletely declared stage — mechanism integration failure | Reader declaration must include referenced/invariant closure | **Unqualified:** frozen test reader refuses; corrective declaration inspected, rerun pending |
| Trace a later behavioral assertion — answering journey | S0 must consume exact subject/qualification/phase/coverage/proof rather than Summary’s overall status | **Proposed/unfinished:** exact parent frame is implemented; final semantic consumption remains S0 work |
| Recollect a changed release/catalog — revision | Fresh capture/configuration identity and self-contained publication | **Interface-checked:** no legacy-ID bridge or incremental repair route introduced in this scope |

No change-cost or performance claim follows from these source walkthroughs.

## 6. Correctness and fidelity gates

| Gate | Verdict | Evidence/action |
|---|---|---|
| G1 Authority | pass in inspected scope | One normalized admission owner, shared condition/obligation operations and one nominal writer per result family |
| G2 Semantic fidelity | **fail for retained restoration scope** | F01 omits required symbolic store/reader meaning; explicit heap uncertainty does not replace it |
| G3 Validity | pass by static inspection; Summary store qualification unresolved | Shared replay and rejection boundaries are implemented; actual Summary rerun must complete |
| G4 Hidden behavior | pass in inspected scope | Pinned captured/model context and pure domain operations; effects remain at generation/runtime boundaries |
| G5 Consistency and recovery | **unresolved at actual Summary boundary** | Limits are explicit; actual Summary publication/closed-reader receipt remains unqualified |
| G6 Transformation and reuse | **fail for restored field composition; otherwise pass statically** | Finite proof/aggregate distinction is correct; F01 requires a supported transformation route |
| G7 Truthful capability claims | pass under this review’s limits | No whole-B3 Tested, heap readiness, serving or performance claim is accepted |
| G8 Library leverage | pass | Existing graph/BDD/store mechanisms reused; specialized finite work/proof semantics justify ordinary domain code |
| CI-G1 Fidelity | pass for presently published conservative meanings | Candidate, exact finite proof, open sibling and negative-under-model distinctions are retained; F01 remains an adequacy gap |
| CI-G2 Evidence closure | **unresolved for final B3→S0 consumption** | Nominal proof closure is implemented; actual Summary replay and final synthesis consumer qualification are pending |
| CI-G3 Evaluation integrity | pass in inspected paths | Native fixture controls challenge production rules; no gold/reference catalog is used as compiler input |

These verdicts are separate. Conservative outputs can satisfy CI-G1 while the supported restoration target fails A2/G2 through missing domain operations.

## 7. Findings and applicability

<a id="P4B3-F01"></a>

### P4B3-F01 — Restore the retained symbolic store-to-reader association without claiming temporal heap identity

**Principles/judgments:** FP-03, FP-04; DP-02, DP-08, DP-24; A2, A3; G2, G6.

**Owner:** Local/Summary behavioral contracts, with C1 association consumption and later C2/S0 lowering.

**Evidence.**

- `local_semantics.rs:75–76` requires an empty-path formal input; `83–96` derives definition outputs and explicitly refuses Field/Global roots.
- `local_fields.rs:25–64` supplies read-shaped receiver locations and field candidates, with mandatory allocation/alias/mutation uncertainty.
- `summary_production.rs:138–151`, `summary_path.rs:40–42` and `summary_alias.rs:14–15` continue proven call values or unrebound locals. They provide no cross-method symbolic field association hop.
- `catalog/evidence/runtime.rs:28–41` publishes receiver-location and constructor candidates with Unknown/HeapFieldStateUnavailable.
- Base field screens retain source locations/sites and complete-negative assessments. That is useful existing evidence, but not the missing store/formal/value-to-reader relationship.

The retained expectation is independently visible in dormant `cpg-core/src/behavior.rs:1088–1165`: an unconditional exact unchanged store, matched to the catalog’s exact formal, receiver field and reader, yields qualified reader alternatives explicitly marked Unknown/ScopeBoundary. The old operation expressly disclaims temporal cross-method value identity.

**Smallest required restoration.**

1. Preserve source stores and their formal/value/receiver/field qualification. Dormant `behavior.rs:283–297` requires plain stores such as `self.name`, `self._prior` and `self._mode` to remain observable while forbidding unsupported cross-method identity.
2. Preserve supported record association and each reader alternative. Dormant `compile.rs:223–229` requires three distinct qualified Unknown alternatives for `RecordHolder.__init__`; the reader condition scope differs from the constructor’s scope.
3. Preserve negative controls. Dormant `compile.rs:235–243` forbids the corresponding depth-two composition for plain `Holder` and forbids positive cross-method conclusions for `ConditionalHolder`.
4. Preserve exact source association and nonleakage. Dormant `catalog.rs:147–160` requires left→read_left and rejects left→read_right/unrelated.
5. Preserve the narrow storage association contract. Dormant `catalog.rs:162–184` admits `DirectOptions` while refusing custom allocation/metaclass, replaced record, descriptor, setter/operator mutation and static-constructor cases.

**Consequence.** Deleting the dormant producer/tests at X0 would retire meaningful retained behavior without a consumer decision. The new types currently describe locations and unavailability, but cannot express and realize the required symbolic association. A blanket Unknown is honest uncertainty, yet insufficient restoration.

**Correction direction.** Define the smallest typed source store/formal/field/reader operation at the owning boundary. Reuse existing normalized identities, native source premises, receiver entry proofs and field-location evidence. Summary should retain the finite symbolic association, each reader alternative and explicit scope/heap uncertainty. C1 should distinguish exact source association from temporal runtime value identity.

Do not introduce a general heap engine, broad alias analysis, compatibility rows, free-form field/group matching or automatic all-field propagation.

**Closure evidence.** Inspect the new authoritative operation and its writer/proof route; run captured-native discriminating twins and actual PostgreSQL publication/replay controls for the obligations above. Remove/corrupt/retarget a store, receiver, reader, field, condition or supporting premise and verify refusal. Establish the adjacent C1/C2 and S0 consumption mapping before deleting its dormant expectations.

**Disposition:** Phase 4 detailed plan §13 owns execution status; §11 retains migration obligations. The coordinator acknowledged this finding for correction. It remains open in this dated assessment until corrective sources and receipts are independently reinspected.

### Applicability verdicts

- **FP-01/FP-02/FP-05/FP-06 satisfied within inspected boundaries:** owners, private admission, lifecycle, failure meanings and pure verification seams are identifiable.
- **FP-03/FP-04 violated for retained field composition:** F01 is a domain operation/composition gap.
- **DP-01/03/04/05/07/09/11/12/18/19/20/21 and CI-01–06/08–10 satisfied statically for inspected implemented routes**, subject to the stated actual-publication qualification.
- **DP-22/23 and CI-11 remain qualification-sensitive:** the review does not promote pending store/synthesis evidence to Tested.
- No applicable SHOULD deviation or speculative framework requirement is used to compensate for F01.

## 8. Library fit and total complexity

| Capability | Selected mechanism and comparison | Judgment |
|---|---|---|
| Invocation components | Pinned petgraph 0.8.3 over borrowed stored topology; canonical domain ordering afterward | Appropriate reuse; rebuilding an independent graph would add identity and lifecycle burden |
| Condition algebra | Pinned biodivine-lib-bdd 0.6.3 behind the owned Diagram kernel | Appropriate for bounded Boolean conditions; substitution and finite theory remain explicit domain operations |
| Finite evidence iteration | Ordinary deterministic worklist plus typed Pareto cost representatives | Appropriate specialized code; graph reachability or a generic Datalog engine does not supply witness occurrence, condition lineage and residual policy automatically |
| Persistence/transport | Existing PostgreSQL generation store and DataFusion source-bound reads | Appropriate reuse; no second behavioral store required |
| Symbolic field association | Small typed operation using existing premises | Prefer this bounded correction over a general heap/alias framework or duplicated catalog-side inference |

Pins were checked against `docs/pins.md`; the graph/reasoning skills supplied bounded alternative context. This review makes no new library API or performance claim.

## 9. Alternatives and tradeoffs

| Alternative | Meaning, propagation and operational consequence | Decision |
|---|---|---|
| Frozen baseline | Strong finite call-proof architecture, but universal heap boundaries omit the retained symbolic field association | Revise through F01 |
| Minimal typed symbolic association | Adds source store/formal/field/reader distinctions once; retains Unknown for temporal claims; existing consumers derive from the owned operation | Preferred correction |
| General heap-state/alias analysis | Could address broader temporal behavior, but changes the model substantially and introduces unsupported lifecycle/alias obligations | Outside the required correction |
| Catalog-only name matching | Superficially cheap, but duplicates behavioral interpretation and leaks between fields/readers | Reject for retained exact association |
| Drop all field composition as unsupported | Honest if accompanied by an explicit consumer-based retirement decision, but conflicts with the authorized restoration target | Not an accepted disposition |
| Replace the finite worklist with generic graph/Datalog machinery | Does not remove the need for complete member, condition lineage, witness-cost and residual contracts | No demonstrated advantage in this scope |

## 10. Verification and uncertainty

No commands were executed by this reviewer. The following are inspected, attributed receipts dated **2026-10-01**.

| Scope/command | Outcome and evidence |
|---|---|
| Wrapped release tests `-p cpg-extract --test finite_summaries --test read_channels --test summary_consequences`, within the root’s combined run | **passed:** three native controls in `/tmp/phase4-summary-publication8.log` |
| Wrapped release `-p cpg-core --test summary_publication`, same combined run | **failed:** both actual PostgreSQL controls in `/tmp/phase4-summary-publication8.log` |
| Behavioral Summary failure | Contribution validator required member qualification equality with an OR aggregate; frozen `bc6428a` contains the inspected implication correction |
| Catalog Summary failure | Closed-reader probe refused its incomplete declared closure; subsequent test-declaration correction inspected, actual rerun **not_run/pending receipt** at review close |
| Runtime integration: real Model/C1/C2/Structural publication | **passed:** seven PostgreSQL controls in `/tmp/phase4-runtime-integration5.log` |
| Wrapped release `-p cpg-extract --test read_channels` | **passed:** three native controls in `/tmp/phase4-field-read-qualified-native.log` |
| Wrapped release `-p cpg-core --test base_execution` with field-screen filters | **passed:** two actual PostgreSQL controls in `/tmp/phase4-field-read-qualified-pg.log` |
| Full Phase 4 integrated gate, pilots, live embeddings and measurements | **not_run:** F0/X0/Q0 incomplete or expressly outside authorized qualification |

The native finite-summary test exercises repeated opaque invocations at depths 0/1/2, source-return continuation, unrebound aliases, no-base recursion, rebinding refusal, proof-cost forgery and open siblings. The field controls challenge missing coverage, hierarchy, dynamic access and coupled output removal.

These receipts do not qualify the full model universe, actual libraries, whole B3, temporal heap state or serving. Whole replay validates stored realization; retained dormant answers and discriminating native controls are still needed to challenge shared semantic errors.

## 11. Authority changes and dispositions

| Required action | Owner and route | Closure |
|---|---|---|
| Correct F01 | Model/Local/Summary/C1 owners; Phase 4 plan §13 disposition and §11 inventory | Independent reinspection plus native/real-store controls |
| Complete Summary publication qualification | Coordinator; actual producer and closed-reader test declaration | Passing rerun with failed logs retained as composite evidence |
| Map behavior to final synthesis | S0 owner | Preserve exact claim subject, condition, proof, coverage and uncertainty; an overall Partial status is insufficient |
| Retire dormant source/tests | X0 owner after functional restoration and mapping | Every removed field/expectation mapped, mechanically derived or explicitly retired by consumer decision |

No ADR change is automatically required merely to implement the accepted bounded association. If the correction changes supported semantics or chooses a different scope, route that decision through the existing ADR/design owner before treating it as closure.

Neither an accepted decision nor deleting a producer closes an implementation obligation.

## 12. Architectural judgment and decision

| Judgment | Verdict | Reason |
|---|---|---|
| A1 Localize change | **satisfied in inspected scope** | Model additions, SCC substitution and condition changes have identifiable owners and bounded consumers |
| A2 Encode domain meaning explicitly | **violated for retained restoration scope** | F01 omits the symbolic store/formal/field/reader operation; location records plus blanket heap uncertainty are insufficient |
| A3 Extend through composition | **violated for retained field composition; otherwise satisfied statically** | Finite call proofs compose through explicit contracts, but the retained field-reader hop has no authoritative construction route |

**Bounded decision: Revise.** Correct P4B3-F01 and independently inspect its realization. Obtain actual Summary publication/replay receipts after the inspected corrections. The rest of the examined B0–B3 architecture has credible semantic owners, composition boundaries and truthful uncertainty.

**Enclosing architecture:** Phase 4 remains unfinished and unqualified. This review does not certify F0/X0/Q0, final S0 consumption, Phase 5 serving or product/performance results.

The coordinator’s next step is the bounded symbolic association correction and Summary qualification. Retain dormant field, compile and behavior expectations until their native authorities and consumer routes are demonstrated.
