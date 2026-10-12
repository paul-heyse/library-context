---
id: ADR-0159
title: Preserve unsupported original owners without semantic admission
status: superseded
date: 2026-10-12
supersedes: []
superseded-by: ADR-0161
design: ["storage-and-publication §5", "storage-and-publication §6.1"]
evidence: Proposed
---

## Context

The adjacent source5 migration encounters completed custom test producers outside the closed
original declaration registry. One uses an invented model identity. Physical completion does not
establish the meaning of those producers; it also does not authorize losing their original bytes.
Authentic registered producers must still pass their declaration, configuration, dependency and
semantic gates. The [efficiency coordinator §4](../plans/efficiency-alignment-plan_2026-10-11.md#4-package-sequence-and-sole-execution-checkpoint)
owns implementation and acceptance of the migration, including these original fixture owners.

## Options

1. Refuse the entire installation whenever unsupported completed output exists. This preserves
   everything but prevents an otherwise supported migration of independent admitted content.
2. Invent a producer declaration, accept the descriptor as authority, or discard the output. These
   alternatives respectively fabricate meaning or lose retained evidence and are rejected.
3. Preserve the exact original closure as opaque current-format data, independently of semantic
   translation. This adds a narrow migration-owned representation with a concrete forensic and
   resume consumer. It allows supported independent content to migrate without changing the
   unsupported owner's meaning.

FP-07/A4 and H6/H13/H18 favor separate meaning and retention boundaries, one immutable preparation
for repeated resume, and explicit ownership rather than duplicated semantic validators. Option 3
requires bounded copying and independent readback; option 1 remains the safe refusal whenever the
closure or separation cannot be established.

## Decision

Compiler migration classifies only actual unsupported completed source owners whose original
specification/completion identities and physical references can be authenticated. A recognized
producer with altered or missing configuration remains an error. Admitted or selected unsupported
semantic content, and a supported semantic dependency on an unsupported owner, refuse migration.

The native migration journal protects opaque original envelopes immediately on allocation. Each
opaque owner binds its complete original closure, including original descriptors, membership,
dependency views, backing, aliases, original chunks and actual incidence/guard records. Exact
original bytes and addresses remain recoverable. These records are outside completed views,
producer assurance, artifact admission and captured-product reuse. They never become a current
completed descriptor merely through deserialization or byte agreement.

The migration verifies every opaque envelope, exact owner incidence and inventory EOF before
retained-source ownership may be preserved on this basis. Global verification and activation
repeat coverage checks. Resume reconstructs the original graph from the preserved envelopes after
other components change physical incidence. Original Control translations are accepted only with
that journal's exact original-guard or frozen-attempt proof; unrelated source mutations refuse.

Classified original attempts and products retain their actual bytes and original ownership
obligations. Classification does not mint a replacement product or collector, release old holds,
or authorize deletion. Shared backing required by supported content also enters its independently
protected target realization. Current product discovery excludes old-model contributions before
attempting current descriptor decoding; an unsupported product cannot gain reuse eligibility.

## Consequences

Supported independent content can migrate while unsupported custom evidence remains accessible to
its named forensic/resume consumer. Extra opaque rows, missing closure members, altered bytes,
uncertain ownership and semantic dependencies across the boundary refuse. Bounded allocation,
resume, tamper, mixed-owner preservation and actual native activation controls remain required.
The decision is accepted; source integration and focused checks do not establish native migration
acceptance or whole-plan closure. Future retirement needs an actual consumer-release and checked
original collector route; no retention deadline or storage collection follows from this decision.
