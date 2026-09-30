---
id: ADR-0098
title: Keep native slot text separate from callable interpretation
status: accepted
date: 2026-09-30
supersedes: []
superseded-by: null
design: [§15]
evidence: Implemented
revisit: A consumer needs to interpret expanded native slots with arbitrary string keys
---

## Context

ADR-0097 separates TypedDict keys from callable parameters. Its native control also exposes
expanded signatures containing an empty keyword-slot name. The provider reports a real string
key; it is evidence rather than a written parameter declaration. [DESIGN §15](../design/sections/semantic-model.md)
owns native correspondence, signature interpretation and type fidelity.

## Options

1. **Retain native text and enforce interpretation at the callable form.** Structural shapes retain
empty text; bindable lists refuse it. Unavailable native signatures/types preserve the slots.
2. Rename or remove empty slots. This changes native evidence and ordinal meaning.
3. Permit empty names as ordinary callable declarations. This grants unsupported invocation meaning.
4. Abort the whole facts generation. This discards independent supported observations.

## Decision

Choose option 1. A parameter shape retains its native optional text. `SignatureForm::List` refuses
empty names; the unavailable native signature form retains them and binding refuses. Append
`CallableForm::NativeUnavailable` without renumbering. Expanded type callables with empty slot names
use this form, retain typed members/returns, contribute a boundary, and have display-only support.
Type closure validates that fidelity transitively; ordinary callable list/partial forms reject
empty names. Requiredness and slot-kind checks remain at the structural owner. Native mapping keys
continue to use ADR-0097's distinct field owner. No consumer may silently treat the unavailable form
as an ordinary callable. Resource and operational-limit failures remain fail-stop.

## Consequences

The model/schema is rebuilt, with no compatibility reader. Native dictionary/signature controls,
model refusal and fidelity mutations, and full facts pilots own qualification in the cutover plan.
Unavailable callable evidence is retained; no new invocation interpretation is claimed.
