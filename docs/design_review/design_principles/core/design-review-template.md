# Design review template

**Version 3.3 · 2026-10-05** · Core layer: repository- and domain-agnostic.
Standard: [design principles](design-principles.md): FP-01–FP-07 organize architectural
assessment; DP-01–DP-24 support it; A1–A4 and G1–G8 remain separate judgments.
Profiles add domain constraints within the slots. The binding supplies local owners and cadence.

Execution-fit assessment added 2026-10-05; existing slot identifiers and bounded review cadence remain.

## Part 1 — The review contract

### Tier and purpose

| Tier | Use for | Required content |
|---|---|---|
| Change | A bounded implementation within an accepted architecture | Scope, affected owners/contracts, one relevant change scenario or a reason it does not apply, applicable gates, findings, library implications, A1–A4 and decision. Compress slots into 1, 6, 7, 8, 12. |
| Design | A substantial stage, architectural decision, new mechanism or boundary; assembled architecture | Slots 1–12, scoped to the subject and adjacent consumers. Reconstruct responsibilities and dependencies before investigating mechanisms. |

| Purpose | Judged against | Handling a blocking repository authority |
|---|---|---|
| Conformance | Accepted architecture and applicable standard | Follow the authority; record the conflict and route to change it. |
| Target | Best available architecture for the functional target and its expected changes | Evaluate the blocking text as a candidate for revision; identify its replacement and decision route in slot 11. |

Depth follows impact and uncertainty. Legacy `compact`, `standard` and `deep` describe effort;
they are not additional tiers. Drop an irrelevant slot with a scope reason. Mechanical changes
can state that ownership, contracts and extension behavior are unchanged and cite the inspected
boundary. Do not narrow away an affected consumer to obtain acceptance.
Use the least investigation sufficient for the scoped judgments. Flow tracing is optional when
it resolves a concrete uncertainty; neither these slots nor A2 require a complete flow trace.

### What a claim can rest on

- **Document subject:** judge whether the architecture and obligations are specified and whether
  change scenarios have credible routes. Claims remain *Proposed*, or *Interface-checked* for
  inspected interfaces. A scenario walkthrough is not executed behavior or measured change cost.
- **Code subject:** cite the actual ownership, dependencies, definitions and executing paths.
  *Implemented* establishes existence; *Tested* and *Measured* name the commands, cases and
  conditions. Historical receipts are explicitly dated and attributed, never reported as fresh runs.
  Assess whether domain definitions govern behavior; domain-named output records alone do not
  establish model alignment.
- **Both:** distinguish implemented state, accepted target and superseded design. State whether
  a discrepancy is incomplete implementation or stale authority. Link executable contracts from
  prose rather than independently restating their definitions.
- Read cited evidence yourself. A prior review or helper's report is a lead to implementation
  evidence; it can also be cited as evidence about the process or its historical assessment.

### Foundation and supporting-rule verdicts

For applicable FP and DP/CI principles: **satisfied** (identified mechanism or reasoned scenario
supports the property), **violated** (a concrete scenario defeats it), **unresolved** (insufficient
specification or evidence). Give a scope reason for non-applicability. Aggregate related rules
where the same evidence settles them; avoid an exhaustive checklist without analysis.

### Finding standard

Group instances by structural cause. A finding can establish architectural damage before an
incorrect output occurs; semantic failure is not a prerequisite.

| Field | Adequate when |
|---|---|
| ID | `F01`, `F02`, … stable within the review; consumers cite `review#F01`. Use an explicit anchor when needed for a resolvable link. |
| Finding | A falsifiable defect in the architecture, contract or implementation. |
| Principles · judgment/gate | Only FP/DP/profile IDs and A/G judgments used in the argument. |
| Evidence or gap | Relevant owner, dependency, contract or executing expression; source path/line or design section, and what is missing. |
| Consequence | A supported input causes semantic failure, **or a realistic change requires duplicated decisions, unrelated internal edits, hidden knowledge, inseparable testing or unjustified machinery; or a supported workload suffers avoidable repeated full-input work, crossings/materialization, excessive live state, queues/fan-out, overbroad invalidation or mismatched resource/transaction lifetimes**. Name the scenario and causal mechanism; file counts alone do not establish it. |
| Correction | Owning boundary and direction, rough affected surface, alternatives and deletion obligations where relevant. |
| Verification | Evidence that would close the finding; a traced change or inspection can suffice. Add a test/probe only when it resolves uncertainty or protects a meaningful regression. |
| Disposition link | Where current execution status is owned, if assigned; otherwise explicit deferred trigger or required decision. A review remains a dated assessment. |

### Priority and decisions

