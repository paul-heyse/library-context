# Phase 3 foundation implementation review

**2026-09-30 · Independent foundation checkpoint · Accept scoped after corrective reinspection.** F01 and F02
have implementation corrections and focused passing control receipts. The initial review found
missing invariant admission before completed-stage reads and premature release of handoff
reservations. The dated findings below retain those original scenarios and identify the
corrective evidence separately. O1's confirmed and failed-drain ownership paths were also
corrected and statically reinspected. The initial decision was **Revise**; the final bounded
decision is recorded in §12.
Current execution disposition
belongs in [Phase 3 plan §11](../../plans/semantic-model-phase3-detailed-plan_2026-09-30.md#11-current-package-status-and-finding-routes).
This review does not certify Phase 3, normalized compilation, or any P4/P5 capability.

## 1. Scope, outcome and coverage

| Field | Value |
|---|---|
| Subject | D0/R1/R2 commits `16aa6f3`, `d56534d`, `7d8cfc1`, plus the uncommitted R3 foundation on shared `main`; original provider fork `30e2aa21cd89fcb9694ce959d24c99cbf5567c85`, final corrective/root-pinned fork `a41da224caf9cae4feff48764d5f6df5ffcb4933` |
| Standard | Core 3.2, code-intelligence profile 1.3, repository binding, loaded through [standard.toml](../design_principles/standard.toml); `design-review` and `design-review-code-intelligence` skills |
| Tier and purpose | Design/target at the assembled R1–R3 checkpoint, within accepted ADR-0101/0102 |
| Reviewer | Independent `p3_foundation_review` agent, 2026-09-30 |
| Intended outcome | Establish whether the frontier/store/runtime foundation can safely support the N1–N7 normalization stages |
| Supported scope examined | Model-owned frontier closure, stage completion, private facts checkpoint, scoped facts availability, source-bound registration, physical scan admission, provider cancellation/close, shared resource runtime and stage coordinator |
| Exclusions | N1–N7 semantics and CLI integration, normalization coverage assembly, projection adapter, X deletion, Q qualification, P4 analysis/catalog, P5 serving. Facts availability is inspected here; normalized capability availability remains N7 work. |
| Method | Static source/type review and inspection of focused control implementations. Test outcomes below are attributed to the primary implementer's reported runs, not independently rerun by this reviewer. No additional probe was necessary to establish F01/F02. |
| Dirty-tree limitation | The primary implementer continued corrective edits during review. Findings identify the initially observed defect and separately identify inspected corrections. Unrelated AGENTS/settings/agent changes were neither edited nor assessed. |

The review follows [DESIGN §15](../../design/sections/semantic-model.md),
ADR-0101 (historical reviewed authority; [recover from Git](../../README.md#historical-recovery)), and the
[detailed plan §6](../../plans/semantic-model-phase3-detailed-plan_2026-09-30.md#6-cumulative-frontiers-and-private-stage-reads).
Those decisions are accepted targets; acceptance is not implementation evidence.

## 2. Responsibilities, dependencies and semantic ownership

| Owner | Responsibility and consumer contract | Expected change |
|---|---|---|
| `lctx-model::domain::admission` | Frontier relation closure, applicable invariant inventory, selectability, exact facts coverage admission, immutable scoped availability | Add the normalized frontier and normalized capability groups at N7 |
| `lctx-model::domain::stages` | One input/output/contribution declaration, transport, dependency order, nominal execution identity, acknowledged completion | Add normalization stage declarations and their input requirements |
| `lctx-postgres::generations` | Atomic freeze/receipts/grants, private checkpoint, generation lifecycle, source authorization, consumer invariant admission and leases | Add new relation-owned or declared shared input checks without copying validator logic; F01 correction |
| `cpg-core::model_runtime` | Shared DataFusion pool, source-bound catalogs, closed prepared query, whole-plan scan admission and retained source ownership | Change query composition while preserving reservation lifetime; F02 correction |
| `cpg-core::generation_read` | Translate the store's lease capability into bound provider sessions and declared schemas | Provider mechanics can change without changing domain source identity or availability |
| Owned provider fork | Fixed physical connections, bounded conversion, cancellation/drain and shared terminal state | Transport/library upgrade under retained lifecycle controls |
| `stage_runtime` / facts / CLI | Verify the executable declaration, compose stages, capture telemetry, allocate the attempt budget before input capture | Compose normalization after the facts checkpoint without inventing a native provider |

Dependencies generally point from store/runtime mechanics to model meaning. `StageTable` and
`AttemptReadContract` are justified by a current consumer boundary: schema equality cannot prove
source identity, attempt ownership, or availability. A second store abstraction or generic provider
framework is not needed.

| Fact/fidelity boundary | Authority and identity | Unknowns and consumers |
|---|---|---|
| Existing L0 assertions | Existing nominal model records and their attributed provider/context identities, unchanged by this slice | No provider observations are merged or promoted by R1–R3 |
| Scoped facts availability | `AdmissionCheck` checks exact expected membership; `ScopedAvailability` retains coverage ID, scope, context, provider, family and availability | Complete, Partial, Unavailable, NotRequested and an admitted empty universe stay distinct; future normalizers must consume the scoped evidence |
| Completed output | Attempt + producer + model + schedule + relation + canonical row count/content receipt | Completion establishes frozen content; the separate consumer read-check receipt establishes required input validity, as corrected under F01 |
| Query output | DataFusion physical result over admitted source handles | It is computation, not a new semantic assertion; downstream normalization owns interpretation |

## 3. Contracts, constraints and testing boundaries

`FrontierDescriptor` is the model's owner. DDL, selection and publication consume it rather
than maintaining an independent Facts switch. Conformance intentionally accepts a small validated
model and cannot mint product admission. Normalized is not advertised before N7.

`StageAccess::complete` poisons execution before awaiting the sink, accepts only the exact
acknowledged output set, then advances dependencies. The PostgreSQL transaction acquires the
installation/generation/output locks, hashes the locked content, revokes INSERT, grants importer
SELECT and persists receipts/outcome together. Serving retains no staging grant. Cancellation or
an uncertain completion cannot be retried through the same execution.

The facts checkpoint uses the existing shared invariant and admission path against frozen L0
content, remains staging, and freezes unrequested empty-family tables too. This is an adequate
barrier for L0. Consumer read-check receipts now establish required invariants over completed
normalized inputs before a dependent stage can open its provider pool.

Private source identity is checked at both capability minting and table registration. A published
table cannot be substituted for a stage input merely because it has the same Arrow schema.
The product read path additionally requires the validated facts checkpoint. Conformance controls
retain an explicit separate route.

Availability admission is conservative: `RequireComplete` rejects a family if any validated scoped
premise is incomplete; `ObserveAvailability` retains the exact evidence vector. This cannot use
a Complete generation aggregate to erase an Unavailable source. Narrower, context-specific
normalization outcomes remain the responsibility of N1–N7. Raw empty query results alone do not
carry an absence claim.

## 4. Composition and execution

| Capability | Composition and effect boundary | Limits and evidence |
|---|---|---|
| Native and future computed stage | `run_declared_stage` verifies declaration digest before handing out access; the same sink completion protocol applies | Telemetry is outside semantic digests; a new computed stage need not masquerade as a provider |
| Handoff source | Reserved typed batches become a MemTable under a checked consumer identity | PreparedQuery and QueryStream retain the source provider/reservation owner; F02 correction |
| Completed-store source | Store capability checks live attempt, exact source and invariant-check receipts, model/physical contract and relation set; each physical connection leases the generation | Lease closes/drains before a generation-exclusive transition; F01 correction supplies semantic eligibility |
| Query | Final physical plan counts each remote scan occurrence and its partitions, including aliases; unknown leaves and demand above capacity refuse before execution | One query gate and one attempt reader-pool guard prevent partial acquisition by competing stage queries |
| Memory | One DataFusion pool backs model reservations; the CLI creates it before capture; spill is disabled | This is a reservation bound, not an absolute RSS limit. Native heaps, driver buffers and calibration remain named qualification allowances. |

No graph or program analysis is implemented in this slice, so profile analysis-record fields
for projections, convergence and heuristic outputs are not applicable here. The relevant
computation is exact content validation and bounded relational execution; semantic analysis
remains excluded.

## 5. Change and failure scenarios

| Scenario and kind | Expected owner/propagation | Assessment |
|---|---|---|
| Add one normalized relation, domain extension | Add its nominal declaration/invariants and owning stage; extend the model frontier at N7; store lowering follows the descriptor | Relation ownership and store dispatch are localized. F01 correction enforces its required frozen-input checks at the consumer boundary. |
| Replace handoff with completed-store input, mechanism binding | Change the declared transport and use the corresponding source handle; preserve consumer identity and declared semantics | The two transports avoid duplicate dependency lists. F02 correction preserves handoff ownership independently of physical-plan internals. |
| Consume a module with unavailable calls, domain journey | Observe exact coverage rows and emit explicit normalized scope outcomes | The foundational vector preserves the distinction. No completed normalized implementation is claimed. |
| Three aliases with one provider slot, resource failure | Count all physical scans before polling | Inspected control expects typed `ResourcesExhausted`; no partially acquired join is permitted. |
| Cancel before `query_raw` yields its stream | Fork lease exists before the first await; cancellation drains the original connection | Inspected independent control blocks on a real table lock and verifies one binding/no replacement afterward. |
| Retain a table after explicit close | Shared pool state becomes Closing/Closed and future acquisitions refuse | Inspected controls retain pool/table clones and refuse later reads. Closed is being distinguished from Lost in the corrective source. |
| Direct INSERT races stage freeze | ACCESS EXCLUSIVE waits out the transaction; the receipt includes its committed row; later INSERT is revoked | The real-store control challenges actual PostgreSQL permissions and count receipts. |
| A later normalized operation assumes a cross-relation invariant | Check the shared invariant on the exact frozen premise closure before the operation | Initially absent; F01 correction now checks it before capability minting and binds the receipt. |
| Finish a handoff consumer while retaining its prepared query | The query must retain the batch reservation owner despite session/execution handoff release | Initially absent; F02 correction and detached-plan control establish ownership through stream release. |

## 6. Correctness and fidelity gates

| Gate | Verdict after corrective reinspection | Evidence/action |
|---|---|---|
| G1 Authority | pass | Model frontier and one shared facts admission/validator path; no parallel semantic store |
| G2 Semantic fidelity | pass within L0 foundation | Scoped availability and source identity stay explicit; no normalized output semantics certified |
| G3 Validity | pass | F01 correction checks required shared invariants and nominal closure before read authorization; positive/value-mismatch/missing-target controls |
| G4 Hidden behavior | pass | Declared effects, source-bound registration, read-only SQL and telemetry separation |
| G5 Consistency and recovery | pass within the declared reservation/lifecycle envelope | F02 retains source reservations; O1 retains reservations through drain or terminal transport abort; no absolute RSS or phase-exit claim |
| G6 Transformation and reuse | pass within inspected source lowering | No independent semantic reinterpretation found in the transport adapters; F01 read-check receipts bind the full frozen premise vector |
| G7 Truthful capability claims | pass within this report's boundary | Focused controls are scoped; normalized/P4/P5 and assembled Q remain not_run |
| G8 Library leverage | pass | DataFusion owns relational plans/pools, PostgreSQL owns transactional locking/grants, Tokio/bb8 own concurrency mechanisms |
| CI-G1 Fidelity | pass for facts availability transport only | Exact scoped rows are retained; no claim that normalized consumers already use them correctly |
| CI-G2 Evidence closure | n.a. | No served claims or P5 executor in this checkpoint |
| CI-G3 Evaluation integrity | pass for inspected changes | No gold/skill/evaluation-reference input route introduced |

## 7. Findings and applicability

<a id="F01"></a>
### F01 — Frozen content is not sufficient read eligibility

**Priority: high. Principles: FP-02/04/05, DP-03/08, A2/A3, G3.**

Initially, [`complete_stage_step`](../../../crates/lctx-postgres/src/generations/receipts.rs)
hashes and freezes outputs, while [`read_contract`](../../../crates/lctx-postgres/src/generations/lifecycle.rs)
checks source receipt identity and the L0 checkpoint. It does not check the shared invariants
required by a later normalized consumer. Shared model invariants run in `validate_scope` only
at a facts checkpoint or final validation. `Stage` initially has no input-invariant declaration.

An N1 relation can therefore satisfy its local row codec and freeze while violating a
cross-relation rule. N2 can read it and invoke an operation whose precondition is invalid;
final publication refusal does not make that intermediate operation valid. This contradicts
plan §6.2 and obstructs safe composition of the next supported layer.

**Correction:** have the model declare consumer invariant requirements and derive required
relation-owned checks. Before minting a read capability, run the same shared validators on
the exact frozen input closure, including nominal references and same-stage cycles. Bind
the proof to invariant identity, model/schedule and the canonical full input receipt vector.
All premises must be declared completed inputs or explicit validated checkpoint premises.
Missing premises or failed checks refuse before a provider pool opens. Do not add copied
test-only validators or let callers omit required checks.

**Closure:** inspect the enforcement path and a positive/negative real-store pair where locally
valid frozen rows violate a cross-relation invariant; the negative must fail before the
consumer executes. The implementer proposed consumer-bound validation during this review;
that proposal alone was not closure.

**Corrective reinspection, 2026-09-30: satisfied.**
[`Stage::read_invariants`](../../../crates/lctx-model/src/domain/stages.rs) derives non-optional
relation-owned checks and adds named consumer requirements. Those requirements enter the stage
digest and schedule dependencies. The asynchronous `GenerationAttempt::read_contract` invokes
[`check_stage_inputs`](../../../crates/lctx-postgres/src/generations/stage_validation.rs) before
minting the capability. It requires every premise to be a declared frozen input or a validated
checkpoint relation, checks nominal/subtype references, and runs the model's invariant instances.
The acknowledged read-check set binds the full ordered source receipt vector, model, schedule,
stage digest and checkpoint content/coverage. Each provider connection rechecks those durable
receipts in the lease binder. Failure/cancellation poisons the attempt.

The inspected [real-store control](../../../crates/lctx-postgres/tests/stage_validation.rs) has
an agreeing positive case, a locally valid value mismatch, and a missing nominal target.
Only the positive mints a capability; negative cases leave no partial read-check receipts.
The primary implementer's named focused command passed all three cases. This establishes the
reported correction without certifying future normalizers' declarations. Current disposition:
[plan §11](../../plans/semantic-model-phase3-detailed-plan_2026-09-30.md#11-current-package-status-and-finding-routes).

<a id="F02"></a>
### F02 — Prepared handoff plans can outlive their charged source owner

**Priority: high. Principles: FP-02/05, DP-19/20, A2/A3, G5.**

Initially, `HandoffTable` retains `Arc<Batch<R>>`, whose reservation owns the typed/Arrow
storage. Its `scan` delegates to DataFusion 55.1 `MemTable::scan`, which clones RecordBatches
into `MemorySourceConfig`; the physical plan does not retain the HandoffTable wrapper.
The original `PreparedQuery` retained only that physical plan, TaskContext, demand, gate
and budget. TaskContext contains execution configuration/functions/runtime, not the catalog.

A caller can prepare a query, drop StageSession, finish the handoff consumer and release
execution's final handoff. The plan still retains the Arrow buffers, while the batch reservation
is dropped. This undercounts live retained state and defeats the shared attempt bound.

**Correction inspected during review (Implemented, 2026-09-30):**
[`StageSession::sql`](../../../crates/cpg-core/src/model_runtime.rs) now transfers retained
provider Arcs into [`PreparedQuery`](../../../crates/cpg-core/src/model_runtime/prepared.rs),
which carries them into `QueryStream`. This preserves the HandoffTable and its Batch reservation
across plan execution. Collected output acquires its own retained-output reservation.

**Closure:** the intended targeted control prepares a handoff query, drops the session,
finishes the consumer and verifies the shared budget remains charged while the prepared plan
and then stream are live, returning to zero only after release.

**Corrective reinspection, 2026-09-30: satisfied.** The inspected
[`prepared_handoff_retains_source_reservations_after_its_stage_finishes`](../../../crates/cpg-core/tests/model_runtime.rs)
control performs that sequence and checks both detached plan and stream ownership. The primary
implementer's focused `model_runtime` run passed all three tests. The correction retains the
source provider directly and introduces no alternate scan implementation. Current disposition:
[plan §11](../../plans/semantic-model-phase3-detailed-plan_2026-09-30.md#11-current-package-status-and-finding-routes).

Applicable foundations FP-01 and FP-06 are satisfied by coherent model/store/runtime owners
and bounded pure/model versus real-store controls. F01/F02 corrective reinspection satisfies
FP-02/03/04/05 for the reported invariant and handoff scenarios. DP-01/02/04/05/13–18/21/24 and CI-01/02/04/10/12 are
satisfied within the inspected boundary; DP-03/08/19/20/23 bear the findings. Graph-specific
DP-07 and CI-03/05/07/09, recursive analysis DP-12 and serving CI-11/13 are outside this slice.

## 8. Library fit and total complexity

DataFusion 55.1 supplies physical plans, built-in joins/aggregations, memory reservations,
MemTable and a disabled disk manager. Using those APIs is preferable to a second SQL executor
or custom spill mechanism. The small query wrapper is justified by source/availability/capacity
admission that ordinary DataFrame execution does not provide. F02 is specifically a wrapper
ownership problem, not a reason to replace MemTable.

PostgreSQL 18 transactional grants and table/advisory locks fit atomic completion. SQLx owns
mutation and control transactions; tokio-postgres/bb8 supply bound read transport. The fork's
terminal state and early cancellation guard address a concrete gap without moving store
authorization into a generic library. The root pin and [pins record](../../pins.md) name the
exact fork revision. Full dependency-family qualification still belongs to Q.

## 9. Alternatives and tradeoffs

| Alternative | Consequence and assessment |
|---|---|
| Retain every L0 batch until all normalization finishes | Simpler lifetime but retains the full facts working set and defeats the agreed bounded stored-stage architecture |
| Publish temporary facts then normalize into another generation | Introduces lineage/retention and compatibility machinery the cumulative target deliberately avoids |
| Validate only at final publication | Protects publication, but cannot establish earlier consumer preconditions; rejected by F01 |
| Validate exact frozen inputs at capability minting | Fits the intended operation boundary, reuses shared validators, and avoids running unrelated not-yet-ready invariants after every native producer |
| Return unrestricted DataFrames | Exposes execution that can bypass whole-plan admission; the closed prepared query is justified |
| Keep source providers alive with the prepared query | Smallest viable F02 correction; no custom memory scan operator is required |

## 10. Verification and uncertainty

The reviewer read the control bodies but did not rerun implementation tests. The primary
implementer reported these **passed** on 2026-09-30:

| Command | Attributed scope |
|---|---|
| `python3 scripts/build_environment.py -- cargo test --release -p lctx-model --test domain_admission -p cpg-core --test generation_read --test stage_checkpoint` | 10 admission, 11 reader and one native checkpoint controls; includes exact scoped availability, duplicate reader-pool refusal, aliases/capacity and staging role denial |
| `python3 scripts/build_environment.py -- cargo test --release -p cpg-core --test model_runtime` | Original two source-registration/shared-memory controls; F02 regression control is additional work |
| `python3 scripts/build_environment.py -- cargo test --release -p lctx-postgres --test stage_validation -p cpg-core --test model_runtime --test generation_read` | Corrective receipt: one real-store invariant/nominal-reference test with three cases, three runtime controls including detached handoff ownership, and 11 generation-reader controls; root used fork `30e2aa21` for this run |
| Fork `cargo test --release --manifest-path build/forks/datafusion-table-providers/Cargo.toml -p datafusion-table-providers-postgres --no-default-features --test bound_lifecycle` through the build-environment wrapper, with protected local PG credentials | Final corrective receipt: three real PostgreSQL controls at `a41da224`, covering terminal close, pre-RowStream cancellation with reservation retention, and terminal abort after failed bounded drain. The last control passed after correcting its setup to commit CREATE before BEGIN/table lock. |
| `python3 scripts/build_environment.py -- cargo test --release -p cpg-core --test generation_read` | Exact final-pin integration: all 11 real PostgreSQL generation-reader controls passed against `a41da224`; reported test elapsed time 13.10s, not a performance comparison |

R2's dated receipt in plan §11 reports the separate real-store `stage_reads` controls for direct
write freeze, cancellation and transactional rollback. Their bodies were inspected here.
The prior P0–P2 qualification does not qualify the R3 changes.

**Observation O1, corrective reinspection (Implemented, 2026-09-30):** originally the fork's
`MemoryReservation` remained in the query future/generator and was released while drain could
still be running. `Lease` now owns that reservation before `query_raw` and transfers it into
the asynchronous drain task. The inspected pre-RowStream cancellation control precharges 4096
bytes, observes that charge immediately after cancellation, and observes zero after confirmed
drain; the implementer reported this fork control passed.

Reinspection also found that a failed bounded drain could trigger a second `ManagedClient`
destructor drain after the first owner's reservation was released. The corrected failure and
no-runtime paths call `abort_request`: mark Lost, clear streaming, and abort the transport task
before dropping the reservation. Static inspection confirms that this prevents the second
drain. The inspected forced-failure control checks terminal acquisition refusal and eventual backend
disappearance. The implementer reported all three fork controls passed at final revision
`a41da224caf9cae4feff48764d5f6df5ffcb4933`, followed by all 11 root generation-reader controls
against that exact pin. Cargo.toml, Cargo.lock and the pins record were reinspected for agreement.
O1 does not assert an absolute RSS guarantee or remove the named native/
driver allowances from Q's resource measurements.

**not_run:** `just fmt`,
`just test-all`, real FastMCP normalized pilots, scan/memory pilot refusals, measured growth
envelope, X deletion and P4/P5/product qualification. These remain at their authorized plan timing.

## 11. Authority changes and dispositions

F01 and F02 require implementation inside ADR-0101's accepted contract; neither needs a new
architectural decision merely to repair enforcement. Their stable source IDs belong to this
review; scheduled status and closure receipts belong only in plan §11. Existing provider-session
F04/F07, lineage and resource findings retain the parent cutover plan's disposition ownership.
Do not close those broader findings solely from this local review.

The owning design sections and STATUS still describe much of R3 as Proposed/design-only.
Update them with bounded implementation/test receipts at the implementation handoff, preserving
N1–N7/X/Q and P4/P5 exclusions. This is documentation follow-through, not a new gate.

## 12. Architectural judgment and decision

| Judgment | Verdict | Reason |
|---|---|---|
| A1 Localize change | satisfied | Frontier semantics, source authorization and compute mechanics have coherent owners; relation extension does not require store-specific frontier classifiers |
| A2 Encode domain meaning explicitly | satisfied after correction | Frozen content and eligible consumer inputs have separate proofs; exact scoped availability and query/drain reservation ownership remain explicit |
| A3 Extend through composition | satisfied after correction | New stored-input stages compose source capabilities and shared invariant definitions; detached query execution preserves ownership without a second scan implementation |

**Bounded decision: Accept scoped.** Static corrective reinspection and the attributed focused
receipts establish the R1–R3 contracts needed to proceed to N1–N7. The original F01/F02 scenarios
are corrected, and O1's ownership mechanism is corrected. This is architecture acceptance of
the named foundation scenarios, not release qualification. Final fork-pin integration passed
the stated focused controls; the remaining Q acceptance commands retain their separate outcomes.

**Enclosing architecture:** Phase 3 remains implementation-in-progress and unqualified.
Normalized semantic fidelity is not assessed until N1–N5; projections/CLI/deletion and assembled
phase exit remain later checkpoints. P4 analysis/catalog and P5 serving remain unavailable.
