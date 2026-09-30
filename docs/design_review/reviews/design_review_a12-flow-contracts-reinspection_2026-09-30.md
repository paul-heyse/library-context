# A12 flow contracts: scoped re-inspection

## 1. Scope, outcome and coverage

**Decision: Accept scoped.** The shared path invariant now enforces the originating use's ordered
operand crossings. Graph lowering returns canonical truth together with typed uncertainty and
preserves expression refusal. This assessment closes the contract defects from the
[initial A12 review](design_review_a12-flow-contracts_2026-09-30.md), subject to the evidence limits
below; it does not accept A13/A14, Q or the enclosing P2 architecture.

| Field | Value |
|---|---|
| Reviewer/date | Independent review agent, 2026-09-30 |
| Standard | [standard.toml](../design_principles/standard.toml): core/template 3.2, CI 1.3 and repository binding; design-review plus companion skill |
| Tier/purpose | Bounded design/target re-inspection of initial F01/F02 |
| Subject | Current dirty shared main: A12 model `domain::flow`, `conditions::graph`, re-export and unchanged kernel/refusal/qualification owners; model tests, shared fixture ordering and permanent PG flow test |
| Supported scope | Path geometry relative to declared Occurrence/CallSyntax records; use anchoring, every declared enclosing call, identity and shared validation; bounded condition conversion and retained exactness/refusal |
| Exclusions | A13 cpg-flow migration; A14 occurrence attachment, native flow/atom production and parity; P3 normalization, P4 summary/transfer proofs, P5 serving, facts pilots, full Q qualification |
| Method | Fresh source/fixture inspection; root-attributed focused receipts; no production edits, probes or test reruns by reviewer |

All implementation judgments are **Implemented, inspected 2026-09-30**. Executed claims are
**Tested only within the root-attributed commands in §10**. The initial report remains a dated
assessment of the earlier tree. The active cutover plan, not either review, owns current status.

## 2. Responsibilities, dependencies and semantic ownership

`domain::flow` remains the semantic owner of ordered raw call paths. The correction adds FlowUse
and a source/structural-path index of CallSyntax occurrences to its shared invariant. The provider
and PostgreSQL do not acquire independent path classifiers. `conditions::graph` owns transient
expressions and their lowering outcome; the existing kernel owns canonical BDD algebra. Domain/
Assertion declarations and generated supports still own Arrow/PG shape and attribution.

| CI family/relation | Fidelity/coverage and identity | Consumer boundary |
|---|---|---|
| Raw Flow path/value assertions | Provider-qualified observations; complete path relative to declared syntax, not a runtime call summary; ordered call/operand/role digest | A14 producer and P3 remain planned consumers |
| Conditions | Exact BDD truth of the retained expression; provider uncertainty separately returns Approximation::Unknown; exact inputs return Exact | Qualification consumes a typed GraphCondition, not a bare Diagram |
| Stored fixture records | Same Domain identities/supports/invariants as memory validation, inside sealed PG generation | Real PG18 focused receipt; native provider fidelity is excluded |

## 3. Contracts, constraints and testing boundaries

**F01 correction:** `flow.rs:251–255` declares FlowUse as an invariant input. `within` at
`flow.rs:269–272` checks source, byte geometry and structural ancestry. Each new crossing is
contained in the preceding operand, with repeated calls explicitly rejected (`flow.rs:283–292`).
The value link joins its originating FlowUse occurrence and requires that use inside the
innermost operand (`flow.rs:296–303`). Prefix enumeration compares every declared enclosing
CallSyntax within the sink to the recorded sequence and rejects omissions/extras (`flow.rs:304–316`).
The unchanged digest, ordinal, qualification and one-link checks still apply.

**F02 correction:** `Diagram::from_graph` returns GraphCondition with a Diagram and the existing
Approximation type. The graph's undirected Boolean uncertainty becomes Unknown, not a fabricated
Over direction. Exact retained expressions remain Exact. Expression combinators detect refusal
before constant folding, so refused AND false and refused OR true still lower to a boundary.
The bounded kernel and complete graph-shape validation remain the only canonical truth route.

Fixture `check` now applies `domain::memory::order` to each invariant's declared input ordering,
so multi-step ordinal validation receives the same semantics as production validation. The test
does not rely on arbitrary fixture batch order. No test-only copy of a path rule is introduced.

## 4. Composition and execution

