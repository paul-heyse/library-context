# Stage 3 architecture calibration

## 1. Scope, outcome and coverage

**Decision: Revise.** The implemented model catalog, shared condition kernel and isolated SCC
adapter provide useful composition boundaries. Finite summary construction does not yet provide
the same local testing boundary. More urgently, an independently maintained native proof-kind
list rejects a proof kind the compiler already produces. These are distinct architectural and
contract findings; the review does not infer Stage 3 completion from focused test history.

| Field | Value |
|---|---|
| Subject | [Behavioral-model forward plan](../../plans/behavioral-model-forward-plan_2026-09-24.md), Stage 3 target, and its current schema/core/analytics/native/serving implementation |
| Revision | `main`, `1b86c50fbad6ab98840a805b8238fa195950974b`; inspected 2026-09-25 |
| Dirty-tree limit | Review skills, core/profile/binding, AGENTS/CLAUDE, DESIGN and ADR documents were already dirty. The plan acquired process/disposition edits during inspection; its diff was read. Production files cited below were committed. This is a review of those production files against the current working-tree standard, not a clean-commit certification. |
| Standard | Core/template 3.0, code-intelligence profile/review additions 1.1, current library-context binding, loaded through `standard.toml`; current ADR-0040 is accepted in the working tree but uncommitted |
| Tier / purpose | Design / target; independent calibration of the revised process |
| Reviewer / date | Independent `calibrate_architecture` reviewer, 2026-09-25 |
| Intended outcome | Decide whether the next normal-completion, SCC, model and serving extensions have coherent owners and contracts |
| Fact/answer scope | Model assertions and applications; condition/proof catalogs; finite value summaries and unknown boundaries; path-local native inspection; adjacent operation lookup/filtering |
| Exclusions | No production edits, executable tests, builds, integrated gates, pilot, plan/ADR/STATUS edits, or other calibration reviews. Full analyzer extraction fidelity, retrieval quality and publication crash recovery were not re-audited. |
| Maturity | Stage 3 is incomplete. Operation-wide compatibility, effect/role filters, recursive summary composition and full proof-span serving remain target obligations. |

**Evidence convention.** All implementation observations below are **Implemented, source-inspected
2026-09-25**, or **Interface-checked** where only a boundary is traced. Corrections are **Proposed**.
No benefit is Measured and no runtime result is newly Tested. Existing test source is evidence
of intended controls and test setup, not a fresh pass. STATUS's interrupted integrated gate is
historical context only.

### Evidence map

Paths and line anchors refer to the inspected checkout; each cited expression was read.

