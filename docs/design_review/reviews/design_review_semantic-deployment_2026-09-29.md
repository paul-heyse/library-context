# Representative deployment contracts and explicit lease release — bounded review

## 1. Scope, outcome and coverage

**Change / conformance; Accept scoped, 2026-09-29.** Independent Codex review using
compressed template slots 1, 6, 7, 8 and 12; [core 3.0, code-intelligence profile 1.1,
template and binding](../design_principles/standard.toml), design-review and companion skill.
Authority: [ADR-0085](../../adr/0085-typed-semantic-domain.md),
[ADR-0086](../../adr/0086-immutable-postgresql-generations.md) and
[cutover plan](../../plans/semantic-model-cutover-plan_2026-09-29.md).
Scope is representative P0-B report/deployment contracts plus the explicit reader-release
correction, not assembled P1 lifecycle qualification.

**Implemented / Interface-checked, 2026-09-29:** independent inspection completed for
[`deployment.rs`](../../../crates/lctx-model/src/domain/deployment.rs),
[`root membership`](../../../crates/lctx-model/src/domain/mod.rs),
[`shared fixture`](../../../crates/lctx-model/tests/fixtures/deployment.rs),
[`model tests`](../../../crates/lctx-model/tests/domain_deployment.rs),
[`PG test`](../../../crates/lctx-postgres/tests/domain_deployment.rs) and
[`GenerationLease::release`](../../../crates/lctx-postgres/src/generations/mod.rs).
Adjacent inspection covered shared assertion/ownership validation, generated codec validation,
old `cpg-schema/src/evidence.rs` and `cpg-extract/src/observations.rs`, and pinned SQLx 0.9.0
pool/PostgreSQL connection source. Line references describe the inspected working tree.

Deployment owns typed report values, collection membership, reported environments and qualified
observations. Shared supports own invocation/scope/source authorization. PostgreSQL owns lease
acquisition, acknowledged release and retirement exclusion; domain records do not duplicate that
lifecycle policy.

| Scenario | Inspected contract and consequence |
|---|---|
| Shuffle maps or repeat command/tool text | Map keys sort canonically; duplicates by map key refuse. Separate ordinals retain ordered command/tool duplicates (`deployment.rs:64–99`). |
| Omit/reorder children or use the wrong collection role | Membership digest, contiguous ordinals and kind checks reject incomplete or malformed collections; environment metadata and invocation roles are checked separately (`deployment.rs:179–235`). |
| Preserve a duration above signed 64-bit range | `Milliseconds` keys encode the unsigned integer; canonical decimal text lowers the full `u64` range without narrowing (`deployment.rs:10–25`). |
| Report an environment different from the analyzer's runtime | `ReportedEnvironment` and `TaskReport` retain reported values. A reported execution status is not pipeline execution certification (`deployment.rs:102–142`). |
| Associate a report with a foreign captured target | Receipt span and target are both assertion subjects; shared acquisition/scope checks cover both. Reported path/hash strings never silently redirect the attributed target (`deployment.rs:144–153`). |
| Retire after releasing one of two readers | Consuming release acknowledges the session unlock; retirement still requires the exclusive lock and must remain Busy while another reader holds its lease (`generations/mod.rs:198–210,337–350`). |

The hand-authored fixtures exercise contracts, not receipt parsing or extraction fidelity.
Complete P2 mapping, including nested Scenario/OptionBinding inventory, remains open. The old
receipt parser's byte equality, fixed policy and runtime-environment verification are not claimed
for this slice. Collection/cardinality caps are local refusal bounds, not coordinated byte/RSS
accounting. Full admission, all P0/P1/P2 qualification and broader lease/cancellation integration
are excluded.

### Focused receipts

All execution receipts are **author-reported, 2026-09-29**, not reviewer-executed or independently
log-verified. Test commands identify the suites; exact original test invocation details were not supplied.

| Command / check | Outcome |
|---|---|
| `cargo test --release -p lctx-model --test domain_deployment` | **passed**, 3 focused model tests. |
| `cargo check -p lctx-model` | **passed**. |
| `cargo test --release -p lctx-postgres --test domain_deployment` — initial run | **failed**, Busy at immediate retirement after lease drop; earlier successful readback was not a suite pass. |
| Same PG suite — first corrected run | **passed**, one real PG18 test with good, foreign-target and missing-child cases, after explicit release was added. |
| Same PG suite — final two-reader revision | **passed**, author-reported: one real PG18 three-case test, including two-reader assertions that retirement remains Busy after the first acknowledged release and succeeds after the second. |
| Reviewer tests, integrated gate, formatting/lints, pilot | **not_run**. |

