# Phase 3 N1–N5 normalization: assembled implementation review

## 1. Scope, outcome and coverage

| Field | Assessment |
|---|---|
| Subject | N1–N5 implementation in `lctx-model::domain::normalized`, the sole binder, `cpg-core::normalize`, adjacent L0 producer contracts and P4 admission boundary |
| Revision | N1–N4 commits `82a805f` through `33cbfcc`; N5 and review corrections in the shared implementation worktree, 2026-09-30 |
| Standard | Core/template 3.2, code-intelligence profile 1.3 and library-context binding through `standard.toml` |
| Tier · purpose | design · target; scheduled assembled checkpoint before N6 |
| Reviewer | Independent `design-reviewer` agent, 2026-09-30 |
| Target authority | Accepted ADR-0101/0102, DESIGN §15 and the [detailed plan](../../plans/semantic-model-phase3-detailed-plan_2026-09-30.md) §§3–5/§11 |
| Supported implementation | Normalized entities/correspondence/owners/exposures; total relationship outcomes; effective callable/signature assessments; complete events and five policy views; stored bindings and private composition admission |
| Exclusions | N6 projection implementation, N7 normalized frontier/CLI publication and final coverage assembly, X final deletion, Q integrated/pilot/phase qualification; P4 analysis/catalog and P5 serving |
| Evidence | Independent source/type/test inspection. Author-run focused receipts are attributed; no new runtime probe or product gate was run by this reviewer. |

The functional journeys are preserving uncertainty during feature discovery, keeping source
contracts available under unknown wrappers, identifying supported invocation alternatives, and
preparing sound admitted inputs for later analysis. Passing facts transport or a test corpus does
not establish retrieval quality or completed behavioral answers. Unrelated dirty agent/settings/
AGENTS changes were preserved.

Two material findings were reported during inspection: F01 on insufficient qualification becoming
established normalized authority, and F02 on the lower premises trusted by private binding-token verification.
Both corrections were independently reinspected on 2026-09-30; the author supplied passing
focused regression receipts. This review **accepts N1–N5 within this checkpoint** and does not
grant a Phase 3 exit.

## 2. Responsibilities, dependencies and semantic ownership

| Owner | Responsibility and consumer contract | Hidden mechanism / reason for change |
|---|---|---|
| L0 producers and typed facts | Attributed native declaration, signature, lexical, body, call and type observations | Provider-private APIs remain in extraction; `FunctionOrigin` now distinguishes synthesis from callable fields |
| `entity_normalization` | Declaration-based entities, total symbol correspondence, exact occurrence owners, parameter/field links and public exposures | Charged identity/grouping indices; meaning must follow qualified premises |
| `relation_normalization` | Total lexical/import/ancestry/mention/type/binder/place/operand outcomes | Original observation/candidate links survive; type-variable anchors use native module coordinates |
| `callable_normalization` | Separate identity/signature/descriptor/body knowledge and ordered variants/slots | Exact builtin recognition consumes normalized lexical evidence; arbitrary wrappers stay unknown |
| `event_normalization` | Whole-event assessment and single five-policy evaluator | Origin/context/channel/phase stay explicit; raw policy route is removed |
| `signature_applicability`, `calls::bind`, `binding_normalization` | Normalized applicability, one argument-shape algorithm, total stored attempts/variant sets and private P4 admission | Raw evidence stays unchanged; shape success, effective authority and body admission are separate |
| `cpg-core::normalize` | Read declared completed inputs, call pure semantic operations and emit typed outputs | Source/scan admission and resource ownership remain runtime/store concerns |

The dependency direction is mechanism → typed meaning. The inventories derive collector,
validator and stage input/output wiring; they do not duplicate policy classifiers. Normalization
policy revision and the model's captured semantic source digest identify the result-affecting
definitions. No legacy-ID translation or parallel writable semantic store was introduced.

### Fact and fidelity table