| Ref | Source and what it establishes |
|---|---|
| E1 | `crates/cpg-schema/src/models.rs:28–85, 584–654, 662–960`: typed model/channels; strict parse; embedded catalog bytes/digest; pinned binding and rule compilation. `crates/cpg-schema/models/external.toml` contains existing instances. |
| E2 | `crates/cpg-core/src/attempt.rs:466–506, 800–922`: acquisition, model compilation and ordered relation writes; finite summaries precede complement boundaries. `crates/cpg-core/src/validate.rs:442–523, 975–987`: publication reuses the summary/model producer and compares stored results. |
| E3 | `crates/cpg-core/src/summaries.rs:19–62, 241–305, 366–454, 499–590, 592–952`: session-bound acquisition, direct proof policy, finalizer insertion, modeled/assignment/local-call composition and depth refusal. |
| E4 | `crates/lctx-analytics/src/summaries.rs:19–174, 176–229`: input-only SCC scheduling and predecessor compatibility, with local controls. `crates/lctx-analytics/Cargo.toml`: no DataFusion/Delta dependency. |
| E5 | `crates/cpg-schema/src/behavior.rs:1268–1340, 2082–2153, 2381–2435, 2825–2933`: summary/step/boundary contracts, predecessor regions, complement reasons and finalizer proof relations. |
| E6 | `crates/cpg-schema/src/codebook.rs:8–16, 1332–1348`: authoritative proof vocabulary including `FinalizerPass = 10`. `crates/cpg-core/src/bundle.rs:216–236`: serving projection derives step text from that codebook. |
| E7 | `python/lctx_semantics/src/lib.rs:175–233, 266–392`: native summary carrier, constructor, independent proof-kind admission, proof ordering and callee checks. |
| E8 | `python/lctx_mcp/src/lctx_mcp/generation.py:404–515`: immutable-generation validation, condition-catalog merge and native transport. `python/lctx_mcp/src/lctx_mcp/value_paths.py:20–99, 133–220`: exact input, generation/query-bound pagination, path-local verdict and work rendering. |
| E9 | `crates/cpg-core/tests/compile.rs:55–83, 620–654, 666–755, 828–878`: extraction/analysis/Delta setup and independent expected finalizer controls. `python/lctx_mcp/tests/test_native_semantics.py:61–106`: native proof controls; `python/lctx_mcp/tests/conftest.py:17–21` and `crates/cpg-core/tests/bundle.rs:303–316`: serving fixture comes from `analysis_shapes`. |
| E10 | `crates/cpg-schema/src/condition_kernel.rs:9–56, 85–123, 237–249, 572–663`: shared hydration, explicit boundary reasons, bounded library operations. `crates/cpg-schema/src/primitive_theory.rs:137–175`: reusable exact-input assessment. |
| E11 | `python/lctx_mcp/src/lctx_mcp/operations.py:1–11, 57–76, 127–167`: materialized facet/operation contracts; no effect/role or compatibility filter yet. DESIGN §11.3 explicitly limits current native inspection and records missing full step spans. |
| E12 | Forward plan, detailed Stage 3 queue orders 1–10 and Stage 3 execution contract; DESIGN §9.9, §11.3; ADR-0034 and ADR-0039. These specify the intended ownership, remaining semantics and acceptance boundaries. |

## 2. Responsibilities, dependencies and semantic ownership

| Component | Responsibility and hidden decisions | Consumer contract / dependency direction | Reason to change |
|---|---|---|---|
| `cpg-schema` model domain | Authored model meaning, target pin, five independent coverage channels, canonical paths and compiled assertions | Pure typed inputs to `Catalog::bind_targets`/`compile_rules`; core supplies context rows and publishes results (E1–E2) | Another existing-kind model is primarily data; a dynamic-schema effect is genuinely new meaning |
| `cpg-schema` conditions and primitive theory | Evaluation identity, bounded Boolean structure, exact-input interpretation and value-link requirements | Compiler and native consumer share `Diagram`, hydration and theory APIs (E10) | New atom/origin semantics or explicit budget policy |
| `cpg-schema::behavior` | Arrow contracts and relational source/model/exit candidate construction | Named DataFusion relations with declared table dependencies; core executes them (E5) | New source fact, supported completion case, or migrated proof/boundary identity |
| `cpg-core::summaries` | Acquires candidate relations and currently also decides finite proof admission/composition | Public entry point is `finite_flows(&SessionContext)`; returns rows and steps only (E3) | Next predecessor completion, conditional callee, recursive and effect summaries |
| `lctx-analytics::summaries` | SCC topology and bounded predecessor compatibility | Immutable ids/edges/rows/diagrams in, typed results out; no store (E4) | New graph scheduling or program-analysis rule |
| `cpg-core::attempt` / `validate` | Sequence acquisition, derivation, writes and reconstruction checks | Canonical rows validated before snapshot publication; reused producer is an integrity check (E2) | A new required relation or changed publication invariant |
| `cpg-core::bundle` / Python generation load | Rebuildable projection, schema/manifest/file checks, native initialization | One FORMAT 7 generation, canonical snapshot and condition format (E6, E8) | Served field or contract-version change |
| Native executor / Python tools | Native condition/proof admission and path-local assessment; Python request validation and response rendering | Constructor transports one generation; typed query result, explicit unknowns and cursor scope (E7–E8) | More semantic queries or another rendering |

