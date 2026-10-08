# Target and implementation alignment review

Command excerpts identify verification scope; original invocations remain in the cited logs and Git history.

**Principal Design/Target review · 2026-10-04 · Core/template 3.2 · Code-intelligence 1.3**

**Decision: Revise.** The implemented architecture supports a substantial part of the API and evidence target through coherent, executable domain owners. It warrants incremental correction, not replacement. The most consequential newly identified defect is a provider mapping that converts an unattached native **Bound** candidate into **Unbound** reaching evidence. Incomplete-inventory labels and Entry safeguards prevent the inspected path from establishing a false behavioral conclusion, but they do not preserve the meaning of the published raw origin.

Three independent architectural causes also remain: repeated stage-publication and grant definitions impede extensions; identity and physical-lowering contracts contain incidental representation dependencies; and some current documentation overstates or misdescribes implementation responsibilities. Serving needs explicit requested-library domain resolution and generation identification in capability resource text.

The previous [library-leverage review](design_review_library-leverage_2026-10-04.md) remains useful, but several of its arguments and remedies need correction. There is no demonstrated Summary epoch collapse, DataFusion already has model-owned membership joins, faithful `Debug` text does not by itself prove a semantic-model violation, and analyzer views must preserve legitimate differences rather than be forced to agree. These refinements do not remove the concrete extension, contract and fidelity obligations identified below.

## 1. Scope, outcome and coverage

| Field | Assessment |
|---|---|
| Subject | Implemented semantic model and adjacent consumers: declarations, providers, normalization, Entry, Local, execution, Model, Summary, Structural, analytics, catalog/evidence/selection, PostgreSQL publication, native serving, retrieval and MCP presentation; documented API/evidence target, including proposed retrieval qualification and evaluation |
| Baseline | `339cf1a5de80001e65cb07280562f637c8986817`, inspected 2026-10-04 |
| Concurrent work | Dirty `docs/library-utilization.jsonl` excluded. No production edits, builds, probes, store operations, commits or publication performed by this reviewer |
| Governing standard | `docs/design_review/design_principles/standard.toml`: core principles/template 3.2, code-intelligence principles/review additions 1.3, library-context binding |
| Tier and purpose | **Design / Target**: assess the best available incremental architecture for the functional target, independently of earlier scoped acceptance |
| Method | Static inspection of governing documents, executable declarations, decisive production paths and adjacent consumers; focused reports used as leads and reconciled against source |
| Outcome enabled | Disposition and subsequent planning of incremental corrections. This is not production remediation or authorization to resume stopped qualification |
| Explicit exclusions | Line-by-line audit, exhaustive library resurvey, gold and heldout contents, fresh runtime qualification, current-latest release/security research, performance and retrieval-quality measurement |
| Maturity | Existing implementation is **Implemented** within the inspected paths. Historical **Tested** receipts retain their recorded revisions, dates and cases. Comparative product qualification remains **Proposed** |

The architecture map, DESIGN §B decisions, [semantic-model owner](../../design/sections/semantic-model.md) and [API/evidence owner](../../design/sections/api-and-evidence-product.md) establish the intended division of responsibilities. The [cutover plan](../../plans/semantic-model-cutover-plan_2026-09-29.md), [Phase 5 plan](../../plans/semantic-model-phase5-detailed-plan_2026-10-01.md), [incremental alignment coordinator](../../plans/semantic-model-incremental-alignment-plan_2026-10-02.md) and [analytical enrichment coordinator](../../plans/code-facts-analytical-enrichment-plan_2026-10-04.md) retain their separate implementation and qualification boundaries.

The current source has pending Named schema migration, matching artifact/adaptor refresh and full functional/non-functional gates. Static review neither supersedes those obligations nor turns predecessor receipts into qualification of this tree.

## 2. Responsibilities, dependencies and governing models

The central design is adequate for much of the target because it models operations as well as stored records. Normalization owns interpretation of provider assertions; Entry owns admissible value witnesses; execution and Summary own finite composition; Structural owns qualified associations; selection owns eligibility and joint applicability; publication owns immutable visibility; serving owns bounded hydration and presentation.

| Component | Responsibility and hidden decisions | Contract and dependency direction | Legitimate change |
|---|---|---|---|
| Acquisition and providers | Pinned inputs, analyzer configuration, native observations and exact source attachment | Produce attributed model facts; provider-private handles remain local | Analyzer/release changes and additional observations |
| `lctx-model::domain` | Nominal identity, declarations, codebooks, qualification, invariants and domain operations | Pure operations consume explicit typed inputs; mechanisms consume its contracts | New distinctions, policies, relations and analysis behavior |
| Normalization and Entry | Correspondence, complete event assessment, policy admissions, bindings and native value evidence | Preserve alternatives and refuse unsupported correspondence or incomplete native closure | New provider shapes or supported binding/value cases |
| Local, execution, Model and Summary | Finite models, transfers, composition, assumptions, verdicts and proof | Model-owned operations over immutable inputs and declared projections | Additional finite semantics and composition laws |
| Structural and analytics | Qualified relationships, traversal, grouping and ranking | Declared universes, settings, budgets and result classes; heuristics remain distinct from proof | New analytic or relationship consumer |
| Catalog, evidence and selection | Public identity, signature roles, options, original evidence, requirements and joint applicability | Canonical typed results consumed by packets and retrieval | New API contract/evidence family or predicate |
| Synthesis, retrieval and embedding | Grounded assertions, contextual units, display policy and spec-qualified representations | Derived from canonical evidence; ranking is navigation, not behavioral truth | Rendering, corpus, ranking or embedding-policy change |
| `cpg-core` | Stage coordination, granted reads, decoding, compute invocation and publication adaptation | Effects depend on model contracts and generation-store APIs | Adapter mechanics and orchestration |
| `lctx-postgres` | Physical lowering, grants, acknowledged epochs, receipts, lifecycle and serving reads | Single relational store; semantic validation uses model operations | Physical layout and effect/lifecycle mechanics |
| Native/Python serving | Process generation guard, request admission, hydration, serialization and MCP transport | Rust owns semantic/wire contracts; Python performs transport effects | New packet/route or transport revision |
| Evaluation | Frozen tasks, independent checks, comparable conditions and sealed confirmation | References remain outside compiler inputs and tuning | Protocol activation and additional task strata |

