---
name: plan-creation
description: Plan how to create an implementation-plan document from a design review or existing plan, including the investigations and design questions that matter.
---

# Plan the creation of a plan

Prepare the investigation and decision-making needed to produce a coherent, implementable
proposal. Build on the user's intended outcome and any approach already supplied. Choose the
questions, context and division of reasoning that will make authoring productive; leave the
target and document structure responsive to what is learned.

Load [create-plan](../create-plan/SKILL.md) for the authoring contract and follow its routes
to the applicable design-review skills, principles and repository binding. Identify the
reference review or existing plan and explore enough current code and architectural context
to make the approach specific. The considerations below guide attention, not required stages
or an exhaustive investigation sequence.

## Separate obligations from open design choices

Establish what the source material supplies: the problem to solve, guarantees to preserve,
accepted constraints, proposed remedies and unresolved questions. A review's illustrative
remedy is not automatically a requirement. Carry forward settled reasoning whose assumptions
remain applicable, and identify where the intended capability or current evidence requires
fresh design work.

Distinguish accepted decisions and intended behavior from implemented foundations. Identify
the existing components and contracts whose suitability for the proposed work needs focused
assessment. That assessment remains integral to plan creation under create-plan; preparation
chooses how to conduct it without repeating an assessment already supported by relevant evidence.

## Investigate consequential decisions in a useful order

Some questions shape many later choices: who owns a meaning, whether an existing foundation
supports the new consumers, or whether a library capability fits the required semantics.
Investigate such questions early enough to avoid developing substantial detail around an
unstable premise. Explain what a result would enable or change in the proposed design.

Choose source inspection, scenarios, library evidence or focused design advice according to
the question. Probes remain discretionary. Parallelize inquiries whose inputs are sufficiently
settled; reconsider dependent reasoning when a shared premise changes. Keep routine local
implementation choices available to the eventual implementer.

This is the order of reasoning while creating the plan. The eventual implementation sequence
and its explanation belong to create-plan. Preparation does not need to settle every target
decision before authoring begins.

## Delegate questions that produce usable design inputs

Explain which decision an assignment will inform. Supply relevant source obligations, target
behavior, current contracts and constraints, distinguishing settled decisions from alternatives
still open. Explain why each reference matters and allow workers to follow discovered
dependencies. Ask for supported conclusions and consequential uncertainty that the author can
use, without prescribing the preferred answer.

For example, the reasoning portion of a brief might say:

> Determine whether the existing request-resolution operation can support the proposed
> persistence consumer. The review establishes a loss-of-meaning problem; it does not establish
> that a new service is necessary. Inspect the current contract and affected consumers.
> Explain what can be reused, what must change, and which assumptions constrain the choice.

Use the [shared agent roles and coordination contract](../../../.agents/roles/README.md) for
assignment mechanics, permissions and evidence handling. Bounded code or library investigation,
independent consideration of alternatives, and focused design advice can help where they resolve
material questions. Delegation is discretionary; the coordinator owns the approach and design
choices. Focused advice does not replace a formal review due under the repository binding.

Bounded drafting can help once its consumed decisions are sufficiently settled. A chapter that
looks independently assignable may still depend on unresolved shared semantics. Choose the
assignment for the design input it can produce, not merely an available document section.

## Reserve attention for assembling the proposal

Consider where investigations will inform a common decision. Individually reasonable proposals
can disagree about ownership, meaning, preservation or dependencies. Keep one author responsible
for assembling the plan, with enough visibility to reconcile those tensions before elaborating
dependent sections. Identify useful follow-up questions when findings conflict; do not resolve
them by simply concatenating contributions.

Keep segmentation provisional until the reasoning reveals useful boundaries. Create-plan owns
document organization, work-package descriptions and acceptance content; this preparation
should enable those decisions without reproducing their authoring guidance.

Present an adaptable approach in the conversation, explaining consequential scope choices,
investigation priorities, assumptions and uncertainties. Use the available planning interface
when useful; ask only for information that materially changes the work. The approach is ready
when useful investigation or authoring can begin, consequential uncertainties have a route,
and further preparation depends on evidence that work will produce.

Follow the user's boundary between preparing the approach and writing the plan with create-plan.
No separate durable preparation document is required by default. Producing the plan does not
itself authorize implementation.