| Family | Origin/fidelity | Coverage and unknowns | Identity / consumers |
|---|---|---|---|
| Entities and symbol correspondence | Derived from provider-qualified L0 declarations or explicit native origin | Missing/conflicting/tentative correspondences remain distinguishable; establishment requires sufficient qualified evidence (F01 corrected) | Exact declaration occurrences; qualified external/synthetic symbols; N2–N5 |
| Lexical/import/ancestry/mention links | Derived links retaining raw observation/candidate IDs | Total outcomes; MRO recovery is not promoted; mention remains mention | Relationship identity and source/context grain; callables, later catalog/analytics |
| Type/binder/place/operand links | Structural matching over existing L0 vocabulary | Native coordinates, unsupported origins, missing operands and coverage reasons survive | Provider TypeTerms and P2 Places are retained; P4 derives behavior later |
| Effective callables/signatures | Bounded model operation over exact native/syntax/lexical premises | Independent knowledge components; unavailable variants and source-known/effective-unknown survive | Callable/context/decorator chain, raw signature/slot IDs; binding and later catalog |
| Events/policies | Derived from complete attributed resolution members | Open/orphan/unresolved/dispatch/disagreement states; Potential only Association | Site/origin/context and target/resolution relationship IDs; generated views |
| Bindings/admission | Model applicability plus sole shape algorithm, stored and replayed | Bound, ProvenIncompatible and Undetermined; failed variants are not dropped | Full event/alternative/signature/syntax premise set; private P4 composition token |

## 3. Contracts, constraints and testing boundaries

| Contract | Enforcement inspected | Failure/uncertainty | Isolated boundary |
|---|---|---|---|
| Correspondence | Nominal rows, total outcome, candidate/support links, shared exact-output invariant | Singleton candidate requires supported Definite/Exact/unconditional establishment; tentative evidence remains available | Pure N1 qualifier twins; native cross-provider/source/stub controls |
| Occurrence ownership | Existing `OwnerTable` over source-local ancestor-closed occurrences | Structural gaps refuse; headers/defaults do not acquire body ownership | Independent scalar `owner_of` oracle |
| Effective surface | Exact lexical builtin target, native trait agreement, syntax coverage, independent body exclusions | Shadowed/stacked/arbitrary decorators, uncertain qualifiers and missing bodies refuse effective/body claims | Pure and native callable controls |
| Event closure | Reconstruct each `CallResolution` member digest; retain site/resolution/target supports and orphans | Incomplete direct phases, unknown receiver or dispatch cannot mint complete Summary admission | Pure policy truth tables and native Pysa shapes |
| Binding shape | Complete actual digest/formals; private normalized applicability; one binder | Unsupported unpacking or unavailable form is Undetermined, not Python rejection | Pure binder controls without store/acquisition |
| Composition admission | Binding set, Summary membership, exact target/context, effective authority, positive body admission and exact N1–N4 replay | Forged lower correspondence/owner/lexical premises refuse even when rebuilt upper layers bind | Pure replay plus native tamper/unknown-variant twins |
| Stored stage execution | Completed-source permissions/receipts, source-bound query wrapper, charged `Rows`, typed sink and shared invariants | Resource refusal is explicit; reader drop retains close/drain reservation | Real disposable PG profile tests |

Meaningful implementation details include default **slot** selection rather than a claimed runtime
value, receiver insertion once, retention of original provider IDs across applicable normalized
signatures, and constructor phase identity. Public row construction is not itself an admission
proof; the private-token boundary must enforce its complete preconditions.

## 4. Composition and execution

