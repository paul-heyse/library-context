---
id: ADR-0093
title: Assess domain modeling during bounded design reviews without mandatory flow tracing
status: accepted
date: 2026-09-30
supersedes: [ADR-0091]
superseded-by: null
design: [§2]
evidence: Implemented
revisit: Routine implementation again triggers mandatory modeling reviews or flow traces, or bounded reviews cannot establish model adequacy and authority.
---

## Context

The operator reports that prescribed tracing is slowing agents and asks to remove domain
modeling as a standing AGENTS.md instruction. Keep it in the design principles and design-review
skill, assessed during confined review periods. ADR-0091's domain-model criterion remains useful;
its standing mandate and prescribed investigation sequences impose unnecessary process.

[DESIGN §2](../design/DESIGN.md#section-2) owns this policy. The affected consumers are the
core/template, profile review guidance, reviewer, process skills and AGENTS.md. Product contracts
and the semantic-model cutover remain governed by their existing owners.

## Options

1. **Delete only the AGENTS.md bullet.** Smallest edit, but the skill, template and reviewer
   would still require tracing sequences and DESIGN would repeat the standing mandate.
2. **Remove the domain-model criterion.** Reduces review work but discards the operator's
   intended assessment of model adequacy and authority over behavior.
3. **Keep the criterion in bounded reviews and leave investigation to judgment** (chosen).
   Preserves architectural assessment while removing a recurring implementation procedure.

## Decision

- Adopt core/template 3.2 and code-intelligence guidance 1.3; preserve all FP, A, DP, G and CI IDs.
  Historical reviews retain their recorded versions. ADR-0040's cadence and independent
  architectural/fidelity judgments remain in force.
- Retain core §1, FP-04 and A2: a coherent, scoped model represents consequential distinctions,
  and domain behavior realizes its owned definitions and operation contracts. Ordinary types,
  relations and domain functions suffice. Output records alone do not establish alignment;
  in-scope MUST gaps still prevent review acceptance.
- Apply domain-model assessment in bounded design/review work at the declared cadence.
  Remove the standing modeling mandate from AGENTS.md and route review instructions to the
  principles and skill. Ordinary implementation does not itself trigger a modeling review.
- Remove prescribed phenomenon-to-consumer and change-propagation tracing sequences from
  active review guidance. Reviewers choose the least investigation sufficient to settle the
  scoped question. Flow tracing is optional when a concrete uncertainty warrants it.
- Retain relevant change scenarios, semantic ownership, composition, local reasoning, library
  fit and truthful evidence. No new hook, gate or review artifact type is introduced. Library
  exploration and capability skills are unchanged.

## Consequences

The guidance is Implemented as documentation policy. Reviewers retain responsibility for
adequate evidence and can investigate flows where needed, without completing a prescribed
trace for every assessment. Reduced agent overhead remains a Proposed benefit, not a measurement.

This supersedes ADR-0091's process clauses and carries forward its model and acceptance criteria
above. It does not certify product code or reopen historical reviews. Dated documentation,
ADR and agent validation belongs in STATUS; cutover qualification and findings remain in the
[cutover plan](../plans/semantic-model-cutover-plan_2026-09-29.md).
