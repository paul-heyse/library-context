# Phase 4 symbolic association — corrective reinspection

**Draft qualification limit, 2026-10-01:** the architectural correction is accepted within this
scope; final F01 verified closure and disposition are pending the current C1 source-link mutation
probes in the coordinator's functional5 rerun. Earlier C1 publication passes do not establish
those later-added negative controls.

## 1. Scope, outcome and coverage

| Field | Value |
|---|---|
| Subject | Corrective implementation of [P4B3-F01](design_review_phase4-behavior_2026-10-01.md#P4B3-F01): normalized symbolic source admission, exact Local store premises, uncertain Summary reader alternatives, C1 source links and S0 facets. Inspected shared main through `44895db9`; production admission/input corrections are in `051ed5c4` and earlier commits. |
| Standard | Core/template 3.2, code-intelligence profile 1.3, library-context binding, loaded through `design_principles/standard.toml`. |
| Tier · purpose | Change · conformance; independent corrective review inside the accepted Phase 4 architecture. |
| Reviewer · date | Delegated design reviewer, 2026-10-01; source and receipts inspected independently. |
| Outcome | **Accept scoped** for the architectural correction, with final verified F01 closure pending current C1 mutation controls. The missing source-association operation is implemented; bounded native/Summary/C1 publication controls pass as a composite receipt. The original review remains its dated assessment. The [parent cutover plan §8](../../plans/semantic-model-cutover-plan_2026-09-29.md#8-findings-disposition) alone owns current finding disposition. |
| Baseline | The original review found a domain-operation gap: field locations and blanket heap uncertainty did not preserve the retained exact store/formal/field/reader association. The [detailed plan](../../plans/semantic-model-phase4-detailed-plan_2026-09-30.md#133-restart-checkpoint-and-remaining-work) supplies its correction contract. |
| Method | Read-only source/contract inspection, followed by inspection of coordinator/worker release receipts. No reviewer probes or builds were needed. This report is the reviewer's sole file change. Unrelated dirty tooling/catalog work is preserved. |
| Limits | No general heap, allocation, alias, mutation or temporal value identity; no promotion to finite transfer proof. No full Phase 4/Q0 exit, integrated gate, real-library upper-frontier pilot, retrieval/embedding quality, measurements, new Phase 5 serving or deletion authorization. Actual upper-CLI/S0 publication remains unqualified. |

The enduring semantic boundary is explicit in [DESIGN §15](../../design/sections/semantic-model.md): the depth-two marker counts constructor→field→reader **source association**, independently of finite Summary call-path proof depth. It does not establish that a later invocation returns the stored runtime value. Keeping the field named `depth` is acceptable with this owned definition and the matching executable comment in `summary_symbolic.rs:25–26`.

### Ownership, fidelity and governing operations

All source references below are under `crates/lctx-model/src/domain/` unless another crate is named.

| Owner / phenomenon | Governing operation and exact identity | Fidelity, coverage and consumers |
|---|---|---|
| Normalized class/store/reader association | `normalized/symbolic_fields.rs:135–187,494–747,748–964`; `callable_aspects.rs:866,874–930` | One source owner used by both profiles. Contextual `SourceSupport` admits the appropriate native or normalized assertion forms without rewriting their original origin/mode/fidelity. Complete source inventories, exact import/symbol targets, field/default membership and initializer signature membership govern admission. |
| Local unchanged source store | `local_symbolic.rs:34–64,66–120`; production `local_semantics.rs:1423–1449`; replay `local_fields.rs:589–609` | Exact native RHS definition/use plus receiver use, exact callable owner and two shared Entry witnesses. The supported `ParameterWithDefault` Child-0 edge resolves the actual native `Parameter`; it does not invent a formal. No reaching-heap or later-read proof follows. |
| Summary uncertain reader association | `execution/summary_symbolic.rs:31–87`; production `summary_production.rs:1909`; consequences `summary_consequences.rs:1017–1024` | Separate symbolic alternative/claim family; exact native value/support, original reader qualification, sink/transfer/through-call and explicit constructor/reader owners. Actual refusal yields Partial/Unknown, proof=None. It does not enter finite transfer/witness authority. |
| C1 exact source link | `catalog/evidence/symbolic.rs:18–41`; `evidence/build.rs:644,1117` | Joins the same normalized association to exact field and constructor parameter options within one C0 member. `source_association=Known`, `runtime_value=Unknown`. It requires no Summary inference or name-based matching. |
| S0 uncertainty facet | `synthesis/summary.rs:71–87,116,320–343,433–435` | Exact claim/conclusion/reader qualification and actual reason. A symbolic facet has no proof/source authority and cannot emit a finding. Text states that the later returned value remains unresolved. |

The generated initializer parameter is owned by the complete synthesized `__init__` signature, not by a class signature (`symbolic_fields.rs:595–615,730–744,915–938`). Standard-record admission checks exact imported dataclass/field targets, literal options/defaults, source field membership and unsupported allocation/metaclass/descriptor/mutation methods. Plain and conditional stores remain observable even when the supported-record association is refused (`855–868`). This preserves positive source evidence without converting refusal into absence.

Summary resolves the **existing initializer callable body** by the native symbol-declaration identity, with a unique existing synthetic callable fallback (`summary_symbolic.rs:46–55`). Declaration evaluation belongs to the enclosing class; it is not the store's executable owner. The corrected route preserves that distinction.

Three source reader sites legitimately produce four native value observations: argument/Identity, return/Identity, return/Derived, and a distinct through-call return/Derived. Every alternative copies its original value qualification. Constructor and reader owners remain separate even when their qualifications or conditions coincide. In particular, the native opaque argument is unconditional; forcing a different condition would fabricate evidence.

### Relevant change scenario

Adding another supported record field and matching reader is an **instance change**. The normalized owner constructs its exact association and links; Local reuses Entry/Flow store admission, Summary preserves each native value alternative, and C1/S0 lower the typed results. Consumers do not independently decide whether a class is a supported record or infer a relationship from field spelling. The existing left/right nonleakage controls exercise this boundary.

Adding a descriptor, dynamic decorator/default, custom allocation or new storage category is a **domain-contract change** owned by normalized admission; current behavior refuses that association. It would require an explicit admissibility/uncertainty rule and the same replay/negative controls, rather than consumer-side exceptions. A general mechanism substitution is outside this bounded correction, which adds no new engine or storage lifecycle.

## 6. Correctness and fidelity gates

| Gate | Verdict | Evidence and boundary |
|---|---|---|
| G1 Authority | pass | One normalized source operation governs both profiles and all four adjacent consumers; Local/Entry, Summary consequences and native attribution retain their own authority. |
| G2 Semantic fidelity | pass | Exact supported association is restored; plain/conditional and unsafe record forms refuse cross-method association. Original native qualifications and separate reader outcomes remain intact. Source depth is explicitly independent of finite proof depth. |
| G3 Validity | pass | Callable metadata replay compares full outputs from Facts-pinned inputs (`callable_aspects.rs:874–901`). Local re-derives both Entry witnesses. Summary replay reconstructs all outputs from immutable predecessors/topology (`summary_replay.rs:103–130`). Actual corruption controls pass. |
| G4 Hidden behavior | pass | Source inspection does not execute decorators, default factories or fixture code. S0 display wording supplies no evidence authority. |
| G5 Consistency/recovery | pass, scoped | Actual generation-store Summary publication, closed-reader replay, intact/permuted controls and sealing pass for both profiles. This does not qualify the failing cumulative CLI publication path. |
| G6 Transformation/reuse | pass | C1 retains Known source/Unknown runtime; Summary/S0 retain Unknown/no-proof alternatives and original reader qualification. C2's actual probe continues to classify runtime ExactReader/ExactStorage as Unresolved. |
| G7 Truthful claims | pass, scoped | Implemented, bounded Tested and broader not-qualified states are separated. Failed combined commands remain in the receipt; no performance or product-quality claim is made. |
| G8 Library leverage | pass | Specialized source joins reuse native extraction, Entry/condition operations, the existing Summary machinery and generation store. No generic capability is replaced by a new bespoke engine. |
| CI-G1 Fidelity | pass | Original attributed facts are admitted contextually; Unknown is neither absence nor runtime identity. Through-call observations remain separate native observations rather than transferred proofs. |
| CI-G2 Evidence closure | pass, scoped | C1 and Summary close over exact source/native premises; S0 pure controls reject proof, authority, qualification, reason and coverage forgeries. New serving and actual cumulative S0 publication are outside this pass. |
| CI-G3 Evaluation independence | n.a. | No evaluation/scoring claim is reviewed; source fixtures are extraction inputs and no gold capabilities or heldout answers enter production. |

The adjacent admission corrections preserve ownership: `Record::family` explicitly describes native provider-completeness metadata and returns None for derived relations (`record.rs:286–288`). The macro now honors `assertion(derived)` while retaining `Assertion::FAMILY` and nominal derived support (`lctx-model-macros/src/lib.rs:392–399`). Catalog S0 excludes unrequested raw Flow leaf/region reads from its production declaration and reads only declared ControlData inputs (`synthesis/production.rs:54–56`; `cpg-core/src/synthesis.rs:61–72`); stored replay retains its full schema inputs and can see empty Catalog relations. These are admission/input repairs, not permission to strengthen a symbolic claim. Broader post-Facts checkpoint/admission failures remain separate qualification work.

## 7. Findings, applicability and verification

### P4B3-F01 corrective disposition recommendation

**The architectural gap is corrected; final verified closure remains pending current C1 mutation controls, 2026-10-01.** The original FP-03/FP-04, A2/A3 and G2/G6 gap is corrected by the authoritative source operation and its Local→Summary→C1/S0 route. No new material architectural finding is identified in this inspected slice. The coordinator retains F01 open until the current source-link mutation probes pass.

Closure rests on the source evidence above and the following **composite** receipts, executed by the coordinator/workers and independently inspected by this reviewer. They are not a clean initial command pass. The [durable qualification receipt](../evidence/2026-10-01_phase4-qualification/README.md) owns commands, raw logs and enclosing outcomes.

| Focused evidence, all dated 2026-10-01 | Outcome and discrimination |
|---|---|
| `symbolic_fields` target in functional2's combined release run, `/tmp/phase4-resume-functional2.log` | **passed**, 5 native controls. Exact record/plain stores, generated/direct controls, left/right nonleakage, exact import/options, and removal/coupled-corruption of source membership/support/default/signature premises. Earlier native8 was a failed whole suite, not a five-control pass. |
| Functional4: `python3 scripts/build_environment.py -- cargo test --release --no-fail-fast -p cpg-core -p cpg-extract -p lctx-model -p lctx --test symbolic_composition --test summary_publication --test synthesis_refutation --test compile_facts --test domain_admission -- --nocapture` | **passed focused targets:** native `symbolic_composition`, 2 controls (1.67s); actual PG `summary_publication`, both profiles, 2 controls (93.08s). **Failed enclosing command:** separate refutation, upper CLI and broader admission targets remain failed. |
| Native composition in functional4 | Exact Local store and both Entry proofs; five independent RHS/receiver/definition/placement support removals refuse. Three readers retain four native sink/value outcomes with exact qualifications and actual initializer-body ownership. Removing Local stores preserves source alternatives with MissingEvidence. |
| Actual PG Summary in functional4 | Intact/permuted production and publication validation pass. Behavioral replay controls 0–11 pass, including symbolic alternative deletion and constructor-qualification laundering. Four conclusions remain Unknown/proof=None with their exact original reader qualification/reason. |
| `catalog_evidence` in functional3's combined release command documented in the qualification receipt | **passed**, both actual PG profiles. Six reader sites yield twelve legitimate links across declared/native field option forms, each with the same exact native constructor parameter. Runtime remains Unknown; complete generation sealing validates the shared C1 replay. |
| `catalog_selection` target in functional2's combined release run | **passed**, 2 actual PG controls, including the existing C2 runtime ExactReader/ExactStorage Unresolved boundary. The additional source-link deletion/runtime-Known/sibling-parameter corruption cases in `fixtures/catalog_runtime.rs:161–185` were added after that binary compiled: **not_run** in the inspected receipts. Their use of the same exact C1 reconstruction is Interface-checked; no executed corruption claim is made for them. Native source corruption and actual Summary corruption cover the F01 source/reader preservation obligation. |
| `symbolic_summary_synthesis` in that functional3 command | **passed**, 2 pure controls. Same-Artifact distinct-owner/original-reader-qualified uncertainty is retained with no finding; proof/authority/qualification/reason/coverage forgeries refuse. The inspected symbolic S0 operation is unchanged in subsequent admission repairs. |

Functional2 and functional3 failures are retained: receiver wrapper/native-formal mapping, initializer body versus header evaluation, Facts-prefix replay, legitimate C1 option multiplicity and the erroneous blanket constructor/reader-condition inequality were corrected. The final guard controls now test exact native qualification rather than manufactured inequality. Functional4's remaining failures do **not** establish full Phase 4 qualification, nor do they erase its decisive native and actual Summary passes. Actual CLI publication of four symbolic S0 facets remains unqualified.

FP-01–FP-06 are **satisfied for this source-association scenario**: coherent ownership, hidden admission decisions, composition, an adequate realized model, explicit lifecycle/uncertainty and proportionate machinery. Applicable DP-01–05/07–09/11–15/18–24 and CI-01–08/10–11 are satisfied at the inspected contracts and stated evidence strength. Heuristic analytics, scoring, retrieval and performance rules are outside this slice. No SHOULD deviation compensates for an in-scope MUST gap.

The original review remains a dated Revise assessment. The coordinator should update only the current parent §8 disposition and its execution/checkpoint links. This reinspection makes no deletion decision: the existing X0 expectation map must preserve these obligations before any authorized retirement.

## 8. Library fit and total complexity

The chosen correction is a small typed operation over existing source/native relations. It composes shared Entry witnesses, the owned Boolean condition kernel, existing finite Summary infrastructure and PostgreSQL publication/replay. A generic heap/alias framework would add allocation, time and mutation decisions outside the required source relationship; catalog-side name matching would duplicate authority and permit field leakage; resurrecting dormant compatibility rows would add a second identity/producer authority. None is needed for this scenario.

This assessment makes no new pinned library API or measured-cost claim. The specialized Python source-admission semantics are not supplied merely by a graph or reasoning library; existing reusable mechanisms already provide the generic work beneath them.

## 12. Architectural judgment and decision

| Judgment | Verdict | Scenario evidence |
|---|---|---|
| A1 Localize change | satisfied | New supported field/reader instances reuse one admission operation and typed consumers; unsupported categories have an explicit owner/refusal boundary. |
| A2 Encode domain meaning explicitly | satisfied | The model distinguishes source store/association, exact Local premises, uncertain native reader alternatives, source-known/runtime-unknown catalog evidence and non-authoritative synthesis facets. Executing operations and shared replay realize those definitions. |
| A3 Extend through composition | satisfied | Existing normalized identities, native Entry/Flow evidence, Summary consequence policy and C1/S0 lowering compose without a parallel store, compatibility ID or general heap engine. |

**Bounded decision: Accept scoped** for the architectural correction, with composite native/actual-store evidence and final F01 verified closure pending current C1 mutation controls. The enclosing Phase 4 architecture and Q0 remain **not accepted by this review**; full integrated gates, cumulative upper-CLI/S0 publication, remaining admission repairs, retirement and assembled review retain their current plan obligations. The next action belongs to the coordinator: complete the current C1 probes, then record the scoped finding disposition and continue those independent obligations.
