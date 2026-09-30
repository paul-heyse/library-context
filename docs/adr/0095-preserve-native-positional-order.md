---
id: ADR-0095
title: Preserve provider positional parameter order
status: accepted
date: 2026-09-30
supersedes: []
superseded-by: null
design: [§15]
evidence: Implemented
revisit: A provider needs a new parameter group rather than positional-kind interleaving
---

## Context

The full FastMCP facts pilot exposed a Pysa method signature containing positional-or-keyword
`self` followed by positional-only `__context`. The native report preserves a legacy typing
convention. `Signature::new` incorrectly ordered parameter kinds by codebook number, rejecting
this evidence. [DESIGN §15](../design/sections/semantic-model.md) owns parameter shapes and binding.

## Options

1. **Preserve positional kinds and ordinal order.** Validate positional parameters as one ordered
group, followed by varargs, keyword-only and kwargs. Binding still distinguishes each slot's kind.
2. Sort or relabel native parameters. This changes ordinal or keyword admissibility and loses fidelity.
3. Omit the signature and report a boundary. This discards supported structure solely because the
model mistook a codebook for grammar.

## Decision

Choose option 1. Positional-only and positional-or-keyword have equal group rank for validation;
provider ordinals and per-slot kinds remain unchanged. Later parameter groups cannot move backward;
duplicate names/varargs/kwargs and required positional parameters after defaults still refuse.
The binder consumes ordinals and kinds, never sorts by codebook. Ruff syntax retains its own grammar.

## Consequences

Valid provider evidence no longer aborts publication. Codebook values remain append-only and unchanged.
A focused native fixture and model binding controls qualify this correction; the cutover plan owns
full pilot and assembled qualification.
