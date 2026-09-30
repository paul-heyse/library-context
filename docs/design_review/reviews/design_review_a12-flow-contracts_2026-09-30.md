# A12 flow contract review

## 1. Scope, outcome and coverage

**Decision: Revise.** The model supplies nominal raw flow records and a bounded expression-to-BDD
route, but two supported operations remain underconstrained: call paths do not establish their
operand chain or originating use, and graph lowering discards approximation metadata.

| Field | Value |
|---|---|
| Subject | In-progress A12 additions in `crates/lctx-model/src/domain/{flow.rs,assertion.rs,conditions/graph.rs,conditions/kernel.rs,conditions/mod.rs,mod.rs,obligation.rs}` and model/shared PostgreSQL flow fixtures/tests; dirty shared main inspected 2026-09-30 |
| Standard | [standard.toml](../design_principles/standard.toml): core/template 3.2, code-intelligence 1.3, [repository binding](../design_principles/binding/library-context.md); both review skills |
| Tier · purpose | Bounded design · target, as requested; the enclosing P2 architecture is not certified |
| Reviewer · date | Independent review agent · 2026-09-30 |
| Outcome to enable | Sound A12 model contracts before A13/A14 attach native flow observations |
| Supported scope | Raw Flow observations, source/call geometry, expression shape/refusal bounds, canonical BDD lowering, shared sealed-content validation |
| Exclusions | A13 provider migration, A14 attachment/atom construction and flow producer, P3 normalization, P4 transfer/composition engine, P5 serving, qualification Q |
| Baseline | [DESIGN §15](../../design/sections/semantic-model.md), especially §15.2–4 and §15.7–11; [cutover plan A12–A14](../../plans/semantic-model-cutover-plan_2026-09-29.md) owns execution |
| Method | Read source, changed contracts, independent expectation tests, PG fixture boundary and current authorities. No production edits, implementation delegation, new probes or test reruns |

**Evidence labels:** source statements below are Implemented, inspected 2026-09-30. Executed
test claims are Tested only for the dated, attributed root receipt in §10. Proposed corrections
are not implemented or verified by this review. The model was held stable during inspection;
unrelated dirty-tree changes are outside the assessment.

The binding retains legacy `cpg-schema`/Delta mapping text. This review follows current DESIGN
§15 and ADR-0085/0086/0087 for the migrated boundary; the older mapping does not introduce a
second authority or change the decision here. STATUS is a dated checkpoint, not an A12 receipt.

## 2. Responsibilities, dependencies and semantic ownership

| Component | Responsibility and hidden decisions | Consumer contract and dependency direction | Change owner |
|---|---|---|---|
| `domain::flow` | Raw test/leaf/load/value assertions, ordered call-path identity, relational source geometry | Providers state nominal observations; shared invariants govern construction/persisted admission; no call-summary proof is inferred | Flow vocabulary and path validity |
| `conditions::graph` | Transient topological Boolean expression with provider-local leaves, root closure and approximation | A13 can emit expressions; A14 maps leaves to EvaluationAtom IDs; `Diagram::from_graph` consumes that mapped graph | Expression representation and lowering contract |
| `conditions::kernel` | Bounded canonical BDD algebra and atom-to-variable mapping | Graph lowering uses the existing kernel, not another persisted Boolean representation | Boolean mechanism and limits |
| `assertion` | Generated supports, qualified subject lineage, run/surface/evidence/coverage authorization | New FlowValue/FlowCallPath/EvaluationAtom subjects traverse their owned sources | Attribution and source authorization |
| `domain::mod` | Facts relation enumeration | Arrow and PG lowerings derive from the same Domain declarations | Facts frontier composition |
| PostgreSQL shared fixture consumer | Sealed generation validation of the same declarations | Owns persistence effects; does not independently define flow semantics | Store realization, outside new semantic authority |

The separation is appropriate: path geometry belongs in the model, analyzer-local indices stay
transient, and the PG fixture exercises the permanent lowering. F01 concerns missing governing
rules, not the location of the rules. F02 concerns the metadata contract at a transformation seam.

**CI fact and fidelity table**

