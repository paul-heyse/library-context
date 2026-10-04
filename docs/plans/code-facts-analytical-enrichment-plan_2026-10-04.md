# Code-facts analytical enrichment: coordinated implementation plan

**Proposed, 2026-10-04.** This series actions the
[analytical architecture review](../design_review/reviews/design_review_code-facts-analytical-architecture_2026-10-03.md)
under core principles 3.2, code-intelligence profile 1.3 and the
[repository binding](../design_review/design_principles/binding/library-context.md).
Inspected baseline: clean `main` at `dbabbe6904a347cc9bb6a074fa00bacf6a0c54d2`.
The operator committed the previously dirty implementation during preparation; its source was
preserved. Current source and exact pinned Rust APIs were inspected on 2026-10-04 at
**Interface-checked** strength. All production changes and acceptance below remain **Proposed / not_run**.
This document is the **sole mutable execution and disposition owner for this new series**.

## 1. Outcome and boundary

The target is incremental improvement of an already useful architecture: one owned Structural
outcome operation, accurate provider guidance, and richer answers assembled from attributed facts.
Agents can compare callable roles, inspect constructor-to-reader associations, follow supported
public import routes, see diagnostics attached to exact uses, inspect contextual typing and find
supported non-call references. Behavioral explanation gains a per-use alternative inventory;
conditional origin proofs are added only if actual native evidence demonstrates their utility.
Independent small-context FCA tests strengthen the existing charged analytic kernel.

The model remains the semantic authority, PostgreSQL the single relational store, DataFusion
in-process compute, and synthesis programmatic. Preserve source/effective/synthesized roles,
five verdicts, unknown defaults, typing versus runtime evidence, finite condition models,
conditional assumptions and exact original-source identity. Catalog Flow remains NotRequested.
Existing singleton Entry and Guard stability are still supported authorities for their own questions.

Completion covers F01/F02, the selected O01–O08 routes and their first consumers, focused controls,
fixture PostgreSQL/serving journeys, and final same-tree functional and hygiene acceptance.
Conditional branches have explicit decision criteria below; an investigated non-fit is a recorded
limit, not an implementation success. No new universal type checker, recursive rule engine,
heap/alias analysis, general protocol graph, reference store or alternate export authority is planned.
Real-library reconstruction, serving activation, client registration, live vectors, comparative
retrieval studies and PR6 remain separately authorized work. Do not resume the interrupted pilot.

## 2. Document and authority routes

| Document | Responsibility and exchanged contracts |
|---|---|
| This coordinator | Shared decisions, F01/F02 packages A0/A1, qualification foundation A2, dependency route, finding disposition, assembled acceptance Q1 |
| [API contracts and evidence](api-contract-and-evidence-enrichment-plan_2026-10-04.md) | O01/O02/O04/O05/O06; model-owned role/correlation/route/context operations, typed Selection and bounded packets |
| [Guarded origins](guarded-origin-analysis-plan_2026-10-04.md) | O03; per-use native closure, inventory explanation, conditional proof usefulness decision, Local/Structural qualification |
| [Python references](python-reference-enrichment-plan_2026-10-04.md) | O07; existing-fact incoming references and hermetic ty differential oracle, explicit production non-fit |
| [Analytic kernel assurance](analytic-kernel-assurance-plan_2026-10-04.md) | O08; odis development oracle, independent consequence checks, retained production kernel |

Enduring contracts live at the [architecture owners](../design/README.md) and executable
`lctx-model::domain` declarations. Plans do not silently amend accepted ADRs. An owner refactor
preserving meaning, such as A0, needs no new decision ceremony. New guarded proof authority,
public request/packet meaning, changed provider observation/pin or another surprising choice uses
the existing ADR route with the owning section amended in the same slice; supersede rather than
edit accepted ADRs where a decision changes. O07/O08 oracle-only additions are not new production
authorities. Implementers should resolve the exact ADR scope before changing those boundaries.

