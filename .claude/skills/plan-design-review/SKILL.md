---
name: plan-design-review
description: Prepare a solution-oriented design review from the refreshed audit and the operator's chosen library and principle focus, leaving investigation and conclusions to reviewer judgment.
---

# Plan a design review

Prepare an inquiry that can produce a well-supported correction, a useful comparison of
alternatives or a reason to retain the existing design. Build on the operator's intended outcome
and incoming hypotheses. The refreshed audit supplies the starting diagnostic account; preparation
concentrates on questions that could change the preferred solution.

Load [design-review](../design-review/SKILL.md) for the assessment contract and its routes to the
standard, profiles and binding. Before a new engagement, use
[design-principle-audit](../design-principle-audit/SKILL.md) to refresh the broad account. An audit
already refreshed for this engagement can be consumed through preparation and review while its
baseline remains applicable; reconcile intervening source or standard changes before relying on
it. Record which audit revision informs the review. Loading these criteria for plan authoring or
discussing this process does not itself start a review or audit.

## Preserve the operator's focus

The operator selects the review type and its incoming focus:

| Type | Library scope | Principle scope |
|---|---|---|
| Holistic, unconstrained | Open | Broad |
| Library targeted | Named libraries or capabilities | Broad |
| Principle focused | Open | Named principles or concerns |
| Combined | Named libraries or capabilities | Named principles or concerns |

Resolve the selection from the request and conversation; clarify it when unspecified rather than
choosing a type for the operator. The reviewer chooses depth, methods and decomposition within
that focus. Tier and effort describe the review's breadth and uncertainty separately from this
choice. Library focus is a strong investigation hypothesis; adoption remains a judgment.
Principle focus directs solution depth while relevant semantic and correctness guarantees still
constrain remedies. Encountered out-of-focus problems may inform the audit without automatically
expanding the solution review.

## Frame questions that could change the design

Consider which audit findings might share a cause or admit a composed correction, and which have
separate obligations. Prior diagnoses remain open to challenge; an adequate existing boundary,
a different explanation or an unsuitable suggested remedy are all possible conclusions.

Explore the intended capabilities and credible changes that make alternatives revealing. Which
responsibility or representation deserves reconsideration? What can a consumer stop interpreting?
Which existing strengths constrain a correction? Where could a library own a complete capability
rather than replace isolated functions while leaving duplicated machinery around it?

Use available [library overviews](../../../docs/libraries/README.md) for orientation, then Context7
and primary sources for consequential details. Compare composed semantics, integration burden and
execution fit. An overview is a source of ideas, not proof of fit.
The selected standard and functional target govern judgment; current ADRs and rules describe
design the review may propose changing.

Where execution fit matters, select revealing operations and growth, skew, concurrency or failure
premises. Consider necessary work, access paths, crossings, working set, reuse and recovery
qualitatively. Source reasoning may settle a structural mismatch; quantitative claims need
measurements. No cost model, mandatory probe or benchmark campaign follows.

## Organize inquiry around consequential uncertainty

Establish enough current context to recognize semantic owners, contracts and adjacent consumers.
Direct deeper investigation toward assumptions whose resolution changes dependent reasoning or
the comparison of remedies. Follow responsibilities across components when a local assessment
would miss their interaction. Make material coverage limits visible within the chosen focus.

Give a reviewer the functional target, selected focus, audit revision, relevant evidence and
uncertainty. Distinguish inspected facts, prior judgments, proposals and accepted current design.
Explain why references matter and include contrary evidence. Source pointers support independent
examination; they are not a search fence or a preferred verdict.

Use the [shared coordination contract](../../../.agents/roles/README.md) when bounded code research,
library investigation or independent assessment improves the inquiry. No roster or fixed sequence
is needed. Keep one principal reviewer able to reconcile shared premises, conflicting remedies
and boundary gaps rather than concatenating local assessments.

## Present an adaptable approach

Explain the consequential solution questions, investigation priorities, assumptions and scope in
the conversation. No separate durable preparation document is required. Investigation assignments
need not become the published structure; design-review owns the resulting argument and report.

Preparation is sufficient when useful solution investigation can begin and consequential
uncertainty has a route. Leave conclusions responsive to evidence. Follow the operator's boundary
between preparation and conducting the review; remediation remains separately authorized work.

A requested [recommendation audit](../design-review-audit/SKILL.md) can reassess the completed
review before planning. Carry the selected focus and input baselines into that handoff; it
produces net changes and an integrated review, without a second preparation artifact or automatic
new broad audit. Do not insert this as an unrequested self-review stage.
