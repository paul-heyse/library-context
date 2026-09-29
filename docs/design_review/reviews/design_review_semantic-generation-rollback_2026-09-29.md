# Generation rollback acknowledgment — bounded review

## 1. Scope, outcome and coverage

**Change / conformance; Accept scoped, 2026-09-29.** Independent Codex source review using
compressed template slots 1, 6, 7, 8 and 12; [core 3.0, code-intelligence profile 1.1,
template and binding](../design_principles/standard.toml), design-review and companion skill.
Authority: [ADR-0086](../../adr/0086-immutable-postgresql-generations.md) and
[cutover plan](../../plans/semantic-model-cutover-plan_2026-09-29.md).
This reviews the actual refusal/cleanup race and its transaction/COPY correction, not full P1.

**Implemented / Interface-checked, 2026-09-29:** inspected
[`generations/mod.rs`](../../../crates/lctx-postgres/src/generations/mod.rs), including common
transaction finalization, pin lock ordering and explicit COPY abort;
[`generation_stages.rs`](../../../crates/lctx-postgres/tests/generation_stages.rs), including the
paired wire-budget control; adjacent guard/transfer/generation tests; and six deterministic cleanup
consumers changed from lease drop to explicit release (coverage, documents, flow, lexical, types,
generation_stages). The dedicated Drop fallback in `tests/generations.rs:136–143` remains.
Pinned SQLx 0.9.0 transaction, pool, PostgreSQL transaction and COPY sources were inspected locally.

| Scenario | Inspected contract and consequence |
|---|---|
| Validation/publication refuses, then caller immediately aborts | The common helper awaits rollback before returning the original body error, releasing transaction locks before subsequent cleanup. |
| Commit or rollback cannot be confirmed | Distinct Commit/Rollback errors retain uncertainty; failed finalization quarantines the connection with close-on-drop. Rollback errors retain the original operation error. |
| Pin races retirement | Shared generation transaction lock protects the published-state check through session-lock acquisition. Reader connections are already close-on-drop; unsuccessful pin cannot return a session-locked connection to the pool. |
| Row budget/encoding refuses after COPY has begun | Explicit `copy.abort` is awaited before transaction rollback. CopyAbort retains the original error plus abort failure; an additional rollback failure retains that chain. |
| Same batch under insufficient/sufficient wire budget | The test sends a small first row, then refuses the larger row under the lower budget: expected committed counts are zero versus two, reservations return to zero, and immediate abort succeeds. |

Generation storage owns finalization. Install/create/seal/validate/publish/select/clear-selection/
cleanup/COPY share the helper; pin uses the same transaction-on-connection path. Domains and
callers do not duplicate rollback policy. Explicit lease release remains the separately reviewed
[deployment correction](design_review_semantic-deployment_2026-09-29.md).

### Focused receipts and limits

All execution receipts are **author-reported, 2026-09-29**, not reviewer-executed or independently
log-verified. Suite commands identify coverage; exact combined invocation details were not supplied.

| Command / check | Outcome |
|---|---|
| Compile checks, including lctx-postgres | **passed**, author-reported, including the final COPY correction; exact final command set not supplied. |
| `cargo test --release -p lctx-postgres --test domain_transfer` before correction | **failed**, Busy at immediate abort after validate/publish refusals; this was not a lease-drop failure. |
| `cargo test --release -p lctx-postgres --test domain_guard_rebase` after qualification-input correction | **passed**, author-reported before the final store/COPY changes; not a final-tree receipt. |
| `cargo test --release -p lctx-postgres --test domain_guard_rebase --test domain_transfer --test generation_stages --test generations` | **passed**, author-reported: four targets, five tests including active-COPY budget refusal/success and 65 MiB evidence. This precedes the separate guard F02 correction; not a post-F02 combined rerun. |
| Post-F02 guard checks | **passed**, author-reported: `cargo test --release -p lctx-model --test domain_guard_rebase` (5 tests), `cargo test --release -p lctx-postgres --test domain_guard_rebase` (one real PG18 test, all 8 cases), and `cargo check -p cpg-extract -p cpg-core`. Detailed correction inspection belongs to the guard review. |
| Reviewer tests, integrated gate, formatting/lints, pilot | **not_run**. |