| Stage/question | Inputs → output and owning mechanism | Limits/model/evidence |
|---|---|---|
| N1: what object/owner does evidence identify? | Qualified declarations/native origins + occurrence structure → nominal entities, resolutions, owners and exposures | Direct attributed relationships; no spelling equivalence or source/stub merge; candidate presence alone is insufficient |
| N2: how do observations relate? | L0 + N1 → total reference/import/ancestry/mention/type/binder/place/operand links | Preserve all candidates and original coordinates; no transfer or guard conclusion |
| N3: which callable contract is established? | N1/N2 + syntax/native traits/bodies/signatures → effective assessments, variants and slots | Bounded supported builtin descriptors; body exclusions and unknown wrappers explicit |
| N4: which calls count? | Entire event's direct/higher-order phases and evidence → one assessment plus five policy memberships | Exact over supplied admitted observations; no runtime reachability or expanded override claim |
| N5: which complete invocation variant binds? | Normalized alternative/signature applicability + complete raw actuals/formals → stored attempts, members, total sets and verified tokens | One Bound plus any Undetermined is not unique; source-only success and excluded body cannot compose |
| Driver/store | Sequential admitted table streams → charged typed collections → pure operation → bounded typed sink | Completed-store reads avoid cross-stage handoff retention, but complete stage inputs/outputs are presently materialized (O1) |

DataFusion and PostgreSQL policy views share `CallPolicy::select_sql`, which selects stored
membership by policy code rather than independently interpreting semantics. Real-store controls
exercise the three-scan view plan. No graph projection/analytic implementation is accepted here;
N6 remains the owner of universe, multiplicity and graph-index obligations.

## 5. Change and failure scenarios

| Scenario / kind | Expected owner and affected consumers | Inspected result / settling evidence |
|---|---|---|
| Add another supported symbol from a provider — binding | Native integration emits qualified facts; N1 correspondence operation applies unchanged | Existing source occurrence permits correspondence; bundled/synthetic identities retain provider scope |
| Candidate declaration shares an anchor — uncertain evidence | Correspondence owner preserves uncertainty into event/applicability | F01 corrected: singleton evidence remains a candidate, while modality/approximation/condition twins refuse establishment |
| Add a policy over current concepts — policy | One evaluator/category; membership renderings follow | No second SQL predicate; independent policy truth tables remain separate from generated parity |
| New decorator behavior — domain extension | Typed evidence and callable authority, then applicability/body consumers | Current arbitrary wrappers remain unknown; no pilot recognizer or reparsing supplies execution authority |
| Bind cross-provider equivalent signatures — composition | Private applicability over established same-entity/input/context proofs | Original raw IDs are retained; shape algorithm is reused; unknown correspondence must refuse |
| Forge N1 owner/correspondence or N2 builtin and rebuild higher results — invalid premises | Private token verification establishes the lower closure | F02 corrected: exact N1–N4 replay rejects lower mutations after higher layers have been rebuilt and still bind |
| Add unsupported overload beside one Bound variant — partial evidence | Total variant-set assessment | Existing controls require non-unique result, retain attempted variants and refuse composition |
| Replace read mechanism — infrastructure | Completed-source/availability contract and runtime adapter | Pure model data/operations stay store-independent; source-bound reader validation remains necessary |
| Missing/incorrect operand type — incomplete observation | N2 exact occurrence/context/role owner | Native controls retain missing outcomes and reject a tampered link through shared validation |
| Scale corpus or replay a large closure — resources | Runtime/model reservations and Q qualification | Explicit refusal exists; full input/output/replay residency is unmeasured at pilot scale (O1) |

## 6. Correctness and fidelity gates

| Gate | Verdict | Independent evidence / action |
|---|---|---|
| G1 Authority | pass | One model operation per normalized meaning, sole binder, generated membership-only views and shared validators |
| G2 Semantic fidelity | pass scoped | F01 corrected: uncertain candidates remain explicit without established identity; native evidence and refusal distinctions survive |
| G3 Validity | pass scoped | F02 corrected: private composition minting establishes N1–N4 derived premises and replays exact bindings |
| G4 Hidden behavior | pass | Pure semantic operations; declared read/store effects; no evaluation references, runtime fixture execution or reparsing |
| G5 Consistency and recovery | pass scoped | Existing completed-stage protocol and charged failure paths used; N7 publication/coverage and whole-phase qualification excluded |
| G6 Transformation and reuse | pass scoped | Corrected correspondence/admission preserve qualifier and proof scope; existing semantic operations compose without a second proof algorithm |
| G7 Truthful claims | pass scoped | Focused implementation receipts and unsupported outcomes distinguished; normalized frontier/product/scale not claimed |
| G8 Library leverage | pass scoped | Standard ordered collections support owned pure semantic operations, existing DataFusion/store/codecs handle boundaries; O1 limits performance claims |
| CI-G1 Fidelity | pass scoped | Qualified uncertainty cannot establish correspondence; full derived-premise replay precedes private body-composition admission |
| CI-G2 Evidence closure | n.a. to serving | P5 served claims are excluded; same-generation references and support retention inspected as prerequisites |
| CI-G3 Evaluation integrity | pass | Independent Python shapes/structural controls are tests; gold and sealed references do not enter production paths |

