# Phase 2 foundation refinements — design/target review

## 1. Scope, outcome and coverage

**Revise, bounded; Interface-checked, 2026-09-30.** Fresh independent reviewer using the
`design-review` and `design-review-code-intelligence` skills. Standard: core/template 3.1,
code-intelligence profile 1.2 and repository binding from
[standard.toml](../design_principles/standard.toml).

Subject: the user-approved P0–P2 execution plan's foundation refinements before A10/A11,
against the inspected source at `ae8354d`. The baseline includes the model's scope/admission,
attribution, calls, types, support authorization and stage contracts; acquisition, Pyrefly
root/dependency-context emission and assembly are adjacent consumers. The review assesses
the proposed contracts, not completed implementation. Only this review document was written.
No tests, formatting, lints or integrated gates were run.

Expected changes: add a selected example/test root; add call-target/type supporting definitions;
analyze the same module with another provider/context; substitute a provider revision; add a
vocabulary contributor. P3–P5 semantic interpretation, serving, resource closure and database
operator transition are excluded and retain the active plan's obligations.

Authority: [DESIGN §15](../../design/sections/semantic-model.md), ADR-0085/0086/0087/0089 and
[the cutover plan](../../plans/semantic-model-cutover-plan_2026-09-29.md). The plan owns current
finding dispositions; this review is dated evidence, not another status register.

## 2. Responsibilities, dependencies and semantic ownership

| Owner | Meaning and governing operation | Consumers and expected change |
|---|---|---|
| Model admission | Family requirements, analyzed scope, expected coverage matrix, sealed admission | Producers and publication must consume one ArtifactUse-based root operation; a new selected role changes this owner once. |
| Model calls/supports | Provider callable identity, caller attribution, support authorization | A10 and generated store contracts; a new caller category changes the sum and its invariant owner. |
| Model types | Nominal versus structural terms and transitive native ownership/fidelity | A11 and support validation; new native arms require a mapping and a stated fidelity. |
| Acquisition | Frozen artifacts, roles, ownership and corpus links | Root selection consumes the recorded uses; acquisition does not separately decide coverage. |
| Provider adapter | Native evidence, transient handles, exact model correspondence | Provider upgrades change mapping and conformance controls, not shared scope semantics. |
| Assembly/stage runtime | Sole shared-vocabulary writer, declared dependencies and contribution merge | Providers contribute bounded subsets; later providers must not read terminal finalized vocabulary. |
| Build identity | Complete source/configuration dependency fingerprint | Provider and stage records; source changes affecting facts alter recorded producer identity. |

| Facts | Provider/fidelity | Coverage/identity | Consumer |
|---|---|---|---|
| Root syntax, declarations, calls and types | Pinned in-process analyzer; native assertions or explicit display-only boundaries | Artifact-scoped roots selected by uses; occurrences and provider-qualified symbols | Admission and later normalization |
| Supporting signatures | Native dependency definition projection required by root facts | Input scope must state the bounded projection, not full dependency analysis | Root resolution and nominal references |
| NotRequested flow | No invocation or provider assertion | Scope/family plus compilation context, provider absent | Catalog inspection and facts admission |
| Provider caller | Provider attribution, separate from semantic occurrence owner | Qualified sum; no pseudo-symbol in target/signature namespace | A10 and later normalization |

## 3. Contracts, constraints and testing boundaries

The scope operation consumes InputRevision, SourceArtifact and ArtifactUse plus validated corpus
membership where cross-input uses apply. It returns deterministic roots and family scopes; both
producer selection and exact admission use it. Preserve ownership classification independently:
an unselected captured dependency is still captured and classified.

ProviderCoverage.provider becomes optional with `None` exactly for NotRequested. That state has
no invocation or failure reason. Attempted states still require owned invocation/provider/context
and shared validation. The change must reach syntax's incomplete-coverage index and admission
keys, not merely serialization.

The typed caller sum is the right separation. Its module-body arm needs explicit attribution
coordinates (F01), and the sum must participate in generated assertion subjects, shared support
authorization and source ownership. Symbols remain actual provider symbols. Keep allocated
SymbolKind codes 6–8 reserved and rejected for current facts; never reuse or renumber them.

Nominal callable/overload fields must be checked by the transitive TypeIndex walk, not only
foreign-key existence (F02). Existing opaque-child DisplayOnly enforcement remains necessary.

## 4. Composition and execution

| Stage/operation | Semantic inputs and outputs | Mechanism/policy and publication | Limits/determinism |
|---|---|---|---|
| Scope derivation | Recorded roles → analyzed roots and family scopes | Pure model operation; shared by provider selection and sealed admission | Stable identity order; unsupported uses refused |
| Provider mapping | Frozen bytes/native assertions → typed facts and vocabulary contributions | Native adapter; no independent completeness policy | Existing explicit boundaries and transfer/resource limits |
| Assembly | All declared contributions → deduplicated shared vocabulary | Existing sole-writer runtime; conflicting same-key payload refuses | No downstream provider dependency on terminal vocabulary |
| Admission | Sealed scope/coverage plus execution receipt → FactsAdmission | Existing private proof constructor | Exact row matrix before availability aggregation |