| Fact family/relation | Provider/revision and fidelity | Coverage/unknowns | Identity | Consumer |
|---|---|---|---|---|
| Flow tests, leaves and attribute loads | Qualified assertions plus generated supports retain provider run/revision, native mode and fidelity; fixture provider is `flow-fixture` revision `1`, not a real analyzer | FactFamily::Flow coverage; raw observations do not establish runtime transfer or normalized leaf/type meaning | Qualification and nominal test/atom/operand/event fields | Planned A14 and P3 |
| Flow value paths | Qualification matches the linked FlowValue; support traverses value-use and path occurrences | Through-call is explicit; missing/extra path link is refused | Ordered call/operand/role digest and ordinal rows, independent of insertion order | Planned A14/P3; shared invariant currently incomplete (F01) |
| Mapped condition graph | Input atoms are occurrence/context keyed; graph indices are local references | Refused nodes and approximation flag exist, but conversion loses the flag (F02) | Only canonical Condition/ConditionNode IDs persist | Condition qualification and later analyses |

## 3. Contracts, constraints and testing boundaries

| Contract | Inputs/outputs and enforcing boundary | Failure/effects | Independent verification and limitation |
|---|---|---|---|
| `FlowCallPath::new` and `flow_call_path_structure` | One to 256 ordered `(call, operand, role)` steps; digest re-derived; CallSyntax/CallArgument role agreement; same-source containment; one link for each through-call value | ModelError refusal, pure validator state | One-step fixture and ordinal/role/source/digest/link negatives; missing chain/use completeness is F01 |
| Flow test/leaf source structure | Atom evaluation lies within test, optional operand lies within evaluation; test and declared scope share source | ModelError refusal | Foreign-source atom and operand negatives; actual ty geometry remains A14 |
| Generated support validation | Sources of atom lineage, value use/sink, path calls/operands must belong to the qualified coverage; atom contexts agree | Pure checks; support is separate from assertion identity | Same shared validator runs in memory and sealed PG contents |
| `Diagram::from_graph` | At most 4096 nodes; valid root; all edges point backward; build only root closure; call bounded BDD operations | MalformedGraph/WorkPreflight/etc., no I/O | Truth table, commutative canonical identity, malformed edges/root, graph cap and orphan refusal; approximation conversion untested and incomplete (F02) |
| `obligation::from_kernel` | Kernel refusal maps to typed obligation reasons; MalformedGraph currently uses ConditionTransferUnsupported | No truth value is required on a refusal | Boundary conversion remains explicit; no new malformed-reason code is required solely by this review |

Local model tests need fixture state but no acquisition, analyzer or store. The PG test needs a
real disposable PG18 database because its purpose is to exercise stored validation. This is
appropriate test isolation, not evidence that a producer establishes the recorded coordinates.

## 4. Composition and execution

Expression composition preserves `approximate` in `not`/`and`/`or` and `map`. At the conversion
boundary, `from_graph` validates the entire graph and then visits only the root closure. This
keeps orphan scratch nodes out of truth and rejects cycles/forward references without recursive
graph evaluation. BDD variable indices remain private to the kernel; persisted identities use
nominal atoms and canonical nodes. However, the final Diagram has no approximation member and
the conversion never reads the graph flag (F02).

Call paths are structural vocabulary, not resolved calls or transfer proofs. Their relationship
to a FlowValue is an attributed assertion. Geometry and digest validation compose with generated
support validation, but source authorization cannot establish operand-chain continuity (F01).

**CI analysis record for the only analysis-like transformation in scope:**

| Question | Universe/method | Exactness/model | Budgets and partial behavior | Output/evidence |
|---|---|---|---|---|
| Which assignments satisfy the provider expression? | Root closure of a validated topological Boolean graph; AND/OR/NOT over nominal EvaluationAtoms; no topology projection or seed | Exact Boolean algebra for exact graph inputs; approximate-input contract is deficient (F02) | 4096 expression nodes; existing kernel atom/node/pair limits; cumulative pair work plus retained result-node count capped at MAX_PAIR_WORK; refusal returns Err | Canonical Diagram/Condition/ConditionNodes; atom lineage remains in shared model |

Resource ownership is explicit for validators: FlowStructure, PathCheck and support indexes use
StateCharge and charged maps/sets tied to the attempt budget. Path cloning is bounded by 256
steps. The graph conversion retains each built intermediate, but cumulatively charges their node
counts against its fixed work cap; graph traversal vectors are capped by 4096. These are inspected
hard bounds, not a measured memory/scale claim or a new shared-budget stress-test receipt.

## 5. Change and failure scenarios