The [earlier code-facts coordinator](code-facts-expansion-plan_2026-10-03.md#7-current-checkpoint-and-next-action)
retains its existing source-qualified findings and Q0 obligations. This series owns only the new
review's F01/F02/O01–O08; links to shared final receipts may serve both ledgers. The stopped
[Phase 5 qualification](semantic-model-phase5-detailed-plan_2026-10-01.md#10-finding-routes-limits-and-current-state)
and [cutover findings](semantic-model-cutover-plan_2026-09-29.md#8-findings-disposition) keep their owners.

## 3. Foundation assessment and selected design

The source review's architecture diagnosis still applies to the committed baseline. Existing
normalization, typed C1 associations, generation attempts/leases, canonical condition algebra,
shared replay and request continuations are suitable foundations. The focused planning assessment
found four consequential consumer gaps rather than reasons to replace those foundations:

- Structural replay reconstructs result rows but omits its own computation outcomes; coverage
  grammar cannot reject jointly forged completion and coverage. A0 fixes the actual operation.
- Existing C1 field links and type/evidence facts are not automatically Selection or packet inputs.
  Every new reader must declare and hydrate its exact rows; DTO presence is not readiness.
- Current overload traces lose member identity, and the supplier's closest index is not a chosen
  member. C2 retains original native origins and selection multiplicity before association.
- Global Flow family coverage does not establish complete alternatives for a specific use.
  G1 adds attributed per-use closure. Current Controls copies call qualification alone even for
  existing qualified singleton contributions; A2 corrects this independently of guarded-proof adoption.

The canonical syntax provider is independent Ruff 0.16.10/crates 0.0.16, source fork
`f7bdff69e1fb94ab0ed5b340e977aac0d26e9301`, parent
`3265ed1f944c98bb4c04d632fbefb1257cdb583d`; ty uses that nominal family and salsa 0.28.5.
Pyrefly is 1.4.0-dev.3, fork `72bb34d6d67c2bc14720c77e2ad7eff6b89d360f`, parent
`80cec3f57364bc11d4a39a419f6894a8eabcaa00`, with its embedded registry Ruff 0.0.14 isolated.
[Current pins](../pins.md), manifests and fork checks own future revisions. C2 may advance the
minimal Pyrefly observation patch while preserving this upstream premise; no unrelated upgrade.
The `python-analyzers` supplier index and exact source underpin API choices; its historical project
notes and Python/CLI documentation do not establish current Rust embedding behavior.

Production joins use existing charged keyed model operations initially. DataFusion joins are a
valid local mechanism when existing prepared tables make them simpler: preserve set versus bag
multiplicity, optional-key semantics, canonical ordering, exact support and allocation lifetimes.
Do not introduce Ascent/Datafrog for finite nonrecursive joins. Import route cycles use existing
petgraph facilities over explicitly admitted typed edges, not another general graph authority.

The new operations are primarily request-time derivations. Persist only genuinely shared evidence:
C2 native trace origins/normalized associations, C4 reusable diagnostic-use correlations and G1/G2
native inventories/proofs. C1 role comparisons, C5 routes, C6 contextual explanations and R1
incoming-reference pages can be derived from one pinned generation. If implementation discovers a
required compile-time consumer, declare its canonical output and replay at the existing stage,
rather than creating a query cache that becomes another store.

## 4. Foundation work packages

### A0 — Own and replay Structural computation outcomes (F01)

`lctx-model::domain::structural` owns a bounded operation over the admitted Structural frame,
canonical definition/invocation and produced traversal/stop inventory. It returns the existing
Structural `AnalysisOutcome` and method capability. Preserve current meanings: Delegation with a
stop is Partial/BudgetReached; DirectUsage and Handoffs retain their current completed computation;
Controls is NotRequested for Catalog, otherwise Partial/BudgetReached on a stop or
Partial/IncompleteDomain. Computation completion and source coverage remain separate dimensions.

Use the frame's existing `controls_requested`, already derived from the Local outcome and checked
against the selected profile by publication. Do not add a second persisted profile or generic status
policy registry. Index relevant rows once under the existing resource budget; retain reservations
while inputs/results live. All four method invocations are independently enumerated, including
empty-result frames. No result does not imply a missing invocation is valid.

Update `structural/frames.rs::Context`, decoding and validation inputs to hydrate **own** outcomes
as well as Local outcomes. Replay independently enumerates expected invocations, rebuilds canonical
results, invokes the shared outcome operation and compares exact outcome membership/payload.
Reject missing, extra, foreign and coupled-removal outcomes; the earlier-frontier empty case must
also require own outcomes to be empty. Keep the separate coverage membership replay. Add the new
operation source to `structural/build.rs`'s semantic definition fingerprint.

Migrate `cpg-core/src/structural.rs` to call the model operation, write its result and feed the
existing coverage fold. Delete its method/status/reason interpretation. Register all publication
inputs at the model owner; PostgreSQL and core do not acquire a classifier. No new outcome table
is needed for this correction. Amend the semantic-model/analytics owners with evidence labels.

Acceptance: focused `lctx-model`/`cpg-core` Structural tests cover all four methods, both profiles,
stop/no-stop and earlier frontiers. Actual disposable PostgreSQL publication must reject altered
outcome+coverage with stop evidence unchanged, missing/extra outcomes, and paired deletion of an
invocation/outcome where the independently enumerated C0 parent still requires them. A producer
round trip alone is insufficient. Legitimate generations still publish and serve the same meanings.

### A1 — Reconcile current extraction guidance (F02)

Correct present-tense parse ownership in semantic-model and acquisition/extraction owners,
architecture-map adjacency and `cpg-extract` module guidance. Independent latest Ruff owns canonical
syntax; Pyrefly's embedded parse supplies its native answers; ty flow uses its declared transformed
view and exact source/role attachment. Explain nominal family isolation and original/runtime view
correspondence rather than saying every consumer shares Pyrefly's AST.

Reconcile specific obsolete “not carried” claims for retained FormatString/capture payloads against
current `type_records.rs`, `capture_records.rs` and flow extraction. Do not turn a selected
subpayload into “all available facts supported,” erase remaining shim/location limits, modify
immutable ADRs or maintain repository wiring in a shared supplier skill. Update only current
owners whose meanings changed. Verify navigation and scoped source searches; `just docs-check`
belongs to final acceptance. This package can start before A0, and should precede provider guidance
being used for C2/G1. It does not require product reconstruction.

### A2 — Preserve existing Local qualification in Structural flow

The focused foundation assessment found a current contract hazard independent of O03's new proof:
`local_semantics.rs` retains the admitted Entry qualification on LocalContribution, while
`structural/controls.rs::flow` emits ArgumentFlow with call-target qualification alone. Its producer
accepts actual identity contributions without requiring their condition to be always. This is
Interface-checked evidence of a permitted strengthening path, not a fresh runtime failure receipt.

Move qualification composition into one Structural model operation over call qualification and
the admitted contribution qualification. Intersect conditions and combine canonical assumptions,
scope/context, modality, approximation and evidence status through existing qualification semantics;
refuse incompatible frames and preserve both premise sets. Derive ArgumentFlow conditionality from
the resulting qualification. Producer, replay, downstream conclusion/S0 and packet hydration use
that result. Include the operation in the Structural semantic fingerprint and exact replay inputs.
Do not limit this correction to new guarded witnesses or erase existing conditional singleton basis.

Acceptance includes an existing singleton contribution with a nontrivial native Value/region basis
and an unconditional call target: served ArgumentFlow must retain the narrower condition and all
assumptions. Independently specified domain controls cover incompatible context, conditional target,
false intersection and missing/altered contribution qualification; actual disposable PostgreSQL
publication and packet expansion reject forged strengthening. Confirm unconditional current cases
retain their meaning. Complete A2 even if G0 declines guarded proof; G3 later reuses this operation.
Coordinate its shared Structural files with A0 rather than imposing an artificial serial build policy.

## 5. Execution route and integration ownership

An agreed sketch permits design, but dependent production work requires the implemented contract
and focused success/refusal evidence. Root owns semantic decisions, shared-file integration and
acceptance. Distinct logical branches may still collide in model declarations, stage inventories,
serving packets/mappings, manifests and snapshots. Give those files one integration owner per slice;
parallel agents may map/test or edit disjoint modules. Do not serialize independent builds merely
because they exist; preserve repository jobs/profile/cache policy.

| Package | Working prerequisite | Delivery and dependency supplied |
|---|---|---|
| A1 | Current source/owner comparison | Accurate extension guidance; independent of all feature branches |
| A0 | Existing Structural frame/result contracts | Owned outcome and publication replay; required before G3 widens Controls |
| A2 | Existing Local/Structural qualification contracts | Qualification-preserving ArgumentFlow and replay, independent of G0/G2 |
| C1 | Existing callable variants/formal/type evidence | Role comparison and operation explanation; does not wait for C2 |
| C2 | A1; original-target observation seam from exact Pyrefly source | Attributed candidate/chosen origin and unique normalized association when established |
| C3 | Existing C1 SourceFieldLink; declared Selection hydration | Source initialization/reader predicate and typed relationship packet |
| C4 | Existing selected diagnostics, source uses and scenario associations | Replayed diagnostic-use correlation and evidence packet; suppression explanations |
| C5 | Captured module universe and import/exposure inputs | Bounded public route explanation; no complete-world claim |
| C6 | Existing selected actual/Expected observations | Contextual explanation with explicit missing parts |
| C7 | C6 output demonstrates argument-context value; C2 only where exact selected origin is asserted | Conditional argument Expected location/producer/consumer migration |
| G0/G1 | Existing flow and source-view contracts; A1 for guidance | Native usefulness decision, per-use closure and candidate explanation |
| G2 | Complete G1 inventory plus real G0 revealing case | Distinct guarded proof and Local consumer, or recorded deferral if premise fails |
| G3 | G2 and A0/A2 implemented/replayed | Guarded evidence through corrected Structural qualification, proof expansion and conditional synthesis |
| R1 | Existing normalized lexical identity and captured search universe | Incoming static references, independent of guarded proofs and ty integration |
| R2 | Exact pinned ty API; hermetic captured fixture environment | Development-only reference oracle; no production cancellation claim |
| K1/K2 | Existing finite FCA contract and exact odis source | Independent basis/consequence assurance; independent of feature branches |
| Q1 | All selected functional paths and decisions integrated | Remaining Q0 fixture journeys, same-tree full gates, scope-qualified handoff |

Start A0/A1/A2 and the narrow G0 evidence question early; C1/C3/C4 deliver useful answers from present
facts while that question is settled. R1/K1 are suitable independent branches. C2/C7 provider changes
follow their concrete consumer contracts. Feature packages include their packet/hydration changes;
there is no late “wire everything up” phase hiding required consumers. Q1 is a join, not a full gate
at every package boundary.

## 6. Migration, verification and completion

New declarations/codebook arms are append-only. Schema and serving-wire changes require reviewed
snapshot diffs and explicit migration; accept only after inspecting `.snap.new`. Update model/input/
analysis/mapping identities and generated contracts through the established mechanism. Stale native
or Python adapters must reject an incompatible describe/schema identity. Python transports typed
results and availability; it never recomputes semantic differences, origin or diagnostic association.

Use current fixture captures and disposable PostgreSQL 18 for plan tests. Publication is atomic;
a failed attempt exposes no half-family and is aborted through existing lifecycle. Request-time
operations hold the generation lease, budget and cursor identity through all reads and pagination.
Proof membership is verified in full before paging its explanation. No old-format readers, legacy
IDs or temporary dual authorities. Retire obsolete classifiers and unused projections introduced by
this work with their replacement. Existing singleton Entry and production FCA remain because they
have current consumers.

Preserve interrupted unselected staging `d3a3fa026a1237a1c4e175b9b1019eeb` and operator registrations
while implementing fixture scope. Real-library activation requires separate authorization and the
current runbook: quiesce readers, rebuild from pinned inputs, verify current contracts, remove obsolete
project state and select only a qualified current generation. This plan creates no rollback runtime.

Q1's assembled fixture set establishes actual packet benefit and fidelity, not just additional rows:
role comparison, constructor option→field→reader, diagnostic→admitted use, public re-export route,
contextual type explanation, non-call reference, and guarded proof/refusal if the G0 branch admits it.
For each, record the original fact IDs/view/support and exact before/after answer; an existing rich
packet is the baseline. Explicit empty, unknown, omitted and truncated results must survive native
service and MCP stdio round trips. Challenge wrong-context/view/member inputs, altered support,
lease/cursor reuse across generations and byte/count limits. Typed conditional, generated, Terminal
and raised-type **actual PostgreSQL** Q0 journeys from the earlier coordinator remain required;
reuse their matching receipts or complete missing cases on this final tree and link both owners.
Do not copy or silently close the earlier findings.

During implementation run scoped release compile checks and focused tests/probes for each touched
capability, with `INSTA_UPDATE=no` outside deliberate snapshot acceptance. After all selected
functional scope, run `just test-all` and `just hygiene` once for the same final tree. Fix findings
and rerun the failing `just <id>`; repeat the functional gate only when a repair materially affects
it. Report a composite receipt honestly when appropriate. The end-of-turn hook owns formatting
and generators; do not run them manually. Final hygiene includes clippy, type/policy/docs/dependency
and store checks; formatting is not a substitute.

Required evidence establishes semantics, replay, resource refusal and served usefulness on named
fixtures. No scale/speed/retrieval-quality improvement is claimed. Broad performance campaigns,
real-library completeness and independent runtime execution studies are deferred until their actual
workload/product task is authorized. Optional measurements cannot block completion of this scope.
Follow the existing review cadence for materially changed proof/provider/public contracts; focused
advice is not certification of an enclosing subsystem and creates no new standing gate.

## 7. Current disposition and checkpoint

Source-qualified links below refer only to the analytical-architecture review; similarly numbered
IDs from earlier reviews remain separate. Supporting plans own design detail, not mutable status.

| Source ID | Responsible component / package | Current disposition and required closure evidence |
|---|---|---|
| [F01](../design_review/reviews/design_review_code-facts-analytical-architecture_2026-10-03.md#F01) | Model Structural/core adapter, A0 | **Scheduled / open**; canonical outcome replay plus adversarial real-PG publication controls |
| [F02](../design_review/reviews/design_review_code-facts-analytical-architecture_2026-10-03.md#F02) | Current architecture/extraction guidance, A1 | **Closed / Implemented guidance and Tested publication, 2026-10-04**; source-aligned owners, ADR-0121/0122 and final docs check passed (289 pages, zero errors) |
| [O01](../design_review/reviews/design_review_code-facts-analytical-architecture_2026-10-03.md#O01) | Model callable/serving and native trace adapter, C1/C2 | **Scheduled / open**; role comparison answer and identity-qualified chosen/candidate association; ambiguous origin remains explicit |
| [O02](../design_review/reviews/design_review_code-facts-analytical-architecture_2026-10-03.md#O02) | Model Selection/catalog, C3 | **Scheduled / open**; hydrated predicate and exact source-field packet, runtime unknown preserved |
| [O03](../design_review/reviews/design_review_code-facts-analytical-architecture_2026-10-03.md#O03) | Flow/Entry/Local/Structural, A2/G0–G3 | **Scheduled / conditional**; qualification correction and complete inventory/explanation required; native usefulness evidence admits proof branch or records Deferred proof with trigger |
| [O04](../design_review/reviews/design_review_code-facts-analytical-architecture_2026-10-03.md#O04) | Model C1 evidence/serving, C4 | **Scheduled / open**; exact diagnostic-use correlation with unassociated remainder and no execution verdict |
| [O05](../design_review/reviews/design_review_code-facts-analytical-architecture_2026-10-03.md#O05) | Model import route/serving, C5 | **Scheduled / open**; bounded supported route, cycle/coverage/candidate controls |
| [O06](../design_review/reviews/design_review_code-facts-analytical-architecture_2026-10-03.md#O06) | Model types/extractor/serving, C6/C7 | **Scheduled / conditional**; existing Expected interpretation required; argument extension only after demonstrated useful retained trace |
| [O07](../design_review/reviews/design_review_code-facts-analytical-architecture_2026-10-03.md#O07) | Model references and ty test harness, R1/R2 | **Scheduled / open**; scoped incoming-reference product and attributed oracle discrepancies; production ty search deferred on resource non-fit |
| [O08](../design_review/reviews/design_review_code-facts-analytical-architecture_2026-10-03.md#O08) | Analytics tests/model bounds, K1/K2 | **Closed / focused Tested, 2026-10-04**; four independent finite-context controls passed, production charged kernel retained. Full-series gates remain separate |

Conditional decisions belong here when executed: name the inspected fixture/API, outcome,
dependent packages enabled or deferred, and reopening trigger. A not-demonstrated benefit may
close an investigation, never a proof implementation or a new Tested capability claim.

Additional provider leads from the review's
[capability evidence §§4–5](../design_review/evidence/2026-10-03_code-facts-analytical-architecture/provider-capabilities.md)
have these concrete routes; they are not a demand to extract the full survey:

| Lead | Selected route or explicit boundary |
|---|---|
| Native original overload/member identity | C2 observation seam; exact source exists before callable erasure, retain representative/multiple selection limits |
| Expected argument trace | C7 conditional consumer-first extension, no second assignability engine |
| `has_base_any` / missing ancestor context | C3 bounded constructor/member uncertainty explanation; retain current known candidates, test dynamic/Any-base contrast |
| Suppression/rule coverage detail | C4 consumes existing selected channels/config; add original directive span only if required to explain exact source, no full lint catalog |
| Per-name ty StarImport state | C5 uncertainty explanation only if current alias/module evidence cannot explain a selected fixture; no alternate export resolution |
| Property getter trace | R2/C6 inspect as corroborating oracle only if a selected property-read discrepancy needs it; origin and access value remain distinct |
| ty call signatures, expression types, descriptor/member/subtype APIs | Oracle controls only on a concrete discrepancy; arity hints/private enumeration are not production facts or whole-world closure |
| Other class/framework/ordering flags, directive fixes, notebook/PEP723/editor APIs | **Deferred**; trigger a named operation/example journey whose answer changes, then establish source/role/resource contract. No current selected consumer establishes benefit |
| General recursive relations, new graph ranking or universal flow graph | **Deferred**; trigger a concrete repeated recursive/path question not answered by current finite operations; no current requirement |

Production execution is **in progress**, 2026-10-04. A0/A2 model ownership and qualification
composition, C1–C6, enabled C7, R1 and G1 observation/explanation consumers are integrated.
The current assembled CLI/extractor release compile passed; individual functional receipts below
do not qualify Q1. K1/K2's independent finite-context controls passed; R2's differential oracle is
implemented with its bounded joined-process runtime oracle passed, including explicit typing-only and unreachable-use controls. G2/G3 remain Deferred under the G0 decision. Final PostgreSQL/MCP journeys and same-tree adapter freshness remain pending. The first
`just test-all` and `just hygiene` attempts **failed** at a concurrent uv build-cache lock timeout;
functional and named hygiene reruns are underway on the assembled tree.

**G0 passed**, 2026-10-04: `cargo test --release -p cpg-flow --test flow_shapes
native_guarded_origin_search -- --nocapture` on the flow branch. The actual fixture
`fixtures/python/guarded_origin_inventory/cases.py` retained eleven identity-return reads, seven with two
alternatives: nine inventories complete and two incomplete, including loop-expanded back-edges.
Native branch-local/early-return singletons remain distinct. No
multi-candidate native Value/region condition excluded every competitor. **G2/G3 Deferred**:
reopen on an actual multi-candidate case with an attributed native Value condition Q whose
conjunction excludes every competing candidate, under complete per-use inventory. G1 and A2
remain required; this decision does not claim guarded proof implementation.

Next: complete native overload/reference/FCA controls and actual PostgreSQL publication/packet
journeys, including the retained conditional/generated/Terminal/raised-type Q0 obligations; then
run the same-tree full functional and hygiene gates. The authoring receipt below retains its
original documentation-only boundary.

### Authoring receipt

**passed**, 2026-10-04: `just docs-check`, 288 canonical pages, zero offline link errors.
Focused independent contract/library advice was integrated: unconditional A2, query-local overload
ordinals with unavailable specialization when not retained, source boundaries for unattached uses,
and accurate FCA cap/resource semantics. This is documentation/source assessment, not production
acceptance. Production compile/tests, `just test-all`, `just hygiene`, new runtime probes and
real-library/operator actions are **not_run** in this documentation-only authoring task. The
concurrent advisory `docs/library-utilization.jsonl` refresh is preserved outside this scoped change.

### Execution receipt

**Implemented / focused Tested**, 2026-10-04; Q1 remains open.

| Command and baseline | Outcome and boundary |
|---|---|
| `cargo check --release -p lctx -p cpg-extract --tests`, assembled main (`lctx-enrichment-integrated-check4.log`) | **passed**; includes supplier `63cda076`, typed observations/association and serving consumers. Subsequent bounded resource refinements require a focused rerun |
| `cargo test --release -p lctx-model --test structural` | **passed**, five controls |
| `cargo test --release -p lctx-model --lib singleton_conditional_basis_is_not_promoted_or_erased` | **passed** on the assembled tree, including conditional-call, false-conjunction, incompatible context, premise preservation and budget-refusal controls |
| `cargo test --release -p cpg-core --test structural` | **failed** before publication: the hand-built fixture model omitted SummaryExceptionOutcome. Canonical model inventory repair integrated; actual PG replay/adversarial rerun pending |
| `cargo test --release -p cpg-extract --test typed_flow actual_native_inventory -- --nocapture`, flow branch | **passed**, 26 actual native inventories and 34 candidates; real loop back-edge preserves loop-expanded/reachability-loss flags. Catalog does not request flow. Initial supplier was `72bb34d6`; the assembled rerun at `63cda076` also **passed** the 26-inventory/34-candidate boundary |
| Release model `domain_selection_catalog` and `serving_contracts`, API branch | **passed**, 41 controls; three additional argument Expected/stored singleton/original inventory controls also **passed**. This is not a served PG receipt |
| Existing API actual PG journey, API branch | **failed** at the old selected-location invariant rejecting captured argument Expected. Exact CallArgument membership/context repair integrated; rerun pending |
| `cargo test --release -p lctx-analytics --test implication_oracle -- --nocapture`, assembled main | **passed**, four controls over 22 finite contexts: independent consequences, full/iceberg concepts, nominal reordering, adversarial mutations and charged cap behavior |
| `cargo test --release -p cpg-extract --test native_overload_origins -- --nocapture`, assembled main | **composite passed**, three controls: exact original declarations through equal-shaped normalization, selection/recovery limits and unavailable generic/bound basis, and replay refusal for dropped/altered/coherently moved vectors. Initial direct control **failed** because fixture lookup selected a declaration instead of the final call; exact lookup repaired before the passing rerun |
| `cargo test --release -p lctx-model --lib diagnostic_uses::tests`, assembled main | **passed**, six controls: exact primary/nearest use, distinct-subject ambiguity, duplicate attribution, foreign artifact/context and secondary-span refusal |
| `cargo test --release -p lctx-model --lib original_members_associate` | **passed**, one control; shape equality does not establish native declaration identity |

C7 is **enabled**: actual retained Expected supplies useful `list[int]` context for `consume([])`
and a retained `str` hint for a failed overloaded call. The initial absence assumption failed
(`lctx-api-expected-trace.log`); corrected native probe and actual consumer qualification are pending.
The corrected native Expected probe **passed**, one control. Expected is not successful applicability. C2's published observation fork
`63cda076956013cd0bd1d0d05c785f747fe6adc0` preserves the upstream parent and embedded Ruff premise.
Its isolated supplier compile **passed** with the existing allocative patch. Original candidates,
selection limits, exact declaration/support association and packets are implemented; native identity,
remaining native controls and actual PG packet acceptance remain pending. **Old/new checker parity
passed**, 2026-10-04: unchanged eight-case harness JSON compared exactly, including inferred
Infer/Any distinctions, old getters and sorted diagnostics. The unchanged old-supplier
checker receipt **passed**; its canonical JSON is retained at
`/home/paul/.cache/lctx-api-overload-checker-old.json` (SHA256
`03a0f28150b2db2ad865be040af953757e5f30303a459f99a6563497a9340df7`).

R1's current `domain_selection_catalog` adversarial support and exact input-inventory controls
**passed**, two tests (`lctx-api-root-model-tests-rerun.log`), after repairing test-only occurrence
roles and ambiguous Fidelity imports. Actual PG serving rerun **failed** before packet assertions
at duplicate Structural Condition declaration; the single vocabulary emitter repair is integrated
at `17a034e7` and its focused release compile **passed**. PG rerun remains pending.
R2's first native run **failed** at an incorrect differently spelled import-alias expectation.
Pinned source confirms requested-spelling filtering despite ResolveAliases and `None` for both
empty search and unavailable resolution. Explicit positive/negative spelling controls and the
honest ambiguous-empty boundary are integrated. The next rerun **failed** at a keyword label
on an untyped decorated callable; a separate known-signature positive control and explicit
unavailable decorated-keyword negative control repair that assumption. Final
`cargo test --release -p cpg-extract --test python_reference_oracle -- --nocapture` **passed**,
2026-10-04: joined parent/child controls completed in 0.40 seconds, with included-file reorder,
shadowing, keyword/member/property/augmented assignment, explicit typing-only/unreachable reads
and original-range normalization checks (`lctx-python-reference-oracle-contexts.log`).
The ignored child is invoked and joined by the passing bounded parent; it is not skipped evidence.

Superseded task-owned compile/test processes ended with scoped SIGINT (exit 130) to release a
confirmed fine-grained Cargo lock cycle. They are interrupted **not_run** receipts, never passing
tests. Jobs, release optimization, shared cache and normal build settings were preserved. Current
queues are performing normal compile/link work; no resource or performance claim follows.
The initial `NEXTEST_TEST_THREADS=8 just test-all` **failed** before Nextest: CLI compilation
passed, but `uv sync --locked` timed out waiting for the concurrently rebuilding editable adapter.
The retry uses `UV_LOCK_TIMEOUT=3600`, preserving ordinary Cargo parallelism and owning one
adapter rebuild. No workspace functional pass or current adapter-freshness pass is claimed yet.

`just hygiene` initially **failed** at the same uv cache timeout after `lint-agents` passed. Named
checks are repaired separately: `UV_NO_SYNC=1 just adr-lint` **passed** (67 records), `just
docs-check` **passed** (289 canonical pages, zero offline errors), `UV_NO_SYNC=1 just ruff`,
`just types`, fixtures/gold/rules checks, full workspace/all-target `just clippy`, and `just deps`
**passed**, 2026-10-04. Dependency repairs remove unused oracle imports, admit libcst's PSF-2.0
license and retain the exact development-only family exceptions; no production oracle closure
is added. The two generated refreshes use the operator's previously granted scoped exception.
Default `just store-check` **failed** with three findings: the installation and preserved staging
use the earlier model/lowering, so the checker does not compare their generation shape. This is
the explicitly preserved operator state, not authorization to reset it. A separate disposable
PostgreSQL 18 database using the unchanged four service roles was provisioned; current-model
`lctx --database <qualification-config> store install` **passed**, and named `just store-check`
with `LCTX_DATABASE_CONFIG=<qualification-config>` **passed**, zero generations/findings, for
model `9767e8813dc7d763d7c9be89e88516da3d117263f432e4a1da189463de30bc4c` at `4d437f30`
(`lctx-enrichment-store-check-current-model.log`). The isolated empty database was reset once
after the decoder source changed its model identity; no operator state was reset. Final handoff
source changes require a matching scoped reset/check. This scopes store qualification
to the planned fixture state; default operator-store migration remains outside Q1.

Actual serving journeys progressed past Analytic after `ca887c56`, then **failed** before packet
assertions at Synthesis's missing declared Terminal input loader. The narrow dispatcher repair
`014d8d29` compiled successfully; all affected native/PostgreSQL journeys are rerunning. It
changes neither declared input inventory nor validators. Composite hygiene and Q1 remain open.

The subsequent consumed-input audit found Synthesis's shared ScopeIndex loader also omitted
CorpusLibrary/InputDistribution. `4d437f30` restores the exact model-owned ownership macro without
widening the declared input inventory. The four Local/Summary/Analytic/Synthesis decoder-registry
controls **passed** for both profiles; the two consumed-input controls **passed**, retaining distinct
immutable epochs and coalescing aliases of the same source. Matching `just native-adapter-ready`
**passed** for this revision (`lctx-enrichment-native-adapter-final.log`).
The genuine full-layer Structural journey then published and validated both profiles, but **failed**
at its Behavioral named-handoff assertion: the older handoff consumer still used global Flow-family
completeness as authority for a particular use. The bounded correction requires the new complete
native per-use inventory/support plus all existing exact reaching/definition/condition premises;
Partial family coverage remains evidence, never enumeration authority. The unchanged positive
journey and incomplete/foreign-inventory controls are pending. This failure is not a decoder or
whole-model validator bypass and does not reopen G2/G3.

ADR-0122 supersedes the old singleton global-family admission requirement while carrying forward
ADR-0111's checked-false retention lane and finite-negative boundaries. The architectural owners
now distinguish per-use enumeration closure from family coverage. Corrected runtime, schema
migration and final adapter/gate receipts remain pending; accepted decision is not verified closure.

`UV_NO_SYNC=1 just adr-lint` **passed**, 67 records, and `UV_NO_SYNC=1 just docs-check`
**passed**, 289 canonical pages, zero errors, after ADR-0122 and owner updates. The queued
`lctx-enrichment-test-all-final.log` attempt was interrupted with scoped SIGINT before tests
while the discovered Entry/handoff admission correction changes shared source; it is **not_run**,
not a functional pass. Final stable-tree acceptance remains pending.

ADR-0122 implementation integrated at `11420bc0`/`0c60d4f7`; the genuine native Entry control
**passed**, one test in 0.62 seconds, including Partial family/complete per-use admission and
missing/incomplete/foreign/non-native refusals. The model seed migration `75e549d2` supplies
explicit finite inventories once; omission/mutation controls do not automatically reseed.
The actual C2 PostgreSQL packet journey **passed**, 84.47 seconds, at model
`9767e8813dc7d763d7c9be89e88516da3d117263f432e4a1da189463de30bc4c`
(`lctx-api-root-packet-tests-decoders-complete.log`). The adjacent API journey **failed** before
stdio at an incorrect decorator direct-field expectation; `f2a2009e`/`eeb4d835` instead check
the exact original name span, canonical Decorator parent/outer placement and truthful Child field.
Its targeted release compile **passed**; final API/MCP runtime remains pending. The native
MCP helper now checks exact generation/model admission before opening stdio.

A bounded independent audit identified raw-helper source-view/qualification/target gaps protected
by publication but insufficient for the helper's own contract. `20f2f678` reuses the existing exact
source-coverage qualification operation, checks original digest/byte length and actual Bound target.
`65f631e4` adds coherent forged-view, Candidate modality, foreign scope, approximate view and
Unbound-target controls; their focused runtime remains pending. The audit of that stable source
accepted the correction statically, **Interface-checked**, 2026-10-04.
The C4/C5 Catalog journey then **failed** before generation/packet assertions because Synthesis's
Catalog input mask inherited six new Flow relations without excluding them. `3a7463b0` fixes that
consumer mask and adds a profile control. Native Flow remains NotRequested in Catalog; no empty
Flow writer, native provider activation, admission or validator exception was introduced.

Full workspace/all-target `just clippy` **passed**, 2026-10-04, on combined `65f631e4`
(`lctx-enrichment-clippy-source-view-closure.log`), after one coverage let-chain lint and a
test-only occurrence-field typo were repaired. Own superseded adapter/snapshot/full-gate
refreshes were interrupted before qualification while these concrete source repairs changed
the model identity; they are **not_run**, never passing receipts. Final matching native adapters,
reviewed model describe snapshot and `just test-all` are running against the stable combined
source. The five surviving conditional/generated/Terminal/raised-type Q0 packet tests and their
upstream PG controls are ordinary non-ignored tests included by that full workspace recipe.

Final focused model rerun **passed**, 2026-10-04, `cargo test --release -p lctx-model
--test domain_flow_inventory --test domain_stability --test native_requests -- --nocapture`: four
inventory, eleven stability/raw-helper and eighteen native-request controls, at `20f2f678` plus
`65f631e4`. The earlier two stability failures are retained in
`/home/paul/.cache/lctx-enrichment-flow-focused-receipts.txt`: a deliberately narrowed positive
fixture needed an explicit inventory update, and the new forged-view check exercised the earlier
helper before its final geometry guard. No implicit reseeding was added.
`cargo test --release -p lctx-model --lib catalog_pattern_inputs_leave_native_flow_unrequested
-- --nocapture` **passed**, one profile control (`lctx-enrichment-catalog-flow-inputs.log`).
Checkpoint `UV_NO_SYNC=1 just docs-check` **passed**, 289 canonical pages and zero errors.

Schema migration review found that native premise tag 37 had been reused when the old
DependencyModule observation was replaced by ModuleResolution. `7ea2f801` retires 37 and
appends ModuleResolution at 75; no old code is renumbered and no old-format reader is retained.
The actual describe snapshot migration is awaiting the matching final test output. Full
workspace/all-target `just clippy` **passed** for that repair (`lctx-enrichment-clippy-codebook37.log`).

The operator explicitly authorized one final `just fmt` before qualification. It **passed**,
2026-10-04 (`lctx-enrichment-authorized-final-format.log`), and its owned changes are committed
at `9dbe7239`. The superseded codebook-final CLI/adapter/snapshot queues were interrupted
before tests with scoped SIGINT; they are **not_run**. The formatted-source functional gate,
actual describe refresh and Clippy are running; Q1 remains open. `UV_NO_SYNC=1 just ruff types`
**passed** after formatting (`lctx-enrichment-python-hygiene-formatted-final.log`). Named
`lint-agents`, `adr-lint` (67), fixtures (187), gold, rules (three controls), `docs-check`
(289 canonical pages, zero errors) and `deps` **passed** before the formatting-only commit;
dependency receipt is `lctx-enrichment-deps-final.log`. The four completed task worktrees were
removed after confirming integration, no active processes and external receipt preservation;
shared build intermediates and unrelated worktrees remain intact.

Final formatted-source schema migration review **passed**, 2026-10-04, on the actual
`lctx model describe` output at `9dbe7239`. The model is
`a00210d0595770adc2d77c4250f24123cb9f32f46cbfab3d8d799c56431f30a8`: 878 relations,
96 additions and two removed old dependency-module relations. Existing field-code meanings
and relative order are unchanged; native premise tag 37 is retired and replacement tag 75
is appended. Existing sum arms add required inventory references only in named handoff/setup
variants. Original imports now target ModuleResolution; additional exception/default/theory/type
codes preserve their prior assignments. The actual snapshot diff was reviewed before
`cargo insta accept --snapshot model_describe__model_describe.snap` **passed**; acceptance is
committed as schema migration `b7e8629b`. Review receipt:
`/home/paul/.cache/lctx-enrichment-model-describe-final-review.json`. The initial deliberate
`INSTA_UPDATE=new cargo test --release -p lctx --test model_describe -- --nocapture` **failed**
only the old snapshot assertion and generated that new output in 14m31s; its assertion rerun
and the full gate remain pending. The isolated empty store reset **passed** for this model,
dropping zero generations; no operator-store reset occurred. Final formatted `just clippy`
**passed** (`lctx-enrichment-clippy-formatted-final.log`).

Reviewed snapshot assertion rerun **passed**, 2026-10-04:
`INSTA_UPDATE=no cargo test --release -p lctx --test model_describe -- --nocapture`, one
control in 0.05 seconds (`lctx-enrichment-model-describe-accepted-final.log`). Scoped final
`LCTX_DATABASE_CONFIG=/home/paul/.cache/lctx-enrichment-store-qualification/postgres.json
just store-check` **passed**, zero generations and zero findings
(`lctx-enrichment-store-check-formatted-final.log`), after reset of only the owned empty
qualification database to model `a00210d0595770adc2d77c4250f24123cb9f32f46cbfab3d8d799c56431f30a8`.
Read-only preservation check confirms the operator's `d3a3fa026a1237a1c4e175b9b1019eeb`
still has 204 receipts, is staging and is unselected (`lctx-enrichment-preserved-staging-final.txt`).
The full gate's shipped CLI build **passed**; its semantics adapter rebuilt and storage adapter
is rebuilding. Functional/PG/MCP qualification remains pending.

Final named hygiene repairs are **composite passed within the scoped current-model store
boundary**, 2026-10-04. The original `just hygiene` failure at the uv cache timeout is retained.
All named checks completed: lint-agents, ADR lint, fixtures, gold, rules scan/test, Ruff,
Pyrefly types, docs, deps, full workspace/all-target Clippy and store-check. Final formatted
Python, Clippy, docs and dependency logs are `lctx-enrichment-{python-hygiene,clippy,docs,deps}
-formatted-final.log`; docs reports 289 canonical pages and zero link errors. Dependency
checks were repeated after formatting changed the checker script and **passed**, including
family/fork, bans/sources/licenses, shear and unchanged Hakari output. Earlier unaffected
agent/ADR/fixture/gold/rule receipts remain applicable. Store-check uses the owned empty PG18
database and final model; the preserved old operator-store failure remains outside this
qualification boundary. This closes F02 guidance/publication only, not runtime Q1. Both native
adapters rebuilt successfully inside the full gate; workspace test compilation remains active.


The assembled `UV_LOCK_TIMEOUT=3600 NEXTEST_TEST_THREADS=8 just test-all` **failed**,
2026-10-04 (`/home/paul/.cache/lctx-enrichment-test-all-formatted-final.log`): 1,096 Rust
controls ran, 997 passed, 95 failed, four timed out and three were skipped. Shipped CLI and
both installed native adapters rebuilt successfully. Python/oracles and doctests are **not_run**
because the Rust step failed. All five surviving Q0 journeys executed: generated-wrapper packets
and both complete/incomplete raised-type controls **passed**; conditional packets **failed**
(Contract), and Terminal packets **failed** (300-second timeout). These are scoped outcomes,
not Q1 acceptance. The original 99 failure/timeout cases are retained in
`/home/paul/.cache/lctx-enrichment-initial-gate-failures.json`.

Integrated repairs preserve strict authority: collapsed native type-parameter capture scopes
produce attributed ScopeBoundary instead of an invalid equal-scope snapshot; native TypedDict
keys retain arbitrary raw spellings with origin validation; Summary hydrates evidence from the
Model vocabulary view; source-class declarations/MRO use bounded inspection authority rather
than executable-read authority. Shared PostgreSQL fixtures now derive their native writers
from the existing canonical assertion/support registry, and current fixture directories, packet
JSON and selection-model declarations are registered. Focused native diagnostics and compile
reruns are in progress; these repairs are **Implemented**, not yet Tested. The prior schema,
Clippy and scoped-store receipts remain dated to the pre-repair model, not the repaired tree.


Focused repair receipts, **2026-10-04**, retain their individual pre-subsequent-repair
baselines and do not establish a current full gate:

- **passed**: direct `cargo test --release -p cpg-core --test behavioral_frontiers -- --nocapture`,
  one actual native frontier/validation control, 440 seconds
  (`lctx-enrichment-frontier-progress-direct.log`). Direct `cargo test --release -p lctx
  --test terminal_question_packets -- --nocapture` **passed**, all three positive and ten
  uncertainty packet controls, 493 seconds (`lctx-enrichment-terminal-progress-direct.log`).
  These receipts justify test-specific 900-second Nextest budgets; product request/resource
  limits and all semantic assertions remain unchanged.
- **failed**: focused model/PostgreSQL/lexical/syntax rerun, 56 passed and one failed
  (`lctx-enrichment-repairs-fixtures-rerun2.log`). All lifecycle, domain call/type and model
  input controls passed. The lexical assertion conflated native Ruff annotation scopes with
  the recognizer oracle; the bounded attribution filter is integrated, rerun pending.
- **failed**: actual serving/selection rerun, three source-characterization controls passed,
  two selection controls failed at duplicate nominal registration, and conditional packets
  failed (`lctx-enrichment-repairs-serving-selection-rerun.log`). Selection fixture replay
  deduplication is integrated. Conditional diagnostics conclusively retained Local finite
  premises with no composed TransferAlternative: capture hydration now retains these valid
  claims without requiring a direct capture DTO. Strict direct-capture checks remain;
  actual conditional/capture reruns are pending.
- Synthesized native initializer shape replay is integrated with exact same-run native
  observation/support, distinct Types/Signatures surfaces and complete slot/digest checks;
  its adversarial and actual field tests are pending. It does not relabel synthesized
  metadata as source/runtime authority. Exported-definition versus overload-metadata
  candidate separation and the remaining read/source-call diagnostics are in progress.

Earlier `a00210d0…` schema/adapters/hygiene receipts do not qualify these subsequent model
changes. Full `just test-all`, matching artifacts and affected hygiene checks remain required.


The direct generated CPython challenge **passed**, 2026-10-04:
`cargo test --release -p lctx --test serving_soundness -- --nocapture`, nine grouped programs,
52 functions, exact_identity=4, refuted_guard=1, refuted_return=1, composed_paths=26;
1,101 seconds (`lctx-enrichment-soundness-progress-direct.log`). This supports a test-specific
1,800-second Nextest budget; product limits and all challenge assertions remain unchanged.
A live CPU sample identified repeated admitted syntax-index construction in completion,
not a stopped service or compiler termination (`lctx-enrichment-soundness-cpu-stack.txt`).
Bounded per-frame index reuse is integrated; its context/resource/work-reset controls passed
in the direct focused model run below. No performance improvement is claimed.
The split structural/documentary rerun **failed**: four passed, four failed, no timeouts
(`lctx-enrichment-structural-documentary-rerun.log`). Exact earlier coverage refusal, failed-stage
rollback controls, named-handoff admission and duplicate signature-role option candidates are
being reconciled with their original strict acceptance obligations. This remains a partial
qualification receipt, not Q1 closure.


Additional bounded repair evidence, **2026-10-04**: the focused Cargo build failed at a
new diagnostic's `SubjectBoundary` namespace; this was repaired to its syntax owner. Other
successfully compiled test binaries were executed directly, with exact commands/artifact
modification times retained in `lctx-enrichment-built-repair-receipts.json`, one test thread
per binary and seven independent binaries. Normalized bindings **passed** all 11 controls,
including native ClassOf/cross-provider receipt refusals; symbolic fields **passed** all five,
including 22 generated-initializer mutations; Entry witnesses **passed** all five; attributed
lexical oracle **passed** both. This does not claim a completed Cargo/Nextest gate.
Normalized recovery **failed** all three controls at the definition report-role gate; the
exact emitter uses ReportProjection and its bounded correction is integrated, rerun pending.
Read channels **failed** two of three, retired read expectations two of five. Their actual
Flow coverage is CompleteUnderStatedModel; field declaration/read receiver admission is
being repaired without relaxing that closure requirement. Direct model completion controls
**passed** both; documentary role controls **passed** all six
(`lctx-enrichment-built-{prepared-completion,documentary-roles}.log`). No speed claim is made.
Actual conditional serving now hydrates without Contract, but its typing-qualified answer is
missing (`lctx-enrichment-local-proof-lexical-rerun2.log`); the positive is retained. Exact finite
claim/facet/owner diagnostics and shared restricted-Local-seed consequence admission are in
progress. Matching artifacts, affected hygiene and the full gate remain open.


The subsequent native-consumer Cargo/Nextest rerun **failed**, 2026-10-04: 39 controls,
33 passed, six failed (`lctx-enrichment-native-consumer-repairs-rerun.log`). All three
normalized recovery controls now passed with exact ReportProjection support; the complete
field-screen control passed without relaxing Flow completeness. Remaining read failures
are being rerun after exact builtin-answer selection; the three source-call failures retain
Bound source shapes with independently Known body identity, but unrelated unavailable
signatures leave artifact family coverage Partial. ADR-0123 and §15.5 accept exact selected
Source-role signature closure for composition only. Global binding and effective-invocation
closure remain unchanged; implementation/refusal controls and runtime reruns are pending.
The serving/structural rerun (`lctx-enrichment-serving-structural-repairs-rerun.log`) **failed**:
four passed, ten failed, one timed out. Its compiled model predates the ReportProjection
repair, so a fresh serving rerun is required; these outcomes are not current repaired-model
acceptance. Shared restricted-Local consequence selection and exact native Ruff/source read
repairs are integrated. Fresh focused Summary/read and serving tests are in progress.


The fresh read/Summary rerun **failed**, 2026-10-04: 12 controls, eight passed and four
failed (`lctx-enrichment-read-summary-current.log`), release compile **passed**. Three Summary
controls passed, including exact exception/capture timing. The fourth passed all twelve replay
challenges, then failed a newly added premise/verdict expectation: its actual finite results
retain Conditional/runtime-empty-basis and Established/one-typing-premise alternatives. The
Literal[0] restriction makes the guard true under that premise; the test now requires both
exact verdicts separately. This is an acceptance-test correction, not a production verdict change.
Read controls still fail at the ChoiceA global initializer and two field-negative closure
assertions; bounded original-premise diagnostics are integrated, and repairs remain open.
Selected source composition is integrated under ADR-0123 (`65605c3d`, `7d02f768`). Independent
static inspection found and repaired standalone declaration/member evidence hydration. It also
found generic Summary explanation visibility missing the scoped enumeration/member domain despite
correct replay; explicit typed premise links and actual explanation qualification remain in progress.
No current full gate, hygiene, adapter or schema acceptance is claimed.


Fresh focused receipts, **2026-10-04** (before the subsequent Summary schema migration):

- Serving/Structural **failed**: 15 controls, 12 passed, two selection controls failed and one
  Behavioral Structural journey timed out at final validation (`lctx-enrichment-serving-current-model.log`).
  Conditional packets, capture packets, all three source-characterization controls, documentary
  synthesis, Catalog Structural and all five corruption controls passed. The selection fixture
  was validating its partial executed assembly as the entire model; its canonical executed-owner
  dependency closure is repaired, with production embedding-text requirements unchanged.
- Source calls/bindings **failed**: 17 controls, 16 passed; all original source-call and 11 binding
  controls passed (`lctx-enrichment-selected-source-closure.log`). The new refusal control reached
  its final foreign-invocation mutation, which assumed an absent fixture row. It now hydrates a
  different actual provider run's invocation evidence; no control is skipped.
- Reads **failed**: nine controls, seven passed; both original field-negative assertions passed
  after shared native builtin-target admission (`lctx-enrichment-read-initializer-diagnostic.log`).
  The new Partial Flow control lacked its required structured reason; that fixture is repaired.
  The remaining ChoiceA positive requires exact cross-scope native FinalReference inspection,
  without granting scope-exact closure-capture or runtime class authority.

ADR-0123's selected enumeration/support Summary premise pair and generic member traversal are
integrated (`79a2b874`, `a6bafbea`). Independent static review is **clean within this identified scope**,
2026-10-04; no new cycle or global authority promotion found. All witness IDs change through new
nullable keyed fields. Actual native PostgreSQL explanation/JSON wire parity, replay corruption
controls, reviewed schema/adapters and the full gate/hygiene remain required. No MCP explanation
interface is added: the new explanation control uses the existing native ProofReference service.


Selected-domain/schema contract checks **passed**, 2026-10-04:
`cargo check --release -p lctx-model -p cpg-extract -p cpg-core -p lctx --tests`, first after
proof-field integration, then after native FinalReference inspection (`lctx-enrichment-selected-domain-contract-check.log`,
`lctx-enrichment-final-reference-contract-check.log`). These are compile receipts, not runtime
closure. FinalReference/Ruff binding correspondence and initializer controls are integrated at
`937e918d`; scope-exact capture remains unchanged. Focused runtime controls and matching native
adapters are being rebuilt against this model; current hygiene/full acceptance remain open.


The updated focused native/core run **failed**, 2026-10-04: 34 controls, 31 passed and three
failed, no timeouts (`lctx-enrichment-scoped-closure-repairs-final.log`). All three read-channel
controls and six retired-read controls passed, including ChoiceA initializer, missing/foreign
FinalReference and strict builtin/Flow refusal. All six source-call controls passed, including
selected closure's 14 adversaries, and all four real PostgreSQL Summary publication controls
passed. The new witness corruption fixture incorrectly requested an absent literal argument
FlowValueObservation; it now uses existing native `inner(value)`. Selection fixture closure
included unexecuted diagnostic families; it now seeds exact publication owners and names missing
canonical dependencies, with a cheap model-closure control. Their reruns are pending.
`UV_LOCK_TIMEOUT=3600 just native-adapter-ready` **passed**, matching current model after
selected-proof schema and FinalReference input changes (`lctx-enrichment-selected-domain-native-adapter.log`).
Actual PostgreSQL serving/explanation and direct Behavioral Structural qualification are queued;
this remains a composite bounded repair receipt, not full Q1 acceptance.


Current PostgreSQL serving packets **failed**, 2026-10-04: nine controls, eight passed and
one constructor-to-reader association-count assertion failed (`lctx-enrichment-selected-domain-serving-packets.log`).
The new native Summary explanation **passed** with exact enumeration/support and member/signature
edges and JSON wire round-trip. Behavioral packets, default/formal/context/set hydration, budget
refusal/resumption, original native-use evidence and typed role/generated-slot proof controls passed.
The rich analytical packet returns four associations where the unchanged fixture requires two;
producer/signature-role identity is under investigation, not silently deduplicated or relabelled.
CLI describe reports model `ece1e6a8242aa3b7bafa039ed0aa07b7c5f6cd601db55626ef3770b76f9321a6`,
878 relations: the previous accepted schema differs only in the two added SummaryWitness fields.
The direct unchanged describe assertion **failed** as expected before deliberate migration
acceptance (`lctx-enrichment-selected-domain-describe-snapshot.log`); snapshot refresh remains
pending final source stability. Full hygiene/functional acceptance is still open.


The selection/witness fixture rerun **passed**, 2026-10-04:
`cargo nextest run --release -p cpg-extract --test transfer_composition -p cpg-core --test catalog_selection --failure-output immediate`,
16 controls, zero skips (`lctx-enrichment-selection-witness-fixture-rerun.log`). Both actual
PostgreSQL selection journeys and the canonical executed-owner dependency-closure control passed;
all 13 composition controls passed, including selected enumeration/support corruption and exact
global-versus-declared witness lineage. Production embedding contracts and Flow authority are
unchanged. The fresh Behavioral Structural direct journey and association-purpose diagnostic
remain in progress; this bounded pass does not establish the full functional or hygiene gate.


The current `UV_LOCK_TIMEOUT=3600 LCTX_DATABASE_CONFIG=<owned-empty-store> just hygiene`
**failed** at ADR lint, 2026-10-04 (`lctx-enrichment-selected-domain-hygiene.log`): §15.6's
decision line omitted ADR-0123, and the generated ADR index is stale. The owning section now
records the exact selected-domain witness/explanation contract and decision link; accepted ADRs
are unchanged. The empty scoped database was reset to the current model; default staging and
registrations were untouched. Remaining hygiene checks were not_run after this prerequisite
failure; generated index refresh and named check resumption remain pending.


The analytical packet's exact evidence-purpose qualification **passed**, 2026-10-04:
`cargo nextest run --release -p lctx --test serving_packets -E 'test(analytical_enrichment_serves_real_comparison_context_and_non_call_references)' --failure-output immediate`,
87.260s (`lctx-enrichment-association-purpose-acceptance.log`). The prior unchanged count
assertion failed with four rows; actual native diagnostics established two logical timeout/title
associations, each carrying distinct DeclaredField and NativeField option evidence. Strengthened
typed checks preserve exact association/reader/parameter/slot identities, synthesized native
signature authority, original declared defaults, native default Unknown, source Known/runtime
Unknown and both evidence purposes. Actual PostgreSQL packets and real MCP stdio passed; no
production deduplication was introduced.

The direct Behavioral Structural control **failed**, 2026-10-04:
`cargo test --release -p cpg-core --test structural structural_candidates_paths_and_usage_publish_in_behavioral -- --exact --nocapture`,
242.78s (`lctx-enrichment-structural-behavioral-direct.log`). All stages and seal validation
completed, then the required named reaching-definition handoff positive was absent. The assertion
is retained and a bounded producer/admission repair is in progress; no new harness timeout is
justified by this failing receipt.

Current hygiene is **composite incomplete**, 2026-10-04. Fixture parsing, gold, rule scan/test,
Ruff, Python types and dependency policy **passed** after the initial ADR failure. Documentation
publication still refuses the stale generated ADR index. Full workspace/all-target `just clippy`
initially **failed** on a missing allowance reason in the shared selection fixture, then **passed**
after `93278a8a` (`lctx-enrichment-selected-domain-clippy-rerun.log`). Scoped current-model
`lctx store check` **passed**, zero generations/findings (`lctx-enrichment-store-check-selected-domain.log`);
default staging is untouched. The operator authorized one additional final formatting and ADR-index
refresh, to run after the remaining functional repair and before final schema/artifact qualification.


The current hygiene sequence is **composite passed**, 2026-10-04, before the remaining handoff
repair/final formatting: the initial `just hygiene` and named repairs/resumption cover every
check. Authorized `just adr index` added ADR-0123; `just adr-lint` and `just docs-check` passed
(290 canonical pages, zero offline link errors). The first final named `just store-check`
omitted the scoped environment and correctly refused the preserved default old-model staging
(`lctx-enrichment-selected-domain-hygiene-final-named.log`); this read-only refusal changed
nothing. The corrected `LCTX_DATABASE_CONFIG=<owned-empty-store> just store-check` **passed**,
zero generations/findings (`lctx-enrichment-selected-domain-hygiene-store-scoped.log`). The
standalone CLI still describes model `ece1e6a8242aa3b7bafa039ed0aa07b7c5f6cd601db55626ef3770b76f9321a6`,
878 relations. These repairs are not an initially clean run or a current full functional pass.
Two completed qualification worktrees were removed after clean status and integration checks;
the handoff worktree remains for the open finding.


The actual named-handoff diagnostic **failed**, 2026-10-04, 237.59s
(`lctx-enrichment-structural-named-handoff-diagnostic.log`; parsed original premises in
`lctx-enrichment-named-handoff-premises.json`). The generated guide block has exact complete
native singleton inventories/supports and source-covering cumulative Input coverage, but no
separate artifact-scoped coverage row. `handoffs::named_definition` incorrectly required only
Artifact coverage while its own qualification contract already admitted Input/Artifact/Module.
`61ee9c0f` factors the same exact source-covering predicate into both checks, retaining identical
native run/provider/context, Complete-or-Partial Flow status and mandatory complete per-use
closure, support, geometry and condition checks. No relation/column/codebook/input declarations
change; no global closure promotion is introduced.
`cargo test --release -p lctx-model --test domain_handoffs` **passed**, three seeded admission
controls (`lctx-enrichment-handoff-source-coverage-unit.log`): matching source scopes/statuses,
foreign scopes, missing/pruned inventories and missing/foreign same-run native pairs. These are
seeded unit controls, not a live provider qualification. The original actual PostgreSQL named
handoff positive and all corruption controls remain required. Authorized final `just fmt` is
in progress before rebuilding matching source-fingerprinted artifacts and final qualification.


The additional operator-authorized final `UV_LOCK_TIMEOUT=3600 just fmt` and `just adr index`
**passed**, 2026-10-04 (`948b2a88`, `439c7d1f`). Matching `just test-cli-build
native-adapter-ready` **passed** (`lctx-enrichment-formatted-final-artifacts.log`). The reviewed
schema migration and `INSTA_UPDATE=no` describe assertion **passed** (`b0256115`): current model
`8f8ec7cdbf9ee0e21d61f84ac6698186320cb9b9d1f2e5c1ac543c33b77a6f86`, 878 relations;
two nullable keyed selected-enumeration witness fields and fourteen invariant input additions,
without relation removals, existing-field changes or codebook changes. Review receipts are
`lctx-enrichment-formatted-final-schema-review.json` and
`lctx-enrichment-formatted-final-snapshot-reviewed.diff` under `/home/paul/.cache/`.
Only the owned empty qualification database was reset to this final model. Final scoped
`just deps store-check` **passed**, zero generations/findings
(`lctx-enrichment-formatted-repair-final-policy-store.log`).

Final `UV_LOCK_TIMEOUT=3600 just clippy` **passed**, workspace/all targets with warnings denied
(`lctx-enrichment-formatted-final-clippy.log`). Final scoped `just hygiene` **failed only at an
unfinished parallel-review documentation link**, after agents, ADRs, fixtures, gold, rules,
Ruff and Pyrefly passed (`lctx-enrichment-formatted-repair-final-hygiene.log`). The operator
explicitly excludes findings related to that concurrent review; its untracked evidence and
advisory catalog changes are preserved. Final deps/Clippy/scoped store-check separately passed.
This is a **composite scoped hygiene pass with the parallel-review docs failure excluded**,
not a claim that `just hygiene` exited zero. The actual repaired Behavioral Structural positive
and same-tree full functional gate remain pending.


The final actual Behavioral Structural positive **failed**, 2026-10-04, after 240.88s
(`lctx-enrichment-structural-source-coverage-final.log`). The source-coverage correction is
necessary but insufficient: ty's exact reaching predicate includes `IsNonTerminalCall`, while
FlowUseObservation is deliberately a structural observation qualified by Always. The current
helper requires use-condition implication of the reaching condition, so it still refuses that
positive. The predicate remains intact; complete native inventory alone does not prove it true.
The concrete next investigation is exact same-run native execution-region evidence and its
explicit proof lineage. The full `UV_LOCK_TIMEOUT=3600 NEXTEST_TEST_THREADS=8 just test-all`
is also running to expose any independent residual failures. Q1 remains open.


`5d5a4c0b` integrates the bounded ADR-0122 implication repair, 2026-10-04. Direct native
Use-to-Reaching implication preserves absent region refs; otherwise canonical ownership and
same-run exact containing native region/support, including its native classification pair, are
required. Use-and-Region must still imply reaching, without false-domain promotion or dropping
call-return predicates. Two optional proof refs are added to each Named handoff/setup variant;
this is a schema migration, with unchanged codebooks and no new relation. Alias publication
composes the effective domain with call/Local qualification and unions all original premise
assumption bases. Synthesis hydrates five canonical inputs, excluding Flow region/support in
Catalog. Existing canonical stored lineage is retained; no new serving explanation endpoint or
derivation expansion is claimed.
`1107a957` strengthens the actual fixture with an intervening call after alias assignment and
requires PostgreSQL publication of region lineage and the exact conditional alias domain.
Six seeded named-admission controls **passed** (`lctx-enrichment-native-read-region-unit.log`).
Two shared qualifier controls are in progress. The bounded independent review found two concrete
resource issues: binary admission failures were wrapped as Invalid instead of Resource, and the
derived domain's reservation was dropped before its consumer finished. Narrow repairs are in
progress; current source runtime qualification and refreshed artifacts/snapshot remain pending.
The superseded diagnostic `just test-all` was interrupted with scoped SIGINT/exit130 while still
compiling, before any tests launched (`lctx-enrichment-test-all-repaired-final.log`); its tests,
Python/oracles and doctests are **not_run**, not a passing receipt.