## 6. Correctness and fidelity gates

**Interface-checked, 2026-09-29:** G1 authority, G2 semantic fidelity, G3 validity, G4 hidden
behavior, G7 truthful capability claims and G8 library fit pass within scope. G5/G6 pass by
inspection for canonical collection reconstruction and the consuming release boundary, supported
by author-reported corrected PG passes, including the final two-reader execution.
CI-G1 passes for attributed reported values; CI-G2 passes for the inspected source ownership and
pinned-reader path; CI-G3 has no evaluation-reference input path in this slice. Assembled recovery,
parser fidelity and production admission remain outside these verdicts.

## 7. Findings and applicability

**No unresolved actionable findings** in the deployment contracts or explicit-release correction.
This includes the originally assigned domain review, not only the subsequent lifecycle seam.

<a id="F01"></a>
### F01 — Immediate retirement assumed synchronous lease-drop cleanup

**Author-discovered integration failure; correction inspected, 2026-09-29.** Stable source:
`design_review_semantic-deployment_2026-09-29.md#F01`. The initial PG test dropped its reader and
immediately retired the generation. SQLx schedules close-on-drop asynchronously, so the server
could still hold the shared advisory lock and correctly return Busy. This was a cleanup
synchronization gap, not evidence that retirement should ignore active leases.

**Owner and correction:** `lctx-postgres::generations`. `GenerationLease::release(self)`
(`generations/mod.rs:343`) checks the boolean response from `pg_advisory_unlock_shared`, rejects
false, then consumes/closes the connection. `pin` already marks it close-on-drop; error or
cancellation does not return the session to the pool. The explicit unlock response supplies the
server acknowledgment; socket close alone does not. Release consumes the reader, preventing
subsequent reads through the released capability. Retirement's Busy policy is unchanged.

**Closure evidence:** correction accepted by independent source inspection; first corrected
three-case PG run passed according to the author. The strengthened two-reader control
(`tests/domain_deployment.rs:47–51`) is inspected and the author reports the final real PG18
three-case test, including those assertions, passed on 2026-09-29. No broader
cancellation or transport-failure test claim follows. Current scheduled disposition belongs to
[cutover plan §8](../../plans/semantic-model-cutover-plan_2026-09-29.md#8-findings-disposition);
this artifact supplies evidence rather than a second mutable status register.

Applicable FP-01–06 and DP-01/02/03/05/07/08/11/12/15/19/20/21/22/23 hold for the named
scenarios, subject to the stated limits. Additional attributed observations do not collapse by
ordinal alone; reported facts remain distinct from verified runtime facts.

## 8. Library fit and total complexity

**Interface-checked, 2026-09-29:** existing derives, typed IDs, Arrow/PG lowering and shared
support validation carry the domain. Domain-specific membership/role checks use small maps and
ordered scans; no new dependency or receipt parser is introduced.

Pinned SQLx 0.9.0 `sqlx-core/src/pool/connection.rs:86,97,157,199` confirms consuming close,
close-on-drop and asynchronous drop cleanup. `sqlx-postgres/src/connection/mod.rs:162` sends
Terminate and shuts down the stream without a server-cleanup acknowledgment. Composing the
explicit PostgreSQL unlock response with consuming close addresses the observed ordering need
without retries in the caller or weakened retirement exclusion. Broad lease integration remains P1.

## 12. Architectural judgment and decision

| Judgment | Bounded assessment |
|---|---|
| A1 Localize change | satisfied: deployment shapes, shared provenance and store lifecycle retain distinct owners. |
| A2 Encode meaning structurally | satisfied: typed values/roles, explicit membership, reported environment identity and consuming release expose the relevant distinctions. |
| A3 Extend through composition | satisfied: declarations reuse shared support/storage contracts; callers use the store-owned release operation. |

**Accept scoped.** Deployment-contract inspection and explicit-release reinspection are complete,
with no unresolved source findings. Author-reported model/check and first corrected PG receipts
passed, including the final two-reader PG rerun. Subsequent uncommitted rebase changes are excluded
from this reviewed snapshot. Revisit producer fidelity and nested field mapping
at P2, and assembled lease/cancellation behavior at P1. No full phase or enclosing architecture
completion is claimed. Only this review artifact was written for this request.
