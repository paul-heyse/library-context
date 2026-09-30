---
id: ADR-0091
title: Require explicit domain models to govern implementation and architectural review
status: accepted
date: 2026-09-29
supersedes: []
superseded-by: null
design: [§2]
evidence: Implemented
revisit: Reviews accept output-only or inadequate domain models, miss independent semantic interpretations, or prescribe modeling machinery without a consumer.
---

## Context

On 2026-09-29 the operator approved making domain modeling mandatory throughout implementation,
while preserving the other architectural foundations and freedom of realization. Core 3.0
already requires semantic authority, typed distinctions and transformation fidelity, but does
not directly establish that these contracts form an adequate model of the intended phenomena
and govern domain behavior. Consistent output schemas can conceal independently defined rules.

This decision extends ADR-0040's standard under [DESIGN §2](../design/DESIGN.md#section-2).
Its ownership, review cadence, independent judgments and finding-disposition policy remain in
force. The affected consumers are the core template, code-intelligence review guidance,
reviewer agent, process skills and agent instructions. Library exploration and capability
skills are outside the operator-authorized change.

## Options

1. **Add only a governing paragraph.** Smallest edit, but the review questions and agent summaries
   would still permit output schemas to stand in for model-governed behavior.
2. **Add another foundation, gate or modeling framework.** Gives the requirement visibility, but
   duplicates existing authority and fidelity checks or prescribes machinery without a consumer.
3. **Strengthen the existing foundations and judgments** (chosen). One governing MUST, FP-04
   ownership, A2 acceptance and aligned review traces make the requirement actionable while
   preserving separation of concerns, composition, local reasoning and proportionality.

## Decision

- Adopt core/template 3.1 and code-intelligence guidance 1.2; retain every FP, A, DP, G and CI ID.
  The manifest declares versions; historical reviews retain the standards they actually used.
- Core §1 requires an explicit, coherent model of supported phenomena and domain behavior.
  Consequential distinctions and domain operations have scoped authorities. Behavior realizes
  those definitions, including at adapter, orchestration, persistence and presentation boundaries.
- FP-04 owns explicit domain modeling and scoped semantic authority. A2 requires both model
  adequacy and authoritative realization; a gap prevents acceptance for supported behavior
  even when current outputs are correct. Deferral cannot waive a MUST.
- Existing rules cover domain operation contracts, invariant ownership, contextual bindings
  and contract-scoped semantic preservation. Ordinary types, relations and domain functions
  suffice; no universal ontology, DSL, registry or serialization of every operation is required.
- Existing review slots trace phenomenon → authoritative concept or operation → implementation
  → consumer → expected change. Classify changes as instances, bindings, compositions, policies,
  domain concepts or mechanisms. Review depth and verification remain proportional to uncertainty;
  no new hook, gate, artifact type or integrated product check is introduced.
- Align guidance that describes the principles, including the reviewer and process skills.
  Library-exploration requirements, review slot 8 and library capability skills are unchanged.

## Consequences

The standard and agent guidance are Implemented as documentation policy. Reviewers must inspect
both the represented phenomena and the executing operations rather than treating consistent
shapes as sufficient. A mechanism replacement should preserve its domain contract or expose an
incompatibility; no guarantee makes every replacement inexpensive.

Improved extension locality and reduced semantic duplication remain Proposed benefits until
supported by actual changes. This revision neither certifies the existing implementation nor
reopens historical reviews. Product implementation and cutover qualification remain with their
existing owners. Validation uses the existing documentation, ADR and agent checks; their dated
outcomes belong in STATUS, separate from product qualification.
