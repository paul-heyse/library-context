# Functional opportunity reviews

Reviews here explore valuable additions or improvements to product, developer, operator and
research capabilities. The operator selects an outcome, workflow, subsystem or tooling/library
ecosystem; reviewers discover useful possibilities and assess their realization within that
scope. A prior diagnostic audit, existing caller or design-principle violation is not required.

Use [plan-functional-extension-review](../../.claude/skills/plan-functional-extension-review/SKILL.md)
for conversational preparation and [functional-extension-review](../../.claude/skills/functional-extension-review/SKILL.md)
for the review. Design principles shape proposed realizations alongside the functional-value
argument. The diagnostic [design-review workflow](../../.claude/skills/design-review/SKILL.md)
retains its separate audit prerequisite and review conventions.

## Outputs and traceability

Use `<subject>_<YYYY-MM-DD>.md` for the principal review, with its scope, inspected source
baseline and evidence limits near the beginning. Explain current capabilities, useful
opportunities, resulting behavior, concrete enabling mechanisms and precise sources, alternatives,
interactions and recommended priorities. Organize flexible Markdown around the argument; no
fixed field schema, feature count or complete ecosystem inventory is required.

Give opportunities stable references such as `O01`, cited as `review-path#O01`; explicit anchors
make these usable across documents. Preserve references when a recommendation changes, and
explain correspondence when opportunities split or combine. Link related diagnostic findings
when useful without requiring a finding or relabeling an opportunity as a defect.

A review proposes value and direction. It does not approve changes to the functional target,
rules or an existing plan, and it does not authorize installation or implementation. When selected
opportunities move into a plan, retain their source references and concrete mechanisms in the
relevant target and packages. The owning plan holds current scheduled scope and disposition;
unscheduled recommendations remain here with meaningful next decisions or revisit conditions.
There is no separate mutable opportunity register or required periodic refresh.

## Recommendation audits

Use [design-review-audit](../../.claude/skills/design-review-audit/SKILL.md) for a requested or
explicitly planned reassessment. It inherits this review's functional scope and assesses value,
scope, realization, interactions and missed opportunities. It does not initiate a broad diagnostic
audit or require the design-review report's full slot structure.

The [companion](audits/README.md) owns dispositions and net changes; the principal review is
revised in place into the complete resulting argument with dated attribution and recoverable
input. Planning consumes that clean integrated review. Editorial dispositions do not change
accepted implementation scope or plan progress.

Retain reviews and companions while they have current consumers. Follow the repository's
document lifecycle: carry surviving obligations and enduring knowledge to their owners before
retirement; Git preserves earlier accounts. Creating this scaffold performs no review or audit.
