---
id: ADR-0072
title: Compile the public catalog by default and compose explicit optional capabilities
status: accepted
date: 2026-09-28
supersedes: []
superseded-by: null
design: [§14.3, §14.4, §14.13]
evidence: Proposed
revisit: A catalog consumer requires an optional analysis to preserve a promised contract, or public exposure identity changes when binding knowledge improves.
---

## Context

ADR-0071 selects the API/evidence product. PR0–PR1 implement its first vertical slice.
Current compilation constructs public paths only with analytics configuration, and serving
unconditionally requires native semantic artifacts. Neither condition is necessary for source
contracts. The operator retains existing and specified additive queries, their structures and
their future obligations while prioritizing catalog delivery.

## Options

1. Keep a behavioral default and a second catalog pipeline: duplicates construction and leaves
   ordinary contract delivery dependent on unrelated analysis.
2. Remove deeper analysis: discards compatible capabilities and violates the preservation requirement.
3. One mandatory catalog with explicit optional enrichment (chosen): composes existing owners
   with a finite capability contract, without a scheduler framework or new store.

## Decision

Catalog is the default compile profile. Behavioral compilation builds the same catalog, adding
existing selected analysis and briefs. Mandatory inputs own public scope, embedding/cache inputs
and compiler provenance; optional analysis owns its existing models and parameters. Existing
analytics root configuration remains supported, including dotted roots. Frozen parameters and
evaluation digests remain unchanged. Scope uses exact-root-or-dot-descendant membership over
final exposures, retaining supporting owners outside the enumeration.

One schema-owned finite capability inventory drives publication, import, serving and recovery.
Not-requested differs from produced-empty, local unknown and corrupt advertised output. Native
value paths require their entire artifact closure. Optional selection never permits ignoring a
selected compiler failure. Brief admissibility remains strict.

Public member identity denotes a public exposure independently of later binding interpretation.
Declarations, overloads, shadowed definitions and provider observations retain distinct roles.
Preserve candidates before lossy winner selection, deriving preferred-path views for retained
semantic consumers. Ambiguous declaration lookups return choices. Operation IDs retain their
meaning; they never silently become public-member IDs.

Canonical Arrow/Delta, Rust semantics, SQLx/pgpq projections, exact retrieval, Qwen/1024 and
generation pinning remain. Use new projection versions and additive migrations. Preserve old
runtime assets for old-format recovery instead of adding a compatibility framework.

## Consequences

Catalog transforms can be tested independently of semantic analysis. Source-known and effective-
unresolved contracts coexist; targeted behavioral queries retain enrichment. Existing/spec'd
additive schemas, tests and deferred scope stay. Only superseded construction paths are replaced,
after their consumers migrate.

Implementation and qualification belong to [forward-plan PR0–PR1 and §6.2](../plans/behavioral-model-forward-plan_2026-09-24.md#30-consolidated-execution).
This decision closes no finding. Behavioral freeze/heldout remain unchanged; new confirmation
is isolated from implementation. Development stays editable, without wheel work or mid-scope gates.
