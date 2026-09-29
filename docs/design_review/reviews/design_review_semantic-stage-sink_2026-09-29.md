# Semantic stage identity and generation sink — bounded review

## 1. Scope, outcome and coverage

| Field | Assessment |
|---|---|
| Tier · purpose | Change · conformance; compressed template slots 1, 6, 7, 8 and 12 |
| Reviewer · date | Independent Codex reviewer, 2026-09-29 |
| Subject | Uncommitted stage/sink changes over `5b46881e9e30115b19a7a8ac4107690a454c83c7`, including the F01 correction inspected in this review session |
| Standard | [Core 3.0 and template](../design_principles/standard.toml), code-intelligence profile 1.1, repository binding; design-review and its companion code-intelligence skill |
| Accepted authority | [ADR-0085](../../adr/0085-typed-semantic-domain.md), [ADR-0086](../../adr/0086-immutable-postgresql-generations.md), [DESIGN §15](../../design/sections/semantic-model.md), [cutover plan P0-D](../../plans/semantic-model-cutover-plan_2026-09-29.md#411-detailed-remaining-execution-order) |
| Bounded decision | **Accept scoped** after correction of F01; no new actionable finding on reinspection |
| Evidence strength | **Interface-checked / Implemented, 2026-09-29:** independent source inspection. **Tested, author-reported, 2026-09-29:** the two command receipts below. The reviewer did not execute those tests or independently inspect their terminal logs. |
| Exclusions | Full production model/producer/coverage admission; assembled P0/P1 or later-phase qualification; production compiler/store cutover; all-parser, driver, codec-total or process-RSS accounting. Concurrent lexical-domain work and its macro/model/assertion wiring are excluded. |

The inspected change scope is
[`stages.rs`](../../../crates/lctx-model/src/domain/stages.rs), its changed
[`domain.rs`](../../../crates/lctx-model/tests/domain.rs) and
[`domain_stages.rs`](../../../crates/lctx-model/tests/domain_stages.rs) controls,
[`generations/mod.rs`](../../../crates/lctx-postgres/src/generations/mod.rs),
[`control.sql`](../../../crates/lctx-postgres/src/generations/control.sql),
[`generation_stages.rs`](../../../crates/lctx-postgres/tests/generation_stages.rs), and
[`model_runtime.rs` tests](../../../crates/cpg-core/tests/model_runtime.rs).
Adjacent inspection covered the
[`model_runtime` consumer](../../../crates/cpg-core/src/model_runtime.rs), generation cleanup,
physical digest construction, existing lifecycle controls, shared resource-budget contract and
the pinned pgpq/SQLx source needed to assess COPY allocation and sending. This is a dated review
of those paths, not of every dirty file in the shared tree.

### Owners, contracts and change scenarios

**Implemented / Interface-checked, 2026-09-29.** `lctx-model` owns schedule membership,
producer input/output declarations, selected profile, effect/code/configuration identity,
execution identity and nominal permits. `lctx-postgres` owns the generation, successful COPY
completion and transactional sealing. `cpg-core` consumes stage identity for fresh DataFusion
catalogs over a shared attempt runtime; it does not acquire PostgreSQL ownership.

| Scenario | Owner and observed propagation | Evidence and boundary |
|---|---|---|
| Change producer code, configuration, effect or selected profile | Change the stage declaration; schedule digest changes and flows into generation metadata and receipt comparison. The store needs no independent producer classifier. | `stages.rs:82–95`; `generations/mod.rs:64–71,311–317`; model identity controls inspected, not rerun |
| Add an output using an existing relation category | Declare the stage output once; the sink derives its expected stage/relation set and refuses sealing if that output never reaches this generation. | `generations/mod.rs:69,301–317`; one authoritative output declaration, separate observed completion |
| Accidentally bind two generations or finish a no-op callback | Exclusive execution binding refuses the sibling. Sink-local successful COPY tracking rejects the no-op while permitting an explicit empty COPY. | F01 correction and independent negative/positive expectations in `generation_stages.rs:30,77–93` |
| Fail or cancel stage work; seal or clean up a generation | Model execution remains poisoned when the write callback fails or is cancelled. Sealing requires its receipt and successful sink outputs; receipts, grant changes and sealed state share a transaction. Cleanup includes stage receipts. | `stages.rs:179–199`; `generations/mod.rs:95–112,203–228,301–317`; source reasoning plus attributed author lifecycle receipt |
| Run another compute stage or reserve COPY bytes while another owner holds memory | Runtime registration compares exact stage/attempt identity; sink accepts the existing neutral budget and reserves wire space before encoding. | `cpg-core/src/model_runtime.rs:19–39`; `generations/mod.rs:259–286`; no total-memory claim |

**CI fact/fidelity scope.** The sink control uses the declared `Package` relation as disposable
conformance data, not extracted production evidence. Stage completion and generation-specific
write completion are distinct from fact-family coverage. Provider outcomes remain explicit in
the model receipt. No served assertion, heuristic, graph projection or gold evaluation is added.
Conformance publication still cannot select the production facts frontier.

### Verification and uncertainty

All receipts below are dated **2026-09-29**. The author/main agent supplied the passing outcomes;
they are **not reviewer-executed results**. The reviewer inspected the corresponding controls.

| Command | Outcome and attribution | Bounded evidence |
|---|---|---|
| `cargo test --release -p lctx-postgres --test generation_stages --test generations` | **passed — author-reported** | One stage-sink test plus two real PG18 lifecycle/chunk tests. Stage controls include generic copy/seal refusal, foreign capabilities/receipt refusal, sibling binding refusal, no-op refusal, explicit-empty success, shared-budget refusal/release, readback and conformance selection refusal. |
| `cargo test --release -p cpg-core --test model_runtime` | **passed — author-reported** | Two controls for fresh catalogs, foreign capability refusal, restricted SQL and shared compute/external reservations. |
| `cargo test --release -p lctx-model --test domain --test domain_stages` | **not_run — reviewer** | Changed model controls were inspected; no fresh execution result is claimed here. |
| The two author commands above, independently rerun | **not_run — reviewer** | Independent review was source-based and did not duplicate the main agent's runs. |
| `just fmt`, `just test-all`, `just pilot`, `just docs-check` | **not_run — reviewer** | No formatting, integrated qualification or documentation publication gate was run for this artifact. |

Receipt atomicity and cleanup inclusion were independently inspected. The stage test does not
directly assert every persisted receipt field, and the resource-refusal case exhausts the budget
before COPY metadata allocation; it is not a measured wire-memory envelope. Preallocation and
reservation lifetime are supported by the inspected code and pinned encoder implementation.

## 6. Correctness and fidelity gates

These verdicts apply to the supported stage/sink subset and have the evidence strength stated
above. A gate pass is not an independently executed test result.

| Gate | Verdict | Evidence / scope reason |
|---|---|---|
| G1 Authority | pass | Typed stage outputs author producer ownership; expected sink outputs derive from them. The written set records effects rather than reauthoring the schedule. |
| G2 Semantic fidelity | pass | Schedule identity retains selected profile/effect/code/configuration; permits and receipts retain attempt identity. Missing output differs from explicitly written empty output. |
| G3 Validity | pass after F01 correction | Binding, nominal permit checks and exact expected/written equality reject the reported bypasses. Existing schedule checks cover membership, writers, required inputs and cycles. |
| G4 Hidden behavior | pass | PostgreSQL effects remain explicit; effect class is included in schedule identity. No new analyzer discovery or ambient input path was introduced in this slice. |
| G5 Consistency and recovery | pass for inspected subset | Completion follows successful COPY commit; sealing retains lock/drain/revoke ordering and atomically persists stage receipts. Cleanup includes those receipts. This does not qualify the enclosing lifecycle. |
| G6 Transformation and reuse | pass for changed identity/encoding boundary | Deterministic ordering and selected identity fields are retained. COPY sizing uses the same pinned encoder rules as encoding. Stage-cache admission is outside scope. |
| G7 Truthful capability claims | pass | Conformance frontier remains explicit and unselectable as production facts; author receipts and independent inspection are distinguished. |
| G8 Library leverage | pass for added machinery | PostgreSQL/SQLx own transactions, pgpq owns encoding/size hints, and the existing neutral budget owns reservations. Sink completion tracking is a bounded domain contract. |
| CI-G1 Fidelity | pass for conformance labeling only | Disposable Package data and completed writes are not promoted to complete production coverage. Full provider fidelity is excluded. |
| CI-G2 Evidence closure | n.a. | No served claims or evidence-navigation consumer is changed. |
| CI-G3 Evaluation integrity | n.a. | No evaluation-reference input or parameter selection is changed. |

## 7. Findings and applicability

<a id="F01"></a>
### F01 — Sealing receipt did not establish writes to the generation being sealed

**Original priority: High.** Stable source identity:
`design_review_semantic-stage-sink_2026-09-29.md#F01`.
The current execution disposition belongs exclusively to
[cutover plan §8](../../plans/semantic-model-cutover-plan_2026-09-29.md#8-findings-disposition).
This section preserves the dated finding and closure evidence; it is not a second status register.

**Original finding, Interface-checked, 2026-09-29.** The initial uncommitted
`begin_conformance(&Execution)` allowed two generations A and B to share one execution identity.
Writing through A, completing the execution and sealing untouched B passed the receipt's
attempt/model/schedule checks. With the Package-only model, empty B could also validate and
publish as conformance data. Separately, a successful no-op `StageAccess::write` callback could
complete the execution without writing its generation. Neither scenario required production
facts admission. The original sites were `generations/mod.rs:62–66,279–284` and
`stages.rs:169–179` in the first reviewed working-tree version; those line numbers describe that
version, not the corrected file.

**Principles and consequence.** FP-02/03/05 and DP-03/19; A2/A3 violated, G3/G5 failed.
Execution completion could be composed with an unrelated or unwritten sink. The required owner
was the model's binding boundary together with PostgreSQL's generation-specific effect tracking.

**Correction inspected, Implemented / Interface-checked, 2026-09-29:**

- `Execution::bind_sink` (`stages.rs:119–124`) admits one sink before completed work and refuses
  reuse; `begin_conformance` requires mutable execution access and binds before creating a generation.
- `GenerationAttempt::copy` (`generations/mod.rs:301–309`) records the stage/relation pair only
  after the permitted COPY transaction succeeds. `seal` (`311–317`) compares that set exactly
  with outputs derived from the schedule, in addition to checking receipt identity.
- `seal_attempt` (`95–112`) inserts generation-specific stage receipts in the same transaction
  as privilege revocation and the sealed transition. `control.sql:42–48` defines their
  generation foreign key and stage/relation key. Cleanup checks and deletes those receipts.

**Closure evidence.** Independent reinspection accepted both corrections; no new actionable
finding arose. `generation_stages.rs:30` supplies sibling refusal; lines 77–93 distinguish
successful no-op refusal from explicit-empty COPY success. The author-reported passing PG18
command in §1 exercises these controls and the existing lifecycle/chunk cases. The reviewer
accepts this evidence for bounded F01 closure; current disposition is maintained in plan §8.

**Applicability after correction.** FP-01/04/06 are satisfied for coherent ownership, derived
expectations and local model testing. FP-02/03/05 are satisfied for the corrected generation
binding and completion composition. DP-03/04/08/18/19/22 and the relevant CI-04 boundary are
satisfied for the named scenarios: identity, declared effects, actual sink completion and
conformance-only claims remain explicit. This does not extend to excluded production coverage
or resource-total obligations.

## 8. Library fit and total complexity

**Interface-checked, 2026-09-29.** The review inspected the local pinned pgpq 0.12.0
`ArrowToPostgresBinaryEncoder::write_batch`, `EncoderBuilder` and scalar/text/list
`byte_size_hint` implementations, plus SQLx 0.9.0 `PgCopyIn::send`. Pins are recorded in
[docs/pins.md](../../pins.md). `copy_size` uses the same default encoder builders and includes
the row field-count prefix; the wire reservation precedes buffer allocation and remains live
through the awaited send. No hand-maintained scalar-width table replaces pgpq.

| Capability | Choice and burden | Assessment |
|---|---|---|
| Generation effect completion | Small expected/written sets, derived from model outputs and successful store operations | Necessary semantic link that SQL transactions alone cannot infer from a generic execution callback |
| Sealing and receipt persistence | Existing SQLx transaction and PostgreSQL locks/constraints | Reuses the lifecycle boundary; no new transaction framework or store |
| COPY sizing and admission | pgpq hints plus existing `ResourceBudget` | Library-owned encoding rules, conservative reservation, explicit failure; no all-driver or RSS guarantee |
| Adjacent compute consumer | Existing exact stage identity and shared runtime budget | No new runtime wrapper or provider framework required |

The simpler alternative of checking execution identity alone is disproved by F01. Exclusive
binding alone would still permit a successful no-op; sink-local completion is also necessary.
No broader architectural alternative or dependency change is requested by this conformance review.

## 12. Architectural judgment and decision

| Judgment | Verdict | Bounded evidence |
|---|---|---|
| A1 Localize change | satisfied | Model declarations own producer changes; store effects and compute catalogs retain separate owners. Corrections stay at the binding/sink boundary. |
| A2 Encode meaning structurally | satisfied after correction | Exclusive binding, nominal attempt identity, derived expected outputs, observed successful writes and generation-specific persisted receipts encode the reviewed distinctions. |
| A3 Extend through composition | satisfied after correction | A declared output composes with permitted COPY and receipt-bound sealing; the no-op/sibling counterexamples now refuse. The existing neutral resource budget composes with COPY without introducing a new resource owner. |

**Bounded change decision: Accept scoped.** F01's correction is accepted at independent
source-inspection strength with separately attributed author test receipts. No further in-slice
correction is requested.

**Enclosing architecture: not assessed as complete.** No full P0 or P1 acceptance, production
admission, full producer roster, all-buffer accounting or downstream availability is certified.
The existing cutover plan remains the execution authority. Revisit this boundary when production
admission or a changed sink/receipt protocol becomes supported; its owners remain `lctx-model`
and `lctx-postgres`, with the assembled review following the plan. Concurrent lexical-domain work
is outside this assessment.
