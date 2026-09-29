# Representative type contracts — bounded review

## 1. Scope, outcome and coverage

**Change / conformance; Accept scoped, 2026-09-29.** Independent Codex review using
compressed template slots 1, 6, 7, 8 and 12; [core 3.0, code-intelligence profile 1.1,
template and binding](../design_principles/standard.toml), design-review and companion skill.
Authority: [ADR-0085](../../adr/0085-typed-semantic-domain.md),
[ADR-0086](../../adr/0086-immutable-postgresql-generations.md) and
[cutover plan P0-B](../../plans/semantic-model-cutover-plan_2026-09-29.md).

**Implemented / Interface-checked, 2026-09-29:** independent inspection of uncommitted
[`types.rs`](../../../crates/lctx-model/src/domain/types.rs), type subjects and support validation
in [`assertion.rs`](../../../crates/lctx-model/src/domain/assertion.rs), root membership,
generated fidelity forwarding in [`macros`](../../../crates/lctx-model-macros/src/lib.rs),
[`shared fixture`](../../../crates/lctx-model/tests/fixtures/types.rs),
[`model tests`](../../../crates/lctx-model/tests/domain_types.rs) and
[`PG test`](../../../crates/lctx-postgres/tests/domain_types.rs). References describe the inspected
working tree. Adjacent evidence: old `cpg-extract/src/types.rs`, `cpg-schema/src/codebook.rs`,
shared qualification/ownership and local Pyrefly `quantified.rs` at pinned commit
`a07b7baead9e0c7b496346d879b88e2fff9cbda7`.

Types own nominal terms, ordered sequence membership, native variable coordinates and qualified
observations/presentations/restrictions. Shared assertion validation owns support authorization;
TypeIndex supplies transitive native-owner and fidelity checks. PostgreSQL consumes generated
contracts and model validators.

| Change scenario | Inspected consequence |
|---|---|
| Reorder or omit type children | Contiguous ordinals and the sequence digest check ordered membership (`types.rs:80–93,139–170`). |
| Change variable namespace or introduce a foreign nested owner | Provider/context plus module/anchor/slot/origin/kind define native identity; closure validation rejects foreign owners. Native anchors are not falsely treated as captured source occurrences. |
| Add a recursive bound/default | Separate qualified restriction edges avoid recursive variable identity; presentations remain separately qualified strings. |
| Add an opaque child under a modeled wrapper | Wrapper structure remains stored; corrected support fidelity conservatively reflects the opaque dependency. |

Complete native forms, actual Pyrefly producer migration, all-field mapping and producer fidelity
remain P2. Full admission, expected coverage construction, normalization, compatibility, caching
and complete resource accounting are excluded. Cardinality/work caps are explicit refusal bounds,
not a byte/RSS guarantee.

### Focused receipts

All test receipts are **author-reported, 2026-09-29**, not reviewer-executed or independently
log-verified. Commands below identify suites; combined invocation details were not supplied.

| Command / check | Outcome |
|---|---|
| `cargo test --release -p lctx-model --test domain_types --test domain_assertions` | **passed**, author-reported after F01: 4 type tests and 3 assertion tests; type test includes direct/nested Other/Truncated across all five fidelities. |
| `cargo test --release -p lctx-postgres --test domain_types` | **passed**, author-reported after F01: one real PG18 four-case test covering modeled good/foreign-owner cases and nested truncated structural rejection/DisplayOnly acceptance. |
| Model compile check | **passed**, author-reported after F01; exact command not supplied. |
| Reviewer tests, integrated gate, formatting/lints, pilot | **not_run**; bounded source review only. |

## 6. Correctness and fidelity gates

**Interface-checked, 2026-09-29:** G1 authority, G2 fidelity after correction, G3 validity,
G4 hidden behavior, G7 truthful claims and G8 library fit pass for the named scenarios.
G5/G6 pass at the inspected model boundary (explicit refusal, canonical membership and typed
lowering), with author-reported post-F01 persisted fidelity controls passed; assembled recovery/reuse is outside
scope. CI-G1 passes after F01; CI-G2/CI-G3 have no new execution or evaluation-reference path.
No gate verdict certifies full producer fidelity or production admission.

## 7. Findings and applicability

<a id="F01"></a>
### F01 — Opaque type evidence could claim structural fidelity

**Original priority: Medium.** Stable source: `design_review_semantic-types_2026-09-29.md#F01`.
Originally, Other/Truncated checked provider/context but allowed structural support labels.
This violated G2/CI-G1 and weakened A2: display-only payload could appear structurally faithful.
Correction owner: `lctx-model` type/support validation, with generated attribution forwarding.

**Correction inspected, 2026-09-29:** `SupportAttribution` exposes fidelity
(`assertion.rs:145`); the derive forwards the stored value (`macros/src/lib.rs:83`);
shared validation calls `TypeIndex::term_support` (`assertion.rs:347`). Its bounded dependency
walk requires `DisplayOnly` for Other/Truncated anywhere in the term closure
(`types.rs:205–225`), while retaining native-owner checks. All three type assertion families
declare their term as a subject and therefore share this enforcement.

The model matrix (`tests/domain_types.rs:43`) covers 20 direct/nested/kind/fidelity combinations;
the author reports it passed. The updated PG test adds nested truncated structural refusal and
DisplayOnly acceptance; the author reports the post-F01 four-case test and updated model check
passed on 2026-09-29. **Closure assessment: correction accepted by source inspection, with
author-reported post-fix model and persisted controls passed.** Current disposition belongs to
[cutover plan §8](../../plans/semantic-model-cutover-plan_2026-09-29.md#8-findings-disposition),
not a second mutable register here. No additional actionable defect identified.

**Fidelity/approximation interpretation:** [DESIGN §3.5](../../design/sections/facts-and-identity.md)
assigns row fidelity from its weakest semantic field. Thus conservative support labeling can
coexist with retained wrapper structure. [§15.3](../../design/sections/semantic-model.md#section-15-3)
separates proposition qualification from support fidelity. This correction does not impose
`Truncated → non-Exact`: an exact observation of an explicitly incomplete provider representation
does not establish an exact complete semantic type. A consumer claiming complete type semantics
must account for that boundary; no such consumer is certified here.

Applicable FP-01–06 and DP-01/02/03/07/08/11/15/21/22/23 hold for the inspected scenarios,
subject to these evidence and scope limits.

## 8. Library fit and total complexity

**Interface-checked, 2026-09-29:** existing derives, typed IDs, literal representation, Arrow/PG
lowering and shared support validation carry the extension. Small domain maps and a bounded walk
express ownership and shape rules; no new dependency or parallel type interpreter is introduced.
Pinned native identity inspection supports the namespace/coordinate distinction, not complete
producer mapping. Revisit that fit when P2 integrates actual Pyrefly output.

## 12. Architectural judgment and decision

| Judgment | Bounded assessment |
|---|---|
| A1 Localize change | satisfied: type shapes and native closure stay with types; shared authorization remains with assertions/ownership. |
| A2 Encode meaning structurally | satisfied after F01: nominal alternatives, explicit order, qualified restrictions and enforced fidelity preserve distinctions. |
| A3 Extend through composition | satisfied: existing literals, qualifications, supports and generated storage contracts are reused. |

**Accept scoped.** Independent source inspection accepts F01's correction with author-reported
post-fix model controls, model check and four-case PG test passed. Deployment changes are excluded.
No full P0/P2 or enclosing
architecture completion is claimed. Only this artifact was written by the reviewer.
