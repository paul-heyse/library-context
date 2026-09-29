# Semantic-model cutover, phase-0 exit (X0) — assembled design/target review

## 1. Scope, outcome and coverage

| Field | Value |
|---|---|
| Subject | Range `7aa7a30..c4c7cba` on `main`, clean tree (17 commits, R1 → E1). `crates/lctx-model/src/domain/*` (model, record, stages, memory, batching, charged, resources, calls, declarations, occurrence_owner, conditions incl. stability/rebase, composition, admission, obligation, transfer, value, attribution, types); `crates/lctx-postgres/src/generations/{mod,ddl,codec}.rs` and `control.sql`; `crates/cpg-core/src/model_runtime.rs`; `crates/cpg-extract/src/{capture,typed_syntax,typed_stages}.rs`; the tests of each crate. Dormant legacy machinery (`lctx-model::{decl,id,legacy,ddl}`, `lctx-postgres::store`, `cpg-core::{store_read,parity}`) was read only to confirm that nothing in the new path consumes it |
| Standard | [Core 3.0 and template](../design_principles/core/design-principles.md), [code-intelligence 1.1](../design_principles/profiles/code-intelligence/principles.md) with [review additions](../design_principles/profiles/code-intelligence/review.md), [repository binding](../design_principles/binding/library-context.md) |
| Tier · purpose | **Design · target** (binding: integrated stage/phase boundary). Judged against [DESIGN §15](../../design/sections/semantic-model.md), ADR-0085–0089, the [cutover plan](../../plans/semantic-model-cutover-plan_2026-09-29.md) §4.1.1 (X0), §4.2 and §8, and the best contracts for P1–P5 |
| Reviewer · date | Fresh-context `design-reviewer` subagent (not the author), 2026-09-29 |
| Maturity and outcome | P0 of a hard layered cutover. The decision should say whether the assembled typed model, stage/sink contract, facts frontier, resource accounting and stage-bound subset are the right contracts for P1 and P2 to build on, and which findings must close first |
| Supported scope | Typed domain authority and generated lowerings; stage schedule, execution, handoffs and contributions; `MemoryGeneration` and the PostgreSQL conformance sink; facts-frontier preflight and admission (not yet consumed by the store); budgeted batches, writers and validator state; declarations, call-site facts, owner rule, witnessed substitution and whole-call composition; obligation/verdict policy; the capture and syntax stages. CI fact families: artifacts, syntax occurrences and identifier observations (produced); calls, declarations, flow, types, lexical, documents, deployment, conditions, transfers (contract fixtures only). Served answers: none |
| Exclusions | P1 lifecycle/session/CLI, P2 producers (A0–A16), P3–P5; integrated gates and pilots (binding: acceptance timing; plan Q) |
| Expected changes | The seven X0 scenarios (§5): P2 multi-provider producers; P3 equivalence and SQL views; P4 SCC composition over `compose_site`; a contract change mid-cutover; the facts frontier refusing a higher layer; budget exhaustion on a large input; an Unavailable or NotRequested provider |
| Baseline | [Core review](design_review_cutover-core_2026-09-29.md) (Revise; C01–C14), bounded [C4/C5 composition review](design_review_c4-c5-composition_2026-09-29.md) (Accept scoped after re-inspection), bounded [resource review](design_review_resource-slices_2026-09-29.md) (Revise; corrections in `c1b9344`) |
| Method and coverage | Source read at HEAD for every file above, §15, ADR-0085–0089, plan §3, §4.1.1, §4.2, §8, the E1 evidence folder, §3.9 of the [behavior model](../../design/sections/behavior-model.md) (verdict/modality semantics), and the pinned Pyrefly fork `a07b7ba` (`module/finder.rs`, `pyrefly_config/src/base.rs`). One focused suite executed (§10). PostgreSQL and `cpg-extract` suites: historical author receipts. Not examined: dormant P3–P5 modules, Python packages |

This review is evidence, not a status register. Current disposition belongs in plan §8 (binding §4); §11 proposes the rows.

## 2. Responsibilities, dependencies and semantic ownership

| Component | Coherent responsibility and hidden decisions | Consumer contract | Direction | Expected reason for change |
|---|---|---|---|---|
| `lctx-model::domain` model/record/identity + `lctx-model-macros` | Domain structs/sums are the authority; derive generates key, identity, Arrow schema/codec, `HeapSize`, invariants hook; `ValidatedModel` privately constructed; model digest over declarations and captured sources (`build.rs`) | `Relation::of::<R>()`, `ValidatedModel::validate`, `Batch<R>` | Pure; Arrow, serde_arrow, blake3, biodivine, petgraph | A relation, field, code or invariant |
| `domain::stages`, `memory` | Stage table as sole writer authority; schedule digest; attempt execution, permits, handoffs, contributions; `StageSink`; `StageOutput` streaming; in-memory generation with the store's validation | `Schedule::build/execute`, `StageAccess`, `StageOutput`, `StageSink::copy` | Pure; sinks implement the trait | A stage, profile, provider or sink |
| `domain::admission` | Facts frontier: family table (grain, profiles, required), preflight before effects, expected coverage, outcome/coverage reconciliation, `FactsAdmission` | `FrontierContract::facts`, `preflight`, `AdmissionCheck::finish` | Pure | A family, profile or frontier |
| `domain::{resources,batching,charged}` | Neutral pool/reservation, transfer limits, fail-stop writers, charged validator state | `ResourceBudget`, `BatchWriter`, `StateCharge`, `Charged*` | Pure; `cpg-core` adapts DataFusion's pool | A limit or holder |
| `domain::{calls,declarations,occurrence_owner}` | Provider symbols/modules, signatures, binder, receiver classification, site facts and five policies, normalization; declaration links; owner rule | `bind`, `classify_receiver`, `SiteTargets::admitted`, `normalize_site`, `OwnerTable` | Pure | A provider report shape, policy or equivalence |
| `domain::{value,conditions,transfer,composition,obligation}` | Places/paths, atoms/BDD kernel, stability witness, guard substitution, transfer keys/alternatives/influence/selection, whole-call composition with stored equation, obligation codebook/priority/verdict/discharge | `compose_call/compose_site`, `merge`, `verdict`, `discharge` | Pure | P4 engine needs, a predicate class, verdict rules |
| `lctx-postgres::generations` | Generation schema lowering (SeaQuery), control catalog, COPY (pgpq), sealing/validation/publication/selection/retire, leases, conformance attempts | `GenerationStore`, `GenerationAttempt: StageSink`, `GenerationLease` | Depends on `lctx-model`; SQLx, pgpq | P1.5–P1.10 lifecycle |
| `cpg-core::model_runtime` | DataFusion runtime as the attempt pool; per-stage fresh catalogs gated by read permits; read-only SQL | `AttemptRuntime`, `StageSession` | DataFusion stays here | P1.10 provider sessions |
| `cpg-extract::{capture,typed_syntax,typed_stages}` | Frozen captured input; pinned Pyrefly/Ruff syntax provider with admission and traversal bounds; capture and syntax stages | `CapturedInput`, `extract`, `run_capture`, `run_syntax` | Depends on the model; provider indices stay local | A0 framework, A2 capture, A4 producer |

Compile-time direction is sound: `lctx-model` ← `lctx-postgres` ← (tests) and `lctx-model` ← `cpg-core`, `cpg-extract`. No new-path module imports `decl`, `id`, `legacy` or `ddl` (searched `crates/`; only `lctx-postgres::store`, `cpg-core::{store_read,parity}`, `cpg-schema` and their tests do, all P1.2 deletions).

