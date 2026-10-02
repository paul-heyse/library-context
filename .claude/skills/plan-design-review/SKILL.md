---
name: plan-design-review
description: Plan a design review around the user's goal, grounded in the repository's design-review skills and principles.
---

# Plan a design review

Prepare an inquiry capable of producing an independent, well-supported judgment. Treat the
user's description as the goal and build on any approach already supplied. Choose the
questions, context and division of reasoning that will make the review productive; leave its
conclusions and document structure responsive to what is discovered.

Load [design-review](../design-review/SKILL.md) and discover the applicable principles,
profiles and binding through repository instructions and `standard.toml`. Those sources own
the assessment criteria and output contract. Explore enough current context to make the
approach specific, distinguishing implementation, accepted constraints and proposed design.
The considerations below guide attention, not required stages or an exhaustive checklist.

## Frame questions that could change the judgment

Identify what the review should help someone decide and which uncertainties could materially
affect that decision. Consider plausible explanations and what evidence would support or
undermine them. Prior findings are useful leads; keep the inquiry open to an adequate existing
design, a different underlying cause, or a sound diagnosis with an unsuitable proposed remedy.

Prepare revealing questions about the intended capabilities and credible changes, rather than
settling the verdict during preparation. Where alternatives or library capabilities matter,
identify the semantic or integration question a comparison would resolve. The selected design
standard remains the basis for judgment.

## Choose breadth and depth deliberately

Establish enough context to recognize relevant responsibilities, semantic owners, suppliers
and consumers so an isolated component does not stand in for its interactions. Concentrate
deeper investigation where a disputed assumption, consequential boundary or revealing scenario
could change the assessment. Choose early investigation for its ability to resolve uncertainty
that other inquiries depend on.

Let the question shape an assignment. A component boundary may be useful; another inquiry may
need to follow a responsibility across several components. Consider what separate local reviews
could leave unexamined between them. Adapt the investigation as evidence changes, making
material coverage limits visible without silently narrowing the user's requested scope.

Further investigation should answer a material question. Existing evidence or source inspection
may suffice; scenarios, probes or specialist advice help when uncertainty warrants them.
Preparing the review requires neither exhaustive mapping nor a new probe or second assessment.

## Provide context without prescribing the verdict

Give a reviewer the intended capability, relevant requirements, baseline, applicable authorities
and known uncertainty. Explain why references matter and distinguish facts, accepted constraints,
proposals and previous judgments. Include relevant contrary evidence. Supply source pointers
that support independent examination; workers may follow discovered dependencies beyond them.

For example, the reasoning portion of a brief might say:

> Assess whether request interpretation remains consistent across interactive and batch entry
> points. Section X owns the intended semantics; these modules implement the two paths. An
> earlier review suspected duplicated policy, but that diagnosis remains to be assessed.
> Consider legitimate differences as well as inconsistencies, including shared failure behavior.

Use the [shared agent roles and coordination contract](../../../.agents/roles/README.md) for
assignment mechanics, permissions and evidence handling. Delegate bounded preparation or
independent assessment when it improves the inquiry; no roster or fixed sequence is required.
Keep the eventual reviewer's judgment independent.

## Anticipate reconciliation and begin when ready

Consider which inquiries share assumptions and which interactions need attention across
assignments. Leave the principal reviewer room to reconcile evidence and judgments at those
boundaries; separate local assessments may depend on incompatible premises. A disagreement
may call for a focused follow-up inquiry. Preserve independent judgments and their evidence
limits rather than forcing agreement.

Investigation assignments need not dictate the published structure. The design-review skill
owns how the principal review, findings and recommendations are expressed.

Present an adaptable approach in the conversation, explaining consequential scope choices,
investigation priorities, assumptions and coverage limits. Use the available planning interface
when useful; no separate durable preparation document is required by default. Ask only for
information that materially changes the work.

The approach is ready when useful investigation can begin, consequential uncertainties have
a route, and further preparation depends on evidence the review itself will produce. Preparing
the review need not settle its judgment. Follow the user's boundary between planning and
conducting the review; remediation remains separately authorized work.
