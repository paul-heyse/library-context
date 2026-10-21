---
name: design-review-audit
description: Independently reassess a completed design or functional-extension review's recommendations and omissions, publish dispositions and net changes, and integrate a revised review for planning. Preserve the source review's diagnostic or functional-value lens. Use for a requested recommendation audit, not ordinary implementation review.
---

# Audit review recommendations

Improve the recommendations and the combined design of an existing review. Account for every
original recommendation, identify valuable omissions within its focus, and leave a coherent
revised review that a planning agent can use without reconstructing the audit debate.

This is a deliberate review boundary, usually before plan creation. It does not implement
remedies, approve rule changes or silently revise an accepted plan. A dedicated fresh assessment
can expose assumptions the first review shared with its subject; independence does not require
disagreement or a quota of changes.

## Establish the basis

Identify the principal review, its kind, supporting material and exact input revision. Inherit
the operator's selected scope; an audit is not an occasion to choose a different focus for them.
For design reviews, retain the holistic, library-targeted, principle-focused or combined focus.
For functional reviews, retain the selected outcome, workflow, subsystem or ecosystem.
Clarify a materially ambiguous scope.
Record the reviewed source state, applicable standard and changes since the original assessment.
Distinguish changed circumstances from errors or missed opportunities in the original review.

For a **design review**, load [design-review](../design-review/SKILL.md) as assessment criteria
and follow its standard, profile, target and binding routes. Apply its independent architectural
and correctness/fidelity judgments. Normally continue the engagement using its still-applicable
diagnostic audit; reconcile intervening changes. A new design-review engagement follows the
binding's freshness rule. Loading assessment criteria alone does not start another broad audit.

For a **functional-extension review**, load
[functional-extension-review](../functional-extension-review/SKILL.md) and its standard routes.
Reassess functional value, proposed scope, realization and valuable omissions as well as design
quality. Current capabilities and intended direction ground the opportunity; a principle violation,
current caller or diagnostic audit is not required. Do not import the diagnostic review's audit
prerequisite or full report-slot structure. Existing rules describe current design in either route;
recommendations can identify rule impacts through the existing decision process.

## Reconsider the recommendations

Choose the investigation that can change the decision. Read decisive evidence yourself; the
first review's citations and conclusions are leads, not independent corroboration. Static
reasoning may suffice. Library research follows the repository's overview, Context7 and primary
source routes; probes are useful when they settle a real uncertainty.

Reassess concrete library mechanisms on which recommendations depend, including their composed
semantics and material limits. Preserve retained or corrected mechanisms,
their design role and precise source references in the integrated review; explain consequential
replacements in the companion. Keep candidates and unresolved feasibility distinguishable from
the resulting selected direction, so synthesis does not erase the enabling functionality.

Distinguish a diagnosis from its remedy. A real defect may have a poor correction; an appealing
mechanism may answer the wrong problem. Consider whether the proposed responsibility boundary
preserves meaning and useful guarantees, whether a simpler or more capable alternative serves
the outcome, and which assumptions make its benefit credible. Challenge consequential remedies
with legitimate supported cases and their affected consumers, including transition implications
where those could invalidate the choice. These are reasoning leads, not a mandatory checklist.

Assess interactions as well as individual recommendations: remedies can duplicate authority,
conflict over a contract or depend on an unstated prerequisite. Look beyond the first review's
enumeration for overlooked causes and improvements within the selected focus. Do not force
distinct problems into one redesign or treat every unexamined area as a defect. Encountered
out-of-focus concerns can be recorded without expanding the solution assignment.

Retain sound recommendations. Improve an adequate direction when another formulation or
mechanism would serve it better; correct faulty premises or scope; omit an unjustified remedy.
Leave a consequential unresolved choice visible with the evidence or decision that would settle
it. Omitting a remedy does not by itself dispose of its still-valid diagnosis.

For functional proposals, distinguish the opportunity from its realization. A valuable capability
can have a poor implementation direction, while an elegant mechanism can enable little useful
behavior. Challenge the proposed consumer, benefit assumptions, prerequisites, integration and
ongoing operation; identify overlooked capabilities and composed opportunities within the chosen
scope. Hypothetical demand remains a hypothesis without making an existing caller a requirement.
Retain or improve valuable proposals without inventing a diagnostic defect to justify them.

## Publish two complementary outputs

For design reviews, the
[binding](../../../docs/design_review/design_principles/binding/library-context.md#design-review-recommendation-audit)
owns locations and the [audit-output guide](../../../docs/design_review/reviews/audits/README.md)
describes companions. Functional reviews use the
[functional collection](../../../docs/functional_reviews/README.md#recommendation-audits) and its
[companion guide](../../../docs/functional_reviews/audits/README.md). Use flexible Markdown at
the depth the reasoning needs.

**Companion net changes.** Identify the exact original review revision, source/standard baselines,
audit date and coverage limits. Account for every original recommendation with a disposition:
**Retain**, **Improve**, **Correct or adjust**, **Omit**, or **Unresolved**. Explain material
changes and their consequences; compact retained entries can share a table. Include additional
recommendations and their evidence, and link changes to the integrated review. A recommendation
reference is distinct from a finding: one diagnosis can support several remedies. Reuse existing
references, or identify recommendations by original section and a short description when no IDs
exist. Preserve correspondence across combined, split or replaced remedies without a new registry.

**Integrated principal review.** Update the original review path with the complete resulting
argument, findings, recommendations and evidence limits. Reconcile the synthesis, alternatives,
dependencies, applicable architectural/fidelity judgments, overall recommendation and rule impacts
with the revised proposal. Design reviews retain their A1–A4 and gate account; functional reviews
retain the value argument, opportunities and priorities without adopting the full design template. The
principal review should explain that design directly; the companion owns judgments of the first
round. Briefly attribute the revised judgment, its date and input revision and link the companion.
Do not imply that the original reviewer authored or agreed with the new judgment.

Preserve stable opportunity, diagnostic/source finding and rule-impact IDs; new items use the existing
convention without reusing removed identities. Preserve original dated receipts and strength
labels rather than promoting them to current passes. If an omitted remedy leaves a finding open,
keep its consequence and next decision discoverable in the integrated review.

Before replacing the principal review, make its exact input recoverable from Git and cite that
revision in the companion. If it is uncommitted, the coordinator first preserves the stabilized
original through the repository's commit policy, coordinating with any concurrent author. A
read-only reviewer returns both complete texts and intended paths for coordinator publication;
publication preserves the auditor's judgment. Do not overwrite another agent's active draft.

Editorial dispositions in the companion are not implementation status. Scheduled findings retain
their owning plan; the revised review links that owner. Flag effects on plans already consuming
the original, without silently accepting new scope or reopening settled execution decisions.
Rule changes still require the operator's confirmation in plan creation. Report the bounded
result, remaining uncertainty and both output paths; review acceptance is not product qualification.