No new projection, graph analytic or synthesis engine is introduced. Coverage derivation is exact
over the declared scope, not a behavioral analysis. Stage scheduling already adds contributor→writer
dependencies and releases output handoffs after their last declared reader
(`stages.rs:129–140,317–327`). Moving vocabulary ownership is sufficient; a second contribution
framework would add machinery without a consumer.

## 5. Change and failure scenarios

| Scenario/kind | Owner and contract propagation | Settling check |
|---|---|---|
| Select a test previously captured only as dependency context / binding | ArtifactUse/model roots; producer and coverage expectations follow mechanically | Selected test gets artifact coverage; unselected dependency does not; role insertion order has no effect |
| A10 adds a referenced dependency target / composition | Adapter extends required supporting-definition projection under the owned contract | Root remains complete only with correctly bounded context coverage and disclosed unresolved targets |
| Same module, second analyzer/configuration / binding | ProviderCallable module-body qualification | Distinct caller identities; foreign caller support refused |
| Named callable term refers to another provider/context / invalid input | TypeIndex native-owner closure | Direct and nested foreign reference refused in memory and PostgreSQL |
| Add ty/doc vocabulary contributions / composition | Assembler vocabulary owner and stage subset declarations | Writer follows all contributors; any contributor reading finalized vocabulary yields a refused cycle |
| Substitute pinned provider revision / mechanism | Adapter, producer fingerprint and native mapping | Changed source/pin changes identity; relocation and source-order permutation do not |

## 6. Correctness and fidelity gates

Verdicts are for the proposal at Interface-checked strength, not runtime qualification.

| Gate | Verdict | Evidence/action |
|---|---|---|
| G1 Authority | unresolved | Shared scope operation is sound direction; supporting-context contract needs F03. |
| G2 Semantic fidelity | unresolved | Caller attribution and nominal terms improve fidelity; F01–F03 remain required refinements. |
| G3 Validity | unresolved | Retarget support authorization and TypeIndex as F01/F02 require. |
| G4 Hidden behavior | pass scoped | Pure scope derivation over captured roles; no new ambient input or acquisition effect. |
| G5 Consistency/recovery | pass scoped | Reuse sealed admission and sole-writer runtime; complete resource/transport recovery excluded. |
| G6 Transformation/reuse | unresolved | Complete code fingerprint is the right contract; coverage semantics and cross-context identity must be settled. |
| G7 Truthful claims | pass scoped | Changes are Proposed; inspection is Interface-checked; tests not claimed. |
| G8 Library leverage | pass scoped | Existing derives, native collectors, Arrow/store/runtime machinery are reused. |
| CI-G1 Fidelity | unresolved | Input-grain completeness and provider ownership require F01–F03. |
| CI-G2 Evidence closure | n.a. | No served claims in this slice; facts support authorization is assessed above. |
| CI-G3 Evaluation integrity | pass scoped | No new evaluation-reference input path; no evaluation performed. |

## 7. Findings and applicability

<a id="F01"></a>
### F01 — Module-body caller identity and authorization need explicit qualification

**Medium; FP-04/05, DP-02/03/04, CI-01 · A2, G2/G3.** The proposed shorthand
`ModuleBody(module)` is insufficient as persistent identity: acquired ProviderModule is
`Acquired{module}` and carries no provider/context (`calls.rs:25–29`), whereas ProviderSymbol
does (`calls.rs:42–46`). Two provider contexts would name the same implicit caller.
Changing ProviderCallSite.caller also removes its existing Subject::Symbol authorization route
(`assertion.rs:314,394–396`). The current CallerCheck alone checks callable kind/source, not
the replacement sum's attribution.

**Correction:** make ModuleBody's key include provider, context and module; validate that symbol,
class-body and decorator arms have the required kinds and owned coordinates. Register the sum as
an assertion Subject with transitive source and native-owner authorization. Preserve occurrence
owner semantics separately. Closure: identity distinction and foreign-provider/context, wrong-kind
and wrong-module refusals through model and stored validation. Disposition: active plan foundation
refinements/A10; extends the obligations of symbols/calls/types review F05.

<a id="F02"></a>
### F02 — Nominal callable references must join the type closure's owner checks

**Medium; FP-04, DP-03/04, CI-01/02 · A2, G2/G3.** Replacing Callable/Overload strings by
ProviderSymbol references fixes name collisions, but the current TypeIndex walk only traverses
their child terms (`types.rs:381–386`). A valid foreign-key symbol can belong to another provider
or context; generated foreign keys do not authorize it.

