---
name: create-plan
description: Create or revise an implementation plan from a design review or existing plan, grounded in current code and the applicable design standard.
---

# Create a plan

Develop a plan that explains how the intended system will fit together and how to implement it.
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
does not certify an enclosing subsystem.

Use the [shared agent roles](../../../.agents/roles/README.md) to delegate bounded code mapping,
library research or focused design advice where useful. Give consequential alternatives independent
consideration and inspect decisive evidence. Keep one author responsible for assembling the plan;
the coordinator retains design decisions. Delegation does not make the foundation assessment optional.

## Develop the design and execution sequence

Establish meaningful ownership, contracts, dependencies and acceptance expectations. Consider
library capabilities and alternatives where they affect the design, using the repository's
capability skills and documentation routes. Choose the level of detail that resolves important
implementation uncertainty; file lists, API sketches and algorithms are useful where they do so.

Turn the design into work packages with clear prerequisites and completion evidence. Carry
foundation improvements into that sequence before the work that relies on them. Include
migration, preservation and retirement obligations where relevant. Retain source-review finding
identifiers and link to the single owner of current disposition rather than duplicating status.

## Write the plan

Use [the suggested structure](references/plan-structure.md) as an adaptable starting point.
Follow the repository's plan location and naming conventions. Scale the document to the work;
combine, expand, rename or omit sections according to the subject.

Check that the proposed contracts, work dependencies and completion evidence agree, and that
material source obligations have an explicit route. Distinguish planned verification from
commands actually run, with the repository's evidence labels and qualification boundaries.
Route changes to accepted architecture through its existing decision process.

For a plan-writing request, improvements are designed and scheduled in the document. Follow any
separate authorization to implement them; creating a plan does not itself authorize production
changes. Return the document path, its principal design choices and any unresolved decisions
that constrain execution.