Representation flow is source/context → attributed relations → candidate relations + conditions
→ finite proofs → validated snapshot → bundle → native index → Python response. The compiler
and native index depend inward on the schema/condition domain. The finite proof policy currently
sits with session orchestration rather than behind the input-only analytics boundary.

| Concept | Authority / revision boundary | Derived consumers |
|---|---|---|
| Model meaning | Strict tagged model + pin/revision; catalog bytes contribute to identity/digest | Typed target/rule/formal rows; shared compiler reconstructs them |
| Condition | Evaluation atoms and content-addressed structural roots; kernel format | Provider/analysis catalogs, native hydrated diagrams; display text is not the decision authority |
| Positive path identity | Canonical callable/formal/input/output/condition/exit/ordered proof recipe, ADR-0034 | `summary_flows`, ordered steps and served projection |
| Proof vocabulary | `SummaryFlowStepKind` codebook | Bundle derives text; native loader independently repeats allowed text, already divergent (F01) |
| Refusal | Condition kernel has typed causes; finite summary producer discards several causes | Separate complement relation assigns generic reasons from source shape (F03) |

**CI fact and fidelity table**

| Family | Provider/revision and fidelity | Coverage/unknowns | Identity and consumers |
|---|---|---|---|
| Flow/region observations used by summaries | Pinned ty flow line and source attribution; provider facts are not completion proofs | Missing/approximate/incompatible conditions constrain admission | Source fact ids and condition roots; relational candidates and summary policy |
| Model assertions/applications | Committed catalog, pinned context definition/signature; `synthetic_model` origin | Separate complete/partial/unspecified channels; dormant unresolved target is not behavior | Model/rule/context-definition references; source candidates, future summary families |
| Finite summaries | Compiler-derived may-path under source/model abstraction | Direct, modeled, unique assignment and unconditional acyclic paths; open siblings remain unknown | Canonical summary id and ordered steps; native path inspection |
| Summary boundaries | Compiler-derived complement of selected source returns | Empty positive set never becomes a negative result; exact refusal cause can be lost (F03) | Aggregate callable/formal/raw-fact/condition key; native open-boundary list |
| Exact-input result | Shared primitive theory with checked entry-value links | Path-local refutation or may-model compatibility; absent links and limits remain unknown | Same generation, operation/formal and summary path; Python only renders result |

## 3. Contracts, constraints and testing boundaries

| Contract | Preconditions / enforcement | Effects and failure | Isolated verification |
|---|---|---|---|
| Model declaration → compiled assertions | Unknown fields, channel/rule consistency, complete signatures and target pin; model compiler reused by publication | Pure after input rows supplied; explicit error or dormant target | In-memory model/context rows are sufficient (E1). Ordinary instances need no server dispatch. |
| Condition catalog → diagrams | Canonical atom/root/node closure and format checked by shared hydration | No I/O; typed caps and rejected malformed catalogs | Direct Rust inputs and independent Boolean controls (E10) |
| Candidate returns → finite proof | SQL eligibility plus Rust conditions/argument/exit checks; reconstructed at publication | Query-session reads interleaved with policy; rows/steps returned, several refusal reasons discarded | Current public API needs registered source/candidate tables. Existing summary controls run extraction, analyzed compile, store and publication (E3, E9; F02). |
| Snapshot → served proof index | Manifest/schema checks, condition hydration, dense proof ordinals and callee references | Generation loading performs declared I/O; invalid load rejected | Native tests can supply tuples, but producer/native contract currently disagrees (F01) |
| Formal + exact primitive → path page | Operation/formal resolution, checked links, query/generation cursor, bounded page | Pure query over loaded index; unknown/refuted/compatible and truncation distinct | Primitive theory itself is isolated; end-to-end serving fixture is a separate necessary boundary check |

