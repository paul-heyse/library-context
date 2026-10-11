---
name: plan-test-review
description: Plan a causal and structural review prompted by test results, choosing useful code review, probes, research and other investigation. Use when preparing a test review rather than simply running or fixing tests.
---

# Plan a test review

Prepare an inquiry that can explain test observations and identify corrections aligned with
the system's functional target and design principles. Build on the user's goal and any supplied
approach. Test outputs supply starting signals; choose the code review, probes, experiments,
library research and other investigation that could materially improve the judgment.
Leave the conclusions and eventual document structure responsive to what is discovered.

Load [execute-test-review](../execute-test-review/SKILL.md) for the investigation and reporting
contract. Discover the applicable principles, heuristic companion, profiles and binding through
repository instructions and `standard.toml`. Explore enough current context to make the approach
specific, distinguishing intended behavior, implemented contracts, proposed changes and historical
results. The considerations below guide attention, not required stages or an exhaustive checklist.

## Frame questions that could change the explanation

Identify what the review should help someone decide. Consider competing explanations and what
would support or undermine them. A production defect, incorrect assertion, obsolete fixture,
incomplete migration, environment problem or harness failure may produce similar symptoms or
coexist. Earlier diagnoses are useful leads, not conclusions to reproduce.

Assess available observations alongside the further code and contract review worth performing.
Which expectation failed, where does its meaning belong, and what conditions make it legitimate?
Which uncertainty calls for a revealing probe or comparison, and which can source inspection
settle? Similar error text is weaker than an established dependency; a passing comparison may
expose an important difference rather than refute the failure.

Consider whether the recorded invocation actually exercised the intended scope and prerequisites.
Capture gaps, stale executables, source edits, configuration differences, interruptions and
cleanup uncertainty can affect interpretation. Missing information may change the next inquiry
without preventing useful investigation elsewhere.

## Choose breadth and depth deliberately

Start from affected semantic owners and adjacent consumers. Follow consequential questions
backward from symptoms through contracts, state transitions, dependencies and physical execution.
Look for similar vulnerable patterns where the suspected mechanism gives a concrete reason to
expect them. A component-wide survey or full flow trace is useful only when it serves the inquiry.

Include the validity of tests and their execution conditions in the planned reasoning. A legacy
test may encode an obsolete mechanism, or correctly expose an unimplemented guarantee. A setup
failure may be incidental, or reveal recurring weaknesses in readiness, isolation, state ownership
or failure handling. Avoid deciding the correction surface before assessing those possibilities.

Use the design principles and heuristic companion when preparing structural questions. Consider
whether repeated work, duplicated meaning, hidden dependencies, invalid reuse or mismatched
lifetimes explain the observations. Compare composed library capabilities and simpler conforming
alternatives where material. Static reasoning can establish avoidable work or architectural damage;
quantitative benefit claims require measurement. A proposed redesign still needs scrutiny of its
own guarantees, integration burden and legitimate failure cases.

Prioritize investigation by consequence and by the uncertainty it resolves for other questions.
Adapt as discoveries change the premises. Preserve the user's scope while making material
unexamined areas and uncertainties visible; neither exhaustive mapping nor fresh execution is a
prerequisite to beginning the review.

## Give investigators room to discover

Supply the functional target, current owners, relevant run/source context and known uncertainty.
Distinguish observations, hypotheses and prior judgments. Include contrary evidence and explain
why references matter, allowing investigators to follow discovered dependencies. Ask questions
that can support a different diagnosis or an adequate existing design.

Use the [shared roles and coordination contract](../../../.agents/roles/README.md) when delegation
improves independent coverage or judgment. Choose assignments for the questions they resolve,
not a fixed roster or one document per component. Anticipate shared assumptions and interactions
that separate local inquiries might miss; leave the principal reviewer room to reconcile them.

## Present an adaptable approach

Explain the consequential questions, useful initial investigations, assumptions and coverage
limits in the conversation. The approach is ready when productive investigation can begin and
material uncertainties have a route; preparation need not settle what the review will discover.
Ask only for information that materially changes the work.

Follow the user's boundary between planning and conducting the review. No separate preparation
document is required by default, and this skill does not switch runtime modes. The execution
skill owns the eventual report; investigation assignments need not determine its organization.
Conducting the review includes useful probes and experiments, while remediation remains
separately authorized work.
