# Representative raw flow contracts — bounded review

## 1. Scope, outcome and coverage

| Field | Assessment |
|---|---|
| Tier · purpose | Change · conformance; compressed template slots 1, 6, 7, 8 and 12 |
| Reviewer · date | Independent Codex reviewer, 2026-09-29 |
| Subject | Uncommitted representative flow definitions, root membership, composite assertion-source handling and focused fixtures, including the inspected F01 correction |
| Standard | [Core 3.0, template, code-intelligence profile 1.1 and repository binding](../design_principles/standard.toml); design-review and companion skill |
| Authority | [ADR-0085](../../adr/0085-typed-semantic-domain.md), [ADR-0086](../../adr/0086-immutable-postgresql-generations.md), [cutover plan P0-B](../../plans/semantic-model-cutover-plan_2026-09-29.md) |
| Decision | **Accept scoped** after source reinspection of F01, with author-reported post-fix model/PG passes |
| Evidence | **Implemented / Interface-checked, 2026-09-29:** independent source inspection. Test results below are explicitly author-reported. |
| Exclusions | Complete ty producer integration and all raw-field mapping in P2; complete flow semantics, production admission, resource budgeting and phase qualification |

Reviewed [`flow.rs`](../../../crates/lctx-model/src/domain/flow.rs), composite source handling in
[`assertion.rs`](../../../crates/lctx-model/src/domain/assertion.rs), root membership in
[`mod.rs`](../../../crates/lctx-model/src/domain/mod.rs),
[`model tests`](../../../crates/lctx-model/tests/domain_flow.rs),
[`shared fixture`](../../../crates/lctx-model/tests/fixtures/flow.rs) and
[`PG test` (pre-pivot source)](https://github.com/paul-heyse/library-context/blob/6f1a7e98ebad029e7e38876cede4d7726a33eb4d/crates/lctx-postgres/tests/domain_flow.rs).
Adjacent inspection covered shared ownership, lexical/place contracts, the old
[`ty flow producer`](../../../crates/cpg-flow/src/lib.rs) and
[`raw tables`](https://github.com/paul-heyse/library-context/blob/0bc8ea11171d6fd96827a8e250964d3de5b3b4ce/crates/cpg-schema/src/tables.rs). Line references describe the inspected
working-tree snapshot and may move with concurrent changes.

**Responsibilities and fidelity — Interface-checked, 2026-09-29.** Flow owns nominal occurrence/
Place use and definition records, qualified use/definition/reaching/value/region observations and
their local structural invariants. Existing LexicalScope, BindingEventKind, Place, conditions and
generated supports supply shared concepts. Assertion validation expands composite subjects to
all their sources; shared ownership determines acquisition and coverage authorization.

| Scenario | Contract and observed consequence |
|---|---|
| Several definitions or an unbound alternative reach one use | Qualified reaching observations remain separate. Explicit `Unbound` is a provider observation, not missing coverage. Bound targets must reference the same nominal Place as the use (`flow.rs:103–118`). |
| A definition's Place root names another source | Composite provenance includes both event occurrence and Place-root source, including through a bound reaching target (`assertion.rs`, `subject_sources`). Acquisition/scope checks still apply. |
| Broad input qualification contains several files | Authorization may admit all files, but the corrected flow invariant independently enforces same-source lexical scopes, definition values and raw use/sink structure (`flow.rs:129–156`). |
| A value dependency crosses a call | `through_call` remains explicit and cannot be combined with identity transfer. Raw dependency observations do not establish a call-summary or value-transfer proof. |
| Existing transfer support uses composite subjects | Input/output Places and optional call-site provenance remain enumerated; the flow extension preserves the prior transfer source set. |

Place-root provenance is deliberately separate from local source geometry. Nominal Place equality
is a structural check, not a proof of reaching semantics or alias resolution. The synthetic fixture
demonstrates contract shapes; it does not establish actual ty output fidelity. Full raw-field
disposition, including call-path details and complete provider place mapping, remains P2 work.

### Focused receipts and uncertainty

All receipts are **author-reported, 2026-09-29**, not reviewer-executed or independently log-verified.
Commands identify the suites; original combined invocation details were not supplied.

| Suite command | Outcome and scope |
|---|---|
| `cargo test --release -p lctx-model --test domain_flow` | **passed — author-reported after F01 correction**: 3 tests, including the six-case broad-input matrix. |
| `cargo test --release -p lctx-model --test domain_transfer` | **passed — author-reported before F01 correction**: 6 tests; no post-F01 rerun claimed. |
| `cargo test --release -p lctx-postgres --test domain_flow` | **passed — author-reported after F01 correction**: one real PG18 test with two good/foreign-Place-root cases. |
| `cargo check -p cpg-extract -p cpg-core` | **passed — author-reported before F01's source-consistency addition**; not post-F01 compilation evidence. |
| Reviewer tests, formatting, linting and integrated gates | **not_run** |

The PG test exercises sealed stored-content validation and publication refusal/readback using
shared validators. It is not an independent ty oracle. The new broad-input matrix exercises the
source-structure invariant directly; its acquired foreign-Place-root positive establishes the
separation of structure from authorization, not validity of every other relation in that mutated
fixture. Post-fix passes above come from the author's explicit follow-up report, not inference
from earlier receipts.

## 6. Correctness and fidelity gates

Verdicts apply to the corrected source at inspection strength.

| Gates | Verdict | Evidence / limit |
|---|---|---|
| G1 Authority | pass scoped | Flow owns local structural meaning; shared Place, qualification, support and ownership concepts are reused. |
| G2 Semantic fidelity; CI-G1 Fidelity | pass scoped | Raw reach/value observations remain distinct from proofs; unbound, conditional and attributed alternatives remain explicit. Complete producer fidelity is excluded. |
| G3 Validity | pass after F01 correction | Source geometry is enforced independently of coverage authorization; reaching Place equality and transitive source checks remain active. |
| G4 Hidden behavior | pass | Definitions and validators introduce no captured-code execution or ambient analyzer input. |
| G5 Consistency and recovery | n.a. to changed lifecycle | No lifecycle change; shared resource admission remains outside this decision. |
| G6 Transformation and reuse | pass scoped | Composite source enumeration retains transfer endpoints and extends it to flow events and bound targets. |
| G7 Truthful capability claims | pass scoped | Fixtures, source inspection, post-fix author passes and pre-F01 transfer/compile receipts are distinguished. |
| G8 Library leverage | pass | Existing derive/lowering mechanisms and standard collections suffice; no new graph or analysis framework. |
| CI-G2 Evidence closure; CI-G3 Evaluation integrity | n.a. | No serving or evaluation-reference consumer changed. |

## 7. Findings and applicability

<a id="F01"></a>
### F01 — Broad coverage authorization admitted cross-file local flow structure

**Original priority: Medium.** Stable source: `design_review_semantic-flow_2026-09-29.md#F01`.

**Original finding — Interface-checked, 2026-09-29.** Use/definition/region observations enumerated
their subjects without relating their source artifacts. Under an input-scoped qualification,
support validation admitted both acquired files independently, allowing a use in one file to name
a lexical scope in another. Definition-value and raw use/sink relationships had the same gap.
The only flow invariant checked reaching Place equality, while the artifact-scoped fixture masked
the missing local constraint. Relevant principles: FP-05, DP-03/07/08; A2 and G3 required revision.
Correction owner: `lctx-model::domain::flow`, not shared acquisition authorization.

**Correction evidence — Implemented / Interface-checked, 2026-09-29.** The second model invariant,
`flow_source_structure` (`flow.rs:90–95`), declares Occurrence, LexicalScope, FlowUse, FlowDefinition
and all four affected observation inputs. `FlowStructure::same_source` resolves actual source IDs;
its handlers enforce use/scope, definition/scope, definition/value, use/sink and statement/scope
agreement (`flow.rs:129–156`). It does not constrain Place-root source equality.

`broad_input_coverage_does_not_authorize_cross_file_flow_structure`
(`domain_flow.rs:32–67`) supplies five crossed-relationship negatives under input scope and an
acquired foreign-Place-root positive. Each changed assertion first passes shared support validation,
then the structure check must reject exactly the five local-geometry violations. This directly
targets the original authorization-versus-structure gap.

**Closure assessment:** source reinspection accepts the correction and its controls. The author
reports post-F01 model tests (3, including the six-case matrix) and the two-case real PG18 test
passed on 2026-09-29. These are author receipts, not reviewer execution. Current disposition belongs to
[cutover plan §8](../../plans/semantic-model-cutover-plan_2026-09-29.md#8-findings-disposition);
this artifact supplies stable source evidence, not a second mutable status register.

No further actionable defect was identified in correction reinspection. Applicable FP-01–06 and
DP-01/02/03/07/08/22/23 obligations hold for the named corrected scenarios. Source consistency
does not prove exact lexical-scope placement or complete ty semantics.

## 8. Library fit and total complexity

**Interface-checked, 2026-09-29.** Typed definitions and existing generated Arrow/PG forms carry
the new family. Small maps implement domain source/Place checks; the shared assertion owner handles
composite provenance. No external dependency, alternate place interpreter or generic graph engine
is introduced. Producer integration remains a separate P2 responsibility.

## 12. Architectural judgment and decision

| Judgment | Verdict | Bounded evidence |
|---|---|---|
| A1 Localize change | satisfied | Flow structure, shared provenance and acquisition authorization retain separate coherent owners. |
| A2 Encode meaning structurally | satisfied after F01 correction | Explicit unbound targets, nominal Places, qualified alternatives and same-source local relationships have enforcement points. |
| A3 Extend through composition | satisfied | Flow reuses lexical/place/condition/support contracts; composite provenance extends without replacing transfer semantics. |

**Bounded decision: Accept scoped.** Independent source inspection accepts F01's correction;
the author reports the post-fix model and PG controls passed. The context compile remains a
pre-F01 receipt. No further in-slice implementation is requested.

**Enclosing architecture: not assessed as complete.** No full P0/P2, producer, admission or resource
qualification follows. Revisit fidelity and all-field mapping when the actual ty producer migrates.
