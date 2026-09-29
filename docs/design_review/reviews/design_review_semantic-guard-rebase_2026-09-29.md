# Local guard rebasing — bounded review

## 1. Scope, outcome and coverage

**Change / conformance; Accept scoped, 2026-09-29.** Independent Codex source review using compressed
template slots 1, 6, 7, 8 and 12; [core 3.0, code-intelligence profile 1.1, template and
binding](../design_principles/standard.toml), design-review and companion skill.
Authority: [ADR-0085](../../adr/0085-typed-semantic-domain.md),
[DESIGN §15.6](../../design/sections/semantic-model.md) and
[cutover plan](../../plans/semantic-model-cutover-plan_2026-09-29.md).

**Implemented / Interface-checked, 2026-09-29:** inspected
[`conditions/rebase.rs`](../../../crates/lctx-model/src/domain/conditions/rebase.rs),
[`EvaluationAtom registration`](../../../crates/lctx-model/src/domain/conditions/mod.rs),
[`Predicate`](../../../crates/lctx-model/src/domain/value.rs),
[`qualification/support validation`](../../../crates/lctx-model/src/domain/assertion.rs),
[`shared fixture`](../../../crates/lctx-model/tests/fixtures/guards.rs),
[`model tests`](../../../crates/lctx-model/tests/domain_guard_rebase.rs) and
[`PG test`](../../../crates/lctx-postgres/tests/domain_guard_rebase.rs).
Adjacent inspection covered the existing simultaneous-substitution implementation, occurrence
roles and shared scope ownership. Line references describe the inspected working tree.

This slice owns local-guard instantiation, typed origin lineage and its persisted/source checks.
It does not implement stability-witness formal substitution, whole-call binding/root mapping or
P2 provider integration. No production or full P0/P1/P2 completion claim is assessed.

| Scenario | Inspected consequence |
|---|---|
| Same guard at different call sites | The new atom keys include caller occurrence and a typed source-atom predicate, preserving distinct sites. |
| Repeated/nested instantiation | Each predicate names the preceding atom; histories remain distinct even at the same outer site. Origin IDs are not encoded as display text. |
| Implicit provider call site | Shared `is_call` accepts `ExprCall` or explicit `OccurrenceRole::Call`; the ExprBinOp/Call control exercises construction. This is a call-role assertion, not a binding/stability witness. |
| Foreign origin beneath an authorized caller | Support validation walks the origin chain and checks evaluation/operand source ownership throughout. A local caller cannot authorize an unacquired origin. |
| Depth or work exhaustion | Construction reserves room for the new wrapper, permits 32 total atoms, then refuses; work and depth have distinct obligations. Stored traversal is also bounded. No byte/RSS accounting claim. |
| Preserve a negative/local condition | Existing simultaneous substitution renames atoms into fresh caller-site atoms; no existential elimination or refusal-as-true/false path is introduced. |
| Formal or receiver operand | Construction and corrected stored invoked-chain validation share `local_root`; ordinary unrebased formal/receiver atoms remain valid (F02 correction). |

Predicate code 8 is appended after existing codes 0–7. Ordinary originating atoms retain their
typed predicate and optional operand; rebased wrappers have no asserted caller operand.

### Focused receipts

All execution receipts are **author-reported, 2026-09-29**, not reviewer-executed or independently
log-verified. Commands identify suites; exact combined model invocation details were not supplied.

| Command / check | Outcome |
|---|---|
| `cargo test --release -p lctx-model --test domain_guard_rebase --test domain_assertions --test domain_transfer --test domain_conditions` | **passed**, earlier 16 tests: guard 3, assertions 3, transfer 6, conditions 4. This predates the new fourth guard control and F01 correction. |
| `cargo test --release -p lctx-model --test domain_guard_rebase` — post-F02 | **passed**, author-reported: 5 tests, including the six-case direct/nested formal/receiver/local matrix and ordinary-atom positives. |
| Compile checks | **passed**, author-reported at the final review checkpoint; exact combined commands not supplied. |
| `cargo test --release -p lctx-postgres --test domain_guard_rebase` — initial run | **failed**, author-reported Predicate/QualificationCheck input mismatch (F01). |
| PG guard rerun after input correction | **passed**, author-reported: good/foreign origins, typed readback and two-reader release. This precedes the final common transaction/COPY correction. |
| Adjacent transfer PG run | **failed**, author-reported: Busy at immediate abort after validation/publication refusals. Separate transaction-finalization correction is reviewed in [generation rollback](design_review_semantic-generation-rollback_2026-09-29.md). |
| PG guard/transfer/stages/generations after transaction/COPY correction | **passed**, author-reported: four targets, five tests including 65 MiB evidence. This precedes F02's stored-root correction. |
| `cargo test --release -p lctx-postgres --test domain_guard_rebase` — post-F02 | **passed**, author-reported: one real PG18 test, all 8 cases. |
| `cargo check -p cpg-extract -p cpg-core` | **passed**, author-reported after F02. |
| Reviewer tests, integrated gate, formatting/lints, pilot | **not_run**. |

## 6. Correctness and fidelity gates