## 7. Findings and applicability

<a id="F01"></a>

**F01 — insufficient qualification became established normalized authority (corrected and reinspected).**
[N1 correspondence](../../../crates/lctx-model/src/domain/normalized/entity_normalization.rs)
originally checked declaration context and syntax kind, grouped candidate entities and produced
Resolved/DeclarationAgreement for one candidate. It did not require the
`SymbolDeclaration` qualification to be Definite, Exact or unconditional. The L0 declaration
validator also permits qualified uncertain links. Downstream event uniqueness treats the resolved
entity as established, and cross-provider applicability accepts DeclarationAgreement. Exact
target/signature rows can therefore acquire effective admission through a tentative declaration
link. Native synthesis/class-origin premises need the same sufficiency discipline.
Follow-up inspection also found that N3's context-wide callable components must require
unconditional premises: modality/approximation alone do not justify dropping a condition, and
each independently reported knowledge component must apply that rule.

Principles: FP-04/FP-05, DP-02/DP-03/DP-08, CI-01/CI-02; A2, G2/G3/G6, CI-G1.
Owner: N1 model correspondence and N3 callable assessment; correct the supported-proof criterion while retaining tentative
candidate evidence, and require downstream exact admission to use the corrected result. Closure:
Candidate/unknown-approximation/conditional links remain non-established, conflicting candidates
survive, exact supported correspondence still works, and existing native/PG controls pass.
Current disposition is owned by [plan §11](../../plans/semantic-model-phase3-detailed-plan_2026-09-30.md#11-current-package-status-and-finding-routes).

Reinspection, 2026-09-30: `certain` requires Definite, Exact and `Diagram::always` for declaration
and eligible native-origin premises. Declaration establishment also requires support. A separate
charged established-entity set controls resolution; every candidate and premise survives, with
append-only `QualifiedUncertainty` for an unestablished singleton. Pure controls vary modality,
approximation and condition independently against an exact positive. Author-run native and real
PostgreSQL reruns passed. N3's exact-premise helper now also requires an unconditional condition;
signature knowledge applies the requirement before both source and external/synthetic branches,
and asynchronous metadata requires exact syntax premises. Additional pure controls cover
conditional body/signature evidence and Candidate/conditional async syntax. Conditions attached
to call alternatives remain attached to their original evidence; this correction addresses
context-wide authority, without claiming unconditional runtime execution. **Closure accepted for
N1–N5**; final phase qualification remains Q.

<a id="F02"></a>

**F02 — private binding-token verification did not establish lower derived premises (corrected and reinspected).**
The initial [binding verification](../../../crates/lctx-model/src/domain/normalized/binding_normalization.rs)
replays N3 callable and N4 event outputs, but accepts N1 `SymbolEntityResolution`/
`OccurrenceOwnership` and N2 reference assessments/candidates from publicly constructible
`BindingData`. Its inventory initially lacks the declarations/supports needed to establish those
lower results. A caller can forge correspondence or builtin/owner premises, rebuild N3–N5, and
receive a `CompositionAdmission` whose upper replays agree. The normal store stage validates its
predecessors, but this public pure/P4 token boundary requires no validated-store capability.

Principles: FP-02/FP-04/FP-05, DP-03/DP-08/DP-15, CI-01/CI-02; A2/A3, G3/G6, CI-G1.
Owner: normalized verification/admission. Reuse the existing N1/N2 exact-output operations to
establish lower closure before token minting, or require an unforgeable validated source context.
Do not add caller booleans or another semantic algorithm. Make the standalone CompleteEvent API's
precondition explicit or restrict it to the validated path. Closure: adversarial lower-result
mutations with rebuilt higher outputs are refused; honest native/pure/store cases still pass;
all additional retained state remains charged. Current disposition is in plan §11.

Reinspection, 2026-09-30: `BindingData` now carries the lower closure. `verify_upstream` invokes
the existing N1, N2, N3 and N4 operations in order and compares exact outputs before admission;
nominal `Source<R>` access and each owner's inventory drive replay copying. Charged intermediate
closures are released by layer. `CompleteEvent`, `VerifiedEvents` and their verifier are confined
to the normalized module; public event validation returns unit. `CompositionAdmission` retains
the complete event member digest. The new native control changes correspondence reason, owner,
lexical reason or declaration support, rebuilds N3–N5, establishes that shape binding still
succeeds, and then requires full verification to reject. All five native binding controls passed
on the corrected tree. **Closure accepted for N1–N5**; retained replay residency remains O1/Q.

**O1 — bounded materialization remains a scale qualification obligation.** The driver streams
`SELECT *` into complete `Rows` collections, then holds complete pure outputs; replay introduces
additional retained closures. This avoids retained cross-stage Arrow handoffs but does not mean
streaming joins or bounded total RSS. The existing explicit budget/refusal contract is meaningful,
and pure model operations have an independent-test consumer. Q must measure stage and replay
peaks before any memory/performance superiority claim; optimize concrete excessive residency or
generic relational work using existing DataFusion facilities when that evidence warrants it.

FP-01–FP-06 are satisfied within the inspected ownership, proof and pure testing boundaries
after F01/F02 correction. DP-01–24 are supported within this checkpoint, subject to O1's
unmeasured resource qualification. CI-01–04/10/12 are supported for the inspected transformations;
CI-05/07/08 projection/analytics and CI-06/09 behavioral/heuristic claims remain P4/N6 scope,
not findings inferred merely from unimplemented later layers.

## 8. Library fit and total complexity

| Capability | Current choice and inspected fit | Tradeoff |
|---|---|---|
| Semantic grouping/matching | Standard ordered collections through charged model helpers; typed operation per concept | Deterministic, pure and testable; complete collection residency and repeated indexing need Q measurements |
| Query/read/registration | Existing DataFusion 55.1/Arrow 59 family and source-bound PostgreSQL provider | Used for transport, schema, policy-view planning and scan admission; no claim that current normalization joins run in DataFusion |
| Schema/serialization/store | Existing domain derives/codec, SQLx, COPY and generated store contracts | No manual table schema or ORM duplication; snapshots/final migration acceptance still Q |
| Call semantics | Existing sole binder and explicit model applicability/policy operations | A generic inference framework or Python execution would not replace these meanings |
| Graph/reasoning | None added by N1–N5 | Correct to defer topology/fixed points to named N6/P4 consumers |

Library pins were not changed for N5. The review uses existing inspected pinned interfaces and
actual consumers, not latest-documentation syntax or catalog counts. No new library or generic
framework is warranted to fix F01/F02; existing semantic validators should compose.

## 9. Alternatives and tradeoffs

| Alternative | Authority and locality | Judgment |
|---|---|---|
| Raw provider-symbol equality in binder | Simple but rejects established normalized cross-provider correspondence or tempts evidence rewriting | Correctly removed in favor of private normalized applicability |
| Separate policy SQL/Rust classifiers | Easy local implementations but two definitions of what counts as a call | Correctly replaced by stored memberships and shared SQL lowering |
| Upper-only replay | Smaller input closure but assumes unvalidated lower derived facts | F02; insufficient for the public private-token promise |
| Full lower replay using existing model operations | Larger charged closure, but one semantic definition and pure test boundary | Appropriate correction; Q must measure resulting residency |
| Capability-gated validated source context | Could avoid repeated semantic reconstruction | Viable alternative if a concrete store/P4 consumer owns the proof lifetime; must not become a caller assertion |
| Move every domain decision into SQL | Uses relational engine but burdens ownership/testing and often recreates typed policy logic | Not required; use engine primitives where they improve actual relational workload while preserving model authority |

## 10. Verification and uncertainty

Author-run receipts below were supplied during this review on 2026-09-30. This reviewer
independently inspected the relevant source and test changes; these are focused composite
receipts, not a reviewer-run integrated gate.

| Command/scope | Outcome and boundary |
|---|---|
| `python3 scripts/build_environment.py -- cargo check -p cpg-core` | passed after production corrections; author receipt before final test-only additions |
| `python3 scripts/build_environment.py -- cargo test --release -p cpg-extract --test normalized_bindings` | passed: 5 native controls after correction; source/default/wrapper/overload/tamper/resource and rebuilt-higher-layer adversarial cases |
| Wrapped release `lctx-model --test domain_calls --test domain_composition` | passed: 16 and 11 before review corrections; retained composition fixture scope repaired |
| Corrected combined command below | passed after correction: 2 real PG profile controls through N5, expanded lower closure, policy-view membership and retained-budget checks; native bindings 4 / entities 4 / relationships 7; pure normalization 4 / sites 11 |
| Final callable-qualification command below | passed after all production corrections: real PG profiles 2; native bindings 5 / callables 4 / relationships 7; pure effective callables 6, including conditional body/signature and uncertain async metadata controls |
| Earlier N1–N4 focused suites | Historical current-day receipts in plan §11; not rerun by reviewer |
| `just fmt`, lints, `just test-all`, normalized/facts pilots and product evaluation | not_run at this checkpoint; integration waits for all P3 functional scope |

Corrected combined command (the shared `normalized_relations` test selector runs the native
relationship suite as well as the core PostgreSQL suite):

```sh
python3 scripts/build_environment.py -- cargo test --release -p lctx-model --test domain_normalized --test domain_sites -p cpg-extract --test normalized_bindings --test normalized_entities -p cpg-core --test normalized_relations
```

Final callable-qualification command:

```sh
python3 scripts/build_environment.py -- cargo test --release -p lctx-model --test domain_effective_callables -p cpg-extract --test normalized_callables --test normalized_bindings -p cpg-core --test normalized_relations
```

No new probe was required to identify the concrete source-visible gaps. The meaningful
regressions are independently authored uncertain correspondence and forged lower-premise twins,
not another invocation of the same recomputation oracle. Mechanical closure checks remain useful
for missing/extra rows and stored tampering. Scale, normalized publication and phase qualification
remain explicitly unestablished.

## 11. Authority changes and dispositions

F01/F02 are implementation corrections to accepted ADR-0102, not new architectural pivots.
Their stable source IDs and current execution status belong in the detailed plan §11. N1–N5
receipt updates must distinguish corrected focused checks from the eventual Q gate. No unrelated
accepted ADR is edited and no existing cross-phase finding is closed by this review alone.

N6 still owns declared projection realization; N7 owns normalized publication and final scoped
coverage; X retains exact dormant-answer migration/deletion obligations; Q owns assembled phase
qualification. The legacy provider-owner composition engine remains dormant and requires its P4
owner cutover before production use.

## 12. Architectural judgment and decision

| Judgment | Verdict | Reason |
|---|---|---|
| A1 Localize change | satisfied | Coherent model operations, derived wiring and pure controls separate semantic decisions from store mechanics |
| A2 Encode domain meaning explicitly | satisfied within scope | Corrected proof strength governs established correspondence; total outcomes, effective authority and body admission remain distinct |
| A3 Extend through composition | satisfied within scope | Private admission composes existing exact N1–N4 validators and the sole binder; owned inventories derive replay wiring |

**Bounded decision: Accept N1–N5**, following F01/F02 correction, independent source reinspection
and the attributed focused controls above, 2026-09-30. No blocking finding remains in this scope.
**Enclosing architecture:** no P3 phase exit or P4/P5/product qualification is established.
Proceed to N6 under the existing plan; carry O1 into Q
measurements and keep N7/X/Q obligations separate from this bounded result.