Prioritize correctness/fidelity failures and architectural choices that make a supported workload
infeasible, unstable or operationally disproportionate. Structural cost evidence can establish
such a failure before measurement. Assess change barriers, repeated semantic ownership and other
material complexity in the same functional context; prioritize by consequence rather than by
whether the evidence is a benchmark. Resolving a defect may require changing a boundary.
Library adoption and bespoke code both carry complexity. A concrete architectural violation is
eligible for revision even when its supporting DP rule is a SHOULD.
The core §1 domain-model MUST is assessed through FP-04 and A2: both model adequacy and
authoritative realization must hold. Correct current outputs cannot compensate for that gap;
recording a deferral does not waive it for supported behavior.

| Situation | Decision |
|---|---|
| Applicable A1–A4 satisfied; no MUST gap or failed/unresolved gate in supported scope | Accept, at the stated evidence strength and scope. |
| Violation or unresolved architectural judgment for an in-scope change scenario | Revise; name the boundary or decision to resolve. |
| A scenario is deliberately excluded, with consequence, disposition and revisit trigger; remaining architecture and gates hold | Accept scoped; state separately what remains unresolved in the enclosing architecture. |
| MUST gap or failed/unresolved gate on claimed behavior | Revise, or remove that behavior explicitly from supported scope. A deferral alone does not make it acceptable. |
| G8 alone fails | Revise, for the demonstrated cost/extension defect. |
| Competing authority, silent semantic loss or unbacked capability at the core | Reject or Revise. |

A documented SHOULD deviation supports scoped acceptance only when the accepted scenarios still
hold. No aggregate score offsets a failed judgment. Separate the bounded slice decision from
architectural acceptance of the enclosing subsystem, and both from release qualification.

### Profile additions

Add domain content within the slots, marked with the profile prefix. Domain fidelity tables and
execution details support the architectural argument. They do not replace ownership, change
scenarios or local reasoning. The binding identifies a single current disposition owner per finding.

### Organize the review around its argument

The slots below identify content and stable references for profile additions. They are not a
required investigation sequence or a demand for one heading and table per slot. Group or
distribute explanations to suit the subject, while keeping the applicable assessment obligations,
foundation and gate judgments, decision and evidence limits discoverable. If a slot's content
appears elsewhere, link it where readers or profile references need a route. Presentation choices
do not change the selected standard's acceptance rules.

Lead with a concise integrated assessment: what the system intends to accomplish, which
responsibilities and choices shape its behavior, what supports the target, and which material
causes explain the problems. Develop the argument through coherent topics and concrete findings.
Several independent concerns may remain; no single root cause or overarching redesign is required.
Use tables for comparisons and navigation, and prose where causal reasoning needs explanation.
A compact finding index can link to detailed arguments instead of compressing them into wide cells.

A formal review has one accountable principal document. Substantial bounded analysis or evidence
may live in supporting documents when independently useful. The principal review owns the combined
scope, architectural explanation, relationships, judgment and evidence limits. Supporting material
names its narrower role, inspected baseline and relevant context and links back to the principal
review. Local judgments do not establish unexamined interactions or create separate overall
decisions. Current finding status remains at the binding-designated disposition owner.

Document boundaries should serve reader understanding. Keep tightly coupled reasoning together;
length, subsystem count and reviewer assignments do not dictate a split. Small reviews can keep
their argument compact in one document. Supporting analysis is optional and does not require a
new evidence folder, artifact hierarchy or registration process.

## Part 2 — The template

### 1. Scope, outcome and coverage

| Field | Value |
|---|---|
| Subject | Document/code paths and revision; dirty-tree limitations when relevant |
| Standard | Core version, profiles and binding |
| Tier · purpose | Change/design · conformance/target |
| Reviewer · date | Accountable reviewer and date |
| Maturity and outcome | Current design phase; what this decision should enable |
| Supported scope | Capabilities, adjacent consumers and guarantees; exclusions; brief workload premise from functional intent (operations, size/skew/growth, concurrency/deployment and resource envelope where material) |
| Expected changes | Selected realistic scenarios and why they matter now |
| Baseline | Existing architecture and material limits |
| Method and coverage | Examined/clean, unresolved, not examined; tests and assumptions |

### 2. Responsibilities, dependencies and semantic ownership

| Component | Coherent responsibility and hidden decisions | Consumer contract | Allowed dependencies/direction | Expected reason for change |
|---|---|---|---|---|

Use a small dependency diagram where useful. Distinguish compile-time dependencies, runtime
coordination and representation flow where they differ. A module is a sufficient owner when its
boundary holds; a crate split is not the objective.

