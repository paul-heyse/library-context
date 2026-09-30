---
name: plan-design-review
description: Plan a design review around the user's goal, grounded in the repository's design-review skills and principles.
---

# Plan a design review

Develop an approach to the requested review. Treat the user's description as the goal;
choose the scope, investigation strategy and depth accordingly.

Load the `design-review` skill and discover the applicable principles, profiles and repository
binding through the repository's instructions and `standard.toml`. Those sources govern the
eventual assessment.

Explore enough of the current repository to make the plan specific and useful. Distinguish
current implementation from accepted or proposed design.

Depending on the goal, useful considerations may include:

- The questions or decisions the review should help resolve.
- Important responsibilities, semantic owners, contracts and consumers.
- Realistic changes that could expose strengths or weaknesses.
- Where broad coverage matters and where deeper investigation is useful.
- Relevant library capabilities, alternatives and total integration cost.
- Uncertainties that source inspection can settle, and those that may warrant probes or
  independent review.

These are suggestions, not required sections or an exhaustive checklist. Choose other
approaches when they better serve the request.

Use the [shared agent roles](../../../.agents/roles/README.md) to identify useful parallel evidence
gathering and independent review. Delegate bounded preparation where it helps make the approach
concrete, while keeping the eventual reviewer independent. Choose roles to fit the questions.

Produce a concrete review plan in the conversation, with enough rationale to explain the
chosen scope and approach. Make material assumptions and coverage limits visible. Ask
questions only when their answers would meaningfully change the plan.

Use the available planning interface when appropriate; otherwise present the plan directly
in the conversation. Keep the plan adaptable as evidence changes. Follow the user's requested
boundary between planning and conducting the review; remediation is separate work.
