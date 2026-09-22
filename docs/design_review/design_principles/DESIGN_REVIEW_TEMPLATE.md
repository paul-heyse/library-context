# Design review template

Reference: `DATA_MODEL_DESIGN_CHARTER.md`. Replace bracketed prompts with evidence. Remove irrelevant sections with a brief scope note rather than inventing requirements. For a small change, use a compact version of this template.

## 1. Decision and scope

**Proposal:** [Name and short description.]  
**Status:** [Proposed / interface-checked / implemented / tested; use claim-specific labels.]  
**Reviewer / author:** [Person, agent, or accountable role.]  
**Affected revisions:** [Model/schema/implementation identifiers as applicable.]

**Observable outcome:** [What becomes possible, safer, simpler, or measurably better?]

**Baseline:** [Existing design and relevant limitations.]

**Supported scope and non-goals:** [State which capabilities and guarantees are actually in scope.]

**Constraints and uncertainty:** [Scale, deployment, compatibility, correctness, external interfaces, unknowns.]

### Method and coverage

[What you looked at, what you did not, what you could not verify and why. A reader has to be able to tell "no defect here" from "not inspected" — otherwise the review's silences read as assurance they haven't earned. Name the guarantees you did not attack.]

## 2. Authority and lifecycle map

| Concept or fact | Semantic type and identity | Authority / owner | Revision or snapshot boundary | Permitted update path | Derived representations |
|---|---|---|---|---|---|
| [Definition] | | | | | |
| [Binding/specification] | | | | | |
| [Policy] | | | | | |
| [Observation/input] | | | | | |
| [Execution state] | | | | | |
| [Result] | | | | | |

**Deliberately opaque behavior:** [What is outside the declarative model, why, and which contract governs it?]

**Identity behavior:** [Rename, reorder, reserialize, regenerate, duplicate, split/merge if applicable.]

## 3. Semantic contracts and invariants

| Contract or invariant | Representation | Enforcement boundary | Failure behavior | Verification evidence |
|---|---|---|---|---|
| [Semantic compatibility] | | | | |
| [Relationship/cardinality] | | | | |
| [Lifecycle consistency] | | | | |
| [Capability requirement] | | | | |

**Absence and uncertainty:** [Distinguish unspecified, unknown, not applicable, uncertain, invalid, partial, failed, or other relevant states.]

**Equivalence requirements:** [Byte / structural / semantic / approximate; tolerances and assumptions.]

## 4. Derivation and execution design

| Stage or operation | Input revisions and dependencies | Output contract | Preconditions / assumptions | Effects and mutable ownership | Provenance / invalidation |
|---|---|---|---|---|---|
| [Declaration/binding] | | | | | |
| [Resolution/validation] | | | | | |
| [Transformation/preparation] | | | | | |
| [Execution] | | | | | |
| [Publication] | | | | | |

**Relationship structures:** [Distinguish ownership, connectivity, dependency, scheduling, provenance, and other graph meanings.]

**Provider selection and limitations:** [Selected version, required capabilities, supported subsets, unsupported behavior.]

**Boundary contracts:** [Schema, identity, metadata, encoding, ownership/lifetime, batching, conversions, and possible loss.]

**Coherent publication:** [What becomes visible, when, and under which consistency protocol?]

## 5. Representative journeys

### Ordinary extension

[Trace one realistic new rule, entity, provider, or policy. Identify new semantic decisions, specialized implementation, generated artifacts, and tests. Distinguish multiple files from duplicated meaning.]

### Meaningful change

[Trace a changed specification, policy, structural definition, or assumption. Explain identity, invalidation, retained artifacts, and semantic diff.]

### Boundary or alternate representation

[Trace meaning through serialization, a language bridge, a backend, or another physical layout. State what is preserved or deliberately omitted.]

### Interruption or failure

[Trace cancellation, crash, retry, unsupported capability, or invalid input. Explain observable state, valid partial outputs, effects already performed, and recovery.]

## 6. Acceptance gates

| Gate | Pass / fail / unresolved / not applicable | Evidence or scope rationale | Required action |
|---|---|---|---|
| G1 — Authority | | | |
| G2 — Semantic fidelity | | | |
| G3 — Validity | | | |
| G4 — Hidden behavior | | | |
| G5 — Consistency and recovery | | | |
| G6 — Transformation and reuse | | | |
| G7 — Truthful capability claims | | | |

An unresolved gate is not a pass. A high score elsewhere cannot offset a failed required gate.

## 7. Principle findings

| Finding | Principle IDs | Concrete evidence or gap | Consequence | Proposed correction | Verification |
|---|---|---|---|---|---|
| [One substantive finding per row] | | | | | |

The **Verification** column must name the executable oracle that would catch a regression — an
`ast-grep` rule in `rules/`, a `just` gate, a hook, or a test. When none exists, say so
explicitly: that is the most valuable output a review produces here, because it becomes a new
entry in `rules/`.

**Applicability:** [Which groups are relevant and why? Do not restate all 60 principles for every change. Say which groups did *not* bear on this scope and why, tied to scope — unapplied is not the same as irrelevant.]

**Optional maturity assessment:** [Use charter dimensions only when helpful. Separate current evidence-supported scores from proposed targets and disclose reweighting.]

## 8. Alternatives and architectural leverage

| Alternative | Semantic duplication and extension locality | Correctness and operational risks | Implementation / maintenance cost | Performance evidence | Why selected or rejected |
|---|---|---|---|---|---|
| Current baseline | | | | | |
| Proposed design | | | | | |
| Simpler viable alternative | | | | | |

**Abstractions justified by current needs:** [Explain registries, compilers, generators, services, stores, and engines that add material complexity.]

**What remains ordinary code:** [Specialized algorithms and mechanisms that should not be turned into an unnecessary DSL.]

## 9. Verification and measurement plan

| Claim or risk | Evidence label | Test / analysis / benchmark | Conditions and expected result | Current result or remaining gap |
|---|---|---|---|---|
| [Invariant enforcement] | | | | |
| [Representation preservation] | | | | |
| [Reuse/invalidation correctness] | | | | |
| [Failure/recovery] | | | | |
| [End-to-end performance] | | | | |

Include relevant round-trip, differential, metamorphic, negative, migration, concurrency, and interrupted-publication checks. Do not assume independent-looking backends constitute independent correctness oracles.

**Cost accounting:** [Construction, validation, preparation, transfers, execution, storage, recovery, inspection, memory; only the material categories.]

## 10. Exceptions and unresolved decisions

**Principle IDs:** [IDs.]  
**Scope:** [Where the deviation applies.]  
**Reason and alternatives:** [Why the preferred default is not suitable.]  
**Consequence and compensating controls:** [What is weakened and how risk is managed.]  
**Evidence:** [Support for this choice.]  
**Owner / accountable role:** [Who maintains the exception.]  
**Revisit trigger:** [A concrete change, scale, capability, or other condition.]

A SHOULD-level exception can be accepted with justification. A MUST-level gap requires narrowing the supported claim or recording the design as unresolved for the affected requirement.

## 11. Decision and implementation changes

**Decision:** [Accept / accept scoped design with documented SHOULD deviations / revise / reject.]  
**Reason:** [The strongest evidence and remaining limitations.]

| Priority | Change | Principle IDs | Acceptance evidence | Regression protection |
|---|---|---|---|---|
| [Correctness first] | | | | |
| [Semantic leverage next] | | | | |
| [Measured performance where justified] | | | | |

**Final check:** The design’s claims match the evidence; the scope matches the implemented guarantees; and later extensions have a clear, validated path.