| Phenomenon, concept or operation | Semantic scope, authority and identity where applicable | Update/revision boundary | Implementation, derived forms and consumers |
|---|---|---|---|

Assess whether the model captures consequential distinctions and behavior uses its definitions
within the review scope. Identify material duplicated decisions, semantics hidden in consumers,
private mechanisms exposed to consumers and opaque behavior. Use relevant contracts and source;
choose further investigation according to what remains uncertain.

### 3. Contracts, constraints and testing boundaries

| Contract | Inputs/outputs and consumer expectations | Preconditions/invariants and enforcement | Effects/lifecycle/failure | Substitution/compatibility | Isolated verification |
|---|---|---|---|---|---|

State meaningful absence states, equality/approximation promises and where invalid construction
is prevented. Explain necessary runtime enforcement and the dependencies needed to test it.
Assess verbs as well as nouns: applicability, inputs, outcomes, state changes, effects and the
invariants owned by each concept, relationship or operation. Distinguish repeated enforcement
from independent definitions. Ordinary domain functions can supply these contracts.

### 4. Composition and execution

| Capability or stage | Semantic inputs/outputs | Owning mechanism | Dependencies and reuse boundary | Policy vs orchestration | Effects, ownership and publication | Limits/determinism |
|---|---|---|---|---|---|---|

Assess how primitives compose. Identify rules that orchestration reimplements, implicit
ordering and special cases. Include cardinality/cost, identity mappings, representation loss and
resource budgets where material. A graph projection states its universe and relationship semantics.
Assess the complete physical operation: access paths, repeated work, crossings and intermediates,
optimizer visibility, reuse, enforcement frequency, transaction and recovery scope. Semantic
composition need not create physical stages or materializations. Distinguish examined-work and
output limits; safe refusal alone does not establish fitness for the workload.

### 5. Change and failure scenarios

| Scenario, trigger and kind of change | Owning component | Contract change | Expected vs observed affected consumers | Independent semantic edits/hidden knowledge/test setup | Evidence or settling check |
|---|---|---|---|---|---|

Choose relevant additions, analyzer/provider upgrades, new compositions/renderings, invariant
changes, implementation replacement and isolated tests. Use declared variation axes. New domain
concepts can legitimately change several contracts. Examine boundary round trips and
interruption/failure where claimed behavior requires them. Observed implementation traces and
proposed routes remain distinct.
Classify the change as an instance, binding, composition, policy, domain concept or execution
mechanism, and explain why the edits belong to the affected authorities. For a substantial
architecture review, examine a relevant domain extension and a mechanism substitution where
credible; give a scope reason when either does not apply. A bounded review still needs only its
relevant scenario. Assess semantic ownership and propagation, not a promise of inexpensive replacement.
Select relevant growth, high-degree/skew, concurrency or failure cases as well. Explain why the
physical route remains credible for the intended use; tiny fixtures cannot silently replace the
product's workload premise. Static reasoning may settle structural cost; measured claims require
measurements. These are lenses within existing slots, not a new campaign or checklist.

### 6. Correctness and fidelity gates

| Gate | Verdict (pass/fail/unresolved/n.a.) | Own evidence or scope reason | Required action |
|---|---|---|---|
| G1 Authority | | | |
| G2 Semantic fidelity | | | |
| G3 Validity | | | |
| G4 Hidden behavior | | | |
| G5 Consistency and recovery | | | |
| G6 Transformation and reuse | | | |
| G7 Truthful capability claims | | | |
| G8 Library leverage | | | |
| Profile gates | | | |

### 7. Findings and applicability

| ID | Finding and scenario consequence | Principles · judgment/gate | Evidence/gap | Correction and affected owner | Closure evidence | Disposition link |
|---|---|---|---|---|---|---|

Give verdicts for applicable foundations/supporting rules. State which boundaries already meet
the scenarios and the evidence for them. Keep observations distinct from actionable findings.

For consequential corrections, explain the intended operation and consumer behavior: inputs,
owned decisions, outputs or effects, preserved guarantees and intentional differences. Existing
strengths can constrain the remedy. Explain enough to distinguish a correction from relocated
complexity, while leaving routine implementation choices open. A narrow finding may need only
one sentence of correction.

Keep diagnosis and remedy maturity distinct. State what the evidence establishes, which direction
is proposed and what unresolved requirement or assumption affects that recommendation. A sound
finding remains actionable without a fully designed replacement. Challenge a consequential remedy
with a legitimate case it might wrongly reject, flatten or mishandle; reasoning may suffice.

