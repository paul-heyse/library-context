---
id: ADR-0100
title: Preserve residual children in typed native envelopes
status: accepted
date: 2026-09-30
supersedes: []
superseded-by: null
design: [§15]
evidence: Implemented
revisit: A consumer interprets a native envelope containing an opaque child
---

## Context

The full catalog pilot exposed a bound-method shape refused during shared validation. Native
bound methods, overloaded signatures and generic envelopes lower children through the bounded
term builder. A child can be `Other` (including unnamed native overloads) or `Truncated` at the
native depth/work limit. The producer propagates opacity, and the shared closure already
requires DisplayOnly support and matching provider/context for every residual.
[DESIGN §15](../design/sections/semantic-model.md) owns that fidelity and the envelope contract.

## Options

1. Preserve explicit residual children at their typed native ports and enforce closure fidelity.
2. Collapse the entire native parent to Other. This discards independently known structure.
3. Treat residual children as ordinary callables/type variables. This grants unsupported meaning.
4. Abort all otherwise supported facts. This discards independent observations.

## Decision

Choose option 1. Structural validation permits only Other/Truncated residuals in bound-method
function, overload signature, and generic parameter/body ports, alongside their existing named
forms. Ordered roles, nonempty sequences and ordinary wrong-arm refusals stay enforced. Shared
transitive support validation requires DisplayOnly and the same provider/context for a residual;
NativeStructural support refuses. This stores the native envelope without establishing invocation
or type-variable interpretation of its unknown child. The producer's existing bounded translation
and opacity propagation remain the sole correspondence owner.

## Consequences

No codebook renumbering or compatibility reader is needed; the model digest and store are rebuilt.
Shared model and real PostgreSQL controls challenge residual kinds, each port, structural fidelity
and foreign ownership. Full native pilots must publish before Q can exit; downstream consumers
must retain uncertainty until they have stronger evidence.
