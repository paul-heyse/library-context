# Design review template

**Version 3.0 · 2026-09-25** · Core layer: repository- and domain-agnostic.
Standard: [design principles](design-principles.md): FP-01–FP-06 organize architectural
assessment; DP-01–DP-24 support it; A1–A3 and G1–G8 remain separate judgments.
Profiles add domain constraints within the slots. The binding supplies local owners and cadence.

## Part 1 — The review contract

### Tier and purpose

| Tier | Use for | Required content |
|---|---|---|
| Change | A bounded implementation within an accepted architecture | Scope, affected owners/contracts, one relevant change scenario or a reason it does not apply, applicable gates, findings, library implications, A1–A3 and decision. Compress slots into 1, 6, 7, 8, 12. |
| Design | A substantial stage, architectural decision, new mechanism or boundary; assembled architecture | Slots 1–12, scoped to the subject and adjacent consumers. Reconstruct responsibilities and dependencies before investigating mechanisms. |

| Purpose | Judged against | Handling a blocking repository authority |
|---|---|---|
| Conformance | Accepted architecture and applicable standard | Follow the authority; record the conflict and route to change it. |
| Target | Best available architecture for the functional target and its expected changes | Evaluate the blocking text as a candidate for revision; identify its replacement and decision route in slot 11. |

Depth follows impact and uncertainty. Legacy `compact`, `standard` and `deep` describe effort;
they are not additional tiers. Drop an irrelevant slot with a scope reason. Mechanical changes
can state that ownership, contracts and extension behavior are unchanged and cite the inspected
boundary. Do not narrow away an affected consumer to obtain acceptance.

### What a claim can rest on

- **Document subject:** judge whether the architecture and obligations are specified and whether
  change scenarios have credible routes. Claims remain *Proposed*, or *Interface-checked* for
  inspected interfaces. A scenario walkthrough is not executed behavior or measured change cost.
- **Code subject:** cite the actual ownership, dependencies, definitions and executing paths.
  *Implemented* establishes existence; *Tested* and *Measured* name the commands, cases and
  conditions. Historical receipts are explicitly dated and attributed, never reported as fresh runs.
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
| Consequence | A supported input causes semantic failure, **or a realistic change requires duplicated decisions, unrelated internal edits, hidden knowledge, inseparable testing or unjustified machinery**. Name the scenario and propagation; file counts alone do not establish it. |
| Correction | Owning boundary and direction, rough affected surface, alternatives and deletion obligations where relevant. |
| Verification | Evidence that would close the finding; a traced change or inspection can suffice. Add a test/probe only when it resolves uncertainty or protects a meaningful regression. |
| Disposition link | Where current execution status is owned, if assigned; otherwise explicit deferred trigger or required decision. A review remains a dated assessment. |

### Priority and decisions

Prioritize correctness and fidelity breaches, then architectural barriers to planned work and
repeated semantic ownership, then other justified improvements and measured costs. Discuss
interactions: resolving a correctness defect may require changing an architectural boundary.
Library adoption and bespoke code both carry complexity. A concrete architectural violation is
eligible for revision even when its supporting DP rule is a SHOULD.

| Situation | Decision |
|---|---|
| Applicable A1–A3 satisfied; no MUST gap or failed/unresolved gate in supported scope | Accept, at the stated evidence strength and scope. |
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

## Part 2 — The template

### 1. Scope, outcome and coverage

| Field | Value |
|---|---|
| Subject | Document/code paths and revision; dirty-tree limitations when relevant |
| Standard | Core version, profiles and binding |
| Tier · purpose | Change/design · conformance/target |
| Reviewer · date | Accountable reviewer and date |
| Maturity and outcome | Current design phase; what this decision should enable |
| Supported scope | Capabilities, adjacent consumers and guarantees; exclusions |
| Expected changes | Selected realistic scenarios and why they matter now |
| Baseline | Existing architecture and material limits |
| Method and coverage | Examined/clean, unresolved, not examined; tests and assumptions |