Reconstruction by the production producer detects tampered/missing stored results. It does not
independently establish Python semantics. Source fixtures and external execution controls must
continue challenging those semantics after any refactor. No test is proposed solely to mirror a
new helper's implementation.

## 4. Composition and execution

| Stage / question | Inputs, method and scope | Policy vs orchestration / limits | Output and evidence |
|---|---|---|---|
| Which model applies? | Pinned context target/signature + strict model data; source application joins | Model module owns meaning; core coordinates writes | Attributed candidate assertions; no automatic positive summary |
| What is the call schedule? | All declared release functions and attributed caller→callee pairs; directed SCC topology | Petgraph `tarjan_scc`; sorted ids and canonical tie ordering. Parallel pairs can collapse for SCC membership without erasing path proofs, which use source facts separately. | Canonical component/member/order rows; topology confers no behavioral verdict |
| Can predecessor conditions coexist? | Cited predecessor/reaching/successor roots; bounded BDD conjunction | Exact only over declared atoms; loops/unavailable roots/caps remain unknown | Compatibility row with source ids and reason (E4) |
| Which finite return path is admitted? | Candidate joins, argument proof, condition implication, exit evidence and callee summary | Four producer paths currently embedded in session-facing code; recursion withheld for modeled/assignment/local-call paths; local depth limit 8 | Rows + canonical ordered steps; generic complement built afterwards |
| What can this exact input refute? | One generation's paths, checked links and exact primitive | Shared theory; Python validates request and aggregates reported work | Path-local result and link spans, explicit note against operation-wide interpretation |

The SCC projection is deliberately weaker than the summary relation: it schedules possible
dependencies, while source target closure, modality and execution witnesses decide admission.
This separation is sound in the inspected code. The planned recursive worklist still needs its
monotone domain, budgeted outcome and invocation diagnostics implemented; its existence is not
established by the SCC schedule.

## 5. Change and failure scenarios

| Scenario / trigger | Owner and expected contract change | Observed propagation, decisions and test setup | Assessment |
|---|---|---|---|
| S1: add another pinned pure helper using an existing identity category | Model catalog adds target/rule/coverage; no new category | `compile_rules` lowers the instance; core and validator reuse it. Data crosses bundle/native through existing summaries. Exact binding/normal-return evidence still must exist. | Locality/composition satisfied for a genuinely existing category, Interface-checked. New dynamic-schema meaning is not this scenario. |
| S2: admit a compatible predecessor that completes normally, then a finite recursive base | Summary domain receives explicit completion evidence and extends composition | Current implementation adds joins in `behavior`, policy inside `finite_flows`, then likely another native proof kind. No input-only finite transform can exercise the new rule; existing tests require unrelated brief/embedding configuration and Delta compile. | F02; target owner already named as `lctx-analytics::summaries` in the plan. Reuse its existing pattern instead of introducing a framework. |
| S3: carry the already implemented pass-finalizer proof into serving | Proof vocabulary and generation format should carry an admitted kind | Compiler emits `FinalizerPass`; bundle derives `finalizer_pass`; native independent match rejects it. Failure occurs while loading the generation, before a query. | F01, a concrete incompatibility established by source trace. |
| S4: explain why an acyclic wrapper was withheld at depth or BDD work limit | Summary owner returns a typed refusal with the candidate identity; serving renders it | Depth ≥8 and unsuccessful condition implication use `continue`; complement SQL has only call-transfer/control categories. It cannot recover whether a kernel cap, missing root or unsupported control caused refusal. | F03; additional SCC iteration limits would repeat this loss without an outcome contract. |
| S5: add an explanation rendering / operation-wide filter | Rendering consumes canonical proof/evidence; semantic executor owns aggregate coverage | Python already avoids its own BDD interpretation. Full step source spans and operation-wide channel coverage do not yet reach the serving contract; DESIGN and queue order 9 acknowledge that. | Unresolved target obligation. Do not infer aggregate completeness from a path page or reanalyze sources in a new renderer. |
| S6: add an analytic over existing facts | Typed projection → isolated analytic → attributed rows | SCC and predecessor APIs can be called with immutable values; graph adapter retains canonical id mappings and parallel arc evidence. Integration legitimately binds invocation/writes. | Satisfied for inspected analytics boundaries; new extraction APIs are unnecessary for this scenario. |

