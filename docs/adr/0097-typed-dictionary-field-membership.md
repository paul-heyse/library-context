---
id: ADR-0097
title: Model typed dictionary fields separately from callable parameters
status: accepted
date: 2026-09-30
supersedes: []
superseded-by: null
design: [§15]
evidence: Implemented
revisit: Another structural mapping type needs field membership
---

## Context

The full FastMCP facts pilot exposed anonymous TypedDict fields named `""` and `" "`.
[DESIGN §15](../design/sections/semantic-model.md) modeled these as keyword-only callable slots,
incorrectly applying parameter-name validation to arbitrary dictionary keys. These native fields
are supported structure, so discarding them or refusing publication loses independent evidence.

## Options

1. **Own dictionary-field membership separately.** Preserve string keys, requiredness, order and
term references under a typed field-list identity and shared stored membership invariant.
2. Loosen callable name validation. This conflates mapping keys with callable parameters.
3. Drop fields or retain only an opaque display. This loses supported structural evidence.
4. Encode dictionary keys as parameter names or invented substitutes. This changes native meaning.

## Decision

Choose option 1. `TypedDictFieldList` and `TypedDictField` own anonymous mapping fields.
`AnonymousTypedDict.fields` references that nominal list type. Arbitrary string keys are valid;
duplicate keys, ordinal gaps and changed membership refuse. The field-count ceiling is typed and
validator retention is charged. Type closure traverses field term references through this owner.
Nominal TypedDict `RecordFieldObservation` likewise permits arbitrary string keys; other record
kinds retain their field-name rule. Native adapters emit these records directly. Existing callable
parameter-name and requiredness rules remain unchanged. No old-schema readers or ID bridges exist.

## Consequences

The generated Arrow and PostgreSQL schema changes and is rebuilt; no codebook is renumbered.
Native dictionary fixtures and model/real PostgreSQL membership controls qualify the new owner;
the cutover plan owns full pilot and assembled acceptance. This is structural field preservation,
not a new invocation or P4 type reasoning claim.