The direction from mechanisms to meaning is substantially realized. The main exceptions are repeated stage contracts and grant construction in adapters, and presentation paths that discard identity or leave a requested scope implicit.

### Fact and fidelity table

All provider pins below refer to the current repository pins and immutable fork revisions, not a claim that they are the newest available releases.

| Family | Provider/fidelity | Coverage and identity | Consumers and assessment |
|---|---|---|---|
| Syntax and lexical facts | Independent Ruff 0.16.10/crates 0.0.16; extracted/native structural assertions | Canonical source occurrences and explicit support; unlocated assertions remain bounded | Normalization, catalog and source evidence. `normalized/native_lexical.rs:21–82` checks provider, run, scope, modality, approximation and evidence |
| Types, signatures and diagnostics | Pyrefly 1.4.0-dev.3 with isolated embedded Ruff 0.0.14; attributed typing assertions | Native role/source correspondence and unavailable shapes are explicit | Callable/catalog/type consumers; typing conclusions remain distinct from runtime behavior |
| Runtime flow | ty 0.0.16 runtime view through `cpg-flow` and `ty_flow` | Candidate inventories retain conditions, unattached state and completeness; mapped target defect in [F01](#F01) | Entry and raw origin serving. Entry safeguards hold; raw origin fidelity does not |
| Normalized calls and bindings | Model-derived correspondence and policy assessment | Alternatives retain identity; complete events precede policy selection | Model, Summary, Structural and API packets |
| Local/Model/Summary | Derived results under finite model, condition and assumption contracts | Native and derived support remain distinguishable; five verdicts and refusals remain explicit | Structural, synthesis and native assessments |
| Structural relationships | Derived, qualified, structurally observed associations | Supporting hop conditions and premise bases retained | Selection and evidence packets; no inspected promotion to unrestricted behavior |
| Communities, PageRank, FCA/RCA and similarity | Exact-under-context or heuristic according to the owned method | Projection/settings/source identity and bounds explicit | Ranking/grouping and finite associations; no inspected heuristic-to-proof substitution |
| Documentation/scenarios/deployment | Original captured evidence plus qualified associations | Release/source context and ambiguous remainder retained | Evidence discovery and bounded implementation packets |
| Retrieval and vectors | Derived units and heuristic ranking | Rendering/text/spec identity distinct from original evidence | Candidate nomination; quality and live-service usefulness remain unmeasured |

The model is broadly adequate, but **Bound with no mapped definition** currently lacks a faithful realization in the reaching relation, and **requested library absent from the admitted input domain** lacks an explicit serving distinction.

## 3. Contracts, constraints and testing boundaries

| Contract | Meaning and enforcement | Effects, failures and substitution |
|---|---|---|
| Typed record construction | Nominal references, append-only codebooks, record validation and exact-schema decoding | Mechanical representations may change; semantic distinctions cannot be inferred from matching primitive encodings |
| Native attachment | Source/role correspondence and provider attribution establish what was observed | Missing correspondence must remain unavailable/unmapped, not become a different native assertion |
| Entry witness | Exact supported evidence, appropriate coverage and complete native singleton closure | `conditions/entry.rs:506`, `:733` and `:864` reject annotation-only, non-Bound and incomplete closure cases |
| Qualification composition | Conditions, scope/context, assumptions, modality and approximation compose through owned operations | Structural conjunction and premise-basis union are explicit; removing a guard requires semantic justification |
| Dependency closure | Exact validator requirements differ from sufficient read grants | Epoch/order requirements remain exact; broader grants cannot redefine the validation universe |
| Stage publication | Declared producer ownership, admitted inputs, invariant validation and acknowledged receipts | Invalid or incomplete outputs refuse publication; visibility is generation-owned |
| Selection | Supported, unresolved and conflicting results remain distinct; joint applicability is separate | Ranking cannot establish eligibility or simultaneous support |
| Native assessment | Original condition, restricted result, verdict, basis, assumptions, proof and limits remain separate | No request-time Python/library execution; resource refusal is observable |
| Request lifecycle | Process generation guard, CPU admission and independently admitted SQL occupancy | Actual blocking work and cleanup retain grants after caller cancellation |
| Retrieval reuse | Corpus rendering, text, embedding use/spec and ranking policy are explicit dependencies | Conservative invalidation is valid; incompatible vectors cannot be silently reused |
| Evaluation | References never become extraction/analysis/synthesis inputs or tuning data | Confirmation criteria and comparable population must be fixed before results |

Pure transformations are testable without acquisition or a store. Real PostgreSQL is necessary for publication, grant, epoch and lifecycle obligations. Repeated publication lists currently make some otherwise local extensions discover omissions only at a store-backed execution boundary; this is an architectural cost, not evidence that all model tests need PostgreSQL.

## 4. Composition and execution

The implementation composes genuinely owned capabilities. It does not merely attach domain names to generic output records.

| Stage/question | Projection, method and semantic class | Settings, budgets and evidence linkage |
|---|---|---|
| Normalization: what did providers assert about this source? | Exact correspondence over captured source occurrences; complete events and alternative identities precede policy views | Unavailable correspondence and open resolution retained; original support linked |
| Entry/Local: what input value evidence is admissible here? | Native per-use inventory, exact conditions and finite value model | Incomplete closure refuses proof; qualifications and source support retained |
| Execution/Model: what follows under this finite model? | Transfer and composition operations, not graph reachability as dataflow | Explicit model/assumptions, bounded conditions, typed outcomes and witnesses |
| Summary: what composes over invocation topology? | Declared invocation graph; SCC schedule and model-owned summary union | `summary_schedule.rs:23–61` uses borrowed topology and canonical SCC ordering; `summary_production.rs:1483` uses `AlternativeUnion` |
| Structural: which qualified associations are established? | Typed paths and relationships over declared source/normalized inputs | `structural/qualifications.rs:43–128` conjoins conditions and unions premise bases; hop conditions remain on paths |
| Analytics: which grouping/ranking/context relations help a named consumer? | Declared universe, relation kinds, multiplicity and weights | Method/settings/budget/outcome recorded; heuristic navigation does not establish behavioral truth |
| Catalog/selection: which public contracts and requirements match? | Typed public exposure, signature roles, evidence associations and contextual classification | One classifier; independent matches do not imply joint applicability |
| Synthesis: which statements can be grounded? | Programmatic assertions over canonical results and checked supports | Qualification and original evidence closure precede publication |
| Retrieval: which units nominate useful candidates? | API/options, documentation/deployment, scenario and source families; eligibility separate from ranks | Declared family normalization and winning-unit witnesses; proposed quality protocol remains unexecuted |
| Serving: what can be returned within this request? | Generation-bound hydration and native reconstruction | Complete mandatory signatures, explicit optional omission, response limits, cancellation and guard lifetime |

### Exact requirements are not grants

The previous review correctly found repeated grant construction but did not establish lost vocabulary epochs.

`dependency_closure.rs:1–190` distinguishes ordered exact validator requirements from sufficient grants and widens acknowledged grants deliberately. Summary grants vocabulary at Model (`summary_replay.rs:282–290`), while `SummaryData` preserves separate Facts and Model views (`summary_production.rs:237–245`). `cpg-core/semantic_summaries.rs:49–64` obtains each consumed source separately. Store validation constructs the required physical epoch views (`stage_validation.rs:260–308`, `validation_views.rs`).

Centralization must preserve these distinctions. A single list or name-only deduplication of requirements would make the architecture worse.

### Relational compute is a selective mechanism

`normalized/events.rs:55–64` owns a join selecting **already validated policy memberships**. PostgreSQL installs that query in production (`generations/ddl.rs:415–425`); DataFusion logical-view registration exists in `cpg-core/model_runtime.rs:129–162`, with the inspected consumer in normalized-relations tests.

The wider derivation pipeline largely scans admitted relations into charged typed `Rows` and invokes pure model kernels. Consequently, broad claims that `cpg-core` presently runs normalized derivations and semantic validators through DataFusion need narrowing. The stronger earlier claim that there are no joins is incorrect.

A model-owned relational lowering can preserve semantic ownership and run against in-memory inputs without a store. Conversely, placing every finite domain operation in a query engine can add ordering, decoding, budget and refusal complexity. Both mechanisms remain eligible under their actual contracts.

## 5. Change and failure scenarios

| Scenario and kind | Expected owner/contract | Observed propagation and assessment |
|---|---|---|
| Add a fact family — domain concept | Declaration, provider attribution/coverage and genuinely new semantics | Existing relation-list machinery helps, but provider/stage lists still restate membership. Preserve unavailable coverage and exact attachment |
| Add an analytic — domain concept/composition | Owned inputs, projection, settings, result class and named consumer | Publication-family membership is repeated across model stage outputs and core writers; omission compiles and fails later |
| Add a publication relation — domain concept | One analysis-family declaration | `analysis/mod.rs`, `catalog/build.rs:805–824` and `catalog_core.rs:110–125` independently expand the same family. Locality violation |
| Add a packet — composition | Canonical results plus bounded hydration | Existing typed packets, optional sections and exact evidence operations are suitable. Resource generation loss must not be copied into new renderings |
| Upgrade analyzer — provider change | Adapter absorbs private changes; semantic differences remain attributed | Source/role boundaries are strong. Unattached Bound mapping and unrecorded decision differences expose fidelity gaps |
| New analyzed release — binding/input change | New pinned input/release, current source identity and rebuild | Generation/source/artifact identity supports conservative invalidation. No cross-release compatibility reader is required |
| Empty answer with incomplete or absent input — domain/query policy | Distinguish known empty admitted scope from unavailable requested scope | Per-requirement uncertainty is explicit; library filtering lacks a requested-domain certificate ([F02](#F02)) |
| Trace conditional claim to facts — evidence composition | Original conditions, assumption bases, supports and same generation | Structural conjunction, Summary alternative union and native proof membership support the inspected contract |
| Change invariant — semantic policy | Owner operation changes; production/test enforcement follows | Shared invariants are strong; grant walks and physical derivation copies require repeated changes |
| Change identity or rendering policy — representation/contract | Explicit equivalence level and invalidation boundary | Source/lockfile sensitivity is intentional; `Debug` components and JSON contracts need precise ownership and vectors |
| Cancel or exhaust resources through prepare/execute/serialize — execution mechanism | Retain grants until actual work and cleanup finish; explicit refusal | Request CPU/query paths retain ownership. Startup preparation deserves separate bounded qualification, not automatic use of request deadlines |
| Substitute mechanism preserving refusal/determinism — mechanism | Same universe, multiplicity, evidence, ordering, charged lifetime and outcome classes | Pure kernels and borrowed graph contracts permit bounded substitution; a query-engine or BDD replacement must preserve or deliberately revise refusal policy |
| Evaluation isolation — protocol | Frozen development criteria, sealed confirmation, comparable population and independent judging | §14.12 specifies these responsibilities coherently; comparative execution remains Proposed |

Profile-dependent port exclusions and unions of upstream inputs are legitimate compositions. A future generated port mechanism must express them without collapsing decoder reachability, actual consumption, exact validation requirements and sufficient grants.

`DependencyClosure::merge` currently rejects differing nonempty validator lists (`dependency_closure.rs:150–158`). A future port composition must decide at that owner whether such lists are incompatible policies or conjunctive obligations. Unconditional union is not justified merely because both lists contain validators.

## 6. Independent correctness and fidelity gates

“Pass” below is a bounded static assessment of the inspected contract, not a fresh test result or whole-repository certification.

| Gate | Verdict | Evidence and limits |
|---|---|---|
| **G1 Authority** | **fail, scoped** | Grant traversal is independently authored beside its owner, with no owner-based reconciliation of the sufficient-grant policy. Runtime checks protect missing prerequisites, but do not make the copies mechanically derived. Publication-list and physical-layout comparisons do reconcile their derived outputs; those are primarily locality defects |
| **G2 Semantic fidelity** | **fail** | Unattached native Bound becomes Unbound reaching evidence and a served raw origin ([F01](#F01)). No epoch-collapse finding is established |
| **G3 Validity** | **pass, scoped** | Typed constructors, shared replay/validation and store admission enforce inspected contracts. The inventory validator explicitly permits F01’s mismatch; that is a faulty fidelity contract rather than an unidentified bypass. Local schedule panic remains a typed-rejection correction |
| **G4 Hidden behavior** | **pass, scoped** | Explicit captured provider inputs and effect boundaries; pure model operations inspected. No fresh hermeticity qualification performed |
| **G5 Consistency/recovery** | **pass, scoped** | Immutable publication, original generation guard and actual-work grant retention. F03 concerns resource self-identification, not demonstrated generation mixing |
| **G6 Transformation/reuse** | **pass, scoped** | Exact epoch validation, explicit qualification composition and spec-qualified cache checks. Conservative source-sensitive invalidation is intentional; no unsafe cache reuse demonstrated |
| **G7 Truthful capability claims** | **fail, scoped** | Current prose overstates bulk DataFusion derivation/validator execution and shared skill/project guidance contradicts linked extraction. Historical Tested scopes and Proposed product qualification remain distinct |
| **G8 Library leverage** | **fail, minor** | Cursor hex and docstring quote recognition repeat available generic capabilities without a demonstrated contract reason. This does not require replacing specialized charged domain kernels |
| **CI-G1 Fidelity** | **fail** | F01 reaches published raw results. Entry refuses incomplete native proof, so no false behavioral verdict is claimed |
| **CI-G2 Evidence closure** | **pass, scoped** | Native assessment checks stored path/original-condition/proof membership (`native_service.rs:293–303`); assertion/brief replay and checked capability hydration close inspected claims. Resource generation loss does not establish missing cited facts |
| **CI-G3 Evaluation integrity** | **pass, static/protocol scope** | Explicit input boundaries and §14.12 isolation/comparability protocol inspected. Gold/heldout contents and confirmation execution not examined; no comparative success claimed |

The DP-04 identity contract also needs revision as discussed below. Its representation gap is not evidence that current source-sensitive invalidation is unsound.

## 7. Findings and reassessed recommendations

New findings use this review’s `F01`–`F03`. Earlier findings are always source-qualified as **LL-Fnn** with current disposition now transferred to the [coordinator §7](../../plans/target-implementation-alignment-plan_2026-10-04.md#7-current-disposition--sole-owner-for-transferred-review-obligations).

### New finding index

| ID | Consequence | Principles/judgment | Owner and closure |
|---|---|---|---|
| [F01](#F01) | Native Bound origin is presented as Unbound when definition attachment fails | FP-04/05, DP-02/08, CI-02/04; A2, G2, CI-G1 | Flow provider/mapping and inventory owner; retain unmapped Bound meaning and reject incompatible mapped members |
| [F02](#F02) | Unknown requested library and admitted empty domain lack an explicit serving distinction | FP-04/05, DP-02/08, CI-04; scoped A2 contract gap | Model-owned library-domain resolution and serving consumers |
| [F03](#F03) | Capability resource discards generation identity carried by the canonical response | DP-21, CI-13; presentation/provenance contract gap | Model resource rendering/native adapter; resource identifies snapshot without changing authored body |

<a id="F01"></a>
### F01 — Unattached Bound candidates are mapped and served as Unbound

**Diagnosis: Implemented, source inspected 2026-10-04.**

`cpg-flow/src/lib.rs:759` deliberately does not map definitions in some PEP 695 class/alias type-parameter scopes. At `:991–1001`, a native Defined candidate whose definition cannot be mapped retains kind **Bound**, marks `unattached`, and emits a reaching record with `def_ix: None`.

`cpg-extract/src/ty_flow.rs:1433–1434` translates every such missing definition index into `ReachingDefinition::Unbound`. Candidate inventory construction retains `unattached` (`:1519`), but `lctx-model/src/domain/flow_inventory.rs:462–470` allows target-kind disagreement when that flag is set. PostgreSQL origin hydration then emits `FlowOriginTarget::Unbound` (`flow_inventory_service.rs:335–345`).

The provider’s assertion is “a bound candidate exists, but this adapter has not attached its definition.” It is not “the read is unbound.” Retaining incomplete coverage alongside the relabelled target does not repair the target’s meaning.

**Limits.** Entry requires appropriate supported evidence, an actual Bound target and complete native singleton inventory. These safeguards prevent the inspected unattached case from becoming a proven value witness. This review does not assert a false Established/Refuted behavioral verdict.

**Correction: Proposed.** The simplest correction is to retain the native Bound candidate and its explicit attachment boundary, omit the fabricated mapped reaching member, and use `mapped_count = 0`. If a named consumer needs a reaching row for this state, add an explicit unmapped-definition variant through the model owner. Do not invent a definition or reinterpret missing attachment as runtime unboundness. Delete the inventory exception that permits incompatible candidate/member kinds.

**Challenge.** Legitimate Undefined and Deleted candidates must still map to Unbound. LoopHeader and Nested cases require their own existing rules. The correction must retain conditions, source attribution, incomplete-inventory explanation and candidate cardinality.

**Closure evidence.** Independent controls for a Bound candidate without attachment, a real Undefined/Deleted candidate and a valid attached Bound candidate; validate through model inventory and actual PostgreSQL raw-origin hydration. Existing Entry refusal remains unchanged. This is a meaningful regression test, not a generated expectation copied from production.

**Disposition.** Transferred to coordinator §7, analyzer A1, 2026-10-04. The correction and raw-origin qualification use that owner; the assessment above remains historical source evidence.

<a id="F02"></a>
### F02 — Requested-library domain admission is implicit in empty lookup results

**Diagnosis: Implemented contract gap, source inspected 2026-10-04.**

Serving requests accept a bounded `Name` for library. `catalog_service.rs:271–286` determines member inclusion by matching first-party release/package names; an unknown name simply matches no members. `packet_service.rs:75–78` returns `OperationResolution::Missing` with `Availability::Available`. `catalog_service.rs:639–707` returns available pages and `CompleteDomain { total: 0 }`.

These can be read narrowly as “the catalog service is available and this filter matched nothing.” They are not proven claims that no Python API exists. Nevertheless, the response has no explicit distinction between a known admitted library with no matching member and a requested library absent from the generation’s analyzed input domain. The target promises declared coverage and local uncertainty, and new consumers should not have to infer admission from an empty member list.

**Correction: Proposed.** Resolve the requested library against captured input/release/corpus coverage before member filtering, through one model-owned operation. Return its admitted domain/coverage or an explicit unavailable/unknown-domain outcome. Reuse that result across lookup, browse, selection and evidence search. A typed refusal is the simplest alternative if unknown libraries are outside the route’s supported input contract.

**Challenge.** A known admitted library with no public members must remain a legitimate empty result. Multiple releases or ambiguous distribution names require explicit resolution; do not select an arbitrary release or claim analyzer completeness solely from corpus presence.

**Closure evidence.** Unknown library, admitted empty library, admitted nonempty library and incomplete admitted scope produce distinguishable model and wire outcomes. Clarify the meaning of `CompleteDomain`, `Available` and `Missing` at the owning contract.

**Disposition.** Transferred to coordinator §7, serving S1, 2026-10-04. No false behavioral/nonexistence result is asserted by this finding.

<a id="F03"></a>
### F03 — Capability resource text loses its snapshot identity

**Diagnosis: Implemented, source inspected 2026-10-04.**

`capability_service.rs:66–77` produces `GetCapabilityResponse` with generation. The native resource adapter takes only `response.capability` (`python/lctx_storage/src/serving.rs:977–988`). `CapabilityPacket::resource_text` appends assertions but no generation (`serving/packets.rs:544–555`). The resource URI is `lctx://capability/{capability}` (`dispatch.rs:157`), and Python serves those text bytes directly (`wire.py:255–263`).

A copied or retained resource cannot independently identify the snapshot against which its evidence references resolve. The generation-pinned process still prevents the inspected request from mixing snapshots; this is a provenance loss at presentation, not a demonstrated concurrency defect.

**Correction: Proposed.** Render resource metadata from the generation-bearing response, preserving canonical authored body bytes. Add a compact generation identifier outside that body, and define whether the URI remains process-relative or becomes generation-qualified. The body must retain existing assertion status, qualification and proof references.

**Closure evidence.** Tool and resource forms name the same generation; a standalone resource retains sufficient snapshot identity to resolve its references. Exercise the actual native/Python resource path and final-byte envelope controls.

**Disposition.** Transferred to coordinator §7, serving S2, 2026-10-04. Phase 5 activation remains separately stopped.

### Independent reassessment of LL-F01–LL-F10

| Prior finding | Reassessment and corrected direction |
|---|---|
| **LL-F01: repeated ports/publication sets** | **Diagnosis retained.** The same publication family is independently expanded in model outputs and core writers. Runtime declaration checks reconcile correctness but do not localize change. Start with one finite typed publication-family expansion; consider broader port generation only where it removes current repetition |
| **LL-F02: copied closure** | **Diagnosis retained, severity narrowed.** Repeated policy and Local panic are real. Exact Summary Facts/Model universes are preserved; epoch collapse is not established. Centralize grant construction without flattening exact requirements or decoder inventories |
| **LL-F03: repeated physical layout** | **Diagnosis retained.** `ddl.rs:67–103`, `:235–279`, `:443–451` and `serving_shape.rs:27–43` repeat layout conventions. Introduce one physical-column lowering, while preserving independent live-schema inspection. Codec semantics and store-hidden columns are different responsibilities |
| **LL-F04: identity encoding** | **Contract correction retained, rationale narrowed.** Model identity includes Arrow `DataType` Debug (`model.rs:231–235`) and Sum Debug (`:311`). Raw source, declaration/macro source, paths, manifests and lockfile intentionally participate (`build.rs:20–39`). Preserve conservative artifact invalidation; specify identity purpose/encoding instead of promising semantic stability across source/provider upgrades |
| **LL-F05: Debug rendering** | **Target improvement retained; automatic A2 diagnosis withdrawn.** Debug text is opaque but faithful. Human display, source text and executable expression are legitimate distinct policies. Specify those modes and use them consistently; retrieval benefit remains unmeasured |
| **LL-F06: DataFusion role** | **“No joins” premise withdrawn.** Model-owned membership joins exist. Broad implemented derivation/validator prose still needs correction. Retain pure kernels and selective relational lowerings; do not prohibit future in-memory model-owned DataFusion computation |
| **LL-F07: Rows helpers** | **Opportunity retained, bounded remedy.** A required lookup may accept a typed error constructor so different domain refusal reasons survive. Add charged indexes only for repeated named access patterns. PostgreSQL `Batch` helpers are not the same contract; avoid a universal query abstraction |
| **LL-F08: dropped facts/decisions/type scopes** | **Mixed.** Unconsumed Ruff Export/Branch emission is real fork maintenance cost. Branch context is not a runtime predicate and Export observations are not complete exports authority. Preserve legitimate Pyrefly/runtime differences; record attributed comparisons where consumed. The type-scope question now has concrete F01 evidence |
| **LL-F09: generic duplicates** | **Small corrections retained.** Hex/quote recognition are suitable built-in candidates. Retry eligibility is separate from failure class; `57014` must not become retryable. Unused exports/dependency declarations need consumer/transitive checks before deletion. BDD substitution changes refusal policy and remains conditional |
| **LL-F10: stale text/triggers** | **Diagnosis retained; decision route corrected.** Current-owner clarification or a new superseding decision is appropriate. Accepted ADR substantive bodies cannot be changed by calling an amendment “lifecycle metadata” |

Two identity details materially change the earlier recommendation. Mapping/consumer identities currently have test-only consumers, so their latent metadata-order risk must not be represented as a production cache failure. Conversely, embedding spec JSON is already persisted and checked in PostgreSQL (`cache.rs:36–49`, `:66–70`); the earlier “persisted cache beyond embedding spec” trigger must not leave the existing spec contract unspecified. `embedding/spec.rs` already declares ordered, whitespace-free serialization and has a known canonical-spec control. Review its required numeric/serialization equivalence before selecting JCS or a new codec; adopting a canonicalizer is not automatically an improvement.

### Prior observations and additional opportunities

| Observation/opportunity | Assessment |
|---|---|
| **LL-O1: 64-round enriched loop** | Explicit bounded execution remains preferable to silent completion. A domain-derived bound or named cap reason would improve diagnosability. No inspected false-complete result established |
| **LL-O2: migration advisory lock on error** | Retain the bounded operational lead. The pinned SQLx skill documents the failure behavior; current `MIGRATOR.run(&pool)` remains. Dedicated connection lifecycle is a plausible correction, not a newly executed defect reproduction |
| **LL-O3: transport mirror and newer releases** | Preserve final-byte mirror and parity controls. Earlier latest-version/security assertions are historical leads, not reverified facts. Any upgrade belongs to `pin-check` and actual transport requalification |
| **LL-O4: strengths** | Preserve pure kernels, charge-before-retention, typed refusals, nominal identity/private indices, exact codecs, iterative SCC, read-only SQL, closed pushdown and independent oracles |
| Validator-list composition | Decide incompatibility versus conjunction at `DependencyClosure`; no indiscriminate union or new universal port DSL |
| Presentation modes | Owned display/source/expression modes can improve discovery while preserving special floats, bytes, status and unknowns. A renderer must not pretend every literal has an executable finite Python expression |
| Analyzer decision comparison | Pyrefly prefix equality (`sys_info.rs:752`) and runtime five-field tuple semantics (`predicate.rs:383`) differ for `sys.version_info == (3, 14)`. Preserve view attribution and record consumed disagreements; neither global equality nor automatic provider replacement follows |
| Local coverage scope | More local premise/attachment coverage may aid diagnosis. Existing explicit unavailable/partial input coverage means this is not, by itself, a proven absence bug |
| Startup preparation lifecycle | Separately qualify preparation admission/cancellation and retained artifacts when serving qualification resumes. Request-lifetime safeguards do not establish all startup behavior, but no new startup failure is demonstrated |
| Product evaluation | Keep §14.12 Proposed. Freeze corpus/rendering/ranking identities and the comparable population before confirmation; do not tune against sealed answers |

## 8. Library fit and total complexity

### Retained conditional leads — coordinator reconciliation

The earlier review's remaining alternatives are retained explicitly below. Its
[compute/storage evidence](../evidence/2026-10-04_library-leverage/l1-compute-storage.md) and
[reasoning/graph evidence](../evidence/2026-10-04_library-leverage/l3-reasoning-graphs.md)
provide dated Interface-checked claims and the named historical Tested probes. Those receipts
are not new tests of this tree; their isolated compiler/profile does not qualify production.
The directions below remain Proposed and do not change the principal judgment.

| Lead | Incremental direction and limit |
|---|---|
| Evidence-preserving SCC condensation | Reconsider the blanket exclusion: the earlier petgraph probe distinguishes `make_acyclic=false`, preserving arcs but creating internal self-loops, from the acyclic form. A future consumer must specify loop treatment, lineage, borrowed identity and charged allocation before adoption; merely changing the flag is insufficient |
| Weighted PageRank alternatives | Keep the current stop/work/determinism contract. The earlier graphops comparison warrants a conditional alternative with distinct-neighbor aggregation and allocation accounting, not a direct replacement over parallel arcs. Revisit for a named ranking extension or measured cost |
| FCA/RCA alternatives | Preserve odis as an independent development oracle. Its lazy concept iteration and public preclosure step must not be described as wholly batch-only; they still do not establish the production iceberg/pseudo-intent and resource contract. Revisit only against a specific operation |
| Datalog/fixpoint substitution | Keep the finite Summary/worklist owner for this correction scope. The earlier Ascent probe did not examine every BYODS/custom-relation or parallel composition, so it cannot establish universal impossibility. A substitution proposal must compare pending/refused evidence, deterministic work limits and admission before mutation on the actual shared-channel contract |
| Text search and stemming | Correct the forward-plan adoption lead to respect Rust-owned tokens. A Rust stemmer or PostgreSQL text-search route remains conditional on the frozen development-set comparison and corpus/version contract; no quality improvement is established here |
| DataFusion instrumentation | The earlier dependency evidence refutes OpenTelemetry as the runtime rejection reason for the inspected tracing release. Defer on lack of a named diagnostic consumer, not that stale dependency claim; retain receipts and admission-versus-RSS distinctions |
| SQL construction and migration lifecycle | Keep sea-query/SQLx/pgpq and the single PostgreSQL effect owner. More typed expression construction needs a concrete query consumer. Preserve the migration-error connection-lifecycle lead separately from schema derivation and request retry policy |

These leads now share the coordinator's disposition route (§11). No additional mutable register,
library upgrade, shared-skill edit or automatic adoption is introduced by this reconciliation.

Pinned capability skills were read for DataFusion/Arrow, Python analyzers, SQLx/PostgreSQL and FastMCP. They supplied qualified leads; current repository/source contracts govern wiring. No whole-catalog refresh or unpinned API transfer was required.

| Capability | Candidates and fit | Target choice |
|---|---|---|
| Publication-family expansion | Existing macros/typed family functions; custom finite derive; runtime reflection or heterogeneous-list machinery | Begin with existing finite typed expansion. A derive is justified only by current repeated structure and satisfactory profile/epoch/async contracts |
| Grant closure | Existing model owner; plain indexed walk; graph traversal library | Reuse the owner. A new graph framework adds no necessary semantics |
| Relational derivation | Charged typed Rows; DataFusion over in-memory batches/model-owned SQL; Arrow kernels | Preserve pure finite operations where nominal/refusal contracts dominate; use relational lowerings where they simplify actual bulk semantics |
| Graph topology | Existing petgraph/native adapters and declared projections | Preserve borrowed universe/identity contracts; no graph-store replacement indicated |
| BDD existential elimination | Current bounded cofactor composition; fused limited library operation | Compare equivalent results and deliberately selected refusal policy. No measured speedup or mandatory substitution follows |
| Physical PostgreSQL lowering | Current sea-query-backed lowering plus one column model | Consolidate physical conventions; do not move semantic invariants into an ORM |
| Required lookup/index | Standard lookup, typed error constructor, charged secondary index | Small local helpers where repeated. Preserve domain-specific errors and charge lifetime |
| Identity serialization | Existing framed KeySink; explicit owned schema encoding; existing specified JSON | Declare equivalence/version and independent vectors. New serialization frameworks are not needed solely to remove Debug |
| Value presentation | Owner-defined display/source/expression functions; templates | Ordinary functions suffice. Templates do not supply domain meaning |
| Cursor/docstring mechanics | Existing hex and linked Ruff string facilities | Prefer qualified built-ins; preserve token format, source offsets and binding checks |
| Retrieval | Existing lexical/exact-vector ranks and family policy; conditional text-search/ANN alternatives | Keep task-triggered comparison. Library availability alone does not justify new indexes or claims |
| Transport/resource rendering | Rust-owned response plus existing FastMCP final-byte adaptation | Preserve thin transport and actual byte controls; add snapshot metadata at the resource owner |

The scoped CI-07 departure is defensible for charged finite domain kernels: explicit nominal inputs, deterministic result order, refusal classes and store-free testing reduce integration burden. It should be documented at the owning compute contract. That rationale does not exempt unrelated generic reimplementations from DP-13.

## 9. Alternatives and transition tradeoffs

**Current baseline.** Preserve the single model, immutable PostgreSQL generations, pure finite kernels, borrowed graph topology and thin transport. It already supplies most necessary ownership and uncertainty distinctions, but repeated contract expansions and incidental encodings increase coordinated change cost.

**Incremental target.** Correct raw mapping first, resolve requested-domain and resource provenance contracts, derive repeated publication/grant/layout mechanics from their owners, and specify identity/rendering policies. This removes repeated decisions without adding a parallel authority.

**Library-owned computation.** A model-owned DataFusion plan over explicit in-memory inputs can retain pure invocation and source identity. It must qualify order, multiplicity, memory/retention, nulls, decoding and refusal. A library BDD operation can simplify mechanics but may accept work the old preflight refused; that is a policy migration, not a transparent optimization.

**Simplest viable alternative.** Ordinary functions and finite typed expansions are sufficient for several corrections. A universal visitor, registry or query framework is unnecessary until its concrete composition and async requirements are established.

Important preservation constraints:

- Keep exact validator requirements separate from sufficient grants.
- Keep real Undefined/Deleted states separate from unattached Bound.
- Preserve independent semantic controls when consolidating mechanical definitions.
- Preserve conservative source/lockfile invalidation; replace incidental encoding without weakening dependencies.
- Keep display text distinct from executable expression and original source.
- Preserve special floats, byte values, unknown/default states and original evidence.
- Keep mutation/rebuild transitions between qualification runs, with explicit schema/identity consequences.
- Do not retain compatibility engines or second stores to ease the transition.

## 10. Verification, maturity and uncertainty

| Claim | Evidence strength/date | Outcome or limit |
|---|---|---|
| F01 mapping and served origin path | **Implemented**, source inspection 2026-10-04 | Concrete production path identified; no runtime probe executed |
| Entry safeguards | **Implemented**, source inspection 2026-10-04 | Inspected refusal boundaries; no fresh test run |
| Summary exact epochs | **Implemented**, source inspection 2026-10-04 | Facts/Model consumed views and store validation retained; no epoch-collapse conclusion |
| Structural condition/basis preservation | **Implemented**, source inspection 2026-10-04 | Conjunction, assumption union and hop-level context inspected; existing targeted receipts remain historical |
| F02/F03 serving gaps | **Implemented**, source inspection 2026-10-04 | Actual lookup/resource code inspected; proposed remedies unimplemented |
| Repeated publication/layout/grants | **Implemented**, source inspection 2026-10-04 | Decisive declaration/consumer examples read; no exhaustive occurrence count claimed |
| DataFusion membership lowering | **Implemented / Interface-checked**, 2026-10-04 | Model SQL and production PG installation inspected; DF logical-view consumer bounded as described |
| Proposed port visitor | **Proposed** | Async visitor shape/profile composition not compile-qualified |
| Retrieval/display benefit | **Proposed** | No quality measurement; frozen development/confirmation protocol required |
| Comparable product evaluation | **Proposed protocol** | No task execution or scoring; heldout/gold contents untouched |
| Review publication and handoff | **Tested documentation boundary**, 2026-10-04 | `UV_NO_SYNC=1 just docs-check` **passed**: 293 canonical pages, zero offline link errors; no product qualification implied |
| Current full qualification | **not_run** by this review | `just test-all`, `just hygiene`, current Named schema/artifact refresh and remaining Q1 journeys remain pending under existing owners |
| Real-library activation/measurement | **not_run** | Stopped Phase 5 activation, live-vector and measurement scope retained |

Static commands used for this review—`git status --short`, `git rev-parse HEAD`, and scoped `rg`, `cat` and `sed` reads—**passed** for the inspected files. They are source-discovery evidence, not functional checks.

Current STATUS attributes prior alignment `just test-all` as **passed**, 2026-10-03, and hygiene as **composite passed** for its earlier tree. Recent native/model/PG controls are bounded targeted receipts. The latest source still requires deliberate Named schema migration and matching artifacts after source formatting. This report preserves those distinctions.

Unexamined breadth includes every individual analytic/template, every provider environment read, every packet hydration branch, all administrative corruption scenarios and startup failure interleavings. None is asserted defective solely because it was not exhaustively examined. Full gates and current operator qualification remain necessary, but a passing gate would not alone settle publication-family locality or identity-contract clarity.

## 11. Authority changes and disposition

This review owns dated evidence. **Disposition transferred, 2026-10-04:** the
[target-alignment coordinator §7](../../plans/target-implementation-alignment-plan_2026-10-04.md#7-current-disposition--sole-owner-for-transferred-review-obligations)
is the single current owner for new F01–F03 and the reassessed library-leverage obligations.
The rows below identify remediation routes and preserved closure requirements, not another
mutable execution ledger. The original static assessment remains unchanged.

| Obligation | Route and disposition owner | Closure or trigger |
|---|---|---|
| New F01: preserve unattached native Bound meaning | [Coordinator §7](../../plans/target-implementation-alignment-plan_2026-10-04.md#7-current-disposition--sole-owner-for-transferred-review-obligations), analyzer A1 | Next mapping/inventory correction or raw-origin qualification; independent native/model/PG controls |
| New F02: requested-domain resolution | [Coordinator §7](../../plans/target-implementation-alignment-plan_2026-10-04.md#7-current-disposition--sole-owner-for-transferred-review-obligations), serving S1 | Next empty-result/coverage or packet qualification; known-empty versus unknown-domain controls |
| New F03: resource snapshot identity | [Coordinator §7](../../plans/target-implementation-alignment-plan_2026-10-04.md#7-current-disposition--sole-owner-for-transferred-review-obligations), serving S2 | Next resource rendering or resource qualification; actual tool/resource generation parity |
| LL-F01–LL-F05, LL-F07–LL-F09 | [Coordinator §7](../../plans/target-implementation-alignment-plan_2026-10-04.md#7-current-disposition--sole-owner-for-transferred-review-obligations), with reassessed directions here | Source-qualified transfer recorded; package controls at the coordinator, no automatic closure from this review |
| LL-F06 compute ownership/prose | Current DESIGN §B3/§15 owner; coordinator T0; ADR route only for a changed architectural choice | Narrow implemented claims, retain model-owned membership lowering and record justified CI-07 scope |
| LL-F10 stale current-owner/skill guidance | Respective current documentation and shared-skill owners, coordinator T0 | Correct current wiring and triggers. Accepted ADR substantive changes require a new/superseding decision |
| Existing alignment/enrichment/Phase 5 acceptance | Existing coordinator and detailed-plan tables | Retain current schema/artifact/full-gate and stopped activation obligations |
| Proposed retrieval/evaluation work | API/evidence owner §14.12 and forward product sequence | Activate only under authorized product scope; frozen identities, comparable population and sealed confirmation |

F01 has highest consequence priority. Publication-family and closure consolidation are architectural prerequisites for easier subsequent extensions, but are not prerequisites for correcting that mapping. Identity/rendering changes affect persisted bytes and corpora, so their rebuild and evaluation boundaries must be planned deliberately. Detailed packages, staffing and migration sequencing belong to subsequent planning.

A deferral records ownership and trigger; it does not waive the failed fidelity gate or establish conformance.

## 12. Architectural judgment and decision

| Foundation | Verdict | Reason |
|---|---|---|
| **FP-01 Separation of concerns** | **violated, scoped** | Orchestration repeats publication-family membership and grant policy; the wider mechanism-to-model direction is sound |
| **FP-02 Stable contracts** | **violated, scoped** | Native attachment failure changes asserted meaning; identity encoding has incidental representation dependencies |
| **FP-03 Composition** | **violated, scoped** | Analytic/publication extensions restate owned sets; profile/validator composition needs an authoritative contract |
| **FP-04 Model and authority** | **violated, scoped** | Unattached Bound is not faithfully realized; copied grant semantics and implicit requested-domain admission remain gaps. Entry/Local/Model/Summary/Structural operations otherwise substantially govern behavior |
| **FP-05 Explicit structure** | **violated, scoped** | Raw mapped target and resource/requested-domain representations omit consequential distinctions. Runtime lifetimes and typed outcomes are strong |
| **FP-06 Local reasoning** | **satisfied for pure semantic kernels; limited in adapter extensions** | Pure inputs and owned operations support isolation; repeated stage/layout definitions defer some mistakes to store-backed execution |

Applicable supporting-rule judgments follow these causes. DP-01/02/04/08/21/24 require the named contract corrections; DP-06/14/16 support consolidation where it removes actual repeated decisions. DP-07/09/11/12/18/19/20 and corresponding CI-03/05/06/08/09/10/11/12 obligations are supported within the inspected boundaries. DP-13/15 require qualified mechanism choices, not universal adoption. CI-01/02/04 fail at F01’s raw mapping; CI-13 needs resource provenance correction. CI-07’s selective pure-kernel departure is acceptable when recorded with the contract rationale above.

| Judgment | Verdict | Scenario evidence |
|---|---|---|
| **A1 Localize change** | **violated** | Adding an analytic/publication relation, changing grants/layout and revising identity encodings require repeated independent edits |
| **A2 Encode domain meaning explicitly** | **violated, scoped** | Bound-but-unattached meaning is reinterpreted; requested input-domain admission and resource snapshot identity are incomplete; copied grant policy bypasses authoritative realization. Faithful opaque display alone is not the basis for this verdict |
| **A3 Extend through composition** | **violated, scoped** | Repeated publication contracts obstruct ordinary analytic composition. Existing semantic, qualification, packet and runtime primitives provide a strong basis for correction |

**Bounded decision: Revise.** G2 and CI-G1 fail for the concrete native-origin relabel. G1, G7 and G8 have the scoped defects stated above. A1–A3 require incremental architectural correction. The remaining strengths do not offset these failures.

**Enclosing architecture: needs revision in the named areas, at Implemented/Interface-checked evidence strength.** The single semantic model, PostgreSQL canonical store, pure bounded operations, explicit qualification and pinned serving decomposition remain suitable foundations. This decision does not certify the whole implementation, current full gates, Phase 5 activation, retrieval usefulness or product differentiation.

The next consequential step is disposition and an incremental plan that preserves these foundations, corrects the fidelity defect, and resolves the repeated contract and presentation boundaries. Historical Tested acceptance remains valid only for its recorded scope and tree.
