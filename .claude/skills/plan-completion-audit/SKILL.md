---
name: plan-completion-audit
description: Independently assess scope claimed complete against the accepted plan and actual implementation, then correct completion state and remaining obligations in the same owning plan. Use for a dedicated completion audit, not routine change review or new design exploration.
---

# Audit plan completion

Determine whether the claimed scope actually delivers the accepted plan, and correct its current
account in place. Completed packages, confident summaries and successful local reviews are useful
leads; none alone establishes that the intended capability and its consumers are complete.

Use this at a deliberate claimed-completion boundary, for the whole plan or an explicitly bounded
package. It complements implementation review through a fresh scope-oriented assessment; it is
not automatically repeated per slice or handoff. Choose investigation and depth for the actual
uncertainty. Do not manufacture gaps when the accepted scope is complete.

## Reconstruct what completion means

Identify the owning plan, claimed completion boundary, accepted scope changes and relevant source
state, including dirty changes that affect the judgment. For a coordinated series, include the
supporting plans and cross-plan obligations needed for that claim. A package audit cannot certify
the coordinator's enclosing scope. Resolve material ambiguity in what was claimed before giving
an overall conclusion; useful bounded inspection can proceed meanwhile.

Read the intended behavior, target design and acceptance discussion as well as work tables and
findings. Reconstruct obligations from those meanings, accepted decisions, migration/preservation/
retirement requirements and affected consumers. A narrative obligation does not disappear
because it was omitted from the package checklist. Conversely, a review's illustrative remedy is
not an accepted obligation merely because it was mentioned before planning.

Use current architectural owners and the plan's source references to understand the intended
contracts. Judge completion against the accepted plan, including authorized amendments; distinguish
new improvement opportunities from unfinished scope. An authorized exclusion or deferral may
bound completion, but an implementer's unilateral deferral cannot redefine it. If the plan and an
accepted authority conflict materially, identify the unresolved decision rather than silently
changing the target or assuming completion.

## Assess the delivered capability

Inspect implementation and consumers beyond the reported changed files. Follow the ordinary
composition and supported paths where they determine whether the capability is actually reached.
Look for seams between packages: a producer can be finished while its consumer still interprets
the old product, or a new operation can exist while defaults continue selecting the old route.
Preservation, retirement, migration and failure/recovery behavior can carry consequential scope
that happy-path controls miss. Use these as investigative leads, not required findings or an
exhaustive flow-tracing exercise.

Where accepted scope depends on a concrete library mechanism, assess whether its intended
guarantees, composition and material advantages reach the actual consumers, or whether a justified
replacement delivers the accepted target. Follow the plan's capability references when needed;
API-name presence alone cannot establish delivery. Distinguish a consequential capability dropped
during implementation from a legitimate local API adjustment or authorized design change.

Separate three questions in the conclusion:

- **Implementation:** does the actual system deliver the claimed functional scope, including
  applicable integration and transition obligations?
- **Verification and qualification:** does applicable evidence cover the required scope on the
  relevant source and execution context, and what remains failed, blocked, unknown or not run?
- **Benefit claims:** what is established about the intended improvement, and which measurements
  are required for completion versus explicitly deferred or optional?

A source-inspected missing consumer can establish incomplete implementation without a test run.
Passing tests cannot establish an obligation their selected scope does not exercise. Read the
relevant receipts and their applicability; keep historical dates, source boundaries and reporting
limits intact. Do not infer complete coverage from recipe names or overlapping pass counts.

Choose new checks only when they settle a material question, following repository timing and
provider/resource rules. An audit does not automatically launch full qualification or replace its
requirements. Unavailable required evidence remains outstanding; a mocked or skipped provider
does not become passed. A plan's claimed full completion must account for all its required
functional, applicable-provider and nonfunctional obligations. Functional implementation can be
complete while required qualification remains incomplete; report that distinction explicitly.

For each genuine gap, establish the missing obligation, source evidence, affected capability and
the repair or settling evidence needed. Give it an owner in the existing plan structure. Preserve
known finding identities; use stable plan-local identities in the plan's convention for new gaps.
Do not turn a newly preferred design into a completion defect or weaken acceptance to close a row.

## Correct the plan in place

Update the same owning plan rather than creating a separate completion-audit report. Maintain a
current **Completion audit** section with the audit date, inspected source baseline, claimed scope,
bounded conclusion, evidence limits and remaining obligations or links to their owning rows.
Organize the explanation for the subject; no formulaic schema or additional register is needed.

Reconcile affected package state, findings disposition, completion summaries and next work with
that judgment. Appending an audit beneath contradictory “complete” rows leaves a misleading plan.
Keep source finding IDs and the single disposition owner. For a series, the coordinator owns the
combined conclusion and dependencies; supporting plans hold only their affected local detail.
Do not duplicate mutable progress across them. The
[plan-structure guide](../create-plan/references/plan-structure.md#completion-audit-and-current-claims)
describes how this fits the existing document.

Keep prior receipts immutable and dated; correcting a completion claim does not invalidate every
earlier result. Git preserves earlier plan states, so replace the current account instead of
building a session diary. Report complete scope only at the established evidence strength, or
name the bounded acceptance and the implementation/qualification obligations still open.

Use the [shared roles](../../../.agents/roles/README.md) when independent judgment or evidence
gathering helps. A read-only implementation reviewer returns findings and proposed plan corrections
for coordinator publication, including a complete judgment when no gaps were found. Audit
authorization covers the plan corrections, not production repairs: describe actionable next
scope and continue repairs only when separately authorized. Ordinary instructions for preserving
parallel work, commits and [handoff](../handoff/SKILL.md) still apply. This audit does not itself
initiate broad principle diagnosis or a new solution review.