An analyzer upgrade was considered only at the consumer seam: the inspected downstream summary
code reads repository rows rather than ty-private handles. The analyzer implementation and its
upgrade conformance were not inspected deeply enough to certify an upgrade.

## 6. Correctness and fidelity gates

These are source-review verdicts, not command outcomes, and do not offset A1–A3.

| Gate | Verdict | Evidence / scope and required action |
|---|---|---|
| G1 Authority | **fail** | Proof-kind authority is repeated and divergent in the native loader (F01). Model compilation and condition hydration otherwise have shared owners. |
| G2 Semantic fidelity | **unresolved** | Existing typed origins/coverage and path-local verdicts are explicit. Planned effect/exception/role composition and operation-wide absence semantics remain incomplete; F03 loses refusal specificity. |
| G3 Validity | **pass, scoped** | Inspected catalog/condition/summary publication paths have rejecting shared validation (E1–E2, E10). This does not certify all schema rules, all native evidence closure or unimplemented Stage 3 channels. |
| G4 Hidden behavior | **pass, scoped** | Model/kernel operations use explicit inputs; core query/store work and generation load are visible effects. F02 is a dependency/testability defect, not evidence of concealed mutation. Full analyzer ambient-input handling not re-audited. |
| G5 Consistency and recovery | **unresolved** | Generation identity and explicit unknown/truncation are present. Integrated publication recovery, recursive budgets and Stage 3 exit were not established here. F03 does not turn unknown into complete. |
| G6 Transformation and reuse | **fail** | A canonical emitted proof cannot survive the current native representation boundary (F01). Shared BDD use is positive evidence for the narrower condition boundary. |
| G7 Truthful capability claims | **unresolved** | DESIGN candidly labels partial path inspection and missing aggregate behavior; no overall acceptance evidence. Native delivery of the finalizer case fails its current contract (F01). |
| G8 Library leverage | **pass, scoped** | Inspected code uses DataFusion relations, petgraph SCCs, bounded biodivine operations and Serde/TOML. No clear library replacement for Python domain completion proof was established. |
| CI-G1 Fidelity | **unresolved** | Path-local exact-input outcomes preserve unknown/model scope. Full recursive, effect and operation-wide claims remain unimplemented and are not certified. No unknown-as-absent defect was established in the inspected Python page. |
| CI-G2 Evidence closure | **unresolved** | Compiler reconstructs summary evidence; native checks cited callee summaries. Full raw/model/exit step evidence is not a served resolvable contract yet (S5), as DESIGN states. |
| CI-G3 Evaluation integrity | **unresolved** | No evaluation executed; the compiler's complete input graph was outside this review. Existing separation in the plan is Proposed evidence, not a new audit pass. |

## 7. Material findings and applicability

<a id="F01"></a>
### F01 — Native admission repeats the proof vocabulary and rejects `finalizer_pass`

**Priority: high.** **FP-02/03/04; DP-01/08/24; A1/A2/A3; G1/G6.**

E6 defines and renders `FinalizerPass`; E3 appends it to direct and modeled/composed finite
proofs. E7's `matches!(kind.as_str(), ...)` at lines 318–330 independently admits only codes
0–9's spellings and omits `finalizer_pass`. Therefore a valid compiled generation containing
the pass-finalizer proof reaches `invalid summary proof step` in native construction. Because
the constructor processes all summaries, failure is not confined to a request for that path.
This is a source-established rejection route, not a newly executed failure.