| Concept | Semantic authority | Update boundary | Derived forms and consumers | Second copy? |
|---|---|---|---|---|
| Relation contract, codes | Domain struct/sum + derive | Model digest → installation refuses (`Error::Contract`) → rebuild | Arrow schema/codec, PG DDL/CHECK/FK, `HeapSize`, content digest | None; but no reviewable snapshot (F06) |
| Producing stage | `Stage.outputs` (typed `RelationUse`) | Schedule digest | Scheduler, permits, sink receipts, preflight | None (C03 closed) |
| Facts frontier (relations and families) | `facts_relations()`, `FACTS_REQUIREMENTS` | Contract digest | Preflight, admission | **Store DDL/receipts/leases use the whole model (F02)** |
| Call admission | `CallPolicy::admits` over `SiteFacts` | One predicate | Composition frames (booleans, composition F04) | SQL rendering deleted; P3 decides (§9) |
| Caller | `OwnerTable` / `owner_of` oracle | — | Composition `site_owner` (caller-supplied, composition F04) | None |
| Verdict | `obligation::verdict` | — | P4 producers (none yet) | **Modality has no authority in it (F01)** |
| Transfer identity | `TransferKey` (condition excluded) | — | Alternatives, selections, composition steps | None (C08 re-confirmed) |
| Budgets and limits | `resources.rs`, `SyntaxLimits`, `Budget` meters | — | Writers, COPY, reads, CHECK, kernels | None |

**CI fact and fidelity table** (produced = implemented producer; fixture = contract fixture only).

| Fact family / relation | Provider and revision | Fidelity | Coverage and unknowns | Identity | Consumers |
|---|---|---|---|---|---|
| Artifacts: `InputRevision`, `SourceArtifact`, `ArtifactChunk` (produced) | capture (own code) | extracted bytes | E1 capture stage has no provider, so no Artifacts coverage (subset refused by preflight, intended) | Manifest/content digests, 1 MiB chunks | Syntax stage, admission |
| Syntax: `Occurrence`, `SyntaxObservation`/`Support` (produced) | `pyrefly-retained-ruff-ast` `a07b7ba;ruff=0.0.11`, build digest over `Cargo.lock` + `typed_syntax.rs` | extracted | Per artifact: Partial(`OutsideProviderModel`: identifier subset), Partial(`SyntaxError`), Unavailable(`UndecodableSource`/`ResourceRefused`), Partial(`ResourceRefused`) with ancestor-closed prefix | (source, span, kind, structural path) | Owner rule, atoms, attachment (P2) |
| Calls, declarations (fixture) | Pysa export (A9/A10) | resolved; modality, channel, phase, receiver | `CallResolution.complete` provider-attributed; unresolved alternative explicit; `Receiver::Unknown` | Qualified; membership digest | `SiteTargets`, binder, composition |
| Flow, stability (fixture) | ty 0.0.14 (A14) | extracted; `ReachingDefinition::{Bound,Unbound,Nested}` | Flow coverage required by witnesses | Occurrence-backed | Witness, substitution |
| Conditions/atoms (produced trivially; fixture otherwise) | kernel | derived | Kernel limits → obligations | (evaluation, context, predicate, operand); Merkle nodes | Transfers, verdicts |
| Transfers, selections, compositions (analysis; fixture) | P4 | derived/composed | Obligations for unsupported cases | `TransferKey` without condition | Verdict (P4), explanations (P5) |
| Coverage: `ProviderCoverage`, `CoverageScope` | each provider | — | Complete/Partial/Unavailable/NotRequested/Failed; NotRequested names an arbitrary provider (F05) | (scope, provider, context, family, run) | Admission, verdict |

## 3. Contracts, constraints and testing boundaries

| Contract | Inputs/outputs and expectation | Invariants and enforcement | Effects/failure | Substitution | Isolated verification |
|---|---|---|---|---|---|
| `ValidatedModel` | Relations → validated model + digest | Membership, identifiers, references, subtypes, companions, sums, derivations, invariant inputs (`model.rs:45-169`); private construction | Pure | One lowering per sink | `domain`, doctests (compile-fail pairs) |
| `Batch<R>` / `BatchWriter<R>` | Rows → canonical, deduplicated, reserved batch | Row validate, conflict refusal, reservation before encode; poisoned after first error (`batching.rs:36-39`) | Typed `Resource`/`Limit`/`Conflict` | Any `ResourcePool` | `domain_resources` |
| Schedule / execution | Stages → ordered schedule, permits | Membership, one writer per profile, readers after writers, contributions before writer, no self-contribution, coverage iff provider (`stages.rs:51-133`); dropped/failed stage poisons attempt | Fail-stop | Memory or PG sink | `domain_stages`, `domain_memory` |
| `StageSink` | Permit + batch → stored | Attempt identity and model digest checked by both sinks | Errors are `ModelError`; store classes collapse to `Codec` (F07) | `MemoryGeneration` digest equals PG (Tested, D0/E1) | Both sinks |
| Facts frontier | Schedule, sealed inputs/artifacts/coverage → `FactsAdmission` | Preflight before effects; exact expected rows; outcome reconciliation; required families (`admission.rs:109-311`); compile-fail construction | Typed `Frontier` refusal | Profile-parameterized | `domain_admission` (store-free) |
| Composition | Caller/callee alternatives, frame → composed alternatives or obligations | Total ports, entry vs variable, identity-only paths, witnessed guards, stored equation (`composition.rs:194-461`) | Obligations, never unconditional flows; caller-supplied admission booleans (composition F04) | — | `domain_composition` (twenty-plus pre-written answers) |
| Verdict | Condition, open obligations, coverage, approximation → five verdicts | Budget/approximation/refusals never proofs; NotRequested → NotAnalyzed | **No modality input (F01)** | — | `domain_verdicts` |
| Generation store | Validated model → schema per generation; lifecycle | Digest-checked state at every operation; CHECK/FK; receipts; conformance unselectable | Transactions acknowledged; COPY abort | — | PG suites (historical receipts) |
| Syntax provider | Captured input → `SyntaxFacts` | Admission before handles; ambient Pyrefly env refused; explicit config; frozen bytes verified | Refusals map to coverage (`typed_syntax.rs:49-68`) | — | `typed_limits`, `typed_conformance` |

**Absence states.** Coverage distinguishes Complete, Partial, Unavailable, NotRequested, Failed and (admission) NoScope. Calls distinguish resolved, unresolved-with-reason and incomplete resolution; receivers None/Bound/Unknown. Reaching distinguishes Bound, Unbound and Nested. Composition distinguishes Transfer, Disjoint, Subsumed and typed obligations. Gaps: a resolved callee without a summary and one whose complete summary has no flow from the port are the same empty result (F03); a NotRequested row has no "no provider" state (F05); a facts generation cannot tell "not in this frontier" from "empty" on read (F02).

## 4. Composition and execution

| Stage / capability | Inputs → outputs | Owner | Reuse boundary | Policy vs orchestration | Effects and publication | Limits and determinism |
|---|---|---|---|---|---|---|
| capture | Frozen tree → revision, origin, artifacts, chunks | `typed_stages::run_capture` | None (fresh recomputation) | No policy | Chunks streamed through a reserved writer; bytes rechecked | Deterministic by relocation (Tested) |
| syntax | Captured input → occurrences, observations, coverage | `typed_syntax::extract` on a 512 MiB-stack thread, then `run_syntax` | None | Coverage outcome from rows (`SyntaxFacts::outcome`) | **Whole provider output held as batches before the stage writes** (scenario 6) | Node/depth/source/callback limits; typed refusal |
| sink | Stage batches → memory or PG staging | `StageSink` | — | — | Exact written = expected outputs before seal | Transfer 4096 rows / 8 MiB; 64 MiB row |
| validation | Sealed contents → content digest, invariant receipts | Model invariants, run by both sinks | — | — | Receipts bound to model/physical digests | Charged state; linear type closure |
| admission (not yet wired to the store) | Receipt + coverage → `FactsAdmission` | `AdmissionCheck` | — | Frontier table is the policy | P1.7 consumes | Order-independent (Tested) |

