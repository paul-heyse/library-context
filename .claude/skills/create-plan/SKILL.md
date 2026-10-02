---
name: create-plan
description: Create or revise an implementation plan from a design review or existing plan, grounded in current code and the applicable design standard.
---

# Create a plan

Develop a plan that gives an intelligent implementer a coherent understanding of the intended
system, the reasoning behind its boundaries, and a credible path from the current state to
completion. Explain consequential choices and leave room for implementation judgment.
Use the supplied design review or existing plan as the starting reference. When the reference
is implicit, resolve it from the conversation and current repository context; ask only when
the ambiguity would materially change the work.

Read the relevant architectural owners and current code. Load the [design-review skill](../design-review/SKILL.md)
and discover the applicable principles, profiles and repository binding through the repository's
instructions and `standard.toml`. Preserve the reference material's relevant obligations while
reconsidering assumptions when current evidence or the intended capabilities warrant a different
approach. Distinguish the implemented baseline, accepted decisions and proposed changes.

## Assess the foundations the plan will use

As part of developing the plan, assess the existing components and contracts on which the
proposed work will depend. Consider whether the intended capabilities reveal opportunities to
improve those foundations, clarify responsibilities or simplify the combined design. Use the
applicable design-review criteria, with depth proportional to the consequential questions.
Carry warranted changes into the target design and execution sequence, including preparatory
work where later packages depend on it.

This focused assessment is integral to authoring the plan. Choose the investigation that settles
the material questions: source inspection may suffice; scenarios, probes or wider review can
help where uncertainty warrants them. Capture suitable integration boundaries as well as needed
changes. Keep unresolved choices and their consequences for dependent work visible.

Usually the conclusions belong in the plan's baseline and design discussion. Follow the
repository's cadence for a separate formal review when applicable; this focused assessment
does not certify an enclosing subsystem. Use existing review conclusions where they settle
the relevant questions. Plan authoring does not require a principle-by-principle reassessment,
compliance matrix or additional review ceremony.

Use the [shared agent roles](../../../.agents/roles/README.md) to delegate bounded code mapping,
library research or focused design advice where useful. Give consequential alternatives independent
consideration and inspect decisive evidence. Keep one author responsible for assembling the plan;
the coordinator retains design decisions. Delegation does not make the foundation assessment optional.

## Explain the target and its reasoning

Describe the resulting responsibilities, contracts and interactions, including how the target
differs from the current system. Make preserved guarantees, intentional behavior changes and
support limits clear where they affect implementation or acceptance. Relevant distinctions may
include data interpretation, failure behavior, ordering, precision or information loss.

Use a few credible future changes to explain what the target is meant to accommodate. Show
where an extension would be expressed, what it would reuse and what should remain unaffected.
These scenarios clarify the reason for a boundary; they need not expand implementation scope.

Explain consequential decisions through their benefits, costs and assumptions. Consider library
capabilities and alternatives where they affect the design, using the repository's capability
skills and documentation routes. Preserve enough rationale for an implementer to recognize
evidence that would reopen a decision. Relate an expected benefit to its mechanism and workload
or conditions, and account for material integration, lifecycle and maintenance obligations.
Quantitative benefits need a baseline and measurement route; unmeasured benefits remain hypotheses.

Resolve choices that materially shape responsibilities, semantics, sequencing or acceptance.
If uncertainty remains, state the question, the evidence needed and the dependent work it
constrains. A bounded investigation can be a work package when its decision outcome changes
the execution path. Local organization, names and routine implementation choices can remain open.

## Organize documents for understanding

Document boundaries group reasoning that belongs together; execution packages group changes
that can be undertaken and verified together. They need not coincide. Split a large plan when
an area has a coherent target, substantial internal reasoning and identifiable contracts with
other areas. Keep tightly coupled explanations together when splitting would force readers to
reconstruct them across documents. Let responsibility, context burden and boundary stability
guide decomposition; document length or a fixed number of phases is insufficient by itself.

For a series, give the coordinator the combined target, scope, shared decisions, cross-plan
dependencies and overall completion conditions. Supporting documents develop their respective
designs and work packages. Each should explain why its work exists, what it consumes and
produces, and where its responsibility ends. A single document can provide these same levels
of explanation for smaller work.

Give shared contracts and current state identifiable owners and link to them. Brief contextual
summaries make a document readable without creating competing definitions or status records.
The coordinator should make the whole undertaking understandable; a supporting document should
make its work actionable with a bounded set of references. Follow local conventions for naming,
location and durable architectural authority.

## Sequence coherent changes through explicit dependencies

