---
name: functional-extension-review
description: Discover and assess valuable additions or improvements to product, developer, operator or research capabilities within the operator-selected scope. Develop grounded opportunities and compare enabling mechanisms, using design principles to shape their realization. Use for requested functional reviews or capability exploration, including ecosystem-led inquiries, without requiring prior diagnostic findings. Ordinary feature implementation does not initiate this review.
---

# Review functional extensions

Explore what valuable capabilities the system could gain and which additions or improvements
are worth pursuing. Functional value includes product behavior, developer workflows, operational
control and research capabilities. Useful opportunities can address design gaps, enable new
activities, improve existing workflows or combine these benefits.

Use this at a requested or explicitly planned exploration boundary. Ordinary feature implementation
follows its accepted scope and does not initiate a functional review.

## Establish the exploration scope

Resolve the operator's chosen outcome, workflow, subsystem or library/tooling ecosystem from the
request and conversation. An exploratory ecosystem request is sufficient; the operator need not
already know which capabilities to request. Preserve supplied priorities and hypotheses while
leaving conclusions responsive to investigation. Clarify only ambiguity that materially changes
the inquiry; do not choose a different exploration scope for the operator.

Read relevant architectural owners, code, interfaces and operational routes to understand current
capabilities and where an addition could connect. Distinguish missing functionality from existing
functionality that is difficult to discover, compose or operate. Relevant plans establish current
work and commitments when needed; they need not determine the opportunity space.

Load the standard through the repository's `standard.toml`: applicable core principles, efficient
architecture heuristics, profiles and their assessment guidance, and the binding. Use these to
shape and assess proposed realizations. This review has no broad design-principle audit
prerequisite. Existing audit findings are optional context when they illuminate an opportunity;
neither an audit finding nor a principle violation is needed to establish functional value.

## Discover useful possibilities

Explore useful activities the system could support, richer forms of existing capabilities and
combinations that available mechanisms make possible. Existing pain, the operator's intended
direction and credible future workflows can all support a proposal. A capability need not have
an existing caller or failure report to merit consideration; explain its plausible consumer and
value rather than inventing demand. Distinguish demonstrated needs from hypotheses.

Use available [capability overviews](../../../docs/libraries/README.md) to broaden awareness,
then Context7 and primary sources when consequential details need resolution. Consider native
functionality, system tools, services, libraries and existing repository capabilities. Follow
promising mechanisms beyond familiar basic usage. Organize the inquiry around what they enable;
catalog presence alone does not justify adoption. Several tools may serve one opportunity, and
one composed mechanism may enable several workflows.

## Develop the functional proposition and its realization

For a consequential opportunity, explain who would use it, what they could accomplish, how the
workflow or behavior would change, and why that matters here. Show enough of the resulting
interaction to make the benefit assessable. Current accepted scope and proposed extensions
remain distinguishable: the present functional target is context, not a ceiling on proposals
within the operator's exploration scope.

Compare suitable realizations through semantic fit, responsibilities, composition, execution
fit and total developer/operator machinery. Consider material prerequisites, consumer integration,
activation, failure behavior, resource lifetimes and ongoing operation where they affect value
or feasibility. Existing strengths constrain a sound proposal. Introducing a tool does not by
itself require a new wrapper, service or product abstraction.

When library or tooling functionality materially enables a recommendation, preserve concrete
capabilities and recognizable API, command or feature names, how they compose, their design role
and the reason for the choice. Include precise overview-section or primary-source references,
material conditions and limits, and evidence strength. Distinguish selected directions from
candidates, illustrative examples and unresolved feasibility. Preserve these leads for planning.

Use qualitative judgment about usefulness, likely frequency, breadth of benefit, integration
burden and ongoing operation. Explain material tradeoffs without numerical scoring, invented
demand estimates, cost models or mandatory benchmark campaigns. Quantitative benefit claims need
measurements; useful proposals can remain explicitly reasoned hypotheses. Choose probes or
scenarios when they settle consequential uncertainty, not as proof of diligence.

## Recommend a coherent set of improvements

Assess interactions: opportunities can overlap, conflict, enable one another or share a useful
foundation. Explain which are compelling, which depend on another capability, which should wait
and which lack sufficient value or settled premises. Preserve meaningful alternatives and the
conditions that would change the recommendation. Scope and priority follow the operator's focus
and the argument, without an adoption quota or exhaustive ecosystem checklist.

Describe what would demonstrate a delivered capability, including actual consumer use where
relevant. Representative examples clarify the proposal without fixing the entire acceptance or
migration scope. Detailed implementation packages belong in subsequent planning unless requested.

Current rules and architecture inform integration and preservation; they can also be proposals
for change within the inquiry. Identify consequential rule impacts and the decision they need.
A favorable review does not approve a new functional target, change an accepted plan, authorize
installation or modify production. Use the repository's existing decision routes when selected
scope proceeds to planning and implementation.

## Publish a usable opportunity review

Use the [output conventions](../../../docs/functional_reviews/README.md). Explain the selected
scope, inspected source baseline, current capabilities, opportunities, proposed behavior,
enabling mechanisms, alternatives, interactions, priorities and material uncertainty at the depth
the reasoning needs. Flexible Markdown is sufficient; these are content leads, not mandatory
headings or fields. Preserve stable opportunity references such as `O01` for downstream use;
an opportunity is not automatically a diagnostic finding or an accepted requirement.

Use the [shared roles](../../../.agents/roles/README.md) when independent judgment or bounded
research improves the inquiry. The existing design-reviewer role can assess functional value and
proposed design under this skill; it need not invoke the diagnostic design-review workflow.
The coordinator publishes one coherent account, preserving independent judgments and evidence
limits. Follow the repository's reporting vocabulary and distinguish proposals from implemented,
tested or measured behavior.

A requested [recommendation audit](../design-review-audit/SKILL.md) preserves this functional
lens, publishes net changes and integrates the revised review at its original path. It is not an
automatic extra stage. [Create-plan](../create-plan/SKILL.md) develops selected opportunities into
implementation scope; execution and requested completion audits use the existing plan workflow.