**Proposed correction / owner:** the schema proof contract and native loader should share typed
vocabulary admission. Parse through the existing codebook's values or an owned conversion;
retain genuinely semantic per-kind checks in the native admission contract. Remove the duplicate
string whitelist. A universal registry, new crate or multi-backend abstraction is unnecessary.
Changing proof semantics later still legitimately requires implementing new semantic checks.

**Closure:** trace all current proof kinds across bundle/native admission and execute a generation
from `return_completion_shapes` through native load/query, including the ordered nested pass
proof and a malformed/unknown-kind rejection. Retain the independent expected finalizer order.
This closes compatibility only; it does not prove broader finalizer behavior.

<a id="F02"></a>
### F02 — Finite summary policy has no isolated transformation contract

**Priority: medium; blocks the next planned extension boundary.** **FP-01/02/03/06;
DP-08/16/17/23; A1/A3.**

`finite_flows(&SessionContext)` acquires conditions, finalizers, preceding calls, components,
modeled seeds, argument evaluations, assignment seeds and local seeds while it also selects
verdicts, composes proof steps and schedules reuse (E3). The seed carriers are private to core.
The next predecessor and SCC rules cannot be called through a finite-summary input/output
contract; a caller must reconstruct the session's relation universe. Current semantic controls
add extraction, analysis configuration, a fake embedder, Delta writes and publication (E9).
Those integration controls are valuable, but their setup is not required by the immutable proof
computation. The existing SCC/predecessor APIs demonstrate a simpler local boundary (E4).

**Proposed correction / owner:** prepare candidate/condition/completion inputs in core, then call
a pure finite-summary operation in the existing analytics summary owner (or an equivalently
bounded module if the dependency contract is preserved). Keep authoritative SQL eligibility in
its relational owner; do not move joins into ad hoc Rust. Return admitted proofs and typed
refusals (F03), and invoke the same operation from publication reconstruction. Delete the
replaced interleaved policy path after migration. No provider trait or new crate is required.

**Closure:** demonstrate the next normal-completion or bounded-SCC rule using explicit small
inputs without `SessionContext`, extraction, storage or serving, with one independently specified
admitted case and one withholding case. Preserve a smaller set of extraction-to-publication
controls for joins, provenance and tampering. This is a structural change and verification
proposal; no reduction in time or maintenance cost has been measured.

<a id="F03"></a>
### F03 — Summary construction discards the reason for refusing a path

**Priority: medium.** **FP-04/05/06; DP-08/12/21; A2/A3.**

In E3, failed/missing implication evidence skips modeled/assignment/local paths, and
`callee.path_depth >= MAX_LOCAL_PATH_DEPTH` skips a local composition. The returned tuple
contains only flows and steps. Core then separately calls `summary_boundaries`; its CASE
expression can choose only `call_transfer` or `unsupported_control_flow` from raw transfer
flags (E2, E5). It cannot distinguish the producer's depth cap from missing evidence or a
kernel work/node refusal. By contrast, predecessor compatibility already carries the actual
boundary reason (E4). Unknown remains unknown, so this is not evidence of an unsound positive.

For S4 and the planned SCC iteration limit, this is a concrete loss of diagnostic meaning and
ownership: implementing a new refusal inside the summary producer does not make its cause
available to publication or serving. Repeating the producer's eligibility logic in SQL to
recover the cause would add another semantic authority. The plan already requires explicit
budget boundaries; the current output contract does not carry enough information to meet it.

**Proposed correction / owner:** have the summary decision return a cited outcome for each
considered candidate, including refusal kind and affected identity. The relational coverage
complement may still identify candidates the engine did not analyze; it should preserve the
engine's specific refusal for candidates it did analyze. Reconcile the plan's existing
path-specific boundary-identity work in this same contract, avoiding a second status register.

**Closure:** a finite acyclic chain at the depth limit and a condition operation refused by its
work budget retain `budget_reached` (with useful subreason/work if the contract provides it)
through publication and native response; an unsupported source case remains distinguishable.
An independently specified unknown control must still withhold a positive. No such checks ran.

