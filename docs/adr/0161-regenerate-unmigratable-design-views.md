---
id: ADR-0161
title: Migrate current views and regenerate unsupported design data
status: accepted
date: 2026-10-12
supersedes: [ADR-0159]
superseded-by: null
design: ["storage-and-publication §5", "storage-and-publication §6.1"]
evidence: Proposed
---

## Context

The efficiency cutover introduced opaque retention and historical fixture requirements to preserve
unsupported source-generation views. On2026-10-12 the operator clarified that this early design-phase
system must retain no legacy runtime code or data. Views should migrate into the current model where
practical; unsupported views should instead be regenerated from their pinned inputs. The
[efficiency coordinator §4](../plans/efficiency-alignment-plan_2026-10-11.md#4-package-sequence-and-sole-execution-checkpoint)
owns implementation and the exact remaining cutover obligations. The shared paused installation
and operator state remain outside the dedicated campaign's effects.

## Options

1. Keep opaque historical closures and old namespaces for later collection. This protects original
   bytes but introduces records, converters and consumers with no continuing product need.
2. Migrate every historical shape before allowing current work. This couples current acceptance to
   unsupported fixture producers and expensive old-generation campaigns.
3. Migrate supported useful views into current exact realizations; regenerate unsupported views
   from pinned inputs and remove obsolete runtime paths after the owned cutover. This keeps current
   semantic ownership explicit without imposing compatibility on an early design-phase system.

FP-07/A4 and H6/H13/H18 favor option3: preparation and assurance serve an actual current consumer,
while lifecycle work ends at the completed transition. Historical preservation is not a product
requirement. First-principles assessment and minimal revealing functional cases are sufficient;
quantitative performance claims still require measurement when such claims are actually made.

## Decision

Current compiler and publication reads require protected exact current realizations. No legacy
view namespace, compatibility reader or opaque historical completed store survives cutover.
Supported views migrate when their meaning and ownership can be established practically. Otherwise
the plan names the view, pinned regeneration inputs and responsible current producer; it does not
invent declarations, assurance or semantic authority for an unsupported old result.

Before replacing old data, the actual owner fences new work and confirms process/service quiescence.
Unresolved interrupted outcomes remain unknown in their receipts; a joined dedicated daemon shutdown
may establish that no old remote executor survives, without turning the interrupted operation into
a pass. Cleanup applies only to the authenticated owned campaign. Shared/operator state is untouched.

After current content is established or regeneration is explicitly dispositioned, remove obsolete
runtime data, old-format converters, frozen historical fixtures and preservation-only code. Current
contracts receive focused functional acceptance; old-revision examples and benchmarks are not
prerequisites. This supersedes the whole opaque-preservation decision in ADR-0159. Exact identities,
current admission, ownership, and truthful cleanup/failure reporting remain required.

## Consequences

The cutover has fewer runtime representations and no ongoing historical consumer. Regeneration may
redo work, which is appropriate while the design is changing. The coordinator must explicitly account
for unmigrated views rather than silently claim equivalent migrated content. Native cutover and current
functional acceptance remain open; acceptance of this decision closes no source finding. Revisit if a
future actual product consumer requires retained historical data or compatibility across deployments.

## Amendments

2026-10-12: the same cutover decision also governs schema evolution in storage-and-publication §6.3; its ownership and recovery constraints remain, while completed historical converter paths retire. This clarifies the owning section without changing the decision.