| Scenario and kind | Expected owner/affected consumers | Observed result and settling check |
|---|---|---|
| `f(g(x), h(y))`, new composition of existing call/argument instances | Flow path invariant; providers/PG use the same rule; no new concept or provider-specific classification | An x path can select outer operand h(y) and then inner g(x); only nested calls, not selected operands, are checked. F01 |
| `f(g(x))`, skipped or repeated path element | Same path owner; explicit outer-to-inner chain semantics should be reusable | A one-step inner-only path or repeated same call can satisfy current containment/digest checks. The validator also never consults FlowUse. F01 |
| Provider reports conservative/approximate condition or hits expression cap | Graph/kernel seam carries exactness or refuses; future A14 should consume the owned outcome | Approximate atom lowers indistinguishably from exact atom; simplified refused expression can lower to canonical false/true. F02 |
| Provider upgrade replaces local leaf representation | Generic graph/map owns representation conversion; A14 owns attachment | Provider handles need not become persisted IDs; malformed graph is explicitly refused. The approximation outcome must first be repaired |
| BDD mechanism substitution | Existing Diagram/kernel boundary owns algebra and canonical atom mapping | No new provider framework is needed. Canonical identities and independent truth-table controls are relevant obligations; an alternate backend implementation is outside A12 |

These address the CI analyzer-upgrade and composed-fact journeys. New serving answers, retrieval
workloads and gold evaluation are excluded because A12 does not implement or expose those paths;
their first consumers at P3–P5 and qualification Q are the revisit triggers.

## 6. Correctness and fidelity gates

| Gate | Verdict | Evidence/action |
|---|---|---|
| G1 Authority | pass | Domain declarations own identity/relations; generated support and PG lowering consume them; no second flow truth encoding is persisted |
| G2 Semantic fidelity | fail | F01 path meaning is not fully enforced; F02 exactness is silently discarded |
| G3 Validity | fail | F01 permits invalid path/value associations through the shared admission invariant |
| G4 Hidden behavior | pass | New model transformations and validators have explicit inputs and scoped state, without I/O or ambient provider lookup |
| G5 Consistency and recovery | fail | F02 can erase expression-limit/refusal metadata after Boolean simplification; hard size/work bounds themselves are explicit |
| G6 Transformation and reuse | fail | F02 changes the required exactness outcome at graph lowering; F01 permits structurally inconsistent path transforms |
| G7 Truthful capability claims | pass, bounded | Root receipt is attributed; no producer parity, product availability or phase exit is asserted |
| G8 Library leverage | pass | Canonical Boolean work reuses the established BDD kernel; transient graph/path validation is specialized model glue |
| CI-G1 Fidelity | fail | F01/F02 permit stronger or different meanings than the supplied raw structure/approximation supports |
| CI-G2 Evidence closure | n.a. | No served claims exist in A12; transitive support/source authorization was inspected, but serving closure requires P5 |
| CI-G3 Evaluation integrity | pass, scoped | New transforms consume graph/model records, not gold/reference inputs; fixture expectation evaluation is separate from production lowering |

Gate verdicts are reasoned review judgments, not command outcomes. The passing focused receipt
does not offset the failed semantic gates.

## 7. Findings and applicability

<a id="F01"></a>

### F01 — Call-path admission does not establish the value's complete operand chain

**Required correction; Implemented defect, inspected 2026-09-30.**

- **Evidence:** `flow.rs:251–255` declares no FlowUse input; `flow.rs:286` checks the next call
  inside the previous **call**, rather than inside its selected operand; `flow.rs:290–295` checks
  qualification/through-call, uniqueness and only the first call's containment in the sink.
  `flow.rs:298–301` proves digest agreement and existence of a link, not path completeness.
- **Consequence:** for `f(g(x), h(y))`, steps `(f, h(y), Argument)`, `(g, x, Argument)` pass each
  typed role check and call containment despite being from different argument branches. A value
  for y can also reuse x's one-step path because its originating use is never inspected. Repeating
  a call passes non-strict containment; recording just g inside sink f can omit an outer crossing.
  A canonical digest establishes which bad sequence was stored, not that it is a valid dependency.
- **Principles:** FP-04/05, DP-02/03/07/08/23, CI-02/03; A2/A3, G2/G3/G6, CI-G1.
- **Correction (Proposed):** make `domain::flow` own an explicit, complete outer-to-inner path
  contract. Join the linked FlowUse occurrence; require each inner call within the previous
  selected operand and the originating use within the innermost operand; refuse repeated/skipped
  crossed calls under the declared syntax/source model. Define the supported outer sink boundary
  rather than treating any contained call as sufficient. Keep that rule in the shared invariant
  instead of teaching A14 or PG separate classifiers.