Describe prerequisites through the contract, capability or evidence they supply, why it is
needed and what makes it available. Distinguish a settled design from an implemented contract,
migrated consumer or verified behavior: design work and production integration may need different
levels of readiness. An agreed interface alone does not establish a working prerequisite.

Look below whole-document dependencies. An apparent cycle may separate into an early shared
contract and later consumers. An ordered table can provide a convenient route without making
every row a synchronization barrier. Among ready packages, consider establishing widely consumed
contracts or resolving consequential uncertainty early; explain such ordering when it matters.
Carry warranted foundation improvements ahead of the work that relies on them.

Keep prerequisites, runtime interactions, shared editing surfaces and common acceptance journeys
distinct. Independent capabilities may still contend for the same files. Semantic ownership and
execution responsibility also differ: a package may migrate several consumers without taking
authority over their contracts. When delegation helps, coordinate shared edit scope and integration
responsibility as well as logical dependencies.

Choose packages that deliver coherent changes in behavior. Consider producers, affected consumers,
interfaces, stored representations and verification together. Cross-cutting work may have a
continuing owner while participating incrementally in other packages. Schedule needed consumer
migrations alongside the change they support; a later inventory-completion phase should not
silently defer necessary integration.

When an early package needs only part of a larger one, identify that prerequisite slice and the
actual target contract it delivers, including the consumers needed for its scope. Keep the
enclosing package's remaining obligations visible. Partial delivery neither requires a temporary
substitute for the target nor establishes completion of the larger package.

## Make work packages concrete

Give the implementer a picture of the resulting behavior: the inputs and context an operation
receives, the decisions it owns, the product or state change it produces, and how consumers use
it. Explain preserved meaning, failure cases, effects, resource lifetime or state transitions
where they resolve consequential uncertainty. Connect these in prose; use tables, diagrams,
interface sketches, file pointers or algorithms when they clarify the work.

Conceptual product names express required meaning, not a demand for a new type, service or file
for every named concept. Choose detail for its decision value. A successful example paired with
a revealing failure case often settles ambiguity better than an exhaustive implementation script.
Distinguish illustrative consumers from the actual migration scope so an example does not become
an accidental completion boundary.

Describe preservation and retirement with replacement, following the project's migration policy.
Explain meaningful intermediate states: which capabilities become usable, which consumers have
moved and which constraints remain. Where adoption requires data migration, activation, rebuilding
derived artifacts or operational coordination, schedule that work explicitly. Address interruption,
recovery and coordinated cutover where consequential; do not introduce rollout or compatibility
machinery without a relevant need and authorization.

## Establish completion and keep the plan usable

Derive acceptance from the behavior and distinctions the design must establish. Choose evidence
capable of exposing a wrong implementation or premise; expected results produced by the same
questionable transformation may only reproduce its defect. Explain suitable independent evidence
where that risk matters, without requiring a new oracle for every change.

Local verification, assembled-system qualification and benefit measurement establish different
claims. Give each relevant scope an owner and deliberate timing under the repository's execution
policy. Package and document boundaries do not automatically require full campaigns. Make clear
which evidence is necessary for completion and which measurements are deferred, with their purpose
and trigger. Retain source-review finding identifiers and link to the single disposition owner;
a package can finish while a broader finding still has other contributors or evidence obligations.

Keep design intent, current progress and completed evidence distinguishable. A checkpoint explains
what exists, which prerequisites are available, what remains uncertain and the next executable
work. Record current state at its owner rather than accumulating duplicate logs. When evidence
changes a shared decision, update its owning explanation and affected dependencies and consumer
expectations together. Move enduring knowledge to the repository's established owners as work
closes, following its retention policy.

## Write the plan

Use [the suggested structure and examples](references/plan-structure.md) when choosing the
document layout or developing package descriptions. Scale the plan to the work; combine, expand,
rename or omit sections according to the subject. These principles describe useful content and
reasoning, not mandatory headings or a checklist. Include detail when it changes an implementer's
understanding or decisions. Repository-specific commands, review cadence and migration policies
come from local instructions, not from this general authoring guidance.

Check that the proposed contracts, work dependencies and completion evidence agree, and that
material source obligations have an explicit route. Distinguish planned verification from
commands actually run, with the repository's evidence labels and qualification boundaries.
Route changes to accepted architecture through its existing decision process.

For a plan-writing request, improvements are designed and scheduled in the document. Follow any
separate authorization to implement them; creating a plan does not itself authorize production
changes. Return the document path, its principal design choices and any unresolved decisions
that constrain execution.