**CI analysis record.**

| Operator | Question | Projection / inputs | Method | Exact / conservative / heuristic, model | Budgets and partial results | Output and evidence linkage |
|---|---|---|---|---|---|---|
| `site_facts` + `CallPolicy` | Which targets a consumer sees | Every resolution at one site (`SiteTargets`) | Phase-group uniqueness, completeness | Exact over provider-attributed facts; equivalence per provider symbol until P3 | 4096 alternatives, typed refusal | Target ids; policy decisions not stored (§9) |
| `compose_site` | What flows through a call | Caller alternatives × admitted targets × variants × callee alternatives | Port map, identity path rule, witnessed substitution | May-analysis under §15.6 port model; modality bounded by premises | Kernel limits → obligations | Composed alternative + `CallCompositionStep` premises |
| `verdict` | Verdict and reason | Condition, obligations, coverage, approximation | Class-ordered priority | **Ignores modality (F01)** | Budget class → Unknown | Reason code |
| `OwnerTable` | Caller of each occurrence | All occurrences of the input | Sorted sweep = oracle | Exact | Charged; gap refusal | Owner ids (not yet stored) |
| Admission | Is this generation a facts frontier | Inputs, artifacts, coverage, receipt | Expected-row reconciliation | Exact | Charged | Availability per family |

## 5. Change and failure scenarios

| Scenario and trigger | Owning component | Contract change | Expected vs observed consumers | Independent edits / hidden knowledge / test setup | Evidence |
|---|---|---|---|---|---|
| **1. P2 multi-provider producers** (pyrefly, ty_flow, documents, deployment; shared vocabulary via contributions; A0 thread/channel) | Provider stages, `stages`, A0 | A stage declaration per provider; `StageOutput::push_batch` from a provider-side writer; contributions for vocabulary and coverage | Expected: one stage declaration + producer; sinks, schedule, admission unchanged. Observed: holds; one provider per stage fits because variation is carried by `ProviderSurface` (provider = pinned build). Gaps: a batched output that also receives contributions bypasses its writer's dedup (F04); contributions enter `StageOutput` only as rows, so batches crossing A0's channel need a contribution batch entry (resource F05, routed); handoffs keep a read relation (for example `Occurrence` for ty attachment) whole in memory, charged (O4); `acquire` must report Artifacts coverage and route coverage rows to the sole `ProviderCoverage` writer, which the P2 stage table does not show (O5) | No hidden provider knowledge crosses a boundary; provider indices stay local | Implemented (stage/sink); D1 fixture schedule; A0 Proposed |
| **2. P3 equivalence and SQL views** | `calls`, `lctx-postgres` view generation | `site_facts` over an equivalence (for example symbols sharing a `SymbolDeclaration` occurrence); policy views | Expected: one owner change in `calls`; views generated. Observed: owner clear; the SQL rendering and its equality matrix were deleted in C6 (re-expressed to P3), so P3 must choose a rendering route (§9). Sum relations share arm columns by name (`place_roots.declaration` for `Formal` and `Entry`), so SQL readers need the discriminant (O7) | None today; P3 decides | Implemented kernel; views Proposed |
| **3. P4 SCC composition over `compose_site`** | P4 engine over `composition`, `transfer::merge`, `obligation` | Engine supplies frames; SCC widening | Expected: engine composes existing kernels. Observed: admission/uniqueness/owner arrive as caller booleans (composition F04), recursive sites mint fresh atoms (F06), obligations lack subjects (F07), and **a delivered value into a resolved target with no callee alternatives yields nothing** because `SiteCall` has no callee summary status (F03). The engine would have to know that an empty branch list means "not yet summarized" | Hidden knowledge moves into the engine | Implemented kernels; engine Proposed |
| **4. Contract change mid-cutover** (new relation, appended code, changed invariant) | Domain struct + membership line (+ stage, + frontier row for a family) | Model digest changes; installation refuses; rebuild | Expected: one declaration. Observed: holds; DDL/codec/CHECK/`HeapSize` follow, both sinks run the same invariant, the frontier refuses a family missing from its table (`admission.rs:78-81`). Gaps: no reviewable schema/codebook snapshot, so a renumbered code or dropped column shows only as an opaque digest change (F06); the model digest also changes on any `lctx-model` source or `Cargo.lock` edit (O1, conservative) | None | Code read; `domain`, doctests |
| **5. Facts frontier refusing a higher layer** | `admission` (schedules); `lctx-postgres` (physical); P1.10/P1.11 readers | — | Expected: refusal, never empty tables (ADR-0087, §15.11). Observed for schedules: preflight refuses a `TransferKey` output, reads above the frontier and unrequested families before effects (Tested). **Not observed for stores/readers**: `ddl::generate`, `validate`, `publish` and `GenerationLease::visit` range over the whole model, so a facts generation will carry empty analysis tables that a lease, `lctx query` or `lctx_serving` (granted `SELECT ON ALL TABLES`) answers with zero rows (F02) | Two owners must agree on "what a facts generation contains" | Code read (`generations/mod.rs:91-92, 131-156, 169-187, 389-395`; `ddl.rs:7-11`) |
| **6. Budget exhaustion on a large input** | Every holder via `ResourceBudget` | — | Expected: typed refusal, reservations 0, nothing published. Observed: Measured on fastmcp 4.0.5 (257 files) at half the peak: `Resource` refusal in extraction, reservation 0. Provider-internal memory (≈326 MB over the 209 MB budget peak) is a named uncounted term. The E1 path holds the whole provider output before the stage writes, so peak reservation grows with total output rather than a channel window; A0 must stream (routed to input-validation F02). A refused module can still be parsed by Pyrefly as a transitive import (F08) | — | [E1 evidence](../evidence/2026-09-29_p0e-subset-envelope/README.md) (Measured) |
| **7. Unavailable or NotRequested provider** | Stage outcome, coverage, admission, verdict | — | Expected: distinct from producer to verdict. Observed: `reconcile` ties outcomes to rows; a required family entirely Unavailable is refused, a non-required one admits as `Unavailable`; NotRequested families are unscheduled and preflight refuses attempts; verdict maps Unavailable → Unknown(`NativeUnavailable`) and NotRequested → NotAnalyzed. Gaps: a NotRequested row must name some provider, and the D1 fixture names ty, which never ran in catalog (F05); `ProviderOutcome::NotRequested` is accepted at stage finish but never admissible (O2); `generation show` is P1.8 | — | `domain_admission`, `domain_verdicts` (Tested) |

**Code-intelligence journeys.** *Add a fact family:* append a `FactFamily` code, add one `FACTS_REQUIREMENTS` row (refused when absent), declare `family` on its assertion relations, and name it in a stage's coverage; preflight enforces writer/coverage agreement. Satisfied. *Upgrade an analyzer:* provider build digest and schedule digest change; content digests compare; no stage reuse exists to go stale. Satisfied at P0 (A0 widens the build digest, F11). *A module full of unresolved references:* unresolved alternatives, incomplete resolutions, Partial coverage and Unknown verdicts are distinct. Satisfied at contract level. *Trace a served claim:* not applicable until P5; derivation targets are typed and views generated (C13 contract side). *Evaluation run:* fixtures under `fixtures/python/`; no `.claude/skills` path in the subject (searched). Satisfied.

## 6. Correctness and fidelity gates