**Correction:** the owned transitive term-support operation checks Callable/Overload named
references against the supporting run and Function/Method subtype. Anonymous callables retain
structural identity. Preserve opaque descendant fidelity checking and explicit special forms/
variable values. Closure: direct and nested wrong-provider/context/kind controls alongside
same-name/different-module and anonymous-equivalence controls. Disposition: foundation/A11;
extends symbols/calls/types review F06.

<a id="F03"></a>
### F03 — Input-scoped supporting coverage needs a bounded semantic universe

**Medium; FP-04, DP-01/08/11, CI-04 · A2, G1/G2, CI-G1.** The current Signatures requirement
has one artifact grain (`admission.rs:34–47`) while dependency definitions use Input-scoped
qualification and retain only referenced/exported definitions (`pyrefly_stage.rs:320–347`).
The proposed second Input grain must not imply every signature in captured dependencies, nor can
adding one complete Input row excuse missing artifact rows. A10/A11 expand what root facts refer
to; producers otherwise decide the completeness universe privately.

**Correction:** define Input Signatures completeness as the declared supporting-definition
projection needed by analyzed roots, including import/re-export context and nominal references
introduced by calls/types. The model owns required scope construction and the interpretation of
Input versus artifact coverage; exact matrix validation precedes aggregation. Root scope selection
uses recorded ArtifactUse rather than path/acquisition-specific classifiers. Unresolved supporting
targets are explicit, not claimed complete resolution. Closure: selected versus captured-only
dependency controls, complete-empty supporting projection, missing/extra coverage refusals and
A10/A11 reference expansion controls. Disposition: foundation/B1; preserve the user-selected
requested-library analysis boundary.

FP-01/02/03/06 and DP-06/13/16/17 are satisfied for the proposed owner and composition routes.
FP-04/05 and DP-01/02/03/04/08/11 remain unresolved until F01–F03 are incorporated. This does
not waive an in-scope domain-model MUST.

## 8. Library fit and total complexity

Reuse the existing typed derives, nominal IDs, shared support validator, native Pysa collectors,
Arrow codecs, PostgreSQL lowerings and declared stage runtime. This scope needs ordinary domain
operations and adapter correspondences. A universal coverage/provider registry, independent
semantic cache, or replacement analyzer is not justified. The Pyrefly/Ruff skill pin is Pyrefly
1.3.1 with Ruff crates 0.0.13; repository Ruff is 0.0.11, so no new Ruff API transfer is certified.
The inspected existing adapters establish interface routes, not completeness of future A10/A11.

## 9. Alternatives and tradeoffs

| Alternative | Judgment |
|---|---|
| Current extension-based admission and private roots | Refuse: captured dependencies and analyzed roots disagree; changing selection duplicates meaning. |
| Shared model operation with Input supporting projection | Prefer after F03: bounded changes and truthful completeness; requires explicit semantic contract. |
| Fully analyze every dependency | Excluded by the user's selected boundary and unnecessary for supporting context. |
| Keep pseudo-symbols with scattered filters | Inferior to the sum: exposes provider execution artifacts to signatures/targets/entity consumers. |
| Typed caller sum and existing validator machinery | Prefer after F01/F02: separates attribution with bounded model changes and no new execution framework. |

## 10. Verification and uncertainty

All review inspection is **Interface-checked, 2026-09-30**. Focused/PG tests, compile checks,
integrated gates and pilots are **not_run** by this reviewer. Prior reviews are source leads and
dated assessments, not fresh receipts. F01–F03 name the independent controls that could falsify
the proposed contracts. Build fingerprint closure needs implementation inspection plus mutation/
relocation controls; no complete-dependency or determinism test is claimed here.

## 11. Authority changes and dispositions

Amend the owning §15 scope/caller/type meaning through the narrowly scoped ADR called for by
the execution plan. Accepted decisions and implementation status remain distinguishable.
Current finding status belongs only in cutover plan §8; link this review's stable F01–F03 IDs
there. Preserve historical source finding IDs F05/F06 and record closure in their existing rows.
Reserved codebook codes and the clean generation rebuild remain binding.

## 12. Architectural judgment and decision

| Judgment | Verdict | Reason |
|---|---|---|
| A1 Localize change | satisfied at proposed strength | Scope/caller/type semantics have bounded model owners; adapters and lowerings consume those contracts. |
| A2 Encode domain meaning explicitly | unresolved | The chosen concepts are appropriate, but F01–F03 require attribution, transitive authorization and coverage-universe contracts. |
| A3 Extend through composition | satisfied at proposed strength | Existing handoffs, contribution merge and private admission compose without another framework. |

**Bounded decision: Revise.** Incorporate F01–F03 in the foundation implementation before
dependent A10/A11 facts production, with focused model/real-PG controls. No alternative
architecture is requested. This assessment does not certify implementation or the enclosing
P0–P2 architecture. Resource closure, all remaining producers, facts pilots, assembled review and
P3–P5 obligations remain with the active plan.