Cancellation retains SQLx drop cleanup rather than acknowledged completion. Production cancellation,
provider integration, transport/finalization failure injection and assembled P1 lifecycle qualification
remain open. Resource controls concern the existing local reservations, not coordinated byte/RSS
accounting. No automatic retry or confirmed commit outcome is inferred from an unconfirmed error.

## 6. Correctness and fidelity gates

**Interface-checked, 2026-09-29:** G1 authority, G2 fidelity, G3 validity, G4 hidden behavior,
G5 consistency/recovery, G6 transformation/reuse, G7 truthful scope and G8 library fit pass for
the inspected normal completion/refusal paths, with author-reported focused PG passes; these
verdicts do not cover injected transport failure or cancellation. CI-G1/CI-G2 preserve model
refusal and pinned-generation boundaries; CI-G3 has no new evaluation-reference path.

## 7. Findings and correction evidence

**No unresolved actionable source findings in this bounded store correction.** The independent
[guard F02](design_review_semantic-guard-rebase_2026-09-29.md#F02) remains outside this acceptance.

<a id="F01"></a>
### F01 — Completed refusal returned before transaction cleanup was acknowledged

**Author-discovered integration failure; correction accepted by source inspection, 2026-09-29.**
Stable source: `design_review_semantic-generation-rollback_2026-09-29.md#F01`.
Previously a body error dropped SQLx Transaction, which queues rollback; an immediate cleanup
operation could still encounter its generation lock and return Busy. Owner: generation store.
Relevant principles: FP-01/03/06, DP-05/12/19/20; G5.

`transaction_on` now awaits commit or rollback explicitly. `transaction` quarantines failed
finalization; ordinary operation errors return only after successful rollback. The pin path checks
state under a transaction lock before adding its session lease. Cleanup policy still refuses
genuinely selected/leased generations.

**COPY extension:** a dropped active SQLx PgCopyIn queues CopyFail, which is insufficient as an
acknowledged abort. `copy_attempt` now awaits abort for send/budget/encoding errors before returning
to transaction rollback. Failed COPY abort is explicitly represented, and rollback is still attempted.
If rollback succeeds, the transaction boundary is confirmed even if COPY cleanup earlier failed;
if rollback fails, the connection is quarantined and cleanup remains unconfirmed. Successful COPY
still requires finish and commit.

The paired `generation_stages.rs:77–95` control distinguishes mid-stream refusal from preflight
refusal: ID ordering puts the small row first; low/high budgets expect Resource refusal/success,
zero/two stored rows, zero outstanding reservations and immediate abort. The dedicated asynchronous
Drop test remains independent. The author reports the four-target, five-test rerun passed on
2026-09-29, including this paired control and 65 MiB evidence. These are author receipts, not
reviewer execution. Current scheduled disposition belongs to
[cutover plan §8](../../plans/semantic-model-cutover-plan_2026-09-29.md#8-findings-disposition),
not a second mutable register here.

Applicable FP-01–06 and DP-01/03/05/07/08/12/19/20/21/22/23 hold within the stated paths and limits.

## 8. Library fit and total complexity

**Interface-checked, 2026-09-29:** SQLx 0.9.0 Transaction Drop queues rollback; its awaited
PostgreSQL rollback executes the command before completing. PgCopyIn Drop queues CopyFail,
whereas `abort` consumes the expected error and ReadyForQuery response. The implementation uses
these existing async APIs and one private AsyncFnOnce helper, introducing no dependency, retry
framework or weaker Busy semantics. Failed-finalization classification preserves uncertainty
instead of hiding it behind the original domain error.

## 12. Architectural judgment and decision

| Judgment | Bounded assessment |
|---|---|
| A1 Localize change | satisfied: store-owned finalization is shared across generation operations. |
| A2 Encode meaning structurally | satisfied: confirmed operation refusal, unconfirmed commit/rollback and failed COPY abort have distinct outcomes. |
| A3 Extend through composition | satisfied: operations compose through existing SQLx transactions/COPY and the common helper; consumers use explicit lease release where acknowledgment is required. |

**Accept scoped by source inspection.** Compile and four-target/five-test PG receipts passed
according to the author. The subsequent guard F02 correction has separately passed model/PG
controls and context compilation, also author-reported. Source reinspection found no unresolved
issue in this bounded rollback/COPY correction.
No full P1, production cancellation or provider qualification is
claimed. Only the authorized guard and rollback review artifacts were written at this checkpoint.