| Gate | Verdict | Own evidence or scope reason | Required action |
|---|---|---|---|
| G1 Authority | pass | One domain declaration per relation; stage table sole writer; invariants shared by memory and PG; limits single-sourced; frontier table single | — |
| G2 Semantic fidelity | **fail (contract level)** | F01: a Candidate or Potential flow with condition `true` is Established; a failed refutation is named by a Selection-class reason. Other distinctions (Entry/Formal, provider modules, Receiver Unknown, typed limits) pass | F01 before X0 |
| G3 Validity | pass | Private model and admission construction (compile-fail); batch validation; stored invariants and CHECKs; preflight before effects; digest-checked store states; poisoned writers. F04 refuses a valid generation (fail-closed, not an invalid admission) | — |
| G4 Hidden behavior | pass | `lctx-model` pure; Pyrefly config constructed; ambient Pyrefly variables refused (`typed_syntax.rs:389-394`) | — |
| G5 Consistency and recovery | pass (P0 scope) | Atomic publication; conformance unselectable; exhaustion publishes nothing (Measured); writer and stage fail-stop. P1 exposure: F02, F07, C01 races | Route F02/F07 to P1.7/P1.10 |
| G6 Transformation and reuse | pass | Memory digest equals PG (Tested); relocation determinism; composition equation re-derived at validation; no caches | — |
| G7 Truthful capability claims | pass | Plan §4.2 labels bounded receipts accurately; E1 is labelled a subset. Minor wording: F05 (plan's "no provider"), F08 (transitive loads undeclared) | Correct with F05/F08 |
| G8 Library leverage | pass | serde_arrow, SeaQuery, pgpq, DataFusion pool, biodivine, arrow-row, Ruff visitor used where they fit; bespoke code is domain semantics. One candidate: Pyrefly `replace-imports-with-any` (F08) | — |
| CI-G1 Fidelity | **fail (contract level; nothing published)** | F01 relabels a candidate/potential call path as established behavior (CI-02, CI-06; §3.9 "a behavior across a candidate or potential arc is at best unknown"). Composition F04 booleans remain routed | F01 before X0 |
| CI-G2 Evidence closure | n.a. | No served claims; premise targets typed, `derivation_acyclic` generated, views granted with the schema | P4/P5 |
| CI-G3 Evaluation integrity | pass | Independent fixtures and pre-written answers; no gold path | — |

## 7. Findings

Severity: **High** — fidelity of claimed behavior. **Medium** — contract gap with a concrete consequence in a planned scenario. **Low** — extension, diagnostics or evidence visibility. Only F01 blocks X0.

<a id="F01"></a>
### F01 · High (contract level) · `lctx-model::domain::obligation` — the verdict ignores modality, so C09 is not re-confirmed

- **Principles · gate:** CI-02, CI-06, DP-02, FP-04 · A2, G2, CI-G1.
- **Evidence.**
  - `VerdictInput` has condition, open obligations, coverage and approximation only (`obligation.rs:197-200`). `verdict` returns Established whenever the condition is `true` and nothing is open (`:219`).
  - Composition produces Candidate and Potential alternatives by design: two admitted targets give two Candidates, a non-Summary site gives Candidate, a Potential target stays Potential (`composition.rs:256-257`; `every_admitted_alternative_composes_separately_and_modality_follows_the_site`, `the_target_modality_bounds_the_composition`). `TransferKey` and the qualification carry that modality.
  - §3.9 (Tested semantics, `behavior-model.md:457-458`): "a behavior across a candidate or potential arc is at best `unknown`"; `override_dispatch` (`:441`).
  - A false condition under incomplete coverage is named `IncompleteDomain` (`:218`), a Selection-class reason ("a requirement's domain or context cannot be applied"). The core review's C09 correction asked for a coverage reason. The priority rationale lives only in test messages.
  - §15.8 lists the verdict inputs without modality (`semantic-model.md:341-342`); no production caller exists yet.
- **Consequence.** In P4, `f(x)` with two resolved targets yields two Candidate composed flows with condition `true`; each is graded Established and would render as `structurally_observed`, although the call may reach neither. Each P4 producer would otherwise demote non-definite modality itself: the restated-verdict-rule defect C09 was raised to prevent. A partial-coverage negative carries a reason that tells a consumer to fix a selection domain rather than coverage.
- **Correction** (owner `obligation`; a few dozen lines and a §15.8 line, no ADR: ADR-0085 delegates verdict contracts to P0.4).
  1. Add `modality: Modality` to `VerdictInput`. Definite is unchanged. Candidate or Potential never yields Established, Conditional or RefutedUnderModel: return Unknown with the first open reason, or a declared default when none is open (append one Resolution-class code, for example `NonDefiniteTarget`, coordinating the next free number with A3's appended codes; or reuse `OverrideDispatch` / `CallTransfer` if the operator prefers the §3.9 names). Strengthening stays with P4 discharge over a complete alternative set (ADR-0064), which produces its own Definite conclusion.
  2. Name a failed refutation under incomplete coverage with a coverage reason (append an Evidence-class `IncompleteCoverage`); keep `IncompleteDomain` for selection.
  3. One doc sentence on `priority` recording the order's rationale (budget stop first, resolution before evidence, unasked last).
  4. Amend §15.8's verdict sentence: "condition, modality, open obligations, coverage and approximation".
- **Closure evidence.** Pre-written `domain_verdicts` cases: Candidate + `true` + complete + exact → Unknown(reason); Potential + `true` → Unknown; Candidate + `false` + complete → Unknown, never Refuted; Definite unchanged (Established, Refuted); Definite + `false` + Partial → Unknown(`IncompleteCoverage`). One composition-to-verdict case: the two-target site's composed alternatives grade Unknown.

<a id="F02"></a>
### F02 · Medium · `lctx-postgres::generations` (P1.7), P1.10/P1.11 readers — the facts frontier has no physical consequence

- **Principles · gate:** CI-04, DP-19, DP-03, FP-05 · A2, G5 (P1), CI-G1.
- **Evidence.** `FrontierContract::facts` defines a facts generation's relations (`admission.rs:66-77`), and §15.11 says "A facts generation publishes only facts relations" (`semantic-model.md:423`). The store ignores it: `ddl::generate` creates a table for every model relation (`ddl.rs:7-11`), including `transfer_keys`, `call_compositions` and the other analysis relations; `validate` and `publish` require receipts for every model relation (`mod.rs:131-156, 169, 180`); publication grants `SELECT ON ALL TABLES` to `lctx_serving` (`:187`); `GenerationLease::visit::<R>` reads any model relation (`:389-395`). The physical digest is over the whole model (`ddl.rs:88-91`). P1.7's row (`begin(.., &FrontierContract, ..)`, `planned_outputs`) and P1.10/P1.11 do not state what a facts generation installs or what a reader does with a relation above its frontier.
- **Consequence.** After P1.7, `lctx query --generation <facts> "SELECT count(*) FROM transfer_keys"`, a lease `visit::<TransferKey>()` or a P5-era serving role returns zero rows: "no transfers" served from a generation that never analysed any, the empty-table answer ADR-0087 forbids. Each L1/L2 relation added in P3/P4 widens the set of such tables.
- **Correction** (P1.7 with P1.10/P1.11). Record the frontier's relation set in the registry and lower only it: DDL, relation receipts, invariants whose inputs are all inside it, publication grants and the physical digest per frontier contract. Readers (lease, `GenerationTable`, `query`) refuse a relation outside the recorded frontier with a typed `Frontier` error before any scan. Refusing at the reader over whole-model schemas is weaker, because raw SQL through the serving grant still sees empty tables.
- **Closure evidence.** A P1.7/P1.10 control: a facts generation's schema lists exactly the facts relations; `visit::<TransferKey>` and a `query` over `transfer_keys` refuse with `Frontier`, not zero rows.

<a id="F03"></a>
### F03 · Medium (P4 barrier) · `lctx-model::domain::composition` — an unsummarized resolved callee is indistinguishable from a callee with no flow

- **Principles · gate:** CI-04, DP-02, DP-12, FP-03 · A1, A3.
- **Evidence.** `compose_site` iterates `call.branches` for a resolved target (`composition.rs:340-342`). With an empty slice it emits nothing; `SiteCall` has no field saying whether the callee has a complete summary (`:330`). Only an unresolved destination turns a delivered value into an obligation (`:339`, `delivers`). §15.6: "A value crossing an unresolved or **unsummarized** call is not a transfer kind. It is an obligation" (`semantic-model.md:253`). No test composes through a resolved target with no alternatives.
- **Consequence.** In the P4 SCC fixpoint, callees in the same component start without summaries. A caller value passed to such a callee composes to nothing on the first pass, the same result as a completely summarized callee that drops the value, so no `CallTransfer` obligation holds the verdict at Unknown. The engine must carry that distinction itself (hidden knowledge), and a first-iteration result can read as "does not flow" (unknown served as absent).
- **Correction** (owner `composition`, with the P4 engine design and composition F04/F06/F07). Give `SiteCall` the callee summary status, for example `Summary::{Complete, Partial(ObligationKind), Absent}`. A value delivered into a resolved target whose summary is not Complete yields that obligation (`CallTransfer` when absent); an empty branch list is "no flow" only under Complete.
- **Closure evidence.** Known answers: delivered into a resolved, unsummarized callee → `CallTransfer`; complete summary without a flow from that port → no result; partial summary → its obligation.

<a id="F04"></a>
### F04 · Low · `lctx-model::domain::stages` — contributions into a batched output bypass its writer's dedup

- **Principles · gate:** DP-01, DP-16, FP-03 · A3.
- **Evidence.** `push_batch` marks an output batched and emits the batch without recording its ids in the output's `BatchWriter` (`stages.rs:361-371`). At finish, contributions are merged through that writer (`:413-419`), whose duplicate index lacks the batched rows. Nothing refuses a batched output that also has contributors.
- **Consequence.** When A0 moves providers to `push_batch` (resource F05) and a writer stage both emits a vocabulary row by batch and receives the equal row from a contributor (a shared `Evidence`, `Literal` or `Condition`), COPY hits the primary key and the attempt fails. It fails closed, but it refuses a legitimate generation and invites per-provider workarounds.
- **Correction.** Either refuse `push_batch` for an output with scheduled contributors (a clear error at `declare`), or register batched ids in the output writer's duplicate index so contributions deduplicate against them.
- **Closure evidence.** A `domain_stages` control: an equal row from a contributor and a batched writer is stored once; a conflicting payload is refused.

<a id="F05"></a>
### F05 · Low · `lctx-model::domain::attribution::ProviderCoverage`; B1 — a NotRequested row must name an arbitrary provider

- **Principles · gate:** DP-02, CI-01 · G7 (wording).
- **Evidence.** `provider` is a mandatory key field (`attribution.rs:116-125`); admission ignores it for unrequested families (`admission.rs:282`, `requested.then_some(row.provider)`). The D1 fixture attributes catalog Flow NotRequested rows to the ty provider (`domain_admission.rs:135`), which no catalog stage runs, while plan §4.2's D1 receipt says "(no provider)".
- **Consequence.** In P2's catalog profile, `assemble` must write a `Provider` row for ty (a provider that did not run) to satisfy the reference, and the row's identity and the content digest depend on that arbitrary choice. A reader sees a provider attributed with a statement it never made.
- **Correction** (B1). Make "not requested" the frontier's statement: either `provider: Option<Id<Provider>>`, `None` exactly for NotRequested (in `validate_coverage`), or attribute NotRequested rows to the assembling compiler as a declared provider. Align the plan text.
- **Closure evidence.** A B1/D1 control: a catalog generation contains no ty `Provider` row, and its NotRequested rows validate.

<a id="F06"></a>
### F06 · Low · `lctx-model` tests; P1.11 — the typed model has no reviewable schema or codebook snapshot

- **Principles · gate:** DP-24, DP-01 · G2 (evolution visibility).
- **Evidence.** AGENTS.md: "Schema contracts are insta snapshots" and "Codebooks are append-only" (`AGENTS.md:173-177`); binding §6 names the codebook snapshot as the cheap check. The only lctx-model snapshots cover the dormant `decl_sample` and `ids` machinery; no test snapshots `ValidatedModel` relations, fields, keys, references or `(code, label)` pairs, and no PG test snapshots generated DDL. T5's reserved `FactFamily` codes are comments.
- **Consequence.** In scenario 4, renumbering an `ObligationKind` or `PlaceRoot` code, dropping a column or changing a key shows only as an opaque model-digest change that forces a rebuild; the reviewer never sees a schema diff, and append-only holds by convention alone.
- **Correction.** One insta snapshot of the model description (relations, fields with roles and types, references, codes). Reuse P1.11's `model describe --format json` as its source instead of a new mechanism.
- **Closure evidence.** The snapshot exists under `INSTA_UPDATE=no`; a renumbered code produces a `.snap.new` diff.

<a id="F07"></a>
### F07 · Low · `StageSink` / `GenerationAttempt::copy`; P1.7/P1.10 — store failures lose their class

- **Principles · gate:** DP-21, DP-02 · G5 (diagnostics).
- **Evidence.** `GenerationAttempt::copy` maps every non-model store error (`Database`, `Commit` "outcome is unconfirmed", `CopyAbort`, `State`, `Contract`, `Busy`) to `ModelError::codec(other)` (`generations/mod.rs:336-338`); `capture_error` does the same for capture I/O (`typed_stages.rs:31-33`). `ModelError` has no infrastructure class.
- **Consequence.** The attempt fails correctly, but a producer, the CLI's exit code (P1.11) and T10/T13's `Interrupted`/`Lost` reporting cannot tell a transport loss or an unconfirmed commit from a codec defect without parsing text.
- **Correction.** A typed store/infrastructure variant (class plus detail), or a sink-associated error type on `StageSink`, preserved to the caller.
- **Closure evidence.** A P1.10 control: an injected transport failure during a stage COPY yields the typed class, and the attempt lists as `Interrupted`.

<a id="F08"></a>
### F08 · Low · `cpg-extract::typed_syntax`; A4 — a refused module is still reachable by Pyrefly's import loading

- **Principles · gate:** DP-20, DP-22, CI-10 · G7 (wording), G8 (candidate).
- **Evidence.** `extract_on_thread` admits sources before building handles (`typed_syntax.rs:412-422`) but keeps the whole captured root as the search path (`:396`); the transaction loads imports at `Require::Exports` (`:425-426`), which parses them. The resource review's F04 closure asked to "declare that transitive loads of a refused module stay provider-internal"; neither the module doc nor the E1 README says so ("bounds what Pyrefly is given"). The pinned fork has `replace-imports-with-any`, which makes the finder return `Ignored` without loading (`pyrefly/lib/module/finder.rs:475`, `pyrefly_config/src/base.rs:242-244`; the field is `pub(crate)`, settable through configuration deserialization or the fork patch). Interface-checked, 2026-09-29.
- **Consequence.** A 20 MiB generated module imported by an admitted one is parsed and solved outside the budget while its coverage says `Unavailable(ResourceRefused)`, "never given to Pyrefly".
- **Correction.** Now: state the transitive-load limit in `typed_syntax`'s doc and the E1 README. At A4: either keep that declared limit, or add refused modules to `replace-imports-with-any` and disclose the importers' `Any`-typed imports in Types coverage (A11).
- **Closure evidence.** Corrected text; if the option is adopted, an A4 control shows a refused module imported by an admitted one is never loaded.

**Foundation and supporting-rule verdicts.**

| Foundation / rule | Verdict | Basis |
|---|---|---|
| FP-01 Separation | satisfied | Pure model; PG effects, DataFusion and providers each in one owner |
| FP-02 Contracts | satisfied (F07 Low) | `StageSink`, `ResourcePool`, admission and composition contracts are narrow and substitutable (memory = PG digest) |
| FP-03 Composition | unresolved for scenario 3 | F03 with composition F04/F06/F07; scenarios 1 and 4 compose |
| FP-04 Authority | violated (bounded) | F01: the verdict owner lacks the modality rule; otherwise one authority per concept |
| FP-05 Explicit structure | satisfied for P0; unresolved for P1 | F02 frontier not enforced physically; F05 sentinel provider |
| FP-06 Local reasoning | satisfied | Every kernel store-free; the same validation in memory and PG |
| DP-02, CI-02, CI-06 | violated (F01) | — |
| DP-12, CI-08 | satisfied | Limits become obligations or typed refusals; refused traversal keeps a labelled prefix |
| DP-19 | satisfied (P0) | Atomic publication; fail-stop attempts |
| DP-20 | satisfied with residuals | All holders charged; provider-internal memory named; streaming at A0 |
| DP-21 | F07 (Low) | — |
| DP-24 | F06 (Low) | — |
| CI-04 | unresolved (F02, F03) | — |
| CI-10 | satisfied | Constructed config, captured root, ambient variables refused |

**Observations** (no finding).

- **O1.** `build.rs` digests every `lctx-model` and macro source plus `Cargo.lock` into the model digest (`model.rs:56`). Any edit, including dormant `decl`/`legacy` code or an unrelated lockfile bump, invalidates every generation. This is conservative and acceptable under ADR-0078/0086 current-only rebuilds, but it blurs §15.3's separation of model and producer identity. Revisit with ADR-0081's reuse trigger or when a full facts rebuild becomes expensive at Q.
- **O2.** `ProviderOutcome::NotRequested` is accepted by `StageAccess::finish` (`stages.rs:267-270`) and always refused by `reconcile` (`admission.rs:197`); plan §3.3 lists it as an outcome. Remove it, or refuse it at finish like `Failed`.
- **O3.** `StageAccess` borrows `&mut Execution`, so stages run strictly in sequence. That fits the plan (sequential Pyrefly sessions). Parallel provider stages would be a contract change, not a tuning knob.
- **O4.** Handoffs keep a read relation whole in memory until its last reader (for example `Occurrence` for ty attachment). It is charged and fails closed; ADR-0089's revisit trigger covers it; measure at Q.
- **O5.** The P2 stage table lists no contribution for `acquire`, but the D1 fixture has `acquire` covering Artifacts with a provider, and coverage rows must reach `assemble`, the sole `ProviderCoverage` writer, as contributions. Settle at B1.
- **O6.** ADR-0089's Consequences still says D0 is open. The binding keeps accepted ADRs immutable except lifecycle metadata; plan C3x's "ADR-0089 consequences brought current" should leave status to the plan.
- **O7.** Sum relations share arm columns by name. P3/P5 generated views should expose per-arm projections so SQL consumers cannot read `declaration` without the tag.

### Re-confirmation of source findings

| Source finding | X0 assessment (2026-09-29) | Evidence | Remaining route |
|---|---|---|---|
| Core C01 | Design implemented; race controls open | Schema per generation; `cleanup` drops schema and registry in one transaction; a reader's shared advisory lock makes retire `Busy`; lock order installation → selection → generation (`mod.rs:216-243, 251-263`) | P1.7/P1.10 concurrency controls |
| Core C02 | PG lease path met; DataFusion path open | `state()` compares model and physical digests on every lease, copy, seal, validate and publish (`mod.rs:445-450`) | P1.10 `GenerationTable` |
| Core C03 | **Closed (contract)** | Typed `RelationUse` by `TypeId`; undeclared relation, second writer, missing writer, self/two-stage cycles, self-contribution refused; domain `Relation` has no stage field; legacy `decl` feeds only dormant code | — |
| Core C04 | **Closed except P3 remainders** | `site_facts` over direct non-Potential alternatives with phase groups; Dataflow excludes higher-order and Potential; Summary needs unique, complete, Definite, Exact and a known receiver; `normalize_site` via `classify_receiver` | P3: equivalence, SQL views (§9); composition F04 admission token |
| Core C05 | **Closed (contract)** | Identity-only path rule, total `Ports`, `Entry` vs `Formal`, slot writes, receiver as first parameter, stored input/output rule | Stored `Entry` justification (P4, composition F02) |
| Core C06 | **Closed (contract)** | Opaque `InvokedGuard`; witness required; `Nested` refuses; stored equation re-derived | Producer control A14 |
| Core C07 | **Closed (contract)** | `ControlInfluence` and `Selection` relations with a connecting check; `TransferKind` has no control kind | Kernel-emitted influence (composition F07, P4) |
| Core C08 | **Re-confirmed** | `TransferKey` excludes the condition; `merge` keeps the key id; premises cite alternatives, which merging never rewrites | SCC churn of callee alternatives is composition F06 |
| Core C09 | **Not re-confirmed** | ScopeBoundary → Unknown, ResponseBudget filtered, approximation → Unknown hold; modality missing and the coverage reason misclassified | **F01 (before X0)** |
| Core C10 | **Closed** | `Receiver::Unknown`; one `classify_receiver` | — |
| Core C11 | Routed | Atoms keyed (evaluation, context, predicate, operand); structural path segments remove the escaping issue | A14 one-predicate-one-atom; A3–A4 kinds |
| Core C12 | **Closed for the owner rule** | `OwnerTable` equals the oracle on shuffled input and the real parse | Attachment half with B2 (exact-only T7) |
| Core C13 | Contract side implemented | Typed premise roles, generated `derivation_acyclic`, generated views covered by the schema grant | Consumer side P4/P5 |
| Core C14 | **Closed** | §15.1 assigns provider registration to `cpg-core` with the wheel reason | — |
| C4/C5 O1 | Confirmed defect | See F01 | F01 |
| C4/C5 R1–R5 | Re-inspected, as recorded | R1 closure read → `Disjoint` in, `Err` out (`composition.rs:186, 225`); R2 `Receiver` refused (`:132`); R3 bound receiver delivers (`:353-354`); R4 restatement used by the composed support (`:446-453`); R5 caller-side output (`:434-441`) | R4 positive control at A10 |
| Resource F01, F03, F09 | **Closed** | Writer poisoning (`batching.rs:36-39, 82`) and stage failure (`stages.rs:331, 355`); memoized charged type closure (`types.rs:182-234`); `ModelError::Limit` at writer, COPY and reads | — |
| Resource F02 | **Closed** | One refusal contract (`typed_syntax.rs:49-68`); E1 stored prefix control | — |
| Resource F04 | Wording and admission implemented; declaration missing | Admission before handles (`:412-422`) | F08 |
| Resource F05 | Partly addressed | `StageOutput::push_batch` exists | A0: channel element, contribution batch entry, bounded-peak control |
| Resource F06–F08, O1 | Unchanged routes | — | P1.9/P1.10; F08 calibration → Q; O1 at B3 scale |
| Input-validation F02 | E1 part met for the stage-bound subset; not closable | Measured envelope, named uncounted terms, clean mid-attempt exhaustion (E1 README). Not covered: both profiles (ty_flow is P2), the full captured closure (A2), SQLx retention, charged-map calibration | Narrow to **A0** (charged channel unit and bounded peak), **P1.9/P1.10** (F06/F07, typed DataFusion refusals), **Q** (both-profile envelope over the full closure, F08 calibration) |

## 8. Library fit and total complexity

| Capability and owner | Consumer | Candidates | Pinned fit and gaps | Burden | Choice |
|---|---|---|---|---|---|
| Arrow codecs (`record`, macros) | Every sink | serde_arrow 0.15.1 with explicit schemas | Fits; derive supplies schemas | Small | Keep |
| DDL lowering (`generations::ddl`) | Store | SeaQuery 1.0.2 | Fits; CHECK/FK/sum checks expressed | Small | Keep; scope to frontier (F02) |
| Attempt memory pool (`resources`, `model_runtime`) | All holders | DataFusion pool; `FixedPool` | Thin trait, two real implementations | Small | Keep |
| Stage scheduling (`stages`) | Schedule | petgraph `toposort` (already a dependency) | Insertion-order ties; own Kahn gives the name tie-break the digest needs | ~20 lines | Keep |
| Refused-module isolation (`typed_syntax`) | Scenario 6 | Pyrefly `replace-imports-with-any` (`a07b7ba`) | Stops the finder from loading; importers see `Any`, a Types fidelity effect to disclose | Config + disclosure | Evaluate at A4 (F08) |
| Policy rendering (`calls`) | P3 views | Generated SQL from a predicate; materialized admission rows | See §9 | — | P3 decision |

## 9. Alternatives and tradeoffs

| Alternative | Change propagation and local reasoning | Authority and composition | Test / substitution | Machinery and risk | Decision and revisit |
|---|---|---|---|---|---|
| Current baseline | Contracts localize scenarios 1, 4, 6, 7 | One authority per concept except F01 | Store-free kernels; memory = PG | Right-sized | Accept after F01 |
| Frontier-scoped generation schemas (F02) | One place decides what a generation contains | Frontier table drives DDL, receipts, grants and readers | Reader refusal testable | Per-frontier physical digest | Recommended for P1.7 |
| Reader-side refusal over whole-model schemas | Smaller change | Raw SQL via the serving grant still sees empty tables | Weaker | — | Rejected |
| P3 policy views generated from a declarative predicate (the deleted C6 route) | Two renderings kept equal by a matrix test | Rust and SQL both encode admission | Needs the equality matrix back | Predicate DSL | Possible |
| P3 materialized admission: a normalization stage stores `SiteFacts` and per-policy admission rows produced by `SiteTargets::admitted`; views join them | One rendering (Rust) | Removes the second authority | Rust controls only | One L1 relation | **Simplest viable; recommended to P3**; revisit if a consumer needs a policy over relations the stage did not materialize |
| `SiteCall` with callee summary status (F03) vs engine-side bookkeeping | Kernel owns the rule | No restatement in the engine | Known answers in `domain_composition` | One enum | Recommended with P4 design |

## 10. Verification and uncertainty

| Claim or scenario | Label / date | Command or inspection | Outcome |
|---|---|---|---|
| All `lctx-model` suites and doctests at HEAD `c4c7cba` | **Tested, 2026-09-29, this review** | `python3 scripts/build_environment.py -- cargo test --release -p lctx-model` | **passed**: 159 tests across all targets and doctests, 1 ignored (legacy example) |
| PG lifecycle, stage sink and domain readbacks; `typed_conformance`, `typed_limits`, `typed_owner`; `model_runtime` | Tested, **historical author receipts** 2026-09-29 (plan §4.2 R1–E1) | As recorded in plan §4.2 | **not_run** here (Docker PG18; no material doubt for these findings) |
| Budget exhaustion and envelope (scenario 6) | Measured, historical 2026-09-29 | E1 README command | As recorded; not rerun |
| F01, F02, F03, F04, F05, F07 | Implemented code read, 2026-09-29 | Cited lines | Direct from source; no probe needed |
| F08 library option | Interface-checked, 2026-09-29 | Pinned fork source | Behavior not executed |
| `just fmt`, `just test-all`, facts pilots | — | — | **not_run**: functional scope incomplete (binding acceptance timing) |

Closure controls for F01, F03 and F04 must be pre-written answers, not values derived from the kernels under test.

## 11. Authority changes and dispositions

Proposed plan §8 rows (the plan is the single disposition owner; this review cannot edit it).

| Required change | Route and owner | Source | Proposed disposition | Closure evidence or revisit trigger |
|---|---|---|---|---|
| Verdict modality, coverage reason, priority rationale; §15.8 line | Implementation in `obligation` + in-place §15.8 amendment (ADR-0085 delegates) | F01, core C09, C4/C5 O1 | open → **before X0** | F01 controls |
| Frontier-scoped schemas and reader refusal | P1.7 (+P1.10, P1.11) | F02 | open → P1.7 | Facts schema lists only facts relations; `Frontier` refusal on read |
| Callee summary status in `SiteCall` | P4 engine design with composition F04/F06/F07 | F03 | deferred → P4, trigger: first `compose_site` caller outside tests | Three known answers |
| Batched output vs contributions | A0 | F04 | open → A0 | Dedup/refusal control |
| NotRequested attribution | B1 | F05 | open → B1; correct plan D1 text now | Catalog generation without a ty `Provider` row |
| Model/codebook snapshot | P1.11 (`model describe --format json`) | F06 | open → P1.11 | Snapshot with a renumbering diff |
| Typed store error class | P1.7/P1.10 | F07 | open → P1.10 | Injected transport failure control |
| Transitive-load declaration or `replace-imports-with-any` | Wording now; A4 | F08, resource F04 | open → wording now, A4 | Text; optional A4 control |
| Input-validation F02 row | Plan §8 | input-validation F02 | narrowed → A0, P1.9/P1.10, Q | As in §7 |
| C03, C05, C06, C07, C10, C12 (owner rule), C14 | Plan §8 core table | core review | closed at contract level (this review's evidence) | — |
| C08 | Plan §8 | core review | re-confirmed | — |

No exception records are requested.

## 12. Architectural judgment and decision

| Judgment | Verdict | Scenario evidence and scope | Required action |
|---|---|---|---|
| A1 Localize change | **satisfied** for scenarios 1, 2, 4, 6, 7; **unresolved** for 3 | A contract change is one declaration plus membership, with every lowering and both sinks following (4). A provider is one stage plus its producer (1). Exhaustion refuses at the holder (6). P4 would carry hidden knowledge about summary status, admission and ownership (F03, composition F04/F07) | P4 design (F03 with F04/F06/F07) |
| A2 Encode meaning structurally | **violated (bounded)** | Modality's effect on verdicts has no authority in the one verdict function (F01). What a facts generation contains is decided in two places that do not consult each other (F02, P1) | F01 before X0; F02 at P1.7 |
| A3 Extend through composition | **satisfied** for 1, 4, 7; **unresolved** for 3; **routed** for 2 | Stages, sinks and admission compose. P3 needs a rendering choice (§9). The P4 engine needs widening (composition F06), subjects (F07) and summary status (F03) | As routed |

**Bounded decision: Revise.** CI-G1 and G2 fail at contract level on F01, so C09 cannot be re-confirmed as X0 requires. The correction is one pure function, its controls and a §15.8 line. Nothing else in the assembled P0 needs rework: C03, C05, C06, C07, C10, C12 (owner rule) and C14 are closed at contract level; C04 is closed apart from its P3 remainders; C08 is re-confirmed; the resource-review corrections F01–F03 and F09 hold, and F04 needs only F08's declaration.

**Exact corrections for X0:**
1. F01 in `obligation::verdict` with its pre-written controls and the §15.8 sentence.
2. The plan §8 rows in §11 (dispositions for F02–F08, the narrowed input-validation F02 row, and the core-finding closures), plus the two wording fixes (F05's D1 text, F08's transitive-load limit).

After re-inspection of item 1, X0 becomes **Accept scoped**. Excluded: scenario 3 (P4 composition engine; composition F04/F06/F07 and F03; trigger: P4 engine design, or the first `compose_site` caller outside tests) and scenario 5's physical enforcement (F02; P1.7). Review acceptance is not release qualification; P0–P2 integrated gates and pilots remain `not_run`.

**Enclosing architecture: accepted for scenarios 1, 4, 6 and 7 at contract level** (Implemented; focused Tested, with historical PG receipts). **Unresolved** for scenario 3 (P4) and, until P1.7, scenario 5. Before P1 and P2 it still lacks:

- **P1.** A frontier-scoped generation schema and reader refusal (F02). Attempt-owned lifecycle with a stored failed state, stored stage outcomes (`seal` consumes the `ExecutionReceipt` that admission needs) and admission consumed at publish (P1.7). Retire/lease/select race controls (C01). The DataFusion reader digest check (C02). Typed store failure classes (F07). SQLx buffer and shared-buffer accounting (resource F06/F07). The model snapshot (F06).
- **P2.** An A0 provider channel carrying `Batch<R>`, with a contribution batch entry, the batched-output dedup rule (F04) and a control that peak reservation is bounded by the channel window, not total output. B1 coverage routing: `acquire` covering Artifacts, coverage rows contributed to `assemble` (O5), NotRequested attribution (F05). The refused-module import policy (F08). A14's one-predicate-one-atom and `Nested` producer controls. The both-profile envelope over the full captured closure at Q.

| Priority | Change and responsible component | Source | Closure evidence or revisit trigger |
|---|---|---|---|
| 1 | Verdict modality and coverage reason (`lctx-model::obligation`, §15.8) | F01 | Pre-written verdict and composition-to-verdict controls |
| 2 | Frontier-scoped schemas and reads (`lctx-postgres`, P1.7/P1.10/P1.11) | F02 | Facts schema and `Frontier` read refusal |
| 2 | Callee summary status (`composition`, P4 design) | F03 | Three known answers |
| 3 | A0 batching and contributions (`stages`, A0) | F04, resource F05 | A0 controls incl. bounded peak |
| 3 | Store error classes; model snapshot | F07, F06 | P1.10 control; snapshot |
| 4 | NotRequested attribution; refused-module imports | F05, F08 | B1 control; text or A4 control |

**Next step.** The plan owner applies F01, enters the §11 rows in plan §8, and requests re-inspection of F01. X0 then closes as Accept scoped, and P1.1 begins with F02 written into P1.7's controls.

## Re-inspection of F01 and the plan §8 entries (`7595557`), 2026-09-29

Scope: F01 against its closure criteria, and the plan §8/§4.1.1 entries proposed in §11. Source
re-read at `7595557`: `obligation.rs`, `domain_verdicts.rs`, the composition-to-verdict case in
`domain_composition.rs`, §15.8, the `typed_syntax::admit` doc, and the plan diff `c4c7cba..7595557`.
The other findings were not re-examined.

**Reviewer-executed checks, 2026-09-29, clean tree at `7595557`.**
- `python3 scripts/build_environment.py -- cargo test --release -p lctx-model --test domain_verdicts --test domain_composition`: **passed**, 5 verdict and 11 composition tests, including `a_candidate_or_potential_alternative_is_never_established_or_refuted` and the extended `every_admitted_alternative_composes_separately_and_modality_follows_the_site`.
- `python3 scripts/build_environment.py -- cargo check -p lctx-postgres -p cpg-extract -p cpg-core`: **passed** (the appended codes compile downstream; existing third-party future-incompatibility warnings only).

| Closure criterion (§7 F01) | Verdict | Evidence |
|---|---|---|
| Modality is a verdict input; Candidate/Potential never Established, Conditional or Refuted | **met** | `VerdictInput.modality`; a non-Definite modality adds `NonDefiniteAlternative` (50, Resolution class) to the open set before `first`. The test covers `true`, `false` under complete coverage, and an atom condition, for both modalities; Definite twins stay Established and Refuted |
| Default reason, with a more specific open reason winning | **met** | A stopped budget still names `BudgetReached`; `NonDefiniteAlternative` ranks before `MissingEvidence` |
| Failed refutation under partial coverage names coverage | **met** | `IncompleteCoverage` (51, Evidence class) replaces `IncompleteDomain`, which stays a selection reason |
| Priority rationale recorded | **met** | Doc on `priority`: most specific cause first |
| §15.8 amended | **met** | Verdict inputs now include coverage and modality, with both rules stated |
| Composition-to-verdict control | **met** | A Candidate composed flow is Unknown(`NonDefiniteAlternative`); its Definite twin is Established |
| Pre-written, independent answers | **met** | Expected conclusions are literals, not kernel output |
| Append-only codes | **met** | 50 and 51 appended; plan A3 now appends from 52 |

One deliberate behavior to note, not a defect: a non-definite alternative under `NotRequested` coverage is Unknown(`NonDefiniteAlternative`), not NotAnalyzed, because the Scope class ranks last. This follows the pre-existing rule that an unasked question is the least specific cause, and it never produces a positive or negative claim.

**F01: closed.** Core C09 is now re-confirmed: ScopeBoundary is Unknown, response budgets are excluded, approximation and non-definite modality block both positive and negative verdicts, and incomplete coverage has its own reason. G2 and CI-G1 now pass at P0 contract scope. Composition F04's caller-supplied booleans remain open on their plan route. A2 moves from violated to **satisfied for the P0 scope**; F02's P1 aspect is routed.

**Plan entries.** The §8 "P0 exit review findings" table carries F01–F08 with the owners and routes proposed in §11. The core rows record C03, C05, C06, C07, C10, C12 (owner half) and C14 as closed at contract level, C04 apart from its P3 items, and C08 as re-confirmed. The input-validation F02 row is narrowed to A0 (charged channel unit with a channel-window peak), P1.9/P1.10 and Q. The D1 receipt wording (F05) is corrected, and P1.7's controls now include the F02 frontier refusal and the F07 typed failure class. All accepted. Four small follow-ups for the plan owner, none blocking:
1. Update the F01 row to closed and the C09 row to re-confirmed, citing this section.
2. F07's row names P1.10, but its control was added to P1.7. Name one owner, or name both in the row.
3. The F08 row says "`admit`/`extract` state the transitive-load limit"; only `admit`'s doc was changed, though it references `extract`. Say `admit`.
4. When A0 starts, copy F04's dedup/refusal control and the channel-window peak control into A0's controls column, so the package spec carries them.

**X0 decision: Accept scoped.** Scope: the P0 contracts (typed model, stages and sinks, facts-frontier preflight and admission, resource accounting, call-site facts, owner rule, witnessed substitution, whole-call composition, verdict policy, stage-bound syntax subset) at Implemented and focused-Tested strength.

Excluded, with triggers:
- **Scenario 3** (the P4 SCC composition engine): F03 with composition F04/F06/F07. Trigger: P4 engine design, or the first `compose_site` caller outside tests.
- **Scenario 5's physical enforcement:** F02, at P1.7.

The enclosing architecture is accepted for scenarios 1, 2 (routed), 4, 6 and 7 at contract level. It remains unresolved for scenarios 3 and 5 as stated. §12's P1/P2 prerequisites are unchanged. This is review acceptance, not release qualification: P0–P2 formatting, integrated gates and pilots remain `not_run` until plan Q.