Nested paths now compose through selected operands and are anchored at the same nominal use.
The governing rule has one owner and runs for memory and sealed PG records. The ancestor index
is keyed by source and structural path, not rendered names or provider-local indices. Ancestor
enumeration is input-depth dependent and does not scan every call for every value; no asymptotic
or representative-scale performance measurement is claimed.

Graph Boolean composition preserves uncertainty. Canonical condition identity still reflects
Boolean truth, while GraphCondition makes exactness an explicit result member. The verdict owner
at `obligation.rs:224` treats every non-Exact approximation as Unknown; the approximation join
owner makes Unknown absorbing. Thus retaining uncertainty does not require a second persisted
truth representation or directional inference from an undirected native flag.

**CI analysis record:** graph root closure is the expression universe; bounded BDD algebra is
exact for that retained expression; graph/provider uncertainty returns Unknown; malformed/limited
conversion returns Err; canonical atom/node identities carry truth and GraphCondition carries
exactness. No topology analysis, heuristic, seed or served claim is introduced.

Resource state remains attempt-owned through StateCharge and charged indexes. Paths retain the
256-step limit and graph conversion its 4096-node/cumulative-work limits. This re-inspection does
not establish representative-scale publication cost or a new memory-pressure test receipt.

## 5. Change and failure scenarios

| Scenario/kind | Owning operation and observed outcome |
|---|---|
| Add a nested argument path, composition of existing instances | Shared invariant admits the nested positive and rejects omission of either outer or inner call |
| Select a sibling operand, repeated call or reversed order | Operand structural ancestry and complete sequence comparison reject the malformed path |
| Associate an unrelated use with a valid path | FlowUse join plus innermost containment reject the association |
| Approximate atom or constant, provider uncertainty | GraphCondition returns Unknown separately from the exact Boolean diagram; negation cannot invent a direction |
| Expression refusal followed by Boolean simplification | Refusal remains Err for either operand order of AND false / OR true and under NOT |
| Provider-local representation changes, mechanism boundary | Generic graph/map and nominal attachment seam remain sufficient; analyzer geometry/parity must be qualified in A14 |

These retain the initial review's domain extension and mechanism boundary scenarios. No
substitution implementation or consumer-serving journey is newly certified.

## 6. Correctness and fidelity gates

| Gate | Verdict | Scoped evidence |
|---|---|---|
| G1 Authority | pass | One model path invariant; canonical BDD owner; mechanically lowered supports/storage |
| G2 Semantic fidelity | pass | Originating-use operand chain is enforced; undirected uncertainty remains Unknown |
| G3 Validity | pass | Listed invalid paths are rejected; complete malformed graph validation precedes BDD operations |
| G4 Hidden behavior | pass | New indexes/transforms remain pure, explicitly supplied and scoped |
| G5 Consistency/recovery | pass, scoped | Fixed bounds and refusal remain explicit; attempt-charged state; no new lifecycle/recovery guarantees |
| G6 Transformation/reuse | pass | Canonical identities/digest rechecks preserve meanings; lowering returns truth and exactness |
| G7 Truthful claims | pass, scoped | Focused receipts attributed; producer and phase-exit claims excluded |
| G8 Library leverage | pass | Existing BDD, model/support/lowering and memory-order mechanisms reused |
| CI-G1 Fidelity | pass, scoped | Raw observations remain raw; approximate graphs cannot emerge labelled Exact or unjustifiably Over |
| CI-G2 Evidence closure | n.a. | No served claims in A12; P5 is the revisit boundary |
| CI-G3 Evaluation integrity | pass, scoped | Production transforms consume supplied graph/model records, not reference fixtures or gold |

Gate judgments are reasoned assessments of the named contracts, not additional executed checks.

## 7. Findings and applicability