- **Closure evidence:** independent sibling-argument, wrong-use, repeated-call and omitted-call
  negatives, with valid nested argument and callee paths as positive twins. Exercise meaningful
  malformed-path cases through sealed real PG18 contents as well as model validation. Producer
  coordinates/parity remain A14; the admission contract is A12.
- **Disposition:** required before A12 acceptance; route to the [cutover plan §8](../../plans/semantic-model-cutover-plan_2026-09-29.md)
  under the A12 owner. This review does not edit the plan's current-status table.

<a id="F02"></a>

### F02 — Graph lowering silently drops approximation and simplified refusal metadata

**Required correction; Implemented defect, inspected 2026-09-30.**

- **Evidence:** `conditions/graph.rs:9,14,18,25–31` represent/propagate approximation; the whole
  `Diagram::from_graph` implementation at `graph.rs:55–84` ignores that member. Diagram in
  `kernel.rs:14` contains only support/context/BDD. `from_kernel` cannot recover a flag from an Ok.
- **Consequence:** `CondGraph::atom(a).with_approximation()` returns the same Diagram and
  canonical condition ID as the exact graph, without an output exactness/refusal state.
  `CondGraph::refused().and(&CondGraph::never())` becomes False with `approximate=true`, then
  lowering returns canonical false; the corresponding OR with always returns true. Boolean
  simplification itself can be legitimate, but losing why an input was approximate/refused is
  not an explicit exactness policy. A future A14 caller must independently remember metadata or
  can feed that bare Diagram into exact verdict construction.
- **Principles:** FP-02/03/04/05, DP-02/08/11/19/23, CI-02/04/06/08; A2/A3, G2/G5/G6, CI-G1.
- **Correction (Proposed):** make the owning conversion return a typed diagram-plus-exactness/
  boundary outcome, or refuse approximate inputs until such a contract is supported. Disclose
  any safe constant-simplification policy and preserve the refusal/approximation outcome through
  qualification. Exactness need not become part of Boolean condition identity; it must remain
  part of the governed conversion result.
- **Closure evidence:** controls for approximate atom and constants, refused AND false / OR true,
  negation and mapping, and exact positive twins. Demonstrate that the returned contract makes
  the required qualification state unavoidable rather than relying on a caller's convention.
- **Disposition:** required before A12 acceptance; same [cutover-plan §8](../../plans/semantic-model-cutover-plan_2026-09-29.md)
  route, with `conditions::graph`/kernel conversion as owner and A14 as the first real consumer.

**Foundation verdicts:** FP-01 and FP-06 satisfied by the coherent model/kernel/support/store
boundaries and isolated tests. FP-02, FP-03, FP-04 and FP-05 violated for F01/F02's supported
scenarios. DP-01/04/05/13/14/17/18/22 and CI-01/10/12 are satisfied within the inspected boundary;
the violated rules are cited per finding. CI-05 topology projections, CI-09 heuristics and CI-13
serving are outside A12. No SHOULD exception is used to waive either MUST gap.

## 8. Library fit and total complexity

The implementation reuses the existing biodivine BDD kernel for Boolean operations and canonical
atom mapping, and the existing Domain/Assertion machinery for Arrow/PG declarations and support
validation. The new graph is a transient expression carrier for provider migration, not another
BDD engine. A plain recursive AST is the simplest competing carrier, but does not by itself solve
provider sharing, topological validation or exactness propagation. The bounded flat graph is a
reasonable choice when its outcome contract is repaired.

Call-path digest and source-role validation are specialized relation semantics that a generic
graph traversal cannot establish. The simplest correction is an ordinary owned validator using
the existing typed syntax and FlowUse relations. No new crate, registry, solver or backend adapter
is justified. This comparison concerns inspected integration shape; no new library API claims,
Context7 lookups or performance claims are made.

## 9. Alternatives and tradeoffs