Synthesize related findings through their causes and obligations. Identify shared causes,
independent defects, prerequisite contracts, alternative remedies and conflicting recommendations
where material. Group concrete manifestations without merging distinct closure obligations.
Explain whether the corrections fit together; neither shared terminology nor the same component
establishes a common cause. No mandatory dependency graph or additional analysis stage is implied.

### 8. Library fit and total complexity

Relevant entries in docs/library-utilization.jsonl, where available, may offer particularly
useful leads on established capabilities and integration patterns. Consulting them is optional.

| Capability and owner | Consumer or planned scenario | Built-in/library/bespoke candidates | Resolved-version semantic fit and gaps | Coupling/lifecycle/test/replacement burden | Choice and reason |
|---|---|---|---|---|---|

Inspect the resolved version's interfaces when making an API claim. A library's full surface is eligible for the
agreed capability; availability alone is not a requirement. An intentional shared library data
contract can be preferable to a forwarding abstraction. Compare composed capabilities, including
optimizer/bulk interfaces, physical access, locality, preparation/reuse, working set and recovery.
Account for generated objects and operator/developer obligations as well as handwritten code.
A built-in must fit the semantic contract and total integration. This is a focused comparison,
not a catalog.

### 9. Alternatives and tradeoffs

| Alternative | Change propagation and local reasoning | Semantic authority and composition | Test/substitution boundary | Total machinery and operational risk | Evidence, decision and revisit condition |
|---|---|---|---|---|---|
| Current baseline | | | | | |
| Proposed design | | | | | |
| Suitable library-owned alternative | | | | | |
| Simplest viable alternative | | | | | |

Rows can coincide or be out of scope with a reason. Explain what becomes easier and harder.
Preserve independent semantic controls when removing obsolete implementation-specific tests.

Relate expected benefits to their mechanism, relevant conditions and material ownership costs.
Explain which premise would change the recommendation and how it could be settled. Consider
external consumers, preserved meaning and intermediate states when transition feasibility affects
the choice. Detailed migration procedures and execution packages belong to subsequent planning
unless explicitly requested.

### 10. Verification and uncertainty

| Claim or scenario | Evidence label/date | Inspection/test/analysis/benchmark | Conditions and expected result | Current outcome or gap |
|---|---|---|---|---|

A traced extension, removed competing authority or bounded test setup can close an architectural
finding. Static review can suffice, including for library features and fit. The reviewing agent
judges whether complexity, criticality or unresolved uncertainty warrants creating and running
probes; this slot and the review tier do not require them or a separate evidence folder.
Independent oracles, when used, must challenge production contracts. Follow the binding's
integrated acceptance timing and retain the evidence required for Tested/Measured claims.

Distinguish unexamined breadth from a missing premise that could change the verdict or corrective
direction. Connect consequential uncertainty to the decision it affects and suitable settling
evidence. An unexamined area is not itself a defect. Evidence for a diagnosis does not automatically
qualify its remedy, and a refined remedy need not invalidate the diagnosis.

### 11. Authority changes and dispositions

| Required change | Decision route and owner | Source findings | Current disposition location | Closure evidence or revisit trigger |
|---|---|---|---|---|

Accepted decisions, implemented changes and verified outcomes are different facts. Exception
records follow principles §H. Current status belongs in one location; subsequent reviews link
there. Preserve original findings and versioned evidence rather than rewriting history.

### 12. Architectural judgment and decision

| Judgment | Verdict (satisfied/violated/unresolved/n.a.) | Scenario evidence and scope | Required action/disposition |
|---|---|---|---|
| A1 Localize change | | | |
| A2 Encode domain meaning explicitly | | | |
| A3 Extend through composition | | | |
| A4 Fit execution to the supported workload | | | |

**Bounded change decision:** Accept / Accept scoped / Revise / Reject, with reason.
**Enclosing architecture:** accepted for named scenarios / needs revision / unresolved / not
assessed (scope reason). State the evidence strength; a passing local slice does not certify it.
A4 needs a credible physical route for the workload premise. Material unresolved execution fit
prevents acceptance of that use; "performance unmeasured" does not excuse known amplification.
A documented cost is justified only by its retained benefit against a conforming alternative.

| Priority | Change and responsible component | Source findings | Closure evidence or revisit trigger |
|---|---|---|---|

Separate consequence-based priority from prerequisite order: a less urgent contract correction
may enable a more urgent consumer fix. Explain the capability or meaning required by a dependency.
Identify the next consequential decision and follow-up obligations without turning the review
into an implementation schedule. In delegated reviews, reconcile shared assumptions, boundary
gaps and conflicting recommendations before stating the combined judgment; preserve material
disagreement and its effect on the decision when the evidence does not settle it.

The conclusion names the next architectural decision or implementation step and its owner.