| Stable source finding | Re-inspection conclusion | Evidence and remaining boundary |
|---|---|---|
| [F01](design_review_a12-flow-contracts_2026-09-30.md#F01) | Contract corrected | Operand ancestry, originating-use join, repetition rejection and complete declared prefix sequence; independent model negatives; nested positive through sealed PG |
| [F02](design_review_a12-flow-contracts_2026-09-30.md#F02) | Contract corrected | Typed GraphCondition, Unknown for undirected uncertainty, refusal before constant folding; final condition-suite receipt in §10 |

No new in-scope material finding remains after the Unknown correction inspected here. A2's
model-adequacy and authoritative-realization requirements now hold for the selected scenarios.
FP-01–06 and the initial findings' applicable DP/CI rules are satisfied within this bounded
contract review. Topology/heuristic/serving concerns remain inapplicable for the reasons in the
initial review. Neither finding is waived by a deferral or a SHOULD exception.

The test named `call_paths_preserve_every_crossing_from_the_originating_use` covers six path
negatives. Its wrong-use case uses a foreign source; same-source sibling misuse is additionally
ruled out by inspected structural ancestry and originating-use containment. The PG suite's new
case is a nested positive; its existing negative remains foreign-place provenance, not a new
malformed-path negative. Shared invariant reuse plus focused memory negatives support this
bounded closure; this is not independent provider extraction evidence.

## 8. Library fit and total complexity

The correction uses the existing typed Approximation vocabulary, rather than inventing parallel
exactness codes. Memory ordering is delegated to the production helper, not a fixture-local
sorting policy. The specialized path invariant stays a model operation over declared syntax and
FlowUse. No new library, framework, store authority or generic graph engine is needed. The
initial library-fit judgment remains valid; no new external library API claim is made.

## 9. Alternatives and tradeoffs

Unknown is the smallest truthful outcome for a directionless flag; carrying Over/Under would
require an actual directional producer contract and polarity-aware propagation. That additional
mechanism has no A12 requirement. Returning only a Diagram would restore F02. Provider-only path
discipline would restore F01; the shared invariant is the appropriate boundary. Complete declared
syntax is a model precondition for the asserted path, while proof that the analyzer supplied
matching coordinates/coverage remains A14/Q.

## 10. Verification and uncertainty

| Evidence, 2026-09-30 | Root-attributed command/result | Scope |
|---|---|---|
| Path correction plus PG | `python3 scripts/build_environment.py -- cargo test --release -p lctx-model --test domain_flow -p lctx-postgres --features testing --test domain_flow`: **passed**, 5 model tests / 1 PG test | PG18 nested positive copied, sealed, validated and published; existing foreign-place negative refused |
| Conditions after final Unknown mapping | `python3 scripts/build_environment.py -- cargo test --release -p lctx-model --test domain_conditions`: **passed**, 6 tests | Truth table, shape/cap controls and approximation/refusal retention; root-attributed final rerun, no reviewer execution |
| Earlier six-condition receipt | Root reports six passed before final Unknown-label correction | Attributed earlier result only; final changed label requires its own receipt |
| Producer/qualification/full product gates | A13/A14, facts pilots, `just test-all`, `just fmt`: **not_run by reviewer** | Outside A12 re-inspection |
| Documentation/catalog | `just docs-check`, final `just library-catalog`: **not_run by reviewer** | Root owns final session checks and shared catalog regeneration |

Independent semantic expectations challenge production rules; memory and PG share the actual
validator rather than independently restating it. The resulting receipt is composite focused
evidence, not an initially clean integrated run, analyzer conformance or product qualification.

## 11. Authority changes and dispositions

The [cutover plan §8](../../plans/semantic-model-cutover-plan_2026-09-29.md) owns current F01/F02
disposition. Record closure using this re-inspection and dated final receipts there. The initial
finding IDs stay stable. Runtime propagation of GraphCondition into provider qualifications is
A14's obligation; no producer implementation is claimed by this review. Facts/serving admission
and complete native syntax/coverage evidence remain later scheduled obligations.

## 12. Architectural judgment and decision

| Judgment | Verdict | Scope evidence |
|---|---|---|
| A1 Localize change | satisfied | Corrections reside in path semantics, graph outcome and shared fixture ordering owners |
| A2 Encode domain meaning explicitly | satisfied | The model now governs complete declared paths and retained uncertainty/refusal |
| A3 Extend through composition | satisfied | Nested crossings and uncertain expressions compose through owned invariants/results without provider-side classifiers |

**Bounded decision: Accept scoped.** A12's contract defects are repaired in the inspected tree,
with final focused path/conditions/PG receipts attributed in §10. This review did not run or
replace those checks.

**Enclosing architecture: unresolved outside this scope.** Revisit at A14 for native attachment,
complete syntax/flow coverage and exactness propagation, and at Q for the assembled P0–P2
qualification. No overall phase, product-availability, performance or release acceptance follows
from this bounded result.
