---
name: design-principle-audit
description: Audit the codebase broadly against the design principles and refresh its persisted diagnostic account before a design-review engagement. Use for an explicit audit or the review prerequisite; ordinary implementation does not initiate an audit.
---

# Audit design principle alignment

Develop a current, evidence-grounded account of design principle misalignment across the codebase,
without an incoming remediation objective, library preference or principle subset. The system's
functional target supplies the context for judgment. An audit can establish a structural problem
before incorrect output occurs; it does not need to settle the best replacement.

Find the layered standard through repository instructions and `standard.toml`. Read the core
principles, heuristic companion, applicable profile principles and repository binding. The binding
owns the audit location, review prerequisite and disposition routes. Use the same criteria and
claim vocabulary as design review, without imposing its solution comparison or acceptance report
on the audit. Loading criteria does not initiate another review.

## Output in this repository

Write and refresh the living audit under `docs/design_review/audit/`. Start at the
[collection README](../../../docs/design_review/audit/README.md), which holds the shared baseline,
coverage limits and suggested topic pages. Add topic documents when findings make them useful;
the scaffold does not require empty category files or a new dated directory for each audit.
The [repository binding](../../../docs/design_review/design_principles/binding/library-context.md#design-principle-audit)
owns freshness and disposition. Use the README's reading structure and the finding-writing
reference below without duplicating the audit account in the skill.

## Establish the account and its baseline

Read the current audit and establish which source state it examined. Distinguish the audited
source commit from the commit that publishes the audit; identify included uncommitted changes
rather than describing them as HEAD. Record the date, applied standard versions and meaningful
coverage limits. An empty scaffold has no audited baseline and supplies no alignment evidence.

For a refresh, use changes since the preceding audited source state to direct attention. Consider
changed contracts, consumers, configuration, dependencies and functional expectations as well as
implementation edits. Reconsider prior findings whose premises may have changed. Then inspect the
current architecture, following affected dependencies into unchanged code where its significance
has changed. The diff and the prior account are leads, not a boundary or a verdict. A first audit
has no prior baseline; changes to the standard can also reopen an unchanged implementation.

Choose breadth, depth and division of reasoning according to the codebase and uncertainty.
Owner boundaries, representative operations and credible changes can reveal different problems.
Look across their interactions as well as within them. State what was inspected and what remains
uncertain without requiring a file census, exhaustive principle matrix or numerical coverage score.
The [review calibration](../design-review/REFERENCE.md#architecture-calibration) offers useful
examples of architectural consequences and false positives; use the relevant examples selectively.

## Explain causes, consequences and limits

Group manifestations when they share a structural cause; sharing a component or principle alone
does not establish one cause. Describe the concrete responsibility, meaning, contract or physical
choice at issue and why it matters to supported behavior, credible change or execution. Distinguish
known manifestations from a suspected wider footprint. A large file, similar-looking code or an
unused library feature alone does not establish a violation.

Keep established diagnoses, hypotheses and unexamined areas distinguishable using the existing
evidence vocabulary. Read decisive sources and include material contrary evidence. Source reasoning
may suffice; probes help when they resolve a consequential uncertainty. Qualitative execution-fit
assessment needs no cost model or benchmark campaign; quantitative claims still need measurements.
Library questions use the repository's overview, Context7 and primary-source research route.

A solution lead may be useful context, but the audit need not choose a remedy, adoption priority or
implementation package. Preserve enough explanation for a later reviewer to challenge the diagnosis
and explore a better correction. Follow the [finding-writing guidance](references/audit-writing.md)
for flexible Markdown organization and examples.

## Maintain one living collection

Update the account in place, retaining stable finding identities as explanations and document
boundaries evolve. Give each finding one primary home, with all relevant principle references and
its affected footprint. Preserve lineage when findings merge or split; never reuse retired IDs.
Git holds earlier accounts. Remove resolved material when current evidence establishes the
correction, retaining a route to the dated evidence through the removal commit.

The audit owns the diagnostic account at its stated baseline. Reviews own solution reasoning;
implementation plans own scheduled work, progress and closure evidence. Link to those owners
without copying their mutable status. A review consumes an identified audit revision, not an
unversioned statement that later edits can silently change. An inherited observation that was not
reexamined keeps its original evidence date and a clear applicability limit.

Use the shared coordination contract when delegation improves coverage or independent judgment.
Read-only contributors return findings, sources, baseline and limits; the coordinator reconciles
and publishes the account. No extra audit role or fixed assignment sequence is required.

Deliver the updated collection, its inspected baseline, material discoveries and coverage limits.
An audit does not certify complete alignment, authorize remediation or automatically start a
solution review. It is refreshed before a new review engagement under the binding, not maintained
as routine production-editing context. A request to change this process does not itself require
performing an audit.
