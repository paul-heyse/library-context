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
| [F02](../design_review/reviews/design_review_code-facts-analytical-architecture_2026-10-03.md#F02) | Current architecture/extraction guidance, A1 | **Scheduled / open**; source-aligned owners and passing docs check |
| [O01](../design_review/reviews/design_review_code-facts-analytical-architecture_2026-10-03.md#O01) | Model callable/serving and native trace adapter, C1/C2 | **Scheduled / open**; role comparison answer and identity-qualified chosen/candidate association; ambiguous origin remains explicit |
| [O02](../design_review/reviews/design_review_code-facts-analytical-architecture_2026-10-03.md#O02) | Model Selection/catalog, C3 | **Scheduled / open**; hydrated predicate and exact source-field packet, runtime unknown preserved |
| [O03](../design_review/reviews/design_review_code-facts-analytical-architecture_2026-10-03.md#O03) | Flow/Entry/Local/Structural, A2/G0–G3 | **Scheduled / conditional**; qualification correction and complete inventory/explanation required; native usefulness evidence admits proof branch or records Deferred proof with trigger |
| [O04](../design_review/reviews/design_review_code-facts-analytical-architecture_2026-10-03.md#O04) | Model C1 evidence/serving, C4 | **Scheduled / open**; exact diagnostic-use correlation with unassociated remainder and no execution verdict |
| [O05](../design_review/reviews/design_review_code-facts-analytical-architecture_2026-10-03.md#O05) | Model import route/serving, C5 | **Scheduled / open**; bounded supported route, cycle/coverage/candidate controls |
| [O06](../design_review/reviews/design_review_code-facts-analytical-architecture_2026-10-03.md#O06) | Model types/extractor/serving, C6/C7 | **Scheduled / conditional**; existing Expected interpretation required; argument extension only after demonstrated useful retained trace |
| [O07](../design_review/reviews/design_review_code-facts-analytical-architecture_2026-10-03.md#O07) | Model references and ty test harness, R1/R2 | **Scheduled / open**; scoped incoming-reference product and attributed oracle discrepancies; production ty search deferred on resource non-fit |
| [O08](../design_review/reviews/design_review_code-facts-analytical-architecture_2026-10-03.md#O08) | Analytics tests/model bounds, K1/K2 | **Scheduled / open**; independent consequence/iceberg controls, production charged kernel retained |

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
composition are implemented in the working tree; focused Structural tests passed (4 controls),
with expanded stop/frontier and qualification controls pending. A1 owner guidance is corrected,
with final documentation validation pending. K1 dev-only dependencies are pinned at `80e994da`;
G1 and API consumers are being implemented on isolated branches. Q1/full gates remain **not_run**.

**G0 passed**, 2026-10-04: `cargo test --release -p cpg-flow --test flow_shapes
native_guarded_origin_search -- --nocapture` on the flow branch. The actual fixture
`fixtures/python/guarded_origin_inventory/cases.py` retained ten identity-return reads, six with two
alternatives, plus an incomplete loop join and native branch-local/early-return singletons. No
multi-candidate native Value/region condition excluded every competitor. **G2/G3 Deferred**:
reopen on an actual multi-candidate case with an attributed native Value condition Q whose
conjunction excludes every competing candidate, under complete per-use inventory. G1 and A2
remain required; this decision does not claim guarded proof implementation.

Plan authoring is complete only when document contracts,
links and dispositions agree and the focused documentation check passes; this does not execute the
production scope. Next implementation action: A0/A1/A2, alongside G0 evidence and independent C1/C3
existing-fact work. Authoring verification is recorded below, separately from eventual Q1 receipts.

### Authoring receipt

**passed**, 2026-10-04: `just docs-check`, 288 canonical pages, zero offline link errors.
Focused independent contract/library advice was integrated: unconditional A2, query-local overload
ordinals with unavailable specialization when not retained, source boundaries for unattached uses,
and accurate FCA cap/resource semantics. This is documentation/source assessment, not production
acceptance. Production compile/tests, `just test-all`, `just hygiene`, new runtime probes and
real-library/operator actions are **not_run** in this documentation-only authoring task. The
concurrent advisory `docs/library-utilization.jsonl` refresh is preserved outside this scoped change.

Execution receipt update, 2026-10-04 (**Implemented / focused Tested**, not Q1):
`cargo test --release -p lctx-model --test structural` **passed** (five controls), and
`cargo test --release -p lctx-model --lib singleton_conditional_basis_is_not_promoted_or_erased`
**passed**. The first `cargo test --release -p cpg-core --test structural` **failed** at fixture
model registration: the hand-built relation set omitted a referenced SummaryExceptionOutcome.
The fixture now uses the canonical model inventory; its rerun is pending. This failure has not
exercised actual publication. C1/C3/C6/R1 initial consumer slice is integrated at `393f904a`;
its three focused model controls passed on the worker baseline, service qualification pending.
C7 is **enabled**: the actual retained Expected probe supplies useful `list[int]` context for
`consume([])` and a retained `str` hint for a failed overloaded call. The initial absence assumption
failed and is retained in `/home/paul/.cache/lctx-api-expected-trace.log`; corrected qualification
and exact argument-location replay remain pending. Expected is not successful applicability.
C2 published observation fork `63cda076956013cd0bd1d0d05c785f747fe6adc0` preserves the upstream
parent and embedded Ruff premise. Its isolated supplier compile **passed** with the existing
allocative patch; repository canonical observations/association/packets are still in progress.
Full functional/hygiene, same-tree adapter readiness and Q1 remain **not_run**.