| Foundations / supporting rules | Judgment for selected scenarios |
|---|---|
| FP-01 and FP-06; DP-17/23 | **violated** at finite-summary policy/test setup (F02); **satisfied** for isolated catalog/kernel/SCC contracts |
| FP-02 and FP-03; DP-08/16 | **violated** at proof transport and finite-summary composition (F01/F02); new model instance follows the existing contract |
| FP-04; DP-01/06 | **violated** for proof vocabulary (F01); authored models and condition rules have coherent owners |
| FP-05; DP-12/21/24 | **unresolved** for complete Stage 3 lifecycle/coverage; F03 requires preserving known refusal causes |
| CI-02/04/06 | **satisfied for the inspected path-page interpretation**, unresolved for operation-wide target; no claim that aggregate summaries are complete |

The absent recursive/effect/role implementation and missing full served step spans are explicit
target obligations, not newly discovered claims of completion. They are not multiplied into
additional findings merely to restate the plan.

## 8. Library fit and total complexity

Pins were checked in `Cargo.toml`, `Cargo.lock` and `docs/pins.md`; the pinned `rust-reasoning`
and `rust-graphs` guidance was consulted. This is not an upgrade recommendation.

| Capability / consumer | Existing fit | Integration burden and choice |
|---|---|---|
| Boolean condition composition for summaries/native queries | biodivine-lib-bdd 0.6.3; local `apply` calls `binary_op_with_limit` and separately preflights node-product work (E10) | Keep shared `Diagram` and typed boundaries. It supplies Boolean operations, not Python completion or effect semantics. No SMT substitution is justified by these scenarios. |
| SCC schedule for finite summaries | petgraph 0.8.3 `tarjan_scc`; existing id mapping and deterministic component schedule (E4) | Keep the adopted graph mechanism. Canonical tie policy and provenance remain repository responsibilities; SCCs alone do not implement transfer fixpoints. |
| Candidate construction and integrity | DataFusion 55.1.0 relations, Arrow 59.3.0 rows; shared reconstruction | Shared Arrow types are intentional contracts. A forwarding abstraction around them would add machinery without addressing F02. |
| Model data parsing | Serde 1.0.229 and TOML 1.1.6 with strict tagged inputs (E1) | Keep data declarations for existing kinds. A new effect meaning needs domain code; adding a generic plugin system is not justified. |
| Native serving | PyO3 0.29.2 transports one generation and reuses schema/kernel | Keep native semantics and Python rendering split. Share proof-kind admission rather than adding a second interpreter. |
| Future recursive rule maintenance | Plan's bounded worklist; Ascent considered only on repeated-rule-family trigger | No new spike or framework now. First expose actual transfer inputs/outcomes; library adoption would still need the same domain proof and evidence contracts. |

## 9. Alternatives and tradeoffs

| Alternative | Local reasoning and composition | Authority / verification | Decision |
|---|---|---|---|
| Keep session-bound finite policy and update native whitelist manually | Fastest local edits, but next proof/limit change requires hidden coordinated knowledge | Existing mismatch and reason loss remain; integration fixtures carry every local rule | Revise (F01–F03) |
| Proposed prepared inputs + pure finite-summary operation in existing owner | Core joins/acquires; domain operation computes cited outcomes; writer and validator reuse it | One policy implementation; independent semantic controls remain necessary; a modest explicit input surface is added | Preferred direction, Proposed; shape should follow the next actual rule |
| Replace with a generic rule engine or provider framework | New lifecycle, rule vocabulary and diagnostic mapping before repeated need is shown | Does not automatically establish Python semantics or serving closure | Defer to documented Ascent trigger / a demonstrated variation need |
| Simplest immediate repair | Derive proof vocabulary admission and carry refusals from current producer before moving policy | Can repair F01/F03 without a crate split; does not by itself close F02 | Valid staged route; do not claim architecture closure from the first repair alone |

## 10. Verification and uncertainty

