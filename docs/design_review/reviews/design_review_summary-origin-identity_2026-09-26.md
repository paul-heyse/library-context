# Design review: summary source-origin identity

**2026-09-26 · design/target · scoped review.** Core 3.0, code-intelligence profile 1.1,
library-context binding (`standard.toml`). Subject: ADR-0054 and the W5 source-contribution,
finite-summary, publication and native-serving contracts in the current working tree. This is
a self-review of the bounded migration; [plan W5](../../plans/behavioral-model-forward-plan_2026-09-24.md#6-findings-disposition)
owns its current disposition. Stage 3 and recursive composition are outside this acceptance.

## 1. Scope and expected change

The trigger is two source contributions with the same raw return fact and condition: a proof
for one must not erase the other's unknown. The next extension is SCC-local recursive value
composition, whose cap must also name the affected source path. This review follows the source
contribution through DataFusion seeds, pure summary admission, Delta validation, FORMAT 8
projection and the native open-boundary response. General predecessor and exit proofs, full
FastMCP Q09 and a real two-origin Delta/native fixture remain open plan scope.

## 2. Responsibilities, authority and fidelity

| Relation or component | Authority and fidelity | Identity, unknown and consumer |
|---|---|---|
| `value_flow_contributions` | `cpg-core::flow_model` derives unaggregated origins from attributed flow facts; the typed Arrow contract is in `cpg-schema` | `origin_id` hashes the declared semantic contribution key. `flow_values` may merge origins for presentation; it is not the proof identity. |
| Modeled/assignment seeds | `cpg-schema::behavior` DataFusion relations derive candidates from source, model and reaching facts | Exact modeled paths carry their source contribution id; assignment paths retain predecessor origin, source key and successor use. Missing evidence gives no positive seed, not a negative conclusion. |
| Finite summary | `lctx-analytics::summaries::finite` owns condition/proof/refusal decisions over typed inputs | A positive, refusal and boundary are keyed to the contribution and condition. The summary id also hashes the source origin and ordered proof. |
| Publication and serving | `cpg-core` publishes and reconstructs through the shared validator; bundle and PyO3 read one generation | Native open boundaries include raw fact, origin, condition and reason. A served unknown remains an unknown under the stated model. |

Provider `use_id` and raw fact identities are observed/extracted inputs; the contribution id
and summary are **derived**, not a new provider assertion. No backend graph index enters the
persistent identity. One snapshot is the scope for validation and serving.

## 3–4. Contracts and composition

The `value-flow-origin` recipe uses the repository's `lctx-id/v1` field encoding. A generated
semantic rule recomputes it with DataFusion's registered `lctx_id` UDF, and reference rules
require published summaries and boundaries to cite a contribution. The finite producer uses
the same input/output boundary for publication and reconstruction. This makes the origin a
checked relation rather than a caller-supplied display label. A positive with a narrower
condition does not silently close the origin's broader condition; an explicit refusal still
keeps that path open. The old SQL-only boundary complement was removed, leaving one owner for
the decision.

`cpg-schema::bundle` declares the native IPC fields; Python's expected Arrow schema mirrors
it, and `lctx_semantics::ipc_input` rejects a mismatch before hydration. The current-store
rebuild choice (ADR-0048) allows this schema migration without a parallel binary reader. The
native response retains both raw fact and origin because they answer different questions:
where the source evidence is, and which contribution remains open.
The unused native `value_paths` method was removed; the tests now exercise the production
`inspect_value_paths` entrypoint.

The producer remains pure and source-order independent. BDD atom, work and node refusals are
unknowns. This migration adds no new path materialization, analyzer, graph projection or
heuristic conclusion. Recursive work and its budgets remain the separate ADR-0053 target.

## 5. Change scenarios and alternatives

| Scenario | Owner and route | Expected affected consumers and limit |
|---|---|---|
| Add a sibling origin on one raw return fact | `flow_model` gives it a distinct checked id; seed and finite producer classify it independently | Delta boundary and native response retain its reason. A full two-origin source-to-native fixture is still needed for W5 closure. |
| Add another finite modeled value case | DataFusion seed selects the exact contribution; analytics adds only its new proof rule | Publisher/validator/bundle reuse the same typed outcome; new argument or exit semantics still require their own witnesses. |
| Add recursive value composition | ADR-0053 worklist keys work and cap by source origin | Existing source IDs are reusable; BDD composition and recursive proof are not established by this review. |

The former grouped raw-fact key was conservative but lost the individual's cause. Repeating
every contribution key field in summary and serving rows would preserve fidelity with more
cross-layer coupling. The chosen id is a small use of the existing `IdHasher`/`lctx_id` contract;
DataFusion's generic hash functions would not encode the declared ID fields and kind tag. No
new library or lifecycle improves this relation. The extra column and projection are the
cost of preserving semantic identity across the existing boundaries.

## 6–9. Gates, findings and library fit

| Gate | Scoped verdict and evidence |
|---|---|
| G1, G2; CI-G1 | Satisfied for a source contribution: the id derives from typed source fields, a sibling does not inherit another proof, and an open path remains unknown. General Python completion remains outside scope. |
| G3 | Satisfied for this contract: shared reconstruction, reference rules and the UDF identity check run before publication. |
| G4 | Satisfied: the pure producer uses explicit inputs; the id recipe reads no ambient state. |
| G5; CI-G2 | Satisfied for the examined generation: schema drift is rejected and native responses retain the origin and reason; full W5 two-origin Delta/native closure remains to verify. |
| G6, G8 | Satisfied: DataFusion performs joins and validation, the existing Rust id recipe gives stable identity, and the analytics producer owns finite semantic decisions. |
| G7 | Satisfied for the bounded claim: no missing positive or capped path is a negative transfer claim. |
| CI-G3 | Unaffected: no gold reference or evaluation parameter enters this migration. |

FP-01–FP-06, DP-01/02/03/04/08/11/12/13/16/18/19/21/24 and CI-01/02/03/04/06/08/11/13
are satisfied for the stated source-origin scenario by the typed identity, one decision owner,
checked references and pure producer. The full recursive and operation-wide architecture is
unresolved, not certified by this slice. **F01 (verification gap):** the pure sibling control
and native sibling admission challenge separate boundaries; a real extracted sibling whose
different causes survive a single Delta publication and native generation has not yet been
shown. The consequence is limited confidence in cross-layer closure, not evidence of a wrong
published result. [Plan W5](../../plans/behavioral-model-forward-plan_2026-09-24.md#6-findings-disposition)
owns the fixture and closure evidence.

## 10–11. Verification and authority

**Tested 2026-09-26:** `cargo test -p lctx-analytics --lib
proved_origin_does_not_erase_an_unproved_sibling_on_the_same_raw_fact --quiet` passed the
same-fact/same-condition sibling case; `INSTA_UPDATE=no cargo test -p cpg-schema --test
contracts --quiet` passed ten reviewed contract/rule snapshots;
`uv run --no-sync pytest python/lctx_mcp/tests/test_native_semantics.py
python/lctx_mcp/tests/test_value_paths.py -q` passed 15 focused native/page cases,
including distinct origin/reason admission and the supported paged entrypoint. `just
py-fixture` rebuilt and passed its single release-profile generation test.
`RUST_MIN_STACK=16777216 INSTA_UPDATE=no cargo test
-p cpg-core --test bundle finite_depth_and_unsupported_refusals_reach_the_native_response
--quiet` passed one real Delta/native refusal after rebuilding the extension. The complete
Stage 3 gate, pilot and full two-origin Delta/native fixture are **not_run**.

ADR-0054 updates DESIGN §B5 and §9.9. This review supplies evidence, not architectural
authority; F01's current status remains in the plan. There is no historical-format compatibility
layer under ADR-0048.

## 12. Judgment

A1 **satisfied** for adding one origin: the semantic id is constructed in one owner, with
typed acquisition and pure composition. A2 **satisfied** for the bounded case: identity and
refusal are structural, and the obsolete SQL complement is gone. A3 **satisfied**: direct,
modeled, assignment and local-call seeds carry one contract into the producer and future
recursive worklist. **Accept scoped** at Tested strength for origin identity and the examined
native boundary. The enclosing Stage 3 architecture remains unresolved; W5's single-run
two-origin publication trace and the remaining order-1/6 work retain plan ownership.
