# B1/Dc facts publication and compile outcomes

## 1. Scope, outcome and coverage

**Design · target; independent review; 2026-09-30.** Core/template 3.2 and code-intelligence
profile 1.3 selected through [`standard.toml`](../design_principles/standard.toml), with the
repository binding. Subject: current shared-main dirty tree based on `9efce30`; production
`cpg-core::facts`, facts-scoped memory validation, assembly/admission, `lctx compile`, and their
generation lifecycle/catalog consumers. Authority: [DESIGN §15.11](../../design/sections/semantic-model.md#section-15-11),
[ADR-0094](../../adr/0094-generation-owned-compile-outcomes.md), and
[cutover plan B1/Dc and §8](../../plans/semantic-model-cutover-plan_2026-09-29.md).

**Final bounded decision: Accept scoped, after F01 correction independently reinspected on
2026-09-30.** The store's canonical outcome authority is appropriate. The driver now preserves
primary failure, secondary cleanup uncertainty and generation identity. The post-abort `store
check` failure was diagnosed as absent service-baseline setup in the new disposable control;
the author reports its corrected real-PG rerun passed. This is bounded source inspection plus
attributed focused evidence, not Q or full-library qualification.

Scope excludes final Q, both-profile FastMCP pilots, full producer fidelity, calibrated resource/RSS
claims, P3–P5 and product/serving qualification. This review is evidence, not another disposition
register. The plan owns current finding status. Review method: production source and authority
inspection; author-reported focused receipts are separately attributed; no reviewer gate or test run.

## 2. Responsibilities, dependencies and semantic ownership

| Owner | Coherent responsibility and operation | Consumer/dependency contract | Reason for change |
|---|---|---|---|
| Model admission | Facts frontier, profiles/grains, exact coverage, outcomes and availability | Driver preflight; memory/PG sealed validation; private FactsAdmission | New domain family or completeness policy |
| Provider stage declarations | Native fact production, named inputs/outputs/contributions and effect | Scheduler and one attempt's stage-bound sink | Provider correspondence or new raw facts |
| Assembly | Shared vocabulary writer plus catalog NotRequested Flow | Contributors feed existing merge; admission checks exact universe | New contributor or unrequested family |
| `cpg-core::facts` | Compose declared providers, preflight, owned attempt, seal/validate/publish and cleanup | Depends on model/store/provider contracts; never selection or legacy ops | Workflow or failure contract |
| Generation lifecycle | Attempt identity, live lock, state transitions and receipt-bound publication | Typestates consumed by driver; catalog reports actual states | Store lifecycle behavior |
| CLI compile | Validate supported frontier/profile, acquire frozen inputs, open role pools, report publication | Thin use of acquisition/driver; operator chooses selection separately | Operator interface |
| Generation catalog / historical runs | Current generations versus retained historical operations | Separate stated purposes; no dual outcome write | A real retention/inspection consumer |

| Fact/outcome | Attribution/fidelity | Coverage and unknowns | Identity/consumer |
|---|---|---|---|
| Raw producer assertions | Named provider/run/context and native support | Complete/Partial/Unavailable under model; failures refuse publication | Typed relation IDs within generation |
| Catalog Flow | No provider or invocation | Explicit NotRequested per requested root | Scope/family/context; admission and operator inspection |
| FactsAdmission | Derived exact matrix plus actual execution receipt | Missing/extra/failed coverage rejects; no scope differs from unavailable | Contract/model/schedule/coverage/content digests |
| Published/current compile outcome | Generation lifecycle, not another operational record | Live/interrupted/failed/published remain distinguishable | GenerationId; list/show/select/abort/retire |
| Removed or pre-generation failure | Immediate error only | No durable history promised by ADR-0094 | CLI diagnostic; no fabricated operational attempt |

## 3. Contracts, constraints and testing boundaries

`publish_declared` refuses ambient configuration, model mismatch and inadequate schedules before
`GenerationStore::begin`. It verifies captured inputs before registration. `begin` repeats the
frontier check at the permanent store boundary, binds one Execution identity and creates the
registry/schema under the lifecycle connection's attempt lock. Repeated enforcement consumes the
same model contract; it is not a second completeness policy.

`compile_facts` compares every offered declaration with the exact scheduled declaration and runs
only profile-selected stages, in derived order. Stage-bound COPY marks an output completed only
after successful store delivery. Seal requires matching execution identity/model/schedule and
planned = written outputs. Stored validation applies scoped relations/invariants and AdmissionCheck;
publication checks the exact receipts, validator set, outputs and admission before state/grants
change in one transaction. Facts memory validation selects the same frontier and content digest;
above-frontier relations are not relabelled empty facts.

Partial facts publish with explicit coverage/reason; failed coverage or an entirely unavailable
required family refuses. Producer failure invokes attempt-owned abort. Seal/validation/publication
refusals end their lifecycle connection and are then cleaned by the driver. Cancelled/dropped
attempts leave inspectable interrupted generations rather than becoming published implicitly.
Unconfirmed commit/rollback/COPY cleanup has an explicit store class; F01 concerns its preservation
through the new driver.

`main` refuses non-facts frontiers before acquisition or database effects; profile parsing is closed.
Compile opens verified owner/importer pools, publishes, closes the writer pool and reports generation,
profile/frontier/content/availability. It never selects. `runs` continues serving historical retained
records and is not appended by this path. Memory fixture controls and disposable real-PG controls
are appropriate separate boundaries; passing memory production is not store lifecycle evidence.

## 4. Composition and execution

| Operation | Question, universe and inputs/outputs | Exactness/method and owner | Effects/evidence/limits |
|---|---|---|---|
| Schedule/frontier preflight | Can these stages produce requested facts? Declarations → admitted schedule | Exact model/stage contracts; no graph inference | Pure before DB registration; profile and source identities enter digests |
| Real facts pipeline | What native assertions were emitted? Frozen inputs → typed facts/vocabulary | Native assertions or explicit boundaries; existing provider stages | Acquisition/extraction effects declared; attempt budget and bounded handoffs |
| Admission/content validation | Are all exact scope/family/provider rows present? Sealed rows/receipt → proof | Shared model rules and deterministic relation digest | Memory for bounded controls; real stored PG contents for publication |
| Publication | Can this validated generation become visible? Proof/receipts → published state/grants | Permanent generation lifecycle, atomic transaction | Never selects; unconfirmed commit must remain unconfirmed |
| Error cleanup | What remains after a refused attempt? Primary error → abort/error | Driver composes existing store cleanup | F01: original cause/id must survive secondary failure |

No analysis/projection/synthesis stage is introduced. The schedule's graph orders declared
dependencies; it does not invent call/dataflow meaning. Cardinality and RSS calibration of the
full captured closure remain Q, not established by fixture equality.

## 5. Change and failure scenarios

| Scenario / change kind | Owning route and propagation | Evidence/gap |
|---|---|---|
| Add a raw fact family / domain extension | Model family/grain and native stage mapping; preflight/admission/lowering follow declarations | Existing checks reject uncovered writer/family; no private CLI family classifier |
| Change catalog to behavioral / policy binding | Profile selects ty; same driver and assembly, model requests Flow | Catalog retains provider-absent NotRequested; no alternative orchestration |
| Replace memory with PostgreSQL / mechanism substitution | Same StageSink/provider contracts; PG adds permanent lifecycle/admission | Author reports equal digests/availability on real producers and three fixture groups |
| Required provider refuses / execution failure | Driver consumes primary error and attempt-owned abort | Empty catalog/reservations zero reported; clean store check still under diagnosis |
| Publish commits but acknowledgment is lost / execution failure | Canonical registry decides actual state; error remains unconfirmed with generation id | F01: later abort refuses Published and currently hides the primary unconfirmed outcome |
| Operator requests analysis or old options / interface policy | CLI parser/dispatch refuses before acquisition | Inspected dispatch has explicit facts-only early refusal |

## 6. Correctness and fidelity gates

| Gate | Reinspection verdict | Evidence/action |
|---|---|---|
| G1 Authority | pass scoped | Generation registry owns new outcomes; frontier and coverage meaning stay model-owned |
| G2 Semantic fidelity | pass scoped | Requested/partial/unavailable/no-scope and current/historical outcomes remain distinct |
| G3 Validity | pass scoped after F01 correction | All primary-error branches preserve primary/cleanup diagnostics, phase and generation identity |
| G4 Hidden behavior | pass scoped | Non-facts request refuses early; DB effects follow preflight; no hidden selection/ops write |
| G5 Consistency/recovery | pass scoped after correction | Permanent state/receipt path and F01 correction inspected; author reports baseline-complete real-PG failure cleanup/store check passed |
| G6 Transformation/reuse | pass scoped | Same facts relation scope/digests; recompute admission retained; no skip-on-key reuse |
| G7 Truthful claims | pass scoped | Review distinguishes source inspection, author receipts and unresolved/full-Q claims |
| G8 Library leverage | pass scoped | Existing typed stages, SQLx transactions, PG store, Arrow validation and hashing reused |
| CI-G1 Fidelity | pass scoped | Shared coverage supports partial/unrequested facts; no inference relabelled runtime behavior |
| CI-G2 Evidence closure | n.a. | No served claim in this slice; raw support validation remains required |
| CI-G3 Evaluation integrity | pass scoped | No new compiler input path from gold/evaluation references |

## 7. Findings and applicability

<a id="F01"></a>
### F01 — A cleanup error erases the primary compile outcome and recovery identity

**Medium; FP-02/05/06, DP-19/21 · A2, G3/G5.** In `facts::publish_declared`, primary-error branches
use `attempt.abort().await.map_err(...)?` or `store.abort(id).await.map_err(...)?` before returning
the captured error. If abort fails, only that secondary failure reaches the caller. Its generation id
and triggering provider/seal/validation/publication error are lost.

Concrete sequence: publication commits, but the client receives an unconfirmed commit error.
The lifecycle cannot mark the now-Published generation failed; subsequent abort correctly refuses
Published. The driver returns State, hiding the unconfirmed publication and the identity the
operator needs to inspect. Provider refusal followed by transport failure during abort has the
same structural cause. This is an error-composition defect, not a need for another outcome store.

**Correction:** at the driver boundary, preserve the primary error/phase, safe generation id and
secondary cleanup class/detail. Explicitly state cleanup/publication uncertainty where applicable;
never force cleanup of Published. Existing model infrastructure classes or a narrow driver error
suffice. Store `Rollback`/`CopyAbort` already preserve both causes and offer a local precedent.

**Closure:** inspect every failure branch and add a bounded independent control for primary plus
cleanup failure, including an unconfirmed-publication case. Rerun the actual failure cleanup control.
Current disposition belongs in cutover plan §8 under B1/Dc; this review remains dated evidence.

**Correction reinspection (Implemented/Interface-checked, 2026-09-30):** `cleanup_error` now
preserves safe generation hex, primary phase/error, secondary cleanup error and the Unconfirmed
infrastructure class, with an instruction to inspect generation state before recovery. Every
provider/seal/validate/publish branch uses it only when abort fails; successful abort returns the
primary error. Cleanup still refuses Published. This resolves the inspected error-composition
defect without creating another outcome authority. The author reports the narrow diagnostic
control passed; actual lost-commit/transport injection is not established by that control.

FP-01–06 and DP-01–09/13–19/21/24 are satisfied for frontier policy, semantic ownership,
composition and the corrected error contract at inspection strength. Full DP-20 and
CI-08 production resource qualification remains Q; this slice does not claim it.

## 8. Library fit and total complexity

Existing stage declarations, generic StageSink, shared invariants, SQLx/PostgreSQL transactions and
generation typestates fit the scope. Ordinary orchestration functions are sufficient. Neither an
additional event store nor a new generic saga/reconciliation engine is justified for F01. No new
library API or pin is certified here; the inspected permanent store interface is reused.

## 9. Alternatives and tradeoffs

| Alternative | Judgment |
|---|---|
| Write new lctx_ops events and generation state independently | Inferior without reconciliation; publication can succeed while the historical event stays unfinished |
| ADR-0094 generation-only outcomes | Preferred: current state has one authority; removed/pre-generation work deliberately has no durable history |
| Link events to generation with explicit retention | Credible only after a real durable-history consumer; unnecessary machinery now |
| Mask primary failure with abort failure | Revise: loses the information required for recovery |
| Preserve both failures/id in existing driver contract | Preferred: bounded correction without changing publication authority or forcing unsafe cleanup |

## 10. Verification and uncertainty

Production inspection is **Implemented/Interface-checked, 2026-09-30**. Reviewer tests/gates,
formatting and pilots: **not_run**. Author reports real production memory=PG publication and
three fixture groups under both profiles **passed**. Exact commands/results belong to the author's
execution receipts; these messages are not independent reviewer runs. Missing schedule is reported
refused without registry effects. The initial injected required-failure run was reported to leave
an empty catalog and zero reservations, while its post-abort `store check` assertion **failed**.
That initial result is retained as a failed control, rather than relabelled as an initially clean run.

The author additionally reports
`python3 scripts/build_environment.py -- cargo test --release -p cpg-core --lib cleanup_diagnostics`
**passed** (one diagnostic control) on 2026-09-30. The store-check report had zero generations and
one Installation finding: service-baseline migration history differed because the new disposable
test omitted baseline migration. The author added migration before generation-store installation;
the following corrected receipts were supplied on 2026-09-30:

| Command | Attributed outcome and limits |
|---|---|
| `python3 scripts/build_environment.py -- cargo test --release -p cpg-core --test facts_generation` | **passed**: two tests, 17.19 seconds. Real producer memory/PG publication; required failure abort, reservations zero, catalog empty, missing stage refused before registry and baseline-complete store check clean |
| `python3 scripts/build_environment.py -- cargo test --release -p lctx --test compile_facts --test store_cli` | Author reports the bounded CLI controls **passed**: one plus two tests. Real semantic providers and PG; catalog/behavioral/repeated behavioral publish without selection, repeated content equals, capture-budget refusal creates no generation and store is clean. Acquisition uses an explicit no-op fixture command; real locked-uv acquisition and FastMCP qualification are not established by this control |

These are composite focused receipts after corrections. No reviewer test execution, complete
initially clean gate, actual lost-commit injection or complete product pipeline pass is claimed.

No fixture equality proves complete producer fidelity, full-library determinism, transport failure
recovery or the Q envelope. F01 can be settled cheaply at its narrow error-composition boundary;
that control does not replace real-PG cleanup and publication evidence.

## 11. Authority changes and dispositions

ADR-0094 records the meaningful alternative and §15.11 carries its current owner contract. No
additional ADR is required to repair F01 within that contract. The active cutover plan owns B1/Dc,
store-lifecycle F09 and this review's F01 disposition; STATUS links there. Accepted design,
implemented path, focused test receipt and assembled qualification remain separate claims.

## 12. Architectural judgment and decision

| Judgment | Reinspection verdict | Scenario consequence |
|---|---|---|
| A1 Localize change | satisfied | Frontier/provider/store/CLI responsibilities have explicit boundaries; F01 is confined to error composition |
| A2 Encode domain meaning explicitly | satisfied after F01 correction | Outcome authority and diagnostics distinguish primary failure, cleanup uncertainty and actual generation state |
| A3 Extend through composition | satisfied | Profiles/providers reuse one driver and permanent store; no dual writer or outcome reconciliation required |

**Initial decision: Revise for F01. Final bounded decision: Accept scoped** after independent
source reinspection and the attributed focused receipts. The diagnostic helper control does not
establish actual transport/lost-commit injection. **Enclosing P0–P2 architecture:
not yet assessed as assembled/qualified.** Q/pilots/full producers, resource envelope and downstream
P3–P5 retain their independent obligations.