| Alternative | Locality/authority and burden | Decision |
|---|---|---|
| Current validator plus provider-side discipline | Provider must reconstruct missing path semantics; direct model/PG inputs can bypass that convention | Revise; violates the shared admission boundary |
| Complete path validation in `domain::flow` | One authoritative geometry rule reused by memory/PG; additional bounded syntax/use indexes | Preferred direction for F01; producer known answers stay independent |
| Bare Diagram plus caller reads `graph.approximate` | Minimal type changes, but every consumer independently carries a decision-relevant outcome | Insufficient governed contract for F02 |
| Typed lowering outcome, or explicit refusal of approximate graphs | Keeps Boolean representation canonical while naming exactness; small consumer migration at A14 | Preferred directions; select the narrowest supported behavior |
| Parallel DNF/persisted provider expression or a new generic framework | More truth authorities and lifecycle than A12 needs | Rejected under current DESIGN §15; no adoption requirement |

## 10. Verification and uncertainty

| Claim | Evidence/date | Command/cases | Outcome or limit |
|---|---|---|---|
| Focused model/PG suites ran successfully | Tested, root-attributed receipt 2026-09-30; not executed by this reviewer | `python3 scripts/build_environment.py -- cargo test --release -p lctx-model --test domain_flow --test domain_conditions -p lctx-postgres --features testing --test domain_flow` | **passed**, 4 model flow / 5 conditions / 1 PG flow tests; PG uses real disposable PostgreSQL 18 |
| Independent graph truth expectation | Implemented test inspected 2026-09-30; exercised by attributed receipt | `transient_graphs_have_one_bounded_canonical_lowering`: 8 assignments for `(a && !b) || c`; commutative identity; malformed/cap/orphan cases | Useful independent expected Boolean values; no approximate/refused-simplification control |
| Path validity controls | Implemented tests inspected 2026-09-30; exercised by attributed receipt | `raw_tests_leaves_and_call_paths_validate_without_provider_indices`: one-step path plus 8 negatives; empty/257-step constructors refused | No sibling/nested-chain, originating-use, repetition or omitted-step controls |
| PG lowering/validation | Implemented shared test inspected 2026-09-30; exercised by attributed receipt | `raw_flow_and_transitive_place_provenance_survive_sealed_validation`: valid extended fixture and foreign-place refusal | Confirms shared semantics through stored contents; foreign control targets existing place provenance, not the new multi-step path semantics |
| Producer migration/parity, Q and full gates | Not in review scope | A13/A14 suites, facts pilots, `just test-all`, `just fmt` | **not_run**; remain open, no phase/product acceptance inferred |
| Review artifact publication/catalog | Parent session coordination | `just docs-check`; final `just library-catalog` | **not_run by reviewer**; root owns the session's final checks and catalog so concurrent review does not regenerate shared outputs |

F01/F02 are source-reasoned defects, not falsely reported failed test executions. Existing positive
fixtures and a shared validator do not independently challenge the missing semantics. No measured
scalability, current native provider fidelity or assembled P0–P2 qualification is established.

## 11. Authority changes and dispositions

The [cutover plan](../../plans/semantic-model-cutover-plan_2026-09-29.md) remains the current
execution/disposition owner. Record this review's stable F01/F02 IDs there when assigning repairs.
Repair the governing model contracts before A13/A14 consume them. If the correction changes an
enduring exactness or call-path semantic decision, update DESIGN §15 and use the existing ADR
route; implementing an already-required invariant alone does not require another ADR.

No finding is assigned to Q merely to keep A12 green. A13/A14 producer correctness, P3
normalization and P4 through-call transfer discharge are separate open obligations. They do not
justify accepting structurally invalid raw paths or an exactness-erasing conversion today.

## 12. Architectural judgment and decision

| Judgment | Verdict | Evidence/action |
|---|---|---|
| A1 Localize change | satisfied, bounded | Graph, geometry, attribution and persistence have coherent owners; F01/F02 can be corrected at those owners with reused consumers |
| A2 Encode domain meaning explicitly | violated | Nominal types exist, but the stored path operation does not enforce its complete meaning and graph conversion loses a consequential outcome; repair F01/F02 |
| A3 Extend through composition | violated | Adding nested paths or approximate expressions requires hidden provider/caller discipline instead of governed composition outcomes; repair the shared seams |

**Bounded decision: Revise.** Retain the model/kernel/store ownership structure and repair F01/F02
with independent negative controls. Re-inspect those contracts before marking A12 accepted.

**Enclosing architecture:** not certified; A13/A14, Q and the assembled P0–P2 review remain open.
This review is a bounded, dated assessment of the in-progress contracts and attributed focused
tests, not release qualification or a current-product availability claim.