| Claim / activity | Evidence label, 2026-09-25 | Command / inspection and outcome |
|---|---|---|
| Current revision and dirty boundary | Interface-checked | `git rev-parse HEAD`; `git status --short`; `git diff -- docs/plans/behavioral-model-forward-plan_2026-09-24.md` — `passed` as read-only inspection |
| Proof-kind mismatch | Implemented / source-inspected | Read E3/E6/E7 and finalizer control source; source trace establishes the rejection branch. Runtime reproduction — `not_run`. |
| Summary isolation and lost refusal causes | Implemented / source-inspected | Read E2–E5/E9; no test-runtime or change-cost claim |
| Model/kernel/SCC boundary fit | Interface-checked | Source and pin inspection; no current-run qualification inferred from historic skill receipts |
| Focused compiler/native cases | Proposed closure checks | `cargo nextest run --release -p cpg-core`; `uv run pytest python/lctx_mcp/tests/test_native_semantics.py` — `not_run`; eventual commands should select the relevant cases and fixture |
| Integrated acceptance | Not established | `just test-all`; `just pilot`; Stage 3 question/evaluation and clean-wheel native query — `not_run` in this review |

The native fixture currently comes from `analysis_shapes`, whereas the compiler finalizer
controls use `return_completion_shapes`. This source-level coverage distinction explains why
the existing fixture boundary cannot be assumed to exercise F01; it does not assert that a
particular historical test run passed or failed. No generated artifact was loaded here.

## 11. Authority changes and dispositions

This report owns the dated findings. **Transferred 2026-09-25** after coordinating-session
source inspection: the [active plan §9.2](../../plans/behavioral-model-forward-plan_2026-09-24.md#6-findings-disposition)
owns current execution status, sequencing and closure evidence. Transfer does not assert an
implemented correction or change this review's judgment.

| Source finding | Current disposition owner |
|---|---|
| F01 | [ARC-01 — shared proof vocabulary](../../plans/behavioral-model-forward-plan_2026-09-24.md#ARC-01), grouped with slice-calibration F01 |
| F02 | [ARC-02 — isolated summary transformation](../../plans/behavioral-model-forward-plan_2026-09-24.md#ARC-02) |
| F03 | [ARC-03 — preserved refusal causes](../../plans/behavioral-model-forward-plan_2026-09-24.md#ARC-03) |

No accepted ADR is rewritten by this review. ADR-0039 already identifies predecessor completion
as its revisit trigger. ADR-0034's ordered finite proof remains suitable for the inspected cases;
a general proof DAG is not prescribed without a concrete multi-parent consumer. Source spans,
operation-wide coverage and integrated gates remain in their existing Stage 3 queue positions.

## 12. Architectural judgment and decision

| Judgment | Verdict | Scenario evidence and required resolution |
|---|---|---|
| A1 Localize change | **violated** | F02 forces normal-completion/SCC rules through session acquisition and integration setup; F01 requires hidden native coordination for a proof extension. Existing-kind model and SCC extensions are local. |
| A2 Encode meaning structurally | **violated** | F01 repeats the proof alphabet; F03 drops known refusal causes before the persisted boundary is constructed. Model/condition/proof identity structures are otherwise useful foundations. |
| A3 Extend through composition | **violated** | Finite-summary rules are not exposed as the same isolated capability used by the existing analytics functions, and current proof extension already breaks its adjacent consumer. |

**Bounded review decision: Revise.** **Enclosing Stage 3 architecture: needs revision for the
selected next-extension scenarios**, at source-inspection strength. This does not reject the
overall relational/graph/BDD architecture, certify other stages, or establish release readiness.

The next implementation decision belongs to the summary and native-contract owners: repair the
shared proof admission boundary first, then expose finite summary inputs/outcomes while adding
the next normal-completion witness. Preserve the typed catalog, shared BDD kernel, relational
candidate construction and independent semantic controls. Reassess these scenarios at the
assembled Stage 3 boundary after the plan's functional and serving acceptance work.