### 2. Responsibilities, dependencies and semantic ownership

| Component | Coherent responsibility and hidden decisions | Consumer contract | Allowed dependencies/direction | Expected reason for change |
|---|---|---|---|---|

Use a small dependency diagram where useful. Distinguish compile-time dependencies, runtime
coordination and representation flow where they differ. A module is a sufficient owner when its
boundary holds; a crate split is not the objective.

| Concept | Semantic authority and identity | Update/revision boundary | Derived forms and consumers |
|---|---|---|---|

Identify duplicated decisions, private mechanisms exposed to consumers, and opaque behavior.

### 3. Contracts, constraints and testing boundaries

| Contract | Inputs/outputs and consumer expectations | Preconditions/invariants and enforcement | Effects/lifecycle/failure | Substitution/compatibility | Isolated verification |
|---|---|---|---|---|---|

State meaningful absence states, equality/approximation promises and where invalid construction
is prevented. Explain necessary runtime enforcement and the dependencies needed to test it.

### 4. Composition and execution

| Capability or stage | Semantic inputs/outputs | Owning mechanism | Dependencies and reuse boundary | Policy vs orchestration | Effects, ownership and publication | Limits/determinism |
|---|---|---|---|---|---|---|

Trace how primitives form a workflow. Identify rules that orchestration reimplements, implicit
ordering and special cases. Include cardinality/cost, identity mappings, representation loss and
resource budgets where material. A graph projection states its universe and relationship semantics.

### 5. Change and failure scenarios

| Scenario and trigger | Owning component | Contract change | Expected vs observed affected consumers | Independent semantic edits/hidden knowledge/test setup | Evidence or settling check |
|---|---|---|---|---|---|

Choose relevant additions, analyzer/provider upgrades, new compositions/renderings, invariant
changes, implementation replacement and isolated tests. Use declared variation axes. New domain
concepts can legitimately change several contracts. Examine boundary round trips and
interruption/failure where claimed behavior requires them. Observed implementation traces and
proposed routes remain distinct.

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

### 8. Library fit and total complexity

| Capability and owner | Consumer or planned scenario | Built-in/library/bespoke candidates | Pinned semantic fit and gaps | Coupling/lifecycle/test/replacement burden | Choice and reason |
|---|---|---|---|---|---|

Inspect pinned interfaces when making an API claim. A library's full surface is eligible for the
agreed capability; availability alone is not a requirement. An intentional shared library data
contract can be preferable to a forwarding abstraction. This is a focused comparison, not a catalog.

### 9. Alternatives and tradeoffs

| Alternative | Change propagation and local reasoning | Semantic authority and composition | Test/substitution boundary | Total machinery and operational risk | Evidence, decision and revisit condition |
|---|---|---|---|---|---|
| Current baseline | | | | | |
| Proposed design | | | | | |
| Suitable library-owned alternative | | | | | |
| Simplest viable alternative | | | | | |

Rows can coincide or be out of scope with a reason. Explain what becomes easier and harder.
Preserve independent semantic controls when removing obsolete implementation-specific tests.

### 10. Verification and uncertainty

| Claim or scenario | Evidence label/date | Inspection/test/analysis/benchmark | Conditions and expected result | Current outcome or gap |
|---|---|---|---|---|

A traced extension, removed competing authority or bounded test setup can close an architectural
finding. Independent oracles must challenge production contracts. Execute only checks that resolve
material uncertainty; follow the binding's integrated acceptance timing.

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
| A2 Encode meaning structurally | | | |
| A3 Extend through composition | | | |

**Bounded change decision:** Accept / Accept scoped / Revise / Reject, with reason.
**Enclosing architecture:** accepted for named scenarios / needs revision / unresolved / not
assessed (scope reason). State the evidence strength; a passing local slice does not certify it.

| Priority | Change and responsible component | Source findings | Closure evidence or revisit trigger |
|---|---|---|---|

The conclusion names the next architectural decision or implementation step and its owner.