**Interface-checked, 2026-09-29:** G1 authority, G4 hidden behavior, G7 truthful scope claims and
G8 library fit pass within scope. G2/G3/CI-G1 pass by source inspection after F01/F02 corrections;
F01 and F02 also have author-reported corrected guard PG passes; post-F02 model controls passed.
G6 passes for inspected atom renaming
and preservation of condition structure, not for deferred whole-call composition. G5 assembled
lifecycle is outside scope. CI-G2 source ownership passes by inspection; CI-G3 introduces no
evaluation-reference path. Execution evidence remains author-reported, separate from independent inspection.

## 7. Findings and applicability

Current scheduled disposition belongs to [cutover plan §8](../../plans/semantic-model-cutover-plan_2026-09-29.md#8-findings-disposition).
The following stable IDs and dated evidence are source material for that owner, not another
mutable status register.

<a id="F01"></a>
### F01 — Qualification validation declared an unhandled Predicate input

**Original priority: High.** Stable source: `design_review_semantic-guard-rebase_2026-09-29.md#F01`.
The input declaration included Predicate while `QualificationCheck::visit` handled only atoms,
condition nodes, conditions and qualifications. Any delivered Predicate batch caused an
undeclared-input failure, rejecting valid catalogs. Owner: shared assertion validation;
principles DP-03/07/08, gate G3.

**Correction inspected, 2026-09-29:** `assertion.rs:152–157` now declares only those handled
inputs. Support validation retains Predicate and GuardIndex (`assertion.rs:221–228`), so removing
the unrelated qualification input does not remove origin provenance checks. Guard validation
separately checks context agreement through lineage. The initial PG failure corroborates the
source finding; the author subsequently reports corrected guard PG passed, followed by the
four-target store/COPY rerun passing five tests. Both precede F02's correction. Source correction accepted.

<a id="F02"></a>
### F02 — Stored invoked chains bypass the formal/receiver refusal

**Original priority: Medium; correction inspected, 2026-09-29.** Stable source:
`design_review_semantic-guard-rebase_2026-09-29.md#F02`.
Originally, construction checked operand Place roots and refused Formal/Receiver, while stored
validation checked contexts, depth, operand-free wrappers and call-site shape without declaring
Place/PlaceRoot inputs or inspecting the terminal origin's operand kind.

A caller could construct a same-context, operand-free InvokedGuard at a valid call site pointing to
an ordinary formal-bearing atom. With valid references and acquired sources, stored origin and
shared support checks accepted it although the local-rebase operation refused it. Nested wrappers
had the same gap. This was a persisted enforcement bypass, not a request to implement substitution.
Evidence is source inspection, not an executed counterexample. Principles FP-03/05/06,
DP-03/07/08/21; A2 and G3 originally required revision.

**Correction evidence — Implemented / Interface-checked, 2026-09-29:** model guard validation
now declares PlaceRoot/Place inputs, loads bounded maps, and applies the shared `local_root`
helper to operand roots whenever lineage length exceeds one (`conditions/rebase.rs:44–75`).
Construction uses that same helper. Missing origin places/roots refuse; ordinary unrebased
formal/receiver atoms remain valid. Shared support owns source authorization while the registered
atom invariant independently enforces eligibility before publication.

The new model control manually constructs six direct/nested Formal/Receiver/local cases, expects
four refusals and two local positives, then checks ordinary atoms remain valid. The PG fixture
expands to eight cases and separately checks support authorization versus structural eligibility.
**Source correction accepted; no unresolved source finding remains.** On 2026-09-29 the author
reports post-F02 model tests (5), the real PG18 test (all 8 cases), and the cpg-extract/cpg-core
compile check passed. These are attributed receipts, not reviewer execution or independently
log-verified results. No deferred stability-witness implementation is claimed.

Other applicable FP-01–06 and DP-01/02/11/15/22/23 obligations hold for the named positive
scenarios, subject to the findings and evidence limits. No additional actionable defect identified.

## 8. Library fit and total complexity

**Interface-checked, 2026-09-29:** rebasing composes through the existing bounded Diagram
substitution operation, typed IDs and shared ownership checks. It adds no dependency, alternate
Boolean engine or textual identity encoding. A small bounded lineage index is appropriate;
the corrected constructor and persisted boundary share root eligibility without adding a general
call-composition framework.

## 12. Architectural judgment and decision

| Judgment | Bounded assessment |
|---|---|
| A1 Localize change | satisfied: condition lineage belongs to conditions; assertion support retains acquisition/scope authorization. |
| A2 Encode meaning structurally | satisfied after F02: typed origins/site history and persisted local-only eligibility have explicit enforcement. |
| A3 Extend through composition | satisfied: rebasing uses existing condition algebra and support ownership rather than erasing guards or duplicating Boolean semantics. |

**Accept scoped by source inspection after F02.** Both corrections are inspected; no unresolved
source finding remains. Post-F02 model5, real PG18 eight-case controls and context compile
passed according to the author. The earlier four-target store/COPY suite is retained as a
pre-F02 receipt, not relabeled as a post-F02 rerun.
Full call binding/root mapping, stability witnesses, provider migration
and enclosing phase qualification remain open. Only this artifact was written for this request.
